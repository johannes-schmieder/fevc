// SPDX-License-Identifier: GPL-3.0-only

use std::ffi::{c_void, CStr};
use std::mem::{offset_of, size_of};
use std::ptr;
use std::sync::Mutex;

use vckss_core::error::ErrorCode;
use vckss_core::exact_estimator::{run_exact_estimator, ExactEstimatorOptions};
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::types::{InputColumns, NuisanceMode, MAX_EXACT_BINARY64_INTEGER};
use vckss_core::ABI_VERSION;
use vckss_plugin::ffi_engine::{
    vckss_rust_backend_capabilities_v1, vckss_rust_backend_request_capability_v1,
    vckss_rust_engine_admit_prepare_probe_order_v1, vckss_rust_engine_admit_prepare_v2,
    vckss_rust_engine_admit_prepare_v3, vckss_rust_engine_admit_prepare_v4,
    vckss_rust_engine_clear_abandoned_v1, vckss_rust_engine_default_prepare_request_interrupt_v1,
    vckss_rust_engine_default_prepare_request_interrupt_v2,
    vckss_rust_engine_default_prepare_request_interrupt_v3,
    vckss_rust_engine_default_prepare_request_v3,
    vckss_rust_engine_default_solve_request_interrupt_v1,
    vckss_rust_engine_default_solve_request_interrupt_v2,
    vckss_rust_engine_default_solve_request_v1, vckss_rust_engine_default_solve_request_v2,
    vckss_rust_engine_detailed_receipt_v2, vckss_rust_engine_detailed_receipt_v3,
    vckss_rust_engine_detailed_receipt_v4, vckss_rust_engine_detailed_receipt_v5,
    vckss_rust_engine_last_error, vckss_rust_engine_preparation_receipt_v1,
    vckss_rust_engine_preparation_receipt_v2, vckss_rust_engine_preparation_receipt_v3,
    vckss_rust_engine_preparation_receipt_v4, vckss_rust_engine_prepare_interrupt_v1,
    vckss_rust_engine_prepare_interrupt_v2, vckss_rust_engine_prepare_interrupt_v3,
    vckss_rust_engine_prepare_interrupt_v4, vckss_rust_engine_prepare_v1,
    vckss_rust_engine_prepare_v2, vckss_rust_engine_prepare_v3, vckss_rust_engine_release_v1,
    vckss_rust_engine_result_v1, vckss_rust_engine_retained_mask_v1,
    vckss_rust_engine_rhs_receipts_v1, vckss_rust_engine_snapshot_v1,
    vckss_rust_engine_solve_interrupt_v1, vckss_rust_engine_solve_interrupt_v2,
    vckss_rust_engine_solve_v1, vckss_rust_engine_solve_v2, vckss_rust_session_clear_abandoned_v1,
    vckss_rust_session_last_error, vckss_rust_session_preparation_receipt_v1,
    vckss_rust_session_prepare_v1, vckss_rust_session_release_v1, vckss_rust_session_snapshot_v1,
    VckssBackendCapabilitiesV1, VckssBackendRequestCapabilityReceiptV1,
    VckssBackendRequestCapabilityRequestV1, VckssColumnsV1, VckssEngineColumnsV1,
    VckssEngineColumnsV2, VckssEngineColumnsV3, VckssEngineDetailedReceiptV1,
    VckssEngineDetailedReceiptV2, VckssEngineDetailedReceiptV3, VckssEngineDetailedReceiptV4,
    VckssEngineDetailedReceiptV5, VckssEnginePreparationReceiptV1, VckssEnginePreparationReceiptV2,
    VckssEnginePreparationReceiptV3, VckssEnginePreparationReceiptV4,
    VckssEnginePrepareRequestInterruptV1, VckssEnginePrepareRequestInterruptV2,
    VckssEnginePrepareRequestInterruptV3, VckssEnginePrepareRequestV1, VckssEnginePrepareRequestV2,
    VckssEnginePrepareRequestV3, VckssEnginePrepareRequestV4, VckssEngineResultV1,
    VckssEngineRhsReceiptV1, VckssEngineSnapshotV1, VckssEngineSolveRequestInterruptV1,
    VckssEngineSolveRequestInterruptV2, VckssEngineSolveRequestV1, VckssEngineSolveRequestV2,
    VckssPreparationReceiptV1, VckssPrepareRequestV1, VckssSessionSnapshotV1, VCKSS_ALGORITHM_AUTO,
    VCKSS_ALGORITHM_EXACT, VCKSS_ALGORITHM_JLA, VCKSS_CORE_FULL_CMG_V2_READY,
    VCKSS_CORE_JLA_PLAN_READY, VCKSS_DELETION_MATCH, VCKSS_DELETION_OBSERVATION,
    VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING, VCKSS_EXACT_DIAGNOSTIC_CONTROL_BASIS,
    VCKSS_EXACT_DIAGNOSTIC_INVERSE_SQRT, VCKSS_EXACT_DIAGNOSTIC_MAKER, VCKSS_INTERRUPT_CONTINUE,
    VCKSS_INTERRUPT_USER_BREAK, VCKSS_NUISANCE_FIXED_OFFSET, VCKSS_NUISANCE_JOINT,
    VCKSS_REQUEST_FREQUENCY_LITERAL, VCKSS_REQUEST_PROFILE_EXACT_V1,
    VCKSS_REQUEST_PROFILE_JLA_COUNTER_V1, VCKSS_REQUEST_REASON_ALGORITHM_AUTO_UNRESOLVED,
    VCKSS_REQUEST_REASON_CONTROLS_LIMIT, VCKSS_REQUEST_REASON_EXACT_RNG,
    VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE, VCKSS_REQUEST_REASON_JLA_CONTROLS,
    VCKSS_REQUEST_REASON_JLA_DELETION, VCKSS_REQUEST_REASON_JLA_NUISANCE,
    VCKSS_REQUEST_REASON_JLA_RNG, VCKSS_REQUEST_REASON_SUPPORTED,
    VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM, VCKSS_REQUEST_REASON_UNKNOWN_DELETION,
    VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE, VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE,
    VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT, VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA,
    VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE, VCKSS_RNG_COUNTER_V1, VCKSS_RNG_NONE,
    VCKSS_ROUTE_AUTO, VCKSS_ROUTE_CMG_PCG, VCKSS_ROUTE_DIAGONAL_PCG, VCKSS_ROUTE_EXACT,
};
use vckss_plugin::ffi_engine::{
    vckss_rust_backend_request_capability_v2, vckss_rust_engine_default_solve_request_interrupt_v3,
    vckss_rust_engine_default_solve_request_v3, vckss_rust_engine_detailed_receipt_v6,
    vckss_rust_engine_rhs_receipts_v2, vckss_rust_engine_solve_interrupt_v3,
    vckss_rust_engine_solve_v3, VckssBackendRequestCapabilityReceiptV2,
    VckssBackendRequestCapabilityRequestV2, VckssEngineDetailedReceiptV6, VckssEngineRhsReceiptV2,
    VckssEngineSolveRequestInterruptV3, VckssEngineSolveRequestV3, VCKSS_BATCH_MODE_AUTO,
    VCKSS_BATCH_MODE_EXPLICIT, VCKSS_DELETION_SOURCE_CELL_DEFAULT,
    VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT, VCKSS_DELETION_SOURCE_OBSERVATION_ROW,
    VCKSS_ENGINE_COMPRESSED, VCKSS_ENGINE_GENERIC, VCKSS_REQUEST_CAPABILITY_SCHEMA_V2,
    VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1, VCKSS_REQUEST_REASON_BATCH_MODE_UNSUPPORTED,
    VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH,
    VCKSS_REQUEST_REASON_JLA_GENERIC_SOLVER_ROUTE, VCKSS_REQUEST_REASON_PHYSICAL_LIMIT,
    VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED, VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED,
    VCKSS_REQUEST_REASON_UNKNOWN_BATCH_MODE, VCKSS_REQUEST_REASON_UNKNOWN_DELETION_UNIT_SOURCE,
    VCKSS_REQUEST_REASON_UNKNOWN_ENGINE, VCKSS_REQUEST_REASON_UNKNOWN_STAYERS_MODE,
    VCKSS_REQUEST_REASON_UNKNOWN_TARGET_WEIGHT_MODE, VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED,
    VCKSS_RESIDUAL_SPACE_WORKER_FIRM, VCKSS_RESIDUAL_SPACE_WORKER_FIRM_CONTROL,
    VCKSS_RHS_STATUS_CONVERGED, VCKSS_RHS_STATUS_ZERO, VCKSS_STAYERS_ALL, VCKSS_STAYERS_MOVERS,
    VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT, VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT,
};
use vckss_plugin::ffi_engine::{
    vckss_rust_backend_request_capability_v3, vckss_rust_engine_augment_stayers_interrupt_v1,
    vckss_rust_engine_augment_stayers_v1, vckss_rust_engine_default_solve_request_interrupt_v4,
    vckss_rust_engine_default_solve_request_v4,
    vckss_rust_engine_default_stayer_augmentation_request_interrupt_v1,
    vckss_rust_engine_detailed_receipt_v7, vckss_rust_engine_execution_plan_receipt_v1,
    vckss_rust_engine_full_cmg_receipt_v1, vckss_rust_engine_performance_receipt_v1,
    vckss_rust_engine_solve_interrupt_v4, vckss_rust_engine_solve_v4, vckss_rust_engine_solve_v5,
    vckss_rust_engine_stayer_augmentation_receipt_v1, vckss_rust_engine_stayer_hybrid_result_v1,
    VckssBackendRequestCapabilityReceiptV3, VckssBackendRequestCapabilityRequestV3,
    VckssEngineDetailedReceiptV7, VckssEnginePerformanceReceiptV1,
    VckssEngineSolveRequestInterruptV4, VckssEngineSolveRequestInterruptV5,
    VckssEngineSolveRequestV4, VckssEngineSolveRequestV5, VckssExecutionPlanReceiptV1,
    VckssFullCmgReceiptV1, VckssStayerAugmentationColumnsV1, VckssStayerAugmentationReceiptV1,
    VckssStayerAugmentationRequestInterruptV1, VckssStayerAugmentationRequestV1,
    VckssStayerHybridResultV1, VCKSS_BATCH_MODE_INDEPENDENT, VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
    VCKSS_ENGINE_NOT_APPLICABLE, VCKSS_PLAN_APPLICABILITY_COMPRESSED,
    VCKSS_PLAN_APPLICABILITY_EXACT, VCKSS_PLAN_APPLICABILITY_GENERIC,
    VCKSS_REQUEST_CAPABILITY_SCHEMA_V3, VCKSS_REQUEST_FREQUENCY_UNIT,
    VCKSS_REQUEST_PROFILE_PLANNED_V1, VCKSS_ROUTE_NOT_APPLICABLE,
};
use vckss_plugin::ffi_engine::{
    vckss_rust_engine_augment_component_inference_interrupt_v1,
    vckss_rust_engine_augment_match_component_inference_interrupt_v1,
    vckss_rust_engine_component_inference_augmentation_receipt_v1,
    vckss_rust_engine_component_inference_result_v2,
    vckss_rust_engine_component_inference_result_v3,
    vckss_rust_engine_component_inference_result_v4,
    vckss_rust_engine_component_inference_unit_receipt_v1,
    vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1,
    VckssComponentInferenceAugmentationReceiptV1,
    VckssComponentInferenceAugmentationRequestInterruptV1,
    VckssComponentInferenceAugmentationRequestV1, VckssComponentInferenceResultReceiptV2,
    VckssComponentInferenceResultReceiptV3, VckssComponentInferenceResultReceiptV4,
    VckssComponentInferenceUnitReceiptV1, VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V2,
    VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V3, VCKSS_COMPONENT_INFERENCE_SCHEMA_V1,
    VCKSS_COMPONENT_REFERENCE_Q0, VCKSS_COMPONENT_REFERENCE_Q1,
    VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON,
};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use vckss_plugin::ffi_engine::{
    vckss_rust_engine_default_solve_request_interrupt_v5, vckss_rust_engine_solve_interrupt_v5,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
#[repr(C)]
struct PollState {
    calls: u32,
    stop_at: u32,
    terminal_status: i32,
}

unsafe extern "C" fn injected_poll(context: *mut c_void) -> i32 {
    // SAFETY: each test passes a live, uniquely borrowed PollState for the
    // duration of the synchronous solve and the engine never retains it.
    let state = unsafe { &mut *context.cast::<PollState>() };
    state.calls += 1;
    if state.calls >= state.stop_at {
        state.terminal_status
    } else {
        VCKSS_INTERRUPT_CONTINUE
    }
}

fn bytes<T>() -> u32 {
    u32::try_from(size_of::<T>()).expect("ABI structure size")
}

fn component_bits(value: vckss_plugin::ffi_engine::VckssComponentVectorV1) -> [u64; 4] {
    [
        value.worker.to_bits(),
        value.firm.to_bits(),
        value.covariance.to_bits(),
        value.total.to_bits(),
    ]
}

fn detailed_receipt_float_bits(value: VckssEngineDetailedReceiptV3) -> [u64; 13] {
    let value = value.v2;
    [
        value.rank_tolerance.to_bits(),
        value.block_tolerance.to_bits(),
        value.full_residual_tolerance.to_bits(),
        value.full_fit_reduced_residual.to_bits(),
        value.full_fit_complete_residual.to_bits(),
        value.max_reduced_residual.to_bits(),
        value.max_complete_residual.to_bits(),
        value.max_leverage.to_bits(),
        value.max_reciprocal_residual.to_bits(),
        value.accounting_residual.to_bits(),
        value.cmg_edge_complexity.to_bits(),
        value.cmg_vertex_complexity.to_bits(),
        value.full_fit_weighted_rss.to_bits(),
    ]
}

#[derive(Debug)]
struct OwnedColumns {
    worker: Vec<f64>,
    firm: Vec<f64>,
    deletion: Vec<f64>,
    outcome: Vec<f64>,
    frequency: Vec<f64>,
    target_weight: Vec<f64>,
}

impl OwnedColumns {
    fn dense() -> Self {
        let workers = 12_usize;
        let firms = 4_usize;
        let rows = workers * firms;
        let mut value = Self {
            worker: Vec::with_capacity(rows),
            firm: Vec::with_capacity(rows),
            deletion: Vec::with_capacity(rows),
            outcome: Vec::with_capacity(rows),
            frequency: Vec::with_capacity(rows),
            target_weight: Vec::with_capacity(rows),
        };
        for worker in 0..workers {
            for firm in 0..firms {
                value.worker.push((worker + 1) as f64);
                value.firm.push((firm + 1) as f64);
                value.deletion.push((worker * firms + firm + 1) as f64);
                let sign = if (worker + firm) % 2 == 0 { 1.0 } else { -1.0 };
                value.outcome.push(sign * (2 * worker + firm + 1) as f64);
                value.frequency.push((firm % 2 + 1) as f64);
                value.target_weight.push((worker % 3 + firm + 1) as f64);
            }
        }
        value
    }

    fn disconnected_components() -> Self {
        Self {
            worker: vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 4.0],
            firm: vec![1.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 4.0, 5.0],
            deletion: (1..=10).map(f64::from).collect(),
            outcome: vec![1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 1.0, -1.0, 2.0, -2.0],
            frequency: vec![100.0, 100.0, 100.0, 100.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            target_weight: vec![1.0; 10],
        }
    }

    fn generic_dense() -> Self {
        let workers = 12_usize;
        let firms = 4_usize;
        let rows = workers * firms * 2;
        let mut value = Self {
            worker: Vec::with_capacity(rows),
            firm: Vec::with_capacity(rows),
            deletion: Vec::with_capacity(rows),
            outcome: Vec::with_capacity(rows),
            frequency: Vec::with_capacity(rows),
            target_weight: Vec::with_capacity(rows),
        };
        for worker in 0..workers {
            for firm in 0..firms {
                for replicate in 0..2 {
                    let row = value.worker.len();
                    value.worker.push((worker + 1) as f64);
                    value.firm.push((firm + 1) as f64);
                    value.deletion.push((worker * firms + firm + 1) as f64);
                    value.outcome.push(
                        0.7 * worker as f64 - 0.45 * firm as f64
                            + 0.3 * replicate as f64
                            + ((row * 7) % 5) as f64 / 11.0,
                    );
                    value.frequency.push(((row % 3) + 1) as f64);
                    value.target_weight.push(0.5 + ((row * 5) % 7) as f64 / 3.0);
                }
            }
        }
        value
    }

    fn structured_component() -> Self {
        Self::structured_component_sized(9, 9)
    }

    fn structured_component_sized(workers: usize, firms: usize) -> Self {
        let rows = workers * firms * 2;
        let mut value = Self {
            worker: Vec::with_capacity(rows),
            firm: Vec::with_capacity(rows),
            deletion: Vec::with_capacity(rows),
            outcome: Vec::with_capacity(rows),
            frequency: Vec::with_capacity(rows),
            target_weight: Vec::with_capacity(rows),
        };
        for worker in 0..workers {
            for firm in 0..firms {
                for replicate in 0..2 {
                    let row = value.worker.len();
                    value.worker.push((worker + 1) as f64);
                    value.firm.push((firm + 1) as f64);
                    value.deletion.push((row + 1) as f64);
                    let scale = 0.04 + 0.008 * worker as f64 + 0.005 * firm as f64;
                    let shock = (((row * 37 + 11) % 101) as f64 / 50.0 - 1.0) * scale;
                    value.outcome.push(
                        0.31 * worker as f64 - 0.23 * firm as f64 + 0.09 * replicate as f64 + shock,
                    );
                    value.frequency.push(1.0);
                    value
                        .target_weight
                        .push(0.75 + ((row * 13) % 29) as f64 / 31.0);
                }
            }
        }
        value
    }

    fn descriptor(&self) -> VckssEngineColumnsV1 {
        VckssEngineColumnsV1 {
            struct_size: bytes::<VckssEngineColumnsV1>(),
            reserved: 0,
            rows: self.worker.len() as u64,
            worker: self.worker.as_ptr(),
            firm: self.firm.as_ptr(),
            deletion: self.deletion.as_ptr(),
            outcome: self.outcome.as_ptr(),
            frequency: self.frequency.as_ptr(),
            target_weight: self.target_weight.as_ptr(),
        }
    }

    fn request(&self) -> VckssEnginePrepareRequestV1 {
        VckssEnginePrepareRequestV1 {
            abi_version: ABI_VERSION,
            struct_size: bytes::<VckssEnginePrepareRequestV1>(),
            rows: self.worker.len() as u64,
            cleanup_abandoned: 0,
            reserved: 0,
        }
    }

    fn request_v2(&self) -> VckssEnginePrepareRequestV2 {
        VckssEnginePrepareRequestV2 {
            abi_version: ABI_VERSION,
            struct_size: bytes::<VckssEnginePrepareRequestV2>(),
            rows: self.worker.len() as u64,
            cleanup_abandoned: 0,
            reserved: 0,
            memory_limit_bytes: 64_u64 << 20,
            caller_copy_bytes: self.worker.len() as u64 * 6 * 8,
        }
    }
}

fn direct_problem(columns: &OwnedColumns, controls: Vec<Vec<f64>>) -> CompressedProblem {
    let rows = columns.worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker: columns.worker.iter().map(|&value| value as u64).collect(),
            firm: columns.firm.iter().map(|&value| value as u64).collect(),
            deletion: columns.deletion.iter().map(|&value| value as u64).collect(),
            outcome: columns.outcome.clone(),
            frequency: columns
                .frequency
                .iter()
                .map(|&value| value as u64)
                .collect(),
            target_weight: columns.target_weight.clone(),
            controls,
        }
        .validate()
        .expect("direct exact fixture validates"),
    )
    .expect("direct exact fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("direct exact fixture compresses")
}

fn reset() {
    assert_eq!(vckss_rust_engine_clear_abandoned_v1(), ErrorCode::Ok as i32);
}

