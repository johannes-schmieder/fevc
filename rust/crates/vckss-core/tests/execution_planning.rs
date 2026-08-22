// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::batch_plan::{
    plan_batches, BatchPlannerCaps, BatchRequest, PhaseMemoryModel, BATCH_ARITHMETIC_CONTRACT,
};
use vckss_core::engine_plan::{
    resolve_engine, resolve_estimator_plan, AlgorithmRequest, EngineRequest,
    EngineResolutionRequest, EstimatorAlgorithm, EstimatorPlanRequest, SelectedEngine,
};
use vckss_core::error::ErrorCode;
use vckss_core::generic_batch::ModelBatchWorkspaceLayout;
use vckss_core::types::{DeletionMode, NuisanceMode};
use vckss_core::wall_plan::{wall_work_receipt, WallCalibration, WallRoutingEffect, WallWork};

fn caps(memory: u64) -> BatchPlannerCaps {
    BatchPlannerCaps {
        probes: 64,
        declared_threads: 8,
        columns_per_thread: 8,
        route_width_cap: 64,
        non_batched_peak_bytes: 0,
        hard_memory_bytes: memory,
    }
}

fn batch_independent_estimator(width: usize, probes: usize) -> Vec<f64> {
    let mut output = vec![0.0; probes];
    for start in (0..probes).step_by(width) {
        let end = start.saturating_add(width).min(probes);
        for (probe, value) in output.iter_mut().enumerate().take(end).skip(start) {
            // The registered contract requires each logical column to keep its
            // scalar arithmetic order; batching may only change grouping.
            let x = probe as f64 + 0.25;
            *value = ((x * 3.0) - x) + (x / 8.0);
        }
    }
    output
}

#[test]
fn memory_driven_width_changes_preserve_the_bitwise_estimator_contract() {
    let memory_model = PhaseMemoryModel {
        fixed_bytes: 100,
        bytes_per_width: 25,
    };
    let narrow = plan_batches(
        BatchRequest::Auto,
        BatchRequest::Auto,
        caps(125),
        memory_model,
        memory_model,
    )
    .expect("narrow plan");
    let wide = plan_batches(
        BatchRequest::Auto,
        BatchRequest::Auto,
        caps(1_700),
        memory_model,
        memory_model,
    )
    .expect("wide plan");
    assert_eq!(narrow.leverage.selected_width, 1);
    assert_eq!(wide.leverage.selected_width, 64);
    assert!(narrow.bitwise_estimator_width_invariance_required);
    assert_eq!(narrow.arithmetic_contract, BATCH_ARITHMETIC_CONTRACT);

    let narrow_bits: Vec<u64> = batch_independent_estimator(
        narrow.leverage.selected_width,
        narrow.leverage.probe_width_cap,
    )
    .into_iter()
    .map(f64::to_bits)
    .collect();
    let wide_bits: Vec<u64> =
        batch_independent_estimator(wide.leverage.selected_width, wide.leverage.probe_width_cap)
            .into_iter()
            .map(f64::to_bits)
            .collect();
    assert_eq!(narrow_bits, wide_bits);
}

#[test]
fn wallseconds_is_advisory_and_cannot_change_engine_resolution() {
    let engine_request = EngineResolutionRequest {
        algorithm: EstimatorAlgorithm::Jla,
        engine: EngineRequest::Auto,
        deletion: DeletionMode::Match,
        nuisance: NuisanceMode::Joint,
        controls: 0,
        compressed_semantic_plan_ready: true,
        compressed_physical_rng_ready: true,
    };
    let before = resolve_engine(engine_request).expect("engine route");
    assert_eq!(before.selected, SelectedEngine::Compressed);

    let work = WallWork {
        preparation: 1,
        engine_setup: 1,
        full_fit: 1,
        leverage: 1_000,
        target: 1_000,
        result_export: 1,
    };
    let calibration = WallCalibration::Calibrated {
        model_id: "TEST-WALL-V1",
        intercept_seconds: 0.0,
        seconds_per_work_unit: 1.0,
        advisory_margin_fraction: 0.0,
    };
    let impossible_envelope =
        wall_work_receipt(work, Some(1.0), calibration).expect("advisory receipt");
    assert_eq!(
        impossible_envelope.routing_effect,
        WallRoutingEffect::AdvisoryOnly
    );

    let after = resolve_engine(engine_request).expect("unchanged engine route");
    assert_eq!(before, after);
}

#[test]
fn public_estimator_plan_separates_algorithm_and_engine_resolution() {
    let boundary = EstimatorPlanRequest {
        algorithm: AlgorithmRequest::Auto,
        engine: EngineRequest::Auto,
        deletion: DeletionMode::Observation,
        nuisance: NuisanceMode::FixedOffset,
        retained_workers: 2,
        retained_firms: 3,
        controls: 2,
        exact_limit: 6,
        compressed_semantic_plan_ready: false,
        compressed_physical_rng_ready: false,
    };
    let exact = resolve_estimator_plan(boundary).expect("boundary automatic exact plan");
    assert_eq!(exact.algorithm.requested, AlgorithmRequest::Auto);
    assert_eq!(exact.algorithm.identified_complexity, 2 + 3 - 1 + 2);
    assert_eq!(exact.algorithm.selected, EstimatorAlgorithm::Exact);
    assert_eq!(exact.engine.requested, EngineRequest::Auto);
    assert_eq!(exact.engine.selected, SelectedEngine::NotApplicable);
    assert_eq!(exact.rng_draws_consumed, 0);
    assert_eq!(exact.rng_counter_atoms_consumed, 0);

    let exact_generic = resolve_estimator_plan(EstimatorPlanRequest {
        engine: EngineRequest::Generic,
        ..boundary
    })
    .expect("engine(generic) syntax is accepted but not applicable to exact");
    assert_eq!(exact_generic.engine.selected, SelectedEngine::NotApplicable);
    let compressed_error = resolve_estimator_plan(EstimatorPlanRequest {
        engine: EngineRequest::Compressed,
        ..boundary
    })
    .expect_err("exact plus compressed rejects");
    assert_eq!(compressed_error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(compressed_error.phase, "engine_resolution");

    let jla = resolve_estimator_plan(EstimatorPlanRequest {
        deletion: DeletionMode::Match,
        nuisance: NuisanceMode::Joint,
        controls: 0,
        exact_limit: 3,
        compressed_semantic_plan_ready: true,
        compressed_physical_rng_ready: true,
        ..boundary
    })
    .expect("above-limit automatic JLA plan");
    assert_eq!(jla.algorithm.selected, EstimatorAlgorithm::Jla);
    assert_eq!(jla.engine.selected, SelectedEngine::Compressed);
    assert!(!jla.opportunistic_fallback_allowed);
}

#[test]
fn model_batch_workspace_bytes_are_checked_and_exact() {
    let layout = ModelBatchWorkspaceLayout::checked(3, 5, 2).expect("workspace layout");
    assert_eq!(layout.worker_values, 6);
    assert_eq!(layout.parameter_values, 10);
    assert_eq!(layout.mean_values, 2);
    assert_eq!(layout.bytes().expect("workspace bytes"), 18 * 8);

    let overflow = ModelBatchWorkspaceLayout {
        worker_values: usize::MAX,
        parameter_values: 1,
        mean_values: 1,
    }
    .bytes()
    .expect_err("workspace byte overflow");
    assert_eq!(overflow.code, ErrorCode::ResourceLimit);
}
