//! VCkss-owned contiguous-column bridge for independent CMG PCG solves.
//!
//! The estimator prepares repeated right-hand sides in one column-major
//! allocation.  This bridge preserves CMG's scalar PCG recurrences and its
//! normal serial/planned/across-RHS routing while avoiding an intermediate
//! `Vec<Vec<f64>>` allocation and copy.

use rayon::prelude::*;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::{
    CmgError, ParallelPcgExecution, ParallelPcgSolver, PcgOptions, PcgResult, PcgWorkspace,
    solve_pcg_with_initial_guess_and_workspace,
    solve_pcg_with_initial_guess_and_workspace_cancellable,
    solve_pcg_with_plan_and_initial_guess_and_workspace,
    solve_pcg_with_plan_and_initial_guess_and_workspace_cancellable,
    solve_pcg_with_plan_and_workspace, solve_pcg_with_plan_and_workspace_cancellable,
    solve_pcg_with_workspace, solve_pcg_with_workspace_cancellable,
};

impl ParallelPcgSolver {
    /// Heap metadata retained by the ordered queue and its returned result list.
    /// Solution payloads are charged separately by the caller's batch forecast.
    pub fn vckss_queue_metadata_bytes(columns: usize) -> Result<usize, CmgError> {
        columns
            .checked_mul(
                std::mem::size_of::<OnceLock<Result<PcgResult, CmgError>>>()
                    + std::mem::size_of::<PcgResult>(),
            )
            .ok_or(CmgError::AllocationFailed {
                context: "contiguous queue metadata overflow",
            })
    }

    /// Empty pool for a two-phase, pre-RNG capacity admission.
    pub fn vckss_empty_contiguous_workspace(&self) -> VckssContiguousPcgWorkspace {
        VckssContiguousPcgWorkspace {
            workspaces: Vec::new(),
        }
    }

    /// Fallibly allocate the selected scalar workspace pool before RNG.
    pub fn vckss_prepared_contiguous_workspace(
        &self,
        count: usize,
    ) -> Result<VckssContiguousPcgWorkspace, CmgError> {
        let mut workspaces = Vec::new();
        workspaces
            .try_reserve_exact(count)
            .map_err(|_| CmgError::AllocationFailed {
                context: "prepared scalar pool",
            })?;
        for _ in 0..count {
            workspaces.push(PcgWorkspace::try_new(self.preconditioner())?);
        }
        Ok(VckssContiguousPcgWorkspace { workspaces })
    }

    /// Run an indexed operation on the solver-owned pool and preserve order.
    pub fn vckss_map_ordered<Input, Output, Operation>(
        &self,
        input: Vec<Input>,
        operation: Operation,
    ) -> Vec<Output>
    where
        Input: Send,
        Output: Send,
        Operation: Fn(Input) -> Output + Send + Sync,
    {
        self.executor()
            .install(|| input.into_par_iter().map(operation).collect())
    }

    /// Allocate the reusable workspace pool for contiguous VCkss batches.
    #[must_use]
    pub fn vckss_contiguous_workspace(&self) -> VckssContiguousPcgWorkspace {
        VckssContiguousPcgWorkspace::new(self)
    }

    /// Solve independent RHS columns from one contiguous column-major block.
    ///
    /// Columns are returned in input order. Each column uses the official
    /// scalar or planned CMG PCG implementation selected by the normal batch
    /// policy; no block-CG coupling or cross-column reductions are introduced.
    pub fn vckss_solve_contiguous_columns_with_workspace(
        &self,
        right_hand_sides: &[f64],
        columns: usize,
        options: PcgOptions,
        workspace: &mut VckssContiguousPcgWorkspace,
    ) -> Result<Vec<PcgResult>, CmgError> {
        self.vckss_solve_contiguous_columns_impl(
            right_hand_sides,
            None,
            columns,
            options,
            workspace,
            None,
        )
    }

    /// Solve independent contiguous columns with cooperative atomic
    /// cancellation at batch, PCG, and recursive V-cycle boundaries.
    pub fn vckss_solve_contiguous_columns_with_workspace_cancellable(
        &self,
        right_hand_sides: &[f64],
        columns: usize,
        options: PcgOptions,
        workspace: &mut VckssContiguousPcgWorkspace,
        cancellation: &AtomicBool,
    ) -> Result<Vec<PcgResult>, CmgError> {
        self.vckss_solve_contiguous_columns_impl(
            right_hand_sides,
            None,
            columns,
            options,
            workspace,
            Some(cancellation),
        )
    }