#[test]
fn public_abi_layout_and_structured_capabilities_are_frozen() {
    assert_eq!(size_of::<VckssBackendCapabilitiesV1>(), 32);
    assert_eq!(size_of::<VckssBackendRequestCapabilityRequestV1>(), 48);
    assert_eq!(size_of::<VckssBackendRequestCapabilityReceiptV1>(), 64);
    assert_eq!(size_of::<VckssBackendRequestCapabilityRequestV2>(), 88);
    assert_eq!(size_of::<VckssBackendRequestCapabilityReceiptV2>(), 104);
    assert_eq!(size_of::<VckssEnginePrepareRequestV1>(), 24);
    assert_eq!(size_of::<VckssEnginePrepareRequestV2>(), 40);
    assert_eq!(size_of::<VckssEnginePrepareRequestV3>(), 56);
    assert_eq!(size_of::<VckssEnginePrepareRequestV4>(), 64);
    assert_eq!(size_of::<VckssEnginePrepareRequestInterruptV1>(), 64);
    assert_eq!(size_of::<VckssEnginePrepareRequestInterruptV2>(), 80);
    assert_eq!(size_of::<VckssEnginePrepareRequestInterruptV3>(), 88);
    assert_eq!(size_of::<VckssEngineColumnsV1>(), 64);
    assert_eq!(size_of::<VckssEngineColumnsV2>(), 80);
    assert_eq!(size_of::<VckssEngineColumnsV3>(), 96);
    assert_eq!(offset_of!(VckssEngineColumnsV3, probe_order), 80);
    assert_eq!(size_of::<VckssEngineSolveRequestV1>(), 176);
    assert_eq!(size_of::<VckssEngineSolveRequestV2>(), 200);
    assert_eq!(size_of::<VckssEngineSolveRequestV3>(), 264);
    assert_eq!(size_of::<VckssEngineSolveRequestInterruptV1>(), 200);
    assert_eq!(size_of::<VckssEngineSolveRequestInterruptV2>(), 224);
    assert_eq!(size_of::<VckssEngineSolveRequestInterruptV3>(), 288);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV1>(), 72);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV2>(), 248);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV3>(), 256);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV4>(), 264);
    assert_eq!(size_of::<VckssEngineResultV1>(), 144);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV1>(), 272);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV2>(), 360);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV3>(), 384);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV4>(), 448);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV5>(), 536);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV6>(), 840);
    assert_eq!(size_of::<VckssEngineRhsReceiptV1>(), 48);
    assert_eq!(size_of::<VckssEngineRhsReceiptV2>(), 96);
    assert_eq!(size_of::<VckssEngineSnapshotV1>(), 24);
    assert_eq!(size_of::<VckssStayerAugmentationRequestV1>(), 32);
    assert_eq!(size_of::<VckssStayerAugmentationRequestInterruptV1>(), 56);
    assert_eq!(size_of::<VckssStayerAugmentationColumnsV1>(), 72);
    assert_eq!(size_of::<VckssStayerAugmentationReceiptV1>(), 192);
    assert_eq!(size_of::<VckssStayerHybridResultV1>(), 360);
    assert_eq!(size_of::<VckssPrepareRequestV1>(), 24);
    assert_eq!(size_of::<VckssColumnsV1>(), 64);
    assert_eq!(size_of::<VckssPreparationReceiptV1>(), 72);
    assert_eq!(size_of::<VckssSessionSnapshotV1>(), 24);
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestV2, memory_limit_bytes),
        24
    );
    assert_eq!(offset_of!(VckssEnginePrepareRequestV4, implicit_match), 56);
    assert_eq!(offset_of!(VckssEnginePrepareRequestInterruptV1, options), 0);
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, interrupt_poll),
        40
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, interrupt_context),
        48
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, checkpoint_interval),
        56
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, reserved),
        60
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV2, interrupt_poll),
        56
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV2, interrupt_context),
        64
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV2, checkpoint_interval),
        72
    );
    assert_eq!(
        offset_of!(VckssEnginePreparationReceiptV2, memory_limit_bytes),
        72
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestV1, cmg_memory_limit_bytes),
        168
    );
    assert_eq!(offset_of!(VckssEngineSolveRequestInterruptV1, options), 0);
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV1, interrupt_poll),
        176
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV1, interrupt_context),
        184
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV1, checkpoint_interval),
        192
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV2, interrupt_poll),
        200
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV2, interrupt_context),
        208
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV2, checkpoint_interval),
        216
    );
    assert_eq!(
        offset_of!(VckssBackendRequestCapabilityRequestV2, engine),
        48
    );
    assert_eq!(
        offset_of!(VckssBackendRequestCapabilityReceiptV2, engine),
        64
    );
    assert_eq!(offset_of!(VckssEngineSolveRequestV3, engine), 200);
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV3, interrupt_poll),
        264
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV1, cmg_dense_factor_bytes),
        264
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV2, full_fit_weighted_rss),
        272
    );
    assert_eq!(
        offset_of!(VckssEnginePreparationReceiptV3, target_weight_sum),
        248
    );
    assert_eq!(
        offset_of!(VckssEnginePreparationReceiptV4, controls_count),
        256
    );
    assert_eq!(offset_of!(VckssEngineDetailedReceiptV3, rng_contract), 360);
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV4, algorithm_requested),
        384
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV5, applicability_flags),
        448
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV5, actual_accounting_residual),
        528
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV6, engine_requested),
        536
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV6, rhs_v2_caller_copy_bytes),
        776
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV6, capability_schema),
        784
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV6, request_signature),
        832
    );
    assert_eq!(offset_of!(VckssEngineRhsReceiptV1, reduced_residual), 32);
    assert_eq!(offset_of!(VckssEngineRhsReceiptV2, status), 48);
    assert_eq!(ErrorCode::SingularInformation as i32, 82);
    assert_eq!(ErrorCode::InverseResidualFailed as i32, 83);
    assert_eq!(ErrorCode::AmbiguousControlBasis as i32, 84);
    assert_eq!(ErrorCode::UnverifiedDeletionRank as i32, 85);
    assert_eq!(ErrorCode::SymmetricEigensolverFailed as i32, 86);

    let mut capabilities = VckssBackendCapabilitiesV1::default();
    assert_eq!(
        vckss_rust_backend_capabilities_v1(
            &mut capabilities,
            bytes::<VckssBackendCapabilitiesV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(capabilities.struct_size, 32);
    assert_eq!(capabilities.abi_version, ABI_VERSION);
    let expected_ready_flags = if cfg!(any(target_os = "macos", target_os = "linux")) {
        1023
    } else {
        767
    };
    assert_eq!(capabilities.core_ready_flags, expected_ready_flags);
    assert_ne!(capabilities.core_ready_flags & VCKSS_CORE_JLA_PLAN_READY, 0);
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        assert_ne!(
            capabilities.core_ready_flags & VCKSS_CORE_FULL_CMG_V2_READY,
            0
        );
    } else {
        assert_eq!(
            capabilities.core_ready_flags & VCKSS_CORE_FULL_CMG_V2_READY,
            0
        );
    }
    assert_eq!(capabilities.support_flags, 38);
    assert_eq!(capabilities.deterministic_parallelism, 1);
    assert_eq!(capabilities.reserved, 0);
}

#[test]
fn v4_implicit_match_admission_is_exact_and_rejects_unsupported_preparation() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let rows = 10_u64;
    let caller_copy_bytes = rows * 7 * 8;
    let exact_forecast = caller_copy_bytes + rows * (768 + 16 + 16) + 4096;
    let mut request = VckssEnginePrepareRequestV4::default();
    request.v3.v2.rows = rows;
    request.v3.v2.caller_copy_bytes = caller_copy_bytes;
    request.v3.v2.memory_limit_bytes = exact_forecast;
    request.v3.deletion_mode = VCKSS_DELETION_MATCH;
    request.implicit_match = 1;

    assert_eq!(
        vckss_rust_engine_admit_prepare_v4(&request, 1),
        ErrorCode::Ok as i32
    );
    request.v3.v2.memory_limit_bytes = exact_forecast - 1;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v4(&request, 1),
        ErrorCode::ResourceLimit as i32
    );

    request.v3.v2.memory_limit_bytes = exact_forecast;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v4(&request, 0),
        ErrorCode::UnsupportedFeature as i32
    );
    request.v3.deletion_mode = VCKSS_DELETION_OBSERVATION;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v4(&request, 1),
        ErrorCode::UnsupportedFeature as i32
    );
    request.v3.deletion_mode = VCKSS_DELETION_MATCH;
    request.v3.controls_count = 1;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v4(&request, 1),
        ErrorCode::UnsupportedFeature as i32
    );
}

fn request_capability(
    request: VckssBackendRequestCapabilityRequestV1,
) -> VckssBackendRequestCapabilityReceiptV1 {
    let mut receipt = VckssBackendRequestCapabilityReceiptV1::default();
    assert_eq!(
        vckss_rust_backend_request_capability_v1(
            &request,
            &mut receipt,
            bytes::<VckssBackendRequestCapabilityReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.struct_size, 64);
    assert_eq!(receipt.abi_version, ABI_VERSION);
    assert_eq!(receipt.request_schema, request.request_schema);
    assert_eq!(receipt.algorithm, request.algorithm);
    assert_eq!(receipt.deletion_mode, request.deletion_mode);
    assert_eq!(receipt.nuisance_mode, request.nuisance_mode);
    assert_eq!(receipt.solver_route, request.solver_route);
    assert_eq!(receipt.rng_contract, request.rng_contract);
    assert_eq!(receipt.controls_count, request.controls_count);
    assert_eq!(receipt.frequency_use, request.frequency_use);
    assert_eq!(receipt.reserved, 0);
    receipt
}

#[test]
fn request_capability_matrix_is_compositional_and_fails_closed() {
    let exact = VckssBackendRequestCapabilityRequestV1::default();
    for deletion_mode in [VCKSS_DELETION_MATCH, VCKSS_DELETION_OBSERVATION] {
        for nuisance_mode in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            for solver_route in [VCKSS_ROUTE_AUTO, VCKSS_ROUTE_EXACT] {
                for controls_count in [0, 32] {
                    for frequency_use in [0, VCKSS_REQUEST_FREQUENCY_LITERAL] {
                        let receipt = request_capability(VckssBackendRequestCapabilityRequestV1 {
                            deletion_mode,
                            nuisance_mode,
                            solver_route,
                            controls_count,
                            frequency_use,
                            ..exact
                        });
                        assert_eq!(receipt.supported, 1);
                        assert_eq!(receipt.reason_code, VCKSS_REQUEST_REASON_SUPPORTED);
                        assert_eq!(receipt.profile_code, VCKSS_REQUEST_PROFILE_EXACT_V1);
                    }
                }
            }
        }
    }

    let jla = VckssBackendRequestCapabilityRequestV1 {
        algorithm: VCKSS_ALGORITHM_JLA,
        rng_contract: VCKSS_RNG_COUNTER_V1,
        ..VckssBackendRequestCapabilityRequestV1::default()
    };
    for solver_route in [
        VCKSS_ROUTE_AUTO,
        VCKSS_ROUTE_EXACT,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_ROUTE_CMG_PCG,
    ] {
        for frequency_use in [0, VCKSS_REQUEST_FREQUENCY_LITERAL] {
            let receipt = request_capability(VckssBackendRequestCapabilityRequestV1 {
                solver_route,
                frequency_use,
                ..jla
            });
            assert_eq!(receipt.supported, 1);
            assert_eq!(receipt.reason_code, VCKSS_REQUEST_REASON_SUPPORTED);
            assert_eq!(receipt.profile_code, VCKSS_REQUEST_PROFILE_JLA_COUNTER_V1);
        }
    }

    let invalid = [
        (
            VckssBackendRequestCapabilityRequestV1 {
                request_schema: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                algorithm: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                algorithm: VCKSS_ALGORITHM_AUTO,
                ..exact
            },
            VCKSS_REQUEST_REASON_ALGORITHM_AUTO_UNRESOLVED,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                deletion_mode: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_DELETION,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                nuisance_mode: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                solver_route: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                rng_contract: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                frequency_use: 99,
                ..exact
            },
            VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                controls_count: 33,
                ..exact
            },
            VCKSS_REQUEST_REASON_CONTROLS_LIMIT,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                rng_contract: VCKSS_RNG_COUNTER_V1,
                ..exact
            },
            VCKSS_REQUEST_REASON_EXACT_RNG,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                solver_route: VCKSS_ROUTE_DIAGONAL_PCG,
                ..exact
            },
            VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                deletion_mode: VCKSS_DELETION_OBSERVATION,
                ..jla
            },
            VCKSS_REQUEST_REASON_JLA_DELETION,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                nuisance_mode: VCKSS_NUISANCE_FIXED_OFFSET,
                ..jla
            },
            VCKSS_REQUEST_REASON_JLA_NUISANCE,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                controls_count: 1,
                ..jla
            },
            VCKSS_REQUEST_REASON_JLA_CONTROLS,
        ),
        (
            VckssBackendRequestCapabilityRequestV1 {
                rng_contract: VCKSS_RNG_NONE,
                ..jla
            },
            VCKSS_REQUEST_REASON_JLA_RNG,
        ),
    ];
    for (request, expected_reason) in invalid {
        let receipt = request_capability(request);
        assert_eq!(receipt.supported, 0);
        assert_eq!(receipt.reason_code, expected_reason);
        assert_eq!(receipt.profile_code, 0);
    }

    let reserved = VckssBackendRequestCapabilityRequestV1 {
        reserved: 1,
        ..exact
    };
    let mut output = VckssBackendRequestCapabilityReceiptV1::default();
    assert_eq!(
        vckss_rust_backend_request_capability_v1(
            &reserved,
            &mut output,
            bytes::<VckssBackendRequestCapabilityReceiptV1>(),
        ),
        ErrorCode::AbiMismatch as i32
    );
}

#[test]
fn request_capability_signature_is_stable_and_covers_every_field() {
    let request = VckssBackendRequestCapabilityRequestV1 {
        deletion_mode: VCKSS_DELETION_OBSERVATION,
        nuisance_mode: VCKSS_NUISANCE_FIXED_OFFSET,
        solver_route: VCKSS_ROUTE_EXACT,
        controls_count: 17,
        frequency_use: VCKSS_REQUEST_FREQUENCY_LITERAL,
        ..VckssBackendRequestCapabilityRequestV1::default()
    };
    let baseline = request_capability(request);
    assert_eq!(baseline.request_signature, 0x511d_0bcb_0a6f_e389);
    assert_eq!(baseline, request_capability(request));
    for changed in [
        VckssBackendRequestCapabilityRequestV1 {
            request_schema: 2,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            algorithm: VCKSS_ALGORITHM_JLA,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            deletion_mode: VCKSS_DELETION_MATCH,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            nuisance_mode: VCKSS_NUISANCE_JOINT,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            solver_route: VCKSS_ROUTE_AUTO,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            rng_contract: VCKSS_RNG_COUNTER_V1,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            controls_count: 18,
            ..request
        },
        VckssBackendRequestCapabilityRequestV1 {
            frequency_use: 0,
            ..request
        },
    ] {
        assert_ne!(
            baseline.request_signature,
            request_capability(changed).request_signature
        );
    }
}

fn request_capability_v2(
    request: VckssBackendRequestCapabilityRequestV2,
) -> VckssBackendRequestCapabilityReceiptV2 {
    let mut receipt = VckssBackendRequestCapabilityReceiptV2::default();
    assert_eq!(
        vckss_rust_backend_request_capability_v2(
            &request,
            &mut receipt,
            bytes::<VckssBackendRequestCapabilityReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.v1.struct_size, 104);
    assert_eq!(receipt.v1.abi_version, ABI_VERSION);
    assert_eq!(receipt.v1.request_schema, request.v1.request_schema);
    assert_eq!(receipt.v1.algorithm, request.v1.algorithm);
    assert_eq!(receipt.v1.deletion_mode, request.v1.deletion_mode);
    assert_eq!(receipt.v1.nuisance_mode, request.v1.nuisance_mode);
    assert_eq!(receipt.v1.solver_route, request.v1.solver_route);
    assert_eq!(receipt.v1.rng_contract, request.v1.rng_contract);
    assert_eq!(receipt.v1.controls_count, request.v1.controls_count);
    assert_eq!(receipt.v1.frequency_use, request.v1.frequency_use);
    assert_eq!(receipt.engine, request.engine);
    assert_eq!(receipt.batch_mode, request.batch_mode);
    assert_eq!(receipt.stayers_mode, request.stayers_mode);
    assert_eq!(receipt.target_weight_mode, request.target_weight_mode);
    assert_eq!(receipt.deletion_unit_source, request.deletion_unit_source);
    assert_eq!(receipt.probeorder_supplied, request.probeorder_supplied);
    assert_eq!(receipt.wallseconds_supplied, request.wallseconds_supplied);
    assert_eq!(receipt.physical_limit, request.physical_limit);
    assert_eq!(receipt.v1.reserved, 0);
    assert_eq!(receipt.reserved_2, 0);
    receipt
}

fn generic_capability_request() -> VckssBackendRequestCapabilityRequestV2 {
    VckssBackendRequestCapabilityRequestV2 {
        v1: VckssBackendRequestCapabilityRequestV1 {
            struct_size: bytes::<VckssBackendRequestCapabilityRequestV2>(),
            request_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V2,
            algorithm: VCKSS_ALGORITHM_JLA,
            deletion_mode: VCKSS_DELETION_MATCH,
            nuisance_mode: VCKSS_NUISANCE_JOINT,
            solver_route: VCKSS_ROUTE_DIAGONAL_PCG,
            rng_contract: VCKSS_RNG_COUNTER_V1,
            frequency_use: VCKSS_REQUEST_FREQUENCY_LITERAL,
            ..VckssBackendRequestCapabilityRequestV1::default()
        },
        engine: VCKSS_ENGINE_GENERIC,
        batch_mode: VCKSS_BATCH_MODE_EXPLICIT,
        stayers_mode: VCKSS_STAYERS_MOVERS,
        target_weight_mode: VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT,
        deletion_unit_source: VCKSS_DELETION_SOURCE_CELL_DEFAULT,
        physical_limit: 50_000_000,
        ..VckssBackendRequestCapabilityRequestV2::default()
    }
}

#[test]
fn generic_request_capability_v2_is_exhaustive_engine_aware_and_signature_bound() {
    let base = generic_capability_request();
    let mut observed_signatures = std::collections::BTreeSet::new();
    for deletion_mode in [VCKSS_DELETION_MATCH, VCKSS_DELETION_OBSERVATION] {
        let deletion_sources: &[u32] = if deletion_mode == VCKSS_DELETION_MATCH {
            &[
                VCKSS_DELETION_SOURCE_CELL_DEFAULT,
                VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT,
            ]
        } else {
            &[VCKSS_DELETION_SOURCE_OBSERVATION_ROW]
        };
        for &deletion_unit_source in deletion_sources {
            for nuisance_mode in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
                for controls_count in [0, 32] {
                    for frequency_use in [0, VCKSS_REQUEST_FREQUENCY_LITERAL] {
                        for target_weight_mode in [
                            VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT,
                            VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT,
                        ] {
                            for physical_limit in [1, MAX_EXACT_BINARY64_INTEGER] {
                                let request = VckssBackendRequestCapabilityRequestV2 {
                                    v1: VckssBackendRequestCapabilityRequestV1 {
                                        deletion_mode,
                                        nuisance_mode,
                                        controls_count,
                                        frequency_use,
                                        ..base.v1
                                    },
                                    deletion_unit_source,
                                    target_weight_mode,
                                    physical_limit,
                                    ..base
                                };
                                let receipt = request_capability_v2(request);
                                assert_eq!(receipt.v1.supported, 1);
                                assert_eq!(receipt.v1.reason_code, VCKSS_REQUEST_REASON_SUPPORTED);
                                assert_eq!(
                                    receipt.v1.profile_code,
                                    VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1
                                );
                                assert!(observed_signatures.insert(receipt.v1.request_signature));
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(observed_signatures.len(), 96);

    let invalid = [
        (
            VckssBackendRequestCapabilityRequestV2 { engine: 99, ..base },
            VCKSS_REQUEST_REASON_UNKNOWN_ENGINE,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                batch_mode: 99,
                ..base
            },
            VCKSS_REQUEST_REASON_UNKNOWN_BATCH_MODE,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                stayers_mode: 99,
                ..base
            },
            VCKSS_REQUEST_REASON_UNKNOWN_STAYERS_MODE,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                target_weight_mode: 99,
                ..base
            },
            VCKSS_REQUEST_REASON_UNKNOWN_TARGET_WEIGHT_MODE,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                deletion_unit_source: 99,
                ..base
            },
            VCKSS_REQUEST_REASON_UNKNOWN_DELETION_UNIT_SOURCE,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                probeorder_supplied: 1,
                ..base
            },
            VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                wallseconds_supplied: 1,
                ..base
            },
            VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                physical_limit: 0,
                ..base
            },
            VCKSS_REQUEST_REASON_PHYSICAL_LIMIT,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                physical_limit: MAX_EXACT_BINARY64_INTEGER + 1,
                ..base
            },
            VCKSS_REQUEST_REASON_PHYSICAL_LIMIT,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                v1: VckssBackendRequestCapabilityRequestV1 {
                    solver_route: VCKSS_ROUTE_EXACT,
                    ..base.v1
                },
                ..base
            },
            VCKSS_REQUEST_REASON_JLA_GENERIC_SOLVER_ROUTE,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                batch_mode: VCKSS_BATCH_MODE_AUTO,
                ..base
            },
            VCKSS_REQUEST_REASON_BATCH_MODE_UNSUPPORTED,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                stayers_mode: VCKSS_STAYERS_ALL,
                ..base
            },
            VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                deletion_unit_source: VCKSS_DELETION_SOURCE_OBSERVATION_ROW,
                ..base
            },
            VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH,
        ),
        (
            VckssBackendRequestCapabilityRequestV2 {
                v1: VckssBackendRequestCapabilityRequestV1 {
                    deletion_mode: VCKSS_DELETION_OBSERVATION,
                    ..base.v1
                },
                ..base
            },
            VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH,
        ),
    ];
    for (request, reason) in invalid {
        let receipt = request_capability_v2(request);
        assert_eq!(receipt.v1.supported, 0);
        assert_eq!(receipt.v1.reason_code, reason);
        assert_eq!(receipt.v1.profile_code, 0);
    }

    let baseline = request_capability_v2(base).v1.request_signature;
    let changed = [
        VckssBackendRequestCapabilityRequestV2 {
            engine: VCKSS_ENGINE_COMPRESSED,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            batch_mode: VCKSS_BATCH_MODE_AUTO,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            stayers_mode: VCKSS_STAYERS_ALL,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            target_weight_mode: VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            deletion_unit_source: VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            probeorder_supplied: 1,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            wallseconds_supplied: 1,
            ..base
        },
        VckssBackendRequestCapabilityRequestV2 {
            physical_limit: base.physical_limit + 1,
            ..base
        },
    ];
    for request in changed {
        assert_ne!(
            baseline,
            request_capability_v2(request).v1.request_signature
        );
    }
}

