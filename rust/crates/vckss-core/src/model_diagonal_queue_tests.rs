// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use crate::interrupt::NeverInterrupt;
use crate::memory::MemoryCheck;
use crate::model_operator::CanonicalModelData;
use crate::model_solver::ModelRoutingOptions;

const WORKERS: usize = 21;
const FIRMS: usize = 11;
const COLUMNS: usize = 17;

struct Fixture {
    worker: Vec<u32>,
    firm: Vec<u32>,
    weight: Vec<f64>,
    controls: Vec<Vec<f64>>,
}

impl Fixture {
    fn new(q: usize) -> Self {
        let mut value = Self {
            worker: Vec::new(),
            firm: Vec::new(),
            weight: Vec::new(),
            controls: vec![Vec::new(); q],
        };
        // Degree 2--8 workers, unequal weights, repeated rows; the adjacent
        // firms connect the graph. Controls also vary within coefficient cells.
        for worker in 0..WORKERS {
            for visit in 0..(2 + worker % 7) {
                for repeat in 0..3 {
                    let row = value.worker.len();
                    value.worker.push(worker as u32);
                    value.firm.push(((worker + visit) % FIRMS) as u32);
                    value
                        .weight
                        .push(0.5 + ((worker + 2 * visit + repeat) % 9) as f64 / 3.0);
                    for (index, column) in value.controls.iter_mut().enumerate() {
                        column.push(((row + 1) as f64 * (index + 1) as f64 * 0.37).sin());
                    }
                }
            }
        }
        value
    }

    fn data(&self) -> CanonicalModelData<'_> {
        CanonicalModelData {
            workers: WORKERS,
            firms: FIRMS,
            row_worker: &self.worker,
            row_firm: &self.firm,
            weight: &self.weight,
            controls: &self.controls,
        }
    }

    // Independent row-level A'WA oracle, not the production Schur action.
    fn rhs(&self) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut worker = vec![0.0; WORKERS * COLUMNS];
        let mut firm = vec![0.0; FIRMS * COLUMNS];
        let mut control = vec![0.0; self.controls.len() * COLUMNS];
        for column in 1..COLUMNS {
            for row in 0..self.worker.len() {
                let w = self.worker[row] as usize;
                let f = self.firm[row] as usize;
                let signal = self.signal(column, row);
                worker[column * WORKERS + w] += signal * self.weight[row];
                firm[column * FIRMS + f] += signal * self.weight[row];
                for q in 0..self.controls.len() {
                    control[column * self.controls.len() + q] +=
                        signal * self.weight[row] * self.controls[q][row];
                }
            }
        }
        (worker, firm, control)
    }

    fn signal(&self, column: usize, row: usize) -> f64 {
        if column == 0 {
            return 0.0;
        }
        let w = self.worker[row] as usize;
        let f = self.firm[row] as usize;
        (w as f64 * column as f64 * 0.11).sin()
            + (f as f64 * column as f64 * 0.19).cos()
            + self
                .controls
                .iter()
                .enumerate()
                .map(|(q, values)| values[row] * (q + column) as f64 / 13.0)
                .sum::<f64>()
    }

    fn check(&self, solved: &ModelBatchSolve, tolerance: f64) {
        for (column, result) in solved.solution.iter().enumerate() {
            assert!(result.residual.relative_norm <= tolerance);
            assert_eq!(result.receipt.full_residual_tolerance, tolerance);
            for row in 0..self.worker.len() {
                let prediction = result.coefficients.worker[self.worker[row] as usize]
                    + result.coefficients.firm[self.firm[row] as usize]
                    + self
                        .controls
                        .iter()
                        .enumerate()
                        .map(|(q, values)| values[row] * result.coefficients.control[q])
                        .sum::<f64>();
                assert!(
                    (prediction - self.signal(column, row)).abs() < 1.0e-8,
                    "row-level oracle mismatch: column {column}, row {row}"
                );
            }
        }
    }
}

