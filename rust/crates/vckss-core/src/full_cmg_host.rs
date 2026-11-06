// SPDX-License-Identifier: MIT
//! FEVC-owned scheduling and storage around the unmodified CMG library.

use cmg::{
    solve_pcg_batch_into, BuildContext, CmgError, CmgOptions, CmgPreconditioner, CmgStorageReport,
    Laplacian, ParallelCmgPlan, ParallelOptions, ParallelPcgExecution, ParallelPcgPolicy,
    ParallelPcgSolver, PcgBatchMut, PcgBatchRef, PcgBatchWorkspace, PcgDiagnostics, PcgOptions,
    SolveContext, Team,
};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;

#[derive(Debug, Default)]
struct Workers(Vec<std::thread::JoinHandle<()>>);

impl Drop for Workers {
    fn drop(&mut self) {
        for worker in self.0.drain(..) {
            let _ = worker.join();
        }
    }
}

#[derive(Debug)]
pub(crate) struct HostPool {
    pool: Option<rayon::ThreadPool>,
    workers: Workers,
    options: ParallelOptions,
}

impl Drop for HostPool {
    fn drop(&mut self) {
        // Signal Rayon before the worker guard joins, including TLS destruction.
        drop(self.pool.take());
    }
}

impl HostPool {
    pub(crate) fn new(options: ParallelOptions) -> Result<Self, CmgError> {
        let mut options = options.validate()?;
        if options.threads == 0 {
            options.threads = std::thread::available_parallelism().map_or(1, usize::from);
        }
        let mut workers = Workers::default();
        workers
            .0
            .try_reserve_exact(options.threads)
            .map_err(|_| allocation("FEVC CMG worker handles"))?;
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(options.threads)
            .stack_size(STACK_BYTES)
            .thread_name(|index| format!("fevc-cmg-{index}"))
            .spawn_handler(|thread| {
                let mut builder = std::thread::Builder::new();
                if let Some(name) = thread.name() {
                    builder = builder.name(name.to_owned());
                }
                if let Some(bytes) = thread.stack_size() {
                    builder = builder.stack_size(bytes);
                }
                workers.0.push(builder.spawn(|| thread.run())?);
                Ok(())
            })
            .build()
            .map_err(|error| CmgError::ParallelRuntime {
                message: error.to_string(),
            })?;
        Ok(Self {
            pool: Some(pool),
            workers,
            options,
        })
    }

    fn pool(&self) -> &rayon::ThreadPool {
        self.pool.as_ref().expect("live FEVC-owned pool")
    }

    pub(crate) fn graph<I>(
        &self,
        vertices: usize,
        edges: I,
        cancel: Option<&AtomicBool>,
    ) -> Result<Laplacian, CmgError>
    where
        I: IntoIterator<Item = (usize, usize, f64)>,
        I::IntoIter: ExactSizeIterator,
    {
        let query = || cancel.is_some_and(|flag| flag.load(Ordering::Acquire));
        Laplacian::from_edges_with(
            vertices,
            edges,
            BuildContext {
                team: Some(self.team()),
                cancel: Some(&query),
                budget_bytes: None,
            },
        )
    }

    fn team(&self) -> Team<'_> {
        Team::new(self.pool(), self.options.threads, self.options).expect("validated host team")
    }
}

// Match FEVC's owned diagonal executor policy. These are planning allowances,
// not a measured or proved bound on Rayon/OS memory.
pub(crate) const STACK_BYTES: usize = 2 * 1024 * 1024;
const RUNTIME_BYTES_PER_WORKER: usize = 128 * 1024;

pub(crate) fn runtime_bytes(threads: usize) -> Result<usize, CmgError> {
    threads
        .checked_mul(
            STACK_BYTES
                + RUNTIME_BYTES_PER_WORKER
                + std::mem::size_of::<std::thread::JoinHandle<()>>(),
        )
        .ok_or_else(|| allocation("FEVC CMG runtime allowance"))
}

#[derive(Debug)]
pub(crate) struct BatchReport {
    execution: ParallelPcgExecution,
    concurrency: usize,
    workspace_each: usize,
    workspace_pool: usize,
}
impl BatchReport {
    pub(crate) fn execution(&self) -> ParallelPcgExecution {
        self.execution
    }
    pub(crate) fn concurrency(&self) -> usize {
        self.concurrency
    }
    pub(crate) fn workspace_bytes_each(&self) -> usize {
        self.workspace_each
    }
    pub(crate) fn workspace_pool_bytes(&self) -> usize {
        self.workspace_pool
    }
}

fn allocation(context: &'static str) -> CmgError {
    CmgError::AllocationFailed { context }
}