#[test]
fn additive_probe_order_preparation_is_memory_bound_and_fails_closed() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let probe_order = (0..columns.worker.len())
        .map(|row| (columns.worker.len() - row) as f64)
        .collect::<Vec<_>>();
    let mut descriptor = VckssEngineColumnsV3 {
        v2: VckssEngineColumnsV2 {
            v1: columns.descriptor(),
            controls: ptr::null(),
            controls_count: 0,
            reserved_2: 0,
        },
        probe_order: probe_order.as_ptr(),
        probeorder_supplied: 1,
        reserved_3: 0,
    };
    descriptor.v2.v1.struct_size = bytes::<VckssEngineColumnsV3>();
    let mut request = VckssEnginePrepareRequestInterruptV2::default();
    assert_eq!(
        vckss_rust_engine_default_prepare_request_interrupt_v2(
            &mut request,
            bytes::<VckssEnginePrepareRequestInterruptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    let rows = columns.worker.len() as u64;
    request.options.v2.rows = rows;
    request.options.v2.memory_limit_bytes = 64_u64 << 20;
    request.options.v2.caller_copy_bytes = rows * 7 * 8;
    assert_eq!(
        vckss_rust_engine_admit_prepare_probe_order_v1(&request.options, 1),
        ErrorCode::Ok as i32
    );
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v3(
            &request,
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_ne!(generation, 0);
    let mut receipt = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.caller_copy_bytes, rows * 7 * 8);
    assert_eq!(
        receipt.preparation_peak_forecast_bytes,
        rows * 7 * 8 + rows * (768 + 16) + 4096
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    descriptor.probeorder_supplied = 0;
    request.options.v2.caller_copy_bytes = rows * 6 * 8;
    generation = u64::MAX;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v3(
            &request,
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(generation, 0);
    reset();
}

#[test]
fn additive_v3_input_and_v2_solve_run_exact_controls_and_observation_deletion() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let control = columns
        .worker
        .iter()
        .zip(&columns.firm)
        .enumerate()
        .map(|(row, (&worker, &firm))| worker * firm + (row % 3) as f64)
        .collect::<Vec<_>>();
    let control_pointers = [control.as_ptr()];
    let mut descriptor = VckssEngineColumnsV2 {
        v1: columns.descriptor(),
        controls: control_pointers.as_ptr(),
        controls_count: 1,
        reserved_2: 0,
    };
    descriptor.v1.struct_size = bytes::<VckssEngineColumnsV2>();
    let mut prepare = VckssEnginePrepareRequestV3::default();
    assert_eq!(
        vckss_rust_engine_default_prepare_request_v3(
            &mut prepare,
            bytes::<VckssEnginePrepareRequestV3>(),
        ),
        ErrorCode::Ok as i32
    );
    prepare.v2.rows = columns.worker.len() as u64;
    prepare.v2.memory_limit_bytes = 64_u64 << 20;
    prepare.v2.caller_copy_bytes = columns.worker.len() as u64 * 7 * 8;
    prepare.controls_count = 1;
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v3(&prepare, &descriptor, &mut generation, bytes::<u64>(),),
        ErrorCode::Ok as i32
    );
    let mut solve = VckssEngineSolveRequestV2::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v2(
            &mut solve,
            bytes::<VckssEngineSolveRequestV2>(),
        ),
        ErrorCode::Ok as i32
    );
    solve.algorithm = VCKSS_ALGORITHM_EXACT;
    solve.nuisance_mode = VCKSS_NUISANCE_FIXED_OFFSET;
    solve.exact_estimator_limit = 100;
    solve.v1.probes = 0;
    solve.v1.pcg_tolerance = 1.0e-8;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::Ok as i32
    );
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::Ok as i32
    );
    assert!(result.corrected.total.is_finite());
    assert_eq!(result.numerical_mcse, Default::default());
    let mut exact_receipt = VckssEngineDetailedReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v4(
            generation,
            &mut exact_receipt,
            bytes::<VckssEngineDetailedReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(exact_receipt.algorithm_selected, VCKSS_ALGORITHM_EXACT);
    assert_eq!(exact_receipt.nuisance_mode, VCKSS_NUISANCE_FIXED_OFFSET);
    assert_eq!(exact_receipt.parameters, 15);
    assert_eq!(exact_receipt.full_parameters, 16);
    assert_eq!(exact_receipt.correction_parameters, 15);
    assert_eq!(exact_receipt.v3.v2.probes_requested, 0);
    assert_eq!(exact_receipt.v3.v2.leverage_rhs_count, 0);
    assert_eq!(exact_receipt.v3.v2.target_rhs_count, 0);
    assert_eq!(exact_receipt.v3.rhs_receipt_rows, 0);
    assert!(exact_receipt.v3.v2.full_fit_weighted_rss.is_finite());
    assert!(exact_receipt.exact_peak_forecast_bytes > 0);

    let direct = run_exact_estimator(
        &direct_problem(&columns, vec![control.clone()]),
        ExactEstimatorOptions {
            nuisance: NuisanceMode::FixedOffset,
            solver_tolerance: solve.v1.pcg_tolerance,
            exact_limit: 100,
            memory_limit_bytes: exact_receipt.v3.v2.memory_limit_bytes,
            prepared_persistent_bytes: exact_receipt.v3.v2.prepared_resident_bytes,
            ..ExactEstimatorOptions::default()
        },
    )
    .expect("direct fixed-offset exact result");
    let exported = exact_receipt.v3.v2;
    let maximum_fit = direct
        .receipt
        .full_fit_relres
        .max(direct.receipt.working_fit_relres);
    assert_eq!(
        exported.full_residual_tolerance,
        direct.receipt.fit_residual_tolerance
    );
    assert_eq!(
        exported.full_fit_complete_residual,
        direct.receipt.full_fit_relres
    );
    assert_eq!(exported.max_complete_residual, maximum_fit);
    assert_eq!(
        exported.max_reciprocal_residual,
        direct.receipt.maker_relres
    );
    assert_eq!(exact_receipt.inverse_relres, direct.receipt.inverse_relres);
    // The frozen reduced fields conservatively mirror the stronger complete
    // fit certificates for exact, never an unrelated inverse certificate.
    assert_eq!(
        exported.full_fit_reduced_residual,
        exported.full_fit_complete_residual
    );
    assert_eq!(
        exported.max_reduced_residual,
        exported.max_complete_residual
    );
    assert!(
        exact_receipt.inverse_relres != exported.full_fit_complete_residual
            || exact_receipt.inverse_relres != exported.max_reciprocal_residual
    );
    let mut exact_receipt_v5 = VckssEngineDetailedReceiptV5::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v5(
            generation,
            &mut exact_receipt_v5,
            bytes::<VckssEngineDetailedReceiptV5>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(exact_receipt_v5.v4, exact_receipt);
    assert_ne!(
        exact_receipt_v5.applicability_flags & VCKSS_EXACT_DIAGNOSTIC_INVERSE_SQRT,
        0
    );
    assert_ne!(
        exact_receipt_v5.applicability_flags & VCKSS_EXACT_DIAGNOSTIC_MAKER,
        0
    );
    assert_ne!(
        exact_receipt_v5.applicability_flags & VCKSS_EXACT_DIAGNOSTIC_CONTROL_BASIS,
        0
    );
    assert_ne!(
        exact_receipt_v5.applicability_flags & VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING,
        0
    );
    assert_eq!(
        exact_receipt_v5.working_fit_complete_residual,
        direct.receipt.working_fit_relres
    );
    assert_eq!(
        exact_receipt_v5.inverse_sqrt_relres,
        direct.receipt.inverse_sqrt_relres
    );
    assert_eq!(exact_receipt_v5.maker_relres, direct.receipt.maker_relres);
    assert_eq!(
        exact_receipt_v5.control_basis_relres,
        direct.receipt.control_basis_relres
    );
    assert_eq!(
        exact_receipt_v5.control_basis_forward_error,
        direct.receipt.control_basis_forward_error
    );
    assert_eq!(
        exact_receipt_v5.deletion_rank_gap,
        direct.receipt.deletion_rank_gap
    );
    assert_eq!(
        exact_receipt_v5.firm_zero_sum_residual,
        direct.receipt.firm_zero_sum_residual
    );
    assert_eq!(
        exact_receipt_v5.fit_peak_forecast_bytes,
        direct.receipt.fit_peak_forecast_bytes
    );
    assert_eq!(
        exact_receipt_v5.correction_peak_forecast_bytes,
        direct.receipt.correction_peak_forecast_bytes
    );
    let expected_accounting = [direct.plugin, direct.correction, direct.corrected]
        .into_iter()
        .map(|value| (value.total - value.worker - value.firm - 2.0 * value.covariance).abs())
        .fold(0.0_f64, f64::max);
    assert_eq!(
        exact_receipt_v5.actual_accounting_residual,
        expected_accounting
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    prepare.v2.caller_copy_bytes = columns.worker.len() as u64 * 7 * 8;
    prepare.deletion_mode = VCKSS_DELETION_OBSERVATION;
    prepare.controls_count = 1;
    generation = 0;
    assert_eq!(
        vckss_rust_engine_prepare_v3(&prepare, &descriptor, &mut generation, bytes::<u64>(),),
        ErrorCode::Ok as i32
    );
    solve.v1.deletion_mode = VCKSS_DELETION_OBSERVATION;
    solve.nuisance_mode = 1;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::Ok as i32
    );
    assert!(result.corrected.total.is_finite());
    exact_receipt = VckssEngineDetailedReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v4(
            generation,
            &mut exact_receipt,
            bytes::<VckssEngineDetailedReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(exact_receipt.deletion_mode, VCKSS_DELETION_OBSERVATION);
    assert_eq!(exact_receipt.parameters, 16);
    assert_eq!(exact_receipt.full_parameters, 16);
    assert_eq!(exact_receipt.correction_parameters, 16);
    assert_eq!(exact_receipt.v3.rhs_receipt_rows, 0);
    assert_eq!(exact_receipt.v3.v2.max_reciprocal_residual, 0.0);
    exact_receipt_v5 = VckssEngineDetailedReceiptV5::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v5(
            generation,
            &mut exact_receipt_v5,
            bytes::<VckssEngineDetailedReceiptV5>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(exact_receipt_v5.v4, exact_receipt);
    assert_eq!(
        exact_receipt_v5.applicability_flags
            & (VCKSS_EXACT_DIAGNOSTIC_INVERSE_SQRT | VCKSS_EXACT_DIAGNOSTIC_MAKER),
        0
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn additive_exact_control_lifecycle_is_interruptible_without_partial_state() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let control = columns
        .worker
        .iter()
        .zip(&columns.firm)
        .map(|(&worker, &firm)| worker * firm)
        .collect::<Vec<_>>();
    let control_pointers = [control.as_ptr()];
    let mut descriptor = VckssEngineColumnsV2 {
        v1: columns.descriptor(),
        controls: control_pointers.as_ptr(),
        controls_count: 1,
        reserved_2: 0,
    };
    descriptor.v1.struct_size = bytes::<VckssEngineColumnsV2>();

    let mut prepare = VckssEnginePrepareRequestInterruptV2::default();
    assert_eq!(
        vckss_rust_engine_default_prepare_request_interrupt_v2(
            &mut prepare,
            bytes::<VckssEnginePrepareRequestInterruptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(prepare.options.v2.struct_size, 80);
    prepare.options.v2.rows = columns.worker.len() as u64;
    prepare.options.v2.memory_limit_bytes = 64_u64 << 20;
    prepare.options.v2.caller_copy_bytes = columns.worker.len() as u64 * 7 * 8;
    prepare.options.controls_count = 1;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v3(&prepare.options),
        ErrorCode::Ok as i32
    );

    let mut prepare_poll = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    prepare.interrupt_poll = Some(injected_poll);
    prepare.interrupt_context = (&mut prepare_poll as *mut PollState).cast();
    prepare.checkpoint_interval = 1;
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v2(
            &prepare,
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(generation, 0);
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 0);

    prepare.interrupt_poll = None;
    prepare.interrupt_context = ptr::null_mut();
    prepare.checkpoint_interval = 0;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v2(
            &prepare,
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );

    let mut solve = VckssEngineSolveRequestInterruptV2::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v2(
            &mut solve,
            bytes::<VckssEngineSolveRequestInterruptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(solve.options.v1.struct_size, 224);
    solve.options.algorithm = VCKSS_ALGORITHM_EXACT;
    solve.options.nuisance_mode = VCKSS_NUISANCE_FIXED_OFFSET;
    solve.options.exact_estimator_limit = 100;
    let mut solve_poll = PollState {
        calls: 0,
        stop_at: 3,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    solve.interrupt_poll = Some(injected_poll);
    solve.interrupt_context = (&mut solve_poll as *mut PollState).cast();
    solve.checkpoint_interval = 1;
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v2(generation, &solve),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(solve_poll.calls, 3);
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn exact_v2_preflights_limits_and_forwards_the_solver_tolerance_gate() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut solve = VckssEngineSolveRequestV2 {
        v1: VckssEngineSolveRequestV1 {
            struct_size: bytes::<VckssEngineSolveRequestV2>(),
            probes: 0,
            ..VckssEngineSolveRequestV1::default()
        },
        algorithm: VCKSS_ALGORITHM_EXACT,
        exact_estimator_limit: 100,
        ..VckssEngineSolveRequestV2::default()
    };

    solve.v1.pcg_tolerance = 0.1;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::InvalidInput as i32
    );
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error
        .to_string_lossy()
        .contains("solver tolerance must be finite"));
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 1, "preflight must preserve prepared state");

    solve.v1.pcg_tolerance = 1.0e-7;
    solve.exact_estimator_limit = 1;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::InvalidInput as i32
    );
    solve.exact_estimator_limit = 100;
    solve.blocksize_limit = 0;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::InvalidInput as i32
    );

    solve.blocksize_limit = 5_000;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::Ok as i32
    );
    let mut receipt = VckssEngineDetailedReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v4(
            generation,
            &mut receipt,
            bytes::<VckssEngineDetailedReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.v3.v2.full_residual_tolerance, 1.0e-6);
    assert!(receipt.v3.v2.full_fit_complete_residual <= receipt.v3.v2.full_residual_tolerance);
    assert!(receipt.v3.v2.max_complete_residual <= receipt.v3.v2.full_residual_tolerance);
    assert_eq!(receipt.v3.v2.probes_requested, 0);
    assert_eq!(receipt.v3.rhs_receipt_rows, 0);
    let exact_memory_limit = receipt.exact_peak_forecast_bytes;
    assert!(exact_memory_limit > receipt.v3.v2.preparation_peak_forecast_bytes);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    for (memory_limit, expected) in [
        (exact_memory_limit, ErrorCode::Ok),
        (exact_memory_limit - 1, ErrorCode::ResourceLimit),
    ] {
        let mut prepare_request = columns.request_v2();
        prepare_request.memory_limit_bytes = memory_limit;
        let mut generation = 0_u64;
        assert_eq!(
            vckss_rust_engine_prepare_v2(
                &prepare_request,
                &columns.descriptor(),
                &mut generation,
                bytes::<u64>(),
            ),
            ErrorCode::Ok as i32
        );
        assert_eq!(
            vckss_rust_engine_solve_v2(generation, &solve),
            expected as i32
        );
        assert_eq!(
            vckss_rust_engine_release_v1(generation),
            ErrorCode::Ok as i32
        );
    }
}

#[test]
fn repaired_exact_statuses_cross_the_structural_error_transport_unchanged() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let first = (0..columns.worker.len())
        .map(|row| 1.0 + row as f64)
        .collect::<Vec<_>>();
    let second = first.iter().map(|value| 2.0 * value).collect::<Vec<_>>();
    let singular_controls = vec![first, second];
    let generation = prepare_with_controls(&columns, &singular_controls, VCKSS_DELETION_MATCH);
    let solve = VckssEngineSolveRequestV2 {
        v1: VckssEngineSolveRequestV1 {
            struct_size: bytes::<VckssEngineSolveRequestV2>(),
            probes: 0,
            ..VckssEngineSolveRequestV1::default()
        },
        algorithm: VCKSS_ALGORITHM_EXACT,
        exact_estimator_limit: 100,
        ..VckssEngineSolveRequestV2::default()
    };
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::SingularInformation as i32
    );
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error
        .to_string_lossy()
        .starts_with("SINGULAR_INFORMATION [control_basis]:"));
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let controls = (0..33)
        .map(|control| {
            (0..columns.worker.len())
                .map(|row| 1.0 + control as f64 + row as f64)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let generation = prepare_with_controls(&columns, &controls, VCKSS_DELETION_MATCH);
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &solve),
        ErrorCode::AmbiguousControlBasis as i32
    );
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error
        .to_string_lossy()
        .starts_with("AMBIGUOUS_CONTROL_BASIS [control_basis]:"));
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn frozen_session_aliases_share_the_engine_registry_and_error_slot() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let request = columns.request();
    let descriptor = columns.descriptor();

    let mut session_generation = 0_u64;
    assert_eq!(
        vckss_rust_session_prepare_v1(&request, &descriptor, &mut session_generation),
        ErrorCode::Ok as i32
    );
    let mut engine_snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut engine_snapshot, bytes::<VckssEngineSnapshotV1>(),),
        ErrorCode::Ok as i32
    );
    assert_eq!(engine_snapshot.generation, session_generation);
    assert_eq!(engine_snapshot.state, 1);
    let mut legacy_receipt = VckssPreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_session_preparation_receipt_v1(session_generation, &mut legacy_receipt),
        ErrorCode::Ok as i32
    );
    assert_eq!(legacy_receipt.generation, session_generation);
    assert_eq!(legacy_receipt.input_rows, columns.worker.len() as u64);
    assert_eq!(
        vckss_rust_engine_release_v1(session_generation),
        ErrorCode::Ok as i32
    );

    let engine_generation = prepare(&columns);
    let mut legacy_snapshot = VckssSessionSnapshotV1::default();
    assert_eq!(
        vckss_rust_session_snapshot_v1(&mut legacy_snapshot),
        ErrorCode::Ok as i32
    );
    assert_eq!(legacy_snapshot.generation, engine_generation);
    assert_eq!(legacy_snapshot.state, 1);
    assert_eq!(
        vckss_rust_session_release_v1(engine_generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_session_clear_abandoned_v1(),
        ErrorCode::Ok as i32
    );
    let error = unsafe { CStr::from_ptr(vckss_rust_session_last_error()) };
    assert_eq!(error.to_str().expect("session error utf-8"), "OK");
}