    /// Refine independent RHS columns from caller-supplied initial guesses.
    ///
    /// RHSs and guesses use the same contiguous column-major layout. Each
    /// column remains an independent scalar PCG solve under the normal batch
    /// routing policy.
    pub fn vckss_solve_contiguous_columns_from_initial_guesses_with_workspace(
        &self,
        right_hand_sides: &[f64],
        initial_guesses: &[f64],
        columns: usize,
        options: PcgOptions,
        workspace: &mut VckssContiguousPcgWorkspace,
    ) -> Result<Vec<PcgResult>, CmgError> {
        self.vckss_solve_contiguous_columns_impl(
            right_hand_sides,
            Some(initial_guesses),
            columns,
            options,
            workspace,
            None,
        )
    }

    /// Refine contiguous independent columns from initial guesses with
    /// cooperative atomic cancellation.
    pub fn vckss_solve_contiguous_columns_from_initial_guesses_with_workspace_cancellable(
        &self,
        right_hand_sides: &[f64],
        initial_guesses: &[f64],
        columns: usize,
        options: PcgOptions,
        workspace: &mut VckssContiguousPcgWorkspace,
        cancellation: &AtomicBool,
    ) -> Result<Vec<PcgResult>, CmgError> {
        self.vckss_solve_contiguous_columns_impl(
            right_hand_sides,
            Some(initial_guesses),
            columns,
            options,
            workspace,
            Some(cancellation),
        )
    }

    fn vckss_solve_contiguous_columns_impl(
        &self,
        right_hand_sides: &[f64],
        initial_guesses: Option<&[f64]>,
        columns: usize,
        options: PcgOptions,
        workspace: &mut VckssContiguousPcgWorkspace,
        cancellation: Option<&AtomicBool>,
    ) -> Result<Vec<PcgResult>, CmgError> {
        crate::cancel::checkpoint(cancellation, "vckss_batch_entry")?;
        let dimension = self.graph().vertex_count();
        let expected = dimension.checked_mul(columns).ok_or_else(|| {
            CmgError::dimension(
                "VCkss contiguous RHS batch",
                usize::MAX,
                right_hand_sides.len(),
            )
        })?;
        if columns == 0 || right_hand_sides.len() != expected {
            return Err(CmgError::dimension(
                "VCkss contiguous RHS batch",
                expected,
                right_hand_sides.len(),
            ));
        }
        if initial_guesses.is_some_and(|guesses| guesses.len() != expected) {
            return Err(CmgError::dimension(
                "VCkss contiguous initial-guess batch",
                expected,
                initial_guesses.map_or(0, <[f64]>::len),
            ));
        }

        let report = self.select_batch_execution(columns)?;
        workspace.ensure_count(report.concurrency().max(1), self)?;
        let solve_one = |rhs: &[f64],
                         initial_guess: Option<&[f64]>,
                         pcg_workspace: &mut PcgWorkspace| {
            crate::cancel::checkpoint(cancellation, "vckss_batch_column")?;
            match (report.execution(), cancellation) {
                (ParallelPcgExecution::Planned, Some(cancellation)) => match initial_guess {
                    Some(initial_guess) => {
                        solve_pcg_with_plan_and_initial_guess_and_workspace_cancellable(
                            self.graph(),
                            self.preconditioner(),
                            self.plan(),
                            rhs,
                            initial_guess,
                            options,
                            pcg_workspace,
                            self.executor(),
                            cancellation,
                        )
                    }
                    None => solve_pcg_with_plan_and_workspace_cancellable(
                        self.graph(),
                        self.preconditioner(),
                        self.plan(),
                        rhs,
                        options,
                        pcg_workspace,
                        self.executor(),
                        cancellation,
                    ),
                },
                (ParallelPcgExecution::Planned, None) => match initial_guess {
                    Some(initial_guess) => solve_pcg_with_plan_and_initial_guess_and_workspace(
                        self.graph(),
                        self.preconditioner(),
                        self.plan(),
                        rhs,
                        initial_guess,
                        options,
                        pcg_workspace,
                        self.executor(),
                    ),
                    None => solve_pcg_with_plan_and_workspace(
                        self.graph(),
                        self.preconditioner(),
                        self.plan(),
                        rhs,
                        options,
                        pcg_workspace,
                        self.executor(),
                    ),
                },
                (
                    ParallelPcgExecution::Serial | ParallelPcgExecution::AcrossRightHandSides,
                    Some(cancellation),
                ) => match initial_guess {
                    Some(initial_guess) => solve_pcg_with_initial_guess_and_workspace_cancellable(
                        self.graph(),
                        self.preconditioner(),
                        rhs,
                        initial_guess,
                        options,
                        pcg_workspace,
                        cancellation,
                    ),
                    None => solve_pcg_with_workspace_cancellable(
                        self.graph(),
                        self.preconditioner(),
                        rhs,
                        options,
                        pcg_workspace,
                        cancellation,
                    ),
                },
                (
                    ParallelPcgExecution::Serial | ParallelPcgExecution::AcrossRightHandSides,
                    None,
                ) => match initial_guess {
                    Some(initial_guess) => solve_pcg_with_initial_guess_and_workspace(
                        self.graph(),
                        self.preconditioner(),
                        rhs,
                        initial_guess,
                        options,
                        pcg_workspace,
                    ),
                    None => solve_pcg_with_workspace(
                        self.graph(),
                        self.preconditioner(),
                        rhs,
                        options,
                        pcg_workspace,
                    ),
                },
            }
        };

        match report.execution() {
            ParallelPcgExecution::Serial | ParallelPcgExecution::Planned => right_hand_sides
                .chunks_exact(dimension)
                .enumerate()
                .map(|(column, rhs)| {
                    let begin = column * dimension;
                    let initial_guess =
                        initial_guesses.map(|guesses| &guesses[begin..begin + dimension]);
                    solve_one(rhs, initial_guess, &mut workspace.workspaces[0])
                })
                .collect(),
            ParallelPcgExecution::AcrossRightHandSides => self.executor().install(|| {
                map_workspace_queue(
                    &mut workspace.workspaces[..report.concurrency()],
                    columns,
                    |column, pcg_workspace| {
                        let begin = column * dimension;
                        let rhs = &right_hand_sides[begin..begin + dimension];
                        let initial_guess =
                            initial_guesses.map(|guesses| &guesses[begin..begin + dimension]);
                        solve_one(rhs, initial_guess, pcg_workspace)
                    },
                )
            }),
        }
    }
}

