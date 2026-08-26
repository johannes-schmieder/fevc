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
            columns,
            options,
            workspace,
            Some(cancellation),
        )
    }

    fn vckss_solve_contiguous_columns_impl(
        &self,
        right_hand_sides: &[f64],
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

        let report = self.select_batch_execution(columns)?;
        workspace.ensure_count(report.concurrency().max(1), self);
        let solve_one = |rhs: &[f64], pcg_workspace: &mut PcgWorkspace| {
            crate::cancel::checkpoint(cancellation, "vckss_batch_column")?;
            match (report.execution(), cancellation) {
                (ParallelPcgExecution::Planned, Some(cancellation)) => {
                    solve_pcg_with_plan_and_workspace_cancellable(
                        self.graph(),
                        self.preconditioner(),
                        self.plan(),
                        rhs,
                        options,
                        pcg_workspace,
                        self.executor(),
                        cancellation,
                    )
                }
                (ParallelPcgExecution::Planned, None) => solve_pcg_with_plan_and_workspace(
                    self.graph(),
                    self.preconditioner(),
                    self.plan(),
                    rhs,
                    options,
                    pcg_workspace,
                    self.executor(),
                ),
                (
                    ParallelPcgExecution::Serial | ParallelPcgExecution::AcrossRightHandSides,
                    Some(cancellation),
                ) => solve_pcg_with_workspace_cancellable(
                    self.graph(),
                    self.preconditioner(),
                    rhs,
                    options,
                    pcg_workspace,
                    cancellation,
                ),
                (
                    ParallelPcgExecution::Serial | ParallelPcgExecution::AcrossRightHandSides,
                    None,
                ) => solve_pcg_with_workspace(
                    self.graph(),
                    self.preconditioner(),
                    rhs,
                    options,
                    pcg_workspace,
                ),
            }
        };

        match report.execution() {
            ParallelPcgExecution::Serial | ParallelPcgExecution::Planned => right_hand_sides
                .chunks_exact(dimension)
                .map(|rhs| solve_one(rhs, &mut workspace.workspaces[0]))
                .collect(),
            ParallelPcgExecution::AcrossRightHandSides => {
                let mut results = Vec::with_capacity(columns);
                for rhs_chunk in right_hand_sides.chunks(dimension * report.concurrency()) {
                    crate::cancel::checkpoint(cancellation, "vckss_batch_chunk")?;
                    let chunk_columns = rhs_chunk.len() / dimension;
                    let chunk_results: Vec<Result<PcgResult, CmgError>> =
                        self.executor().install(|| {
                            workspace.workspaces[..chunk_columns]
                                .par_iter_mut()
                                .zip(rhs_chunk.par_chunks_exact(dimension))
                                .map(|(pcg_workspace, rhs)| solve_one(rhs, pcg_workspace))
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
}