fn prepare(columns: &OwnedColumns) -> u64 {
    let mut generation = 0_u64;
    let request = columns.request_v2();
    assert_eq!(
        vckss_rust_engine_admit_prepare_v2(&request),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_prepare_v2(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    generation
}

fn prepare_with_controls(columns: &OwnedColumns, controls: &[Vec<f64>], deletion_mode: u32) -> u64 {
    prepare_with_controls_memory(columns, controls, deletion_mode, 64_u64 << 20)
}

fn prepare_with_controls_memory(
    columns: &OwnedColumns,
    controls: &[Vec<f64>],
    deletion_mode: u32,
    memory_limit_bytes: u64,
) -> u64 {
    let control_pointers = controls
        .iter()
        .map(Vec::as_ptr)
        .collect::<Vec<*const f64>>();
    let mut descriptor = VckssEngineColumnsV2 {
        v1: columns.descriptor(),
        controls: if control_pointers.is_empty() {
            ptr::null()
        } else {
            control_pointers.as_ptr()
        },
        controls_count: u32::try_from(controls.len()).expect("control count"),
        reserved_2: 0,
    };
    descriptor.v1.struct_size = bytes::<VckssEngineColumnsV2>();
    let mut request = VckssEnginePrepareRequestV3::default();
    request.v2.rows = columns.worker.len() as u64;
    request.v2.memory_limit_bytes = memory_limit_bytes;
    request.v2.caller_copy_bytes = columns.worker.len() as u64
        * u64::try_from(6 + controls.len()).expect("numeric columns")
        * 8;
    request.deletion_mode = deletion_mode;
    request.controls_count = u32::try_from(controls.len()).expect("control count");
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v3(&request, &descriptor, &mut generation, bytes::<u64>(),),
        ErrorCode::Ok as i32
    );
    generation
}

fn prepare_with_implicit_match_probe_order_memory(
    columns: &OwnedColumns,
    memory_limit_bytes: u64,
) -> u64 {
    let probe_order = (0..columns.worker.len())
        .map(|row| (row + 1) as f64)
        .collect::<Vec<_>>();
    let mut descriptor = VckssEngineColumnsV3 {
        v2: VckssEngineColumnsV2 {
            v1: columns.descriptor(),
            controls: ptr::null(),
            controls_count: 0,
            reserved_2: 0,
        },
        probe_order: probe_order.as_ptr(),
        probeorder_supplied: 1,
        reserved_3: 0,
    };
    descriptor.v2.v1.struct_size = bytes::<VckssEngineColumnsV3>();
    let mut request = VckssEnginePrepareRequestInterruptV3::default();
    assert_eq!(
        vckss_rust_engine_default_prepare_request_interrupt_v3(
            &mut request,
            bytes::<VckssEnginePrepareRequestInterruptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    let rows = columns.worker.len() as u64;
    request.options.v3.v2.rows = rows;
    request.options.v3.v2.memory_limit_bytes = memory_limit_bytes;
    request.options.v3.v2.caller_copy_bytes = rows * 7 * 8;
    request.options.v3.deletion_mode = VCKSS_DELETION_MATCH;
    request.options.v3.controls_count = 0;
    request.options.implicit_match = 1;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v4(&request.options, 1),
        ErrorCode::Ok as i32
    );
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v4(
            &request,
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    let mut receipt = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        receipt.preparation_peak_forecast_bytes,
        rows * 7 * 8 + rows * (768 + 16 + 16) + 4096
    );
    generation
}

fn one_generic_control(columns: &OwnedColumns) -> Vec<f64> {
    columns
        .worker
        .iter()
        .zip(&columns.firm)
        .enumerate()
        .map(|(row, (&worker, &firm))| {
            let replicate = (row % 2) as f64;
            (worker - 0.4 * firm) * (replicate + 1.0) + ((row * 3) % 7) as f64 / 17.0
        })
        .collect()
}

fn generic_solve_request(
    deletion_mode: u32,
    nuisance_mode: u32,
    controls_count: u32,
    probes: u32,
) -> (
    VckssBackendRequestCapabilityReceiptV2,
    VckssEngineSolveRequestV3,
) {
    let deletion_unit_source = if deletion_mode == VCKSS_DELETION_MATCH {
        VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
    } else {
        VCKSS_DELETION_SOURCE_OBSERVATION_ROW
    };
    let capability_request = VckssBackendRequestCapabilityRequestV2 {
        v1: VckssBackendRequestCapabilityRequestV1 {
            struct_size: bytes::<VckssBackendRequestCapabilityRequestV2>(),
            request_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V2,
            algorithm: VCKSS_ALGORITHM_JLA,
            deletion_mode,
            nuisance_mode,
            solver_route: VCKSS_ROUTE_DIAGONAL_PCG,
            rng_contract: VCKSS_RNG_COUNTER_V1,
            controls_count,
            frequency_use: VCKSS_REQUEST_FREQUENCY_LITERAL,
            ..VckssBackendRequestCapabilityRequestV1::default()
        },
        engine: VCKSS_ENGINE_GENERIC,
        batch_mode: VCKSS_BATCH_MODE_EXPLICIT,
        stayers_mode: VCKSS_STAYERS_MOVERS,
        target_weight_mode: VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT,
        deletion_unit_source,
        physical_limit: 50_000_000,
        ..VckssBackendRequestCapabilityRequestV2::default()
    };
    let capability = request_capability_v2(capability_request);
    assert_eq!(capability.v1.supported, 1);

    let mut request = VckssEngineSolveRequestV3::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v3(
            &mut request,
            bytes::<VckssEngineSolveRequestV3>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(request.v2.v1.struct_size, 264);
    request.v2.v1.seed = 81_227;
    request.v2.v1.probes = probes;
    request.v2.v1.leverage_batch_width = 2;
    request.v2.v1.target_batch_width = 3;
    request.v2.v1.deletion_mode = deletion_mode;
    request.v2.v1.pcg_tolerance = 1.0e-12;
    request.v2.v1.maximum_iterations = 10_000;
    request.v2.v1.residual_replacement_interval = 7;
    request.v2.algorithm = VCKSS_ALGORITHM_JLA;
    request.v2.nuisance_mode = nuisance_mode;
    request.engine = capability.engine;
    request.batch_mode = capability.batch_mode;
    request.stayers_mode = capability.stayers_mode;
    request.target_weight_mode = capability.target_weight_mode;
    request.deletion_unit_source = capability.deletion_unit_source;
    request.probeorder_supplied = capability.probeorder_supplied;
    request.wallseconds_supplied = capability.wallseconds_supplied;
    request.capability_schema = capability.v1.request_schema;
    request.capability_profile = capability.v1.profile_code;
    request.frequency_use = capability.v1.frequency_use;
    request.physical_limit = capability.physical_limit;
    request.request_signature = capability.v1.request_signature;
    (capability, request)
}

fn solve_generic_request(
    generation: u64,
    deletion_mode: u32,
    nuisance_mode: u32,
    controls_count: u32,
    probes: u32,
) -> VckssBackendRequestCapabilityReceiptV2 {
    let (capability, request) =
        generic_solve_request(deletion_mode, nuisance_mode, controls_count, probes);
    let status = vckss_rust_engine_solve_v3(generation, &request);
    // SAFETY: engine error strings are thread-local and remain valid until
    // this thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert_eq!(
        status,
        ErrorCode::Ok as i32,
        "generic solve failed: {}",
        error.to_string_lossy()
    );
    capability
}

fn assert_generic_rhs_surface(
    generation: u64,
    deletion_mode: u32,
    nuisance_mode: u32,
    controls_count: usize,
    probes: usize,
) -> (
    VckssEngineResultV1,
    VckssEngineDetailedReceiptV6,
    Vec<VckssEngineRhsReceiptV2>,
) {
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::Ok as i32
    );
    for value in [result.plugin, result.correction, result.corrected] {
        assert!(value.worker.is_finite());
        assert!(value.firm.is_finite());
        assert!(value.covariance.is_finite());
        assert!(value.total.is_finite());
        assert!((value.total - value.worker - value.firm - 2.0 * value.covariance).abs() <= 1e-10);
    }

    let mut detailed = VckssEngineDetailedReceiptV6::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v6(
            generation,
            &mut detailed,
            bytes::<VckssEngineDetailedReceiptV6>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(detailed.engine_requested, VCKSS_ENGINE_GENERIC);
    assert_eq!(detailed.engine_selected, VCKSS_ENGINE_GENERIC);
    assert_eq!(detailed.controls_count as usize, controls_count);
    assert_eq!(detailed.rhs_receipt_schema, 2);
    assert_eq!(
        detailed.control_projection_rhs_count as usize,
        controls_count
    );
    assert_eq!(detailed.v5.v4.algorithm_requested, VCKSS_ALGORITHM_JLA);
    assert_eq!(detailed.v5.v4.algorithm_selected, VCKSS_ALGORITHM_JLA);
    assert_eq!(detailed.v5.v4.deletion_mode, deletion_mode);
    assert_eq!(detailed.v5.v4.nuisance_mode, nuisance_mode);
    assert_eq!(
        detailed.v5.v4.v3.v2.solver_requested,
        VCKSS_ROUTE_DIAGONAL_PCG
    );
    assert_eq!(
        detailed.v5.v4.v3.v2.solver_selected,
        VCKSS_ROUTE_DIAGONAL_PCG
    );
    assert_eq!(detailed.v5.v4.v3.v2.solver_fallback, 0);
    assert_eq!(detailed.v5.v4.v3.v2.leverage_batch_width, 2);
    assert_eq!(detailed.v5.v4.v3.v2.target_batch_width, 3);
    assert_eq!(detailed.v5.v4.v3.rng_contract, VCKSS_RNG_COUNTER_V1);
    assert_eq!(
        detailed.v5.v4.v3.v2.full_fit_complete_residual,
        detailed.full_joint_fit_complete_residual
    );
    assert_eq!(
        detailed.v5.v4.v3.v2.max_reciprocal_residual,
        detailed.generic_maker_relres
    );
    assert_eq!(detailed.v5.v4.inverse_relres, 0.0);
    assert!(detailed.v5.actual_accounting_residual <= 1e-10);
    assert_eq!(
        detailed.capability_schema,
        VCKSS_REQUEST_CAPABILITY_SCHEMA_V2
    );
    assert_eq!(
        detailed.capability_profile,
        VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1
    );
    assert_eq!(detailed.batch_mode, VCKSS_BATCH_MODE_EXPLICIT);
    assert_eq!(detailed.stayers_mode, VCKSS_STAYERS_MOVERS);
    assert_eq!(
        detailed.target_weight_mode,
        VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT
    );
    assert_eq!(
        detailed.deletion_unit_source,
        if deletion_mode == VCKSS_DELETION_MATCH {
            VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
        } else {
            VCKSS_DELETION_SOURCE_OBSERVATION_ROW
        }
    );
    assert_eq!(detailed.probeorder_supplied, 0);
    assert_eq!(detailed.wallseconds_supplied, 0);
    assert_eq!(detailed.frequency_use, VCKSS_REQUEST_FREQUENCY_LITERAL);
    assert_eq!(detailed.physical_limit, 50_000_000);
    assert_ne!(detailed.request_signature, 0);

    let distinct_working =
        usize::from(nuisance_mode == VCKSS_NUISANCE_FIXED_OFFSET && controls_count != 0);
    let rows = controls_count + 1 + distinct_working + 3 * probes;
    assert_eq!(detailed.v5.v4.v3.rhs_receipt_rows as usize, rows);
    assert_eq!(detailed.v5.v4.v3.caller_result_copy_bytes, 0);
    assert_eq!(
        detailed.rhs_v2_caller_copy_bytes,
        rows as u64 * (size_of::<VckssEngineRhsReceiptV2>() as u64 + 15 * 8)
    );
    assert_eq!(
        detailed.v5.v4.v3.v2.result_forecast_bytes,
        detailed.generic_result_forecast_bytes
    );
    assert_eq!(
        detailed.v5.v4.v3.v2.solve_peak_forecast_bytes,
        detailed.generic_peak_forecast_bytes
    );
    assert_eq!(
        detailed.v5.v4.v3.v2.command_peak_forecast_bytes,
        detailed
            .v5
            .v4
            .v3
            .v2
            .preparation_peak_forecast_bytes
            .max(detailed.generic_peak_forecast_bytes)
    );
    let mut rhs = vec![VckssEngineRhsReceiptV2::default(); rows];
    assert_eq!(
        vckss_rust_engine_rhs_receipts_v2(generation, rhs.as_mut_ptr(), rows as u64),
        ErrorCode::Ok as i32
    );
    for (control, receipt) in rhs.iter().take(controls_count).enumerate() {
        assert_eq!(receipt.v1.phase, 5);
        assert_eq!(receipt.v1.side, 0);
        assert_eq!(receipt.v1.probe, control as i64);
        assert_eq!(receipt.residual_space, VCKSS_RESIDUAL_SPACE_WORKER_FIRM);
    }
    let full = &rhs[controls_count];
    assert_eq!(full.v1.phase, 1);
    assert_eq!(full.v1.side, 0);
    assert_eq!(full.v1.probe, -1);
    assert_eq!(
        full.residual_space,
        VCKSS_RESIDUAL_SPACE_WORKER_FIRM_CONTROL
    );
    assert_eq!(
        full.v1.complete_residual,
        detailed.full_joint_fit_complete_residual
    );
    if distinct_working == 1 {
        let working = &rhs[controls_count + 1];
        assert_eq!(working.v1.phase, 4);
        assert_eq!(working.v1.side, 0);
        assert_eq!(working.v1.probe, -1);
        assert_eq!(working.residual_space, VCKSS_RESIDUAL_SPACE_WORKER_FIRM);
        assert_eq!(
            working.v1.complete_residual,
            detailed.generic_working_fit_complete_residual
        );
    }
    let leverage_start = controls_count + 1 + distinct_working;
    for (probe, receipt) in rhs[leverage_start..leverage_start + probes]
        .iter()
        .enumerate()
    {
        assert_eq!(receipt.v1.phase, 2);
        assert_eq!(receipt.v1.probe, probe as i64);
    }
    for (logical, receipt) in rhs[leverage_start + probes..].iter().enumerate() {
        assert_eq!(receipt.v1.phase, 3);
        assert_eq!(receipt.v1.probe, (logical / 2) as i64);
        assert_eq!(receipt.v1.side, if logical % 2 == 0 { 1 } else { 2 });
    }
    for receipt in &rhs {
        assert!(matches!(
            receipt.status,
            VCKSS_RHS_STATUS_ZERO | VCKSS_RHS_STATUS_CONVERGED
        ));
        assert_eq!(
            receipt.v1.zero_rhs,
            u32::from(receipt.status == VCKSS_RHS_STATUS_ZERO)
        );
        assert_eq!(receipt.v1.route, VCKSS_ROUTE_DIAGONAL_PCG);
        assert!(receipt.v1.reduced_residual.is_finite());
        assert!(receipt.v1.complete_residual.is_finite());
        assert!(receipt.v1.complete_residual <= receipt.full_residual_tolerance);
        assert!(receipt.solver_dimension > 0);
    }
    let exported_max_complete = rhs
        .iter()
        .map(|value| value.v1.complete_residual)
        .fold(0.0_f64, f64::max);
    assert_eq!(
        detailed.v5.v4.v3.v2.max_complete_residual,
        exported_max_complete
    );
    let exported_max_reduced = rhs
        .iter()
        .map(|value| value.v1.reduced_residual)
        .fold(0.0_f64, f64::max);
    assert_eq!(
        detailed.v5.v4.v3.v2.max_reduced_residual,
        exported_max_reduced
    );
    (result, detailed, rhs)
}

#[test]
fn generic_v3_lifecycle_covers_all_deletion_nuisance_and_control_combinations() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let columns = OwnedColumns::generic_dense();
    let control = one_generic_control(&columns);
    let probes = 7_usize;
    for deletion_mode in [VCKSS_DELETION_MATCH, VCKSS_DELETION_OBSERVATION] {
        for nuisance_mode in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            for controls in [Vec::new(), vec![control.clone()]] {
                reset();
                let generation = prepare_with_controls(&columns, &controls, deletion_mode);
                solve_generic_request(
                    generation,
                    deletion_mode,
                    nuisance_mode,
                    controls.len() as u32,
                    probes as u32,
                );
                let (_, detailed, rhs) = assert_generic_rhs_surface(
                    generation,
                    deletion_mode,
                    nuisance_mode,
                    controls.len(),
                    probes,
                );
                if nuisance_mode == VCKSS_NUISANCE_FIXED_OFFSET && !controls.is_empty() {
                    assert_eq!(rhs[controls.len() + 1].v1.phase, 4);
                    assert_eq!(
                        detailed.v5.v4.v3.v2.max_complete_residual,
                        detailed
                            .full_joint_fit_complete_residual
                            .max(detailed.generic_working_fit_complete_residual)
                            .max(
                                rhs.iter()
                                    .map(|value| value.v1.complete_residual)
                                    .fold(0.0_f64, f64::max)
                            )
                    );
                }
                assert_eq!(
                    vckss_rust_engine_release_v1(generation),
                    ErrorCode::Ok as i32
                );
                assert_eq!(
                    vckss_rust_engine_release_v1(generation),
                    ErrorCode::Ok as i32
                );
            }
        }
    }
}