/// Each admitted workspace drains a common queue. Only the small result slots
/// are shared; numerical state and arithmetic stay local to one RHS. A failed
/// column stops later claims, while earlier in-flight columns finish so errors
/// remain ordered by input position rather than worker completion time.
fn map_workspace_queue<Workspace, Output, Operation>(
    workspaces: &mut [Workspace],
    columns: usize,
    operation: Operation,
) -> Result<Vec<Output>, CmgError>
where
    Workspace: Send,
    Output: Send + Sync,
    Operation: Fn(usize, &mut Workspace) -> Result<Output, CmgError> + Sync,
{
    if columns > 0 && workspaces.is_empty() {
        return Err(CmgError::InvalidHierarchy {
            context: "empty contiguous queue workspace pool",
        });
    }
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(columns)
        .map_err(|_| CmgError::AllocationFailed {
            context: "contiguous queue result slots",
        })?;
    slots.resize_with(columns, OnceLock::new);
    // Reserve both lists before numerical work, so allocation failure cannot
    // replace an already observed input-ordered solver error.
    let mut results = Vec::new();
    results
        .try_reserve_exact(columns)
        .map_err(|_| CmgError::AllocationFailed {
            context: "contiguous queue ordered results",
        })?;
    let next = AtomicUsize::new(0);
    let first_failure = AtomicUsize::new(columns);
    workspaces.par_iter_mut().for_each(|workspace| {
        loop {
            let column = next.fetch_add(1, Ordering::Relaxed);
            if column >= columns || column >= first_failure.load(Ordering::Acquire) {
                break;
            }
            let result = operation(column, workspace);
            if result.is_err() {
                first_failure.fetch_min(column, Ordering::AcqRel);
            }
            // Unique monotonically claimed indices have exactly one writer.
            let _ = slots[column].set(result);
        }
    });
    for slot in slots {
        results.push(slot.into_inner().ok_or(CmgError::InvalidHierarchy {
            context: "missing contiguous queue result before first failure",
        })??);
    }
    Ok(results)
}

