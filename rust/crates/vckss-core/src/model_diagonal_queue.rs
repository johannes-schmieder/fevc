// SPDX-License-Identifier: GPL-3.0-only

//! Isolated, admitted execution for independent diagonal model RHSs.
//! No production routing selects this executor yet. In particular it does not
//! reinterpret V5 threads or replace a requested preconditioner with CMG.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::{
    column_context, rhs_column, validate_batch_rhs, ModelBatchSolve, ModelSolve,
    ModelSolverOptions, ModelSolverRoute, PreparedModelSolver,
};
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{CancellationToken, InterruptCheck};
use crate::memory::MemoryBudget;
use crate::model_operator::reserve_exact;

const PHASE: &str = "model_diagonal_queue";
const STACK_BYTES: usize = 2 * 1024 * 1024;
// Conservative runtime heap allowance, not a measured Rayon allocation or RSS.
const RUNTIME_BYTES_PER_WORKER: usize = 128 * 1024;
type Slot = Mutex<Option<Result<ModelSolve>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagonalQueuePlan {
    pub permitted_threads: usize,
    pub workers: usize,
    pub maximum_rhs: usize,
    pub workspace_bytes_per_worker: u64,
    pub output_bytes: u64,
    pub queue_metadata_bytes: u64,
    pub runtime_allowance_bytes: u64,
    pub stack_reservation_bytes: u64,
    pub incremental_peak_bytes: u64,
    pub command_peak_bytes: u64,
}

fn resource(message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, PHASE, message)
}

fn add(left: usize, right: usize) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| resource("byte-count addition overflow"))
}

fn mul(left: usize, right: usize) -> Result<usize> {
    left.checked_mul(right)
        .filter(|value| *value <= isize::MAX as usize)
        .ok_or_else(|| resource("allocation byte-count overflow"))
}