fn checkpoint(cancel: Option<&AtomicBool>, phase: &'static str) -> Result<(), CmgError> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        Err(CmgError::Cancelled { phase })
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ColumnResult {
    solution: Vec<f64>,
    diagnostics: PcgDiagnostics,
}

impl ColumnResult {
    pub(crate) fn iterations(&self) -> usize {
        self.diagnostics.iterations()
    }
    pub(crate) fn restarts(&self) -> usize {
        self.diagnostics.restarts()
    }
    pub(crate) fn initial_residual_norm(&self) -> f64 {
        self.diagnostics.initial_residual_norm()
    }
    pub(crate) fn into_solution(self) -> Vec<f64> {
        self.solution
    }
}

#[derive(Debug, Default)]
pub(crate) struct WorkspacePool {
    workspaces: Vec<PcgBatchWorkspace>,
}

impl WorkspacePool {
    pub(crate) fn byte_len(&self) -> Result<usize, CmgError> {
        CmgStorageReport::measure(&[&self.workspaces])?.total_bytes()
    }

    pub(crate) fn workspace_count(&self) -> usize {
        self.workspaces.len()
    }

    fn ensure_count(
        &mut self,
        count: usize,
        solver: &HostCmg,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), CmgError> {
        if self.workspace_count() >= count {
            return Ok(());
        }
        if !self.workspaces.is_empty() {
            return Err(CmgError::InvalidHierarchy {
                context: "FEVC CMG solve exceeds admitted workspace bank",
            });
        }
        let query = || cancel.is_some_and(|flag| flag.load(Ordering::Acquire));
        self.workspaces = PcgBatchWorkspace::new_bank_with(
            solver.preconditioner(),
            count,
            BuildContext {
                cancel: Some(&query),
                budget_bytes: solver.runtime.options.workspace_memory_budget_bytes,
                ..BuildContext::default()
            },
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct HostCmg {
    solver: ParallelPcgSolver,
    runtime: HostPool,
}

impl HostCmg {
    pub(crate) fn build(
        graph: &Laplacian,
        options: CmgOptions,
        runtime: HostPool,
        cancel: Option<&AtomicBool>,
    ) -> Result<Self, CmgError> {
        checkpoint(cancel, "FEVC CMG pool construction")?;

        let query = || cancel.is_some_and(|flag| flag.load(Ordering::Acquire));
        let context = BuildContext {
            team: Some(runtime.team()),
            cancel: Some(&query),
            budget_bytes: None,
        };
        let preconditioner = CmgPreconditioner::build_with(graph, options.into(), context)?;
        let solver = ParallelPcgSolver::from_preconditioner(
            preconditioner,
            runtime.team(),
            ParallelPcgPolicy::default(),
        )?;
        solver.try_plan_with(context)?;
        Ok(Self { solver, runtime })
    }

    pub(crate) fn preconditioner(&self) -> &CmgPreconditioner {
        self.solver.preconditioner()
    }
    pub(crate) fn graph(&self) -> &Laplacian {
        self.solver.graph()
    }
    pub(crate) fn plan(&self) -> &ParallelCmgPlan {
        self.solver.initialized_plan().expect("prepared CMG plan")
    }
    pub(crate) fn select_batch_execution(&self, columns: usize) -> Result<BatchReport, CmgError> {
        let each = PcgBatchWorkspace::bank_allocation_bytes(self.preconditioner(), 1)?;
        let maximum = self
            .runtime
            .options
            .workspace_memory_budget_bytes
            .map_or(columns, |bytes| bytes / each.max(1));
        if columns > 0 && maximum == 0 {
            return Err(CmgError::MemoryBudgetExceeded {
                required_bytes: each,
                budget_bytes: self.runtime.options.workspace_memory_budget_bytes.unwrap(),
            });
        }
        let selected = self
            .solver
            .select_batch_execution(columns.min(maximum), self.runtime.team())?;
        let count = selected.concurrency();
        Ok(BatchReport {
            execution: selected.execution(),
            concurrency: count,
            workspace_each: each,
            workspace_pool: PcgBatchWorkspace::bank_allocation_bytes(self.preconditioner(), count)?,
        })
    }

    pub(crate) fn storage_bytes(&self) -> Result<(usize, usize), CmgError> {
        let hierarchy = CmgStorageReport::measure(&[self.preconditioner()])?.total_bytes()?;
        let total =
            CmgStorageReport::measure(&[self.preconditioner(), self.plan()])?.total_bytes()?;
        Ok((
            hierarchy,
            total
                .checked_sub(hierarchy)
                .ok_or_else(|| allocation("FEVC CMG ownership union"))?,
        ))
    }

    pub(crate) fn runtime_bytes(&self) -> Result<usize, CmgError> {
        runtime_bytes(self.runtime.workers.0.capacity())
    }

    pub(crate) fn queue_metadata_bytes(columns: usize) -> Result<usize, CmgError> {
        columns
            .checked_mul(
                std::mem::size_of::<OnceLock<Result<ColumnResult, CmgError>>>()
                    + std::mem::size_of::<ColumnResult>(),
            )
            .ok_or_else(|| allocation("FEVC CMG queue metadata overflow"))
    }

    pub(crate) fn prepared_workspace(
        &self,
        count: usize,
        cancel: Option<&AtomicBool>,
    ) -> Result<WorkspacePool, CmgError> {
        let mut pool = WorkspacePool::default();
        pool.ensure_count(count, self, cancel)?;
        if pool.byte_len()?
            > PcgBatchWorkspace::bank_allocation_bytes(self.preconditioner(), count)?
        {
            return Err(CmgError::InvalidHierarchy {
                context: "FEVC CMG workspace exceeds admitted layout",
            });
        }
        Ok(pool)
    }

    pub(crate) fn map_ordered<Input, Output, Operation>(
        &self,
        input: Vec<Input>,
        operation: Operation,
    ) -> Vec<Output>
    where
        Input: Send,
        Output: Send,
        Operation: Fn(Input) -> Output + Send + Sync,
    {
        self.runtime
            .pool()
            .install(|| input.into_par_iter().map(operation).collect())
    }

    pub(crate) fn solve_columns(
        &self,
        rhs: &[f64],
        guesses: Option<&[f64]>,
        columns: usize,
        options: PcgOptions,
        workspace: &mut WorkspacePool,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<ColumnResult>, CmgError> {
        checkpoint(cancel, "FEVC CMG batch entry")?;
        let n = self.graph().vertex_count();
        let expected = n
            .checked_mul(columns)
            .ok_or_else(|| allocation("FEVC CMG batch dimensions"))?;
        if columns == 0
            || rhs.len() != expected
            || guesses.is_some_and(|values| values.len() != expected)
        {
            return Err(CmgError::InvalidHierarchy {
                context: "FEVC CMG batch shape mismatch",
            });
        }
        let report = self.select_batch_execution(columns)?;
        workspace.ensure_count(report.concurrency().max(1), self, cancel)?;
        let query = || cancel.is_some_and(|flag| flag.load(Ordering::Acquire));
        let team = self.runtime.team();
        let planned = report.execution() == ParallelPcgExecution::Planned;
        let context = SolveContext {
            plan: planned.then(|| self.plan()),
            team: Some(if planned { team } else { team.single_lane() }),
            cancel: Some(&query),
            ..SolveContext::default()
        };
        let operation = |column: usize, scratch: &mut PcgBatchWorkspace| {
            checkpoint(cancel, "FEVC CMG batch column")?;
            let mut solution = Vec::new();
            solution
                .try_reserve_exact(n)
                .map_err(|_| allocation("FEVC CMG output column"))?;
            while solution.len() < n {
                checkpoint(cancel, "FEVC CMG output preparation")?;
                solution.resize(n.min(solution.len().saturating_add(4096)), 0.0);
            }
            let begin = column * n;
            let mut diagnostics = [PcgDiagnostics::default()];
            solve_pcg_batch_into(
                self.graph(),
                self.preconditioner(),
                PcgBatchRef::contiguous(&rhs[begin..begin + n], 1, n)?,
                guesses
                    .map(|values| PcgBatchRef::contiguous(&values[begin..begin + n], 1, n))
                    .transpose()?,
                PcgBatchMut::contiguous(&mut solution, 1, n)?,
                &mut diagnostics,
                options,
                std::slice::from_mut(scratch),
                context,
            )?;
            let diagnostics = diagnostics[0];
            Ok(ColumnResult {
                solution,
                diagnostics,
            })
        };
        let result = if report.execution() == ParallelPcgExecution::AcrossRightHandSides {
            self.runtime.pool().install(|| {
                map_workspace_queue(
                    &mut workspace.workspaces[..report.concurrency()],
                    columns,
                    operation,
                )
            })
        } else {
            map_workspace_queue_serial(&mut workspace.workspaces[0], columns, operation)
        };
        checkpoint(cancel, "FEVC CMG batch publication")?;
        result
    }
}

fn map_workspace_queue_serial<Workspace, Output>(
    workspace: &mut Workspace,
    columns: usize,
    operation: impl Fn(usize, &mut Workspace) -> Result<Output, CmgError>,
) -> Result<Vec<Output>, CmgError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(columns)
        .map_err(|_| allocation("FEVC CMG ordered results"))?;
    for column in 0..columns {
        result.push(operation(column, workspace)?);
    }
    Ok(result)
}

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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(HostCmg::queue_metadata_bytes(0).unwrap(), 0);
        assert!(HostCmg::queue_metadata_bytes(usize::MAX).is_err());
        let each = std::mem::size_of::<OnceLock<Result<ColumnResult, CmgError>>>()
            + std::mem::size_of::<ColumnResult>();
        assert_eq!(HostCmg::queue_metadata_bytes(33).unwrap(), 33 * each);
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
}
