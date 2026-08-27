//! VCkss-owned contiguous-column bridge for independent CMG PCG solves.
//!
//! The estimator prepares repeated right-hand sides in one column-major
//! allocation.  This bridge preserves CMG's scalar PCG recurrences and its
//! normal serial/planned/across-RHS routing while avoiding an intermediate
//! `Vec<Vec<f64>>` allocation and copy.

use rayon::prelude::*;
use std::sync::atomic::AtomicBool;

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
        workspace.ensure_count(report.concurrency().max(1), self);
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
            ParallelPcgExecution::AcrossRightHandSides => {
                let mut results = Vec::with_capacity(columns);
                for (chunk_index, rhs_chunk) in right_hand_sides
                    .chunks(dimension * report.concurrency())
                    .enumerate()
                {
                    crate::cancel::checkpoint(cancellation, "vckss_batch_chunk")?;
                    let chunk_columns = rhs_chunk.len() / dimension;
                    let first_column = chunk_index * report.concurrency();
                    let chunk_results: Vec<Result<PcgResult, CmgError>> =
                        self.executor().install(|| {
                            workspace.workspaces[..chunk_columns]
                                .par_iter_mut()
                                .enumerate()
                                .map(|(offset, pcg_workspace)| {
                                    let rhs_begin = offset * dimension;
                                    let rhs = &rhs_chunk[rhs_begin..rhs_begin + dimension];
                                    let initial_begin = (first_column + offset) * dimension;
                                    let initial_guess = initial_guesses.map(|guesses| {
                                        &guesses[initial_begin..initial_begin + dimension]
                                    });
                                    solve_one(rhs, initial_guess, pcg_workspace)
                                })
                                .collect()
                        });
                    for result in chunk_results {
                        results.push(result?);
                    }
                }
                Ok(results)
            }
        }
    }
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

    fn ensure_count(&mut self, count: usize, solver: &ParallelPcgSolver) {
        self.workspaces.extend(
            (self.workspaces.len()..count).map(|_| PcgWorkspace::new(solver.preconditioner())),
        );
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