impl DiagonalQueuePlan {
    /// `other_live_bytes` must include the prepared model, borrowed input
    /// buffers, any retained results from earlier calls, and every other
    /// caller-owned live allocation throughout the executor's use. The increment
    /// counts this executor's outputs, queue, runtime and concurrent scratch.
    pub fn checked(
        workers: usize,
        firms: usize,
        controls: usize,
        permitted_threads: usize,
        maximum_rhs: usize,
        other_live_bytes: u64,
    ) -> Result<Self> {
        if permitted_threads == 0 || maximum_rhs == 0 {
            return Err(BackendError::invalid(
                PHASE,
                "threads and RHS capacity must be positive",
            ));
        }
        let threads = permitted_threads.min(maximum_rhs);
        let original = add(add(workers, firms)?, controls)?;
        let reduced = add(firms, controls)?;
        // Upper envelope over reduced PCG, recovery and complete W+F+Q
        // residual certification. These lifetimes are summed conservatively;
        // no observation-by-RHS matrix is allocated by the solver.
        let workspace = add(mul(add(mul(reduced, 16)?, mul(original, 16)?)?, 8)?, 1024)?;
        let output = mul(
            maximum_rhs,
            add(mul(original, 16)?, size_of::<ModelSolve>())?,
        )?;
        // Collection temporarily retains both queue slots and output headers.
        // Coefficient/residual buffers move; their payload is counted once.
        let statistical_metadata =
            usize::try_from(crate::ordered_work::metadata_bytes(maximum_rhs)?)
                .map_err(|_| resource("statistical metadata size overflow"))?;
        let metadata = add(mul(maximum_rhs, size_of::<Slot>())?, statistical_metadata)?;
        let runtime = if threads == 1 {
            0
        } else {
            mul(threads, RUNTIME_BYTES_PER_WORKER)?
        };
        // Only the owned Rayon workers need new stacks. The scoped dispatch
        // and host interruption checks stay on the existing caller thread.
        let stack = if threads == 1 {
            0
        } else {
            mul(threads, STACK_BYTES)?
        };
        let incremental = add(
            add(add(mul(threads, workspace)?, output)?, metadata)?,
            add(runtime, stack)?,
        )?;
        let incremental =
            u64::try_from(incremental).map_err(|_| resource("peak byte count overflow"))?;
        Ok(Self {
            permitted_threads,
            workers: threads,
            maximum_rhs,
            workspace_bytes_per_worker: workspace as u64,
            output_bytes: output as u64,
            queue_metadata_bytes: metadata as u64,
            runtime_allowance_bytes: runtime as u64,
            stack_reservation_bytes: stack as u64,
            incremental_peak_bytes: incremental,
            command_peak_bytes: other_live_bytes
                .checked_add(incremental)
                .ok_or_else(|| resource("command peak overflow"))?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagonalQueueReceipt {
    pub columns: usize,
    pub active_worker_limit: usize,
    pub maximum_active_workers: usize,
    pub retained_queue_metadata_bytes: u64,
}

/// Reuses an owned bounded thread pool, but not PCG scratch buffers. Scratch
/// remains fallibly allocated per RHS and is bounded by the active workers.
#[derive(Debug)]
pub struct DiagonalBatchExecutor<'solver, 'data> {
    solver: &'solver PreparedModelSolver<'data>,
    runtime: DiagonalQueueRuntime,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DiagonalQueueWorkReceipt {
    pub completed_rhs: usize,
    pub completed_batches: usize,
    pub parallel_batches: usize,
    pub maximum_active_workers: usize,
}

/// One runtime shared by the full and FE-only solvers; it borrows neither.
#[derive(Debug)]
pub(crate) struct DiagonalQueueRuntime {
    dimensions: [usize; 3],
    plan: DiagonalQueuePlan,
    pool: Option<rayon::ThreadPool>,
    // Serialize calls on one executor: simultaneous scratch and queue storage
    // must not multiply. Returned results become caller-owned live memory.
    call_lock: Mutex<()>,
    work: Mutex<DiagonalQueueWorkReceipt>,
}

impl<'solver, 'data> DiagonalBatchExecutor<'solver, 'data> {
    pub fn new(
        solver: &'solver PreparedModelSolver<'data>,
        threads: usize,
        maximum_rhs: usize,
        other_live_bytes: u64,
        budget: MemoryBudget,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if solver.receipt.selected != ModelSolverRoute::Diagonal || solver.direct.is_some() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                PHASE,
                "the diagonal executor requires an already-selected diagonal model solver",
            ));
        }
        let runtime = DiagonalQueueRuntime::new(
            [
                solver.operator.workers(),
                solver.operator.firms(),
                solver.operator.controls(),
            ],
            threads,
            maximum_rhs,
            other_live_bytes,
            budget,
            interrupt,
        )?;
        Ok(Self { solver, runtime })
    }

    #[must_use]
    pub const fn plan(&self) -> DiagonalQueuePlan {
        self.runtime.plan
    }

    pub fn solve_batch_with_interrupt(
        &self,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        control_rhs: &[f64],
        columns: usize,
        options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<(ModelBatchSolve, DiagonalQueueReceipt)> {
        self.runtime.solve_batch_with_interrupt(
            self.solver,
            worker_rhs,
            firm_rhs,
            control_rhs,
            columns,
            options,
            interrupt,
        )
    }
}

impl DiagonalQueueRuntime {
    pub(crate) fn statistical_work<I, Iter, F>(
        &self,
        input: Iter,
        phase: &'static str,
        operation: F,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()>
    where
        I: Send,
        Iter: ExactSizeIterator<Item = I>,
        F: Fn(usize, I, &mut dyn InterruptCheck) -> Result<()> + Sync,
    {
        let _guard = self.call_lock.lock().map_err(|_| poisoned())?;
        crate::ordered_work::run(
            self.pool.as_ref().map(crate::ordered_work::Pool::Rayon),
            self.plan.maximum_rhs,
            input,
            phase,
            operation,
            interrupt,
        )
    }

    pub(crate) fn new(
        dimensions: [usize; 3],
        threads: usize,
        maximum_rhs: usize,
        other_live_bytes: u64,
        budget: MemoryBudget,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if budget == MemoryBudget::Legacy {
            return Err(BackendError::invalid(
                PHASE,
                "use an explicit or unspecified budget",
            ));
        }
        let plan = DiagonalQueuePlan::checked(
            dimensions[0],
            dimensions[1],
            dimensions[2],
            threads,
            maximum_rhs,
            other_live_bytes,
        )?;
        budget.admit(plan.command_peak_bytes, 0, PHASE)?;
        interrupt.checkpoint("model_diagonal_queue_admitted")?;
        let pool = if plan.workers == 1 {
            None
        } else {
            Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(plan.workers)
                    .stack_size(STACK_BYTES)
                    .thread_name(|index| format!("fevc-diagonal-{index}"))
                    .build()
                    .map_err(|error| {
                        BackendError::new(
                            ErrorCode::AllocationFailed,
                            PHASE,
                            format!("could not create diagonal worker pool: {error}"),
                        )
                    })?,
            )
        };
        Ok(Self {
            dimensions,
            plan,
            pool,
            call_lock: Mutex::new(()),
            work: Mutex::new(DiagonalQueueWorkReceipt::default()),
        })
    }

    #[must_use]
    pub(crate) const fn plan(&self) -> DiagonalQueuePlan {
        self.plan
    }

    pub(crate) fn work_receipt(&self) -> Result<DiagonalQueueWorkReceipt> {
        Ok(*self.work.lock().map_err(|_| poisoned())?)
    }

    pub(crate) fn solve_batch_with_interrupt(
        &self,
        solver: &PreparedModelSolver<'_>,
        worker_rhs: &[f64],
        firm_rhs: &[f64],
        control_rhs: &[f64],
        columns: usize,
        options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<(ModelBatchSolve, DiagonalQueueReceipt)> {
        // The caller supplies its phase options unchanged, including stricter
        // rank/inference options. This executor never selects a tolerance.
        options.validate()?;
        if solver.receipt.selected != ModelSolverRoute::Diagonal
            || solver.direct.is_some()
            || solver.operator.workers() != self.dimensions[0]
            || solver.operator.firms() != self.dimensions[1]
            || solver.operator.controls() > self.dimensions[2]
        {
            return Err(BackendError::invariant(
                PHASE,
                "solver differs from the admitted diagonal dimensions/route",
            ));
        }
        if columns > self.plan.maximum_rhs {
            return Err(resource("RHS count exceeds the pre-admitted capacity"));
        }
        if columns == 0 {
            if !worker_rhs.is_empty() || !firm_rhs.is_empty() || !control_rhs.is_empty() {
                return Err(BackendError::invalid(
                    PHASE,
                    "zero work has nonempty RHS buffers",
                ));
            }
            interrupt.checkpoint(PHASE)?;
            return Ok((
                ModelBatchSolve {
                    columns,
                    solution: Vec::new(),
                },
                DiagonalQueueReceipt {
                    columns,
                    active_worker_limit: 0,
                    maximum_active_workers: 0,
                    retained_queue_metadata_bytes: 0,
                },
            ));
        }
        validate_batch_rhs(
            &solver.operator,
            worker_rhs,
            firm_rhs,
            control_rhs,
            columns,
            columns,
            interrupt,
        )?;
        let _call = loop {
            interrupt.checkpoint(PHASE)?;
            match self.call_lock.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::WouldBlock) => {
                    std::thread::park_timeout(Duration::from_millis(1))
                }
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(BackendError::new(
                        ErrorCode::ContextPoisoned,
                        PHASE,
                        "diagonal call lock poisoned",
                    ))
                }
            }
        };
        let mut slots = Vec::new();
        reserve_exact(&mut slots, columns, "diagonal queue slots")?;
        let retained_metadata = mul(slots.capacity(), size_of::<Slot>())? as u64;
        if retained_metadata > self.plan.queue_metadata_bytes {
            return Err(resource("retained queue capacity exceeds admission"));
        }
        for _ in 0..columns {
            slots.push(Mutex::new(None));
        }
        let mut solution = Vec::new();
        reserve_exact(&mut solution, columns, "diagonal queue outputs")?;
        if solution.capacity() > self.plan.maximum_rhs {
            return Err(resource(
                "retained output header capacity exceeds admission",
            ));
        }
        let operation = |column, check: &mut dyn InterruptCheck| {
            let operator = &solver.operator;
            let mut result = solver
                .solve_batch_serial_with_options_and_interrupt(
                    rhs_column(worker_rhs, column, operator.workers()),
                    rhs_column(firm_rhs, column, operator.firms()),
                    rhs_column(control_rhs, column, operator.controls()),
                    1,
                    1,
                    options,
                    check,
                )
                .map_err(|error| column_context(error, column))?;
            result
                .solution
                .pop()
                .ok_or_else(|| BackendError::invariant(PHASE, "missing scalar result"))
        };
        let peak = AtomicUsize::new(0);
        // A dependent single-RHS action has no parallel work. Keep it on the
        // caller instead of paying a cross-thread handoff and wake-up. This
        // structural choice does not depend on timing or solve outcomes.
        if let Some(pool) = self.pool.as_ref().filter(|_| columns > 1) {
            run_parallel(
                pool,
                &slots,
                self.plan.workers.min(columns),
                &peak,
                operation,
                interrupt,
            )?;
        } else {
            for (column, slot) in slots.iter().enumerate() {
                interrupt.checkpoint(PHASE)?;
                *slot.lock().map_err(|_| poisoned())? = Some(operation(column, interrupt));
                if slot
                    .lock()
                    .map_err(|_| poisoned())?
                    .as_ref()
                    .is_some_and(Result::is_err)
                {
                    break;
                }
            }
            peak.store(1, Ordering::Relaxed);
        }
        for slot in slots {
            interrupt.checkpoint("model_diagonal_queue_collect")?;
            solution.push(
                slot.into_inner().map_err(|_| poisoned())?.ok_or_else(|| {
                    BackendError::invariant(PHASE, "missing ordered queue result")
                })??,
            );
        }
        let maximum_active_workers = peak.load(Ordering::Relaxed);
        let mut work = self.work.lock().map_err(|_| poisoned())?;
        work.completed_rhs = work
            .completed_rhs
            .checked_add(columns)
            .ok_or_else(|| resource("queue RHS receipt overflow"))?;
        work.completed_batches = work
            .completed_batches
            .checked_add(1)
            .ok_or_else(|| resource("queue batch receipt overflow"))?;
        work.parallel_batches = work
            .parallel_batches
            .checked_add(usize::from(maximum_active_workers > 1))
            .ok_or_else(|| resource("queue parallel receipt overflow"))?;
        work.maximum_active_workers = work.maximum_active_workers.max(maximum_active_workers);
        Ok((
            ModelBatchSolve { columns, solution },
            DiagonalQueueReceipt {
                columns,
                active_worker_limit: columns.min(self.plan.workers),
                maximum_active_workers,
                retained_queue_metadata_bytes: retained_metadata,
            },
        ))
    }
}

fn poisoned() -> BackendError {
    BackendError::new(
        ErrorCode::ContextPoisoned,
        PHASE,
        "diagonal result slot poisoned",
    )
}

struct WorkerInterrupt<'a> {
    local: &'a CancellationToken,
    parent: Option<&'a CancellationToken>,
}

