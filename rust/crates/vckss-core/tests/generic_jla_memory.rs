// SPDX-License-Identifier: GPL-3.0-only

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use vckss_core::batch_plan::BatchRequest;
use vckss_core::cmg::CmgOptions;
use vckss_core::error::ErrorCode;
use vckss_core::full_cmg::FullCmgPlanOptions;
use vckss_core::generic_jla::run_generic_jla_with_direct_solver_interrupt;
use vckss_core::generic_jla::{
    run_generic_jla, run_generic_jla_routed, GenericJlaExecutionOptions, GenericJlaMemoryPeakPhase,
    GenericJlaOptions,
};
use vckss_core::krylov::PcgOptions;
use vckss_core::memory::MemoryBudget;
use vckss_core::model_operator::CanonicalModelData;
use vckss_core::model_solver::diagonal_queue::DiagonalBatchExecutor;
use vckss_core::model_solver::{
    ModelRoutingOptions, ModelSolverOptions, ModelSolverRoute, PreparedModelSolver,
};
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::types::{DeletionMode, InputColumns, NuisanceMode};

struct TrackingAllocator;

static TRACKING: AtomicBool = AtomicBool::new(false);
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn isolated_diagonal_queue_incremental_heap_stays_within_admitted_payload() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut weight = Vec::new();
    let mut controls = vec![Vec::new(); 2];
    for w in 0..140_u32 {
        for visit in 0..(2 + w % 7) {
            for repeat in 0..4 {
                let row = worker.len();
                worker.push(w);
                firm.push((w + visit) % 71);
                weight.push(0.5 + ((w + repeat + visit) % 13) as f64 / 3.0);
                controls[0].push((row as f64 * 0.37).sin());
                controls[1].push((row as f64 * 0.57).cos());
            }
        }
    }
    let solver = PreparedModelSolver::prepare(
        CanonicalModelData {
            workers: 140,
            firms: 71,
            row_worker: &worker,
            row_firm: &firm,
            weight: &weight,
            controls: &controls,
        },
        ModelSolverOptions::default(),
    )
    .unwrap();
    let columns = 33;
    let mut worker_rhs = vec![0.0; 140 * columns];
    let mut firm_rhs = vec![0.0; 71 * columns];
    let mut control_rhs = vec![0.0; 2 * columns];
    for column in 0..columns {
        for row in 0..worker.len() {
            let value = ((row + column) as f64 * 0.11).cos() * weight[row];
            worker_rhs[140 * column + worker[row] as usize] += value;
            firm_rhs[71 * column + firm[row] as usize] += value;
            for q in 0..2 {
                control_rhs[2 * column + q] += value * controls[q][row];
            }
        }
    }
    for threads in [1, 7] {
        let ((queue, solved), peak, live) = measured_live(|| {
            let queue = DiagonalBatchExecutor::new(
                &solver,
                threads,
                columns,
                0,
                MemoryBudget::Unspecified,
                &mut vckss_core::interrupt::NeverInterrupt,
            )
            .unwrap();
            let solved = queue
                .solve_batch_with_interrupt(
                    &worker_rhs,
                    &firm_rhs,
                    &control_rhs,
                    columns,
                    ModelSolverOptions::default(),
                    &mut vckss_core::interrupt::NeverInterrupt,
                )
                .unwrap();
            (queue, solved)
        });
        let plan = queue.plan();
        let heap_forecast = plan.incremental_peak_bytes - plan.stack_reservation_bytes;
        eprintln!("diagonal_queue threads={threads} measured_heap_peak={peak} retained_heap={live} heap_forecast={heap_forecast} stack_reservation={}", plan.stack_reservation_bytes);
        assert!(peak as u64 <= heap_forecast);
        assert!(live <= peak);
        assert_eq!(solved.0.solution.len(), columns);
        assert!(solved.1.retained_queue_metadata_bytes <= plan.queue_metadata_bytes);
        drop(solved);
        drop(queue);
    }
}

fn record_allocation(bytes: usize) {
    if TRACKING.load(Ordering::Relaxed) {
        let current = CURRENT.fetch_add(bytes, Ordering::Relaxed) + bytes;
        PEAK.fetch_max(current, Ordering::Relaxed);
    }
}

fn record_deallocation(bytes: usize) {
    if TRACKING.load(Ordering::Relaxed) {
        CURRENT.fetch_sub(bytes, Ordering::Relaxed);
    }
}

// SAFETY: System receives every request unchanged; the atomics only observe
// requested payload bytes and never affect allocation or deallocation.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged to the process system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged to the process system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_deallocation(layout.size());
        // SAFETY: caller supplies the pointer/layout pair issued by this
        // forwarding allocator.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarded unchanged to the process system allocator.
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() && TRACKING.load(Ordering::Relaxed) {
            if new_size >= layout.size() {
                record_allocation(new_size - layout.size());
            } else {
                record_deallocation(layout.size() - new_size);
            }
        }
        replacement
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

fn measured<T>(action: impl FnOnce() -> T) -> (T, usize) {
    assert!(!TRACKING.swap(true, Ordering::SeqCst));
    CURRENT.store(0, Ordering::SeqCst);
    PEAK.store(0, Ordering::SeqCst);
    let output = action();
    TRACKING.store(false, Ordering::SeqCst);
    (output, PEAK.load(Ordering::SeqCst))
}

fn measured_live<T>(action: impl FnOnce() -> T) -> (T, usize, usize) {
    assert!(!TRACKING.swap(true, Ordering::SeqCst));
    CURRENT.store(0, Ordering::SeqCst);
    PEAK.store(0, Ordering::SeqCst);
    let output = action();
    let live = CURRENT.load(Ordering::SeqCst);
    TRACKING.store(false, Ordering::SeqCst);
    (output, PEAK.load(Ordering::SeqCst), live)
}