/// Reusable official-CMG workspaces for contiguous VCkss batches.
#[derive(Debug)]
pub struct VckssContiguousPcgWorkspace {
    workspaces: Vec<PcgWorkspace>,
}

impl VckssContiguousPcgWorkspace {
    fn new(solver: &ParallelPcgSolver) -> Self {
        Self {
            workspaces: vec![PcgWorkspace::new(solver.preconditioner())],
        }
    }

    fn ensure_count(&mut self, count: usize, solver: &ParallelPcgSolver) -> Result<(), CmgError> {
        self.workspaces
            .try_reserve_exact(count.saturating_sub(self.workspaces.len()))
            .map_err(|_| CmgError::AllocationFailed {
                context: "contiguous workspace growth",
            })?;
        while self.workspaces.len() < count {
            self.workspaces
                .push(PcgWorkspace::try_new(solver.preconditioner())?);
        }
        Ok(())
    }

    /// Return the retained workspace count.
    #[must_use]
    pub fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    /// Return bytes retained by the principal workspace arrays.
    #[must_use]
    pub fn byte_len(&self) -> usize {
        self.workspaces.iter().map(PcgWorkspace::byte_len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CmgOptions, Laplacian, ParallelOptions};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn queued_columns_are_ordered_across_odd_and_partial_batches() {
        for workers in [1, 2, 3, 4, 7, 14, 28, 64] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .unwrap();
            for columns in [0, 1, 2, 5, 8, 31, 32, 33, 65] {
                let mut scratch = vec![0_usize; workers];
                let result = pool
                    .install(|| {
                        map_workspace_queue(&mut scratch, columns, |index, count| {
                            *count += 1;
                            Ok(index * 3)
                        })
                    })
                    .unwrap();
                assert_eq!(
                    result,
                    (0..columns).map(|index| index * 3).collect::<Vec<_>>()
                );
                assert_eq!(scratch.iter().sum::<usize>(), columns);
            }
        }
    }