impl InterruptCheck for WorkerInterrupt<'_> {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if self.local.is_cancelled() || self.parent.is_some_and(CancellationToken::is_cancelled) {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "diagonal solve cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

struct CallerInterrupt<'a> {
    host: &'a mut dyn InterruptCheck,
    worker: WorkerInterrupt<'a>,
    failure: &'a mut Option<BackendError>,
    next_poll: Instant,
}

impl InterruptCheck for CallerInterrupt<'_> {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        self.worker.checkpoint(phase)?;
        let now = Instant::now();
        if now >= self.next_poll {
            // Match the existing 1 ms host-poll interval. Forwarding every
            // scalar-kernel checkpoint to Stata would make caller work much
            // more expensive than worker work. Tokens are still checked on
            // every checkpoint; timing never selects numerical work or routing.
            self.next_poll = now + Duration::from_millis(1);
            if let Err(error) = self.host.checkpoint(phase) {
                self.worker.local.cancel();
                *self.failure = Some(error.clone());
                return Err(error);
            }
        }
        Ok(())
    }
}

// Wake the caller even if a worker unwinds. The release/acquire pair publishes
// completion before the caller decides whether to park; an early unpark leaves
// a token, so completion cannot be lost between that decision and parking.
struct WorkerCompletion<'a> {
    remaining: &'a AtomicUsize,
    caller: &'a std::thread::Thread,
}