#[test]
fn compressed_statistical_command_heap_and_retained_payload_are_admitted() {
    use vckss_core::engine::{
        run_jla_no_controls_planned, JlaEngineOptions, PlannedJlaEngineOptions,
    };
    let _lock = TEST_LOCK.lock().unwrap();
    let selected = std::env::var("FEVC_COMPRESSED_STATISTICAL_MEMORY_THREADS").ok();
    if selected.is_none() {
        // One allocator window per process, like the other pool-lifetime tests:
        // a preceding pool's runtime teardown must not cross a fresh counter.
        for threads in [1, 7] {
            assert!(std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "compressed_statistical_command_heap_and_retained_payload_are_admitted",
                    "--nocapture",
                    "--test-threads=1"
                ])
                .env(
                    "FEVC_COMPRESSED_STATISTICAL_MEMORY_THREADS",
                    threads.to_string()
                )
                .status()
                .unwrap()
                .success());
        }
        return;
    }
    let problem = q0_cmg_setup_problem(257);
    for threads in [1, 7] {
        if selected.as_deref() != Some(threads.to_string().as_str()) {
            continue;
        }
        let ((result,), peak, live) = measured_live(|| {
            let mut cmg = FullCmgPlanOptions::production(threads, 1e-10, None);
            cmg.memory_budget = MemoryBudget::Unspecified;
            (run_jla_no_controls_planned(
                &problem,
                PlannedJlaEngineOptions {
                    estimator: JlaEngineOptions {
                        probes: 33,
                        memory_budget: MemoryBudget::Unspecified,
                        memory_limit_bytes: 0,
                        ..Default::default()
                    },
                    full_cmg: Some(cmg),
                    ..Default::default()
                },
            )
            .unwrap(),)
        });
        let forecast = result.execution.memory.solve_peak_forecast_bytes;
        eprintln!("compressed-statistics threads={threads} heap_peak={peak} retained={live} forecast={forecast}");
        assert!(peak as u64 <= forecast);
        assert!(live <= peak);
        assert!(result.estimator.receipt.max_complete_residual <= 1e-5);
        assert_eq!(result.execution.full_cmg.as_ref().unwrap().rhs_count, 100);
        assert_eq!(
            result.execution.batch.plan.selected_command_peak_bytes,
            forecast
        );
        drop(result);
    }
}

fn q32_problem(large_match_block: bool) -> CompressedProblem {
    const Q: usize = 32;
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut controls = vec![Vec::new(); Q];
    for worker_index in 0..4_u64 {
        for firm_index in 0..4_u64 {
            let repetitions = 34;
            for repetition in 0..repetitions {
                let row = worker.len();
                worker.push(100 + worker_index);
                firm.push(1_000 + firm_index);
                deletion.push(
                    if large_match_block && worker_index == 0 && firm_index == 0 {
                        50_000
                    } else {
                        60_000 + row as u64
                    },
                );
                outcome.push(
                    0.4 * worker_index as f64 - 0.3 * firm_index as f64
                        + repetition as f64 / 17.0
                        + (row % 9) as f64 / 23.0,
                );
                frequency.push(1);
                target_weight.push(0.75 + (row + 1) as f64 / 1_000.0);
                for (control, column) in controls.iter_mut().enumerate() {
                    let angle =
                        core::f64::consts::PI * (repetition as f64 + 0.5) * (control as f64 + 1.0)
                            / 34.0;
                    let block_scale = if large_match_block && worker_index == 0 && firm_index == 0 {
                        0.01
                    } else {
                        1.0
                    };
                    column.push(block_scale * angle.cos());
                }
            }
        }
    }
    let retained = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls,
        }
        .validate()
        .expect("Q=32 memory fixture validates"),
    )
    .expect("Q=32 memory fixture canonicalizes")
    .compress(&vec![true; retained])
    .expect("Q=32 memory fixture compresses")
}

fn q32_many_cell_rank_problem() -> CompressedProblem {
    const Q: usize = 32;
    const FIRMS: usize = 200;
    const CONTRAST_CELLS: usize = 80;
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut controls = vec![Vec::new(); Q];
    for worker_index in 0..2_usize {
        for firm_index in 0..FIRMS {
            let repetitions = if worker_index == 0 && firm_index < CONTRAST_CELLS {
                2
            } else {
                1
            };
            for repetition in 0..repetitions {
                let row = worker.len();
                worker.push(10 + worker_index as u64);
                firm.push(1_000 + firm_index as u64);
                deletion.push(100_000 + row as u64);
                outcome.push(
                    worker_index as f64 / 3.0 - firm_index as f64 / 101.0
                        + repetition as f64 / 17.0,
                );
                frequency.push(1);
                target_weight.push(0.5 + (row + 1) as f64 / 10_000.0);
                for (control, column) in controls.iter_mut().enumerate() {
                    let value = if repetitions == 2 {
                        let angle = core::f64::consts::PI
                            * (firm_index as f64 + 0.5)
                            * (control as f64 + 1.0)
                            / CONTRAST_CELLS as f64;
                        if repetition == 0 {
                            angle.cos()
                        } else {
                            -angle.cos()
                        }
                    } else {
                        0.0
                    };
                    column.push(value);
                }
            }
        }
    }
    let retained = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls,
        }
        .validate()
        .expect("many-cell Q=32 memory fixture validates"),
    )
    .expect("many-cell Q=32 memory fixture canonicalizes")
    .compress(&vec![true; retained])
    .expect("many-cell Q=32 memory fixture compresses")
}