    #[test]
    fn queue_does_not_wait_for_a_slow_previous_wave() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let later_started = AtomicBool::new(false);
        let mut scratch = [(), ()];
        pool.install(|| {
            map_workspace_queue(&mut scratch, 4, |index, _| {
                if index == 0 {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                    while !later_started.load(Ordering::Acquire)
                        && std::time::Instant::now() < deadline
                    {
                        std::thread::yield_now();
                    }
                    assert!(
                        later_started.load(Ordering::Acquire),
                        "later work blocked on the previous wave"
                    );
                } else if index == 2 {
                    later_started.store(true, Ordering::Release);
                }
                Ok(index)
            })
        })
        .unwrap();
    }

    #[test]
    fn queue_allocation_overflow_is_typed() {
        assert_eq!(ParallelPcgSolver::vckss_queue_metadata_bytes(0).unwrap(), 0);
        assert!(ParallelPcgSolver::vckss_queue_metadata_bytes(usize::MAX).is_err());
        let each = std::mem::size_of::<OnceLock<Result<PcgResult, CmgError>>>()
            + std::mem::size_of::<PcgResult>();
        assert_eq!(
            ParallelPcgSolver::vckss_queue_metadata_bytes(33).unwrap(),
            33 * each
        );
        let error = map_workspace_queue(&mut [()], usize::MAX, |index, _| Ok(index)).unwrap_err();
        assert!(matches!(error, CmgError::AllocationFailed { .. }));
        assert!(map_workspace_queue::<(), usize, _>(&mut [], 1, |index, _| Ok(index)).is_err());
    }

    #[test]
    fn queue_returns_first_input_error_not_first_completed_error() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(3)
            .build()
            .unwrap();
        let later_failed = AtomicBool::new(false);
        let mut scratch = [(), (), ()];
        let error = pool
            .install(|| {
                map_workspace_queue(&mut scratch, 12, |index, _| {
                    if index == 0 {
                        let deadline =
                            std::time::Instant::now() + std::time::Duration::from_secs(3);
                        while !later_failed.load(Ordering::Acquire)
                            && std::time::Instant::now() < deadline
                        {
                            std::thread::yield_now();
                        }
                        return Err(CmgError::InvalidHierarchy {
                            context: "first input error",
                        });
                    }
                    if index == 1 {
                        later_failed.store(true, Ordering::Release);
                        return Err(CmgError::InvalidHierarchy {
                            context: "later input error",
                        });
                    }
                    Ok(index)
                })
            })
            .unwrap_err();
        assert!(matches!(
            error,
            CmgError::InvalidHierarchy {
                context: "first input error"
            }
        ));
    }

    fn path_solver(threads: usize) -> ParallelPcgSolver {
        let edges = (0..127)
            .map(|vertex| (vertex, vertex + 1, 1.0))
            .collect::<Vec<_>>();
        let graph = Laplacian::from_edges(128, edges).unwrap();
        ParallelPcgSolver::build(
            &graph,
            CmgOptions::default(),
            ParallelOptions {
                threads,
                workspace_memory_budget_bytes: None,
                ..ParallelOptions::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn contiguous_columns_match_normal_batch_and_preserve_order() {
        for threads in [1, 4] {
            let solver = path_solver(threads);
            let columns = (0..9)
                .map(|column| {
                    let mut rhs = vec![0.0; 128];
                    rhs[column + 1] = 1.0;
                    rhs[127 - column] = -1.0;
                    rhs
                })
                .collect::<Vec<_>>();
            let contiguous = columns.concat();
            let options = PcgOptions::default();
            let expected = solver.solve_batch(&columns, options).unwrap();
            let mut workspace = solver.vckss_contiguous_workspace();
            let actual = solver
                .vckss_solve_contiguous_columns_with_workspace(
                    &contiguous,
                    columns.len(),
                    options,
                    &mut workspace,
                )
                .unwrap();

            assert_eq!(actual.len(), expected.results().len());
            for (actual, expected) in actual.iter().zip(expected.results()) {
                assert_eq!(actual.solution(), expected.solution());
                assert_eq!(actual.iterations(), expected.iterations());
                assert_eq!(actual.residual_norm(), expected.residual_norm());
            }
            assert_eq!(
                workspace.workspace_count(),
                expected.report().concurrency().max(1)
            );
        }
    }

    #[test]
    fn cancelled_contiguous_batch_fails_before_column_work() {
        let solver = path_solver(4);
        let mut workspace = solver.vckss_contiguous_workspace();
        let cancellation = AtomicBool::new(true);
        let error = solver
            .vckss_solve_contiguous_columns_with_workspace_cancellable(
                &[0.0; 128],
                1,
                PcgOptions::default(),
                &mut workspace,
                &cancellation,
            )
            .expect_err("cancelled batch");
        assert_eq!(
            error,
            CmgError::Cancelled {
                phase: "vckss_batch_entry"
            }
        );
        cancellation.store(false, Ordering::Release);
        let result = solver
            .vckss_solve_contiguous_columns_with_workspace_cancellable(
                &[0.0; 128 * 7],
                7,
                PcgOptions::default(),
                &mut workspace,
                &cancellation,
            )
            .expect("reuse after cancellation");
        assert_eq!(result.len(), 7);
        assert_eq!(workspace.workspace_count(), 4);
    }

    #[test]
    fn contiguous_warm_starts_match_cold_certification_and_reduce_iterations() {
        for threads in [1, 4] {
            let solver = path_solver(threads);
            let mut rhs = vec![0.0; 128];
            rhs[3] = 1.0;
            rhs[121] = -1.0;
            let loose = PcgOptions {
                relative_tolerance: 1.0e-5,
                ..PcgOptions::default()
            };
            let tight = PcgOptions {
                relative_tolerance: 1.0e-10,
                ..PcgOptions::default()
            };
            let mut workspace = solver.vckss_contiguous_workspace();
            let first = solver
                .vckss_solve_contiguous_columns_with_workspace(&rhs, 1, loose, &mut workspace)
                .unwrap()
                .pop()
                .unwrap();
            let warm = solver
                .vckss_solve_contiguous_columns_from_initial_guesses_with_workspace(
                    &rhs,
                    first.solution(),
                    1,
                    tight,
                    &mut workspace,
                )
                .unwrap()
                .pop()
                .unwrap();
            let cold = solver
                .vckss_solve_contiguous_columns_with_workspace(&rhs, 1, tight, &mut workspace)
                .unwrap()
                .pop()
                .unwrap();
            assert!(warm.relative_residual() <= 1.0e-10);
            assert!(cold.relative_residual() <= 1.0e-10);
            assert!(warm.iterations() < cold.iterations());
        }
    }
}