impl Drop for WorkerCompletion<'_> {
    fn drop(&mut self) {
        self.remaining.fetch_sub(1, Ordering::Release);
        self.caller.unpark();
    }
}

fn run_parallel(
    pool: &rayon::ThreadPool,
    slots: &[Slot],
    workers: usize,
    peak: &AtomicUsize,
    operation: impl Fn(usize, &mut dyn InterruptCheck) -> Result<ModelSolve> + Sync,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let local = CancellationToken::new();
    let parent = interrupt.cancellation_token();
    let next = AtomicUsize::new(0);
    let first_error = AtomicUsize::new(slots.len());
    let active = AtomicUsize::new(0);
    // The caller executes one worker's share. Spawning only the other shares
    // bounds active solves by `workers` and overlaps useful work with wake-up.
    let remaining = AtomicUsize::new(workers.saturating_sub(1));
    let caller = std::thread::current();
    let mut caller_error = None;
    let run_worker = |check: &mut dyn InterruptCheck| {
        loop {
            let column = next.fetch_add(1, Ordering::Relaxed);
            if column >= slots.len()
                || column > first_error.load(Ordering::Acquire)
                || check.checkpoint(PHASE).is_err()
            {
                break;
            }
            let running = active.fetch_add(1, Ordering::Relaxed) + 1;
            peak.fetch_max(running, Ordering::Relaxed);
            let result = catch_unwind(AssertUnwindSafe(|| operation(column, check)))
                .unwrap_or_else(|_| {
                    Err(BackendError::new(
                        ErrorCode::Panic,
                        PHASE,
                        format!("diagonal worker panicked at zero-based column {column}"),
                    ))
                });
            active.fetch_sub(1, Ordering::Relaxed);
            if result.is_err() {
                first_error.fetch_min(column, Ordering::AcqRel);
            }
            // A later numerical failure must not cancel an earlier column.
            *slots[column]
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(result);
        }
    };
    let scoped = catch_unwind(AssertUnwindSafe(|| {
        // Unlike pool.scope, in_place_scope keeps this closure on the caller.
        // Host callbacks must never run on a Rayon worker. Reusing the owned
        // pool also avoids an OS coordinator spawn for every small batch.
        pool.in_place_scope(|scope| {
            for _ in 1..workers {
                scope.spawn(|_| {
                    let _completion = WorkerCompletion {
                        remaining: &remaining,
                        caller: &caller,
                    };
                    let mut check = WorkerInterrupt {
                        local: &local,
                        parent: parent.as_ref(),
                    };
                    run_worker(&mut check);
                });
            }
            run_worker(&mut CallerInterrupt {
                host: interrupt,
                worker: WorkerInterrupt {
                    local: &local,
                    parent: parent.as_ref(),
                },
                failure: &mut caller_error,
                next_poll: Instant::now(),
            });
            loop {
                if caller_error.is_none() {
                    if let Err(error) = interrupt.checkpoint("model_diagonal_queue_wait") {
                        local.cancel();
                        caller_error = Some(error);
                    }
                }
                if remaining.load(Ordering::Acquire) == 0 {
                    break;
                }
                // Periodic polling is only for host cancellation during a
                // slow solve. Worker completion wakes this wait immediately.
                std::thread::park_timeout(Duration::from_millis(1));
            }
        });
    }));
    if let Some(error) = caller_error {
        return Err(error);
    }
    scoped.map_err(|_| BackendError::new(ErrorCode::Panic, PHASE, "diagonal scope panicked"))?;
    WorkerInterrupt {
        local: &local,
        parent: parent.as_ref(),
    }
    .checkpoint(PHASE)?;
    interrupt.checkpoint(PHASE)
}

#[cfg(test)]
#[path = "model_diagonal_queue_tests.rs"]
mod tests;