fn q0_cmg_setup_problem(firms: usize) -> CompressedProblem {
    let rows = 4 * firms;
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut deletion = Vec::with_capacity(rows);
    let mut outcome = Vec::with_capacity(rows);
    let mut frequency = Vec::with_capacity(rows);
    let mut target_weight = Vec::with_capacity(rows);
    for worker_index in 0..2_usize {
        for firm_index in 0..firms {
            for replicate in 0..2_usize {
                let row = worker.len();
                worker.push(10 + worker_index as u64);
                firm.push(1_000 + firm_index as u64);
                deletion.push(100_000 + row as u64);
                outcome.push(
                    worker_index as f64 / 3.0 - firm_index as f64 / 101.0
                        + if (worker_index + firm_index + replicate) % 2 == 0 {
                            0.4
                        } else {
                            -0.25
                        },
                );
                frequency.push(1);
                target_weight.push(0.75 + (row % 11) as f64 / 17.0);
            }
        }
    }
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: Vec::new(),
        }
        .validate()
        .expect("Q=0 CMG setup fixture validates"),
    )
    .expect("Q=0 CMG setup fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("Q=0 CMG setup fixture compresses")
}

fn options(deletion: DeletionMode, nuisance: NuisanceMode) -> GenericJlaOptions {
    GenericJlaOptions {
        seed: 0x1234_5678_9abc_def0,
        probes: 2,
        leverage_batch_width: 2,
        target_batch_width: 2,
        deletion,
        nuisance,
        blocksize_limit: 128,
        rank_tolerance: 1.0e-10,
        block_tolerance: 1.0e-10,
        solver: ModelSolverOptions {
            pcg: PcgOptions {
                tolerance: 1.0e-11,
                maximum_iterations: 10_000,
                residual_replacement_interval: 23,
            },
            rank_tolerance: 1.0e-11,
        },
        ..GenericJlaOptions::default()
    }
}

fn cmg_options(estimator: GenericJlaOptions) -> GenericJlaExecutionOptions {
    GenericJlaExecutionOptions {
        estimator,
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Cmg,
            cmg_minimum_dimension: 1,
            allow_automatic_cmg_setup_fallback: false,
            solver: estimator.solver,
            cmg: CmgOptions::default(),
        },
        leverage_batch: BatchRequest::Explicit(estimator.leverage_batch_width),
        target_batch: BatchRequest::Explicit(estimator.target_batch_width),
        wallseconds: None,
    }
}

#[test]
fn controlled_direct_forecasts_cover_q32_allocator_peaks() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    let selected = std::env::var("FEVC_CONTROL_DIRECT_MEMORY_CASE").ok();
    if selected.is_none() {
        for deletion in ["Observation", "Match"] {
            for threads in [1, 7] {
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "controlled_direct_forecasts_cover_q32_allocator_peaks",
                        "--nocapture",
                    ])
                    .env(
                        "FEVC_CONTROL_DIRECT_MEMORY_CASE",
                        format!("{deletion}-{threads}"),
                    )
                    .status()
                    .unwrap();
                assert!(status.success());
            }
        }
        return;
    }
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        for threads in [1, 7] {
            if selected.as_deref() != Some(format!("{deletion:?}-{threads}").as_str()) {
                continue;
            }
            let problem = q32_problem(true);
            let mut estimator = options(deletion, NuisanceMode::Joint);
            estimator.memory_budget = MemoryBudget::Unspecified;
            estimator.memory_limit_bytes = 0;
            let mut request = cmg_options(estimator);
            request.leverage_batch = BatchRequest::Auto;
            request.target_batch = BatchRequest::Auto;
            let (result, observed) = measured(|| {
                run_generic_jla_with_direct_solver_interrupt(
                    &problem,
                    request,
                    None,
                    None,
                    None,
                    Some(FullCmgPlanOptions::production(threads, 1e-10, None)),
                    &mut vckss_core::interrupt::NeverInterrupt,
                )
            });
            let result = result.expect("controlled direct Q32 fixture estimates");
            let forecast = result.receipt.execution.memory.peak_bytes;
            eprintln!("controlled_direct_memory {deletion:?} threads={threads} observed={observed} forecast={forecast}");
            assert!(observed as u64 <= forecast);
            assert_eq!(result.receipt.control_rank.projection_rhs.len(), 32);
            assert_eq!(
                result
                    .receipt
                    .execution
                    .full_cmg
                    .as_ref()
                    .unwrap()
                    .setup
                    .admitted_peak_bytes,
                forecast
            );
        }
    }
}

#[test]
fn queued_diagonal_inference_forecast_covers_heap() {
    use vckss_core::component_inference::{
        ComponentInferenceOptions, ComponentInferenceUnit, ComponentVarianceSource,
    };
    use vckss_core::interrupt::NeverInterrupt;
    let _serial = TEST_LOCK.lock().unwrap();
    let selected = std::env::var("FEVC_DIAGONAL_INFERENCE_MEMORY_CASE").ok();
    if selected.is_none() {
        for unit in ["Observation", "Match"] {
            for threads in [1, 7] {
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "queued_diagonal_inference_forecast_covers_heap",
                        "--nocapture",
                    ])
                    .env(
                        "FEVC_DIAGONAL_INFERENCE_MEMORY_CASE",
                        format!("{unit}-{threads}"),
                    )
                    .status()
                    .unwrap();
                assert!(status.success());
            }
        }
        return;
    }
    for (deletion, unit, nuisance) in [
        (
            DeletionMode::Observation,
            ComponentInferenceUnit::Observation,
            NuisanceMode::Joint,
        ),
        (
            DeletionMode::Match,
            ComponentInferenceUnit::Match,
            NuisanceMode::FixedOffset,
        ),
    ] {
        for threads in [1, 7] {
            if selected.as_deref() != Some(format!("{unit:?}-{threads}").as_str()) {
                continue;
            }
            let mut problem = q32_problem(false);
            problem.controls.truncate(2);
            let prepared = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
                &problem,
                unit,
                ComponentVarianceSource::StructuredLeverage,
                ComponentInferenceOptions {
                    probes: 129,
                    batch_width: 7,
                    spectrum_probes: 17,
                    spectrum_iterations: 16,
                    ..ComponentInferenceOptions::default()
                },
                vckss_core::structured_variance::StructuredVarianceOptions::default(),
                513,
                &mut NeverInterrupt,
            )
            .unwrap();
            let mut estimator = options(deletion, nuisance);
            estimator.probes = 200;
            estimator.memory_budget = MemoryBudget::Unspecified;
            let mut request = cmg_options(estimator);
            request.routing.route = ModelSolverRoute::Diagonal;
            // Capacity four forces seven-column residual callbacks into
            // partial physical chunks while preserving all logical outputs.
            request.leverage_batch = BatchRequest::Explicit(2);
            request.target_batch = BatchRequest::Explicit(2);
            let (result, observed) = measured(|| {
                vckss_core::generic_jla::run_generic_jla_with_diagonal_queue_attachments_interrupt(
                    &problem,
                    request,
                    None,
                    Some(&prepared),
                    None,
                    threads,
                    &mut NeverInterrupt,
                )
            });
            let result = result.unwrap();
            let queue = result.receipt.execution.diagonal_queue.unwrap();
            let heap = queue.plan.command_peak_bytes - queue.plan.stack_reservation_bytes;
            eprintln!("diagonal_inference_heap {unit:?} threads={threads} observed={observed} heap_bound={heap} command_bound={} stack_reservation={}", queue.plan.command_peak_bytes, queue.plan.stack_reservation_bytes);
            assert!(observed as u64 <= heap);
            assert_eq!(queue.plan.maximum_rhs, 4);
            assert_eq!(
                queue.plan.command_peak_bytes,
                result.receipt.execution.memory.peak_bytes
            );
            let inference = result.component_inference.unwrap();
            assert_eq!(inference.residual_moments.unwrap().projections.len(), 513);
            assert_eq!(
                queue.work.completed_rhs,
                2 + 600 + 513 + inference.solve_receipts.len()
            );
        }
    }
}