#[test]
fn generic_result_export_memory_has_an_exact_one_byte_admission_boundary() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let control = one_generic_control(&columns);
    let controls = vec![control];
    let probes = 257_u32;

    let generation = prepare_with_controls(&columns, &controls, VCKSS_DELETION_MATCH);
    solve_generic_request(
        generation,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_FIXED_OFFSET,
        1,
        probes,
    );
    let (_, baseline, _) = assert_generic_rhs_surface(
        generation,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_FIXED_OFFSET,
        1,
        probes as usize,
    );
    let rows = 1_u64 + 1 + 1 + 3 * u64::from(probes);
    assert_eq!(baseline.v5.v4.v3.caller_result_copy_bytes, 0);
    assert_eq!(baseline.rhs_v2_caller_copy_bytes, rows * 216);
    assert_eq!(
        baseline.generic_result_forecast_bytes, baseline.generic_peak_forecast_bytes,
        "fixture makes retained/export result memory the generic solve peak"
    );
    assert!(
        baseline.generic_peak_forecast_bytes > baseline.v5.v4.v3.v2.preparation_peak_forecast_bytes
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let exact_limit = baseline.generic_peak_forecast_bytes;
    let generation =
        prepare_with_controls_memory(&columns, &controls, VCKSS_DELETION_MATCH, exact_limit);
    solve_generic_request(
        generation,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_FIXED_OFFSET,
        1,
        probes,
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let generation =
        prepare_with_controls_memory(&columns, &controls, VCKSS_DELETION_MATCH, exact_limit - 1);
    let (_, request) =
        generic_solve_request(VCKSS_DELETION_MATCH, VCKSS_NUISANCE_FIXED_OFFSET, 1, probes);
    assert_eq!(
        vckss_rust_engine_solve_v3(generation, &request),
        ErrorCode::ResourceLimit as i32
    );
    // SAFETY: the thread-local error remains valid until this thread's next
    // engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().contains("generic_jla_memory"));
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn generic_v3_zero_rhs_status_is_lossless_and_does_not_consume_pcg_work() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let mut columns = OwnedColumns::generic_dense();
    columns.outcome.fill(0.0);
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
    solve_generic_request(generation, VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, 0, 5);
    let (_, _, rhs) =
        assert_generic_rhs_surface(generation, VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, 0, 5);
    let full = &rhs[0];
    assert_eq!(full.v1.phase, 1);
    assert_eq!(full.status, VCKSS_RHS_STATUS_ZERO);
    assert_eq!(full.v1.zero_rhs, 1);
    assert_eq!(full.v1.iterations, 0);
    assert_eq!(full.residual_replacements, 0);
    assert_eq!(full.operator_applications, 0);
    assert_eq!(full.preconditioner_applications, 0);
    assert_eq!(full.v1.reduced_residual.to_bits(), 0.0_f64.to_bits());
    assert_eq!(full.v1.complete_residual.to_bits(), 0.0_f64.to_bits());
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn generic_v3_failure_and_interrupt_replay_retain_preparation_until_idempotent_release() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
    let (_, mut bad_signature) =
        generic_solve_request(VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, 0, 5);
    bad_signature.request_signature ^= 1;
    assert_eq!(
        vckss_rust_engine_solve_v3(generation, &bad_signature),
        ErrorCode::UnsupportedFeature as i32
    );
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::UnsupportedFeature as i32
    );
    let mut preparation = VckssEnginePreparationReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v4(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.controls_count, 0);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
    let (_, mut options) = generic_solve_request(VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, 0, 5);
    let mut interrupt = VckssEngineSolveRequestInterruptV3::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v3(
            &mut interrupt,
            bytes::<VckssEngineSolveRequestInterruptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    options.v2.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV3>();
    interrupt.options = options;
    let mut poll = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    interrupt.interrupt_poll = Some(injected_poll);
    interrupt.interrupt_context = (&mut poll as *mut PollState).cast();
    interrupt.checkpoint_interval = 1;
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v3(generation, &interrupt),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(poll.calls, 1);
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v4(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn generic_v3_physical_resource_and_control_rank_failures_keep_typed_codes() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
    let (capability, mut request) =
        generic_solve_request(VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, 0, 5);
    let constrained_capability = request_capability_v2(VckssBackendRequestCapabilityRequestV2 {
        v1: VckssBackendRequestCapabilityRequestV1 {
            struct_size: bytes::<VckssBackendRequestCapabilityRequestV2>(),
            request_schema: capability.v1.request_schema,
            algorithm: capability.v1.algorithm,
            deletion_mode: capability.v1.deletion_mode,
            nuisance_mode: capability.v1.nuisance_mode,
            solver_route: capability.v1.solver_route,
            rng_contract: capability.v1.rng_contract,
            controls_count: capability.v1.controls_count,
            frequency_use: capability.v1.frequency_use,
            ..VckssBackendRequestCapabilityRequestV1::default()
        },
        engine: capability.engine,
        batch_mode: capability.batch_mode,
        stayers_mode: capability.stayers_mode,
        target_weight_mode: capability.target_weight_mode,
        deletion_unit_source: capability.deletion_unit_source,
        probeorder_supplied: capability.probeorder_supplied,
        wallseconds_supplied: capability.wallseconds_supplied,
        physical_limit: 1,
        ..VckssBackendRequestCapabilityRequestV2::default()
    });
    assert_eq!(constrained_capability.v1.supported, 1);
    request.physical_limit = 1;
    request.request_signature = constrained_capability.v1.request_signature;
    assert_eq!(
        vckss_rust_engine_solve_v3(generation, &request),
        ErrorCode::ResourceLimit as i32
    );
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::ResourceLimit as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let zero_control = vec![0.0; columns.worker.len()];
    let generation = prepare_with_controls(&columns, &[zero_control], VCKSS_DELETION_OBSERVATION);
    let (_, rank_request) =
        generic_solve_request(VCKSS_DELETION_OBSERVATION, VCKSS_NUISANCE_JOINT, 1, 5);
    let rank_status = vckss_rust_engine_solve_v3(generation, &rank_request);
    assert!(matches!(
        rank_status,
        value if value == ErrorCode::SingularInformation as i32
            || value == ErrorCode::AmbiguousControlBasis as i32
            || value == ErrorCode::UnverifiedDeletionRank as i32
    ));
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        rank_status
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

fn prepare_interrupt_request(
    columns: &OwnedColumns,
    poll: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    context: *mut c_void,
    interval: u32,
) -> VckssEnginePrepareRequestInterruptV1 {
    let mut request = VckssEnginePrepareRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_prepare_request_interrupt_v1(
            &mut request,
            bytes::<VckssEnginePrepareRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    let options = columns.request_v2();
    request.options.abi_version = options.abi_version;
    request.options.rows = options.rows;
    request.options.cleanup_abandoned = options.cleanup_abandoned;
    request.options.memory_limit_bytes = options.memory_limit_bytes;
    request.options.caller_copy_bytes = options.caller_copy_bytes;
    request.interrupt_poll = poll;
    request.interrupt_context = context;
    request.checkpoint_interval = interval;
    request
}

#[test]
fn interrupt_prepare_default_and_contract_are_frozen() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let request = prepare_interrupt_request(&columns, None, ptr::null_mut(), 0);
    assert_eq!(request.options.struct_size, 64);
    assert!(request.interrupt_poll.is_none());
    assert!(request.interrupt_context.is_null());
    assert_eq!(request.checkpoint_interval, 0);
    assert_eq!(request.reserved, 0);
    assert_eq!(
        vckss_rust_engine_admit_prepare_v2(&request.options),
        ErrorCode::Ok as i32
    );

    let invalid = prepare_interrupt_request(&columns, None, ptr::null_mut(), 1);
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &invalid,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(generation, 0);

    let malformed_header = [ABI_VERSION, 8_u32];
    generation = 99;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            malformed_header
                .as_ptr()
                .cast::<VckssEnginePrepareRequestInterruptV1>(),
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(generation, 0);

    generation = 99;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            malformed_header
                .as_ptr()
                .cast::<VckssEnginePrepareRequestInterruptV1>(),
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>() - 1,
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(generation, 99);
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            malformed_header
                .as_ptr()
                .cast::<VckssEnginePrepareRequestInterruptV1>(),
            &columns.descriptor(),
            ptr::null_mut(),
            bytes::<u64>(),
        ),
        ErrorCode::InvalidInput as i32
    );

    let mut state = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: 77,
    };
    let unknown = prepare_interrupt_request(
        &columns,
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    generation = 99;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &unknown,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::InternalInvariantFailed as i32
    );
    assert_eq!(generation, 0);
}

#[test]
fn prepare_user_breaks_leave_zero_generation_and_empty_registry() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();

    let mut counting = PollState {
        calls: 0,
        stop_at: u32::MAX,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let request = prepare_interrupt_request(
        &columns,
        Some(injected_poll),
        (&mut counting as *mut PollState).cast(),
        1,
    );
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    let total = counting.calls;
    assert!(total > 16, "fixture must cross multiple preparation phases");
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let stops = [1, 2, total / 4, total / 2, total - 1];
    for stop_at in stops {
        let mut state = PollState {
            calls: 0,
            stop_at,
            terminal_status: VCKSS_INTERRUPT_USER_BREAK,
        };
        let request = prepare_interrupt_request(
            &columns,
            Some(injected_poll),
            (&mut state as *mut PollState).cast(),
            1,
        );
        generation = 99;
        assert_eq!(
            vckss_rust_engine_prepare_interrupt_v1(
                &request,
                &columns.descriptor(),
                &mut generation,
                bytes::<u64>(),
            ),
            ErrorCode::UserBreak as i32,
            "stop_at={stop_at}"
        );
        assert_eq!(generation, 0);
        assert_eq!(state.calls, stop_at);
        let mut snapshot = VckssEngineSnapshotV1::default();
        assert_eq!(
            vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
            ErrorCode::Ok as i32
        );
        assert_eq!(snapshot.state, 0);
        assert_eq!(snapshot.generation, 0);
    }
}

#[test]
fn interrupt_prepare_preserves_cleanup_abandoned_contract() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let abandoned = prepare(&columns);

    let mut request = prepare_interrupt_request(&columns, None, ptr::null_mut(), 0);
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::ContextPoisoned as i32
    );
    assert_eq!(generation, 0);

    request.options.cleanup_abandoned = 1;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_ne!(generation, abandoned);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn cleanup_releases_the_old_generation_before_replacement_ingest_can_fail() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let abandoned = prepare(&columns);
    let mut state = PollState {
        calls: 0,
        stop_at: 2,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let mut request = prepare_interrupt_request(
        &columns,
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    request.options.cleanup_abandoned = 1;
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(generation, 0);
    assert_eq!(state.calls, 2);
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 0);
    assert_eq!(snapshot.generation, 0);
    assert_eq!(snapshot.last_released_generation, abandoned);
}

#[test]
fn no_callback_prepare_schema_matches_v2_receipt_and_mask() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut expected_receipt = VckssEnginePreparationReceiptV2::default();
    let mut expected_mask = vec![0_u8; columns.worker.len()];
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut expected_receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(
            generation,
            expected_mask.as_mut_ptr(),
            expected_mask.len() as u64,
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let request = prepare_interrupt_request(&columns, None, ptr::null_mut(), 0);
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut actual_receipt = VckssEnginePreparationReceiptV2::default();
    let mut actual_mask = vec![0_u8; columns.worker.len()];
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut actual_receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(
            generation,
            actual_mask.as_mut_ptr(),
            actual_mask.len() as u64,
        ),
        ErrorCode::Ok as i32
    );
    expected_receipt.generation = 0;
    actual_receipt.generation = 0;
    assert_eq!(expected_receipt, actual_receipt);
    assert_eq!(expected_mask, actual_mask);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn one_engine_generation_exports_science_mask_and_fixed_receipts() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);

    let mut preparation = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.generation, generation);
    assert_eq!(preparation.input_rows, 48);
    assert_eq!(preparation.retained_rows, 48);
    assert_eq!(preparation.memory_limit_bytes, 64_u64 << 20);
    assert_eq!(preparation.caller_copy_bytes, 48 * 6 * 8);
    assert!(preparation.preparation_peak_forecast_bytes >= preparation.caller_copy_bytes);
    assert!(preparation.prepared_resident_bytes > 0);
    assert_eq!(preparation.graph_input_rows, 48);
    assert_eq!(preparation.graph_retained_rows, 48);
    assert_eq!(preparation.graph_input_physical_mass, 72);
    assert_eq!(preparation.graph_retained_physical_mass, 72);
    assert_eq!(preparation.graph_initial_components, 1);
    assert_eq!(preparation.graph_maximum_components, 1);
    assert_eq!(preparation.graph_initial_component_rows, 48);
    assert_eq!(preparation.graph_mover_input_rows, 48);
    assert_eq!(preparation.graph_initial_deletion_edges, 48);
    assert_eq!(preparation.graph_retained_deletion_edges, 48);

    let mut preparation_v3 = VckssEnginePreparationReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v3(
            generation,
            &mut preparation_v3,
            bytes::<VckssEnginePreparationReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation_v3.v2, preparation);
    assert_eq!(
        preparation_v3.target_weight_sum,
        columns.target_weight.iter().sum()
    );
    let mut preparation_v4 = VckssEnginePreparationReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v4(
            generation,
            &mut preparation_v4,
            bytes::<VckssEnginePreparationReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation_v4.v3, preparation_v3);
    assert_eq!(preparation_v4.controls_count, 0);
    assert_eq!(preparation_v4.deletion_mode, VCKSS_DELETION_MATCH);

    let mut mask = vec![0_u8; 48];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));

    let mut solve = VckssEngineSolveRequestV1::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v1(
            &mut solve,
            bytes::<VckssEngineSolveRequestV1>(),
        ),
        ErrorCode::Ok as i32
    );
    solve.seed = 91_827;
    solve.probes = 6;
    solve.leverage_batch_width = 3;
    solve.target_batch_width = 2;
    solve.solver_route = VCKSS_ROUTE_EXACT;
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &solve),
        ErrorCode::Ok as i32
    );

    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>(),),
        ErrorCode::Ok as i32
    );
    for ((plugin, correction), corrected) in [
        result.plugin.worker,
        result.plugin.firm,
        result.plugin.covariance,
        result.plugin.total,
    ]
    .into_iter()
    .zip([
        result.correction.worker,
        result.correction.firm,
        result.correction.covariance,
        result.correction.total,
    ])
    .zip([
        result.corrected.worker,
        result.corrected.firm,
        result.corrected.covariance,
        result.corrected.total,
    ]) {
        assert!((plugin - correction - corrected).abs() <= 1.0e-10);
    }
    assert!(result.numerical_mcse.worker.is_finite());
    assert!(result.numerical_mcse.worker >= 0.0);

    let mut receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.seed, solve.seed);
    assert_eq!(receipt.probes_requested, solve.probes);
    assert_eq!(receipt.solver_requested, VCKSS_ROUTE_EXACT);
    assert_eq!(receipt.solver_selected, VCKSS_ROUTE_EXACT);
    assert_eq!(receipt.leverage_batch_width, 3);
    assert_eq!(receipt.target_batch_width, 2);
    assert_eq!(receipt.leverage_rhs_count, 6);
    assert_eq!(receipt.target_rhs_count, 12);
    assert!(receipt.full_fit_weighted_rss.is_finite());
    assert!(receipt.full_fit_weighted_rss > 0.0);
    assert_eq!(receipt.memory_limit_bytes, preparation.memory_limit_bytes);
    assert_eq!(receipt.caller_copy_bytes, preparation.caller_copy_bytes);
    assert_eq!(
        receipt.prepared_resident_bytes,
        preparation.prepared_resident_bytes
    );
    assert_eq!(
        receipt.command_peak_forecast_bytes,
        receipt
            .preparation_peak_forecast_bytes
            .max(receipt.solve_peak_forecast_bytes)
    );
    assert!(receipt.command_peak_forecast_bytes <= receipt.memory_limit_bytes);

    let mut receipt_v3 = VckssEngineDetailedReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v3(
            generation,
            &mut receipt_v3,
            bytes::<VckssEngineDetailedReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt_v3.v2, receipt);
    assert_eq!(receipt_v3.rng_contract, 1);
    assert_eq!(receipt_v3.rhs_receipt_rows, 19);
    assert_eq!(receipt_v3.caller_result_copy_bytes, 19 * 14 * 8);
    assert!(receipt_v3.v2.result_forecast_bytes >= receipt_v3.caller_result_copy_bytes);
    let mut receipt_v4 = VckssEngineDetailedReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v4(
            generation,
            &mut receipt_v4,
            bytes::<VckssEngineDetailedReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut receipt_v5 = VckssEngineDetailedReceiptV5::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v5(
            generation,
            &mut receipt_v5,
            bytes::<VckssEngineDetailedReceiptV5>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt_v5.v4, receipt_v4);
    assert_eq!(
        receipt_v5.applicability_flags,
        VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING
    );
    assert_eq!(receipt_v5.working_fit_complete_residual, 0.0);
    assert_eq!(receipt_v5.inverse_sqrt_relres, 0.0);
    let expected_accounting = [result.plugin, result.correction, result.corrected]
        .into_iter()
        .map(|value| (value.total - value.worker - value.firm - 2.0 * value.covariance).abs())
        .fold(0.0_f64, f64::max);
    assert_eq!(receipt_v5.actual_accounting_residual, expected_accounting);

    let mut rhs = vec![VckssEngineRhsReceiptV1::default(); 19];
    assert_eq!(
        vckss_rust_engine_rhs_receipts_v1(generation, rhs.as_mut_ptr(), rhs.len() as u64),
        ErrorCode::Ok as i32
    );
    assert_eq!((rhs[0].phase, rhs[0].side, rhs[0].probe), (1, 0, -1));
    for (probe, row) in rhs[1..7].iter().enumerate() {
        assert_eq!((row.phase, row.side, row.probe), (2, 0, probe as i64));
    }
    for probe in 0..6 {
        let worker = rhs[7 + 2 * probe];
        let firm = rhs[8 + 2 * probe];
        assert_eq!(
            (worker.phase, worker.side, worker.probe),
            (3, 1, probe as i64)
        );
        assert_eq!((firm.phase, firm.side, firm.probe), (3, 2, probe as i64));
    }
    assert!(rhs.iter().all(|row| row.route == VCKSS_ROUTE_EXACT));
    assert!(rhs.iter().all(|row| row.complete_residual.is_finite()));

    // Preparation metadata remains in the same generation after solve.
    preparation = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    mask.fill(0);
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));

    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn v2_memory_admission_fails_before_column_descriptor_access() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let mut request = columns.request_v2();
    request.memory_limit_bytes = request.caller_copy_bytes;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v2(&request),
        ErrorCode::ResourceLimit as i32
    );

    let mut generation = 77_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(&request, ptr::null(), &mut generation, bytes::<u64>(),),
        ErrorCode::ResourceLimit as i32
    );
    assert_eq!(generation, 0);
    // SAFETY: engine error strings are thread-local and valid until this
    // thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().contains("simultaneous C/Rust"));
}

#[test]
fn invalid_frequency_weight_has_the_weight_error_code() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let mut columns = OwnedColumns::dense();
    columns.frequency[0] = -1.0;
    let request = columns.request_v2();
    let descriptor = columns.descriptor();
    let mut generation = 77_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(&request, &descriptor, &mut generation, bytes::<u64>(),),
        ErrorCode::InvalidWeight as i32
    );
    assert_eq!(generation, 0);
    // SAFETY: engine error strings are thread-local and valid until this
    // thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().starts_with("INVALID_WEIGHT "));
    assert!(error.to_string_lossy().contains("frequency weight"));
}

