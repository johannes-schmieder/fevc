// SPDX-License-Identifier: GPL-3.0-only

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use vckss_core::batch_plan::BatchRequest;
use vckss_core::cmg::CmgOptions;
use vckss_core::error::ErrorCode;
use vckss_core::generic_jla::{
    run_generic_jla, run_generic_jla_routed, GenericJlaExecutionOptions, GenericJlaMemoryPeakPhase,
    GenericJlaOptions,
};
use vckss_core::krylov::PcgOptions;
use vckss_core::model_operator::CanonicalModelData;
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