#[test]
fn direct_cmg_inference_forecast_covers_q32_heap() {
    use vckss_core::component_inference::{
        ComponentInferenceOptions, ComponentInferenceUnit, ComponentVarianceSource,
    };
    use vckss_core::interrupt::NeverInterrupt;
    let _serial = TEST_LOCK.lock().unwrap();
    let selected = std::env::var("FEVC_DIRECT_INFERENCE_MEMORY_CASE").ok();
    if selected.is_none() {
        for unit in ["Observation", "Match"] {
            for threads in [1, 7] {
                assert!(std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "direct_cmg_inference_forecast_covers_q32_heap",
                        "--nocapture"
                    ])
                    .env(
                        "FEVC_DIRECT_INFERENCE_MEMORY_CASE",
                        format!("{unit}-{threads}")
                    )
                    .status()
                    .unwrap()
                    .success());
            }
        }
        return;
    }
    for (deletion, unit, nuisance) in [
        (
            DeletionMode::Observation,
            ComponentInferenceUnit::Observation,
            NuisanceMode::Joint,
        ),
        (
            DeletionMode::Match,
            ComponentInferenceUnit::Match,
            NuisanceMode::FixedOffset,
        ),
    ] {
        for threads in [1, 7] {
            if selected.as_deref() != Some(format!("{unit:?}-{threads}").as_str()) {
                continue;
            }
            let problem = q32_problem(false);
            let prepared = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
                &problem,
                unit,
                ComponentVarianceSource::StructuredLeverage,
                ComponentInferenceOptions {
                    probes: 129,
                    batch_width: 7,
                    spectrum_probes: 17,
                    spectrum_iterations: 16,
                    ..ComponentInferenceOptions::default()
                },
                vckss_core::structured_variance::StructuredVarianceOptions::default(),
                513,
                &mut NeverInterrupt,
            )
            .unwrap();
            let mut estimator = options(deletion, nuisance);
            estimator.probes = 200;
            // Minimum physical capacity forces both 32 rank RHSs and seven
            // Gram columns through the same two-RHS pool. No hard rejection.
            estimator.memory_budget = MemoryBudget::Explicit {
                bytes: 1,
                check: vckss_core::memory::MemoryCheck::Warn,
            };
            let mut request = cmg_options(estimator);
            request.leverage_batch = BatchRequest::Auto;
            request.target_batch = BatchRequest::Auto;
            let (result, observed) = measured(|| {
                vckss_core::generic_jla::run_generic_jla_with_direct_attachments_interrupt(
                    &problem,
                    request,
                    None,
                    Some(&prepared),
                    None,
                    FullCmgPlanOptions::production(threads, 1e-10, None),
                    &mut NeverInterrupt,
                )
            });
            let result = result.unwrap();
            let execution = &result.receipt.execution;
            let cmg = execution.full_cmg.as_ref().unwrap();
            let work = execution.direct_attachments.unwrap();
            eprintln!("direct_inference_heap {unit:?} threads={threads} observed={observed} command_bound={} retained_solver={} workspace_pool={} workspace_count={}",
                execution.memory.peak_bytes, cmg.setup.actual_retained_bytes,
                cmg.setup.admitted_workspace_pool_bytes, cmg.setup.workspace_count);
            assert!(observed as u64 <= execution.memory.peak_bytes);
            assert_eq!(cmg.setup.maximum_batch_rhs, 2);
            assert_eq!(work.control_projection_rhs, 32);
            assert_eq!(work.gram_rhs, 513);
            assert_eq!(cmg.setup.admitted_peak_bytes, execution.memory.peak_bytes);
        }
    }
}