#[test]
fn headers_and_every_output_capacity_fail_before_full_value_access_or_write() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let capability_request = VckssBackendRequestCapabilityRequestV1::default();
    let mut capability_receipt = VckssBackendRequestCapabilityReceiptV1 {
        request_signature: 123,
        ..VckssBackendRequestCapabilityReceiptV1::default()
    };
    assert_eq!(
        vckss_rust_backend_request_capability_v1(
            &capability_request,
            &mut capability_receipt,
            bytes::<VckssBackendRequestCapabilityReceiptV1>() - 1,
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(capability_receipt.request_signature, 123);
    let columns = OwnedColumns::dense();
    let descriptor = columns.descriptor();
    let short_request = [ABI_VERSION, 4_u32];
    let mut generation = 77_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v1(
            short_request.as_ptr().cast::<VckssEnginePrepareRequestV1>(),
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(generation, 77);

    let request = columns.request();
    assert_eq!(
        vckss_rust_engine_prepare_v1(&request, &descriptor, &mut generation, 7),
        ErrorCode::AbiMismatch as i32
    );
    generation = prepare(&columns);

    let mut receipt = VckssEnginePreparationReceiptV1 {
        generation: 123,
        ..VckssEnginePreparationReceiptV1::default()
    };
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v1(
            generation,
            &mut receipt,
            bytes::<VckssEnginePreparationReceiptV1>() - 1,
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(receipt.generation, 123);
    let mut receipt_v4 = VckssEnginePreparationReceiptV4 {
        controls_count: 123,
        ..VckssEnginePreparationReceiptV4::default()
    };
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v4(
            generation,
            &mut receipt_v4,
            bytes::<VckssEnginePreparationReceiptV4>() - 1,
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(receipt_v4.controls_count, 123);
    let mut short_mask = vec![9_u8; columns.worker.len() - 1];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(
            generation,
            short_mask.as_mut_ptr(),
            short_mask.len() as u64,
        ),
        ErrorCode::InvalidInput as i32
    );
    assert!(short_mask.iter().all(|&value| value == 9));
    assert_eq!(
        vckss_rust_engine_result_v1(generation, ptr::null_mut(), 0),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn failed_solve_preserves_authoritative_preparation_and_typed_error() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let solve = VckssEngineSolveRequestV1 {
        probes: 1,
        ..VckssEngineSolveRequestV1::default()
    };
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &solve),
        ErrorCode::InvalidInput as i32
    );
    // SAFETY: engine error strings are thread-local and valid until this
    // thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().contains("at least two"));

    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>(),),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 4);
    let mut preparation = VckssEnginePreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v1(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.input_rows, 48);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

fn interrupt_request(
    poll: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    context: *mut c_void,
    interval: u32,
) -> VckssEngineSolveRequestInterruptV1 {
    let mut request = VckssEngineSolveRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v1(
            &mut request,
            bytes::<VckssEngineSolveRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    request.options.seed = 91_827;
    request.options.probes = 20;
    request.options.leverage_batch_width = 4;
    request.options.target_batch_width = 4;
    request.options.solver_route = VCKSS_ROUTE_EXACT;
    request.interrupt_poll = poll;
    request.interrupt_context = context;
    request.checkpoint_interval = interval;
    request
}

#[test]
fn immediate_user_break_preserves_context_until_release_and_rerun() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut state = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let request = interrupt_request(
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &request),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(state.calls, 1);

    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 4);
    assert_eq!(snapshot.generation, generation);

    let mut preparation = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut mask = vec![0_u8; columns.worker.len()];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64,),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let rerun_generation = prepare(&columns);
    let mut continue_state = PollState {
        calls: 0,
        stop_at: u32::MAX,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let rerun = interrupt_request(
        Some(injected_poll),
        (&mut continue_state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(rerun_generation, &rerun),
        ErrorCode::Ok as i32
    );
    assert!(continue_state.calls > 1);
    assert_eq!(
        vckss_rust_engine_release_v1(rerun_generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn deep_user_break_stops_at_the_requested_checkpoint() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut state = PollState {
        calls: 0,
        stop_at: 40,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let request = interrupt_request(
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &request),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(state.calls, 40);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn invalid_and_unknown_callback_contracts_fail_closed() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);

    let invalid = interrupt_request(None, ptr::null_mut(), 1);
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &invalid),
        ErrorCode::InvalidInput as i32
    );
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 1);

    let mut state = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: 77,
    };
    let unknown = interrupt_request(
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &unknown),
        ErrorCode::InternalInvariantFailed as i32
    );
    assert_eq!(state.calls, 1);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn no_callback_interrupt_schema_is_bitwise_identical_to_v1() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let v1 = VckssEngineSolveRequestV1 {
        seed: 91_827,
        probes: 20,
        leverage_batch_width: 4,
        target_batch_width: 4,
        solver_route: VCKSS_ROUTE_EXACT,
        ..VckssEngineSolveRequestV1::default()
    };
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &v1),
        ErrorCode::Ok as i32
    );
    let mut v1_result = VckssEngineResultV1::default();
    let mut v1_receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut v1_result, bytes::<VckssEngineResultV1>(),),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut v1_receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let generation = prepare(&columns);
    let interrupt = interrupt_request(None, ptr::null_mut(), 0);
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &interrupt),
        ErrorCode::Ok as i32
    );
    let mut interrupt_result = VckssEngineResultV1::default();
    let mut interrupt_receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_result_v1(
            generation,
            &mut interrupt_result,
            bytes::<VckssEngineResultV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut interrupt_receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    v1_result.generation = 0;
    interrupt_result.generation = 0;
    v1_receipt.generation = 0;
    interrupt_receipt.generation = 0;
    assert_eq!(v1_result, interrupt_result);
    assert_eq!(v1_receipt, interrupt_receipt);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn additive_v2_no_control_match_jla_is_bitwise_identical_to_v1() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let request_v1 = VckssEngineSolveRequestV1 {
        seed: 71_991,
        probes: 20,
        leverage_batch_width: 4,
        target_batch_width: 4,
        solver_route: VCKSS_ROUTE_EXACT,
        ..VckssEngineSolveRequestV1::default()
    };

    let generation = prepare(&columns);
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &request_v1),
        ErrorCode::Ok as i32
    );
    let mut expected_result = VckssEngineResultV1::default();
    let mut expected_receipt = VckssEngineDetailedReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_result_v1(
            generation,
            &mut expected_result,
            bytes::<VckssEngineResultV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v3(
            generation,
            &mut expected_receipt,
            bytes::<VckssEngineDetailedReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut expected_rhs =
        vec![VckssEngineRhsReceiptV1::default(); expected_receipt.rhs_receipt_rows as usize];
    assert_eq!(
        vckss_rust_engine_rhs_receipts_v1(
            generation,
            expected_rhs.as_mut_ptr(),
            expected_rhs.len() as u64,
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let generation = prepare(&columns);
    let mut request_v2 = VckssEngineSolveRequestV2::default();
    let outer_size = request_v2.v1.struct_size;
    request_v2.v1 = request_v1;
    request_v2.v1.struct_size = outer_size;
    request_v2.algorithm = VCKSS_ALGORITHM_JLA;
    assert_eq!(
        vckss_rust_engine_solve_v2(generation, &request_v2),
        ErrorCode::Ok as i32
    );
    let mut actual_result = VckssEngineResultV1::default();
    let mut actual_receipt = VckssEngineDetailedReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_result_v1(
            generation,
            &mut actual_result,
            bytes::<VckssEngineResultV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v3(
            generation,
            &mut actual_receipt,
            bytes::<VckssEngineDetailedReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut actual_rhs =
        vec![VckssEngineRhsReceiptV1::default(); actual_receipt.rhs_receipt_rows as usize];
    assert_eq!(
        vckss_rust_engine_rhs_receipts_v1(
            generation,
            actual_rhs.as_mut_ptr(),
            actual_rhs.len() as u64,
        ),
        ErrorCode::Ok as i32
    );

    expected_result.generation = 0;
    actual_result.generation = 0;
    expected_receipt.v2.generation = 0;
    actual_receipt.v2.generation = 0;
    assert_eq!(expected_result, actual_result);
    assert_eq!(expected_receipt, actual_receipt);
    assert_eq!(
        [
            component_bits(expected_result.plugin),
            component_bits(expected_result.correction),
            component_bits(expected_result.corrected),
            component_bits(expected_result.numerical_mcse),
        ],
        [
            component_bits(actual_result.plugin),
            component_bits(actual_result.correction),
            component_bits(actual_result.corrected),
            component_bits(actual_result.numerical_mcse),
        ]
    );
    assert_eq!(
        detailed_receipt_float_bits(expected_receipt),
        detailed_receipt_float_bits(actual_receipt)
    );
    assert_eq!(expected_rhs, actual_rhs);
    assert!(expected_rhs.iter().zip(&actual_rhs).all(|(left, right)| {
        left.reduced_residual.to_bits() == right.reduced_residual.to_bits()
            && left.complete_residual.to_bits() == right.complete_residual.to_bits()
    }));
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn v2_exports_nontrivial_graph_selection_receipt() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::disconnected_components();
    let generation = prepare(&columns);
    let mut receipt = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.graph_input_rows, 10);
    assert_eq!(receipt.graph_retained_rows, 6);
    assert_eq!(receipt.graph_input_physical_mass, 406);
    assert_eq!(receipt.graph_retained_physical_mass, 6);
    assert_eq!(receipt.graph_initial_components, 2);
    assert_eq!(receipt.graph_maximum_components, 2);
    assert_eq!(receipt.graph_initial_component_rows, 6);
    assert_eq!(receipt.graph_mover_input_rows, 6);
    assert_eq!(receipt.graph_initial_deletion_edges, 6);
    assert_eq!(receipt.graph_retained_deletion_edges, 6);
    let mut mask = vec![0_u8; 10];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), 10),
        ErrorCode::Ok as i32
    );
    assert_eq!(mask, vec![0, 0, 0, 0, 1, 1, 1, 1, 1, 1]);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn tight_memory_limit_covers_single_mask_and_allocation_free_finalization() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut solve = interrupt_request(None, ptr::null_mut(), 0);
    solve.options.probes = 256;
    solve.options.leverage_batch_width = 8;
    solve.options.target_batch_width = 8;
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &solve),
        ErrorCode::Ok as i32
    );
    let mut receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    let exact_limit = receipt.command_peak_forecast_bytes;
    assert_eq!(exact_limit, receipt.solve_peak_forecast_bytes);
    assert!(
        receipt.solve_peak_forecast_bytes > receipt.preparation_peak_forecast_bytes,
        "fixture must exercise solve-time admission"
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let mut prepare_request = columns.request_v2();
    prepare_request.memory_limit_bytes = exact_limit;
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(
            &prepare_request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &solve),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    prepare_request.memory_limit_bytes = exact_limit - 1;
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(
            &prepare_request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &solve),
        ErrorCode::ResourceLimit as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

fn request_capability_v3(
    request: VckssBackendRequestCapabilityRequestV3,
) -> VckssBackendRequestCapabilityReceiptV3 {
    let mut receipt = VckssBackendRequestCapabilityReceiptV3::default();
    assert_eq!(
        vckss_rust_backend_request_capability_v3(
            &request,
            &mut receipt,
            bytes::<VckssBackendRequestCapabilityReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    receipt
}

fn planned_capability_request(
    algorithm: u32,
    engine: u32,
    route: u32,
    deletion: u32,
    nuisance: u32,
    controls: u32,
    leverage_mode: u32,
    target_mode: u32,
) -> VckssBackendRequestCapabilityRequestV3 {
    let batch_mode = if leverage_mode == VCKSS_BATCH_MODE_AUTO
        && target_mode == VCKSS_BATCH_MODE_AUTO
    {
        VCKSS_BATCH_MODE_AUTO
    } else if leverage_mode == VCKSS_BATCH_MODE_EXPLICIT && target_mode == VCKSS_BATCH_MODE_EXPLICIT
    {
        VCKSS_BATCH_MODE_EXPLICIT
    } else {
        VCKSS_BATCH_MODE_INDEPENDENT
    };
    let deletion_unit_source = if deletion == VCKSS_DELETION_MATCH {
        VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
    } else {
        VCKSS_DELETION_SOURCE_OBSERVATION_ROW
    };
    let rng_contract = if algorithm == VCKSS_ALGORITHM_EXACT {
        VCKSS_RNG_NONE
    } else {
        VCKSS_RNG_COUNTER_V1
    };
    VckssBackendRequestCapabilityRequestV3 {
        v2: VckssBackendRequestCapabilityRequestV2 {
            v1: VckssBackendRequestCapabilityRequestV1 {
                struct_size: bytes::<VckssBackendRequestCapabilityRequestV3>(),
                request_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V3,
                algorithm,
                deletion_mode: deletion,
                nuisance_mode: nuisance,
                solver_route: route,
                rng_contract,
                controls_count: controls,
                frequency_use: VCKSS_REQUEST_FREQUENCY_LITERAL,
                ..VckssBackendRequestCapabilityRequestV1::default()
            },
            engine,
            batch_mode,
            stayers_mode: VCKSS_STAYERS_MOVERS,
            target_weight_mode: VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT,
            deletion_unit_source,
            physical_limit: 50_000_000,
            ..VckssBackendRequestCapabilityRequestV2::default()
        },
        leverage_batch_mode: leverage_mode,
        target_batch_mode: target_mode,
        allow_automatic_cmg_setup_fallback: u32::from(route == VCKSS_ROUTE_AUTO),
        ..VckssBackendRequestCapabilityRequestV3::default()
    }
}

fn planned_solve_request(
    capability_request: VckssBackendRequestCapabilityRequestV3,
    leverage_width: u32,
    target_width: u32,
) -> VckssEngineSolveRequestV4 {
    let capability = request_capability_v3(capability_request);
    assert_eq!(capability.v2.v1.supported, 1);
    assert_eq!(
        capability.v2.v1.profile_code,
        VCKSS_REQUEST_PROFILE_PLANNED_V1
    );
    let mut request = VckssEngineSolveRequestV4::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v4(
            &mut request,
            bytes::<VckssEngineSolveRequestV4>(),
        ),
        ErrorCode::Ok as i32
    );
    request.v3.v2.v1.seed = 61_991;
    request.v3.v2.v1.probes = 5;
    request.v3.v2.v1.leverage_batch_width = leverage_width;
    request.v3.v2.v1.target_batch_width = target_width;
    request.v3.v2.v1.deletion_mode = capability.v2.v1.deletion_mode;
    request.v3.v2.v1.rng_contract = capability.v2.v1.rng_contract;
    request.v3.v2.v1.solver_route = capability.v2.v1.solver_route;
    request.v3.v2.v1.allow_automatic_cmg_setup_fallback =
        capability.allow_automatic_cmg_setup_fallback;
    request.v3.v2.v1.pcg_tolerance = 1.0e-12;
    request.v3.v2.v1.maximum_iterations = 10_000;
    request.v3.v2.v1.residual_replacement_interval = 7;
    request.v3.v2.algorithm = capability.v2.v1.algorithm;
    request.v3.v2.nuisance_mode = capability.v2.v1.nuisance_mode;
    request.v3.engine = capability.v2.engine;
    request.v3.batch_mode = capability.v2.batch_mode;
    request.v3.stayers_mode = capability.v2.stayers_mode;
    request.v3.target_weight_mode = capability.v2.target_weight_mode;
    request.v3.deletion_unit_source = capability.v2.deletion_unit_source;
    request.v3.probeorder_supplied = capability.v2.probeorder_supplied;
    request.v3.wallseconds_supplied = capability.v2.wallseconds_supplied;
    request.v3.capability_schema = capability.v2.v1.request_schema;
    request.v3.capability_profile = capability.v2.v1.profile_code;
    request.v3.frequency_use = capability.v2.v1.frequency_use;
    request.v3.physical_limit = capability.v2.physical_limit;
    request.v3.request_signature = capability.v2.v1.request_signature;
    request.leverage_batch_mode = capability.leverage_batch_mode;
    request.target_batch_mode = capability.target_batch_mode;
    request.wallseconds = capability.wallseconds;
    request
}

#[test]
fn planned_abi_layouts_and_capability_signature_are_frozen_and_exhaustive() {
    assert_eq!(size_of::<VckssBackendRequestCapabilityRequestV3>(), 120);
    assert_eq!(size_of::<VckssBackendRequestCapabilityReceiptV3>(), 160);
    assert_eq!(size_of::<VckssEngineSolveRequestV4>(), 288);
    assert_eq!(size_of::<VckssEngineSolveRequestInterruptV4>(), 312);
    assert_eq!(size_of::<VckssEngineSolveRequestV5>(), 304);
    assert_eq!(size_of::<VckssEngineSolveRequestInterruptV5>(), 328);
    assert_eq!(size_of::<VckssExecutionPlanReceiptV1>(), 1000);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV7>(), 1840);
    assert_eq!(size_of::<VckssEnginePerformanceReceiptV1>(), 96);
    assert_eq!(size_of::<VckssFullCmgReceiptV1>(), 400);
    assert_eq!(
        offset_of!(VckssEngineSolveRequestV4, leverage_batch_mode),
        264
    );
    assert_eq!(offset_of!(VckssEngineDetailedReceiptV7, execution), 840);
    assert_eq!(offset_of!(VckssEnginePerformanceReceiptV1, ingest_ns), 32);

    let base = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        1,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_EXPLICIT,
    );
    let baseline = request_capability_v3(base);
    assert_eq!(baseline.v2.v1.supported, 1);
    assert_eq!(baseline.v2.batch_mode, VCKSS_BATCH_MODE_INDEPENDENT);
    assert_eq!(baseline.leverage_batch_resolution_deferred, 1);
    assert_eq!(baseline.target_batch_resolution_deferred, 0);
    assert_eq!(baseline.wall_advisory_only, 1);
    for changed in [
        VckssBackendRequestCapabilityRequestV3 {
            leverage_batch_mode: VCKSS_BATCH_MODE_EXPLICIT,
            v2: VckssBackendRequestCapabilityRequestV2 {
                batch_mode: VCKSS_BATCH_MODE_EXPLICIT,
                ..base.v2
            },
            ..base
        },
        VckssBackendRequestCapabilityRequestV3 {
            target_batch_mode: VCKSS_BATCH_MODE_AUTO,
            v2: VckssBackendRequestCapabilityRequestV2 {
                batch_mode: VCKSS_BATCH_MODE_AUTO,
                ..base.v2
            },
            ..base
        },
        VckssBackendRequestCapabilityRequestV3 {
            v2: VckssBackendRequestCapabilityRequestV2 {
                wallseconds_supplied: 1,
                ..base.v2
            },
            wallseconds: 321.5,
            ..base
        },
    ] {
        let receipt = request_capability_v3(changed);
        assert_eq!(receipt.v2.v1.supported, 1);
        assert_ne!(
            receipt.v2.v1.request_signature,
            baseline.v2.v1.request_signature
        );
    }

    let auto_compressed = planned_capability_request(
        VCKSS_ALGORITHM_AUTO,
        VCKSS_ENGINE_COMPRESSED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    assert_eq!(request_capability_v3(auto_compressed).v2.v1.supported, 0);
}

#[test]
fn structured_component_inference_has_an_atomic_versioned_plugin_lifecycle() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    assert_eq!(
        size_of::<VckssComponentInferenceAugmentationRequestV1>(),
        104
    );
    assert_eq!(
        size_of::<VckssComponentInferenceAugmentationRequestInterruptV1>(),
        128
    );
    assert_eq!(
        size_of::<VckssComponentInferenceAugmentationReceiptV1>(),
        64
    );
    assert_eq!(size_of::<VckssComponentInferenceResultReceiptV2>(), 168);
    assert_eq!(size_of::<VckssComponentInferenceResultReceiptV3>(), 192);

    let columns = OwnedColumns::structured_component();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(
            &mut augmentation,
            bytes::<VckssComponentInferenceAugmentationRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(augmentation.options.struct_size, 128);
    augmentation.options.variance_source = VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON;
    augmentation.options.reference_distribution = VCKSS_COMPONENT_REFERENCE_Q0;
    augmentation.options.seed = 0x31c0_ffee_782a_19d4;
    augmentation.options.probes = 512;
    augmentation.options.batch_width = 13;
    augmentation.options.spectrum_probes = 32;
    augmentation.options.spectrum_iterations = 128;
    augmentation.options.spectrum_tolerance = 1.0e-2;
    augmentation.options.fold_seed = 0x83d4_2556_97ab_c10e;
    assert_eq!(
        vckss_rust_engine_augment_component_inference_interrupt_v1(generation, &augmentation,),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    let mut augmentation_receipt = VckssComponentInferenceAugmentationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_component_inference_augmentation_receipt_v1(
            generation,
            &mut augmentation_receipt,
            bytes::<VckssComponentInferenceAugmentationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        augmentation_receipt.schema_version,
        VCKSS_COMPONENT_INFERENCE_SCHEMA_V1
    );
    assert_eq!(augmentation_receipt.rows, columns.worker.len() as u64);
    assert_eq!(augmentation_receipt.component_persistent_bytes, 0);

    let capability_request = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_EXPLICIT,
        VCKSS_BATCH_MODE_EXPLICIT,
    );
    let mut solve = planned_solve_request(capability_request, 13, 17);
    solve.v3.v2.v1.probes = 256;
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &solve),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let mut primitive = [0.0; 9];
    let mut covariance = [0.0; 16];
    let mut mcse = [0.0; 9];
    let mut spectrum = [0.0; 60];
    let mut summaries = [0.0; 24];
    let mut folds = [0.0; 150];
    let mut cv = [0.0; 490];
    let mut receipt = VckssComponentInferenceResultReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_component_inference_result_v2(
            generation,
            primitive.as_mut_ptr(),
            primitive.len() as u64,
            covariance.as_mut_ptr(),
            covariance.len() as u64,
            mcse.as_mut_ptr(),
            mcse.len() as u64,
            spectrum.as_mut_ptr(),
            spectrum.len() as u64,
            ptr::null_mut(),
            0,
            summaries.as_mut_ptr(),
            summaries.len() as u64,
            folds.as_mut_ptr(),
            folds.len() as u64,
            cv.as_mut_ptr(),
            cv.len() as u64,
            &mut receipt,
            bytes::<VckssComponentInferenceResultReceiptV2>(),
        ),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    assert_eq!(
        receipt.schema_version,
        VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V2
    );
    assert_eq!(
        receipt.variance_source,
        VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON
    );
    assert_eq!(receipt.reference_distribution, VCKSS_COMPONENT_REFERENCE_Q0);
    assert_eq!(receipt.q1_present, 0);
    assert_eq!(receipt.fold_rows, 10);
    assert_eq!(receipt.cv_rows, 70);
    assert!(primitive
        .iter()
        .chain(&covariance)
        .all(|value| value.is_finite()));
    assert!(summaries.iter().all(|value| value.is_finite()));
    for target in 0..4 {
        let concentration = spectrum[target * 15 + 14];
        assert!(concentration.is_finite() && concentration > 0.0 && concentration <= 1.0);
    }
    for index in 0..4 {
        assert_eq!(
            covariance[12 + index],
            covariance[index] + covariance[4 + index] + 2.0 * covariance[8 + index]
        );
    }
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn individual_v5_default_observation_export_is_atomic_and_reports_real_fit() {
    check_individual_v5_fitter_export(false, false, None);
}

#[test]
fn unified_v3_observation_export_is_atomic_and_reports_real_fit() {
    check_individual_v5_fitter_export(false, true, None);
}

#[test]
fn unified_v3_match_export_is_atomic_and_reports_real_fit() {
    check_individual_v5_fitter_export(true, true, None);
}

#[test]
fn direct_v4_exports_requested_counts_for_both_units() {
    for grouped in [false, true] {
        for probes in [512, 2048] {
            check_individual_v5_fitter_export(grouped, true, Some(probes));
        }
    }
}

fn check_individual_v5_fitter_export(grouped: bool, unified: bool, gram_probes: Option<u32>) {
    use vckss_plugin::ffi_engine::{
        vckss_rust_component_inference_interface_version,
        vckss_rust_engine_augment_component_inference_interrupt_v2,
        vckss_rust_engine_augment_component_inference_interrupt_v3,
        vckss_rust_engine_augment_match_component_inference_interrupt_v3,
        vckss_rust_engine_component_inference_result_v5, VckssComponentInferenceResultReceiptV5,
    };
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    assert_eq!(vckss_rust_component_inference_interface_version(), 4);
    assert_eq!(size_of::<VckssComponentInferenceResultReceiptV5>(), 288);
    let mut columns = OwnedColumns::structured_component();
    if grouped {
        for row in 0..columns.worker.len() {
            columns.deletion[row] = (row / 2 + 1) as f64;
            columns.frequency[row] = 2.0;
        }
    }
    let deletion = if grouped {
        VCKSS_DELETION_MATCH
    } else {
        VCKSS_DELETION_OBSERVATION
    };
    let nuisance = if grouped {
        VCKSS_NUISANCE_FIXED_OFFSET
    } else {
        VCKSS_NUISANCE_JOINT
    };
    let generation = prepare_with_controls(&columns, &[], deletion);
    let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(
            &mut augmentation,
            bytes::<VckssComponentInferenceAugmentationRequestInterruptV1>()
        ),
        0
    );
    augmentation.options.variance_source = VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON;
    augmentation.options.seed = 8_675_309;
    augmentation.options.spectrum_iterations = 512;
    let attach = if grouped {
        vckss_rust_engine_augment_match_component_inference_interrupt_v3
    } else if unified {
        vckss_rust_engine_augment_component_inference_interrupt_v3
    } else {
        vckss_rust_engine_augment_component_inference_interrupt_v2
    };
    if let Some(probes) = gram_probes {
        let attach_v4 = if grouped {
            vckss_plugin::ffi_engine::vckss_rust_engine_augment_match_component_inference_interrupt_v4
        } else {
            vckss_plugin::ffi_engine::vckss_rust_engine_augment_component_inference_interrupt_v4
        };
        for invalid in [0, 1, 511, i32::MAX as u32 + 1, u32::MAX] {
            assert_eq!(
                attach_v4(generation, &augmentation, invalid),
                ErrorCode::InvalidInput as i32
            );
        }
        assert_eq!(attach_v4(generation, &augmentation, probes), 0);
    } else {
        assert_eq!(attach(generation, &augmentation), 0);
    }
    let capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        deletion,
        nuisance,
        0,
        VCKSS_BATCH_MODE_EXPLICIT,
        VCKSS_BATCH_MODE_EXPLICIT,
    );
    let mut solve = planned_solve_request(capability, 16, 16);
    solve.v3.v2.v1.seed = 8_675_309;
    solve.v3.v2.v1.probes = 200;
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &solve),
        0,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    let mut primitive = [42.0; 9];
    let mut covariance = [42.0; 16];
    let mut mcse = [42.0; 9];
    let mut spectrum = [42.0; 60];
    let mut summaries = [42.0; 24];
    let mut folds = [42.0; 150];
    let mut cv = [42.0; 490];
    let mut targets = [42.0; 8];
    let mut receipt = VckssComponentInferenceResultReceiptV5::default();
    for (target_capacity, receipt_capacity) in [(7, 288), (8, 287), (8, 288)] {
        let status = vckss_rust_engine_component_inference_result_v5(
            generation,
            primitive.as_mut_ptr(),
            9,
            covariance.as_mut_ptr(),
            16,
            mcse.as_mut_ptr(),
            9,
            spectrum.as_mut_ptr(),
            60,
            ptr::null_mut(),
            0,
            summaries.as_mut_ptr(),
            24,
            folds.as_mut_ptr(),
            150,
            cv.as_mut_ptr(),
            490,
            targets.as_mut_ptr(),
            target_capacity,
            &mut receipt,
            receipt_capacity,
        );
        if target_capacity == 7 || receipt_capacity == 287 {
            assert_ne!(status, 0);
            assert_eq!(primitive, [42.0; 9]);
            assert_eq!(targets, [42.0; 8]);
            assert_eq!(receipt.v4.v3.v2.schema_version, 0);
        } else {
            assert_eq!(status, 0);
        }
    }
    assert_eq!(receipt.v4.v3.v2.schema_version, 5);
    assert_eq!(receipt.variance_fit, 2);
    assert_eq!(receipt.gram_probes, gram_probes.unwrap_or(512));
    assert_eq!(receipt.ordering, if grouped { 3 } else { 2 });
    assert!(receipt.gram_rcond > 0.0);
    assert!(receipt.v4.solver_columns > 1515);
    assert_eq!(receipt.v4.v3.v2.fold_rows, 0);
    assert!(summaries
        .iter()
        .chain(&folds)
        .chain(&cv)
        .all(|x| x.is_nan()));
    assert!(targets.iter().all(|x| x.is_finite()));
    for target in 0..4 {
        assert_eq!(
            targets[2 * target + 1],
            f64::from(receipt.q0_status[target])
        );
    }
    if receipt.joint_status == 0 {
        assert!(covariance.iter().all(|x| x.is_finite()));
    } else {
        assert!(primitive.iter().chain(&covariance).all(|x| x.is_nan()));
    }
    assert_eq!(vckss_rust_engine_release_v1(generation), 0);
    reset();
}