#[test]
fn diagonal_queue_weighted_mixed_degrees_all_thread_counts_and_strict_options() {
    for q in [0, 2] {
        let fixture = Fixture::new(q);
        let options = ModelSolverOptions::default();
        let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
        let (worker, firm, control) = fixture.rhs();
        for tolerance in [1.0e-10, 1.0e-13] {
            let mut strict = options;
            strict.pcg.tolerance = tolerance;
            let serial = solver
                .solve_batch_with_options_and_interrupt(
                    &worker,
                    &firm,
                    &control,
                    COLUMNS,
                    5,
                    strict,
                    &mut NeverInterrupt,
                )
                .unwrap();
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                let queue = DiagonalBatchExecutor::new(
                    &solver,
                    threads,
                    COLUMNS,
                    0,
                    MemoryBudget::Unspecified,
                    &mut NeverInterrupt,
                )
                .unwrap();
                let (result, receipt) = queue
                    .solve_batch_with_interrupt(
                        &worker,
                        &firm,
                        &control,
                        COLUMNS,
                        strict,
                        &mut NeverInterrupt,
                    )
                    .unwrap();
                fixture.check(&result, strict.full_residual_tolerance());
                assert_eq!(receipt.columns, COLUMNS);
                assert_eq!(receipt.active_worker_limit, threads.min(COLUMNS));
                assert!(receipt.maximum_active_workers <= receipt.active_worker_limit);
                assert!(receipt.maximum_active_workers > 0);
                assert!(receipt.retained_queue_metadata_bytes <= queue.plan().queue_metadata_bytes);
                for (left, right) in serial.solution.iter().zip(&result.solution) {
                    assert_eq!(left.coefficients.worker, right.coefficients.worker);
                    assert_eq!(left.coefficients.firm, right.coefficients.firm);
                    assert_eq!(left.coefficients.control, right.coefficients.control);
                    assert_eq!(left.receipt.pcg.iterations, right.receipt.pcg.iterations);
                }
                let (empty, receipt) = queue
                    .solve_batch_with_interrupt(&[], &[], &[], 0, strict, &mut NeverInterrupt)
                    .unwrap();
                assert!(empty.solution.is_empty());
                assert_eq!(receipt.maximum_active_workers, 0);
            }
        }
    }
}

#[test]
fn diagonal_queue_ordered_failures_and_reuse() {
    let fixture = Fixture::new(2);
    let options = ModelSolverOptions::default();
    let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
    let (worker, firm, control) = fixture.rhs();
    for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
        let queue = DiagonalBatchExecutor::new(
            &solver,
            threads,
            COLUMNS,
            0,
            MemoryBudget::Unspecified,
            &mut NeverInterrupt,
        )
        .unwrap();
        let mut incompatible = worker.clone();
        incompatible[2 * WORKERS] += 1.0;
        incompatible[9 * WORKERS] += 1.0;
        let error = queue
            .solve_batch_with_interrupt(
                &incompatible,
                &firm,
                &control,
                COLUMNS,
                options,
                &mut NeverInterrupt,
            )
            .unwrap_err();
        assert!(error.message.contains("column 2"), "{error}");
        let mut hard = options;
        hard.pcg.maximum_iterations = 1;
        let error = queue
            .solve_batch_with_interrupt(
                &worker,
                &firm,
                &control,
                COLUMNS,
                hard,
                &mut NeverInterrupt,
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::PcgMaxIterations);
        assert!(error.message.contains("column 1"), "{error}");
        let (result, _) = queue
            .solve_batch_with_interrupt(
                &worker,
                &firm,
                &control,
                COLUMNS,
                options,
                &mut NeverInterrupt,
            )
            .unwrap();
        fixture.check(&result, options.full_residual_tolerance());
    }
}

struct BreakOnWait {
    caller: std::thread::ThreadId,
    phase: &'static str,
}