#[test]
fn automatic_inference_forecast_covers_q32_heap() {
    use vckss_core::component_inference::{
        ComponentInferenceOptions, ComponentInferenceUnit, ComponentVarianceSource,
    };
    use vckss_core::generic_jla::run_generic_jla_with_automatic_component_batches_interrupt;
    use vckss_core::interrupt::NeverInterrupt;
    let _serial = TEST_LOCK.lock().unwrap();
    let selected = std::env::var("FEVC_AUTO_INFERENCE_MEMORY_CASE").ok();
    if selected.is_none() {
        for unit in ["Observation", "Match"] {
            for threads in [1, 7] {
                for cmg in [false, true] {
                    assert!(std::process::Command::new(std::env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "automatic_inference_forecast_covers_q32_heap",
                            "--nocapture"
                        ])
                        .env(
                            "FEVC_AUTO_INFERENCE_MEMORY_CASE",
                            format!("{unit}-{threads}-{cmg}")
                        )
                        .status()
                        .unwrap()
                        .success());
                }
            }
        }
        return;
    }
    for (deletion, unit, nuisance) in [
        (
            DeletionMode::Observation,
            ComponentInferenceUnit::Observation,
            NuisanceMode::Joint,
        ),
        (
            DeletionMode::Match,
            ComponentInferenceUnit::Match,
            NuisanceMode::FixedOffset,
        ),
    ] {
        for threads in [1, 7] {
            for cmg in [false, true] {
                if selected.as_deref() != Some(format!("{unit:?}-{threads}-{cmg}").as_str()) {
                    continue;
                }
                let problem = q32_problem(false);
                let prepared =
                    vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
                        &problem,
                        unit,
                        ComponentVarianceSource::StructuredLeverage,
                        ComponentInferenceOptions {
                            probes: 129,
                            batch_width: 8,
                            spectrum_probes: 17,
                            spectrum_iterations: 16,
                            ..ComponentInferenceOptions::default()
                        },
                        vckss_core::structured_variance::StructuredVarianceOptions::default(),
                        513,
                        &mut NeverInterrupt,
                    )
                    .unwrap();
                let mut estimator = options(deletion, nuisance);
                estimator.probes = 33;
                estimator.memory_budget = MemoryBudget::Unspecified;
                let mut request = cmg_options(estimator);
                request.leverage_batch = BatchRequest::Auto;
                request.target_batch = BatchRequest::Auto;
                request.routing.route = if cmg {
                    ModelSolverRoute::Cmg
                } else {
                    ModelSolverRoute::Diagonal
                };
                let (result, observed) = measured(|| {
                    run_generic_jla_with_automatic_component_batches_interrupt(
                        &problem,
                        request,
                        &prepared,
                        threads,
                        cmg.then(|| FullCmgPlanOptions::production(threads, 1e-10, None)),
                        &mut NeverInterrupt,
                    )
                });
                let result = result.unwrap();
                let execution = &result.receipt.execution;
                let batch = execution.component_batch.unwrap();
                assert_eq!(
                    (batch.component_width, batch.gram_width),
                    (32.max(8 * threads), 32.max(8 * threads))
                );
                let stack = execution
                    .diagonal_queue
                    .map_or(0, |queue| queue.plan.stack_reservation_bytes);
                let heap_bound = execution.memory.peak_bytes - stack;
                eprintln!("automatic_inference_heap {unit:?} threads={threads} cmg={cmg} observed={observed} heap_bound={heap_bound} command_bound={} component_width={} gram_width={}", execution.memory.peak_bytes, batch.component_width, batch.gram_width);
                assert!(observed as u64 <= heap_bound);
                assert_eq!(
                    execution.batch.plan.selected_command_peak_bytes,
                    execution.memory.peak_bytes
                );
                assert_eq!(result.receipt.control_rank.projection_rhs.len(), 32);
                assert_eq!(
                    result
                        .component_inference
                        .unwrap()
                        .residual_moments
                        .unwrap()
                        .projections
                        .len(),
                    513
                );
            }
        }
    }
}

#[test]
fn queued_diagonal_estimator_forecast_covers_q32_heap() {
    let _serial = TEST_LOCK.lock().unwrap();
    // One process per tracking epoch avoids asynchronous runtime teardown
    // from a preceding pool being attributed to the next measurement.
    let selected = std::env::var("FEVC_DIAGONAL_ESTIMATOR_MEMORY_CASE").ok();
    if selected.is_none() {
        for deletion in ["Observation", "Match"] {
            for threads in [1, 7] {
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "queued_diagonal_estimator_forecast_covers_q32_heap",
                        "--nocapture",
                    ])
                    .env(
                        "FEVC_DIAGONAL_ESTIMATOR_MEMORY_CASE",
                        format!("{deletion}-{threads}"),
                    )
                    .status()
                    .unwrap();
                assert!(status.success());
            }
        }
        return;
    }
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        for threads in [1, 7] {
            if selected.as_deref() != Some(format!("{deletion:?}-{threads}").as_str()) {
                continue;
            }
            let problem = q32_problem(true);
            let mut estimator = options(deletion, NuisanceMode::Joint);
            estimator.memory_budget = MemoryBudget::Unspecified;
            estimator.memory_limit_bytes = 0;
            let mut request = cmg_options(estimator);
            request.routing.route = ModelSolverRoute::Diagonal;
            request.leverage_batch = BatchRequest::Auto;
            request.target_batch = BatchRequest::Auto;
            let (result, observed) = measured(|| {
                vckss_core::generic_jla::run_generic_jla_with_diagonal_queue_interrupt(
                    &problem,
                    request,
                    None,
                    None,
                    threads,
                    &mut vckss_core::interrupt::NeverInterrupt,
                )
            });
            let result = result.unwrap();
            let queue = result.receipt.execution.diagonal_queue.unwrap();
            let heap_bound = queue.plan.command_peak_bytes - queue.plan.stack_reservation_bytes;
            eprintln!("diagonal_estimator_heap {deletion:?} threads={threads} observed={observed} heap_bound={heap_bound} command_bound={} stack_reservation={}", queue.plan.command_peak_bytes, queue.plan.stack_reservation_bytes);
            assert!(observed as u64 <= heap_bound);
            assert_eq!(
                result.receipt.execution.memory.peak_bytes,
                queue.plan.command_peak_bytes
            );
            assert_eq!(result.receipt.control_rank.projection_rhs.len(), 32);
            assert_eq!(queue.work.completed_rhs, 32 + 3 * estimator.probes as usize);
        }
    }
}