#[test]
fn match_component_attachment_requires_fixedoffset_and_exports_independent_units() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let mut columns = OwnedColumns::structured_component_sized(20, 20);
    for row in 0..columns.worker.len() {
        columns.deletion[row] = (row / 2 + 1) as f64;
        columns.frequency[row] = 2.0;
        let worker = columns.worker[row] - 1.0;
        let firm = columns.firm[row] - 1.0;
        let shock =
            (((row * 37 + 11) % 101) as f64 / 50.0 - 1.0) * (1.4 + 0.05 * worker + 0.036 * firm);
        columns.outcome[row] = worker - 0.8 * firm + shock;
    }
    let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
    augmentation.options.probes = 512;
    augmentation.options.spectrum_probes = 32;
    augmentation.options.spectrum_iterations = 128;
    augmentation.options.spectrum_tolerance = 1e-2;
    for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
        let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
        assert_eq!(
            vckss_rust_engine_augment_match_component_inference_interrupt_v1(
                generation,
                &augmentation
            ),
            ErrorCode::Ok as i32,
        );
        let capability = planned_capability_request(
            VCKSS_ALGORITHM_JLA,
            VCKSS_ENGINE_GENERIC,
            VCKSS_ROUTE_DIAGONAL_PCG,
            VCKSS_DELETION_MATCH,
            nuisance,
            0,
            VCKSS_BATCH_MODE_EXPLICIT,
            VCKSS_BATCH_MODE_EXPLICIT,
        );
        let mut solve = planned_solve_request(capability, 13, 17);
        solve.v3.v2.v1.probes = 256;
        let status = vckss_rust_engine_solve_v4(generation, &solve);
        if nuisance == VCKSS_NUISANCE_JOINT {
            assert_eq!(status, ErrorCode::UnsupportedFeature as i32);
        } else {
            assert_eq!(
                status,
                ErrorCode::Ok as i32,
                "{}",
                unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
            );
            let mut receipt = VckssComponentInferenceUnitReceiptV1::default();
            assert_eq!(size_of::<VckssComponentInferenceUnitReceiptV1>(), 64);
            assert_ne!(
                vckss_rust_engine_component_inference_unit_receipt_v1(generation, &mut receipt, 63),
                ErrorCode::Ok as i32
            );
            assert_eq!(receipt, VckssComponentInferenceUnitReceiptV1::default());
            assert_eq!(
                vckss_rust_engine_component_inference_unit_receipt_v1(generation, &mut receipt, 64),
                ErrorCode::Ok as i32
            );
            assert_eq!(receipt.schema_version, 1);
            assert_eq!(receipt.generation, generation);
            assert_eq!(receipt.deletion_mode, VCKSS_DELETION_MATCH);
            assert_eq!(receipt.nuisance_uncertainty_omitted, 1);
            assert_eq!(receipt.independent_units, 400);
            assert!((receipt.effective_match_count - 400.0).abs() < 1e-12);
            assert!((receipt.largest_match_mass_share - 1.0 / 400.0).abs() < 1e-12);
            assert!(receipt.largest_match_leverage > 0.0);
            assert!(receipt.smallest_maker_denominator > 0.0);
        }
        assert_eq!(
            vckss_rust_engine_release_v1(generation),
            ErrorCode::Ok as i32
        );
    }
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    assert_eq!(
        vckss_rust_engine_augment_match_component_inference_interrupt_v1(generation, &augmentation),
        ErrorCode::UnsupportedFeature as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn q1_component_inference_v3_reports_raw_recenter_and_identity_diagnostics() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();

    let columns = OwnedColumns::structured_component();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_component_inference_augmentation_request_interrupt_v1(
            &mut augmentation,
            bytes::<VckssComponentInferenceAugmentationRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    augmentation.options.variance_source = VCKSS_COMPONENT_VARIANCE_STRUCTURED_COMMON;
    augmentation.options.reference_distribution = VCKSS_COMPONENT_REFERENCE_Q1;
    augmentation.options.seed = 0x31c0_ffee_782a_19d4;
    augmentation.options.probes = 512;
    augmentation.options.batch_width = 13;
    augmentation.options.spectrum_probes = 32;
    augmentation.options.spectrum_iterations = 128;
    augmentation.options.spectrum_tolerance = 1.0e-2;
    augmentation.options.fold_seed = 0x83d4_2556_97ab_c10e;
    augmentation.options.critical_simulations = 100_000;
    assert_eq!(
        vckss_rust_engine_augment_component_inference_interrupt_v1(generation, &augmentation),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let capability_request = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_EXPLICIT,
        VCKSS_BATCH_MODE_EXPLICIT,
    );
    let mut solve = planned_solve_request(capability_request, 13, 17);
    solve.v3.v2.v1.probes = 256;
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &solve),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let mut primitive = [0.0; 9];
    let mut covariance = [0.0; 16];
    let mut mcse = [0.0; 9];
    let mut spectrum = [0.0; 60];
    let mut q1 = [0.0; 64];
    let mut summaries = [0.0; 24];
    let mut folds = [0.0; 150];
    let mut cv = [0.0; 490];
    let mut receipt = VckssComponentInferenceResultReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_component_inference_result_v3(
            generation,
            primitive.as_mut_ptr(),
            primitive.len() as u64,
            covariance.as_mut_ptr(),
            covariance.len() as u64,
            mcse.as_mut_ptr(),
            mcse.len() as u64,
            spectrum.as_mut_ptr(),
            spectrum.len() as u64,
            q1.as_mut_ptr(),
            q1.len() as u64,
            summaries.as_mut_ptr(),
            summaries.len() as u64,
            folds.as_mut_ptr(),
            folds.len() as u64,
            cv.as_mut_ptr(),
            cv.len() as u64,
            &mut receipt,
            bytes::<VckssComponentInferenceResultReceiptV3>(),
        ),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    assert_eq!(
        receipt.v2.schema_version,
        VCKSS_COMPONENT_INFERENCE_RESULT_SCHEMA_V3
    );
    assert_eq!(receipt.v2.q1_present, 1);
    assert_eq!(receipt.q1_columns, 16);
    assert_eq!(receipt.critical_simulations, 100_000);
    let maximum_identity_error = q1
        .chunks_exact(16)
        .map(|row| row[15])
        .fold(0.0_f64, f64::max);
    assert_eq!(
        receipt.maximum_remainder_identity_error.to_bits(),
        maximum_identity_error.to_bits()
    );
    for row in q1.chunks_exact(16) {
        assert!(row.iter().all(|value| value.is_finite()));
        assert!(row[2] >= 0.0);
        assert!(row[14].is_finite());
        assert!(row[15] >= 0.0);
    }
    let mut q1_v4 = [f64::NAN; 80];
    let mut receipt_v4 = VckssComponentInferenceResultReceiptV4::default();
    for capacity in [79, 80] {
        let status = vckss_rust_engine_component_inference_result_v4(
            generation,
            primitive.as_mut_ptr(),
            9,
            covariance.as_mut_ptr(),
            16,
            mcse.as_mut_ptr(),
            9,
            spectrum.as_mut_ptr(),
            60,
            q1_v4.as_mut_ptr(),
            capacity,
            summaries.as_mut_ptr(),
            24,
            folds.as_mut_ptr(),
            150,
            cv.as_mut_ptr(),
            490,
            &mut receipt_v4,
            bytes::<VckssComponentInferenceResultReceiptV4>(),
        );
        if capacity == 79 {
            assert_ne!(status, ErrorCode::Ok as i32);
            assert!(q1_v4.iter().all(|value| value.is_nan()));
        } else {
            assert_eq!(status, ErrorCode::Ok as i32);
        }
    }
    assert_eq!(receipt_v4.v3.v2.schema_version, 4);
    assert_eq!(receipt_v4.v3.q1_columns, 20);
    assert_eq!(receipt_v4.computed_targets, 4);
    assert_eq!(size_of::<VckssComponentInferenceResultReceiptV4>(), 208);
    assert_eq!(receipt_v4.critical_draws, 400000);
    assert!(receipt_v4.solver_columns > 512);
    for (old, new) in q1.chunks_exact(16).zip(q1_v4.chunks_exact(20)) {
        assert_eq!(old, &new[..16]);
        assert_eq!(new[16], 0.0);
        assert!(new[17] > 0.0 && new[17] <= 1.0);
        assert!((new[18] - new[19] - new[6]).abs() < 1e-12);
    }
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    // An uncertified mode must reach the versioned boundary as a missing
    // target interval, not suppress the shared covariance or point results.
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    augmentation.options.spectrum_iterations = 2;
    augmentation.options.spectrum_tolerance = 1e-10;
    assert_eq!(
        vckss_rust_engine_augment_component_inference_interrupt_v1(generation, &augmentation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &solve),
        ErrorCode::Ok as i32
    );
    q1.fill(123.0);
    primitive.fill(123.0);
    assert_eq!(
        vckss_rust_engine_component_inference_result_v3(
            generation,
            primitive.as_mut_ptr(),
            9,
            covariance.as_mut_ptr(),
            16,
            mcse.as_mut_ptr(),
            9,
            spectrum.as_mut_ptr(),
            60,
            q1.as_mut_ptr(),
            64,
            summaries.as_mut_ptr(),
            24,
            folds.as_mut_ptr(),
            150,
            cv.as_mut_ptr(),
            490,
            &mut receipt,
            bytes::<VckssComponentInferenceResultReceiptV3>(),
        ),
        ErrorCode::UnsupportedFeature as i32,
    );
    assert_eq!(q1, [123.0; 64]);
    assert_eq!(primitive, [123.0; 9]);
    assert_eq!(
        vckss_rust_engine_component_inference_result_v4(
            generation,
            primitive.as_mut_ptr(),
            9,
            covariance.as_mut_ptr(),
            16,
            mcse.as_mut_ptr(),
            9,
            spectrum.as_mut_ptr(),
            60,
            q1_v4.as_mut_ptr(),
            80,
            summaries.as_mut_ptr(),
            24,
            folds.as_mut_ptr(),
            150,
            cv.as_mut_ptr(),
            490,
            &mut receipt_v4,
            bytes::<VckssComponentInferenceResultReceiptV4>(),
        ),
        ErrorCode::Ok as i32,
    );
    assert!(receipt_v4.computed_targets < 4);
    assert!(primitive
        .iter()
        .chain(&covariance)
        .all(|value| value.is_finite()));
    for row in q1_v4.chunks_exact(20) {
        if row[16] == 6.0 {
            assert!(row[10].is_nan() && row[11].is_nan());
            assert!(row[15] <= 1e-9);
        }
    }
    assert_eq!(
        receipt_v4.critical_draws,
        100000 * u64::from(receipt_v4.computed_targets)
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn v4_exact_compressed_and_generic_store_truthful_frozen_execution_plans() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let columns = OwnedColumns::generic_dense();
    let control = one_generic_control(&columns);
    for (algorithm, engine, controls, expected_engine, applicability) in [
        (
            VCKSS_ALGORITHM_AUTO,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            1_usize,
            VCKSS_ENGINE_NOT_APPLICABLE,
            VCKSS_PLAN_APPLICABILITY_EXACT,
        ),
        (
            VCKSS_ALGORITHM_JLA,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            0,
            VCKSS_ENGINE_COMPRESSED,
            VCKSS_PLAN_APPLICABILITY_COMPRESSED,
        ),
        (
            VCKSS_ALGORITHM_JLA,
            VCKSS_ENGINE_GENERIC,
            1,
            VCKSS_ENGINE_GENERIC,
            VCKSS_PLAN_APPLICABILITY_GENERIC,
        ),
    ] {
        reset();
        let controls_values = if controls == 0 {
            Vec::new()
        } else {
            vec![control.clone()]
        };
        let generation = prepare_with_controls(&columns, &controls_values, VCKSS_DELETION_MATCH);
        let route = if algorithm == VCKSS_ALGORITHM_AUTO {
            VCKSS_ROUTE_AUTO
        } else {
            VCKSS_ROUTE_DIAGONAL_PCG
        };
        let request = planned_solve_request(
            planned_capability_request(
                algorithm,
                engine,
                route,
                VCKSS_DELETION_MATCH,
                VCKSS_NUISANCE_JOINT,
                controls as u32,
                VCKSS_BATCH_MODE_AUTO,
                VCKSS_BATCH_MODE_EXPLICIT,
            ),
            0,
            2,
        );
        assert_eq!(
            vckss_rust_engine_solve_v4(generation, &request),
            ErrorCode::Ok as i32,
            "{}",
            unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
        );
        let mut plan = VckssExecutionPlanReceiptV1::default();
        assert_eq!(
            vckss_rust_engine_execution_plan_receipt_v1(
                generation,
                &mut plan,
                bytes::<VckssExecutionPlanReceiptV1>(),
            ),
            ErrorCode::Ok as i32
        );
        assert_eq!(plan.generation, generation);
        assert_eq!(plan.request_signature, request.v3.request_signature);
        assert_eq!(plan.resolution.engine_selected, expected_engine);
        assert_eq!(plan.solver.applicability, applicability);
        assert_eq!(plan.solver.threads_requested, 1);
        assert_eq!(plan.solver.threads_used, 1);
        assert_eq!(plan.solver.parallel_regions, 0);
        assert_eq!(plan.solver.logical_atoms_before_plan_freeze, 0);
        assert_eq!(plan.solver.unique_words_before_plan_freeze, 0);
        assert_eq!(plan.solver.physical_trials_before_plan_freeze, 0);
        assert_eq!(plan.counter.logical_atoms_before_plan_freeze, 0);
        assert_eq!(plan.counter.unique_words_before_plan_freeze, 0);
        assert_eq!(plan.counter.physical_trials_before_plan_freeze, 0);
        if expected_engine == VCKSS_ENGINE_NOT_APPLICABLE {
            assert_eq!(plan.solver.requested_route, VCKSS_ROUTE_NOT_APPLICABLE);
            assert_eq!(plan.counter.rng_contract, VCKSS_RNG_NONE);
            assert_eq!(plan.counter.total.actual_logical_atoms, 0);
        } else {
            assert_eq!(plan.counter.rng_contract, VCKSS_RNG_COUNTER_V1);
            assert!(plan.counter.total.actual_logical_atoms > 0);
            assert_eq!(
                plan.counter.total.actual_logical_atoms,
                plan.counter.leverage.actual_logical_atoms
                    + plan.counter.target.actual_logical_atoms
            );
            assert_eq!(plan.counter.completed, 1);
            assert_eq!(plan.batch.leverage.request_mode, VCKSS_BATCH_MODE_AUTO);
            assert_eq!(plan.batch.target.request_mode, VCKSS_BATCH_MODE_EXPLICIT);
        }
        let mut detailed = VckssEngineDetailedReceiptV7::default();
        assert_eq!(
            vckss_rust_engine_detailed_receipt_v7(
                generation,
                &mut detailed,
                bytes::<VckssEngineDetailedReceiptV7>(),
            ),
            ErrorCode::Ok as i32
        );
        assert_eq!(
            plan.solver.planned_rhs,
            detailed.v6.v5.v4.v3.rhs_receipt_rows
        );
        assert_eq!(detailed.execution, plan);
        assert_eq!(detailed.v6.engine_selected, expected_engine);
        assert_eq!(
            detailed.v6.capability_schema,
            VCKSS_REQUEST_CAPABILITY_SCHEMA_V3
        );
        assert_eq!(detailed.v6.request_signature, request.v3.request_signature);
        let mut performance = VckssEnginePerformanceReceiptV1::default();
        assert_eq!(
            vckss_rust_engine_performance_receipt_v1(
                generation,
                &mut performance,
                bytes::<VckssEnginePerformanceReceiptV1>(),
            ),
            ErrorCode::Ok as i32
        );
        assert_eq!(performance.struct_size, 96);
        assert_eq!(performance.schema_version, 1);
        assert_eq!(performance.generation, generation);
        assert_eq!(performance.applicability_flags & 3, 3);
        assert_eq!(
            performance.algorithm_selected,
            detailed.v6.v5.v4.algorithm_selected
        );
        assert_eq!(performance.engine_selected, expected_engine);
        assert_eq!(
            performance.native_total_ns,
            performance
                .ingest_ns
                .saturating_add(performance.canonicalize_ns)
                .saturating_add(performance.graph_ns)
                .saturating_add(performance.compress_ns)
                .saturating_add(performance.plan_ns)
                .saturating_add(performance.stayer_augmentation_ns)
                .saturating_add(performance.solve_ns)
        );
        assert_eq!(
            vckss_rust_engine_release_v1(generation),
            ErrorCode::Ok as i32
        );
    }
}

#[test]
fn v5_explicit_full_cmg_obeys_platform_contract_and_exports_source_receipt() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_implicit_match_probe_order_memory(&columns, 1_u64 << 30);
    let mut capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    capability.v2.v1.frequency_use = VCKSS_REQUEST_FREQUENCY_UNIT;
    capability.v2.target_weight_mode = VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT;
    capability.v2.deletion_unit_source = VCKSS_DELETION_SOURCE_CELL_DEFAULT;
    capability.v2.probeorder_supplied = 1;
    let mut v4 = planned_solve_request(capability, 0, 0);
    v4.v3.v2.v1.pcg_tolerance = 1.0e-10;
    let mut request = VckssEngineSolveRequestV5 {
        v4,
        threads: 2,
        tolerance_supplied: 0,
        full_cmg_v2: 1,
        reserved_5: 0,
    };
    request.v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestV5>();
    let solve_status = vckss_rust_engine_solve_v5(generation, &request);
    if cfg!(not(any(target_os = "macos", target_os = "linux"))) {
        assert_eq!(
            solve_status,
            ErrorCode::UnsupportedFeature as i32,
            "{}",
            unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
        );
        assert_eq!(
            vckss_rust_engine_release_v1(generation),
            ErrorCode::Ok as i32
        );
        return;
    }
    assert_eq!(
        solve_status,
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let mut receipt = unsafe { std::mem::zeroed::<VckssFullCmgReceiptV1>() };
    assert_eq!(
        vckss_rust_engine_full_cmg_receipt_v1(
            generation,
            &mut receipt,
            bytes::<VckssFullCmgReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.schema_version, 1);
    assert_eq!(receipt.backend_identity, 2);
    assert_eq!(receipt.generation, generation);
    assert_eq!(receipt.threads_requested, 2);
    assert_eq!(receipt.threads_used, 2);
    assert_eq!(receipt.fit_effective_tolerance, 1.0e-10);
    assert_eq!(receipt.probe_effective_tolerance, 1.0e-6);
    assert_eq!(receipt.fit_initial_inner_tolerance, 1.0e-12);
    assert_eq!(receipt.probe_initial_inner_tolerance, 1.0e-6);
    assert_eq!(
        std::str::from_utf8(&receipt.cmg_source_commit).expect("source commit"),
        "92a12f2d572ca56b30a035220953f9dd4bced999"
    );
    assert!(receipt.hierarchy_levels > 0);
    assert!(receipt.rhs_count > 0);
    assert!(receipt.total_operator_applications >= receipt.total_iterations);
    assert!(receipt.maximum_complete_residual <= 1.0e-5);
    assert!(receipt.preparation_peak_bytes > 0);
    assert!(receipt.prepared_persistent_bytes > 0);
    assert!(receipt.non_cmg_command_peak_bytes >= receipt.prepared_persistent_bytes);
    assert!(receipt.pre_rng_forecast_bytes >= receipt.admitted_peak_bytes);
    assert!(receipt.actual_retained_bytes > 0);
    assert_eq!(
        receipt.allocator_allowance_bytes,
        receipt.actual_retained_bytes / 5
    );
    // Selected automatic capacity: this fixture has five probes and
    // each target probe produces two RHSs. The public field/layout is fixed.
    assert_eq!(receipt.maximum_batch_rhs, 10);
    assert!(receipt.workspace_count > 0);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn v5_full_cmg_user_break_is_coordinated_and_generation_is_releasable_once() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_implicit_match_probe_order_memory(&columns, 1_u64 << 30);
    let mut capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    capability.v2.v1.frequency_use = VCKSS_REQUEST_FREQUENCY_UNIT;
    capability.v2.target_weight_mode = VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT;
    capability.v2.deletion_unit_source = VCKSS_DELETION_SOURCE_CELL_DEFAULT;
    capability.v2.probeorder_supplied = 1;
    let mut v4 = planned_solve_request(capability, 0, 0);
    v4.v3.v2.v1.pcg_tolerance = 1.0e-10;
    let options = VckssEngineSolveRequestV5 {
        v4,
        threads: 2,
        tolerance_supplied: 0,
        full_cmg_v2: 1,
        reserved_5: 0,
    };
    let mut request = VckssEngineSolveRequestInterruptV5::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v5(
            &mut request,
            bytes::<VckssEngineSolveRequestInterruptV5>(),
        ),
        ErrorCode::Ok as i32
    );
    request.options = options;
    request.options.v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV5>();
    let mut poll = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    request.interrupt_poll = Some(injected_poll);
    request.interrupt_context = (&mut poll as *mut PollState).cast();
    request.checkpoint_interval = 1;
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v5(generation, &request),
        ErrorCode::UserBreak as i32
    );
    assert!(poll.calls >= 1);
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 4);
    assert_eq!(snapshot.generation, generation);
    let mut preparation = VckssEnginePreparationReceiptV4::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v4(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.v3.v2.generation, generation);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 0);
    assert_eq!(snapshot.generation, 0);
}