impl InterruptCheck for BreakOnWait {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        assert_eq!(
            std::thread::current().id(),
            self.caller,
            "host callback on a worker"
        );
        if phase == self.phase {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "injected caller cancellation",
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn diagonal_queue_caller_cancellation_and_reuse() {
    let fixture = Fixture::new(2);
    let options = ModelSolverOptions::default();
    let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
    let (worker, firm, control) = fixture.rhs();
    for threads in [1, 4, 7] {
        let queue = DiagonalBatchExecutor::new(
            &solver,
            threads,
            COLUMNS,
            0,
            MemoryBudget::Unspecified,
            &mut NeverInterrupt,
        )
        .unwrap();
        let mut breaker = BreakOnWait {
            caller: std::thread::current().id(),
            phase: if threads == 1 {
                "model_batch_pcg_iteration"
            } else {
                "model_diagonal_queue_wait"
            },
        };
        let error = queue
            .solve_batch_with_interrupt(&worker, &firm, &control, COLUMNS, options, &mut breaker)
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::UserBreak);
        let (result, _) = queue
            .solve_batch_with_interrupt(
                &worker,
                &firm,
                &control,
                COLUMNS,
                options,
                &mut NeverInterrupt,
            )
            .unwrap();
        fixture.check(&result, options.full_residual_tolerance());
    }
}

#[test]
fn diagonal_queue_budget_capacity_overflow_and_route_gates() {
    let fixture = Fixture::new(0);
    let options = ModelSolverOptions::default();
    let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
    let plan = DiagonalQueuePlan::checked(WORKERS, FIRMS, 0, 7, COLUMNS, 1234).unwrap();
    for (bytes, passes) in [
        (plan.command_peak_bytes, true),
        (plan.command_peak_bytes - 1, false),
    ] {
        let result = DiagonalBatchExecutor::new(
            &solver,
            7,
            COLUMNS,
            1234,
            MemoryBudget::Explicit {
                bytes,
                check: MemoryCheck::Error,
            },
            &mut NeverInterrupt,
        );
        assert_eq!(result.is_ok(), passes);
        if let Err(error) = result {
            assert_eq!(error.code, ErrorCode::ResourceLimit);
        }
    }
    for check in [MemoryCheck::Warn, MemoryCheck::Off] {
        let queue = DiagonalBatchExecutor::new(
            &solver,
            7,
            COLUMNS,
            1234,
            MemoryBudget::Explicit { bytes: 1, check },
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(queue.plan(), plan);
    }
    let mut breaker = BreakOnWait {
        caller: std::thread::current().id(),
        phase: "model_diagonal_queue_admitted",
    };
    assert_eq!(
        DiagonalBatchExecutor::new(
            &solver,
            7,
            COLUMNS,
            0,
            MemoryBudget::Unspecified,
            &mut breaker
        )
        .unwrap_err()
        .code,
        ErrorCode::UserBreak
    );
    let queue = DiagonalBatchExecutor::new(
        &solver,
        64,
        1,
        0,
        MemoryBudget::Unspecified,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(queue.plan().workers, 1);
    assert!(queue.runtime.pool.is_none());
    assert_eq!(
        queue
            .solve_batch_with_interrupt(&[], &[], &[], 2, options, &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        queue
            .solve_batch_with_interrupt(&[0.0], &[], &[], 0, options, &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    for plan in [
        DiagonalQueuePlan::checked(usize::MAX, 1, 0, 7, COLUMNS, 0),
        DiagonalQueuePlan::checked(WORKERS, FIRMS, 0, 7, usize::MAX, 0),
        DiagonalQueuePlan::checked(WORKERS, FIRMS, 0, 7, COLUMNS, u64::MAX),
    ] {
        assert_eq!(plan.unwrap_err().code, ErrorCode::ResourceLimit);
    }
    assert!(DiagonalQueuePlan::checked(1, 1, 0, 0, 1, 0).is_err());
    assert!(DiagonalQueuePlan::checked(1, 1, 0, 1, 0, 0).is_err());
    let routed = PreparedModelSolver::prepare_routed(
        fixture.data(),
        ModelRoutingOptions {
            route: ModelSolverRoute::Cmg,
            ..ModelRoutingOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        DiagonalBatchExecutor::new(
            &routed,
            7,
            COLUMNS,
            0,
            MemoryBudget::Unspecified,
            &mut NeverInterrupt
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedFeature
    );
}

#[test]
fn diagonal_queue_parent_token_and_worker_panic_are_not_poisoning() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let slots = (0..9).map(|_| Mutex::new(None)).collect::<Vec<Slot>>();
    let peak = AtomicUsize::new(0);
    run_parallel(
        &pool,
        &slots,
        4,
        &peak,
        |column, _| {
            if column == 0 {
                panic!("injected worker panic");
            }
            Err(BackendError::new(
                ErrorCode::InvalidInput,
                PHASE,
                format!("column {column}"),
            ))
        },
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(
        slots[0]
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap_err()
            .code,
        ErrorCode::Panic
    );
    let fixture = Fixture::new(0);
    let solver =
        PreparedModelSolver::prepare(fixture.data(), ModelSolverOptions::default()).unwrap();
    let zero = solver
        .solve(crate::model_operator::ModelRhs {
            worker: &[0.0; WORKERS],
            firm: &[0.0; FIRMS],
            control: &[],
        })
        .unwrap();
    run_parallel(
        &pool,
        &slots,
        4,
        &peak,
        |_, _| Ok(zero.clone()),
        &mut NeverInterrupt,
    )
    .unwrap();
    assert!(slots
        .iter()
        .all(|slot| slot.lock().unwrap().as_ref().unwrap().is_ok()));
    // Cancellation also works when a token-bearing caller itself returns Ok.
    struct TokenOnly(CancellationToken);
    impl InterruptCheck for TokenOnly {
        fn checkpoint(&mut self, _: &'static str) -> Result<()> {
            Ok(())
        }
        fn cancellation_token(&self) -> Option<CancellationToken> {
            Some(self.0.clone())
        }
    }
    let token = CancellationToken::new();
    token.cancel();
    let error = run_parallel(
        &pool,
        &slots,
        4,
        &peak,
        |_, _| unreachable!(),
        &mut TokenOnly(token),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::UserBreak);
}

#[test]
fn diagonal_queue_proves_concurrency_without_timing_tiny_solves() {
    let fixture = Fixture::new(0);
    let solver =
        PreparedModelSolver::prepare(fixture.data(), ModelSolverOptions::default()).unwrap();
    let zero = solver
        .solve(crate::model_operator::ModelRhs {
            worker: &[0.0; WORKERS],
            firm: &[0.0; FIRMS],
            control: &[],
        })
        .unwrap();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let slots = (0..9).map(|_| Mutex::new(None)).collect::<Vec<Slot>>();
    let peak = AtomicUsize::new(0);
    let entered = AtomicUsize::new(0);
    let caller = std::thread::current().id();
    let caller_work = AtomicUsize::new(0);
    run_parallel(
        &pool,
        &slots,
        4,
        &peak,
        |column, _| {
            if std::thread::current().id() == caller {
                caller_work.fetch_add(1, Ordering::Relaxed);
            }
            if column < 4 {
                entered.fetch_add(1, Ordering::SeqCst);
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while entered.load(Ordering::SeqCst) < 4 {
                    if std::time::Instant::now() >= deadline {
                        return Err(BackendError::invariant(
                            PHASE,
                            "workers did not enter concurrently",
                        ));
                    }
                    std::thread::yield_now();
                }
            }
            Ok(zero.clone())
        },
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(peak.load(Ordering::Relaxed), 4);
    assert!(caller_work.load(Ordering::Relaxed) > 0);
    assert!(slots
        .iter()
        .all(|slot| slot.lock().unwrap().as_ref().unwrap().is_ok()));

    // A 64-thread unit allocation is not a 64-core timing claim. Capacity 65
    // deliberately exceeds the pool and leaves one final queued RHS.
    let queue = DiagonalBatchExecutor::new(
        &solver,
        64,
        65,
        0,
        MemoryBudget::Unspecified,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(
        queue.runtime.pool.as_ref().unwrap().current_num_threads(),
        64
    );
    let (result, receipt) = queue
        .solve_batch_with_interrupt(
            &vec![0.0; WORKERS * 65],
            &vec![0.0; FIRMS * 65],
            &[],
            65,
            ModelSolverOptions::default(),
            &mut NeverInterrupt,
        )
        .unwrap();
    assert_eq!(result.solution.len(), 65);
    assert_eq!(receipt.active_worker_limit, 64);
    assert!(receipt.maximum_active_workers <= 64);
    assert!(result
        .solution
        .iter()
        .all(|value| value.residual.relative_norm == 0.0));
}

#[test]
fn diagonal_completion_publishes_on_success_and_unwind() {
    let remaining = AtomicUsize::new(2);
    let caller = std::thread::current();
    drop(WorkerCompletion {
        remaining: &remaining,
        caller: &caller,
    });
    assert_eq!(remaining.load(Ordering::Acquire), 1);
    assert!(catch_unwind(AssertUnwindSafe(|| {
        let _completion = WorkerCompletion {
            remaining: &remaining,
            caller: &caller,
        };
        panic!("injected completion unwind");
    }))
    .is_err());
    assert_eq!(remaining.load(Ordering::Acquire), 0);
    let plan = DiagonalQueuePlan::checked(WORKERS, FIRMS, 0, 4, 32, 0).unwrap();
    assert_eq!(plan.stack_reservation_bytes, (4 * STACK_BYTES) as u64);
}

#[test]
fn diagonal_queue_reuses_pool_for_short_dependent_batches() {
    let fixture = Fixture::new(2);
    let options = ModelSolverOptions::default();
    let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
    let (worker, firm, control) = fixture.rhs();
    let serial = solver
        .solve_batch_with_interrupt(
            &worker,
            &firm,
            &control,
            COLUMNS,
            COLUMNS,
            &mut NeverInterrupt,
        )
        .unwrap();
    for threads in [2, 3, 4, 7, 14, 28, 64] {
        let queue = DiagonalBatchExecutor::new(
            &solver,
            threads,
            COLUMNS,
            0,
            MemoryBudget::Unspecified,
            &mut NeverInterrupt,
        )
        .unwrap();
        for columns in [1, 3, 1, COLUMNS, 2, 1] {
            let (result, receipt) = queue
                .solve_batch_with_interrupt(
                    &worker[..columns * WORKERS],
                    &firm[..columns * FIRMS],
                    &control[..columns * 2],
                    columns,
                    options,
                    &mut NeverInterrupt,
                )
                .unwrap();
            assert!(receipt.maximum_active_workers <= threads.min(columns));
            for (actual, expected) in result.solution.iter().zip(&serial.solution) {
                assert_eq!(actual.coefficients.worker, expected.coefficients.worker);
                assert_eq!(actual.coefficients.firm, expected.coefficients.firm);
                assert_eq!(actual.coefficients.control, expected.coefficients.control);
                assert_eq!(
                    actual.residual.relative_norm,
                    expected.residual.relative_norm
                );
            }
        }
        let work = queue.runtime.work_receipt().unwrap();
        assert_eq!(work.completed_batches, 6);
        assert_eq!(work.completed_rhs, COLUMNS + 8);
    }
}

#[test]
fn diagonal_single_rhs_stays_on_caller_and_can_break_inside_pcg() {
    struct CheckSingle {
        caller: std::thread::ThreadId,
        iterations: usize,
        cancel: bool,
    }
    impl InterruptCheck for CheckSingle {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            assert_eq!(std::thread::current().id(), self.caller);
            if phase == "model_batch_pcg_iteration" {
                self.iterations += 1;
                if self.cancel {
                    return Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "single RHS break",
                    ));
                }
            }
            Ok(())
        }
    }
    let fixture = Fixture::new(2);
    let options = ModelSolverOptions::default();
    let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
    let (worker, firm, control) = fixture.rhs();
    let queue = DiagonalBatchExecutor::new(
        &solver,
        4,
        COLUMNS,
        0,
        MemoryBudget::Unspecified,
        &mut NeverInterrupt,
    )
    .unwrap();
    for cancel in [false, true, false] {
        let mut check = CheckSingle {
            caller: std::thread::current().id(),
            iterations: 0,
            cancel,
        };
        let result = queue.solve_batch_with_interrupt(
            &worker[WORKERS..2 * WORKERS],
            &firm[FIRMS..2 * FIRMS],
            &control[2..4],
            1,
            options,
            &mut check,
        );
        assert!(
            check.iterations > 0,
            "single RHS was handed off to a worker"
        );
        if cancel {
            assert_eq!(result.unwrap_err().code, ErrorCode::UserBreak);
        } else {
            let (solved, receipt) = result.unwrap();
            assert_eq!(receipt.maximum_active_workers, 1);
            assert_eq!(solved.solution.len(), 1);
        }
    }
    assert_eq!(queue.runtime.work_receipt().unwrap().completed_rhs, 2);
}

#[test]
fn diagonal_caller_polling_is_bounded_but_tokens_are_immediate() {
    struct Host(usize);
    impl InterruptCheck for Host {
        fn checkpoint(&mut self, _: &'static str) -> Result<()> {
            self.0 += 1;
            Ok(())
        }
    }
    let local = CancellationToken::new();
    let mut host = Host(0);
    let mut failure = None;
    let mut check = CallerInterrupt {
        host: &mut host,
        worker: WorkerInterrupt {
            local: &local,
            parent: None,
        },
        failure: &mut failure,
        next_poll: Instant::now() + Duration::from_secs(60),
    };
    for _ in 0..100 {
        check.checkpoint(PHASE).unwrap();
    }
    // Manually make the deadline due: this test never sleeps or assumes a
    // machine-speed threshold. A pending token must bypass the deadline.
    check.next_poll = Instant::now();
    check.checkpoint(PHASE).unwrap();
    check.next_poll = Instant::now() + Duration::from_secs(60);
    local.cancel();
    assert_eq!(
        check.checkpoint(PHASE).unwrap_err().code,
        ErrorCode::UserBreak
    );
    assert_eq!(host.0, 1);
    assert!(failure.is_none());
}