#[test]
fn deletion_neutral_direct_forecasts_cover_allocator_peaks() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    // Rayon releases some thread-local allocations asynchronously. Isolate each
    // measurement so a previous pool's teardown cannot cross this allocator's
    // tracking epoch (the estimator itself has already returned and dropped it).
    let selected = std::env::var("FEVC_DIRECT_MEMORY_CASE").ok();
    if selected.is_none() {
        for firms in [128, 512] {
            for deletion in ["Observation", "Match"] {
                for threads in [1, 4, 7] {
                    let status = std::process::Command::new(std::env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "deletion_neutral_direct_forecasts_cover_allocator_peaks",
                            "--nocapture",
                        ])
                        .env(
                            "FEVC_DIRECT_MEMORY_CASE",
                            format!("{deletion}-{firms}-{threads}"),
                        )
                        .status()
                        .unwrap();
                    assert!(status.success());
                }
            }
        }
        return;
    }
    for firms in [128, 512] {
        let problem = q0_cmg_setup_problem(firms);
        for deletion in [DeletionMode::Observation, DeletionMode::Match] {
            for threads in [1, 4, 7] {
                if selected.as_deref() != Some(format!("{deletion:?}-{firms}-{threads}").as_str()) {
                    continue;
                }
                let mut estimator = options(deletion, NuisanceMode::Joint);
                estimator.probes = 33;
                estimator.memory_limit_bytes = 0;
                estimator.memory_budget = MemoryBudget::Unspecified;
                let mut request = cmg_options(estimator);
                request.leverage_batch = BatchRequest::Auto;
                request.target_batch = BatchRequest::Auto;
                struct Trace {
                    samples: Vec<(&'static str, usize, usize)>,
                }
                impl vckss_core::interrupt::InterruptCheck for Trace {
                    fn checkpoint(&mut self, phase: &'static str) -> vckss_core::error::Result<()> {
                        let peak = PEAK.load(Ordering::Relaxed);
                        if self.samples.last().is_none_or(|s| peak > s.2)
                            && self.samples.len() < self.samples.capacity()
                        {
                            self.samples
                                .push((phase, CURRENT.load(Ordering::Relaxed), peak));
                        }
                        Ok(())
                    }
                }
                let mut trace = Trace {
                    samples: Vec::with_capacity(4096),
                };
                let (result, observed) = measured(|| {
                    run_generic_jla_with_direct_solver_interrupt(
                        &problem,
                        request,
                        None,
                        None,
                        None,
                        Some(FullCmgPlanOptions::production(threads, 1e-10, Some(1e-10))),
                        &mut trace,
                    )
                });
                let result = result.expect("direct weighted repeated-row fixture estimates");
                let execution = &result.receipt.execution;
                if observed as u64 > execution.memory.peak_bytes {
                    eprintln!(
                        "trace: {:?}\nmemory: {:?}",
                        &trace.samples[trace.samples.len().saturating_sub(12)..],
                        execution.memory
                    );
                }
                assert!(observed as u64 <= execution.memory.peak_bytes,
                    "{deletion:?}, firms={firms}, threads={threads}: observed {observed} exceeds forecast {}",
                    execution.memory.peak_bytes);
                assert_eq!(
                    execution
                        .full_cmg
                        .as_ref()
                        .unwrap()
                        .setup
                        .admitted_peak_bytes,
                    execution.memory.peak_bytes
                );
                eprintln!("direct_memory {deletion:?} firms={firms} threads={threads} observed={observed} forecast={}",
                    execution.memory.peak_bytes);
            }
        }
    }
}

#[test]
fn q32_phase_lifetimes_dominate_observed_allocator_peaks_at_exact_boundaries() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    for (large_block, deletion, nuisance) in [
        (false, DeletionMode::Match, NuisanceMode::Joint),
        (true, DeletionMode::Match, NuisanceMode::Joint),
        (false, DeletionMode::Observation, NuisanceMode::FixedOffset),
    ] {
        let problem = q32_problem(large_block);
        let estimator_options = options(deletion, nuisance);
        let (result, observed_peak) = measured(|| run_generic_jla(&problem, estimator_options));
        let result = result.expect("Q=32 memory fixture estimates");

        let rows = problem.outcome.len();
        let q = problem.controls.len();
        assert_eq!(q, 32);
        assert_eq!(result.receipt.target_strata, rows, "fixture has T = N");
        if !large_block {
            assert_eq!(
                usize::try_from(result.receipt.deletion_units).expect("D fits usize"),
                rows,
                "singleton match/observation fixture has D = N"
            );
        }
        let geometry_floor =
            rows * q * size_of::<f64>() + rows * size_of::<f64>() + q * q * size_of::<f64>();
        assert!(
            observed_peak >= geometry_floor,
            "allocator peak {observed_peak} misses independently enumerated live geometry {geometry_floor}"
        );
        assert!(
            u64::try_from(observed_peak).expect("peak fits u64")
                <= result.receipt.peak_forecast_bytes,
            "forecast must dominate allocator-observed module peak"
        );
        if large_block {
            assert!(34 > q + 1, "fixture exercises B > P maker route");
        } else if deletion == DeletionMode::Match {
            assert!(1 <= q + 1, "fixture exercises B <= P maker route");
        }

        let mut exact = estimator_options;
        exact.memory_limit_bytes = result.receipt.peak_forecast_bytes;
        run_generic_jla(&problem, exact).expect("exact enumerated forecast boundary accepts");
        exact.memory_limit_bytes -= 1;
        let error = run_generic_jla(&problem, exact).expect_err("one byte below boundary rejects");
        assert_eq!(error.code, ErrorCode::ResourceLimit);
        assert_eq!(error.phase, "generic_jla_memory");
    }
}