#[test]
fn v4_physical_copy_limit_is_jla_only_and_remains_pre_rng() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let columns = OwnedColumns::generic_dense();
    let control = one_generic_control(&columns);
    let controls = vec![control];

    reset();
    let generation = prepare_with_controls(&columns, &controls, VCKSS_DELETION_MATCH);
    let mut exact_capability = planned_capability_request(
        VCKSS_ALGORITHM_AUTO,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        1,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    exact_capability.v2.physical_limit = 1;
    let exact_request = planned_solve_request(exact_capability, 0, 0);
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &exact_request),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    let mut exact_receipt = VckssEngineDetailedReceiptV7::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v7(
            generation,
            &mut exact_receipt,
            bytes::<VckssEngineDetailedReceiptV7>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        exact_receipt.execution.resolution.engine_selected,
        VCKSS_ENGINE_NOT_APPLICABLE
    );
    assert_eq!(exact_receipt.execution.counter.completed, 1);
    assert_eq!(
        exact_receipt.execution.counter.total.actual_logical_atoms,
        0
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    reset();
    let generation = prepare_with_controls(&columns, &controls, VCKSS_DELETION_MATCH);
    let mut jla_capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        1,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    jla_capability.v2.physical_limit = 1;
    let jla_request = planned_solve_request(jla_capability, 0, 0);
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &jla_request),
        ErrorCode::ResourceLimit as i32
    );
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy();
    assert!(error.contains("retained physical mass exceeds physical_limit()"));
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn exact_stayer_augmentation_is_reconciled_solved_and_released_once() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let mover_control = one_generic_control(&columns);
    let generation = prepare_with_controls(
        &columns,
        std::slice::from_ref(&mover_control),
        VCKSS_DELETION_MATCH,
    );

    let firm = [1.0, 1.0, 2.0];
    let worker = [1.0, 1.0, 2.0];
    let outcome = [0.25, 1.75, -0.5];
    let frequency = [1.0, 1.0, 2.0];
    let target_weight = [0.75, 1.25, 2.5];
    let stayer_control = [0.5, 1.5, -0.25];
    let control_pointers = [stayer_control.as_ptr()];
    let descriptor = VckssStayerAugmentationColumnsV1 {
        struct_size: bytes::<VckssStayerAugmentationColumnsV1>(),
        reserved: 0,
        rows: 3,
        firm: firm.as_ptr(),
        worker: worker.as_ptr(),
        outcome: outcome.as_ptr(),
        frequency: frequency.as_ptr(),
        target_weight: target_weight.as_ptr(),
        controls: control_pointers.as_ptr(),
        controls_count: 1,
        reserved_2: 0,
    };
    let request = VckssStayerAugmentationRequestV1 {
        rows: 3,
        controls_count: 1,
        caller_copy_bytes: 3 * 6 * 8,
        ..VckssStayerAugmentationRequestV1::default()
    };
    assert_eq!(
        vckss_rust_engine_augment_stayers_v1(generation, &request, &descriptor),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let mut augmentation = VckssStayerAugmentationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_stayer_augmentation_receipt_v1(
            generation,
            &mut augmentation,
            bytes::<VckssStayerAugmentationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(augmentation.generation, generation);
    assert_eq!(augmentation.mover_stored_rows, columns.worker.len() as u64);
    assert_eq!(augmentation.stayer_stored_rows, 3);
    assert_eq!(
        augmentation.combined_stored_rows,
        columns.worker.len() as u64 + 3
    );
    assert_eq!(augmentation.stayer_physical_mass, 4);
    assert_eq!(augmentation.stayer_workers, 2);
    assert_eq!(augmentation.stayer_deletion_units, 4);
    assert_eq!(augmentation.stayer_target_mass, 4.5);
    assert!(augmentation.augmentation_peak_forecast_bytes <= augmentation.memory_limit_bytes);
    assert!(augmentation.total_prepared_resident_bytes > augmentation.augmented_resident_bytes);

    let mut capability = planned_capability_request(
        VCKSS_ALGORITHM_EXACT,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        1,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    capability.v2.stayers_mode = VCKSS_STAYERS_ALL;
    let solve = planned_solve_request(capability, 0, 0);
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &solve),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let mut hybrid = VckssStayerHybridResultV1::default();
    assert_eq!(
        vckss_rust_engine_stayer_hybrid_result_v1(
            generation,
            &mut hybrid,
            bytes::<VckssStayerHybridResultV1>(),
        ),
        ErrorCode::Ok as i32
    );
    for (total, mover, stayer) in [
        (
            hybrid.correction.worker,
            hybrid.mover_correction.worker,
            hybrid.stayer_correction.worker,
        ),
        (
            hybrid.correction.firm,
            hybrid.mover_correction.firm,
            hybrid.stayer_correction.firm,
        ),
        (
            hybrid.correction.covariance,
            hybrid.mover_correction.covariance,
            hybrid.stayer_correction.covariance,
        ),
        (
            hybrid.correction.total,
            hybrid.mover_correction.total,
            hybrid.stayer_correction.total,
        ),
    ] {
        assert!((total - mover - stayer).abs() < 1.0e-12);
    }
    assert_eq!(hybrid.deletion_units, augmentation.combined_deletion_units);
    assert_eq!(hybrid.topology_checksum, augmentation.topology_checksum);
    assert!(hybrid.accounting_residual <= 1.0e-12);
    assert!(hybrid.peak_forecast_bytes <= augmentation.memory_limit_bytes);

    let mut detailed = VckssEngineDetailedReceiptV7::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v7(
            generation,
            &mut detailed,
            bytes::<VckssEngineDetailedReceiptV7>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(detailed.v6.stayers_mode, VCKSS_STAYERS_ALL);
    assert_eq!(
        detailed.v6.v5.v4.v3.v2.prepared_resident_bytes,
        augmentation.total_prepared_resident_bytes
    );
    assert_eq!(
        detailed.execution.memory.prepared_persistent_bytes,
        augmentation.total_prepared_resident_bytes
    );
    assert_eq!(
        detailed.execution.resolution.engine_selected,
        VCKSS_ENGINE_NOT_APPLICABLE
    );
    assert_eq!(detailed.execution.counter.total.actual_logical_atoms, 0);
    assert_eq!(
        detailed.execution.counter.total.actual_unique_packed_words,
        0
    );
    assert_eq!(detailed.execution.counter.total.actual_physical_trials, 0);

    let mut after_solve = VckssStayerAugmentationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_stayer_augmentation_receipt_v1(
            generation,
            &mut after_solve,
            bytes::<VckssStayerAugmentationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(augmentation, after_solve);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn generic_jla_stayer_augmentation_is_the_primary_combined_result() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);

    let firm = [1.0, 1.0, 2.0];
    let worker = [1.0, 1.0, 2.0];
    let outcome = [0.25, 1.75, -0.5];
    let frequency = [1.0, 1.0, 2.0];
    let target_weight = [0.75, 1.25, 2.5];
    let descriptor = VckssStayerAugmentationColumnsV1 {
        struct_size: bytes::<VckssStayerAugmentationColumnsV1>(),
        reserved: 0,
        rows: 3,
        firm: firm.as_ptr(),
        worker: worker.as_ptr(),
        outcome: outcome.as_ptr(),
        frequency: frequency.as_ptr(),
        target_weight: target_weight.as_ptr(),
        controls: ptr::null(),
        controls_count: 0,
        reserved_2: 0,
    };
    let request = VckssStayerAugmentationRequestV1 {
        rows: 3,
        controls_count: 0,
        caller_copy_bytes: 3 * 5 * 8,
        ..VckssStayerAugmentationRequestV1::default()
    };
    assert_eq!(
        vckss_rust_engine_augment_stayers_v1(generation, &request, &descriptor),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
    let mut augmentation = VckssStayerAugmentationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_stayer_augmentation_receipt_v1(
            generation,
            &mut augmentation,
            bytes::<VckssStayerAugmentationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );

    let mut capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    capability.v2.stayers_mode = VCKSS_STAYERS_ALL;
    let solve = planned_solve_request(capability, 0, 0);
    assert_eq!(
        vckss_rust_engine_solve_v4(generation, &solve),
        ErrorCode::Ok as i32,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );

    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::Ok as i32
    );
    for value in [result.plugin, result.correction, result.corrected] {
        assert!(value.worker.is_finite());
        assert!(value.firm.is_finite());
        assert!(value.covariance.is_finite());
        assert!(value.total.is_finite());
        assert!((value.total - value.worker - value.firm - 2.0 * value.covariance).abs() <= 1e-10);
    }

    let mut detailed = VckssEngineDetailedReceiptV7::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v7(
            generation,
            &mut detailed,
            bytes::<VckssEngineDetailedReceiptV7>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(detailed.v6.stayers_mode, VCKSS_STAYERS_ALL);
    assert_eq!(detailed.v6.engine_selected, VCKSS_ENGINE_GENERIC);
    assert_eq!(
        detailed.v6.v5.v4.v3.v2.prepared_resident_bytes,
        augmentation.total_prepared_resident_bytes
    );
    assert_eq!(
        detailed.v6.v5.v4.parameters,
        augmentation.combined_workers + augmentation.firms - 1
    );
    assert_eq!(
        detailed.v6.v5.v4.full_parameters,
        augmentation.combined_workers + augmentation.firms - 1
    );
    assert_eq!(
        detailed.execution.resolution.engine_selected,
        VCKSS_ENGINE_GENERIC
    );
    assert_eq!(detailed.execution.counter.completed, 1);
    assert!(detailed.execution.counter.total.actual_logical_atoms > 0);
    assert!(detailed.v6.v5.actual_accounting_residual <= 1.0e-10);

    let mut exact_only = VckssStayerHybridResultV1::default();
    assert_eq!(
        vckss_rust_engine_stayer_hybrid_result_v1(
            generation,
            &mut exact_only,
            bytes::<VckssStayerHybridResultV1>(),
        ),
        ErrorCode::UnsupportedFeature as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn interrupted_stayer_copy_leaves_generation_releasable() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
    let rows = 20_000_usize;
    let firm = vec![1.0; rows];
    let worker = vec![1.0; rows];
    let outcome = vec![0.0; rows];
    let frequency = vec![1.0; rows];
    let target_weight = vec![1.0; rows];
    let descriptor = VckssStayerAugmentationColumnsV1 {
        struct_size: bytes::<VckssStayerAugmentationColumnsV1>(),
        reserved: 0,
        rows: rows as u64,
        firm: firm.as_ptr(),
        worker: worker.as_ptr(),
        outcome: outcome.as_ptr(),
        frequency: frequency.as_ptr(),
        target_weight: target_weight.as_ptr(),
        controls: ptr::null(),
        controls_count: 0,
        reserved_2: 0,
    };
    let mut request = VckssStayerAugmentationRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_stayer_augmentation_request_interrupt_v1(
            &mut request,
            bytes::<VckssStayerAugmentationRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    request.options.rows = rows as u64;
    request.options.caller_copy_bytes = rows as u64 * 5 * 8;
    let mut poll = PollState {
        calls: 0,
        stop_at: 2,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    request.interrupt_poll = Some(injected_poll);
    request.interrupt_context = (&mut poll as *mut PollState).cast();
    request.checkpoint_interval = 1;
    assert_eq!(
        vckss_rust_engine_augment_stayers_interrupt_v1(generation, &request, &descriptor),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn v4_interrupt_default_and_user_break_preserve_failed_generation_lifecycle() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::generic_dense();
    let control = one_generic_control(&columns);
    let generation = prepare_with_controls(&columns, &[control], VCKSS_DELETION_MATCH);
    let options = planned_solve_request(
        planned_capability_request(
            VCKSS_ALGORITHM_JLA,
            VCKSS_ENGINE_GENERIC,
            VCKSS_ROUTE_DIAGONAL_PCG,
            VCKSS_DELETION_MATCH,
            VCKSS_NUISANCE_JOINT,
            1,
            VCKSS_BATCH_MODE_AUTO,
            VCKSS_BATCH_MODE_AUTO,
        ),
        0,
        0,
    );
    let mut request = VckssEngineSolveRequestInterruptV4::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v4(
            &mut request,
            bytes::<VckssEngineSolveRequestInterruptV4>(),
        ),
        ErrorCode::Ok as i32
    );
    request.options = options;
    request.options.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV4>();
    let mut poll = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    request.interrupt_poll = Some(injected_poll);
    request.interrupt_context = (&mut poll as *mut PollState).cast();
    request.checkpoint_interval = 1;
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v4(generation, &request),
        ErrorCode::UserBreak as i32
    );
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 4);
    assert_eq!(snapshot.generation, generation);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}