#[test]
fn q32_many_cell_rank_artifact_is_observed_and_sets_the_exact_boundary() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    let problem = q32_many_cell_rank_problem();
    let estimator_options = options(DeletionMode::Observation, NuisanceMode::FixedOffset);
    let (result, observed_peak) = measured(|| run_generic_jla(&problem, estimator_options));
    let result = result.expect("many-cell Q=32 fixture estimates");

    let rows = problem.outcome.len();
    let cells = problem.cells();
    let q = problem.controls.len();
    assert_eq!(q, 32);
    assert!(cells * 6 >= rows * 5, "fixture has C close to N");
    let f64_bytes = size_of::<f64>();
    let independent_rank_floor = 3 * rows * q * f64_bytes
        + 4 * cells * q * f64_bytes
        + rows * f64_bytes
        + q * q * f64_bytes
        + cells * f64_bytes;
    assert!(
        observed_peak >= independent_rank_floor,
        "allocator peak {observed_peak} misses independently enumerated rank lifetime {independent_rank_floor}"
    );
    let phase_peak = [
        result.receipt.canonicalization_peak_forecast_bytes,
        result.receipt.fit_peak_forecast_bytes,
        result.receipt.geometry_peak_forecast_bytes,
        result.receipt.leverage_peak_forecast_bytes,
        result.receipt.target_peak_forecast_bytes,
        result.receipt.maker_peak_forecast_bytes,
        result.receipt.result_forecast_bytes,
    ]
    .into_iter()
    .max()
    .expect("phase forecasts are nonempty");
    assert_eq!(result.receipt.peak_forecast_bytes, phase_peak);
    assert!(
        result.receipt.fit_peak_forecast_bytes > 0
            && result.receipt.geometry_peak_forecast_bytes > 0,
        "retained strict-projection and moved geometry lifetimes are both reported"
    );
    assert!(
        u64::try_from(observed_peak).expect("peak fits u64") <= result.receipt.peak_forecast_bytes
    );

    let mut exact = estimator_options;
    exact.memory_limit_bytes = result.receipt.peak_forecast_bytes;
    run_generic_jla(&problem, exact).expect("exact rank-phase boundary accepts");
    exact.memory_limit_bytes -= 1;
    let error = run_generic_jla(&problem, exact).expect_err("one byte below rank peak rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.phase, "generic_jla_memory");
}

#[test]
fn routed_cmg_receipt_prices_exact_persistent_and_setup_allocations() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    let problem = q32_problem(false);
    let configured = cmg_options(options(DeletionMode::Match, NuisanceMode::Joint));
    let (result, observed_peak) = measured(|| run_generic_jla_routed(&problem, configured));
    let result = result.expect("routed CMG memory fixture estimates");
    let execution = &result.receipt.execution;
    let memory = &execution.memory;
    let cmg = execution
        .full_solver_setup
        .cmg
        .as_ref()
        .expect("forced CMG setup receipt");

    let expected_preconditioner = cmg.fine_vertices * 2 * size_of::<f64>();
    assert_eq!(
        usize::try_from(cmg.preconditioner_bytes).expect("preconditioner bytes fit usize"),
        expected_preconditioner
    );
    assert_eq!(
        memory.cmg_preconditioner_workspace_bytes,
        cmg.preconditioner_bytes
    );
    assert_eq!(
        memory.shared_cmg_persistent_bytes,
        cmg.structural_bytes
            + cmg.workspace_bytes
            + cmg.preconditioner_bytes
            + cmg.dense_factor_bytes,
        "the one shared CMG lifetime includes both retained full vectors exactly once"
    );

    let rows = problem.outcome.len();
    let cells = problem.cells();
    let expected_cells = rows * (2 * size_of::<u32>() + size_of::<f64>());
    assert_eq!(
        usize::try_from(memory.cmg_aggregated_cell_capacity_bytes)
            .expect("aggregated capacity bytes fit usize"),
        expected_cells,
        "model_cmg_problem reserves all three aggregated columns at N capacity"
    );
    assert!(rows > cells);
    assert!(
        expected_cells > cells * (2 * size_of::<u32>() + size_of::<f64>()),
        "fixture distinguishes N-capacity pricing from final C length"
    );
    let expected_groups =
        2 * cells * size_of::<u32>() + (problem.workers() + problem.firms() + 2) * size_of::<u64>();
    assert_eq!(
        usize::try_from(memory.cmg_group_index_bytes).expect("group-index bytes fit usize"),
        expected_groups
    );
    let auxiliary_vertices = cmg.fine_vertices - problem.firms();
    let expected_hybrid =
        cmg.fine_vertices * 48 + cmg.fine_edges * 40 + auxiliary_vertices * size_of::<u32>();
    assert_eq!(
        usize::try_from(memory.cmg_hybrid_graph_bytes).expect("hybrid bytes fit usize"),
        expected_hybrid
    );
    assert!(
        u64::try_from(observed_peak).expect("observed peak fits u64") <= memory.peak_bytes,
        "route-aware forecast must dominate the allocator-observed command peak"
    );

    let mut exact = configured;
    exact.estimator.memory_limit_bytes = memory.peak_bytes;
    run_generic_jla_routed(&problem, exact).expect("exact routed CMG boundary accepts");
    exact.estimator.memory_limit_bytes -= 1;
    let error = run_generic_jla_routed(&problem, exact)
        .expect_err("one byte below the frozen explicit CMG plan rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.phase, "generic_jla_memory");
}

#[test]
fn direct_cmg_setup_peak_and_retained_lifetime_expose_corrected_terms() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    let problem = q0_cmg_setup_problem(256);
    let weights = problem
        .frequency
        .iter()
        .map(|&value| value as f64)
        .collect::<Vec<_>>();
    let data = CanonicalModelData {
        workers: problem.workers(),
        firms: problem.firms(),
        row_worker: &problem.row_worker,
        row_firm: &problem.row_firm,
        weight: &weights,
        controls: &[],
    };
    let routing = ModelRoutingOptions {
        route: ModelSolverRoute::Cmg,
        cmg_minimum_dimension: 1,
        allow_automatic_cmg_setup_fallback: false,
        solver: options(DeletionMode::Match, NuisanceMode::Joint).solver,
        cmg: CmgOptions::default(),
    };
    let (prepared, observed_peak, observed_live) =
        measured_live(|| PreparedModelSolver::prepare_routed(data, routing));
    let prepared = prepared.expect("direct forced-CMG preparation");
    let cmg = prepared.receipt().cmg.as_ref().expect("CMG receipt");
    let persistent = cmg.structural_bytes
        + cmg.workspace_bytes
        + cmg.preconditioner_bytes
        + cmg.dense_factor_bytes;
    let operator_persistent = problem.outcome.len() * size_of::<usize>()
        + (problem.workers() + 2 * problem.firms()) * size_of::<f64>();
    assert!(
        observed_live
            >= operator_persistent + usize::try_from(cmg.preconditioner_bytes).expect("bytes fit"),
        "live solver allocation must include the model operator and both CMG full vectors"
    );

    let rows = problem.outcome.len();
    let cells = problem.cells();
    assert!(
        rows > cells,
        "fixture distinguishes N capacity from C length"
    );
    let aggregated_n_capacity = rows * (2 * size_of::<u32>() + size_of::<f64>());
    let group_index =
        2 * cells * size_of::<u32>() + (problem.workers() + problem.firms() + 2) * size_of::<u64>();
    assert!(
        observed_peak >= operator_persistent + aggregated_n_capacity + group_index,
        "allocator-observed setup must expose N-capacity aggregate columns beside group indices"
    );

    let mut exact = routing;
    exact.cmg.memory_limit_bytes = persistent;
    PreparedModelSolver::prepare_routed(data, exact)
        .expect("exact retained CMG lifetime boundary accepts");
    exact.cmg.memory_limit_bytes -= 1;
    let error = PreparedModelSolver::prepare_routed(data, exact)
        .expect_err("one byte below retained CMG lifetime rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.phase, "cmg_setup");
}

#[test]
fn dense_terminal_matrix_and_lower_define_exact_setup_and_command_boundaries() {
    let _serial = TEST_LOCK.lock().expect("memory tests serialize");
    let problem = q0_cmg_setup_problem(256);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator.leverage_batch_width = 1;
    estimator.target_batch_width = 1;
    let mut configured = cmg_options(estimator);
    configured.leverage_batch = BatchRequest::Explicit(1);
    configured.target_batch = BatchRequest::Explicit(1);
    let fine_vertices = problem.workers() + problem.firms();
    configured.routing.cmg.terminal_vertices = fine_vertices;
    configured.routing.cmg.dense_vertex_cap = fine_vertices;

    let (result, observed_peak) = measured(|| run_generic_jla_routed(&problem, configured));
    let result = result.expect("large dense-terminal forced-CMG command");
    let execution = &result.receipt.execution;
    let memory = &execution.memory;
    let cmg = execution
        .full_solver_setup
        .cmg
        .as_ref()
        .expect("forced-CMG receipt");
    assert_eq!(cmg.fine_vertices, fine_vertices);
    assert_eq!(cmg.terminal_vertices, fine_vertices);
    let reduced = fine_vertices - 1;
    let expected_dense = reduced * reduced * size_of::<f64>();
    assert_eq!(
        usize::try_from(cmg.dense_factor_bytes).expect("dense factor bytes fit usize"),
        expected_dense
    );

    let hierarchy_build = memory.cmg_aggregated_cell_capacity_bytes
        + memory.cmg_group_index_bytes
        + memory.cmg_hybrid_graph_bytes
        + cmg.structural_bytes
        + 2 * cmg.dense_factor_bytes;
    assert_eq!(
        memory.setup_transient_bytes, hierarchy_build,
        "dense construction must price the simultaneous matrix and lower"
    );
    assert_eq!(memory.peak_phase, GenericJlaMemoryPeakPhase::SolverSetup);
    assert_eq!(memory.peak_bytes, memory.setup_peak_bytes);
    assert!(
        observed_peak >= 2 * expected_dense,
        "allocator peak must observe both dense terminal arrays simultaneously"
    );
    assert!(
        u64::try_from(observed_peak).expect("allocator peak fits u64") <= memory.peak_bytes,
        "dense-terminal route forecast must dominate observed allocation"
    );

    let dense_setup_limit = cmg.structural_bytes + cmg.workspace_bytes + 2 * cmg.dense_factor_bytes;
    assert!(
        dense_setup_limit
            >= cmg.structural_bytes
                + cmg.workspace_bytes
                + cmg.preconditioner_bytes
                + cmg.dense_factor_bytes,
        "fixture isolates dense setup above retained CMG memory"
    );
    let mut exact_setup = configured;
    exact_setup.routing.cmg.memory_limit_bytes = dense_setup_limit;
    run_generic_jla_routed(&problem, exact_setup).expect("exact dense setup limit accepts");
    exact_setup.routing.cmg.memory_limit_bytes -= 1;
    let setup_error = run_generic_jla_routed(&problem, exact_setup)
        .expect_err("one byte below dense setup limit rejects");
    assert_eq!(setup_error.code, ErrorCode::ResourceLimit);
    assert_eq!(setup_error.phase, "cmg_setup");

    let mut exact_command = configured;
    exact_command.estimator.memory_limit_bytes = memory.peak_bytes;
    run_generic_jla_routed(&problem, exact_command).expect("exact setup-dominant command accepts");
    exact_command.estimator.memory_limit_bytes -= 1;
    let command_error = run_generic_jla_routed(&problem, exact_command)
        .expect_err("one byte below setup-dominant command limit rejects");
    assert_eq!(command_error.code, ErrorCode::ResourceLimit);
    assert_eq!(command_error.phase, "generic_jla_memory");
}
