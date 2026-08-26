// SPDX-License-Identifier: GPL-3.0-only

//! Versioned, panic-contained production engine ABI.
//!
//! The caller owns all pointer storage. Rust copies every input descriptor and
//! column before returning from preparation, retains no caller pointer, and
//! validates every output buffer capacity before writing. One generation-safe
//! registry owns the problem, preparation metadata, retained-row mask, result,
//! and numerical receipt for the complete command lifecycle.

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CString};
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, ThreadId};
use std::time::Instant;

use vckss_core::batch_plan::{
    BatchPhaseReceipt, BatchPlanReceipt, BatchRequest, BatchSelectionReason,
};
use vckss_core::cmg::CmgOptions;
use vckss_core::counter_accounting::{CounterExecutionReceipt, CounterPhaseExecutionReceipt};
use vckss_core::engine::{
    run_jla_no_controls_planned_with_interrupt, run_jla_no_controls_with_interrupt,
    run_jla_no_controls_with_plan_and_interrupt, CompressedJlaExecutionReceipt, JlaEngineOptions,
    JlaEngineResult, JlaRhsReceipt, JlaRhsSide, JlaSolvePhase, NumericalMcse,
    PlannedJlaEngineOptions,
};
use vckss_core::engine_plan::{
    resolve_estimator_plan, AlgorithmRequest, AlgorithmResolutionReason,
    CompressedEligibilityReason, EngineRequest, EngineResolutionReason, EstimatorAlgorithm,
    EstimatorPlanReceipt, EstimatorPlanRequest, SelectedEngine,
};
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::exact_estimator::{
    run_exact_estimator_planned_with_interrupt, run_exact_estimator_with_interrupt,
    run_exact_stayer_hybrid_with_interrupt, ExactEstimatorOptions, ExactEstimatorResult,
    ExactExecutionReceipt, ExactStayerHybridResult, PlannedExactEstimatorOptions,
};
use vckss_core::full_cmg::{FullCmgPlanOptions, FullCmgReceipt};
use vckss_core::generic_batch::ModelPcgStatus;
use vckss_core::generic_jla::{
    run_generic_jla_routed_with_interrupt, run_generic_jla_with_interrupt,
    GenericJlaExecutionOptions, GenericJlaExecutionReceipt, GenericJlaMemoryPeakPhase,
    GenericJlaOptions, GenericJlaResult, GenericJlaRhsPhase, GenericJlaRhsReceipt,
    GenericJlaRhsSide,
};
use vckss_core::interrupt::{
    checkpoint_chunk, CancellationInterrupt, InterruptCheck, NeverInterrupt,
};
use vckss_core::jla::VarianceComponents;
use vckss_core::krylov::PcgOptions;
use vckss_core::model_solver::{ModelRoutingOptions, ModelSolverOptions, ModelSolverRoute};
use vckss_core::rng::MAX_PHYSICAL_WORDS_PER_ATOM;
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute};
use vckss_core::stayer_hybrid::{StayerAugmentationInput, StayerAugmentationReceipt};
use vckss_core::types::{
    DeletionMode, InputColumns, NuisanceMode, RngContract, MAX_EXACT_BINARY64_INTEGER,
};
use vckss_core::wall_plan::{WallAdvisoryStatus, WallWorkReceipt};
use vckss_core::ABI_VERSION;

use crate::context::{ContextHandle, ContextPayloadRef, ContextRegistry, ContextStateTag};
use crate::session::{
    admit_prepare_memory, admit_prepare_memory_with_controls_and_probe_order,
    admit_prepare_memory_with_controls_probe_order_and_implicit_match,
    admit_stayer_augmentation_memory, PreparationMemoryReceipt, PreparationReceipt,
    StayerAugmentationMemoryReceipt,
};
use crate::session_retained::{
    bit_packed_capacity_bytes, duration_ns, NativePhaseTimings, PreparedProblemWithMask,
};

pub const VCKSS_DELETION_MATCH: u32 = 1;
pub const VCKSS_DELETION_OBSERVATION: u32 = 2;
pub const VCKSS_RNG_NONE: u32 = 0;
pub const VCKSS_RNG_COUNTER_V1: u32 = 1;
pub const VCKSS_ALGORITHM_AUTO: u32 = 0;
pub const VCKSS_ALGORITHM_EXACT: u32 = 1;
pub const VCKSS_ALGORITHM_JLA: u32 = 2;
pub const VCKSS_NUISANCE_JOINT: u32 = 1;
pub const VCKSS_NUISANCE_FIXED_OFFSET: u32 = 2;
pub const VCKSS_ENGINE_AUTO_OR_UNSPECIFIED: u32 = 0;
pub const VCKSS_ENGINE_COMPRESSED: u32 = 1;
pub const VCKSS_ENGINE_GENERIC: u32 = 2;
pub const VCKSS_ENGINE_NOT_APPLICABLE: u32 = 3;
pub const VCKSS_ROUTE_AUTO: u32 = 0;
pub const VCKSS_ROUTE_EXACT: u32 = 1;
pub const VCKSS_ROUTE_DIAGONAL_PCG: u32 = 2;
pub const VCKSS_ROUTE_CMG_PCG: u32 = 3;
pub const VCKSS_CORE_MATCH_GRAPH_READY: u64 = 1 << 0;
pub const VCKSS_CORE_EXACT_READY: u64 = 1 << 1;
pub const VCKSS_CORE_DIAGONAL_PCG_READY: u64 = 1 << 2;
pub const VCKSS_CORE_BATCHED_PCG_READY: u64 = 1 << 3;
pub const VCKSS_CORE_CMG_GRAPH_READY: u64 = 1 << 4;
pub const VCKSS_CORE_SOLVER_ROUTER_READY: u64 = 1 << 5;
pub const VCKSS_CORE_COUNTER_RNG_READY: u64 = 1 << 6;
pub const VCKSS_CORE_JLA_PLAN_READY: u64 = 1 << 7;
pub const VCKSS_SUPPORT_EXACT: u64 = 1 << 0;
pub const VCKSS_SUPPORT_JLA: u64 = 1 << 1;
pub const VCKSS_SUPPORT_MATCH_DELETION: u64 = 1 << 2;
pub const VCKSS_SUPPORT_OBSERVATION_DELETION: u64 = 1 << 3;
pub const VCKSS_SUPPORT_CONTROLS: u64 = 1 << 4;
pub const VCKSS_SUPPORT_DIAGONAL: u64 = 1 << 5;
pub const VCKSS_SUPPORT_CMG: u64 = 1 << 6;
const VCKSS_BACKEND_CAPABILITIES_V1_CORE_READY_FLAGS: u64 = VCKSS_CORE_MATCH_GRAPH_READY
    | VCKSS_CORE_EXACT_READY
    | VCKSS_CORE_DIAGONAL_PCG_READY
    | VCKSS_CORE_BATCHED_PCG_READY
    | VCKSS_CORE_CMG_GRAPH_READY
    | VCKSS_CORE_SOLVER_ROUTER_READY
    | VCKSS_CORE_COUNTER_RNG_READY
    | VCKSS_CORE_JLA_PLAN_READY;
const VCKSS_BACKEND_CAPABILITIES_V1_SUPPORT_FLAGS: u64 =
    VCKSS_SUPPORT_JLA | VCKSS_SUPPORT_MATCH_DELETION | VCKSS_SUPPORT_DIAGONAL;
pub const VCKSS_INTERRUPT_CONTINUE: i32 = 0;
pub const VCKSS_INTERRUPT_USER_BREAK: i32 = 1;

pub const VCKSS_REQUEST_CAPABILITY_SCHEMA_V1: u32 = 1;
pub const VCKSS_REQUEST_CAPABILITY_SCHEMA_V2: u32 = 2;
pub const VCKSS_REQUEST_CAPABILITY_SCHEMA_V3: u32 = 3;
pub const VCKSS_REQUEST_FREQUENCY_UNIT: u32 = 0;
pub const VCKSS_REQUEST_FREQUENCY_LITERAL: u32 = 1;
pub const VCKSS_REQUEST_PROFILE_NONE: u32 = 0;
pub const VCKSS_REQUEST_PROFILE_EXACT_V1: u32 = 1;
pub const VCKSS_REQUEST_PROFILE_JLA_COUNTER_V1: u32 = 2;
pub const VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1: u32 = 3;
pub const VCKSS_REQUEST_PROFILE_PLANNED_V1: u32 = 4;
pub const VCKSS_REQUEST_REASON_SUPPORTED: u32 = 0;
pub const VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA: u32 = 1;
pub const VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM: u32 = 2;
pub const VCKSS_REQUEST_REASON_ALGORITHM_AUTO_UNRESOLVED: u32 = 3;
pub const VCKSS_REQUEST_REASON_UNKNOWN_DELETION: u32 = 4;
pub const VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE: u32 = 5;
pub const VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE: u32 = 6;
pub const VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT: u32 = 7;
pub const VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE: u32 = 8;
pub const VCKSS_REQUEST_REASON_CONTROLS_LIMIT: u32 = 9;
pub const VCKSS_REQUEST_REASON_EXACT_RNG: u32 = 10;
pub const VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE: u32 = 11;
pub const VCKSS_REQUEST_REASON_JLA_DELETION: u32 = 12;
pub const VCKSS_REQUEST_REASON_JLA_NUISANCE: u32 = 13;
pub const VCKSS_REQUEST_REASON_JLA_CONTROLS: u32 = 14;
pub const VCKSS_REQUEST_REASON_JLA_RNG: u32 = 15;
pub const VCKSS_REQUEST_REASON_UNKNOWN_ENGINE: u32 = 16;
pub const VCKSS_REQUEST_REASON_EXACT_ENGINE: u32 = 17;
pub const VCKSS_REQUEST_REASON_JLA_ENGINE_AUTO_UNRESOLVED: u32 = 18;
pub const VCKSS_REQUEST_REASON_JLA_GENERIC_SOLVER_ROUTE: u32 = 19;

pub const VCKSS_GENERIC_DIAGNOSTIC_CONTROL_RANK: u64 = 1 << 0;
pub const VCKSS_GENERIC_DIAGNOSTIC_DELETION_RANK: u64 = 1 << 1;
pub const VCKSS_GENERIC_DIAGNOSTIC_FULL_JOINT_FIT: u64 = 1 << 2;
pub const VCKSS_GENERIC_DIAGNOSTIC_WORKING_FIT: u64 = 1 << 3;
pub const VCKSS_GENERIC_DIAGNOSTIC_MAKER: u64 = 1 << 4;
pub const VCKSS_GENERIC_DIAGNOSTIC_MEMORY_PHASES: u64 = 1 << 5;
pub const VCKSS_GENERIC_DIAGNOSTIC_RHS_V2: u64 = 1 << 6;

pub const VCKSS_RHS_STATUS_ZERO: u32 = 1;
pub const VCKSS_RHS_STATUS_CONVERGED: u32 = 2;
pub const VCKSS_RESIDUAL_SPACE_WORKER_FIRM: u32 = 1;
pub const VCKSS_RESIDUAL_SPACE_WORKER_FIRM_CONTROL: u32 = 2;
pub const VCKSS_BATCH_MODE_AUTO: u32 = 0;
pub const VCKSS_BATCH_MODE_EXPLICIT: u32 = 1;
pub const VCKSS_BATCH_MODE_INDEPENDENT: u32 = 2;
pub const VCKSS_BATCH_MODE_NOT_APPLICABLE: u32 = 3;
pub const VCKSS_STAYERS_MOVERS: u32 = 1;
pub const VCKSS_STAYERS_ALL: u32 = 2;
pub const VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT: u32 = 0;
pub const VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT: u32 = 1;
pub const VCKSS_DELETION_SOURCE_CELL_DEFAULT: u32 = 1;
pub const VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT: u32 = 2;
pub const VCKSS_DELETION_SOURCE_OBSERVATION_ROW: u32 = 3;
pub const VCKSS_REQUEST_REASON_UNKNOWN_BATCH_MODE: u32 = 20;
pub const VCKSS_REQUEST_REASON_UNKNOWN_STAYERS_MODE: u32 = 21;
pub const VCKSS_REQUEST_REASON_UNKNOWN_TARGET_WEIGHT_MODE: u32 = 22;
pub const VCKSS_REQUEST_REASON_UNKNOWN_DELETION_UNIT_SOURCE: u32 = 23;
pub const VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED: u32 = 24;
pub const VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED: u32 = 25;
pub const VCKSS_REQUEST_REASON_PHYSICAL_LIMIT: u32 = 26;
pub const VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH: u32 = 27;
pub const VCKSS_REQUEST_REASON_BATCH_MODE_UNSUPPORTED: u32 = 28;
pub const VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED: u32 = 29;
pub const VCKSS_REQUEST_REASON_BATCH_SUMMARY_MISMATCH: u32 = 30;
pub const VCKSS_REQUEST_REASON_WALLSECONDS_VALUE: u32 = 31;
pub const VCKSS_REQUEST_REASON_FALLBACK_ROUTE_MISMATCH: u32 = 32;
pub const VCKSS_REQUEST_REASON_AUTO_RNG: u32 = 33;
pub const VCKSS_REQUEST_REASON_AUTO_ENGINE_COMPRESSED: u32 = 34;
pub const VCKSS_REQUEST_REASON_COMPRESSED_SCIENTIFIC_INELIGIBILITY: u32 = 35;

pub const VCKSS_ROUTE_NOT_APPLICABLE: u32 = 4;
pub const VCKSS_PLAN_APPLICABILITY_NONE: u32 = 0;
pub const VCKSS_PLAN_APPLICABILITY_EXACT: u32 = 1;
pub const VCKSS_PLAN_APPLICABILITY_COMPRESSED: u32 = 2;
pub const VCKSS_PLAN_APPLICABILITY_GENERIC: u32 = 3;
pub const VCKSS_BATCH_SELECTION_NOT_APPLICABLE: u32 = 0;
pub const VCKSS_BATCH_SELECTION_AUTO: u32 = 1;
pub const VCKSS_BATCH_SELECTION_EXPLICIT: u32 = 2;
pub const VCKSS_WALL_STATUS_NOT_REQUESTED: u32 = 0;
pub const VCKSS_WALL_STATUS_UNCALIBRATED: u32 = 1;
pub const VCKSS_WALL_STATUS_WITHIN: u32 = 2;
pub const VCKSS_WALL_STATUS_EXCEEDS: u32 = 3;
pub const VCKSS_STAYER_AUGMENTATION_SCHEMA_V1: u32 = 1;
pub const VCKSS_STAYER_HYBRID_RESULT_SCHEMA_V1: u32 = 1;

pub const VCKSS_EXACT_DIAGNOSTIC_WORKING_FIT: u64 = 1 << 0;
pub const VCKSS_EXACT_DIAGNOSTIC_INVERSE_SQRT: u64 = 1 << 1;
pub const VCKSS_EXACT_DIAGNOSTIC_MAKER: u64 = 1 << 2;
pub const VCKSS_EXACT_DIAGNOSTIC_CONTROL_BASIS: u64 = 1 << 3;
pub const VCKSS_EXACT_DIAGNOSTIC_DELETION_RANK: u64 = 1 << 4;
pub const VCKSS_EXACT_DIAGNOSTIC_FIRM_ZERO_SUM: u64 = 1 << 5;
pub const VCKSS_EXACT_DIAGNOSTIC_FIT_PEAK: u64 = 1 << 6;
pub const VCKSS_EXACT_DIAGNOSTIC_CORRECTION_PEAK: u64 = 1 << 7;
pub const VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING: u64 = 1 << 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssBackendCapabilitiesV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub core_ready_flags: u64,
    pub support_flags: u64,
    pub deterministic_parallelism: u32,
    pub reserved: u32,
}

/// Compositionally complete capability request. Unlike the frozen independent
/// capability bits, this tuple is evaluated as one request and never implies
/// support by combining unrelated flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct VckssBackendRequestCapabilityRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub request_schema: u32,
    pub algorithm: u32,
    pub deletion_mode: u32,
    pub nuisance_mode: u32,
    pub solver_route: u32,
    pub rng_contract: u32,
    pub controls_count: u32,
    pub frequency_use: u32,
    pub reserved: u64,
}

impl Default for VckssBackendRequestCapabilityRequestV1 {
    fn default() -> Self {
        Self {
            abi_version: ABI_VERSION,
            struct_size: u32::try_from(size_of::<Self>()).expect("request capability size"),
            request_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V1,
            algorithm: VCKSS_ALGORITHM_EXACT,
            deletion_mode: VCKSS_DELETION_MATCH,
            nuisance_mode: VCKSS_NUISANCE_JOINT,
            solver_route: VCKSS_ROUTE_AUTO,
            rng_contract: VCKSS_RNG_NONE,
            controls_count: 0,
            frequency_use: VCKSS_REQUEST_FREQUENCY_UNIT,
            reserved: 0,
        }
    }
}

/// Request-specific capability result. Every request field is echoed before a
/// deterministic signature is attached, including unsupported and unknown
/// combinations. `reason_code == 0` is the only supported state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssBackendRequestCapabilityReceiptV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub request_schema: u32,
    pub supported: u32,
    pub reason_code: u32,
    pub profile_code: u32,
    pub algorithm: u32,
    pub deletion_mode: u32,
    pub nuisance_mode: u32,
    pub solver_route: u32,
    pub rng_contract: u32,
    pub controls_count: u32,
    pub frequency_use: u32,
    pub reserved: u32,
    pub request_signature: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct VckssBackendRequestCapabilityRequestV2 {
    pub v1: VckssBackendRequestCapabilityRequestV1,
    pub engine: u32,
    pub batch_mode: u32,
    pub stayers_mode: u32,
    pub target_weight_mode: u32,
    pub deletion_unit_source: u32,
    pub probeorder_supplied: u32,
    pub wallseconds_supplied: u32,
    pub reserved_2: u32,
    pub physical_limit: u64,
}

impl Default for VckssBackendRequestCapabilityRequestV2 {
    fn default() -> Self {
        Self {
            v1: VckssBackendRequestCapabilityRequestV1 {
                struct_size: u32::try_from(size_of::<Self>()).expect("V2 request capability size"),
                request_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V2,
                ..VckssBackendRequestCapabilityRequestV1::default()
            },
            engine: VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            batch_mode: VCKSS_BATCH_MODE_EXPLICIT,
            stayers_mode: VCKSS_STAYERS_MOVERS,
            target_weight_mode: VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT,
            deletion_unit_source: VCKSS_DELETION_SOURCE_CELL_DEFAULT,
            probeorder_supplied: 0,
            wallseconds_supplied: 0,
            reserved_2: 0,
            physical_limit: 50_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssBackendRequestCapabilityReceiptV2 {
    pub v1: VckssBackendRequestCapabilityReceiptV1,
    pub engine: u32,
    pub batch_mode: u32,
    pub stayers_mode: u32,
    pub target_weight_mode: u32,
    pub deletion_unit_source: u32,
    pub probeorder_supplied: u32,
    pub wallseconds_supplied: u32,
    pub reserved_2: u32,
    pub physical_limit: u64,
}

/// Additive planned-command capability tuple. The V2 prefix remains frozen;
/// V3 separates the two independent JLA batch requests and binds the
/// advisory wall value and automatic-CMG setup-fallback permission.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct VckssBackendRequestCapabilityRequestV3 {
    pub v2: VckssBackendRequestCapabilityRequestV2,
    pub leverage_batch_mode: u32,
    pub target_batch_mode: u32,
    pub allow_automatic_cmg_setup_fallback: u32,
    pub reserved_3: u32,
    pub wallseconds: f64,
    pub reserved_4: u64,
}

impl Default for VckssBackendRequestCapabilityRequestV3 {
    fn default() -> Self {
        Self {
            v2: VckssBackendRequestCapabilityRequestV2 {
                v1: VckssBackendRequestCapabilityRequestV1 {
                    struct_size: u32::try_from(size_of::<Self>())
                        .expect("V3 request capability size"),
                    request_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V3,
                    ..VckssBackendRequestCapabilityRequestV1::default()
                },
                batch_mode: VCKSS_BATCH_MODE_AUTO,
                ..VckssBackendRequestCapabilityRequestV2::default()
            },
            leverage_batch_mode: VCKSS_BATCH_MODE_AUTO,
            target_batch_mode: VCKSS_BATCH_MODE_AUTO,
            allow_automatic_cmg_setup_fallback: 1,
            reserved_3: 0,
            wallseconds: 0.0,
            reserved_4: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssBackendRequestCapabilityReceiptV3 {
    pub v2: VckssBackendRequestCapabilityReceiptV2,
    pub leverage_batch_mode: u32,
    pub target_batch_mode: u32,
    pub allow_automatic_cmg_setup_fallback: u32,
    pub reserved_3: u32,
    pub wallseconds: f64,
    pub algorithm_resolution_deferred: u32,
    pub engine_resolution_deferred: u32,
    pub route_resolution_deferred: u32,
    pub leverage_batch_resolution_deferred: u32,
    pub target_batch_resolution_deferred: u32,
    pub wall_advisory_only: u32,
    pub reserved_4: u64,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub rows: u64,
    pub cleanup_abandoned: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestV2 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub rows: u64,
    pub cleanup_abandoned: u32,
    pub reserved: u32,
    pub memory_limit_bytes: u64,
    pub caller_copy_bytes: u64,
}

/// Additive interruptible preparation schema. The embedded V2 options remain
/// an exact 40-byte prefix; its `struct_size` reports the complete outer
/// structure so the existing V2 admission routine can validate that prefix.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestInterruptV1 {
    pub options: VckssEnginePrepareRequestV2,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEnginePrepareRequestInterruptV1 {
    fn default() -> Self {
        Self {
            options: VckssEnginePrepareRequestV2 {
                abi_version: ABI_VERSION,
                struct_size: u32::try_from(size_of::<Self>())
                    .expect("interrupt prepare request size"),
                rows: 0,
                cleanup_abandoned: 0,
                reserved: 0,
                memory_limit_bytes: 0,
                caller_copy_bytes: 0,
            },
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineColumnsV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub rows: u64,
    pub worker: *const f64,
    pub firm: *const f64,
    pub deletion: *const f64,
    pub outcome: *const f64,
    pub frequency: *const f64,
    pub target_weight: *const f64,
}

/// Additive dynamic-column descriptor.  The complete V1 descriptor remains
/// an exact prefix; each control pointer addresses `rows` doubles and is
/// copied synchronously during preparation.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineColumnsV2 {
    pub v1: VckssEngineColumnsV1,
    pub controls: *const *const f64,
    pub controls_count: u32,
    pub reserved_2: u32,
}

/// Additive semantic-order descriptor.  `probe_order` is a final JLA
/// tie-breaker only and is never included among model controls.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineColumnsV3 {
    pub v2: VckssEngineColumnsV2,
    pub probe_order: *const f64,
    pub probeorder_supplied: u32,
    pub reserved_3: u32,
}

/// Additive synchronous augmentation of one prepared mover generation.
/// Every pointer is copied before the call returns and is never retained.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssStayerAugmentationRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub rows: u64,
    pub controls_count: u32,
    pub reserved: u32,
    pub caller_copy_bytes: u64,
}

impl Default for VckssStayerAugmentationRequestV1 {
    fn default() -> Self {
        Self {
            abi_version: ABI_VERSION,
            struct_size: u32::try_from(size_of::<Self>())
                .expect("stayer augmentation request size"),
            rows: 0,
            controls_count: 0,
            reserved: 0,
            caller_copy_bytes: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssStayerAugmentationRequestInterruptV1 {
    pub options: VckssStayerAugmentationRequestV1,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssStayerAugmentationRequestInterruptV1 {
    fn default() -> Self {
        let options = VckssStayerAugmentationRequestV1 {
            struct_size: u32::try_from(size_of::<Self>())
                .expect("interrupt stayer augmentation request size"),
            ..VckssStayerAugmentationRequestV1::default()
        };
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssStayerAugmentationColumnsV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub rows: u64,
    pub firm: *const f64,
    pub worker: *const f64,
    pub outcome: *const f64,
    pub frequency: *const f64,
    pub target_weight: *const f64,
    pub controls: *const *const f64,
    pub controls_count: u32,
    pub reserved_2: u32,
}

/// Additive preparation options for deletion-mode and dynamic-control input.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestV3 {
    pub v2: VckssEnginePrepareRequestV2,
    pub deletion_mode: u32,
    pub controls_count: u32,
    pub reserved_2: u64,
}

impl Default for VckssEnginePrepareRequestV3 {
    fn default() -> Self {
        Self {
            v2: VckssEnginePrepareRequestV2 {
                abi_version: ABI_VERSION,
                struct_size: u32::try_from(size_of::<Self>()).expect("V3 prepare request size"),
                rows: 0,
                cleanup_abandoned: 0,
                reserved: 0,
                memory_limit_bytes: 0,
                caller_copy_bytes: 0,
            },
            deletion_mode: VCKSS_DELETION_MATCH,
            controls_count: 0,
            reserved_2: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestInterruptV2 {
    pub options: VckssEnginePrepareRequestV3,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

/// Additive production preparation schema for the certified implicit
/// worker-firm match key. Older callers retain the V3 semantics exactly.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestV4 {
    pub v3: VckssEnginePrepareRequestV3,
    pub implicit_match: u32,
    pub reserved_3: u32,
}

impl Default for VckssEnginePrepareRequestV4 {
    fn default() -> Self {
        let mut v3 = VckssEnginePrepareRequestV3::default();
        v3.v2.struct_size = u32::try_from(size_of::<Self>()).expect("V4 prepare request size");
        Self {
            v3,
            implicit_match: 0,
            reserved_3: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestInterruptV3 {
    pub options: VckssEnginePrepareRequestV4,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEnginePrepareRequestInterruptV3 {
    fn default() -> Self {
        let mut options = VckssEnginePrepareRequestV4::default();
        options.v3.v2.struct_size =
            u32::try_from(size_of::<Self>()).expect("V3 interrupt prepare request size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

impl Default for VckssEnginePrepareRequestInterruptV2 {
    fn default() -> Self {
        let mut options = VckssEnginePrepareRequestV3::default();
        options.v2.struct_size =
            u32::try_from(size_of::<Self>()).expect("V2 interrupt prepare request size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

// Frozen ABI-1 names retained for source and symbol compatibility.  The
// exported session entry points below are aliases over the engine registry;
// these aliases must never acquire a second lifecycle registry.
pub type VckssPrepareRequestV1 = VckssEnginePrepareRequestV1;
pub type VckssColumnsV1 = VckssEngineColumnsV1;

/// Complete option set currently implemented by the no-control,
/// match-deletion, Counter-V1 JLA engine.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub seed: u64,
    pub probes: u32,
    pub leverage_batch_width: u32,
    pub target_batch_width: u32,
    pub deletion_mode: u32,
    pub rng_contract: u32,
    pub solver_route: u32,
    pub allow_automatic_cmg_setup_fallback: u32,
    pub exact_dimension_limit: u64,
    pub cmg_minimum_dimension: u64,
    pub pcg_tolerance: f64,
    pub maximum_iterations: u32,
    pub residual_replacement_interval: u32,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    pub cmg_terminal_vertices: u64,
    pub cmg_dense_vertex_cap: u64,
    pub cmg_maximum_levels: u64,
    pub cmg_aggregate_cap: u64,
    pub cmg_minimum_reduction: f64,
    pub cmg_jacobi_weight: f64,
    pub cmg_pre_sweeps: u32,
    pub cmg_post_sweeps: u32,
    pub cmg_maximum_edge_complexity: f64,
    pub cmg_maximum_vertex_complexity: f64,
    pub cmg_memory_limit_bytes: u64,
}

impl Default for VckssEngineSolveRequestV1 {
    fn default() -> Self {
        let options = JlaEngineOptions::default();
        Self {
            abi_version: ABI_VERSION,
            struct_size: u32::try_from(size_of::<Self>()).expect("V1 solve request size"),
            seed: options.seed,
            probes: options.probes,
            leverage_batch_width: u32::try_from(options.leverage_batch_width)
                .expect("default leverage width"),
            target_batch_width: u32::try_from(options.target_batch_width)
                .expect("default target width"),
            deletion_mode: VCKSS_DELETION_MATCH,
            rng_contract: VCKSS_RNG_COUNTER_V1,
            solver_route: route_code(options.solver.route),
            allow_automatic_cmg_setup_fallback: u32::from(
                options.solver.allow_automatic_cmg_setup_fallback,
            ),
            exact_dimension_limit: u64::try_from(options.solver.exact_dimension_limit)
                .expect("default exact dimension"),
            cmg_minimum_dimension: u64::try_from(options.solver.cmg_minimum_dimension)
                .expect("default CMG dimension"),
            pcg_tolerance: options.solver.pcg.tolerance,
            maximum_iterations: options.solver.pcg.maximum_iterations,
            residual_replacement_interval: options.solver.pcg.residual_replacement_interval,
            rank_tolerance: options.rank_tolerance,
            block_tolerance: options.block_tolerance,
            cmg_terminal_vertices: u64::try_from(options.solver.cmg.terminal_vertices)
                .expect("default terminal vertices"),
            cmg_dense_vertex_cap: u64::try_from(options.solver.cmg.dense_vertex_cap)
                .expect("default dense cap"),
            cmg_maximum_levels: u64::try_from(options.solver.cmg.maximum_levels)
                .expect("default maximum levels"),
            cmg_aggregate_cap: u64::try_from(options.solver.cmg.aggregate_cap)
                .expect("default aggregate cap"),
            cmg_minimum_reduction: options.solver.cmg.minimum_reduction,
            cmg_jacobi_weight: options.solver.cmg.jacobi_weight,
            cmg_pre_sweeps: options.solver.cmg.pre_sweeps,
            cmg_post_sweeps: options.solver.cmg.post_sweeps,
            cmg_maximum_edge_complexity: options.solver.cmg.maximum_edge_complexity,
            cmg_maximum_vertex_complexity: options.solver.cmg.maximum_vertex_complexity,
            cmg_memory_limit_bytes: options.solver.cmg.memory_limit_bytes,
        }
    }
}

/// Additive estimator-level solve schema.  V1 continues to mean the original
/// match-deletion JLA lifecycle; V2 selects exact/JLA/auto and nuisance
/// semantics without changing any frozen V1 field.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV2 {
    pub v1: VckssEngineSolveRequestV1,
    pub algorithm: u32,
    pub nuisance_mode: u32,
    pub exact_estimator_limit: u64,
    pub blocksize_limit: u64,
}

impl Default for VckssEngineSolveRequestV2 {
    fn default() -> Self {
        Self {
            v1: VckssEngineSolveRequestV1 {
                struct_size: u32::try_from(size_of::<Self>()).expect("V2 solve request size"),
                ..VckssEngineSolveRequestV1::default()
            },
            algorithm: VCKSS_ALGORITHM_JLA,
            nuisance_mode: VCKSS_NUISANCE_JOINT,
            exact_estimator_limit: 500,
            blocksize_limit: 5_000,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV3 {
    pub v2: VckssEngineSolveRequestV2,
    pub engine: u32,
    pub batch_mode: u32,
    pub stayers_mode: u32,
    pub target_weight_mode: u32,
    pub deletion_unit_source: u32,
    pub probeorder_supplied: u32,
    pub wallseconds_supplied: u32,
    pub capability_schema: u32,
    pub capability_profile: u32,
    pub frequency_use: u32,
    pub physical_limit: u64,
    pub request_signature: u64,
    pub reserved_3: u64,
}

/// Additive planned-command solve schema. All V1--V3 bytes and meanings are
/// frozen; automatic phase widths are represented by a zero in the old width
/// fields and their V4 mode is authoritative.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV4 {
    pub v3: VckssEngineSolveRequestV3,
    pub leverage_batch_mode: u32,
    pub target_batch_mode: u32,
    pub wallseconds: f64,
    pub reserved_4: u64,
}

impl Default for VckssEngineSolveRequestV4 {
    fn default() -> Self {
        let mut v3 = VckssEngineSolveRequestV3::default();
        v3.v2.v1.struct_size = u32::try_from(size_of::<Self>()).expect("V4 solve request size");
        v3.v2.v1.solver_route = VCKSS_ROUTE_AUTO;
        v3.v2.v1.leverage_batch_width = 0;
        v3.v2.v1.target_batch_width = 0;
        v3.v2.algorithm = VCKSS_ALGORITHM_AUTO;
        v3.engine = VCKSS_ENGINE_AUTO_OR_UNSPECIFIED;
        v3.batch_mode = VCKSS_BATCH_MODE_AUTO;
        v3.capability_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V3;
        v3.capability_profile = VCKSS_REQUEST_PROFILE_PLANNED_V1;
        Self {
            v3,
            leverage_batch_mode: VCKSS_BATCH_MODE_AUTO,
            target_batch_mode: VCKSS_BATCH_MODE_AUTO,
            wallseconds: 0.0,
            reserved_4: 0,
        }
    }
}

/// Additive production full-CMG solve schema. The complete V4 value remains
/// an exact prefix. V5 is the only solve request that can opt into
/// `CMG_FULL_V2`; older callers retain their existing routes.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV5 {
    pub v4: VckssEngineSolveRequestV4,
    pub threads: u32,
    pub tolerance_supplied: u32,
    pub full_cmg_v2: u32,
    pub reserved_5: u32,
}

impl Default for VckssEngineSolveRequestV5 {
    fn default() -> Self {
        let mut v4 = VckssEngineSolveRequestV4::default();
        v4.v3.v2.v1.struct_size = u32::try_from(size_of::<Self>()).expect("V5 solve request size");
        Self {
            v4,
            threads: 1,
            tolerance_supplied: 0,
            full_cmg_v2: 0,
            reserved_5: 0,
        }
    }
}

impl Default for VckssEngineSolveRequestV3 {
    fn default() -> Self {
        Self {
            v2: VckssEngineSolveRequestV2 {
                v1: VckssEngineSolveRequestV1 {
                    struct_size: u32::try_from(size_of::<Self>()).expect("V3 solve request size"),
                    solver_route: VCKSS_ROUTE_DIAGONAL_PCG,
                    allow_automatic_cmg_setup_fallback: 0,
                    ..VckssEngineSolveRequestV1::default()
                },
                ..VckssEngineSolveRequestV2::default()
            },
            engine: VCKSS_ENGINE_GENERIC,
            batch_mode: VCKSS_BATCH_MODE_EXPLICIT,
            stayers_mode: VCKSS_STAYERS_MOVERS,
            target_weight_mode: VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT,
            deletion_unit_source: VCKSS_DELETION_SOURCE_CELL_DEFAULT,
            probeorder_supplied: 0,
            wallseconds_supplied: 0,
            capability_schema: VCKSS_REQUEST_CAPABILITY_SCHEMA_V2,
            capability_profile: VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1,
            frequency_use: VCKSS_REQUEST_FREQUENCY_UNIT,
            physical_limit: 50_000_000,
            request_signature: 0,
            reserved_3: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV2 {
    pub options: VckssEngineSolveRequestV2,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEngineSolveRequestInterruptV2 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV2::default();
        options.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V2 interrupt solve request size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV3 {
    pub options: VckssEngineSolveRequestV3,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV4 {
    pub options: VckssEngineSolveRequestV4,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV5 {
    pub options: VckssEngineSolveRequestV5,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEngineSolveRequestInterruptV5 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV5::default();
        options.v4.v3.v2.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V5 interrupt solve request size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

impl Default for VckssEngineSolveRequestInterruptV4 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV4::default();
        options.v3.v2.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V4 interrupt solve request size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

impl Default for VckssEngineSolveRequestInterruptV3 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV3::default();
        options.v2.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V3 interrupt solve request size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

/// Synchronous caller-thread polling callback. The callback must not unwind,
/// reenter the engine, retain `context`, or call any host API except its
/// documented polling primitive.
pub type VckssInterruptPollV1 = Option<unsafe extern "C" fn(*mut c_void) -> i32>;

/// Additive interruptible solve schema. The embedded V1 options remain an
/// exact prefix; its `struct_size` reports the complete outer structure.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV1 {
    pub options: VckssEngineSolveRequestV1,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEngineSolveRequestInterruptV1 {
    fn default() -> Self {
        let options = VckssEngineSolveRequestV1 {
            struct_size: u32::try_from(size_of::<Self>()).expect("interrupt solve request size"),
            ..VckssEngineSolveRequestV1::default()
        };
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

#[derive(Debug)]
struct CallbackInterrupt {
    poll: unsafe extern "C" fn(*mut c_void) -> i32,
    context: *mut c_void,
    owner: ThreadId,
    interval: u32,
    remaining: u32,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl CallbackInterrupt {
    fn new(
        poll: VckssInterruptPollV1,
        context: *mut c_void,
        interval: u32,
        reserved: u32,
        label: &'static str,
    ) -> Result<Option<Self>> {
        if reserved != 0 {
            return Err(abi_error(format!(
                "reserved interrupt {label} fields must be zero"
            )));
        }
        match poll {
            None => {
                if !context.is_null() || interval != 0 {
                    return Err(BackendError::invalid(
                        "engine_interrupt",
                        "a null interrupt callback requires null context and zero interval",
                    ));
                }
                Ok(None)
            }
            Some(poll) => {
                if interval == 0 {
                    return Err(BackendError::invalid(
                        "engine_interrupt",
                        "a nonnull interrupt callback requires a positive checkpoint interval",
                    ));
                }
                Ok(Some(Self {
                    poll,
                    context,
                    owner: thread::current().id(),
                    interval,
                    remaining: 0,
                    _not_send_or_sync: PhantomData,
                }))
            }
        }
    }
}

impl InterruptCheck for CallbackInterrupt {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if thread::current().id() != self.owner {
            return Err(BackendError::invariant(
                "engine_interrupt",
                "interrupt callback would run off its owning caller thread",
            ));
        }
        if self.remaining != 0 {
            self.remaining -= 1;
            return Ok(());
        }
        self.remaining = self.interval - 1;
        // SAFETY: the ABI caller guarantees that the synchronous callback and
        // context remain valid for this solve. Neither value is retained.
        match unsafe { (self.poll)(self.context) } {
            VCKSS_INTERRUPT_CONTINUE => Ok(()),
            VCKSS_INTERRUPT_USER_BREAK => Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "user requested interruption",
            )),
            value => Err(BackendError::new(
                ErrorCode::InternalInvariantFailed,
                "engine_interrupt",
                format!("interrupt callback returned unsupported status {value}"),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEnginePreparationReceiptV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub input_rows: u64,
    pub retained_rows: u64,
    pub workers: u64,
    pub firms: u64,
    pub cells: u64,
    pub deletion_units: u64,
    pub target_strata: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEnginePreparationReceiptV2 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub input_rows: u64,
    pub retained_rows: u64,
    pub workers: u64,
    pub firms: u64,
    pub cells: u64,
    pub deletion_units: u64,
    pub target_strata: u64,
    pub memory_limit_bytes: u64,
    pub caller_copy_bytes: u64,
    pub preparation_peak_forecast_bytes: u64,
    pub prepared_resident_bytes: u64,
    pub graph_input_rows: u64,
    pub graph_retained_rows: u64,
    pub graph_input_physical_mass: u64,
    pub graph_retained_physical_mass: u64,
    pub graph_initial_components: u64,
    pub graph_maximum_components: u64,
    pub graph_initial_component_rows: u64,
    pub graph_mover_input_rows: u64,
    pub graph_initial_deletion_edges: u64,
    pub graph_retained_deletion_edges: u64,
    pub graph_insufficient_workers_removed: u64,
    pub graph_articulation_workers_removed: u64,
    pub graph_bridge_units_removed: u64,
    pub graph_bridge_rows_removed: u64,
    pub graph_degree_iterations: u64,
    pub graph_articulation_iterations: u64,
    pub graph_bridge_iterations: u64,
    pub graph_fixed_point_iterations: u64,
}

/// Additive public-command preparation receipt. The complete V2 value is an
/// exact prefix so older callers and frozen layouts remain unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEnginePreparationReceiptV3 {
    pub v2: VckssEnginePreparationReceiptV2,
    pub target_weight_sum: f64,
}

/// Native preparation facts that were not present in the frozen V1--V3
/// receipts. These values come from the retained native problem, not from a
/// caller-side echo.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEnginePreparationReceiptV4 {
    pub v3: VckssEnginePreparationReceiptV3,
    pub controls_count: u32,
    pub deletion_mode: u32,
}

/// Source and memory identity for the retained mover-plus-stayer preparation.
/// It is available both before solve and after a successful solve.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssStayerAugmentationReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub mover_stored_rows: u64,
    pub stayer_stored_rows: u64,
    pub combined_stored_rows: u64,
    pub mover_physical_mass: u64,
    pub stayer_physical_mass: u64,
    pub combined_physical_mass: u64,
    pub mover_workers: u64,
    pub stayer_workers: u64,
    pub combined_workers: u64,
    pub firms: u64,
    pub mover_deletion_units: u64,
    pub stayer_deletion_units: u64,
    pub combined_deletion_units: u64,
    pub mover_target_mass: f64,
    pub stayer_target_mass: f64,
    pub combined_target_mass: f64,
    pub topology_checksum: u64,
    pub memory_limit_bytes: u64,
    pub caller_copy_bytes: u64,
    pub augmentation_peak_forecast_bytes: u64,
    pub augmented_resident_bytes: u64,
    pub total_prepared_resident_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssComponentVectorV1 {
    pub worker: f64,
    pub firm: f64,
    pub covariance: f64,
    pub total: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineResultV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub plugin: VckssComponentVectorV1,
    pub correction: VckssComponentVectorV1,
    pub corrected: VckssComponentVectorV1,
    /// Numerical probe dispersion only; not an econometric standard error.
    pub numerical_mcse: VckssComponentVectorV1,
}

/// Secondary exact mixed-deletion result. The ordinary `VckssEngineResultV1`
/// remains the mover-only headline and therefore preserves `e(sample)` and all
/// frozen V4/V7 reconciliation semantics.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssStayerHybridResultV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub plugin: VckssComponentVectorV1,
    pub correction: VckssComponentVectorV1,
    pub corrected: VckssComponentVectorV1,
    pub mover_correction: VckssComponentVectorV1,
    pub stayer_correction: VckssComponentVectorV1,
    pub weighted_rss: f64,
    pub parameters: u64,
    pub full_parameters: u64,
    pub correction_parameters: u64,
    pub deletion_units: u64,
    pub max_leverage: f64,
    pub information_rcond: f64,
    pub inverse_relres: f64,
    pub inverse_original_relres: f64,
    pub inverse_sqrt_relres: f64,
    pub maker_relres: f64,
    pub full_fit_relres: f64,
    pub working_fit_relres: f64,
    pub fit_residual_tolerance: f64,
    pub control_basis_relres: f64,
    pub control_basis_forward_error: f64,
    pub deletion_rank_gap: f64,
    pub firm_zero_sum_residual: f64,
    pub peak_forecast_bytes: u64,
    pub fit_peak_forecast_bytes: u64,
    pub correction_peak_forecast_bytes: u64,
    pub topology_checksum: u64,
    pub accounting_residual: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub seed: u64,
    pub probes_requested: u32,
    pub leverage_probes_accepted: u32,
    pub target_probes_accepted: u32,
    pub solver_requested: u32,
    pub solver_selected: u32,
    pub solver_fallback: u32,
    pub solver_fallback_error: i32,
    pub solver_dimension: u64,
    pub leverage_batch_width: u64,
    pub target_batch_width: u64,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    pub full_residual_tolerance: f64,
    pub full_fit_route: u32,
    pub full_fit_iterations: u32,
    /// JLA reports the reduced-system residual. Exact has no reduced solve;
    /// its V3/V4 export mirrors the stronger complete original-system fit
    /// certificate here to keep this frozen finite-valued prefix truthful.
    pub full_fit_reduced_residual: f64,
    pub full_fit_complete_residual: f64,
    pub full_fit_zero_rhs: u32,
    pub reserved_1: u32,
    pub leverage_rhs_count: u64,
    pub target_rhs_count: u64,
    /// JLA reports the maximum reduced-system residual. Exact has no reduced
    /// solve and mirrors its maximum complete fit certificate in this frozen
    /// field; callers can distinguish that meaning through `full_fit_route`
    /// and, on V4, `algorithm_selected`.
    pub max_reduced_residual: f64,
    pub max_complete_residual: f64,
    pub max_leverage: f64,
    pub max_reciprocal_residual: f64,
    pub accounting_residual: f64,
    pub topology_checksum: u64,
    pub cmg_levels: u64,
    pub cmg_fine_vertices: u64,
    pub cmg_fine_edges: u64,
    pub cmg_terminal_vertices: u64,
    pub cmg_edge_complexity: f64,
    pub cmg_vertex_complexity: f64,
    pub cmg_structural_bytes: u64,
    pub cmg_workspace_bytes: u64,
    pub cmg_dense_factor_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV2 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub seed: u64,
    pub probes_requested: u32,
    pub leverage_probes_accepted: u32,
    pub target_probes_accepted: u32,
    pub solver_requested: u32,
    pub solver_selected: u32,
    pub solver_fallback: u32,
    pub solver_fallback_error: i32,
    pub solver_dimension: u64,
    pub leverage_batch_width: u64,
    pub target_batch_width: u64,
    pub rank_tolerance: f64,
    pub block_tolerance: f64,
    pub full_residual_tolerance: f64,
    pub full_fit_route: u32,
    pub full_fit_iterations: u32,
    /// See [`VckssEngineDetailedReceiptV1::full_fit_reduced_residual`].
    pub full_fit_reduced_residual: f64,
    pub full_fit_complete_residual: f64,
    pub full_fit_zero_rhs: u32,
    pub reserved_1: u32,
    pub leverage_rhs_count: u64,
    pub target_rhs_count: u64,
    /// See [`VckssEngineDetailedReceiptV1::max_reduced_residual`].
    pub max_reduced_residual: f64,
    pub max_complete_residual: f64,
    pub max_leverage: f64,
    pub max_reciprocal_residual: f64,
    pub accounting_residual: f64,
    pub topology_checksum: u64,
    pub cmg_levels: u64,
    pub cmg_fine_vertices: u64,
    pub cmg_fine_edges: u64,
    pub cmg_terminal_vertices: u64,
    pub cmg_edge_complexity: f64,
    pub cmg_vertex_complexity: f64,
    pub cmg_structural_bytes: u64,
    pub cmg_workspace_bytes: u64,
    pub cmg_dense_factor_bytes: u64,
    pub full_fit_weighted_rss: f64,
    pub memory_limit_bytes: u64,
    pub caller_copy_bytes: u64,
    pub preparation_peak_forecast_bytes: u64,
    pub prepared_resident_bytes: u64,
    pub solver_setup_forecast_bytes: u64,
    pub leverage_phase_forecast_bytes: u64,
    pub target_phase_forecast_bytes: u64,
    pub result_forecast_bytes: u64,
    pub solve_peak_forecast_bytes: u64,
    pub command_peak_forecast_bytes: u64,
}

/// Additive public-command result receipt. The complete V2 value is an exact
/// prefix; V3 adds the explicit RNG contract and caller-copy accounting for
/// the lossless per-RHS export.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV3 {
    pub v2: VckssEngineDetailedReceiptV2,
    pub rng_contract: u32,
    pub reserved_2: u32,
    pub rhs_receipt_rows: u64,
    pub caller_result_copy_bytes: u64,
}

/// Additive estimator-level receipt.  The V3 prefix remains byte-for-byte
/// stable.  Exact results use zero probe/RHS fields in that prefix and expose
/// their dense-estimator dimensions and certificates here.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV4 {
    pub v3: VckssEngineDetailedReceiptV3,
    pub algorithm_requested: u32,
    pub algorithm_selected: u32,
    pub deletion_mode: u32,
    pub nuisance_mode: u32,
    pub parameters: u64,
    pub full_parameters: u64,
    pub correction_parameters: u64,
    pub information_rcond: f64,
    pub inverse_relres: f64,
    pub exact_peak_forecast_bytes: u64,
}

/// Additive exact diagnostic receipt. The complete V4 value is an exact
/// prefix. Applicability is explicit so an inapplicable numerical zero can
/// never be mistaken for a native certificate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV5 {
    pub v4: VckssEngineDetailedReceiptV4,
    pub applicability_flags: u64,
    pub working_fit_complete_residual: f64,
    pub inverse_sqrt_relres: f64,
    pub maker_relres: f64,
    pub control_basis_relres: f64,
    pub control_basis_forward_error: f64,
    pub deletion_rank_gap: f64,
    pub firm_zero_sum_residual: f64,
    pub fit_peak_forecast_bytes: u64,
    pub correction_peak_forecast_bytes: u64,
    pub actual_accounting_residual: f64,
}

/// Additive generic-JLA diagnostic receipt. The V5 prefix remains frozen and
/// retains its exact-only diagnostic meanings; generic certificates live only
/// in this extension and are guarded by `generic_applicability_flags`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV6 {
    pub v5: VckssEngineDetailedReceiptV5,
    pub engine_requested: u32,
    pub engine_selected: u32,
    pub generic_applicability_flags: u64,
    pub controls_count: u32,
    pub rhs_receipt_schema: u32,
    pub control_projection_rhs_count: u64,
    pub control_rank_rcond: f64,
    pub control_rank_smallest_generalized_eigenvalue_lower: f64,
    pub control_rank_largest_generalized_eigenvalue_upper: f64,
    pub control_rank_projection_error_bound: f64,
    pub control_rank_normalization_error_bound: f64,
    pub control_rank_fe_information_eigenvalue_lower_bound: f64,
    pub control_rank_maximum_projection_residual: f64,
    pub control_rank_effective_tolerance: f64,
    pub control_rank_projection_pcg_tolerance: f64,
    pub control_rank_projection_residual_gate: f64,
    pub generic_control_basis_relres: f64,
    pub generic_control_basis_forward_error: f64,
    pub generic_control_schur_rcond: f64,
    pub generic_control_schur_relres: f64,
    pub generic_deletion_rank_gap: f64,
    pub full_joint_fit_complete_residual: f64,
    pub generic_working_fit_complete_residual: f64,
    pub generic_maker_relres: f64,
    pub canonicalization_peak_forecast_bytes: u64,
    pub generic_fit_peak_forecast_bytes: u64,
    pub geometry_peak_forecast_bytes: u64,
    pub generic_leverage_peak_forecast_bytes: u64,
    pub generic_target_peak_forecast_bytes: u64,
    pub maker_peak_forecast_bytes: u64,
    pub generic_result_forecast_bytes: u64,
    pub generic_peak_forecast_bytes: u64,
    /// Combined simultaneously-live V2 export workspace: the native 96-byte
    /// row array plus the caller's fifteen-double matrix, 216 bytes per row.
    pub rhs_v2_caller_copy_bytes: u64,
    pub capability_schema: u32,
    pub capability_profile: u32,
    pub batch_mode: u32,
    pub stayers_mode: u32,
    pub target_weight_mode: u32,
    pub deletion_unit_source: u32,
    pub probeorder_supplied: u32,
    pub wallseconds_supplied: u32,
    pub frequency_use: u32,
    pub reserved_6: u32,
    pub physical_limit: u64,
    pub request_signature: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEstimatorResolutionReceiptV1 {
    pub algorithm_schema: u32,
    pub algorithm_requested: u32,
    pub algorithm_selected: u32,
    pub algorithm_reason: u32,
    pub engine_schema: u32,
    pub engine_requested: u32,
    pub engine_selected: u32,
    pub engine_reason: u32,
    pub compressed_eligibility: u32,
    pub resolved_before_rng: u32,
    pub opportunistic_engine_fallback_allowed: u32,
    pub reserved: u32,
    pub identified_complexity: u64,
    pub exact_limit: u64,
    pub rng_draws_before_resolution: u64,
    pub counter_atoms_before_resolution: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssSolverExecutionReceiptV1 {
    pub schema_version: u32,
    pub requested_route: u32,
    pub selected_route: u32,
    pub fallback_used: u32,
    pub fallback_error: i32,
    pub full_setup_complete: u32,
    pub fe_setup_complete: u32,
    pub fe_hierarchy_reused: u32,
    pub plan_frozen_before_rng: u32,
    pub auto_route_contract: u32,
    pub threads_requested: u32,
    pub threads_used: u32,
    pub parallel_regions: u32,
    pub applicability: u32,
    pub reserved_1: u32,
    pub reserved_2: u32,
    pub planned_rhs: u64,
    pub auto_firm_threshold: u64,
    pub auto_rhs_threshold: u64,
    pub full_solver_dimension: u64,
    pub fe_solver_dimension: u64,
    pub logical_atoms_before_plan_freeze: u64,
    pub unique_words_before_plan_freeze: u64,
    pub physical_trials_before_plan_freeze: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssBatchPhasePlanReceiptV1 {
    pub request_mode: u32,
    pub selection_reason: u32,
    pub applicability: u32,
    pub reserved: u32,
    pub requested_width: u64,
    pub selected_width: u64,
    pub probe_width_cap: u64,
    pub declared_threads: u64,
    pub thread_width_cap: u64,
    pub route_width_cap: u64,
    pub effective_width_cap: u64,
    pub hard_memory_bytes: u64,
    pub width_one_forecast_bytes: u64,
    pub selected_forecast_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssBatchExecutionReceiptV1 {
    pub schema_version: u32,
    pub deterministic: u32,
    pub width_invariance_required: u32,
    pub arithmetic_contract: u32,
    pub whole_command_admitted: u32,
    pub applicability: u32,
    pub non_batched_peak_bytes: u64,
    pub selected_command_peak_bytes: u64,
    pub leverage: VckssBatchPhasePlanReceiptV1,
    pub target: VckssBatchPhasePlanReceiptV1,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssWallExecutionReceiptV1 {
    pub schema_version: u32,
    pub model_code: u32,
    pub status: u32,
    pub routing_effect: u32,
    pub requested_applicable: u32,
    pub forecast_applicable: u32,
    pub advisory_applicable: u32,
    pub margin_applicable: u32,
    pub requested_seconds: f64,
    pub forecast_seconds: f64,
    pub advisory_seconds: f64,
    pub advisory_margin_fraction: f64,
    pub preparation_work: u64,
    pub engine_setup_work: u64,
    pub full_fit_work: u64,
    pub leverage_work: u64,
    pub target_work: u64,
    pub result_export_work: u64,
    pub total_work: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssCounterPhaseExecutionReceiptV1 {
    pub planned_logical_atoms: u64,
    pub actual_logical_atoms: u64,
    pub planned_unique_packed_words: u64,
    pub actual_unique_packed_words: u64,
    pub planned_physical_trials: u64,
    pub actual_physical_trials: u64,
    pub planned_generator_work: u64,
    pub actual_generator_work: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssCounterExecutionReceiptV1 {
    pub schema_version: u32,
    pub rng_contract: u32,
    pub generator_work_applicable: u32,
    pub completed: u32,
    pub leverage: VckssCounterPhaseExecutionReceiptV1,
    pub target: VckssCounterPhaseExecutionReceiptV1,
    pub total: VckssCounterPhaseExecutionReceiptV1,
    pub logical_atoms_before_plan_freeze: u64,
    pub unique_words_before_plan_freeze: u64,
    pub physical_trials_before_plan_freeze: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssExecutionMemoryReceiptV1 {
    pub schema_version: u32,
    pub applicability: u32,
    pub peak_phase: u32,
    pub reserved: u32,
    pub hard_limit_bytes: u64,
    pub prepared_persistent_bytes: u64,
    pub setup_peak_bytes: u64,
    pub fit_peak_bytes: u64,
    pub correction_peak_bytes: u64,
    pub leverage_peak_bytes: u64,
    pub target_peak_bytes: u64,
    pub result_peak_bytes: u64,
    pub non_batched_peak_bytes: u64,
    pub command_peak_bytes: u64,
    pub shared_cmg_persistent_bytes: u64,
    pub full_control_block_persistent_bytes: u64,
    pub setup_transient_bytes: u64,
    pub cmg_preconditioner_workspace_bytes: u64,
    pub cmg_aggregated_cell_capacity_bytes: u64,
    pub cmg_group_index_bytes: u64,
    pub cmg_hybrid_graph_bytes: u64,
    pub retained_nq_bytes: u64,
    pub retained_q2_bytes: u64,
}

/// Frozen pre-RNG execution plan. This receipt is stored with the solved
/// generation and is never synthesized from post-hoc timing or iteration
/// behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssExecutionPlanReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub applicability_flags: u64,
    pub contract_flags: u64,
    pub request_signature: u64,
    pub resolution: VckssEstimatorResolutionReceiptV1,
    pub solver: VckssSolverExecutionReceiptV1,
    pub batch: VckssBatchExecutionReceiptV1,
    pub wall: VckssWallExecutionReceiptV1,
    pub counter: VckssCounterExecutionReceiptV1,
    pub memory: VckssExecutionMemoryReceiptV1,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineDetailedReceiptV7 {
    pub v6: VckssEngineDetailedReceiptV6,
    pub execution: VckssExecutionPlanReceiptV1,
}

/// Additive diagnostic-only performance receipt. It is deliberately separate
/// from the frozen numerical and pre-RNG execution-plan receipts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEnginePerformanceReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub applicability_flags: u64,
    pub algorithm_selected: u32,
    pub engine_selected: u32,
    pub ingest_ns: u64,
    pub canonicalize_ns: u64,
    pub graph_ns: u64,
    pub compress_ns: u64,
    pub plan_ns: u64,
    pub stayer_augmentation_ns: u64,
    pub solve_ns: u64,
    pub native_total_ns: u64,
}

/// Additive source-bound receipt for the production direct full-CMG route.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct VckssFullCmgReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub backend_identity: u32,
    pub platform_os: u32,
    pub platform_arch: u32,
    pub batch_strategy_mask: u32,
    pub cmg_source_commit: [u8; 40],
    pub threads_requested: u32,
    pub threads_used: u32,
    pub maximum_concurrency: u64,
    pub vertices: u64,
    pub edges: u64,
    pub hierarchy_levels: u64,
    pub terminal_vertices: u64,
    pub graph_copy_bytes: u64,
    pub hierarchy_bytes: u64,
    pub plan_bytes: u64,
    pub workspace_bytes_each: u64,
    pub workspace_pool_bytes: u64,
    pub admitted_peak_bytes: u64,
    pub fit_effective_tolerance: f64,
    pub probe_effective_tolerance: f64,
    pub fit_initial_inner_tolerance: f64,
    pub probe_initial_inner_tolerance: f64,
    pub refinement_attempts: u64,
    pub refined_columns: u64,
    pub batch_calls: u64,
    pub rhs_count: u64,
    pub serial_batches: u64,
    pub planned_batches: u64,
    pub across_rhs_batches: u64,
    pub total_iterations: u64,
    pub total_operator_applications: u64,
    pub total_preconditioner_applications: u64,
    pub maximum_reduced_residual: f64,
    pub maximum_complete_residual: f64,
    pub graph_ns: u64,
    pub hierarchy_plan_ns: u64,
    pub rhs_ns: u64,
    pub solve_ns: u64,
    pub extraction_ns: u64,
    pub preparation_peak_bytes: u64,
    pub prepared_persistent_bytes: u64,
    pub non_cmg_command_peak_bytes: u64,
    pub pre_rng_forecast_bytes: u64,
    pub actual_retained_bytes: u64,
    pub allocator_allowance_bytes: u64,
    pub maximum_batch_rhs: u64,
    pub workspace_count: u64,
}

// Compile-time ABI fences complement the cross-language layout tests. The
// array lengths fail to type-check if a field reorders, padding changes, or a
// supposedly prefix-compatible receipt grows in place.
const _: [(); 48] = [(); size_of::<VckssBackendRequestCapabilityRequestV1>()];
const _: [(); 64] = [(); size_of::<VckssBackendRequestCapabilityReceiptV1>()];
const _: [(); 88] = [(); size_of::<VckssBackendRequestCapabilityRequestV2>()];
const _: [(); 104] = [(); size_of::<VckssBackendRequestCapabilityReceiptV2>()];
const _: [(); 120] = [(); size_of::<VckssBackendRequestCapabilityRequestV3>()];
const _: [(); 160] = [(); size_of::<VckssBackendRequestCapabilityReceiptV3>()];
const _: [(); 48] = [(); std::mem::offset_of!(VckssBackendRequestCapabilityRequestV2, engine)];
const _: [(); 64] = [(); std::mem::offset_of!(VckssBackendRequestCapabilityReceiptV2, engine)];
const _: [(); 256] = [(); size_of::<VckssEnginePreparationReceiptV3>()];
const _: [(); 264] = [(); size_of::<VckssEnginePreparationReceiptV4>()];
const _: [(); 256] = [(); std::mem::offset_of!(VckssEnginePreparationReceiptV4, controls_count)];
const _: [(); 260] = [(); std::mem::offset_of!(VckssEnginePreparationReceiptV4, deletion_mode)];
const _: [(); 448] = [(); size_of::<VckssEngineDetailedReceiptV4>()];
const _: [(); 536] = [(); size_of::<VckssEngineDetailedReceiptV5>()];
const _: [(); 840] = [(); size_of::<VckssEngineDetailedReceiptV6>()];
const _: [(); 1000] = [(); size_of::<VckssExecutionPlanReceiptV1>()];
const _: [(); 1840] = [(); size_of::<VckssEngineDetailedReceiptV7>()];
const _: [(); 96] = [(); size_of::<VckssEnginePerformanceReceiptV1>()];
const _: [(); 400] = [(); size_of::<VckssFullCmgReceiptV1>()];
const _: [(); 448] = [(); std::mem::offset_of!(VckssEngineDetailedReceiptV5, applicability_flags)];
const _: [(); 528] =
    [(); std::mem::offset_of!(VckssEngineDetailedReceiptV5, actual_accounting_residual)];
const _: [(); 536] = [(); std::mem::offset_of!(VckssEngineDetailedReceiptV6, engine_requested)];
const _: [(); 776] =
    [(); std::mem::offset_of!(VckssEngineDetailedReceiptV6, rhs_v2_caller_copy_bytes)];
const _: [(); 784] = [(); std::mem::offset_of!(VckssEngineDetailedReceiptV6, capability_schema)];
const _: [(); 832] = [(); std::mem::offset_of!(VckssEngineDetailedReceiptV6, request_signature)];
const _: [(); 264] = [(); size_of::<VckssEngineSolveRequestV3>()];
const _: [(); 288] = [(); size_of::<VckssEngineSolveRequestInterruptV3>()];
const _: [(); 288] = [(); size_of::<VckssEngineSolveRequestV4>()];
const _: [(); 312] = [(); size_of::<VckssEngineSolveRequestInterruptV4>()];
const _: [(); 304] = [(); size_of::<VckssEngineSolveRequestV5>()];
const _: [(); 328] = [(); size_of::<VckssEngineSolveRequestInterruptV5>()];
const _: [(); 200] = [(); std::mem::offset_of!(VckssEngineSolveRequestV3, engine)];
const _: [(); 264] = [(); std::mem::offset_of!(VckssEngineSolveRequestInterruptV3, interrupt_poll)];
const _: [(); 264] = [(); std::mem::offset_of!(VckssEngineSolveRequestV4, leverage_batch_mode)];
const _: [(); 288] = [(); std::mem::offset_of!(VckssEngineSolveRequestInterruptV4, interrupt_poll)];
const _: [(); 288] = [(); std::mem::offset_of!(VckssEngineSolveRequestV5, threads)];
const _: [(); 304] = [(); std::mem::offset_of!(VckssEngineSolveRequestInterruptV5, interrupt_poll)];
const _: [(); 840] = [(); std::mem::offset_of!(VckssEngineDetailedReceiptV7, execution)];
const _: [(); 96] = [(); size_of::<VckssEngineRhsReceiptV2>()];
const _: [(); 48] = [(); std::mem::offset_of!(VckssEngineRhsReceiptV2, status)];
const _: [(); 32] = [(); size_of::<VckssStayerAugmentationRequestV1>()];
const _: [(); 56] = [(); size_of::<VckssStayerAugmentationRequestInterruptV1>()];
const _: [(); 72] = [(); size_of::<VckssStayerAugmentationColumnsV1>()];
const _: [(); 192] = [(); size_of::<VckssStayerAugmentationReceiptV1>()];
const _: [(); 360] = [(); size_of::<VckssStayerHybridResultV1>()];

/// Lossless native receipt for one logical original-system right-hand side.
/// Rows are exported in full-fit, leverage-probe, then target worker/firm
/// order. `probe == -1` is the full-fit sentinel; all other probes are
/// zero-based.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineRhsReceiptV1 {
    pub phase: u32,
    pub side: u32,
    pub probe: i64,
    pub route: u32,
    pub iterations: u32,
    pub zero_rhs: u32,
    pub reserved: u32,
    pub reduced_residual: f64,
    pub complete_residual: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineRhsReceiptV2 {
    pub v1: VckssEngineRhsReceiptV1,
    pub status: u32,
    pub residual_replacements: u32,
    pub operator_applications: u64,
    pub preconditioner_applications: u64,
    pub full_residual_tolerance: f64,
    pub residual_space: u32,
    pub reserved_2: u32,
    pub solver_dimension: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEngineSnapshotV1 {
    pub struct_size: u32,
    pub state: u32,
    pub generation: u64,
    pub last_released_generation: u64,
}

pub type VckssPreparationReceiptV1 = VckssEnginePreparationReceiptV1;
pub type VckssSessionSnapshotV1 = VckssEngineSnapshotV1;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum EngineEstimate {
    Jla(JlaEngineResult),
    GenericJla(GenericJlaResult),
    Exact(ExactEstimatorResult),
}

impl EngineEstimate {
    fn plugin(&self) -> VarianceComponents {
        match self {
            Self::Jla(value) => value.plugin,
            Self::GenericJla(value) => value.plugin,
            Self::Exact(value) => value.plugin,
        }
    }

    fn correction(&self) -> VarianceComponents {
        match self {
            Self::Jla(value) => value.correction,
            Self::GenericJla(value) => value.correction,
            Self::Exact(value) => value.correction,
        }
    }

    fn corrected(&self) -> VarianceComponents {
        match self {
            Self::Jla(value) => value.corrected,
            Self::GenericJla(value) => value.corrected,
            Self::Exact(value) => value.corrected,
        }
    }

    fn numerical_mcse(&self) -> NumericalMcse {
        match self {
            Self::Jla(value) => value.numerical_mcse,
            Self::GenericJla(value) => NumericalMcse {
                worker: value.numerical_mcse.worker,
                firm: value.numerical_mcse.firm,
                covariance: value.numerical_mcse.covariance,
                total: value.numerical_mcse.total,
            },
            Self::Exact(_) => NumericalMcse::default(),
        }
    }

    fn weighted_rss(&self) -> f64 {
        match self {
            Self::Jla(value) => value.weighted_rss,
            Self::GenericJla(value) => value.weighted_rss,
            Self::Exact(value) => value.weighted_rss,
        }
    }

    fn as_jla(&self) -> Result<&JlaEngineResult> {
        match self {
            Self::Jla(value) => Ok(value),
            Self::GenericJla(_) | Self::Exact(_) => Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_receipt",
                "the frozen JLA detailed receipt is unavailable for an exact estimator result",
            )),
        }
    }
}

#[derive(Debug)]
struct EngineSolved {
    result: EngineEstimate,
    algorithm_requested: u32,
    algorithm_selected: u32,
    engine_requested: u32,
    engine_selected: u32,
    seed: u64,
    probes: u32,
    leverage_batch_width: u32,
    target_batch_width: u32,
    nuisance: NuisanceMode,
    deletion: DeletionMode,
    rank_tolerance: f64,
    block_tolerance: f64,
    controls_count: u32,
    capability_schema: u32,
    capability_profile: u32,
    batch_mode: u32,
    stayers_mode: u32,
    target_weight_mode: u32,
    deletion_unit_source: u32,
    probeorder_supplied: u32,
    wallseconds_supplied: u32,
    frequency_use: u32,
    physical_limit: u64,
    request_signature: u64,
    execution_plan: Option<VckssExecutionPlanReceiptV1>,
    full_cmg: Option<FullCmgReceipt>,
    preparation: PreparationReceipt,
    retained: Arc<Vec<bool>>,
    stayer_augmentation: Option<EngineStayerAugmentationReceipt>,
    stayer_hybrid: Option<ExactStayerHybridResult>,
    performance: NativePhaseTimings,
}

#[derive(Clone, Copy, Debug)]
struct EngineStayerAugmentationReceipt {
    core: StayerAugmentationReceipt,
    memory: StayerAugmentationMemoryReceipt,
}

#[derive(Debug)]
struct EngineState {
    registry: ContextRegistry<PreparedProblemWithMask, EngineSolved>,
}

impl EngineState {
    const fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
        }
    }
}

static ENGINE: OnceLock<Mutex<EngineState>> = OnceLock::new();

thread_local! {
    static ENGINE_LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("OK").expect("literal CString"));
}

fn engine() -> &'static Mutex<EngineState> {
    ENGINE.get_or_init(|| Mutex::new(EngineState::new()))
}

fn ffi_status(function: impl FnOnce() -> Result<()>) -> i32 {
    match catch_unwind(AssertUnwindSafe(function)) {
        Ok(Ok(())) => {
            store_error_text("OK");
            ErrorCode::Ok as i32
        }
        Ok(Err(error)) => {
            store_error_text(&error.to_string());
            error.code as i32
        }
        Err(_) => {
            let error = BackendError::new(
                ErrorCode::Panic,
                "engine_ffi",
                "Rust panic was contained at the numerical engine ABI boundary",
            );
            store_error_text(&error.to_string());
            ErrorCode::Panic as i32
        }
    }
}

#[no_mangle]
pub extern "C" fn vckss_rust_backend_capabilities_v1(
    output: *mut VckssBackendCapabilitiesV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssBackendCapabilitiesV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "backend capabilities",
        )?;
        write_output(
            output,
            VckssBackendCapabilitiesV1 {
                struct_size: struct_size_u32::<VckssBackendCapabilitiesV1>()?,
                abi_version: ABI_VERSION,
                core_ready_flags: VCKSS_BACKEND_CAPABILITIES_V1_CORE_READY_FLAGS,
                support_flags: VCKSS_BACKEND_CAPABILITIES_V1_SUPPORT_FLAGS,
                deterministic_parallelism: 1,
                reserved: 0,
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_backend_request_capability_v1(
    request: *const VckssBackendRequestCapabilityRequestV1,
    output: *mut VckssBackendRequestCapabilityReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssBackendRequestCapabilityReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "request capability receipt",
        )?;
        let request = copy_request_struct(request, "request capability")?;
        require_abi(request.abi_version)?;
        if request.reserved != 0 {
            return Err(abi_error("reserved request capability fields must be zero"));
        }
        let (reason_code, profile_code) = request_capability_classification(request);
        write_output(
            output,
            VckssBackendRequestCapabilityReceiptV1 {
                struct_size: struct_size_u32::<VckssBackendRequestCapabilityReceiptV1>()?,
                abi_version: ABI_VERSION,
                request_schema: request.request_schema,
                supported: u32::from(reason_code == VCKSS_REQUEST_REASON_SUPPORTED),
                reason_code,
                profile_code,
                algorithm: request.algorithm,
                deletion_mode: request.deletion_mode,
                nuisance_mode: request.nuisance_mode,
                solver_route: request.solver_route,
                rng_contract: request.rng_contract,
                controls_count: request.controls_count,
                frequency_use: request.frequency_use,
                reserved: 0,
                request_signature: request_capability_signature(request),
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_backend_request_capability_v2(
    request: *const VckssBackendRequestCapabilityRequestV2,
    output: *mut VckssBackendRequestCapabilityReceiptV2,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssBackendRequestCapabilityReceiptV2>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V2 request capability receipt",
        )?;
        let request = copy_request_struct(request, "V2 request capability")?;
        require_abi(request.v1.abi_version)?;
        if request.v1.struct_size < struct_size_u32::<VckssBackendRequestCapabilityRequestV2>()? {
            return Err(abi_error(
                "V2 request capability reports a short structure size",
            ));
        }
        if request.v1.reserved != 0 || request.reserved_2 != 0 {
            return Err(abi_error(
                "reserved V2 request capability fields must be zero",
            ));
        }
        let (reason_code, profile_code) = request_capability_classification_v2(request);
        write_output(
            output,
            VckssBackendRequestCapabilityReceiptV2 {
                v1: VckssBackendRequestCapabilityReceiptV1 {
                    struct_size: struct_size_u32::<VckssBackendRequestCapabilityReceiptV2>()?,
                    abi_version: ABI_VERSION,
                    request_schema: request.v1.request_schema,
                    supported: u32::from(reason_code == VCKSS_REQUEST_REASON_SUPPORTED),
                    reason_code,
                    profile_code,
                    algorithm: request.v1.algorithm,
                    deletion_mode: request.v1.deletion_mode,
                    nuisance_mode: request.v1.nuisance_mode,
                    solver_route: request.v1.solver_route,
                    rng_contract: request.v1.rng_contract,
                    controls_count: request.v1.controls_count,
                    frequency_use: request.v1.frequency_use,
                    reserved: 0,
                    request_signature: request_capability_signature_v2(request),
                },
                engine: request.engine,
                batch_mode: request.batch_mode,
                stayers_mode: request.stayers_mode,
                target_weight_mode: request.target_weight_mode,
                deletion_unit_source: request.deletion_unit_source,
                probeorder_supplied: request.probeorder_supplied,
                wallseconds_supplied: request.wallseconds_supplied,
                reserved_2: 0,
                physical_limit: request.physical_limit,
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_backend_request_capability_v3(
    request: *const VckssBackendRequestCapabilityRequestV3,
    output: *mut VckssBackendRequestCapabilityReceiptV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssBackendRequestCapabilityReceiptV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 request capability receipt",
        )?;
        let request = copy_request_struct(request, "V3 request capability")?;
        require_abi(request.v2.v1.abi_version)?;
        if request.v2.v1.struct_size < struct_size_u32::<VckssBackendRequestCapabilityRequestV3>()?
        {
            return Err(abi_error(
                "V3 request capability reports a short structure size",
            ));
        }
        if request.v2.v1.reserved != 0
            || request.v2.reserved_2 != 0
            || request.reserved_3 != 0
            || request.reserved_4 != 0
        {
            return Err(abi_error(
                "reserved V3 request capability fields must be zero",
            ));
        }
        let (reason_code, profile_code) = request_capability_classification_v3(request);
        let signature = request_capability_signature_v3(request);
        let algorithm_deferred = u32::from(request.v2.v1.algorithm == VCKSS_ALGORITHM_AUTO);
        let engine_deferred = u32::from(
            request.v2.v1.algorithm == VCKSS_ALGORITHM_AUTO
                || (request.v2.v1.algorithm == VCKSS_ALGORITHM_JLA
                    && request.v2.engine == VCKSS_ENGINE_AUTO_OR_UNSPECIFIED),
        );
        let route_deferred = u32::from(
            request.v2.v1.algorithm != VCKSS_ALGORITHM_EXACT
                && request.v2.v1.solver_route == VCKSS_ROUTE_AUTO,
        );
        write_output(
            output,
            VckssBackendRequestCapabilityReceiptV3 {
                v2: VckssBackendRequestCapabilityReceiptV2 {
                    v1: VckssBackendRequestCapabilityReceiptV1 {
                        struct_size: struct_size_u32::<VckssBackendRequestCapabilityReceiptV3>()?,
                        abi_version: ABI_VERSION,
                        request_schema: request.v2.v1.request_schema,
                        supported: u32::from(reason_code == VCKSS_REQUEST_REASON_SUPPORTED),
                        reason_code,
                        profile_code,
                        algorithm: request.v2.v1.algorithm,
                        deletion_mode: request.v2.v1.deletion_mode,
                        nuisance_mode: request.v2.v1.nuisance_mode,
                        solver_route: request.v2.v1.solver_route,
                        rng_contract: request.v2.v1.rng_contract,
                        controls_count: request.v2.v1.controls_count,
                        frequency_use: request.v2.v1.frequency_use,
                        reserved: 0,
                        request_signature: signature,
                    },
                    engine: request.v2.engine,
                    batch_mode: request.v2.batch_mode,
                    stayers_mode: request.v2.stayers_mode,
                    target_weight_mode: request.v2.target_weight_mode,
                    deletion_unit_source: request.v2.deletion_unit_source,
                    probeorder_supplied: request.v2.probeorder_supplied,
                    wallseconds_supplied: request.v2.wallseconds_supplied,
                    reserved_2: 0,
                    physical_limit: request.v2.physical_limit,
                },
                leverage_batch_mode: request.leverage_batch_mode,
                target_batch_mode: request.target_batch_mode,
                allow_automatic_cmg_setup_fallback: request.allow_automatic_cmg_setup_fallback,
                reserved_3: 0,
                wallseconds: request.wallseconds,
                algorithm_resolution_deferred: algorithm_deferred,
                engine_resolution_deferred: engine_deferred,
                route_resolution_deferred: route_deferred,
                leverage_batch_resolution_deferred: u32::from(
                    request.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO,
                ),
                target_batch_resolution_deferred: u32::from(
                    request.target_batch_mode == VCKSS_BATCH_MODE_AUTO,
                ),
                wall_advisory_only: 1,
                reserved_4: 0,
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v1(
    output: *mut VckssEngineSolveRequestV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine default solve request",
        )?;
        write_output(output, VckssEngineSolveRequestV1::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_prepare_request_v3(
    output: *mut VckssEnginePrepareRequestV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePrepareRequestV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 engine default prepare request",
        )?;
        write_output(output, VckssEnginePrepareRequestV3::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v2(
    output: *mut VckssEngineSolveRequestV2,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV2>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V2 engine default solve request",
        )?;
        write_output(output, VckssEngineSolveRequestV2::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v3(
    output: *mut VckssEngineSolveRequestV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 engine default solve request",
        )?;
        write_output(output, VckssEngineSolveRequestV3::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v4(
    output: *mut VckssEngineSolveRequestV4,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV4>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V4 engine default solve request",
        )?;
        write_output(output, VckssEngineSolveRequestV4::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v5(
    output: *mut VckssEngineSolveRequestV5,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV5>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V5 engine default solve request",
        )?;
        write_output(output, VckssEngineSolveRequestV5::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_prepare_request_interrupt_v1(
    output: *mut VckssEnginePrepareRequestInterruptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePrepareRequestInterruptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine default interrupt prepare request",
        )?;
        write_output(output, VckssEnginePrepareRequestInterruptV1::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_prepare_request_interrupt_v2(
    output: *mut VckssEnginePrepareRequestInterruptV2,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePrepareRequestInterruptV2>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V2 engine default interrupt prepare request",
        )?;
        write_output(output, VckssEnginePrepareRequestInterruptV2::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_prepare_request_interrupt_v3(
    output: *mut VckssEnginePrepareRequestInterruptV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePrepareRequestInterruptV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 engine default interrupt prepare request",
        )?;
        write_output(output, VckssEnginePrepareRequestInterruptV3::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_stayer_augmentation_request_interrupt_v1(
    output: *mut VckssStayerAugmentationRequestInterruptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssStayerAugmentationRequestInterruptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "default interrupt stayer augmentation request",
        )?;
        write_output(output, VckssStayerAugmentationRequestInterruptV1::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v1(
    output: *mut VckssEngineSolveRequestInterruptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine default interrupt solve request",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV1::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v2(
    output: *mut VckssEngineSolveRequestInterruptV2,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV2>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V2 engine default interrupt solve request",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV2::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v3(
    output: *mut VckssEngineSolveRequestInterruptV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 engine default interrupt solve request",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV3::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v4(
    output: *mut VckssEngineSolveRequestInterruptV4,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV4>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V4 engine default interrupt solve request",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV4::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v5(
    output: *mut VckssEngineSolveRequestInterruptV5,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV5>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V5 engine default interrupt solve request",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV5::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_admit_prepare_v2(
    request: *const VckssEnginePrepareRequestV2,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V2 engine prepare request")?;
        validate_prepare_request_v2(request)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_admit_prepare_v3(
    request: *const VckssEnginePrepareRequestV3,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V3 engine prepare request")?;
        validate_prepare_request_v3(request)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_admit_prepare_probe_order_v1(
    request: *const VckssEnginePrepareRequestV3,
    probeorder_supplied: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "probe-order engine prepare request")?;
        if probeorder_supplied > 1 {
            return Err(BackendError::invalid(
                "engine_prepare",
                "probeorder_supplied must be zero or one",
            ));
        }
        validate_prepare_request_v3_with_probe_order(request, probeorder_supplied == 1)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_admit_prepare_v4(
    request: *const VckssEnginePrepareRequestV4,
    probeorder_supplied: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V4 engine prepare request")?;
        if probeorder_supplied > 1 {
            return Err(BackendError::invalid(
                "engine_prepare",
                "probeorder_supplied must be zero or one",
            ));
        }
        validate_prepare_request_v4(request, probeorder_supplied == 1)?;
        Ok(())
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_prepare_v2(
    request: *const VckssEnginePrepareRequestV2,
    columns: *const VckssEngineColumnsV1,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        prepare_v2_inner(
            request,
            columns,
            output_handle,
            output_capacity_bytes,
            &mut NeverInterrupt,
        )
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_prepare_v3(
    request: *const VckssEnginePrepareRequestV3,
    columns: *const VckssEngineColumnsV2,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V3 engine prepare request")?;
        prepare_v3_value(
            request,
            columns,
            output_handle,
            output_capacity_bytes,
            &mut NeverInterrupt,
        )
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_prepare_interrupt_v2(
    request: *const VckssEnginePrepareRequestInterruptV2,
    columns: *const VckssEngineColumnsV2,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<u64>(
            output_handle.cast::<u8>(),
            output_capacity_bytes,
            "engine output handle",
        )?;
        write_output(output_handle, 0);
        let request = copy_request_struct(request, "V2 interrupt engine prepare request")?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "prepare",
        )?;
        match interrupt {
            Some(mut interrupt) => prepare_v3_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut interrupt,
            ),
            None => prepare_v3_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut NeverInterrupt,
            ),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_prepare_interrupt_v3(
    request: *const VckssEnginePrepareRequestInterruptV2,
    columns: *const VckssEngineColumnsV3,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<u64>(
            output_handle.cast::<u8>(),
            output_capacity_bytes,
            "engine output handle",
        )?;
        write_output(output_handle, 0);
        let request = copy_request_struct(request, "V4 interrupt engine prepare request")?;
        let columns = copy_sized_struct(columns, "V3 engine column descriptor")?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "prepare",
        )?;
        match interrupt {
            Some(mut interrupt) => prepare_v3_columns_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut interrupt,
            ),
            None => prepare_v3_columns_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut NeverInterrupt,
            ),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_prepare_interrupt_v4(
    request: *const VckssEnginePrepareRequestInterruptV3,
    columns: *const VckssEngineColumnsV3,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<u64>(
            output_handle.cast::<u8>(),
            output_capacity_bytes,
            "engine output handle",
        )?;
        write_output(output_handle, 0);
        let request = copy_request_struct(request, "V3 interrupt engine prepare request")?;
        let columns = copy_sized_struct(columns, "V3 engine column descriptor")?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "prepare",
        )?;
        match interrupt {
            Some(mut interrupt) => prepare_v4_columns_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut interrupt,
            ),
            None => prepare_v4_columns_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut NeverInterrupt,
            ),
        }
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_prepare_interrupt_v1(
    request: *const VckssEnginePrepareRequestInterruptV1,
    columns: *const VckssEngineColumnsV1,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<u64>(
            output_handle.cast::<u8>(),
            output_capacity_bytes,
            "engine output handle",
        )?;
        // A caller that supplied a writable handle slot never observes a
        // stale generation, even when the request header itself is malformed.
        // Capacity and nullness are checked before either pointer is touched.
        write_output(output_handle, 0);
        let request = copy_request_struct(request, "interrupt engine prepare request")?;
        let mut interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "prepare",
        )?;
        match interrupt.as_mut() {
            Some(interrupt) => prepare_v2_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                interrupt,
            ),
            None => prepare_v2_value(
                request.options,
                columns,
                output_handle,
                output_capacity_bytes,
                &mut NeverInterrupt,
            ),
        }
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_prepare_v1(
    request: *const VckssEnginePrepareRequestV1,
    columns: *const VckssEngineColumnsV1,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "engine prepare request")?;
        let columns = copy_sized_struct(columns, "engine column descriptor")?;
        require_abi(request.abi_version)?;
        if request.reserved != 0 || columns.reserved != 0 {
            return Err(abi_error("reserved preparation fields must be zero"));
        }
        if request.rows == 0 || request.rows != columns.rows {
            return Err(BackendError::invalid(
                "engine_prepare",
                "request and column row counts must agree and be positive",
            ));
        }
        if request.cleanup_abandoned > 1 {
            return Err(BackendError::invalid(
                "engine_prepare",
                "cleanup_abandoned must be zero or one",
            ));
        }
        require_output_capacity::<u64>(
            output_handle.cast::<u8>(),
            output_capacity_bytes,
            "engine output handle",
        )?;
        write_output(output_handle, 0);
        let rows = to_usize(request.rows, "engine_prepare", "row count")?;
        clear_abandoned_before_replacement(request.cleanup_abandoned)?;
        let ingest_start = Instant::now();
        let input = copy_columns(&columns, rows)?;
        let ingest_ns = duration_ns(ingest_start.elapsed());
        let mut prepared = PreparedProblemWithMask::from_columns(input)?;
        prepared.performance.ingest_ns = ingest_ns;

        let mut state = lock_engine("engine_prepare")?;
        let handle = state.registry.prepare(prepared)?;
        // SAFETY: capacity for one u64 was validated; unaligned C storage is
        // permitted and no Rust reference is formed.
        unsafe { output_handle.write_unaligned(handle.generation()) };
        Ok(())
    })
}

fn prepare_v2_inner(
    request: *const VckssEnginePrepareRequestV2,
    columns: *const VckssEngineColumnsV1,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let request = copy_request_struct(request, "V2 engine prepare request")?;
    prepare_v2_value(
        request,
        columns,
        output_handle,
        output_capacity_bytes,
        interrupt,
    )
}

fn prepare_v2_value(
    request: VckssEnginePrepareRequestV2,
    columns: *const VckssEngineColumnsV1,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    require_output_capacity::<u64>(
        output_handle.cast::<u8>(),
        output_capacity_bytes,
        "engine output handle",
    )?;
    // A failed preparation never exposes a stale or partial generation.
    write_output(output_handle, 0);
    interrupt.checkpoint("engine_prepare_entry")?;
    let memory = validate_prepare_request_v2(request)?;
    let columns = copy_sized_struct(columns, "engine column descriptor")?;
    if columns.reserved != 0 || request.rows != columns.rows {
        return Err(abi_error(
            "V2 preparation columns must match the request and reserve zero",
        ));
    }
    let rows = to_usize(request.rows, "engine_prepare", "row count")?;
    clear_abandoned_before_replacement(request.cleanup_abandoned)?;
    let ingest_start = Instant::now();
    let input = copy_columns_with_interrupt(&columns, rows, interrupt)?;
    let ingest_ns = duration_ns(ingest_start.elapsed());
    let mut prepared =
        PreparedProblemWithMask::from_columns_with_memory_and_interrupt(input, memory, interrupt)?;
    prepared.performance.ingest_ns = ingest_ns;
    interrupt.checkpoint("engine_prepare_final")?;

    let mut state = lock_engine("engine_prepare")?;
    let handle = state.registry.prepare(prepared)?;
    write_output(output_handle, handle.generation());
    Ok(())
}

fn prepare_v3_value(
    request: VckssEnginePrepareRequestV3,
    columns: *const VckssEngineColumnsV2,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    require_output_capacity::<u64>(
        output_handle.cast::<u8>(),
        output_capacity_bytes,
        "engine output handle",
    )?;
    write_output(output_handle, 0);
    interrupt.checkpoint("engine_prepare_entry")?;
    let (deletion, memory) = validate_prepare_request_v3(request)?;
    let columns = copy_sized_struct(columns, "V2 engine column descriptor")?;
    if columns.v1.reserved != 0 || columns.reserved_2 != 0 {
        return Err(abi_error("reserved V2 column fields must be zero"));
    }
    if request.v2.rows != columns.v1.rows {
        return Err(BackendError::invalid(
            "engine_prepare",
            "request and column row counts must agree",
        ));
    }
    if request.controls_count != columns.controls_count {
        return Err(BackendError::invalid(
            "engine_prepare",
            "request and descriptor control counts disagree",
        ));
    }
    let rows = to_usize(request.v2.rows, "engine_prepare", "row count")?;
    clear_abandoned_before_replacement(request.v2.cleanup_abandoned)?;
    let ingest_start = Instant::now();
    let input = copy_columns_v2_with_interrupt(&columns, rows, interrupt)?;
    let ingest_ns = duration_ns(ingest_start.elapsed());
    let mut prepared = PreparedProblemWithMask::from_columns_with_mode_and_memory_and_interrupt(
        input, deletion, memory, interrupt,
    )?;
    prepared.performance.ingest_ns = ingest_ns;
    interrupt.checkpoint("engine_prepare_final")?;

    let mut state = lock_engine("engine_prepare")?;
    let handle = state.registry.prepare(prepared)?;
    write_output(output_handle, handle.generation());
    Ok(())
}

fn prepare_v3_columns_value(
    request: VckssEnginePrepareRequestV3,
    columns: VckssEngineColumnsV3,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let (deletion, memory) =
        validate_prepare_request_v3_with_probe_order(request, columns.probeorder_supplied == 1)?;
    prepare_columns_value(
        request.v2,
        request.controls_count,
        deletion,
        false,
        memory,
        columns,
        output_handle,
        output_capacity_bytes,
        interrupt,
    )
}

fn prepare_v4_columns_value(
    request: VckssEnginePrepareRequestV4,
    columns: VckssEngineColumnsV3,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let (deletion, implicit_match, memory) =
        validate_prepare_request_v4(request, columns.probeorder_supplied == 1)?;
    prepare_columns_value(
        request.v3.v2,
        request.v3.controls_count,
        deletion,
        implicit_match,
        memory,
        columns,
        output_handle,
        output_capacity_bytes,
        interrupt,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_columns_value(
    request: VckssEnginePrepareRequestV2,
    controls_count: u32,
    deletion: DeletionMode,
    implicit_match: bool,
    memory: PreparationMemoryReceipt,
    columns: VckssEngineColumnsV3,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    require_output_capacity::<u64>(
        output_handle.cast::<u8>(),
        output_capacity_bytes,
        "engine output handle",
    )?;
    write_output(output_handle, 0);
    interrupt.checkpoint("engine_prepare_entry")?;
    if columns.v2.v1.reserved != 0
        || columns.v2.reserved_2 != 0
        || columns.reserved_3 != 0
        || columns.probeorder_supplied > 1
    {
        return Err(abi_error(
            "reserved or boolean V3 column fields are invalid",
        ));
    }
    if request.rows != columns.v2.v1.rows {
        return Err(BackendError::invalid(
            "engine_prepare",
            "request and column row counts must agree",
        ));
    }
    if controls_count != columns.v2.controls_count {
        return Err(BackendError::invalid(
            "engine_prepare",
            "request and descriptor control counts disagree",
        ));
    }
    let rows = to_usize(request.rows, "engine_prepare", "row count")?;
    let ingest_start = Instant::now();
    let probe_order = if columns.probeorder_supplied == 1 {
        Some(copy_finite_column(
            columns.probe_order,
            rows,
            "probe order",
            interrupt,
        )?)
    } else {
        if !columns.probe_order.is_null() {
            return Err(BackendError::invalid(
                "engine_prepare",
                "an omitted probe order requires a null column pointer",
            ));
        }
        None
    };
    clear_abandoned_before_replacement(request.cleanup_abandoned)?;
    let input = copy_columns_v2_with_identifier_mode_and_interrupt(
        &columns.v2,
        rows,
        implicit_match,
        interrupt,
    )?;
    let ingest_ns = duration_ns(ingest_start.elapsed());
    let mut prepared =
        PreparedProblemWithMask::from_columns_with_probe_order_mode_implicit_match_memory_and_interrupt(
            input,
            probe_order,
            deletion,
            implicit_match,
            memory,
            interrupt,
        )?;
    prepared.performance.ingest_ns = ingest_ns;
    interrupt.checkpoint("engine_prepare_final")?;

    let mut state = lock_engine("engine_prepare")?;
    let handle = state.registry.prepare(prepared)?;
    write_output(output_handle, handle.generation());
    Ok(())
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_augment_stayers_v1(
    generation: u64,
    request: *const VckssStayerAugmentationRequestV1,
    columns: *const VckssStayerAugmentationColumnsV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "stayer augmentation request")?;
        augment_stayers_value(generation, request, columns, &mut NeverInterrupt)
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_augment_stayers_interrupt_v1(
    generation: u64,
    request: *const VckssStayerAugmentationRequestInterruptV1,
    columns: *const VckssStayerAugmentationColumnsV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "interrupt stayer augmentation request")?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "stayer augmentation",
        )?;
        match interrupt {
            Some(mut interrupt) => {
                augment_stayers_value(generation, request.options, columns, &mut interrupt)
            }
            None => {
                augment_stayers_value(generation, request.options, columns, &mut NeverInterrupt)
            }
        }
    })
}

fn augment_stayers_value(
    generation: u64,
    request: VckssStayerAugmentationRequestV1,
    columns: *const VckssStayerAugmentationColumnsV1,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    interrupt.checkpoint("engine_stayer_augmentation_entry")?;
    require_abi(request.abi_version)?;
    if request.struct_size < struct_size_u32::<VckssStayerAugmentationRequestV1>()?
        || request.reserved != 0
    {
        return Err(abi_error(
            "stayer augmentation request is short or reserves nonzero fields",
        ));
    }
    let columns = copy_sized_struct(columns, "stayer augmentation column descriptor")?;
    if columns.reserved != 0 || columns.reserved_2 != 0 {
        return Err(abi_error(
            "reserved stayer augmentation column fields must be zero",
        ));
    }
    if request.rows != columns.rows || request.controls_count != columns.controls_count {
        return Err(BackendError::invalid(
            "engine_stayer_augmentation",
            "stayer request and column dimensions disagree",
        ));
    }
    let rows = to_usize(
        request.rows,
        "engine_stayer_augmentation",
        "stayer row count",
    )?;
    let input = copy_stayer_augmentation_columns(&columns, rows, interrupt)?;
    interrupt.checkpoint("engine_stayer_augmentation_copied")?;
    let handle = ContextHandle::from_generation(generation)?;
    let mut state = lock_engine("engine_stayer_augmentation")?;
    state.registry.augment_prepared(handle, |prepared| {
        let memory = admit_stayer_augmentation_memory(
            prepared.receipt.retained_rows,
            request.rows,
            request.controls_count,
            prepared.receipt.memory.hard_limit_bytes,
            prepared.receipt.memory.prepared_resident_bytes,
            request.caller_copy_bytes,
        )?;
        prepared.augment_stayers_with_memory_and_interrupt(input, memory, interrupt)
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_stayer_augmentation_receipt_v1(
    generation: u64,
    output: *mut VckssStayerAugmentationReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssStayerAugmentationReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "stayer augmentation receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_stayer_augmentation_receipt")?;
        let receipt = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => {
                prepared
                    .stayer_augmentation
                    .as_ref()
                    .map(|value| EngineStayerAugmentationReceipt {
                        core: value.core.receipt,
                        memory: value.memory,
                    })
            }
            ContextPayloadRef::Solved(solved) => solved.stayer_augmentation,
        }
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_stayer_augmentation_receipt",
                "the generation has no stayer augmentation",
            )
        })?;
        write_output(output, stayer_augmentation_receipt_v1(generation, receipt)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_preparation_receipt_v1(
    generation: u64,
    output: *mut VckssEnginePreparationReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePreparationReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine preparation receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_receipt")?;
        let receipt = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => prepared.receipt,
            ContextPayloadRef::Solved(solved) => solved.preparation,
        };
        write_output(
            output,
            VckssEnginePreparationReceiptV1 {
                struct_size: struct_size_u32::<VckssEnginePreparationReceiptV1>()?,
                reserved: 0,
                generation,
                input_rows: receipt.input_rows,
                retained_rows: receipt.retained_rows,
                workers: receipt.workers,
                firms: receipt.firms,
                cells: receipt.cells,
                deletion_units: receipt.deletion_units,
                target_strata: receipt.target_strata,
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_preparation_receipt_v2(
    generation: u64,
    output: *mut VckssEnginePreparationReceiptV2,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePreparationReceiptV2>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V2 engine preparation receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_receipt")?;
        let receipt = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => prepared.receipt,
            ContextPayloadRef::Solved(solved) => solved.preparation,
        };
        write_output(output, preparation_receipt_v2(generation, receipt)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_preparation_receipt_v3(
    generation: u64,
    output: *mut VckssEnginePreparationReceiptV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePreparationReceiptV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 engine preparation receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_receipt")?;
        let receipt = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => prepared.receipt,
            ContextPayloadRef::Solved(solved) => solved.preparation,
        };
        write_output(
            output,
            VckssEnginePreparationReceiptV3 {
                v2: preparation_receipt_v2(generation, receipt)?,
                target_weight_sum: receipt.target_weight_sum,
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_preparation_receipt_v4(
    generation: u64,
    output: *mut VckssEnginePreparationReceiptV4,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePreparationReceiptV4>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V4 engine preparation receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_receipt")?;
        let (receipt, controls_count, deletion_mode) = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => (
                prepared.receipt,
                u32::try_from(prepared.problem.controls.len()).map_err(|_| {
                    resource_error(
                        "engine_receipt",
                        "control count is not representable as u32",
                    )
                })?,
                deletion_code(prepared.deletion),
            ),
            ContextPayloadRef::Solved(solved) => (
                solved.preparation,
                solved.controls_count,
                deletion_code(solved.deletion),
            ),
        };
        write_output(
            output,
            VckssEnginePreparationReceiptV4 {
                v3: VckssEnginePreparationReceiptV3 {
                    v2: preparation_receipt_v2(generation, receipt)?,
                    target_weight_sum: receipt.target_weight_sum,
                },
                controls_count,
                deletion_mode,
            },
        );
        Ok(())
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_retained_mask_v1(
    generation: u64,
    output: *mut u8,
    output_capacity_bytes: u64,
) -> i32 {
    ffi_status(|| {
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_retained")?;
        let retained = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => prepared.retained.as_slice(),
            ContextPayloadRef::Solved(solved) => solved.retained.as_slice(),
        };
        let required = u64::try_from(retained.len()).map_err(|_| {
            resource_error(
                "engine_retained",
                "retained-mask length is not representable as u64",
            )
        })?;
        if output.is_null() || output_capacity_bytes < required {
            return Err(BackendError::invalid(
                "engine_retained",
                format!(
                    "retained-mask capacity {output_capacity_bytes} is below required size {required}"
                ),
            ));
        }
        for (index, &kept) in retained.iter().enumerate() {
            // SAFETY: the validated caller capacity covers every bounded write.
            unsafe { output.add(index).write(u8::from(kept)) };
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v1(
    generation: u64,
    request: *const VckssEngineSolveRequestV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "engine solve request")?;
        require_abi(request.abi_version)?;
        let mut interrupt = NeverInterrupt;
        solve_engine(generation, request, &mut interrupt)
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v1(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "interrupt engine solve request")?;
        require_abi(request.options.abi_version)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) => solve_engine(generation, request.options, &mut interrupt),
            None => {
                let mut interrupt = NeverInterrupt;
                solve_engine(generation, request.options, &mut interrupt)
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v2(
    generation: u64,
    request: *const VckssEngineSolveRequestV2,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V2 engine solve request")?;
        require_abi(request.v1.abi_version)?;
        solve_engine_v2(generation, request, &mut NeverInterrupt)
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v2(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV2,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V2 interrupt engine solve request")?;
        require_abi(request.options.v1.abi_version)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) => solve_engine_v2(generation, request.options, &mut interrupt),
            None => solve_engine_v2(generation, request.options, &mut NeverInterrupt),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v3(
    generation: u64,
    request: *const VckssEngineSolveRequestV3,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V3 engine solve request")?;
        require_abi(request.v2.v1.abi_version)?;
        solve_engine_v3(generation, request, &mut NeverInterrupt)
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v3(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV3,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V3 interrupt engine solve request")?;
        require_abi(request.options.v2.v1.abi_version)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) => solve_engine_v3(generation, request.options, &mut interrupt),
            None => solve_engine_v3(generation, request.options, &mut NeverInterrupt),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v4(
    generation: u64,
    request: *const VckssEngineSolveRequestV4,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V4 engine solve request")?;
        require_abi(request.v3.v2.v1.abi_version)?;
        solve_engine_v4(
            generation,
            request,
            None,
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v4(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV4,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V4 interrupt engine solve request")?;
        require_abi(request.options.v3.v2.v1.abi_version)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) => solve_engine_v4(
                generation,
                request.options,
                None,
                V4SolveExecution::Caller(&mut interrupt),
            ),
            None => solve_engine_v4(
                generation,
                request.options,
                None,
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v5(
    generation: u64,
    request: *const VckssEngineSolveRequestV5,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V5 engine solve request")?;
        require_abi(request.v4.v3.v2.v1.abi_version)?;
        let full_cmg = validate_full_cmg_v2_request(request)?;
        solve_engine_v4(
            generation,
            request.v4,
            full_cmg,
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v5(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV5,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V5 interrupt engine solve request")?;
        require_abi(request.options.v4.v3.v2.v1.abi_version)?;
        let full_cmg = validate_full_cmg_v2_request(request.options)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) if full_cmg.is_some() => solve_engine_v4(
                generation,
                request.options.v4,
                full_cmg,
                V4SolveExecution::Coordinated(&mut interrupt),
            ),
            Some(mut interrupt) => solve_engine_v4(
                generation,
                request.options.v4,
                full_cmg,
                V4SolveExecution::Caller(&mut interrupt),
            ),
            None => solve_engine_v4(
                generation,
                request.options.v4,
                full_cmg,
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

enum V4SolveExecution<'a> {
    Caller(&'a mut dyn InterruptCheck),
    Coordinated(&'a mut CallbackInterrupt),
}

#[allow(clippy::too_many_lines)]
fn solve_engine_v4(
    generation: u64,
    request: VckssEngineSolveRequestV4,
    full_cmg: Option<FullCmgPlanOptions>,
    execution: V4SolveExecution<'_>,
) -> Result<()> {
    if request.v3.v2.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV4>()? {
        return Err(abi_error("V4 solve request reports a short structure size"));
    }
    if request.v3.reserved_3 != 0 || request.reserved_4 != 0 {
        return Err(abi_error("reserved V4 solve request fields must be zero"));
    }
    if request.v3.capability_schema != VCKSS_REQUEST_CAPABILITY_SCHEMA_V3
        || request.v3.capability_profile != VCKSS_REQUEST_PROFILE_PLANNED_V1
    {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "V4 solve requires the planned-command V3 capability profile",
        ));
    }
    let (exact_limit, blocksize_limit) = validate_exact_request_options(request.v3.v2)?;
    let algorithm_requested = algorithm_request_from_code(request.v3.v2.algorithm)?;
    let engine_requested = engine_request_from_code(request.v3.engine)?;
    let deletion = deletion_from_code(request.v3.v2.v1.deletion_mode)?;
    let nuisance = nuisance_from_code(request.v3.v2.nuisance_mode)?;
    let leverage_batch = batch_request_from_code(
        request.leverage_batch_mode,
        request.v3.v2.v1.leverage_batch_width,
        "leverage",
    )?;
    let target_batch = batch_request_from_code(
        request.target_batch_mode,
        request.v3.v2.v1.target_batch_width,
        "target",
    )?;
    let wallseconds = if request.v3.wallseconds_supplied == 1 {
        Some(request.wallseconds)
    } else {
        None
    };
    if request.v3.v2.algorithm != VCKSS_ALGORITHM_EXACT && request.v3.v2.v1.probes < 2 {
        return Err(BackendError::invalid(
            "engine_solve",
            "JLA-capable V4 requests require at least two probes",
        ));
    }
    let handle = ContextHandle::from_generation(generation)?;
    let operation = move |prepared: &PreparedProblemWithMask,
                          interrupt: &mut dyn InterruptCheck|
          -> Result<EngineSolved> {
        if request.v3.probeorder_supplied != u32::from(prepared.problem.probe_order.is_some()) {
            return Err(BackendError::invalid(
                "engine_solve",
                "solve probe-order declaration differs from prepared input",
            ));
        }
        let controls_count = u32::try_from(prepared.problem.controls.len()).map_err(|_| {
            resource_error("engine_solve", "control count is not representable as u32")
        })?;
        if full_cmg.is_some() && controls_count != 0 {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "cmg_full_v2",
                "CMG_FULL_V2 does not yet support controls",
            ));
        }
        let capability = capability_request_v3_for_solve(request, controls_count)?;
        let (reason, profile) = request_capability_classification_v3(capability);
        if reason != VCKSS_REQUEST_REASON_SUPPORTED
            || profile != request.v3.capability_profile
            || request_capability_signature_v3(capability) != request.v3.request_signature
        {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_solve",
                "V4 solve does not reconcile with its V3 capability receipt",
            ));
        }
        if prepared.deletion != deletion {
            return Err(BackendError::invalid(
                "engine_solve",
                "solve deletion mode differs from the prepared graph mode",
            ));
        }
        let stayer_augmentation = match request.v3.stayers_mode {
            VCKSS_STAYERS_MOVERS => {
                if prepared.stayer_augmentation.is_some() {
                    return Err(BackendError::invalid(
                        "engine_solve",
                        "a mover-only solve cannot consume a stayer-augmented generation",
                    ));
                }
                None
            }
            VCKSS_STAYERS_ALL => Some(prepared.stayer_augmentation.as_ref().ok_or_else(|| {
                BackendError::invalid(
                    "engine_solve",
                    "a stayers-all solve requires a reconciled stayer augmentation",
                )
            })?),
            _ => {
                return Err(BackendError::invalid(
                    "engine_solve",
                    "the solve request contains an unknown stayer mode",
                ));
            }
        };
        let compressed_physical_rng_ready = prepared.plan.as_ref().is_some_and(|plan| {
            plan.deletion
                .physical_count
                .iter()
                .chain(&plan.target.physical_count)
                .all(|&count| count != 0 && count.div_ceil(64) <= MAX_PHYSICAL_WORDS_PER_ATOM)
        });
        let estimator_plan = resolve_estimator_plan(EstimatorPlanRequest {
            algorithm: algorithm_requested,
            engine: engine_requested,
            deletion,
            nuisance,
            retained_workers: prepared.problem.workers(),
            retained_firms: prepared.problem.firms(),
            controls: prepared.problem.controls.len(),
            exact_limit,
            compressed_semantic_plan_ready: prepared.plan.is_some(),
            compressed_physical_rng_ready,
        })?;
        if estimator_plan.engine.selected != SelectedEngine::NotApplicable
            && prepared.problem.physical_total > request.v3.physical_limit
        {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "engine_solve",
                "retained physical mass exceeds physical_limit()",
            ));
        }
        let memory_limit_bytes = if prepared.receipt.memory.hard_limit_bytes == 0 {
            u64::MAX
        } else {
            prepared.receipt.memory.hard_limit_bytes
        };
        let prepared_persistent_bytes = stayer_augmentation
            .map_or(prepared.receipt.memory.prepared_resident_bytes, |value| {
                value.memory.total_prepared_resident_bytes
            });
        let full_cmg = full_cmg.map(|plan| {
            plan.with_prepared_memory(
                prepared.receipt.memory.preparation_peak_forecast_bytes,
                prepared_persistent_bytes,
            )
        });
        let retained_mask_bytes = to_u64(
            bit_packed_capacity_bytes(prepared.retained.capacity()),
            "bit-packed retained-mask capacity",
        )?;
        let solve_start = Instant::now();
        let (result, execution_plan, leverage_active, target_active, stayer_hybrid, full_cmg) =
            match estimator_plan.engine.selected {
                SelectedEngine::NotApplicable => {
                    if prepared.deletion == DeletionMode::Match
                        && (0..prepared.problem.deletion_units()).any(|group| {
                            prepared.problem.deletion_index.range(group).len() > blocksize_limit
                        })
                    {
                        return Err(BackendError::new(
                            ErrorCode::ResourceLimit,
                            "exact_estimator",
                            "a deletion block exceeds blocksize_limit()",
                        ));
                    }
                    let exact_options = ExactEstimatorOptions {
                        deletion,
                        nuisance,
                        rank_tolerance: request.v3.v2.v1.rank_tolerance,
                        block_tolerance: request.v3.v2.v1.block_tolerance,
                        solver_tolerance: request.v3.v2.v1.pcg_tolerance,
                        exact_limit,
                        blocksize_limit,
                        memory_limit_bytes,
                        prepared_persistent_bytes,
                    };
                    let planned = run_exact_estimator_planned_with_interrupt(
                        &prepared.problem,
                        PlannedExactEstimatorOptions {
                            estimator: exact_options,
                            wallseconds,
                        },
                        interrupt,
                    )?;
                    let stayer_hybrid = stayer_augmentation
                        .map(|augmentation| {
                            run_exact_stayer_hybrid_with_interrupt(
                                &augmentation.core.problem,
                                &augmentation.core.plan,
                                exact_options,
                                interrupt,
                            )
                        })
                        .transpose()?;
                    let plan = execution_plan_exact(
                        generation,
                        request.v3.request_signature,
                        &estimator_plan,
                        &planned.execution,
                        memory_limit_bytes,
                        prepared_persistent_bytes,
                    )?;
                    (
                        EngineEstimate::Exact(planned.estimator),
                        plan,
                        0,
                        0,
                        stayer_hybrid,
                        None,
                    )
                }
                SelectedEngine::Compressed => {
                    let mut estimator_request = request.v3.v2.v1;
                    if request.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO {
                        estimator_request.leverage_batch_width = 1;
                    }
                    if request.target_batch_mode == VCKSS_BATCH_MODE_AUTO {
                        estimator_request.target_batch_width = 1;
                    }
                    let mut estimator = options_from_request(estimator_request)?;
                    apply_prepared_memory_admission(
                        &mut estimator,
                        memory_limit_bytes,
                        prepared_persistent_bytes,
                        full_cmg.is_some(),
                    );
                    let planned = run_jla_no_controls_planned_with_interrupt(
                        &prepared.problem,
                        PlannedJlaEngineOptions {
                            estimator,
                            leverage_batch,
                            target_batch,
                            wallseconds,
                            full_cmg,
                        },
                        interrupt,
                    )?;
                    let leverage_active = to_u32(
                        planned.execution.batch.leverage_active_width,
                        "compressed leverage batch width",
                    )?;
                    let target_active = to_u32(
                        planned.execution.batch.target_active_width,
                        "compressed target batch width",
                    )?;
                    let plan = execution_plan_compressed(
                        generation,
                        request.v3.request_signature,
                        &estimator_plan,
                        &planned.execution,
                    )?;
                    let full_cmg = planned.execution.full_cmg.clone();
                    (
                        EngineEstimate::Jla(planned.estimator),
                        plan,
                        leverage_active,
                        target_active,
                        None,
                        full_cmg,
                    )
                }
                SelectedEngine::Generic => {
                    let (_, rhs_export_bytes) = generic_rhs_export_memory(
                        controls_count,
                        request.v3.v2.v1.probes,
                        nuisance,
                    )?;
                    let routing = model_routing_from_request(request.v3.v2.v1)?;
                    let result = run_generic_jla_routed_with_interrupt(
                        &prepared.problem,
                        GenericJlaExecutionOptions {
                            estimator: GenericJlaOptions {
                                seed: request.v3.v2.v1.seed,
                                probes: request.v3.v2.v1.probes,
                                leverage_batch_width: 1,
                                target_batch_width: 1,
                                deletion,
                                nuisance,
                                rank_tolerance: request.v3.v2.v1.rank_tolerance,
                                block_tolerance: request.v3.v2.v1.block_tolerance,
                                blocksize_limit,
                                memory_limit_bytes,
                                prepared_persistent_bytes,
                                retained_mask_bytes,
                                rhs_export_bytes,
                                solver: routing.solver,
                            },
                            routing,
                            leverage_batch,
                            target_batch,
                            wallseconds,
                        },
                        interrupt,
                    )?;
                    let leverage_active = to_u32(
                        result.receipt.execution.batch.leverage_active_width,
                        "generic leverage batch width",
                    )?;
                    let target_active = to_u32(
                        result.receipt.execution.batch.target_active_width,
                        "generic target batch width",
                    )?;
                    let plan = execution_plan_generic(
                        generation,
                        request.v3.request_signature,
                        &estimator_plan,
                        &result.receipt.execution,
                        &result.receipt,
                        memory_limit_bytes,
                        prepared_persistent_bytes,
                    )?;
                    (
                        EngineEstimate::GenericJla(result),
                        plan,
                        leverage_active,
                        target_active,
                        None,
                        None,
                    )
                }
            };
        let mut performance = prepared.performance;
        performance.solve_ns = duration_ns(solve_start.elapsed());
        Ok(EngineSolved {
            result,
            algorithm_requested: request.v3.v2.algorithm,
            algorithm_selected: algorithm_code(estimator_plan.algorithm.selected),
            engine_requested: request.v3.engine,
            engine_selected: selected_engine_code(estimator_plan.engine.selected),
            seed: if estimator_plan.algorithm.selected == EstimatorAlgorithm::Exact {
                0
            } else {
                request.v3.v2.v1.seed
            },
            probes: if estimator_plan.algorithm.selected == EstimatorAlgorithm::Exact {
                0
            } else {
                request.v3.v2.v1.probes
            },
            leverage_batch_width: leverage_active,
            target_batch_width: target_active,
            nuisance,
            deletion,
            rank_tolerance: request.v3.v2.v1.rank_tolerance,
            block_tolerance: request.v3.v2.v1.block_tolerance,
            controls_count,
            capability_schema: request.v3.capability_schema,
            capability_profile: request.v3.capability_profile,
            batch_mode: request.v3.batch_mode,
            stayers_mode: request.v3.stayers_mode,
            target_weight_mode: request.v3.target_weight_mode,
            deletion_unit_source: request.v3.deletion_unit_source,
            probeorder_supplied: request.v3.probeorder_supplied,
            wallseconds_supplied: request.v3.wallseconds_supplied,
            frequency_use: request.v3.frequency_use,
            physical_limit: request.v3.physical_limit,
            request_signature: request.v3.request_signature,
            execution_plan: Some(execution_plan),
            full_cmg,
            preparation: prepared.receipt,
            retained: Arc::clone(&prepared.retained),
            stayer_augmentation: stayer_augmentation.map(|value| EngineStayerAugmentationReceipt {
                core: value.core.receipt,
                memory: value.memory,
            }),
            stayer_hybrid,
            performance,
        })
    };
    let mut state = lock_engine("engine_solve")?;
    match execution {
        V4SolveExecution::Caller(interrupt) => state
            .registry
            .solve_preserving(handle, |prepared| operation(prepared, interrupt)),
        V4SolveExecution::Coordinated(callback) => state.registry.solve_preserving_coordinated(
            handle,
            |prepared, cancellation| {
                let mut interrupt = CancellationInterrupt::new(cancellation);
                operation(prepared, &mut interrupt)
            },
            || callback.checkpoint("cmg_full_v2_coordinator"),
        ),
    }
}

fn solve_engine_v3(
    generation: u64,
    request: VckssEngineSolveRequestV3,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if request.v2.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV3>()? {
        return Err(abi_error("V3 solve request reports a short structure size"));
    }
    if request.reserved_3 != 0 {
        return Err(abi_error("reserved V3 solve request fields must be zero"));
    }
    if request.v2.algorithm != VCKSS_ALGORITHM_JLA {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "V3 generic-engine solve requires explicit JLA",
        ));
    }
    if request.engine != VCKSS_ENGINE_GENERIC {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "V3 solve currently supports only the explicit generic engine",
        ));
    }
    if request.capability_schema != VCKSS_REQUEST_CAPABILITY_SCHEMA_V2
        || request.capability_profile != VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1
    {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "V3 generic solve requires the generic-JLA capability schema and profile",
        ));
    }
    let nuisance = nuisance_from_code(request.v2.nuisance_mode)?;
    let deletion = deletion_from_code(request.v2.v1.deletion_mode)?;
    let blocksize_limit = validate_generic_request_options(request)?;
    let handle = ContextHandle::from_generation(generation)?;
    let mut state = lock_engine("engine_solve")?;
    state.registry.solve_preserving(handle, |prepared| {
        if prepared.deletion != deletion {
            return Err(BackendError::invalid(
                "engine_solve",
                "solve deletion mode differs from the prepared graph mode",
            ));
        }
        let controls_count = u32::try_from(prepared.problem.controls.len()).map_err(|_| {
            resource_error("engine_solve", "control count is not representable as u32")
        })?;
        if controls_count > 32 {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_solve",
                "generic JLA supports at most 32 controls",
            ));
        }
        let capability = VckssBackendRequestCapabilityRequestV2 {
            v1: VckssBackendRequestCapabilityRequestV1 {
                abi_version: request.v2.v1.abi_version,
                struct_size: struct_size_u32::<VckssBackendRequestCapabilityRequestV2>()?,
                request_schema: request.capability_schema,
                algorithm: request.v2.algorithm,
                deletion_mode: request.v2.v1.deletion_mode,
                nuisance_mode: request.v2.nuisance_mode,
                solver_route: request.v2.v1.solver_route,
                rng_contract: request.v2.v1.rng_contract,
                controls_count,
                frequency_use: request.frequency_use,
                reserved: 0,
            },
            engine: request.engine,
            batch_mode: request.batch_mode,
            stayers_mode: request.stayers_mode,
            target_weight_mode: request.target_weight_mode,
            deletion_unit_source: request.deletion_unit_source,
            probeorder_supplied: request.probeorder_supplied,
            wallseconds_supplied: request.wallseconds_supplied,
            reserved_2: 0,
            physical_limit: request.physical_limit,
        };
        let (reason, profile) = request_capability_classification_v2(capability);
        if reason != VCKSS_REQUEST_REASON_SUPPORTED
            || profile != request.capability_profile
            || request_capability_signature_v2(capability) != request.request_signature
        {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_solve",
                "V3 solve does not reconcile with its generic-JLA capability receipt",
            ));
        }
        if prepared.problem.physical_total > request.physical_limit {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "engine_solve",
                "retained physical mass exceeds physical_limit()",
            ));
        }
        let memory_limit_bytes = if prepared.receipt.memory.hard_limit_bytes == 0 {
            u64::MAX
        } else {
            prepared.receipt.memory.hard_limit_bytes
        };
        let (_, rhs_export_bytes) =
            generic_rhs_export_memory(controls_count, request.v2.v1.probes, nuisance)?;
        let retained_mask_bytes = to_u64(
            bit_packed_capacity_bytes(prepared.retained.capacity()),
            "bit-packed retained-mask capacity",
        )?;
        let solve_start = Instant::now();
        let result = run_generic_jla_with_interrupt(
            &prepared.problem,
            GenericJlaOptions {
                seed: request.v2.v1.seed,
                probes: request.v2.v1.probes,
                leverage_batch_width: usize::try_from(request.v2.v1.leverage_batch_width).map_err(
                    |_| resource_error("engine_solve", "leverage width is not representable"),
                )?,
                target_batch_width: usize::try_from(request.v2.v1.target_batch_width).map_err(
                    |_| resource_error("engine_solve", "target width is not representable"),
                )?,
                deletion,
                nuisance,
                rank_tolerance: request.v2.v1.rank_tolerance,
                block_tolerance: request.v2.v1.block_tolerance,
                blocksize_limit,
                memory_limit_bytes,
                prepared_persistent_bytes: prepared.receipt.memory.prepared_resident_bytes,
                retained_mask_bytes,
                rhs_export_bytes,
                solver: ModelSolverOptions {
                    pcg: PcgOptions {
                        tolerance: request.v2.v1.pcg_tolerance,
                        maximum_iterations: request.v2.v1.maximum_iterations,
                        residual_replacement_interval: request.v2.v1.residual_replacement_interval,
                    },
                    rank_tolerance: request.v2.v1.rank_tolerance,
                },
            },
            interrupt,
        )?;
        let mut performance = prepared.performance;
        performance.solve_ns = duration_ns(solve_start.elapsed());
        Ok(EngineSolved {
            result: EngineEstimate::GenericJla(result),
            algorithm_requested: VCKSS_ALGORITHM_JLA,
            algorithm_selected: VCKSS_ALGORITHM_JLA,
            engine_requested: VCKSS_ENGINE_GENERIC,
            engine_selected: VCKSS_ENGINE_GENERIC,
            seed: request.v2.v1.seed,
            probes: request.v2.v1.probes,
            leverage_batch_width: request.v2.v1.leverage_batch_width,
            target_batch_width: request.v2.v1.target_batch_width,
            nuisance,
            deletion,
            rank_tolerance: request.v2.v1.rank_tolerance,
            block_tolerance: request.v2.v1.block_tolerance,
            controls_count,
            capability_schema: request.capability_schema,
            capability_profile: request.capability_profile,
            batch_mode: request.batch_mode,
            stayers_mode: request.stayers_mode,
            target_weight_mode: request.target_weight_mode,
            deletion_unit_source: request.deletion_unit_source,
            probeorder_supplied: request.probeorder_supplied,
            wallseconds_supplied: request.wallseconds_supplied,
            frequency_use: request.frequency_use,
            physical_limit: request.physical_limit,
            request_signature: request.request_signature,
            execution_plan: None,
            full_cmg: None,
            preparation: prepared.receipt,
            retained: Arc::clone(&prepared.retained),
            stayer_augmentation: None,
            stayer_hybrid: None,
            performance,
        })
    })
}

fn solve_engine_v2(
    generation: u64,
    request: VckssEngineSolveRequestV2,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if request.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV2>()? {
        return Err(abi_error("V2 solve request reports a short structure size"));
    }
    let nuisance = nuisance_from_code(request.nuisance_mode)?;
    let requested_deletion = deletion_from_code(request.v1.deletion_mode)?;
    let algorithm_requested = algorithm_from_code(request.algorithm)?;
    let (exact_limit, blocksize_limit) = validate_exact_request_options(request)?;
    let handle = ContextHandle::from_generation(generation)?;
    let mut state = lock_engine("engine_solve")?;
    state.registry.solve_preserving(handle, |prepared| {
        let controls_count = u32::try_from(prepared.problem.controls.len()).map_err(|_| {
            resource_error("engine_solve", "control count is not representable as u32")
        })?;
        if prepared.deletion != requested_deletion {
            return Err(BackendError::invalid(
                "engine_solve",
                "solve deletion mode differs from the prepared graph mode",
            ));
        }
        let full_parameters = prepared
            .problem
            .workers()
            .checked_add(prepared.problem.firms() - 1)
            .and_then(|value| value.checked_add(prepared.problem.controls.len()))
            .ok_or_else(|| resource_error("engine_solve", "parameter count overflow"))?;
        let algorithm = match algorithm_requested {
            VCKSS_ALGORITHM_AUTO => {
                if full_parameters <= exact_limit {
                    VCKSS_ALGORITHM_EXACT
                } else {
                    VCKSS_ALGORITHM_JLA
                }
            }
            VCKSS_ALGORITHM_EXACT => VCKSS_ALGORITHM_EXACT,
            VCKSS_ALGORITHM_JLA => VCKSS_ALGORITHM_JLA,
            _ => unreachable!("algorithm code was validated before lifecycle transition"),
        };
        let solve_start = Instant::now();
        let result = if algorithm == VCKSS_ALGORITHM_EXACT {
            if full_parameters > exact_limit {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "exact_estimator",
                    "identified coefficient dimension exceeds exact_limit()",
                ));
            }
            if prepared.deletion == DeletionMode::Match
                && (0..prepared.problem.deletion_units()).any(|group| {
                    prepared.problem.deletion_index.range(group).len() > blocksize_limit
                })
            {
                return Err(BackendError::new(
                    ErrorCode::ResourceLimit,
                    "exact_estimator",
                    "a deletion block exceeds blocksize_limit()",
                ));
            }
            let exact = run_exact_estimator_with_interrupt(
                &prepared.problem,
                ExactEstimatorOptions {
                    deletion: prepared.deletion,
                    nuisance,
                    rank_tolerance: request.v1.rank_tolerance,
                    block_tolerance: request.v1.block_tolerance,
                    solver_tolerance: request.v1.pcg_tolerance,
                    exact_limit,
                    blocksize_limit,
                    memory_limit_bytes: if prepared.receipt.memory.hard_limit_bytes == 0 {
                        u64::MAX
                    } else {
                        prepared.receipt.memory.hard_limit_bytes
                    },
                    prepared_persistent_bytes: prepared.receipt.memory.prepared_resident_bytes,
                },
                interrupt,
            )?;
            EngineEstimate::Exact(exact)
        } else {
            if nuisance != NuisanceMode::Joint {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "engine_solve",
                    "the current Rust JLA path does not yet support fixed-offset nuisance controls",
                ));
            }
            let mut options = options_from_request(request.v1)?;
            if prepared.receipt.memory.hard_limit_bytes != 0 {
                options.memory_limit_bytes = prepared.receipt.memory.hard_limit_bytes;
                options.prepared_persistent_bytes = prepared.receipt.memory.prepared_resident_bytes;
            }
            let plan = prepared.plan.as_ref().ok_or_else(|| {
                BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "engine_solve",
                    "the current Rust JLA path requires a prepared no-control match plan",
                )
            })?;
            EngineEstimate::Jla(run_jla_no_controls_with_plan_and_interrupt(
                &prepared.problem,
                plan,
                options,
                interrupt,
            )?)
        };
        let mut performance = prepared.performance;
        performance.solve_ns = duration_ns(solve_start.elapsed());
        Ok(EngineSolved {
            result,
            algorithm_requested,
            algorithm_selected: algorithm,
            engine_requested: VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            engine_selected: if algorithm == VCKSS_ALGORITHM_JLA {
                VCKSS_ENGINE_COMPRESSED
            } else {
                VCKSS_ENGINE_NOT_APPLICABLE
            },
            seed: request.v1.seed,
            probes: request.v1.probes,
            leverage_batch_width: request.v1.leverage_batch_width,
            target_batch_width: request.v1.target_batch_width,
            nuisance,
            deletion: prepared.deletion,
            rank_tolerance: request.v1.rank_tolerance,
            block_tolerance: request.v1.block_tolerance,
            controls_count,
            capability_schema: 0,
            capability_profile: 0,
            batch_mode: 0,
            stayers_mode: 0,
            target_weight_mode: 0,
            deletion_unit_source: 0,
            probeorder_supplied: 0,
            wallseconds_supplied: 0,
            frequency_use: 0,
            physical_limit: 0,
            request_signature: 0,
            execution_plan: None,
            full_cmg: None,
            preparation: prepared.receipt,
            retained: Arc::clone(&prepared.retained),
            stayer_augmentation: None,
            stayer_hybrid: None,
            performance,
        })
    })
}

fn solve_engine(
    generation: u64,
    request: VckssEngineSolveRequestV1,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let options = options_from_request(request)?;
    let handle = ContextHandle::from_generation(generation)?;
    let mut state = lock_engine("engine_solve")?;
    state.registry.solve_preserving(handle, |prepared| {
        let controls_count = u32::try_from(prepared.problem.controls.len()).map_err(|_| {
            resource_error("engine_solve", "control count is not representable as u32")
        })?;
        let mut admitted_options = options;
        if prepared.receipt.memory.hard_limit_bytes != 0 {
            admitted_options.memory_limit_bytes = prepared.receipt.memory.hard_limit_bytes;
            admitted_options.prepared_persistent_bytes =
                prepared.receipt.memory.prepared_resident_bytes;
        } else {
            // The frozen V1 preparation surface never accepted a memory
            // envelope. Preserve that legacy behavior rather than silently
            // imposing the modern default admission limit.
            admitted_options.memory_limit_bytes = u64::MAX;
            admitted_options.prepared_persistent_bytes = 0;
        }
        let solve_start = Instant::now();
        let result = if prepared.receipt.memory.hard_limit_bytes == 0 {
            run_jla_no_controls_with_interrupt(&prepared.problem, admitted_options, interrupt)?
        } else {
            let plan = prepared.plan.as_ref().ok_or_else(|| {
                BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "engine_solve",
                    "the frozen V1 JLA solve requires a prepared no-control match plan",
                )
            })?;
            run_jla_no_controls_with_plan_and_interrupt(
                &prepared.problem,
                plan,
                admitted_options,
                interrupt,
            )?
        };
        let mut performance = prepared.performance;
        performance.solve_ns = duration_ns(solve_start.elapsed());
        Ok(EngineSolved {
            result: EngineEstimate::Jla(result),
            algorithm_requested: VCKSS_ALGORITHM_JLA,
            algorithm_selected: VCKSS_ALGORITHM_JLA,
            engine_requested: VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            engine_selected: VCKSS_ENGINE_COMPRESSED,
            seed: request.seed,
            probes: request.probes,
            leverage_batch_width: request.leverage_batch_width,
            target_batch_width: request.target_batch_width,
            nuisance: NuisanceMode::Joint,
            deletion: DeletionMode::Match,
            rank_tolerance: request.rank_tolerance,
            block_tolerance: request.block_tolerance,
            controls_count,
            capability_schema: 0,
            capability_profile: 0,
            batch_mode: 0,
            stayers_mode: 0,
            target_weight_mode: 0,
            deletion_unit_source: 0,
            probeorder_supplied: 0,
            wallseconds_supplied: 0,
            frequency_use: 0,
            physical_limit: 0,
            request_signature: 0,
            execution_plan: None,
            full_cmg: None,
            preparation: prepared.receipt,
            retained: Arc::clone(&prepared.retained),
            stayer_augmentation: None,
            stayer_hybrid: None,
            performance,
        })
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_result_v1(
    generation: u64,
    output: *mut VckssEngineResultV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineResultV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine result",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_result")?;
        let result = &state.registry.result(handle)?.result;
        let plugin = result.plugin();
        let correction = result.correction();
        let corrected = result.corrected();
        plugin.verify_accounting(1.0e-10)?;
        correction.verify_accounting(1.0e-10)?;
        corrected.verify_accounting(1.0e-10)?;
        write_output(
            output,
            VckssEngineResultV1 {
                struct_size: struct_size_u32::<VckssEngineResultV1>()?,
                reserved: 0,
                generation,
                plugin: component_vector(plugin),
                correction: component_vector(correction),
                corrected: component_vector(corrected),
                numerical_mcse: mcse_vector(result.numerical_mcse()),
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_stayer_hybrid_result_v1(
    generation: u64,
    output: *mut VckssStayerHybridResultV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssStayerHybridResultV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "stayer hybrid result",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_stayer_hybrid_result")?;
        let solved = state.registry.result(handle)?;
        let result = solved.stayer_hybrid.as_ref().ok_or_else(|| {
            BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_stayer_hybrid_result",
                "the solved generation has no stayer-hybrid result",
            )
        })?;
        write_output(output, stayer_hybrid_result_v1(generation, result)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v1(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let result = state.registry.result(handle)?.result.as_jla()?;
        let receipt = &result.receipt;
        write_output(output, detailed_receipt(generation, receipt)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v2(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV2,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV2>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V2 engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let solved = state.registry.result(handle)?;
        write_output(output, detailed_receipt_v2(generation, solved)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v3(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV3,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV3>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V3 engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let solved = state.registry.result(handle)?;
        write_output(output, detailed_receipt_v3(generation, solved)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v4(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV4,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV4>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V4 engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let solved = state.registry.result(handle)?;
        write_output(output, detailed_receipt_v4(generation, solved)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v5(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV5,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV5>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V5 engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let solved = state.registry.result(handle)?;
        write_output(output, detailed_receipt_v5(generation, solved)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v6(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV6,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV6>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V6 engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let solved = state.registry.result(handle)?;
        write_output(output, detailed_receipt_v6(generation, solved)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_execution_plan_receipt_v1(
    generation: u64,
    output: *mut VckssExecutionPlanReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssExecutionPlanReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine execution-plan receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_execution_plan_receipt")?;
        let solved = state.registry.result(handle)?;
        let plan = solved.execution_plan.ok_or_else(|| {
            BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_execution_plan_receipt",
                "the frozen execution-plan receipt is unavailable for a legacy solve",
            )
        })?;
        write_output(output, plan);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_detailed_receipt_v7(
    generation: u64,
    output: *mut VckssEngineDetailedReceiptV7,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineDetailedReceiptV7>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "V7 engine detailed receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_detailed_receipt")?;
        let solved = state.registry.result(handle)?;
        let execution = solved.execution_plan.ok_or_else(|| {
            BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_detailed_receipt",
                "the V7 execution-plan receipt is unavailable for a legacy solve",
            )
        })?;
        write_output(
            output,
            VckssEngineDetailedReceiptV7 {
                v6: detailed_receipt_v6(generation, solved)?,
                execution,
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_performance_receipt_v1(
    generation: u64,
    output: *mut VckssEnginePerformanceReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEnginePerformanceReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine performance receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_performance_receipt")?;
        let solved = state.registry.result(handle)?;
        let performance = solved.performance;
        let mut applicability_flags = 1_u64 | (1_u64 << 1);
        if performance.stayer_augmentation_ns != 0 {
            applicability_flags |= 1_u64 << 2;
        }
        write_output(
            output,
            VckssEnginePerformanceReceiptV1 {
                struct_size: struct_size_u32::<VckssEnginePerformanceReceiptV1>()?,
                schema_version: 1,
                generation,
                applicability_flags,
                algorithm_selected: solved.algorithm_selected,
                engine_selected: solved.engine_selected,
                ingest_ns: performance.ingest_ns,
                canonicalize_ns: performance.canonicalize_ns,
                graph_ns: performance.graph_ns,
                compress_ns: performance.compress_ns,
                plan_ns: performance.plan_ns,
                stayer_augmentation_ns: performance.stayer_augmentation_ns,
                solve_ns: performance.solve_ns,
                native_total_ns: performance.total_ns(),
            },
        );
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_full_cmg_receipt_v1(
    generation: u64,
    output: *mut VckssFullCmgReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssFullCmgReceiptV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "full-CMG receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_full_cmg_receipt")?;
        let solved = state.registry.result(handle)?;
        let receipt = solved.full_cmg.as_ref().ok_or_else(|| {
            BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_full_cmg_receipt",
                "the solved generation did not use CMG_FULL_V2",
            )
        })?;
        write_output(output, full_cmg_receipt_v1(generation, receipt)?);
        Ok(())
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_rhs_receipts_v1(
    generation: u64,
    output: *mut VckssEngineRhsReceiptV1,
    output_capacity_rows: u64,
) -> i32 {
    ffi_status(|| {
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_rhs_receipts")?;
        let result = state.registry.result(handle)?.result.as_jla()?;
        let receipt = &result.receipt;
        let required = rhs_receipt_count(receipt)?;
        if output.is_null() || output_capacity_rows < required {
            return Err(BackendError::invalid(
                "engine_rhs_receipts",
                format!(
                    "RHS receipt row capacity {output_capacity_rows} is below required size {required}"
                ),
            ));
        }
        let mut row = 0_usize;
        write_rhs_receipt(output, row, &receipt.full_fit)?;
        row += 1;
        for value in &receipt.leverage_rhs {
            write_rhs_receipt(output, row, value)?;
            row += 1;
        }
        for value in &receipt.target_rhs {
            write_rhs_receipt(output, row, value)?;
            row += 1;
        }
        if u64::try_from(row).map_err(|_| {
            resource_error(
                "engine_rhs_receipts",
                "RHS receipt row count is not representable",
            )
        })? != required
        {
            return Err(BackendError::invariant(
                "engine_rhs_receipts",
                "RHS receipt export count changed during serialization",
            ));
        }
        Ok(())
    })
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_rhs_receipts_v2(
    generation: u64,
    output: *mut VckssEngineRhsReceiptV2,
    output_capacity_rows: u64,
) -> i32 {
    ffi_status(|| {
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_rhs_receipts")?;
        let solved = state.registry.result(handle)?;
        let EngineEstimate::GenericJla(result) = &solved.result else {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "engine_rhs_receipts",
                "RHS receipt V2 is available only for generic JLA",
            ));
        };
        let required = to_u64(result.receipt.rhs.len(), "generic RHS receipt count")?;
        if output.is_null() || output_capacity_rows < required {
            return Err(BackendError::invalid(
                "engine_rhs_receipts",
                format!(
                    "RHS V2 receipt row capacity {output_capacity_rows} is below required size {required}"
                ),
            ));
        }
        for (row, value) in result.receipt.rhs.iter().enumerate() {
            let exported = generic_rhs_receipt_value(solved, result, value)?;
            // SAFETY: the caller supplied a non-null buffer with at least the
            // validated number of rows; each row is written exactly once.
            unsafe { output.add(row).write(exported) };
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_release_v1(generation: u64) -> i32 {
    ffi_status(|| {
        let handle = ContextHandle::from_generation(generation)?;
        lock_engine("engine_release")?.registry.release(handle)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_clear_abandoned_v1() -> i32 {
    ffi_status(|| {
        lock_engine("engine_clear")?.registry.clear_abandoned();
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_snapshot_v1(
    output: *mut VckssEngineSnapshotV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSnapshotV1>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "engine snapshot",
        )?;
        let state = lock_engine("engine_snapshot")?;
        let snapshot = state.registry.snapshot();
        write_output(
            output,
            VckssEngineSnapshotV1 {
                struct_size: struct_size_u32::<VckssEngineSnapshotV1>()?,
                state: state_code(snapshot.state),
                generation: snapshot.generation.unwrap_or(0),
                last_released_generation: snapshot.last_released_generation,
            },
        );
        Ok(())
    })
}

/// Valid until the next engine ABI call on the same thread. Calls on another
/// thread cannot invalidate this pointer.
#[no_mangle]
pub extern "C" fn vckss_rust_engine_last_error() -> *const c_char {
    ENGINE_LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

/// Compatibility aliases for the original ABI-1 session surface.  The old
/// signatures predated explicit output capacities, so the frozen structure
/// sizes are supplied here while all state and error transport remain owned by
/// the generation-safe engine registry.
#[no_mangle]
pub extern "C" fn vckss_rust_session_prepare_v1(
    request: *const VckssPrepareRequestV1,
    columns: *const VckssColumnsV1,
    output_handle: *mut u64,
) -> i32 {
    vckss_rust_engine_prepare_v1(request, columns, output_handle, 8_u32)
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_preparation_receipt_v1(
    generation: u64,
    output: *mut VckssPreparationReceiptV1,
) -> i32 {
    vckss_rust_engine_preparation_receipt_v1(generation, output, 72_u32)
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_release_v1(generation: u64) -> i32 {
    vckss_rust_engine_release_v1(generation)
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_clear_abandoned_v1() -> i32 {
    vckss_rust_engine_clear_abandoned_v1()
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_snapshot_v1(output: *mut VckssSessionSnapshotV1) -> i32 {
    vckss_rust_engine_snapshot_v1(output, 24_u32)
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_last_error() -> *const c_char {
    vckss_rust_engine_last_error()
}

fn options_from_request(request: VckssEngineSolveRequestV1) -> Result<JlaEngineOptions> {
    if request.deletion_mode != VCKSS_DELETION_MATCH {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "the production Rust engine currently supports match deletion only",
        ));
    }
    if request.rng_contract != VCKSS_RNG_COUNTER_V1 {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "the production Rust engine currently supports Counter-V1 RNG only",
        ));
    }
    if request.allow_automatic_cmg_setup_fallback > 1 {
        return Err(BackendError::invalid(
            "engine_solve",
            "allow_automatic_cmg_setup_fallback must be zero or one",
        ));
    }
    let pcg = PcgOptions {
        tolerance: request.pcg_tolerance,
        maximum_iterations: request.maximum_iterations,
        residual_replacement_interval: request.residual_replacement_interval,
    };
    let solver = LinearSolverOptions {
        route: route_from_code(request.solver_route)?,
        exact_dimension_limit: to_usize(
            request.exact_dimension_limit,
            "engine_solve",
            "exact dimension limit",
        )?,
        cmg_minimum_dimension: to_usize(
            request.cmg_minimum_dimension,
            "engine_solve",
            "CMG minimum dimension",
        )?,
        allow_automatic_cmg_setup_fallback: request.allow_automatic_cmg_setup_fallback == 1,
        pcg,
        cmg: CmgOptions {
            terminal_vertices: to_usize(
                request.cmg_terminal_vertices,
                "engine_solve",
                "CMG terminal vertices",
            )?,
            dense_vertex_cap: to_usize(
                request.cmg_dense_vertex_cap,
                "engine_solve",
                "CMG dense vertex cap",
            )?,
            maximum_levels: to_usize(
                request.cmg_maximum_levels,
                "engine_solve",
                "CMG maximum levels",
            )?,
            aggregate_cap: to_usize(
                request.cmg_aggregate_cap,
                "engine_solve",
                "CMG aggregate cap",
            )?,
            minimum_reduction: request.cmg_minimum_reduction,
            jacobi_weight: request.cmg_jacobi_weight,
            pre_sweeps: request.cmg_pre_sweeps,
            post_sweeps: request.cmg_post_sweeps,
            maximum_edge_complexity: request.cmg_maximum_edge_complexity,
            maximum_vertex_complexity: request.cmg_maximum_vertex_complexity,
            memory_limit_bytes: request.cmg_memory_limit_bytes,
        },
        full_residual_tolerance: (10.0 * request.pcg_tolerance).max(1.0e-11),
    };
    Ok(JlaEngineOptions {
        seed: request.seed,
        probes: request.probes,
        leverage_batch_width: usize::try_from(request.leverage_batch_width)
            .map_err(|_| resource_error("engine_solve", "leverage width is not representable"))?,
        target_batch_width: usize::try_from(request.target_batch_width)
            .map_err(|_| resource_error("engine_solve", "target width is not representable"))?,
        deletion: DeletionMode::Match,
        rng: RngContract::CounterV1,
        rank_tolerance: request.rank_tolerance,
        block_tolerance: request.block_tolerance,
        memory_limit_bytes: JlaEngineOptions::default().memory_limit_bytes,
        prepared_persistent_bytes: 0,
        solver,
    })
}

fn validate_prepare_request_v2(
    request: VckssEnginePrepareRequestV2,
) -> Result<PreparationMemoryReceipt> {
    require_abi(request.abi_version)?;
    if request.reserved != 0 {
        return Err(abi_error("reserved V2 preparation fields must be zero"));
    }
    if request.cleanup_abandoned > 1 {
        return Err(BackendError::invalid(
            "engine_prepare",
            "cleanup_abandoned must be zero or one",
        ));
    }
    admit_prepare_memory(
        request.rows,
        request.memory_limit_bytes,
        request.caller_copy_bytes,
    )
}

fn validate_prepare_request_v3(
    request: VckssEnginePrepareRequestV3,
) -> Result<(DeletionMode, PreparationMemoryReceipt)> {
    validate_prepare_request_v3_with_probe_order(request, false)
}

fn validate_prepare_request_v3_with_probe_order(
    request: VckssEnginePrepareRequestV3,
    probeorder_supplied: bool,
) -> Result<(DeletionMode, PreparationMemoryReceipt)> {
    require_abi(request.v2.abi_version)?;
    if request.v2.struct_size < struct_size_u32::<VckssEnginePrepareRequestV3>()? {
        return Err(abi_error(
            "V3 prepare request reports a short structure size",
        ));
    }
    if request.v2.reserved != 0 || request.reserved_2 != 0 {
        return Err(abi_error("reserved V3 preparation fields must be zero"));
    }
    if request.v2.cleanup_abandoned > 1 {
        return Err(BackendError::invalid(
            "engine_prepare",
            "cleanup_abandoned must be zero or one",
        ));
    }
    let deletion = deletion_from_code(request.deletion_mode)?;
    let memory = admit_prepare_memory_with_controls_and_probe_order(
        request.v2.rows,
        request.controls_count,
        probeorder_supplied,
        request.v2.memory_limit_bytes,
        request.v2.caller_copy_bytes,
    )?;
    Ok((deletion, memory))
}

fn validate_prepare_request_v4(
    request: VckssEnginePrepareRequestV4,
    probeorder_supplied: bool,
) -> Result<(DeletionMode, bool, PreparationMemoryReceipt)> {
    require_abi(request.v3.v2.abi_version)?;
    if request.v3.v2.struct_size < struct_size_u32::<VckssEnginePrepareRequestV4>()? {
        return Err(abi_error(
            "V4 prepare request reports a short structure size",
        ));
    }
    if request.v3.v2.reserved != 0
        || request.v3.reserved_2 != 0
        || request.reserved_3 != 0
        || request.implicit_match > 1
    {
        return Err(abi_error(
            "reserved or boolean V4 preparation fields are invalid",
        ));
    }
    if request.v3.v2.cleanup_abandoned > 1 {
        return Err(BackendError::invalid(
            "engine_prepare",
            "cleanup_abandoned must be zero or one",
        ));
    }
    let deletion = deletion_from_code(request.v3.deletion_mode)?;
    let implicit_match = request.implicit_match == 1;
    if implicit_match
        && (deletion != DeletionMode::Match
            || request.v3.controls_count != 0
            || !probeorder_supplied)
    {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_prepare",
            "implicit-match preparation requires match deletion, no controls, and an explicit probe order",
        ));
    }
    let memory = admit_prepare_memory_with_controls_probe_order_and_implicit_match(
        request.v3.v2.rows,
        request.v3.controls_count,
        probeorder_supplied,
        implicit_match,
        request.v3.v2.memory_limit_bytes,
        request.v3.v2.caller_copy_bytes,
    )?;
    Ok((deletion, implicit_match, memory))
}

fn detailed_receipt(
    generation: u64,
    receipt: &vckss_core::engine::JlaEngineReceipt,
) -> Result<VckssEngineDetailedReceiptV1> {
    let (fallback, fallback_error) = receipt
        .solver
        .fallback
        .as_ref()
        .map_or((0, 0), |value| (1, value.code as i32));
    let cmg = receipt.solver.cmg.as_ref();
    Ok(VckssEngineDetailedReceiptV1 {
        struct_size: struct_size_u32::<VckssEngineDetailedReceiptV1>()?,
        reserved: 0,
        generation,
        seed: receipt.seed,
        probes_requested: receipt.probes_requested,
        leverage_probes_accepted: receipt.leverage_probes_accepted,
        target_probes_accepted: receipt.target_probes_accepted,
        solver_requested: route_code(receipt.solver.requested),
        solver_selected: route_code(receipt.solver.selected),
        solver_fallback: fallback,
        solver_fallback_error: fallback_error,
        solver_dimension: to_u64(receipt.solver.dimension, "solver dimension")?,
        leverage_batch_width: to_u64(receipt.leverage_batch_width, "leverage batch width")?,
        target_batch_width: to_u64(receipt.target_batch_width, "target batch width")?,
        rank_tolerance: receipt.rank_tolerance,
        block_tolerance: receipt.block_tolerance,
        full_residual_tolerance: receipt.full_residual_tolerance,
        full_fit_route: route_code(receipt.full_fit.route),
        full_fit_iterations: receipt.full_fit.iterations,
        full_fit_reduced_residual: receipt.full_fit.reduced_residual,
        full_fit_complete_residual: receipt.full_fit.complete_residual,
        full_fit_zero_rhs: u32::from(receipt.full_fit.zero_rhs),
        reserved_1: 0,
        leverage_rhs_count: to_u64(receipt.leverage_rhs.len(), "leverage RHS count")?,
        target_rhs_count: to_u64(receipt.target_rhs.len(), "target RHS count")?,
        max_reduced_residual: receipt.max_reduced_residual,
        max_complete_residual: receipt.max_complete_residual,
        max_leverage: receipt.max_leverage,
        max_reciprocal_residual: receipt.max_reciprocal_residual,
        accounting_residual: receipt.accounting_residual,
        topology_checksum: receipt.topology_checksum,
        cmg_levels: option_usize(cmg.map(|value| value.levels), "CMG levels")?,
        cmg_fine_vertices: option_usize(cmg.map(|value| value.fine_vertices), "CMG vertices")?,
        cmg_fine_edges: option_usize(cmg.map(|value| value.fine_edges), "CMG edges")?,
        cmg_terminal_vertices: option_usize(
            cmg.map(|value| value.terminal_vertices),
            "CMG terminal vertices",
        )?,
        cmg_edge_complexity: cmg.map_or(0.0, |value| value.edge_complexity),
        cmg_vertex_complexity: cmg.map_or(0.0, |value| value.vertex_complexity),
        cmg_structural_bytes: cmg.map_or(0, |value| value.structural_bytes),
        cmg_workspace_bytes: cmg.map_or(0, |value| value.workspace_bytes),
        cmg_dense_factor_bytes: cmg.map_or(0, |value| value.dense_factor_bytes),
    })
}

fn generic_detailed_receipt(
    generation: u64,
    solved: &EngineSolved,
    result: &GenericJlaResult,
) -> Result<VckssEngineDetailedReceiptV1> {
    let receipt = &result.receipt;
    let full_fit = receipt
        .rhs
        .iter()
        .find(|value| value.phase == GenericJlaRhsPhase::FullJointFit)
        .ok_or_else(|| {
            BackendError::invariant(
                "engine_detailed_receipt",
                "generic-JLA full-fit RHS receipt is absent",
            )
        })?;
    let solver_dimension = solved
        .preparation
        .firms
        .checked_add(u64::from(solved.controls_count))
        .ok_or_else(|| resource_error("engine_detailed_receipt", "solver dimension overflow"))?;
    Ok(VckssEngineDetailedReceiptV1 {
        struct_size: struct_size_u32::<VckssEngineDetailedReceiptV1>()?,
        reserved: 0,
        generation,
        seed: solved.seed,
        probes_requested: solved.probes,
        leverage_probes_accepted: receipt.leverage_probes_accepted,
        target_probes_accepted: receipt.target_probes_accepted,
        solver_requested: VCKSS_ROUTE_DIAGONAL_PCG,
        solver_selected: VCKSS_ROUTE_DIAGONAL_PCG,
        solver_fallback: 0,
        solver_fallback_error: 0,
        solver_dimension,
        leverage_batch_width: u64::from(solved.leverage_batch_width),
        target_batch_width: u64::from(solved.target_batch_width),
        rank_tolerance: solved.rank_tolerance,
        block_tolerance: solved.block_tolerance,
        full_residual_tolerance: receipt.full_residual_tolerance,
        full_fit_route: VCKSS_ROUTE_DIAGONAL_PCG,
        full_fit_iterations: full_fit.pcg.iterations,
        full_fit_reduced_residual: full_fit.pcg.relative_residual,
        full_fit_complete_residual: receipt.full_fit_complete_residual,
        full_fit_zero_rhs: u32::from(full_fit.pcg.status == ModelPcgStatus::ZeroRhs),
        reserved_1: 0,
        leverage_rhs_count: to_u64(
            receipt
                .rhs
                .iter()
                .filter(|value| value.phase == GenericJlaRhsPhase::Leverage)
                .count(),
            "generic leverage RHS count",
        )?,
        target_rhs_count: to_u64(
            receipt
                .rhs
                .iter()
                .filter(|value| value.phase == GenericJlaRhsPhase::Target)
                .count(),
            "generic target RHS count",
        )?,
        max_reduced_residual: receipt.maximum_reduced_residual,
        max_complete_residual: receipt.maximum_complete_residual,
        max_leverage: receipt.maximum_leverage,
        max_reciprocal_residual: receipt.maximum_maker_relres,
        accounting_residual: result_accounting_residual(&solved.result),
        topology_checksum: receipt.topology_checksum,
        cmg_levels: 0,
        cmg_fine_vertices: 0,
        cmg_fine_edges: 0,
        cmg_terminal_vertices: 0,
        cmg_edge_complexity: 0.0,
        cmg_vertex_complexity: 0.0,
        cmg_structural_bytes: 0,
        cmg_workspace_bytes: 0,
        cmg_dense_factor_bytes: 0,
    })
}

fn detailed_receipt_v2(
    generation: u64,
    solved: &EngineSolved,
) -> Result<VckssEngineDetailedReceiptV2> {
    let (base, solver_setup, leverage_phase, target_phase, result_forecast, solve_peak) =
        match &solved.result {
            EngineEstimate::Jla(result) => {
                let memory = result.receipt.memory;
                (
                    detailed_receipt(generation, &result.receipt)?,
                    memory.solver_setup_forecast_bytes,
                    memory.leverage_phase_forecast_bytes,
                    memory.target_phase_forecast_bytes,
                    memory.result_forecast_bytes,
                    memory.solve_peak_forecast_bytes,
                )
            }
            EngineEstimate::GenericJla(result) => {
                let receipt = &result.receipt;
                (
                    generic_detailed_receipt(generation, solved, result)?,
                    receipt
                        .canonicalization_peak_forecast_bytes
                        .max(receipt.fit_peak_forecast_bytes)
                        .max(receipt.geometry_peak_forecast_bytes),
                    receipt.leverage_peak_forecast_bytes,
                    receipt.target_peak_forecast_bytes,
                    receipt.result_forecast_bytes,
                    receipt.peak_forecast_bytes,
                )
            }
            EngineEstimate::Exact(_) => {
                return Err(BackendError::new(
                    ErrorCode::UnsupportedFeature,
                    "engine_detailed_receipt",
                    "the frozen JLA V2 receipt is unavailable for an exact estimator result",
                ));
            }
        };
    let preparation_memory = solved.preparation.memory;
    Ok(VckssEngineDetailedReceiptV2 {
        struct_size: struct_size_u32::<VckssEngineDetailedReceiptV2>()?,
        reserved: base.reserved,
        generation: base.generation,
        seed: base.seed,
        probes_requested: base.probes_requested,
        leverage_probes_accepted: base.leverage_probes_accepted,
        target_probes_accepted: base.target_probes_accepted,
        solver_requested: base.solver_requested,
        solver_selected: base.solver_selected,
        solver_fallback: base.solver_fallback,
        solver_fallback_error: base.solver_fallback_error,
        solver_dimension: base.solver_dimension,
        leverage_batch_width: base.leverage_batch_width,
        target_batch_width: base.target_batch_width,
        rank_tolerance: base.rank_tolerance,
        block_tolerance: base.block_tolerance,
        full_residual_tolerance: base.full_residual_tolerance,
        full_fit_route: base.full_fit_route,
        full_fit_iterations: base.full_fit_iterations,
        full_fit_reduced_residual: base.full_fit_reduced_residual,
        full_fit_complete_residual: base.full_fit_complete_residual,
        full_fit_zero_rhs: base.full_fit_zero_rhs,
        reserved_1: base.reserved_1,
        leverage_rhs_count: base.leverage_rhs_count,
        target_rhs_count: base.target_rhs_count,
        max_reduced_residual: base.max_reduced_residual,
        max_complete_residual: base.max_complete_residual,
        max_leverage: base.max_leverage,
        max_reciprocal_residual: base.max_reciprocal_residual,
        accounting_residual: base.accounting_residual,
        topology_checksum: base.topology_checksum,
        cmg_levels: base.cmg_levels,
        cmg_fine_vertices: base.cmg_fine_vertices,
        cmg_fine_edges: base.cmg_fine_edges,
        cmg_terminal_vertices: base.cmg_terminal_vertices,
        cmg_edge_complexity: base.cmg_edge_complexity,
        cmg_vertex_complexity: base.cmg_vertex_complexity,
        cmg_structural_bytes: base.cmg_structural_bytes,
        cmg_workspace_bytes: base.cmg_workspace_bytes,
        cmg_dense_factor_bytes: base.cmg_dense_factor_bytes,
        full_fit_weighted_rss: solved.result.weighted_rss(),
        memory_limit_bytes: preparation_memory.hard_limit_bytes,
        caller_copy_bytes: preparation_memory.caller_copy_bytes,
        preparation_peak_forecast_bytes: preparation_memory.preparation_peak_forecast_bytes,
        prepared_resident_bytes: preparation_memory.prepared_resident_bytes,
        solver_setup_forecast_bytes: solver_setup,
        leverage_phase_forecast_bytes: leverage_phase,
        target_phase_forecast_bytes: target_phase,
        result_forecast_bytes: result_forecast,
        solve_peak_forecast_bytes: solve_peak,
        command_peak_forecast_bytes: preparation_memory
            .preparation_peak_forecast_bytes
            .max(solve_peak),
    })
}

fn apply_prepared_memory_admission(
    options: &mut JlaEngineOptions,
    memory_limit_bytes: u64,
    prepared_persistent_bytes: u64,
    full_cmg_v2: bool,
) {
    options.memory_limit_bytes = memory_limit_bytes;
    options.prepared_persistent_bytes = prepared_persistent_bytes;
    if full_cmg_v2 {
        // The direct solver consumes the CMG sub-option while the estimator
        // admission consumes the command-level option. They must carry the
        // same already-reconciled prepared limit; otherwise the legacy 2 GiB
        // CMG default can reject a production command admitted at a larger
        // memory_gib() ceiling.
        options.solver.cmg.memory_limit_bytes = memory_limit_bytes;
    }
}

fn detailed_receipt_v3(
    generation: u64,
    solved: &EngineSolved,
) -> Result<VckssEngineDetailedReceiptV3> {
    match &solved.result {
        EngineEstimate::Jla(result) => {
            let rows = rhs_receipt_count(&result.receipt)?;
            let caller_result_copy_bytes = rows
                .checked_mul(14)
                .and_then(|value| value.checked_mul(8))
                .ok_or_else(|| {
                    resource_error(
                        "engine_detailed_receipt",
                        "caller RHS receipt matrix byte count overflow",
                    )
                })?;
            Ok(VckssEngineDetailedReceiptV3 {
                v2: detailed_receipt_v2(generation, solved)?,
                rng_contract: rng_contract_code(result.receipt.rng),
                reserved_2: 0,
                rhs_receipt_rows: rows,
                caller_result_copy_bytes,
            })
        }
        EngineEstimate::GenericJla(result) => {
            let rows = to_u64(result.receipt.rhs.len(), "generic RHS receipt count")?;
            Ok(VckssEngineDetailedReceiptV3 {
                v2: detailed_receipt_v2(generation, solved)?,
                rng_contract: VCKSS_RNG_COUNTER_V1,
                reserved_2: 0,
                rhs_receipt_rows: rows,
                // The frozen caller-copy field describes the V1 RHS export,
                // which intentionally rejects generic JLA. V6 reports the
                // lossless V2 export footprint instead.
                caller_result_copy_bytes: 0,
            })
        }
        EngineEstimate::Exact(result) => {
            let preparation = solved.preparation.memory;
            let prepared_resident_bytes = solved
                .stayer_augmentation
                .map_or(preparation.prepared_resident_bytes, |augmentation| {
                    augmentation.memory.total_prepared_resident_bytes
                });
            let exact = &result.receipt;
            let maximum_fit_residual = exact.full_fit_relres.max(exact.working_fit_relres);
            let base = VckssEngineDetailedReceiptV1 {
                struct_size: struct_size_u32::<VckssEngineDetailedReceiptV1>()?,
                reserved: 0,
                generation,
                seed: 0,
                probes_requested: 0,
                leverage_probes_accepted: 0,
                target_probes_accepted: 0,
                solver_requested: VCKSS_ROUTE_EXACT,
                solver_selected: VCKSS_ROUTE_EXACT,
                solver_fallback: 0,
                solver_fallback_error: 0,
                solver_dimension: to_u64(exact.parameters, "exact dimension")?,
                leverage_batch_width: 0,
                target_batch_width: 0,
                rank_tolerance: solved.rank_tolerance,
                block_tolerance: solved.block_tolerance,
                full_residual_tolerance: exact.fit_residual_tolerance,
                full_fit_route: VCKSS_ROUTE_EXACT,
                full_fit_iterations: 0,
                // The exact estimator does not solve the JLA reduced system.
                // Mirror its stronger complete original-system certificate in
                // the frozen reduced field rather than inventing or aliasing
                // an unrelated inverse residual.
                full_fit_reduced_residual: exact.full_fit_relres,
                full_fit_complete_residual: exact.full_fit_relres,
                full_fit_zero_rhs: 0,
                reserved_1: 0,
                leverage_rhs_count: 0,
                target_rhs_count: 0,
                max_reduced_residual: maximum_fit_residual,
                max_complete_residual: maximum_fit_residual,
                max_leverage: exact.max_leverage,
                max_reciprocal_residual: exact.maker_relres,
                accounting_residual: 0.0,
                topology_checksum: exact.topology_checksum,
                cmg_levels: 0,
                cmg_fine_vertices: 0,
                cmg_fine_edges: 0,
                cmg_terminal_vertices: 0,
                cmg_edge_complexity: 0.0,
                cmg_vertex_complexity: 0.0,
                cmg_structural_bytes: 0,
                cmg_workspace_bytes: 0,
                cmg_dense_factor_bytes: 0,
            };
            let v2 = VckssEngineDetailedReceiptV2 {
                struct_size: struct_size_u32::<VckssEngineDetailedReceiptV2>()?,
                reserved: base.reserved,
                generation,
                seed: base.seed,
                probes_requested: base.probes_requested,
                leverage_probes_accepted: base.leverage_probes_accepted,
                target_probes_accepted: base.target_probes_accepted,
                solver_requested: base.solver_requested,
                solver_selected: base.solver_selected,
                solver_fallback: base.solver_fallback,
                solver_fallback_error: base.solver_fallback_error,
                solver_dimension: base.solver_dimension,
                leverage_batch_width: 0,
                target_batch_width: 0,
                rank_tolerance: base.rank_tolerance,
                block_tolerance: base.block_tolerance,
                full_residual_tolerance: base.full_residual_tolerance,
                full_fit_route: base.full_fit_route,
                full_fit_iterations: 0,
                full_fit_reduced_residual: base.full_fit_reduced_residual,
                full_fit_complete_residual: base.full_fit_complete_residual,
                full_fit_zero_rhs: 0,
                reserved_1: 0,
                leverage_rhs_count: 0,
                target_rhs_count: 0,
                max_reduced_residual: base.max_reduced_residual,
                max_complete_residual: base.max_complete_residual,
                max_leverage: base.max_leverage,
                max_reciprocal_residual: base.max_reciprocal_residual,
                accounting_residual: 0.0,
                topology_checksum: base.topology_checksum,
                cmg_levels: 0,
                cmg_fine_vertices: 0,
                cmg_fine_edges: 0,
                cmg_terminal_vertices: 0,
                cmg_edge_complexity: 0.0,
                cmg_vertex_complexity: 0.0,
                cmg_structural_bytes: 0,
                cmg_workspace_bytes: 0,
                cmg_dense_factor_bytes: 0,
                full_fit_weighted_rss: result.weighted_rss,
                memory_limit_bytes: preparation.hard_limit_bytes,
                caller_copy_bytes: preparation.caller_copy_bytes,
                preparation_peak_forecast_bytes: preparation.preparation_peak_forecast_bytes,
                prepared_resident_bytes,
                solver_setup_forecast_bytes: 0,
                leverage_phase_forecast_bytes: 0,
                target_phase_forecast_bytes: 0,
                result_forecast_bytes: u64::try_from(size_of::<ExactEstimatorResult>())
                    .map_err(|_| resource_error("engine_detailed_receipt", "result size"))?,
                solve_peak_forecast_bytes: exact.peak_forecast_bytes,
                command_peak_forecast_bytes: preparation
                    .preparation_peak_forecast_bytes
                    .max(exact.peak_forecast_bytes),
            };
            Ok(VckssEngineDetailedReceiptV3 {
                v2,
                rng_contract: 0,
                reserved_2: 0,
                rhs_receipt_rows: 0,
                caller_result_copy_bytes: 0,
            })
        }
    }
}

fn detailed_receipt_v4(
    generation: u64,
    solved: &EngineSolved,
) -> Result<VckssEngineDetailedReceiptV4> {
    let (
        parameters,
        full_parameters,
        correction_parameters,
        information_rcond,
        inverse_relres,
        exact_peak_forecast_bytes,
    ) = match &solved.result {
        EngineEstimate::Exact(value) => (
            to_u64(value.receipt.parameters, "exact parameters")?,
            to_u64(value.receipt.full_parameters, "exact full parameters")?,
            to_u64(
                value.receipt.correction_parameters,
                "exact correction parameters",
            )?,
            value.receipt.information_rcond,
            value.receipt.inverse_relres,
            value.receipt.peak_forecast_bytes,
        ),
        EngineEstimate::Jla(_) => {
            let parameters = solved
                .preparation
                .workers
                .checked_add(solved.preparation.firms)
                .and_then(|value| value.checked_sub(1))
                .ok_or_else(|| resource_error("engine_detailed_receipt", "parameter count"))?;
            (parameters, parameters, parameters, 0.0, 0.0, 0)
        }
        EngineEstimate::GenericJla(value) => (
            to_u64(value.receipt.parameters, "generic parameters")?,
            to_u64(value.receipt.full_parameters, "generic full parameters")?,
            to_u64(
                value.receipt.correction_parameters,
                "generic correction parameters",
            )?,
            0.0,
            0.0,
            0,
        ),
    };
    Ok(VckssEngineDetailedReceiptV4 {
        v3: detailed_receipt_v3(generation, solved)?,
        algorithm_requested: solved.algorithm_requested,
        algorithm_selected: solved.algorithm_selected,
        deletion_mode: deletion_code(solved.deletion),
        nuisance_mode: nuisance_code(solved.nuisance),
        parameters,
        full_parameters,
        correction_parameters,
        information_rcond,
        inverse_relres,
        exact_peak_forecast_bytes,
    })
}

fn detailed_receipt_v5(
    generation: u64,
    solved: &EngineSolved,
) -> Result<VckssEngineDetailedReceiptV5> {
    let actual_accounting_residual = result_accounting_residual(&solved.result);
    match &solved.result {
        EngineEstimate::Exact(value) => {
            let exact = &value.receipt;
            let mut applicability_flags = VCKSS_EXACT_DIAGNOSTIC_WORKING_FIT
                | VCKSS_EXACT_DIAGNOSTIC_DELETION_RANK
                | VCKSS_EXACT_DIAGNOSTIC_FIRM_ZERO_SUM
                | VCKSS_EXACT_DIAGNOSTIC_FIT_PEAK
                | VCKSS_EXACT_DIAGNOSTIC_CORRECTION_PEAK
                | VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING;
            if solved.deletion == DeletionMode::Match {
                applicability_flags |=
                    VCKSS_EXACT_DIAGNOSTIC_INVERSE_SQRT | VCKSS_EXACT_DIAGNOSTIC_MAKER;
            }
            if solved.controls_count != 0 {
                applicability_flags |= VCKSS_EXACT_DIAGNOSTIC_CONTROL_BASIS;
            }
            Ok(VckssEngineDetailedReceiptV5 {
                v4: detailed_receipt_v4(generation, solved)?,
                applicability_flags,
                working_fit_complete_residual: exact.working_fit_relres,
                inverse_sqrt_relres: exact.inverse_sqrt_relres,
                maker_relres: exact.maker_relres,
                control_basis_relres: exact.control_basis_relres,
                control_basis_forward_error: exact.control_basis_forward_error,
                deletion_rank_gap: exact.deletion_rank_gap,
                firm_zero_sum_residual: exact.firm_zero_sum_residual,
                fit_peak_forecast_bytes: exact.fit_peak_forecast_bytes,
                correction_peak_forecast_bytes: exact.correction_peak_forecast_bytes,
                actual_accounting_residual,
            })
        }
        EngineEstimate::Jla(_) | EngineEstimate::GenericJla(_) => {
            Ok(VckssEngineDetailedReceiptV5 {
                v4: detailed_receipt_v4(generation, solved)?,
                applicability_flags: VCKSS_DIAGNOSTIC_ACTUAL_ACCOUNTING,
                actual_accounting_residual,
                ..VckssEngineDetailedReceiptV5::default()
            })
        }
    }
}

fn detailed_receipt_v6(
    generation: u64,
    solved: &EngineSolved,
) -> Result<VckssEngineDetailedReceiptV6> {
    let v5 = detailed_receipt_v5(generation, solved)?;
    let EngineEstimate::GenericJla(result) = &solved.result else {
        return Ok(VckssEngineDetailedReceiptV6 {
            v5,
            engine_requested: solved.engine_requested,
            engine_selected: solved.engine_selected,
            controls_count: solved.controls_count,
            rhs_receipt_schema: if matches!(solved.result, EngineEstimate::Jla(_)) {
                1
            } else {
                0
            },
            capability_schema: solved.capability_schema,
            capability_profile: solved.capability_profile,
            batch_mode: solved.batch_mode,
            stayers_mode: solved.stayers_mode,
            target_weight_mode: solved.target_weight_mode,
            deletion_unit_source: solved.deletion_unit_source,
            probeorder_supplied: solved.probeorder_supplied,
            wallseconds_supplied: solved.wallseconds_supplied,
            frequency_use: solved.frequency_use,
            physical_limit: solved.physical_limit,
            request_signature: solved.request_signature,
            ..VckssEngineDetailedReceiptV6::default()
        });
    };
    let receipt = &result.receipt;
    let rank = &receipt.control_rank;
    let mut flags = VCKSS_GENERIC_DIAGNOSTIC_DELETION_RANK
        | VCKSS_GENERIC_DIAGNOSTIC_FULL_JOINT_FIT
        | VCKSS_GENERIC_DIAGNOSTIC_WORKING_FIT
        | VCKSS_GENERIC_DIAGNOSTIC_MAKER
        | VCKSS_GENERIC_DIAGNOSTIC_MEMORY_PHASES
        | VCKSS_GENERIC_DIAGNOSTIC_RHS_V2;
    if solved.controls_count != 0 {
        flags |= VCKSS_GENERIC_DIAGNOSTIC_CONTROL_RANK;
    }
    let rhs_rows = to_u64(receipt.rhs.len(), "generic RHS receipt count")?;
    let (expected_rows, expected_export_bytes) =
        generic_rhs_export_memory(solved.controls_count, solved.probes, solved.nuisance)?;
    let expected_mask_bytes = to_u64(
        bit_packed_capacity_bytes(solved.retained.capacity()),
        "bit-packed retained-mask capacity",
    )?;
    if rhs_rows != expected_rows
        || receipt.rhs_export_bytes != expected_export_bytes
        || receipt.retained_mask_bytes != expected_mask_bytes
    {
        return Err(BackendError::invariant(
            "engine_detailed_receipt",
            "generic result memory receipt does not reconcile with its retained export surface",
        ));
    }
    Ok(VckssEngineDetailedReceiptV6 {
        v5,
        engine_requested: solved.engine_requested,
        engine_selected: solved.engine_selected,
        generic_applicability_flags: flags,
        controls_count: solved.controls_count,
        rhs_receipt_schema: 2,
        control_projection_rhs_count: to_u64(
            rank.projection_rhs.len(),
            "control projection RHS count",
        )?,
        control_rank_rcond: rank.rcond,
        control_rank_smallest_generalized_eigenvalue_lower: rank
            .smallest_generalized_eigenvalue_lower,
        control_rank_largest_generalized_eigenvalue_upper: rank
            .largest_generalized_eigenvalue_upper,
        control_rank_projection_error_bound: rank.projection_error_bound,
        control_rank_normalization_error_bound: rank.normalization_error_bound,
        control_rank_fe_information_eigenvalue_lower_bound: rank
            .fe_information_eigenvalue_lower_bound,
        control_rank_maximum_projection_residual: rank.maximum_projection_residual,
        control_rank_effective_tolerance: rank.effective_rank_tolerance,
        control_rank_projection_pcg_tolerance: rank.projection_pcg_tolerance,
        control_rank_projection_residual_gate: rank.projection_residual_gate,
        generic_control_basis_relres: receipt.control_basis_relres,
        generic_control_basis_forward_error: receipt.control_basis_forward_error,
        generic_control_schur_rcond: receipt.control_schur_rcond,
        generic_control_schur_relres: receipt.control_schur_relres,
        generic_deletion_rank_gap: receipt.deletion_rank_gap,
        full_joint_fit_complete_residual: receipt.full_fit_complete_residual,
        generic_working_fit_complete_residual: receipt.working_fit_complete_residual,
        generic_maker_relres: receipt.maximum_maker_relres,
        canonicalization_peak_forecast_bytes: receipt.canonicalization_peak_forecast_bytes,
        generic_fit_peak_forecast_bytes: receipt.fit_peak_forecast_bytes,
        geometry_peak_forecast_bytes: receipt.geometry_peak_forecast_bytes,
        generic_leverage_peak_forecast_bytes: receipt.leverage_peak_forecast_bytes,
        generic_target_peak_forecast_bytes: receipt.target_peak_forecast_bytes,
        maker_peak_forecast_bytes: receipt.maker_peak_forecast_bytes,
        generic_result_forecast_bytes: receipt.result_forecast_bytes,
        generic_peak_forecast_bytes: receipt.peak_forecast_bytes,
        // Combined peak workspace for the native 96-byte V2 row array and
        // the caller-owned fifteen-double Stata matrix: 216 bytes per row.
        rhs_v2_caller_copy_bytes: receipt.rhs_export_bytes,
        capability_schema: solved.capability_schema,
        capability_profile: solved.capability_profile,
        batch_mode: solved.batch_mode,
        stayers_mode: solved.stayers_mode,
        target_weight_mode: solved.target_weight_mode,
        deletion_unit_source: solved.deletion_unit_source,
        probeorder_supplied: solved.probeorder_supplied,
        wallseconds_supplied: solved.wallseconds_supplied,
        frequency_use: solved.frequency_use,
        reserved_6: 0,
        physical_limit: solved.physical_limit,
        request_signature: solved.request_signature,
    })
}

fn resolution_receipt(plan: &EstimatorPlanReceipt) -> Result<VckssEstimatorResolutionReceiptV1> {
    Ok(VckssEstimatorResolutionReceiptV1 {
        algorithm_schema: plan.algorithm.schema_version,
        algorithm_requested: algorithm_request_code(plan.algorithm.requested),
        algorithm_selected: algorithm_code(plan.algorithm.selected),
        algorithm_reason: algorithm_reason_code(plan.algorithm.reason),
        engine_schema: plan.engine.schema_version,
        engine_requested: engine_request_code(plan.engine.requested),
        engine_selected: selected_engine_code(plan.engine.selected),
        engine_reason: engine_reason_code(plan.engine.reason),
        compressed_eligibility: compressed_eligibility_code(plan.engine.compressed_eligibility),
        resolved_before_rng: u32::from(plan.resolved_before_rng),
        opportunistic_engine_fallback_allowed: u32::from(plan.opportunistic_fallback_allowed),
        reserved: 0,
        identified_complexity: to_u64(
            plan.algorithm.identified_complexity,
            "identified complexity",
        )?,
        exact_limit: to_u64(plan.algorithm.exact_limit, "exact limit")?,
        rng_draws_before_resolution: plan.rng_draws_consumed,
        counter_atoms_before_resolution: plan.rng_counter_atoms_consumed,
    })
}

fn execution_plan_exact(
    generation: u64,
    signature: u64,
    plan: &EstimatorPlanReceipt,
    execution: &ExactExecutionReceipt,
    hard_limit: u64,
    prepared_persistent: u64,
) -> Result<VckssExecutionPlanReceiptV1> {
    Ok(VckssExecutionPlanReceiptV1 {
        struct_size: struct_size_u32::<VckssExecutionPlanReceiptV1>()?,
        schema_version: 1,
        generation,
        applicability_flags: 1 | 2 | 8 | 16 | 32,
        contract_flags: 1 | 2 | 4 | 8 | 16,
        request_signature: signature,
        resolution: resolution_receipt(plan)?,
        solver: VckssSolverExecutionReceiptV1 {
            schema_version: execution.schema_version,
            requested_route: VCKSS_ROUTE_NOT_APPLICABLE,
            selected_route: VCKSS_ROUTE_NOT_APPLICABLE,
            plan_frozen_before_rng: u32::from(execution.plan_frozen_before_execution),
            threads_requested: to_u32(execution.threads.requested, "exact requested threads")?,
            threads_used: to_u32(execution.threads.used, "exact used threads")?,
            parallel_regions: to_u32(execution.threads.parallel_regions, "exact parallel regions")?,
            applicability: VCKSS_PLAN_APPLICABILITY_EXACT,
            logical_atoms_before_plan_freeze: execution.logical_atoms_before_plan_freeze,
            unique_words_before_plan_freeze: execution.unique_packed_words_before_plan_freeze,
            physical_trials_before_plan_freeze: execution.physical_trials_before_plan_freeze,
            ..VckssSolverExecutionReceiptV1::default()
        },
        batch: batch_receipt_not_applicable(VCKSS_PLAN_APPLICABILITY_EXACT),
        wall: wall_execution_receipt(&execution.wall, VCKSS_PLAN_APPLICABILITY_EXACT),
        counter: counter_execution_receipt(
            &execution.counter,
            VCKSS_RNG_NONE,
            execution.logical_atoms_before_plan_freeze,
            execution.unique_packed_words_before_plan_freeze,
            execution.physical_trials_before_plan_freeze,
        ),
        memory: VckssExecutionMemoryReceiptV1 {
            schema_version: 1,
            applicability: VCKSS_PLAN_APPLICABILITY_EXACT,
            hard_limit_bytes: hard_limit,
            prepared_persistent_bytes: prepared_persistent,
            fit_peak_bytes: execution.fit_peak_forecast_bytes,
            correction_peak_bytes: execution.correction_peak_forecast_bytes,
            non_batched_peak_bytes: execution.peak_forecast_bytes,
            command_peak_bytes: execution.peak_forecast_bytes,
            ..VckssExecutionMemoryReceiptV1::default()
        },
    })
}

fn execution_plan_compressed(
    generation: u64,
    signature: u64,
    plan: &EstimatorPlanReceipt,
    execution: &CompressedJlaExecutionReceipt,
) -> Result<VckssExecutionPlanReceiptV1> {
    let fallback = execution.solver_setup.fallback.as_ref();
    Ok(VckssExecutionPlanReceiptV1 {
        struct_size: struct_size_u32::<VckssExecutionPlanReceiptV1>()?,
        schema_version: 1,
        generation,
        applicability_flags: 1 | 2 | 4 | 8 | 16 | 32,
        contract_flags: 1 | 2 | 4 | 8,
        request_signature: signature,
        resolution: resolution_receipt(plan)?,
        solver: VckssSolverExecutionReceiptV1 {
            schema_version: execution.schema_version,
            requested_route: route_code(execution.requested_solver_route),
            selected_route: route_code(execution.selected_solver_route),
            fallback_used: u32::from(fallback.is_some()),
            fallback_error: fallback.map_or(0, |value| value.code as i32),
            full_setup_complete: 1,
            fe_setup_complete: 1,
            fe_hierarchy_reused: 1,
            plan_frozen_before_rng: u32::from(execution.plan_frozen_before_rng),
            threads_requested: to_u32(execution.threads.requested, "compressed requested threads")?,
            threads_used: to_u32(execution.threads.used, "compressed used threads")?,
            parallel_regions: to_u32(
                execution.threads.parallel_regions,
                "compressed parallel regions",
            )?,
            applicability: VCKSS_PLAN_APPLICABILITY_COMPRESSED,
            planned_rhs: execution.planned_rhs,
            full_solver_dimension: to_u64(
                execution.solver_setup.dimension,
                "compressed solver dimension",
            )?,
            fe_solver_dimension: to_u64(
                execution.solver_setup.dimension,
                "compressed FE solver dimension",
            )?,
            logical_atoms_before_plan_freeze: execution.logical_atoms_before_plan_freeze,
            unique_words_before_plan_freeze: execution.unique_packed_words_before_plan_freeze,
            physical_trials_before_plan_freeze: execution.physical_trials_before_plan_freeze,
            ..VckssSolverExecutionReceiptV1::default()
        },
        batch: batch_execution_receipt(&execution.batch.plan, VCKSS_PLAN_APPLICABILITY_COMPRESSED)?,
        wall: wall_execution_receipt(&execution.wall, VCKSS_PLAN_APPLICABILITY_COMPRESSED),
        counter: counter_execution_receipt(
            &execution.counter,
            VCKSS_RNG_COUNTER_V1,
            execution.logical_atoms_before_plan_freeze,
            execution.unique_packed_words_before_plan_freeze,
            execution.physical_trials_before_plan_freeze,
        ),
        memory: VckssExecutionMemoryReceiptV1 {
            schema_version: 1,
            applicability: VCKSS_PLAN_APPLICABILITY_COMPRESSED,
            hard_limit_bytes: execution.memory.hard_limit_bytes,
            prepared_persistent_bytes: execution.memory.prepared_persistent_bytes,
            setup_peak_bytes: execution.memory.solver_setup_forecast_bytes,
            leverage_peak_bytes: execution.memory.leverage_phase_forecast_bytes,
            target_peak_bytes: execution.memory.target_phase_forecast_bytes,
            result_peak_bytes: execution.memory.result_forecast_bytes,
            non_batched_peak_bytes: execution.memory.non_batched_phase_forecast_bytes,
            command_peak_bytes: execution.memory.solve_peak_forecast_bytes,
            ..VckssExecutionMemoryReceiptV1::default()
        },
    })
}

fn execution_plan_generic(
    generation: u64,
    signature: u64,
    plan: &EstimatorPlanReceipt,
    execution: &GenericJlaExecutionReceipt,
    receipt: &vckss_core::generic_jla::GenericJlaReceipt,
    hard_limit: u64,
    prepared_persistent: u64,
) -> Result<VckssExecutionPlanReceiptV1> {
    let fallback = execution.fallback.as_ref();
    Ok(VckssExecutionPlanReceiptV1 {
        struct_size: struct_size_u32::<VckssExecutionPlanReceiptV1>()?,
        schema_version: 1,
        generation,
        applicability_flags: 1 | 2 | 4 | 8 | 16 | 32,
        contract_flags: 1 | 2 | 4 | 8,
        request_signature: signature,
        resolution: resolution_receipt(plan)?,
        solver: VckssSolverExecutionReceiptV1 {
            schema_version: execution.schema_version,
            requested_route: model_route_code(execution.requested_route),
            selected_route: model_route_code(execution.selected_route),
            fallback_used: u32::from(fallback.is_some()),
            fallback_error: fallback.map_or(0, |value| value.code as i32),
            full_setup_complete: u32::from(execution.full_model_setup_complete),
            fe_setup_complete: u32::from(execution.fe_solver_setup_complete),
            fe_hierarchy_reused: u32::from(execution.fe_hierarchy_reused),
            plan_frozen_before_rng: u32::from(execution.plan_frozen_before_rng),
            auto_route_contract: 1,
            threads_requested: to_u32(execution.threads.requested, "generic requested threads")?,
            threads_used: to_u32(execution.threads.used, "generic used threads")?,
            parallel_regions: to_u32(
                execution.threads.parallel_regions,
                "generic parallel regions",
            )?,
            applicability: VCKSS_PLAN_APPLICABILITY_GENERIC,
            planned_rhs: to_u64(execution.planned_rhs, "generic planned RHS")?,
            auto_firm_threshold: to_u64(
                execution.auto_firm_threshold,
                "generic auto firm threshold",
            )?,
            auto_rhs_threshold: to_u64(
                execution.auto_planned_rhs_threshold,
                "generic auto RHS threshold",
            )?,
            full_solver_dimension: to_u64(
                execution.full_solver_setup.dimension,
                "generic full solver dimension",
            )?,
            fe_solver_dimension: to_u64(
                execution.fe_solver_setup.dimension,
                "generic FE solver dimension",
            )?,
            logical_atoms_before_plan_freeze: execution.counter_atoms_before_plan_freeze,
            unique_words_before_plan_freeze: execution.unique_packed_words_before_plan_freeze,
            physical_trials_before_plan_freeze: execution.physical_trials_before_plan_freeze,
            ..VckssSolverExecutionReceiptV1::default()
        },
        batch: batch_execution_receipt(&execution.batch.plan, VCKSS_PLAN_APPLICABILITY_GENERIC)?,
        wall: wall_execution_receipt(&execution.wall, VCKSS_PLAN_APPLICABILITY_GENERIC),
        counter: counter_execution_receipt(
            &execution.counter,
            VCKSS_RNG_COUNTER_V1,
            execution.counter_atoms_before_plan_freeze,
            execution.unique_packed_words_before_plan_freeze,
            execution.physical_trials_before_plan_freeze,
        ),
        memory: VckssExecutionMemoryReceiptV1 {
            schema_version: 1,
            applicability: VCKSS_PLAN_APPLICABILITY_GENERIC,
            peak_phase: generic_memory_peak_phase_code(execution.memory.peak_phase),
            hard_limit_bytes: hard_limit,
            prepared_persistent_bytes: prepared_persistent,
            setup_peak_bytes: execution.memory.setup_peak_bytes,
            fit_peak_bytes: receipt.fit_peak_forecast_bytes,
            leverage_peak_bytes: receipt.leverage_peak_forecast_bytes,
            target_peak_bytes: receipt.target_peak_forecast_bytes,
            result_peak_bytes: receipt.result_forecast_bytes,
            non_batched_peak_bytes: execution.batch.plan.non_batched_peak_bytes,
            command_peak_bytes: execution.memory.peak_bytes,
            shared_cmg_persistent_bytes: execution.memory.shared_cmg_persistent_bytes,
            full_control_block_persistent_bytes: execution
                .memory
                .full_control_block_persistent_bytes,
            setup_transient_bytes: execution.memory.setup_transient_bytes,
            cmg_preconditioner_workspace_bytes: execution.memory.cmg_preconditioner_workspace_bytes,
            cmg_aggregated_cell_capacity_bytes: execution.memory.cmg_aggregated_cell_capacity_bytes,
            cmg_group_index_bytes: execution.memory.cmg_group_index_bytes,
            cmg_hybrid_graph_bytes: execution.memory.cmg_hybrid_graph_bytes,
            retained_nq_bytes: execution.memory.retained_nq_bytes,
            retained_q2_bytes: execution.memory.retained_q2_bytes,
            ..VckssExecutionMemoryReceiptV1::default()
        },
    })
}

fn batch_receipt_not_applicable(applicability: u32) -> VckssBatchExecutionReceiptV1 {
    VckssBatchExecutionReceiptV1 {
        schema_version: 1,
        applicability,
        leverage: VckssBatchPhasePlanReceiptV1 {
            request_mode: VCKSS_BATCH_MODE_NOT_APPLICABLE,
            applicability,
            ..VckssBatchPhasePlanReceiptV1::default()
        },
        target: VckssBatchPhasePlanReceiptV1 {
            request_mode: VCKSS_BATCH_MODE_NOT_APPLICABLE,
            applicability,
            ..VckssBatchPhasePlanReceiptV1::default()
        },
        ..VckssBatchExecutionReceiptV1::default()
    }
}

fn batch_execution_receipt(
    receipt: &BatchPlanReceipt,
    applicability: u32,
) -> Result<VckssBatchExecutionReceiptV1> {
    Ok(VckssBatchExecutionReceiptV1 {
        schema_version: receipt.schema_version,
        deterministic: u32::from(receipt.deterministic),
        width_invariance_required: u32::from(receipt.bitwise_estimator_width_invariance_required),
        arithmetic_contract: 1,
        whole_command_admitted: u32::from(receipt.whole_command_admitted),
        applicability,
        non_batched_peak_bytes: receipt.non_batched_peak_bytes,
        selected_command_peak_bytes: receipt.selected_command_peak_bytes,
        leverage: batch_phase_receipt(&receipt.leverage, applicability)?,
        target: batch_phase_receipt(&receipt.target, applicability)?,
    })
}

fn batch_phase_receipt(
    receipt: &BatchPhaseReceipt,
    applicability: u32,
) -> Result<VckssBatchPhasePlanReceiptV1> {
    let (request_mode, requested_width) = match receipt.requested {
        BatchRequest::Auto => (VCKSS_BATCH_MODE_AUTO, 0),
        BatchRequest::Explicit(width) => (
            VCKSS_BATCH_MODE_EXPLICIT,
            to_u64(width, "requested batch width")?,
        ),
    };
    Ok(VckssBatchPhasePlanReceiptV1 {
        request_mode,
        selection_reason: match receipt.reason {
            BatchSelectionReason::LargestAdmissibleCandidate => VCKSS_BATCH_SELECTION_AUTO,
            BatchSelectionReason::ExplicitWidth => VCKSS_BATCH_SELECTION_EXPLICIT,
        },
        applicability,
        reserved: 0,
        requested_width,
        selected_width: to_u64(receipt.selected_width, "selected batch width")?,
        probe_width_cap: to_u64(receipt.probe_width_cap, "probe batch-width cap")?,
        declared_threads: to_u64(receipt.declared_threads, "declared threads")?,
        thread_width_cap: to_u64(receipt.thread_width_cap, "thread batch-width cap")?,
        route_width_cap: to_u64(receipt.route_width_cap, "route batch-width cap")?,
        effective_width_cap: to_u64(receipt.effective_width_cap, "effective batch-width cap")?,
        hard_memory_bytes: receipt.hard_memory_bytes,
        width_one_forecast_bytes: receipt.width_one_forecast_bytes,
        selected_forecast_bytes: receipt.selected_forecast_bytes,
    })
}

fn wall_execution_receipt(
    receipt: &WallWorkReceipt,
    applicability: u32,
) -> VckssWallExecutionReceiptV1 {
    VckssWallExecutionReceiptV1 {
        schema_version: receipt.schema_version,
        model_code: applicability,
        status: match receipt.status {
            WallAdvisoryStatus::NotRequested => VCKSS_WALL_STATUS_NOT_REQUESTED,
            WallAdvisoryStatus::Uncalibrated => VCKSS_WALL_STATUS_UNCALIBRATED,
            WallAdvisoryStatus::WithinRequestedEnvelope => VCKSS_WALL_STATUS_WITHIN,
            WallAdvisoryStatus::ExceedsRequestedEnvelope => VCKSS_WALL_STATUS_EXCEEDS,
        },
        routing_effect: 1,
        requested_applicable: u32::from(receipt.requested_seconds.is_some()),
        forecast_applicable: u32::from(receipt.forecast_seconds.is_some()),
        advisory_applicable: u32::from(receipt.advisory_seconds.is_some()),
        margin_applicable: u32::from(receipt.advisory_margin_fraction.is_some()),
        requested_seconds: receipt.requested_seconds.unwrap_or(0.0),
        forecast_seconds: receipt.forecast_seconds.unwrap_or(0.0),
        advisory_seconds: receipt.advisory_seconds.unwrap_or(0.0),
        advisory_margin_fraction: receipt.advisory_margin_fraction.unwrap_or(0.0),
        preparation_work: receipt.work.preparation,
        engine_setup_work: receipt.work.engine_setup,
        full_fit_work: receipt.work.full_fit,
        leverage_work: receipt.work.leverage,
        target_work: receipt.work.target,
        result_export_work: receipt.work.result_export,
        total_work: receipt.total_work,
    }
}

fn counter_execution_receipt(
    receipt: &CounterExecutionReceipt,
    rng_contract: u32,
    logical_before: u64,
    words_before: u64,
    trials_before: u64,
) -> VckssCounterExecutionReceiptV1 {
    let applicable = receipt.total.planned_generator_word_evaluations.is_some();
    VckssCounterExecutionReceiptV1 {
        schema_version: receipt.schema_version,
        rng_contract,
        generator_work_applicable: u32::from(applicable),
        completed: u32::from(
            receipt.total.actual_logical_atoms == receipt.total.planned_logical_atoms
                && receipt.total.actual_unique_packed_words
                    == receipt.total.planned_unique_packed_words
                && receipt.total.actual_physical_bernoulli_trials
                    == receipt.total.planned_physical_bernoulli_trials
                && receipt.total.actual_generator_word_evaluations
                    == receipt.total.planned_generator_word_evaluations,
        ),
        leverage: counter_phase_receipt(receipt.leverage),
        target: counter_phase_receipt(receipt.target),
        total: counter_phase_receipt(receipt.total),
        logical_atoms_before_plan_freeze: logical_before,
        unique_words_before_plan_freeze: words_before,
        physical_trials_before_plan_freeze: trials_before,
    }
}

fn counter_phase_receipt(
    receipt: CounterPhaseExecutionReceipt,
) -> VckssCounterPhaseExecutionReceiptV1 {
    VckssCounterPhaseExecutionReceiptV1 {
        planned_logical_atoms: receipt.planned_logical_atoms,
        actual_logical_atoms: receipt.actual_logical_atoms,
        planned_unique_packed_words: receipt.planned_unique_packed_words,
        actual_unique_packed_words: receipt.actual_unique_packed_words,
        planned_physical_trials: receipt.planned_physical_bernoulli_trials,
        actual_physical_trials: receipt.actual_physical_bernoulli_trials,
        planned_generator_work: receipt.planned_generator_word_evaluations.unwrap_or(0),
        actual_generator_work: receipt.actual_generator_word_evaluations.unwrap_or(0),
    }
}

const fn algorithm_request_code(value: AlgorithmRequest) -> u32 {
    match value {
        AlgorithmRequest::Auto => VCKSS_ALGORITHM_AUTO,
        AlgorithmRequest::Exact => VCKSS_ALGORITHM_EXACT,
        AlgorithmRequest::Jla => VCKSS_ALGORITHM_JLA,
    }
}

const fn algorithm_code(value: EstimatorAlgorithm) -> u32 {
    match value {
        EstimatorAlgorithm::Exact => VCKSS_ALGORITHM_EXACT,
        EstimatorAlgorithm::Jla => VCKSS_ALGORITHM_JLA,
    }
}

const fn engine_request_code(value: EngineRequest) -> u32 {
    match value {
        EngineRequest::Auto => VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        EngineRequest::Compressed => VCKSS_ENGINE_COMPRESSED,
        EngineRequest::Generic => VCKSS_ENGINE_GENERIC,
    }
}

const fn selected_engine_code(value: SelectedEngine) -> u32 {
    match value {
        SelectedEngine::NotApplicable => VCKSS_ENGINE_NOT_APPLICABLE,
        SelectedEngine::Compressed => VCKSS_ENGINE_COMPRESSED,
        SelectedEngine::Generic => VCKSS_ENGINE_GENERIC,
    }
}

const fn algorithm_reason_code(value: AlgorithmResolutionReason) -> u32 {
    match value {
        AlgorithmResolutionReason::ExplicitExact => 1,
        AlgorithmResolutionReason::ExplicitJla => 2,
        AlgorithmResolutionReason::AutomaticWithinExactLimit => 3,
        AlgorithmResolutionReason::AutomaticAboveExactLimit => 4,
    }
}

const fn engine_reason_code(value: EngineResolutionReason) -> u32 {
    match value {
        EngineResolutionReason::ExactNotApplicable => 1,
        EngineResolutionReason::JlaExplicitGeneric => 2,
        EngineResolutionReason::JlaExplicitCompressed => 3,
        EngineResolutionReason::JlaAutomaticCompressed => 4,
        EngineResolutionReason::JlaAutomaticGenericScientificIneligibility => 5,
    }
}

const fn compressed_eligibility_code(value: CompressedEligibilityReason) -> u32 {
    match value {
        CompressedEligibilityReason::NotApplicable => 0,
        CompressedEligibilityReason::Eligible => 1,
        CompressedEligibilityReason::ObservationDeletion => 2,
        CompressedEligibilityReason::ControlsPresent => 3,
        CompressedEligibilityReason::SemanticPlanUnavailable => 4,
        CompressedEligibilityReason::PhysicalRngContractUnavailable => 5,
    }
}

const fn model_route_code(value: ModelSolverRoute) -> u32 {
    match value {
        ModelSolverRoute::Auto => VCKSS_ROUTE_AUTO,
        ModelSolverRoute::Diagonal => VCKSS_ROUTE_DIAGONAL_PCG,
        ModelSolverRoute::Cmg => VCKSS_ROUTE_CMG_PCG,
    }
}

const fn generic_memory_peak_phase_code(value: GenericJlaMemoryPeakPhase) -> u32 {
    match value {
        GenericJlaMemoryPeakPhase::Canonicalization => 1,
        GenericJlaMemoryPeakPhase::SolverSetup => 2,
        GenericJlaMemoryPeakPhase::Fit => 3,
        GenericJlaMemoryPeakPhase::Geometry => 4,
        GenericJlaMemoryPeakPhase::Leverage => 5,
        GenericJlaMemoryPeakPhase::Target => 6,
        GenericJlaMemoryPeakPhase::Maker => 7,
        GenericJlaMemoryPeakPhase::Result => 8,
    }
}

fn result_accounting_residual(result: &EngineEstimate) -> f64 {
    [result.plugin(), result.correction(), result.corrected()]
        .into_iter()
        .map(component_identity_residual)
        .fold(0.0_f64, f64::max)
}

fn full_cmg_receipt_v1(generation: u64, receipt: &FullCmgReceipt) -> Result<VckssFullCmgReceiptV1> {
    if receipt.schema != "CMG_FULL_V2" || receipt.source_commit.len() != 40 {
        return Err(BackendError::invariant(
            "engine_full_cmg_receipt",
            "core full-CMG receipt identity is inconsistent",
        ));
    }
    let mut source_commit = [0_u8; 40];
    source_commit.copy_from_slice(receipt.source_commit.as_bytes());
    let batch_strategy_mask = u32::from(receipt.serial_batches > 0)
        | (u32::from(receipt.planned_batches > 0) << 1)
        | (u32::from(receipt.across_rhs_batches > 0) << 2);
    Ok(VckssFullCmgReceiptV1 {
        struct_size: struct_size_u32::<VckssFullCmgReceiptV1>()?,
        schema_version: 1,
        generation,
        backend_identity: 2,
        platform_os: if cfg!(target_os = "macos") {
            1
        } else if cfg!(target_os = "linux") {
            2
        } else if cfg!(target_os = "windows") {
            3
        } else {
            0
        },
        platform_arch: if cfg!(target_arch = "aarch64") {
            1
        } else if cfg!(target_arch = "x86_64") {
            2
        } else {
            0
        },
        batch_strategy_mask,
        cmg_source_commit: source_commit,
        threads_requested: to_u32(receipt.setup.threads, "full-CMG requested threads")?,
        threads_used: to_u32(receipt.setup.threads, "full-CMG used threads")?,
        maximum_concurrency: to_u64(receipt.maximum_concurrency, "full-CMG concurrency")?,
        vertices: to_u64(receipt.setup.vertices, "full-CMG vertices")?,
        edges: to_u64(receipt.setup.edges, "full-CMG edges")?,
        hierarchy_levels: to_u64(receipt.setup.hierarchy_levels, "full-CMG hierarchy levels")?,
        terminal_vertices: to_u64(
            receipt.setup.terminal_vertices,
            "full-CMG terminal vertices",
        )?,
        graph_copy_bytes: receipt.setup.graph_copy_bytes,
        hierarchy_bytes: receipt.setup.hierarchy_bytes,
        plan_bytes: receipt.setup.plan_bytes,
        workspace_bytes_each: receipt.setup.workspace_bytes_each,
        workspace_pool_bytes: receipt.setup.admitted_workspace_pool_bytes,
        admitted_peak_bytes: receipt.setup.admitted_peak_bytes,
        fit_effective_tolerance: receipt.setup.fit_effective_tolerance,
        probe_effective_tolerance: receipt.setup.probe_effective_tolerance,
        fit_initial_inner_tolerance: receipt.setup.fit_inner_tolerance,
        probe_initial_inner_tolerance: receipt.setup.probe_inner_tolerance,
        refinement_attempts: receipt.refinement_attempts,
        refined_columns: receipt.refined_columns,
        batch_calls: receipt.batch_calls,
        rhs_count: receipt.rhs_count,
        serial_batches: receipt.serial_batches,
        planned_batches: receipt.planned_batches,
        across_rhs_batches: receipt.across_rhs_batches,
        total_iterations: receipt.total_iterations,
        total_operator_applications: receipt.total_operator_applications,
        total_preconditioner_applications: receipt.total_preconditioner_applications,
        maximum_reduced_residual: receipt.maximum_reduced_residual,
        maximum_complete_residual: receipt.maximum_complete_residual,
        graph_ns: receipt_ns(receipt.setup.graph_nanoseconds, "full-CMG graph time")?,
        hierarchy_plan_ns: receipt_ns(
            receipt.setup.solver_nanoseconds,
            "full-CMG hierarchy/plan time",
        )?,
        rhs_ns: receipt_ns(receipt.rhs_nanoseconds, "full-CMG RHS time")?,
        solve_ns: receipt_ns(receipt.solve_nanoseconds, "full-CMG solve time")?,
        extraction_ns: receipt_ns(receipt.extraction_nanoseconds, "full-CMG extraction time")?,
        preparation_peak_bytes: receipt.setup.preparation_peak_bytes,
        prepared_persistent_bytes: receipt.setup.prepared_persistent_bytes,
        non_cmg_command_peak_bytes: receipt.setup.non_cmg_command_peak_bytes,
        pre_rng_forecast_bytes: receipt.setup.pre_rng_forecast_bytes,
        actual_retained_bytes: receipt.setup.actual_retained_bytes,
        allocator_allowance_bytes: receipt.setup.allocator_allowance_bytes,
        maximum_batch_rhs: to_u64(
            receipt.setup.maximum_batch_rhs,
            "full-CMG maximum batch RHS",
        )?,
        workspace_count: to_u64(receipt.setup.workspace_count, "full-CMG workspace count")?,
    })
}

fn receipt_ns(value: u128, context: &'static str) -> Result<u64> {
    u64::try_from(value).map_err(|_| resource_error("engine_full_cmg_receipt", context))
}

fn component_identity_residual(value: VarianceComponents) -> f64 {
    (value.total - value.worker - value.firm - 2.0 * value.covariance).abs()
}

fn preparation_receipt_v2(
    generation: u64,
    receipt: PreparationReceipt,
) -> Result<VckssEnginePreparationReceiptV2> {
    let graph = receipt.graph;
    let memory = receipt.memory;
    Ok(VckssEnginePreparationReceiptV2 {
        struct_size: struct_size_u32::<VckssEnginePreparationReceiptV2>()?,
        reserved: 0,
        generation,
        input_rows: receipt.input_rows,
        retained_rows: receipt.retained_rows,
        workers: receipt.workers,
        firms: receipt.firms,
        cells: receipt.cells,
        deletion_units: receipt.deletion_units,
        target_strata: receipt.target_strata,
        memory_limit_bytes: memory.hard_limit_bytes,
        caller_copy_bytes: memory.caller_copy_bytes,
        preparation_peak_forecast_bytes: memory.preparation_peak_forecast_bytes,
        prepared_resident_bytes: memory.prepared_resident_bytes,
        graph_input_rows: graph.input_rows,
        graph_retained_rows: graph.retained_rows,
        graph_input_physical_mass: graph.input_physical_mass,
        graph_retained_physical_mass: graph.retained_physical_mass,
        graph_initial_components: graph.initial_components,
        graph_maximum_components: graph.maximum_components,
        graph_initial_component_rows: graph.initial_component_rows,
        graph_mover_input_rows: graph.mover_input_rows,
        graph_initial_deletion_edges: graph.initial_deletion_edges,
        graph_retained_deletion_edges: graph.retained_deletion_edges,
        graph_insufficient_workers_removed: graph.insufficient_workers_removed,
        graph_articulation_workers_removed: graph.articulation_workers_removed,
        graph_bridge_units_removed: graph.bridge_units_removed,
        graph_bridge_rows_removed: graph.bridge_rows_removed,
        graph_degree_iterations: graph.degree_iterations,
        graph_articulation_iterations: graph.articulation_iterations,
        graph_bridge_iterations: graph.bridge_iterations,
        graph_fixed_point_iterations: graph.fixed_point_iterations,
    })
}

fn stayer_augmentation_receipt_v1(
    generation: u64,
    receipt: EngineStayerAugmentationReceipt,
) -> Result<VckssStayerAugmentationReceiptV1> {
    let core = receipt.core;
    let memory = receipt.memory;
    Ok(VckssStayerAugmentationReceiptV1 {
        struct_size: struct_size_u32::<VckssStayerAugmentationReceiptV1>()?,
        schema_version: VCKSS_STAYER_AUGMENTATION_SCHEMA_V1,
        generation,
        mover_stored_rows: core.mover_stored_rows,
        stayer_stored_rows: core.stayer_stored_rows,
        combined_stored_rows: core.combined_stored_rows,
        mover_physical_mass: core.mover_physical_mass,
        stayer_physical_mass: core.stayer_physical_mass,
        combined_physical_mass: core.combined_physical_mass,
        mover_workers: core.mover_workers,
        stayer_workers: core.stayer_workers,
        combined_workers: core.combined_workers,
        firms: core.firms,
        mover_deletion_units: core.mover_deletion_units,
        stayer_deletion_units: core.stayer_deletion_units,
        combined_deletion_units: core.combined_deletion_units,
        mover_target_mass: core.mover_target_mass,
        stayer_target_mass: core.stayer_target_mass,
        combined_target_mass: core.combined_target_mass,
        topology_checksum: core.topology_checksum,
        memory_limit_bytes: memory.hard_limit_bytes,
        caller_copy_bytes: memory.caller_copy_bytes,
        augmentation_peak_forecast_bytes: memory.augmentation_peak_forecast_bytes,
        augmented_resident_bytes: memory.augmented_resident_bytes,
        total_prepared_resident_bytes: memory.total_prepared_resident_bytes,
    })
}

fn stayer_hybrid_result_v1(
    generation: u64,
    result: &ExactStayerHybridResult,
) -> Result<VckssStayerHybridResultV1> {
    let estimator = &result.estimator;
    estimator.plugin.verify_accounting(1.0e-10)?;
    estimator.correction.verify_accounting(1.0e-10)?;
    estimator.corrected.verify_accounting(1.0e-10)?;
    result.mover_correction.verify_accounting(1.0e-10)?;
    result.stayer_correction.verify_accounting(1.0e-10)?;
    let source_sum = VarianceComponents {
        worker: result.mover_correction.worker + result.stayer_correction.worker,
        firm: result.mover_correction.firm + result.stayer_correction.firm,
        covariance: result.mover_correction.covariance + result.stayer_correction.covariance,
        total: result.mover_correction.total + result.stayer_correction.total,
    };
    let source_residual = [
        source_sum.worker - estimator.correction.worker,
        source_sum.firm - estimator.correction.firm,
        source_sum.covariance - estimator.correction.covariance,
        source_sum.total - estimator.correction.total,
    ]
    .into_iter()
    .map(f64::abs)
    .fold(0.0_f64, f64::max);
    if source_residual > 1.0e-10 {
        return Err(BackendError::invariant(
            "engine_stayer_hybrid_result",
            "mixed-deletion source corrections do not sum to the total correction",
        ));
    }
    let receipt = &estimator.receipt;
    Ok(VckssStayerHybridResultV1 {
        struct_size: struct_size_u32::<VckssStayerHybridResultV1>()?,
        schema_version: VCKSS_STAYER_HYBRID_RESULT_SCHEMA_V1,
        generation,
        plugin: component_vector(estimator.plugin),
        correction: component_vector(estimator.correction),
        corrected: component_vector(estimator.corrected),
        mover_correction: component_vector(result.mover_correction),
        stayer_correction: component_vector(result.stayer_correction),
        weighted_rss: estimator.weighted_rss,
        parameters: to_u64(receipt.parameters, "hybrid exact parameters")?,
        full_parameters: to_u64(receipt.full_parameters, "hybrid exact full parameters")?,
        correction_parameters: to_u64(
            receipt.correction_parameters,
            "hybrid exact correction parameters",
        )?,
        deletion_units: receipt.deletion_units,
        max_leverage: receipt.max_leverage,
        information_rcond: receipt.information_rcond,
        inverse_relres: receipt.inverse_relres,
        inverse_original_relres: receipt.inverse_original_relres,
        inverse_sqrt_relres: receipt.inverse_sqrt_relres,
        maker_relres: receipt.maker_relres,
        full_fit_relres: receipt.full_fit_relres,
        working_fit_relres: receipt.working_fit_relres,
        fit_residual_tolerance: receipt.fit_residual_tolerance,
        control_basis_relres: receipt.control_basis_relres,
        control_basis_forward_error: receipt.control_basis_forward_error,
        deletion_rank_gap: receipt.deletion_rank_gap,
        firm_zero_sum_residual: receipt.firm_zero_sum_residual,
        peak_forecast_bytes: receipt.peak_forecast_bytes,
        fit_peak_forecast_bytes: receipt.fit_peak_forecast_bytes,
        correction_peak_forecast_bytes: receipt.correction_peak_forecast_bytes,
        topology_checksum: receipt.topology_checksum,
        accounting_residual: [
            component_identity_residual(estimator.plugin),
            component_identity_residual(estimator.correction),
            component_identity_residual(estimator.corrected),
            source_residual,
        ]
        .into_iter()
        .fold(0.0_f64, f64::max),
    })
}

fn rhs_receipt_count(receipt: &vckss_core::engine::JlaEngineReceipt) -> Result<u64> {
    let leverage = to_u64(receipt.leverage_rhs.len(), "leverage RHS receipt count")?;
    let target = to_u64(receipt.target_rhs.len(), "target RHS receipt count")?;
    1_u64
        .checked_add(leverage)
        .and_then(|value| value.checked_add(target))
        .ok_or_else(|| resource_error("engine_rhs_receipts", "RHS receipt count overflow"))
}

fn rng_contract_code(contract: RngContract) -> u32 {
    match contract {
        RngContract::StataCompatibility => 0,
        RngContract::CounterV1 => VCKSS_RNG_COUNTER_V1,
    }
}

fn rhs_phase_code(phase: JlaSolvePhase) -> u32 {
    match phase {
        JlaSolvePhase::FullFit => 1,
        JlaSolvePhase::Leverage => 2,
        JlaSolvePhase::Target => 3,
    }
}

fn rhs_side_code(side: JlaRhsSide) -> u32 {
    match side {
        JlaRhsSide::Joint => 0,
        JlaRhsSide::Worker => 1,
        JlaRhsSide::Firm => 2,
    }
}

fn generic_rhs_phase_code(phase: GenericJlaRhsPhase) -> u32 {
    match phase {
        GenericJlaRhsPhase::FullJointFit => 1,
        GenericJlaRhsPhase::Leverage => 2,
        GenericJlaRhsPhase::Target => 3,
        GenericJlaRhsPhase::FixedOffsetWorkingFit => 4,
        GenericJlaRhsPhase::ControlProjection => 5,
    }
}

fn generic_rhs_side_code(side: GenericJlaRhsSide) -> u32 {
    match side {
        GenericJlaRhsSide::Joint => 0,
        GenericJlaRhsSide::Worker => 1,
        GenericJlaRhsSide::Firm => 2,
    }
}

fn generic_rhs_receipt_value(
    solved: &EngineSolved,
    result: &GenericJlaResult,
    value: &GenericJlaRhsReceipt,
) -> Result<VckssEngineRhsReceiptV2> {
    let probe = value.probe.map_or(-1_i64, i64::from);
    let controls = u64::from(solved.controls_count);
    let firms = solved.preparation.firms;
    let (full_residual_tolerance, residual_space, solver_dimension) = match value.phase {
        GenericJlaRhsPhase::ControlProjection => {
            let projection = result
                .receipt
                .control_rank
                .projection_rhs
                .get(usize::try_from(probe).map_err(|_| {
                    resource_error(
                        "engine_rhs_receipts",
                        "control projection index is not addressable",
                    )
                })?)
                .ok_or_else(|| {
                    BackendError::invariant(
                        "engine_rhs_receipts",
                        "control projection receipt index is out of range",
                    )
                })?;
            (
                projection.complete_residual_tolerance,
                VCKSS_RESIDUAL_SPACE_WORKER_FIRM,
                to_u64(
                    projection.solver_dimension,
                    "control projection solver dimension",
                )?,
            )
        }
        GenericJlaRhsPhase::FullJointFit => (
            result.receipt.full_residual_tolerance,
            VCKSS_RESIDUAL_SPACE_WORKER_FIRM_CONTROL,
            firms.checked_add(controls).ok_or_else(|| {
                resource_error("engine_rhs_receipts", "full solver dimension overflow")
            })?,
        ),
        GenericJlaRhsPhase::FixedOffsetWorkingFit | GenericJlaRhsPhase::Leverage => (
            result.receipt.full_residual_tolerance,
            VCKSS_RESIDUAL_SPACE_WORKER_FIRM,
            firms,
        ),
        GenericJlaRhsPhase::Target => {
            let joint = solved.nuisance == NuisanceMode::Joint;
            (
                result.receipt.full_residual_tolerance,
                if joint {
                    VCKSS_RESIDUAL_SPACE_WORKER_FIRM_CONTROL
                } else {
                    VCKSS_RESIDUAL_SPACE_WORKER_FIRM
                },
                if joint {
                    firms.checked_add(controls).ok_or_else(|| {
                        resource_error("engine_rhs_receipts", "target solver dimension overflow")
                    })?
                } else {
                    firms
                },
            )
        }
    };
    Ok(VckssEngineRhsReceiptV2 {
        v1: VckssEngineRhsReceiptV1 {
            phase: generic_rhs_phase_code(value.phase),
            side: generic_rhs_side_code(value.side),
            probe,
            route: VCKSS_ROUTE_DIAGONAL_PCG,
            iterations: value.pcg.iterations,
            zero_rhs: u32::from(value.pcg.status == ModelPcgStatus::ZeroRhs),
            reserved: 0,
            reduced_residual: value.pcg.relative_residual,
            complete_residual: value.complete_residual,
        },
        status: match value.pcg.status {
            ModelPcgStatus::ZeroRhs => VCKSS_RHS_STATUS_ZERO,
            ModelPcgStatus::Converged => VCKSS_RHS_STATUS_CONVERGED,
        },
        residual_replacements: value.pcg.residual_replacements,
        operator_applications: u64::from(value.pcg.operator_applications),
        preconditioner_applications: u64::from(value.pcg.preconditioner_applications),
        full_residual_tolerance,
        residual_space,
        reserved_2: 0,
        solver_dimension,
    })
}

fn rhs_receipt_value(value: &JlaRhsReceipt) -> Result<VckssEngineRhsReceiptV1> {
    let probe = match value.probe {
        Some(probe) => i64::try_from(probe).map_err(|_| {
            resource_error(
                "engine_rhs_receipts",
                "zero-based probe index is not representable as i64",
            )
        })?,
        None => -1,
    };
    Ok(VckssEngineRhsReceiptV1 {
        phase: rhs_phase_code(value.phase),
        side: rhs_side_code(value.side),
        probe,
        route: route_code(value.route),
        iterations: value.iterations,
        zero_rhs: u32::from(value.zero_rhs),
        reserved: 0,
        reduced_residual: value.reduced_residual,
        complete_residual: value.complete_residual,
    })
}

fn write_rhs_receipt(
    output: *mut VckssEngineRhsReceiptV1,
    row: usize,
    value: &JlaRhsReceipt,
) -> Result<()> {
    let exported = rhs_receipt_value(value)?;
    // SAFETY: the caller capacity is checked for the complete receipt vector
    // before this bounded row index is reached, and no Rust reference forms.
    unsafe { output.add(row).write_unaligned(exported) };
    Ok(())
}

fn request_capability_classification(
    request: VckssBackendRequestCapabilityRequestV1,
) -> (u32, u32) {
    let reason = if request.request_schema != VCKSS_REQUEST_CAPABILITY_SCHEMA_V1 {
        VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA
    } else if !matches!(
        request.algorithm,
        VCKSS_ALGORITHM_AUTO | VCKSS_ALGORITHM_EXACT | VCKSS_ALGORITHM_JLA
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM
    } else if !matches!(
        request.deletion_mode,
        VCKSS_DELETION_MATCH | VCKSS_DELETION_OBSERVATION
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_DELETION
    } else if !matches!(
        request.nuisance_mode,
        VCKSS_NUISANCE_JOINT | VCKSS_NUISANCE_FIXED_OFFSET
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE
    } else if !matches!(
        request.solver_route,
        VCKSS_ROUTE_AUTO | VCKSS_ROUTE_EXACT | VCKSS_ROUTE_DIAGONAL_PCG | VCKSS_ROUTE_CMG_PCG
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE
    } else if !matches!(request.rng_contract, VCKSS_RNG_NONE | VCKSS_RNG_COUNTER_V1) {
        VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT
    } else if !matches!(
        request.frequency_use,
        VCKSS_REQUEST_FREQUENCY_UNIT | VCKSS_REQUEST_FREQUENCY_LITERAL
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE
    } else if request.algorithm == VCKSS_ALGORITHM_AUTO {
        // Dataset dimensions are intentionally absent from this request. An
        // unresolved estimator-level auto choice therefore cannot inherit
        // support from either concrete profile.
        VCKSS_REQUEST_REASON_ALGORITHM_AUTO_UNRESOLVED
    } else if request.algorithm == VCKSS_ALGORITHM_EXACT && request.controls_count > 32 {
        VCKSS_REQUEST_REASON_CONTROLS_LIMIT
    } else if request.algorithm == VCKSS_ALGORITHM_EXACT && request.rng_contract != VCKSS_RNG_NONE {
        VCKSS_REQUEST_REASON_EXACT_RNG
    } else if request.algorithm == VCKSS_ALGORITHM_EXACT
        && !matches!(request.solver_route, VCKSS_ROUTE_AUTO | VCKSS_ROUTE_EXACT)
    {
        // The dense exact estimator has generic/auto-equivalent solver
        // semantics and consumes no iterative preconditioner route.
        VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE
    } else if request.algorithm == VCKSS_ALGORITHM_JLA
        && request.deletion_mode != VCKSS_DELETION_MATCH
    {
        VCKSS_REQUEST_REASON_JLA_DELETION
    } else if request.algorithm == VCKSS_ALGORITHM_JLA
        && request.nuisance_mode != VCKSS_NUISANCE_JOINT
    {
        VCKSS_REQUEST_REASON_JLA_NUISANCE
    } else if request.algorithm == VCKSS_ALGORITHM_JLA && request.controls_count != 0 {
        VCKSS_REQUEST_REASON_JLA_CONTROLS
    } else if request.algorithm == VCKSS_ALGORITHM_JLA
        && request.rng_contract != VCKSS_RNG_COUNTER_V1
    {
        VCKSS_REQUEST_REASON_JLA_RNG
    } else {
        VCKSS_REQUEST_REASON_SUPPORTED
    };
    let profile = if reason != VCKSS_REQUEST_REASON_SUPPORTED {
        VCKSS_REQUEST_PROFILE_NONE
    } else if request.algorithm == VCKSS_ALGORITHM_EXACT {
        VCKSS_REQUEST_PROFILE_EXACT_V1
    } else {
        VCKSS_REQUEST_PROFILE_JLA_COUNTER_V1
    };
    (reason, profile)
}

fn request_capability_signature(request: VckssBackendRequestCapabilityRequestV1) -> u64 {
    // Fixed-domain FNV-1a over little-endian schema fields. This is an
    // identity receipt, not a cryptographic authentication mechanism.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"VCKSS-REQUEST-CAPABILITY-V1" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for value in [
        request.request_schema,
        request.algorithm,
        request.deletion_mode,
        request.nuisance_mode,
        request.solver_route,
        request.rng_contract,
        request.controls_count,
        request.frequency_use,
    ] {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn request_capability_classification_v2(
    request: VckssBackendRequestCapabilityRequestV2,
) -> (u32, u32) {
    let value = request.v1;
    let reason = if value.request_schema != VCKSS_REQUEST_CAPABILITY_SCHEMA_V2 {
        VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA
    } else if !matches!(
        value.algorithm,
        VCKSS_ALGORITHM_AUTO | VCKSS_ALGORITHM_EXACT | VCKSS_ALGORITHM_JLA
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM
    } else if !matches!(
        value.deletion_mode,
        VCKSS_DELETION_MATCH | VCKSS_DELETION_OBSERVATION
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_DELETION
    } else if !matches!(
        value.nuisance_mode,
        VCKSS_NUISANCE_JOINT | VCKSS_NUISANCE_FIXED_OFFSET
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE
    } else if !matches!(
        value.solver_route,
        VCKSS_ROUTE_AUTO | VCKSS_ROUTE_EXACT | VCKSS_ROUTE_DIAGONAL_PCG | VCKSS_ROUTE_CMG_PCG
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE
    } else if !matches!(value.rng_contract, VCKSS_RNG_NONE | VCKSS_RNG_COUNTER_V1) {
        VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT
    } else if !matches!(
        value.frequency_use,
        VCKSS_REQUEST_FREQUENCY_UNIT | VCKSS_REQUEST_FREQUENCY_LITERAL
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE
    } else if !matches!(
        request.engine,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED | VCKSS_ENGINE_COMPRESSED | VCKSS_ENGINE_GENERIC
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_ENGINE
    } else if !matches!(
        request.batch_mode,
        VCKSS_BATCH_MODE_AUTO | VCKSS_BATCH_MODE_EXPLICIT
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_BATCH_MODE
    } else if !matches!(
        request.stayers_mode,
        VCKSS_STAYERS_MOVERS | VCKSS_STAYERS_ALL
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_STAYERS_MODE
    } else if !matches!(
        request.target_weight_mode,
        VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT | VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_TARGET_WEIGHT_MODE
    } else if !matches!(
        request.deletion_unit_source,
        VCKSS_DELETION_SOURCE_CELL_DEFAULT
            | VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
            | VCKSS_DELETION_SOURCE_OBSERVATION_ROW
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_DELETION_UNIT_SOURCE
    } else if request.probeorder_supplied > 1 {
        VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED
    } else if request.wallseconds_supplied > 1 {
        VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED
    } else if request.physical_limit == 0 || request.physical_limit > MAX_EXACT_BINARY64_INTEGER {
        VCKSS_REQUEST_REASON_PHYSICAL_LIMIT
    } else if value.algorithm == VCKSS_ALGORITHM_AUTO {
        VCKSS_REQUEST_REASON_ALGORITHM_AUTO_UNRESOLVED
    } else if value.controls_count > 32 {
        VCKSS_REQUEST_REASON_CONTROLS_LIMIT
    } else if value.algorithm == VCKSS_ALGORITHM_EXACT {
        if request.engine == VCKSS_ENGINE_COMPRESSED {
            VCKSS_REQUEST_REASON_EXACT_ENGINE
        } else if value.rng_contract != VCKSS_RNG_NONE {
            VCKSS_REQUEST_REASON_EXACT_RNG
        } else if !matches!(value.solver_route, VCKSS_ROUTE_AUTO | VCKSS_ROUTE_EXACT) {
            VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE
        } else {
            VCKSS_REQUEST_REASON_SUPPORTED
        }
    } else if request.engine == VCKSS_ENGINE_GENERIC {
        if value.rng_contract != VCKSS_RNG_COUNTER_V1 {
            VCKSS_REQUEST_REASON_JLA_RNG
        } else if value.solver_route != VCKSS_ROUTE_DIAGONAL_PCG {
            VCKSS_REQUEST_REASON_JLA_GENERIC_SOLVER_ROUTE
        } else if request.batch_mode != VCKSS_BATCH_MODE_EXPLICIT {
            VCKSS_REQUEST_REASON_BATCH_MODE_UNSUPPORTED
        } else if request.stayers_mode != VCKSS_STAYERS_MOVERS {
            VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED
        } else if request.probeorder_supplied != 0 {
            VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED
        } else if request.wallseconds_supplied != 0 {
            VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED
        } else if (value.deletion_mode == VCKSS_DELETION_MATCH
            && !matches!(
                request.deletion_unit_source,
                VCKSS_DELETION_SOURCE_CELL_DEFAULT | VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
            ))
            || (value.deletion_mode == VCKSS_DELETION_OBSERVATION
                && request.deletion_unit_source != VCKSS_DELETION_SOURCE_OBSERVATION_ROW)
        {
            VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH
        } else {
            VCKSS_REQUEST_REASON_SUPPORTED
        }
    } else {
        let mut legacy = value;
        legacy.request_schema = VCKSS_REQUEST_CAPABILITY_SCHEMA_V1;
        let (legacy_reason, _) = request_capability_classification(legacy);
        if request.engine == VCKSS_ENGINE_AUTO_OR_UNSPECIFIED
            && legacy_reason != VCKSS_REQUEST_REASON_SUPPORTED
        {
            VCKSS_REQUEST_REASON_JLA_ENGINE_AUTO_UNRESOLVED
        } else {
            legacy_reason
        }
    };
    let profile = if reason != VCKSS_REQUEST_REASON_SUPPORTED {
        VCKSS_REQUEST_PROFILE_NONE
    } else if value.algorithm == VCKSS_ALGORITHM_EXACT {
        VCKSS_REQUEST_PROFILE_EXACT_V1
    } else if request.engine == VCKSS_ENGINE_GENERIC {
        VCKSS_REQUEST_PROFILE_JLA_GENERIC_COUNTER_V1
    } else {
        VCKSS_REQUEST_PROFILE_JLA_COUNTER_V1
    };
    (reason, profile)
}

fn request_capability_signature_v2(request: VckssBackendRequestCapabilityRequestV2) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"VCKSS-REQUEST-CAPABILITY-V2" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for value in [
        request.v1.request_schema,
        request.v1.algorithm,
        request.v1.deletion_mode,
        request.v1.nuisance_mode,
        request.v1.solver_route,
        request.v1.rng_contract,
        request.v1.controls_count,
        request.v1.frequency_use,
        request.engine,
        request.batch_mode,
        request.stayers_mode,
        request.target_weight_mode,
        request.deletion_unit_source,
        request.probeorder_supplied,
        request.wallseconds_supplied,
    ] {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    for byte in request.physical_limit.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn request_capability_classification_v3(
    request: VckssBackendRequestCapabilityRequestV3,
) -> (u32, u32) {
    let value = request.v2.v1;
    let phase_modes_known = matches!(
        request.leverage_batch_mode,
        VCKSS_BATCH_MODE_AUTO | VCKSS_BATCH_MODE_EXPLICIT
    ) && matches!(
        request.target_batch_mode,
        VCKSS_BATCH_MODE_AUTO | VCKSS_BATCH_MODE_EXPLICIT
    );
    let expected_summary = if request.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO
        && request.target_batch_mode == VCKSS_BATCH_MODE_AUTO
    {
        VCKSS_BATCH_MODE_AUTO
    } else if request.leverage_batch_mode == VCKSS_BATCH_MODE_EXPLICIT
        && request.target_batch_mode == VCKSS_BATCH_MODE_EXPLICIT
    {
        VCKSS_BATCH_MODE_EXPLICIT
    } else {
        VCKSS_BATCH_MODE_INDEPENDENT
    };
    let deletion_source_matches = (value.deletion_mode == VCKSS_DELETION_MATCH
        && matches!(
            request.v2.deletion_unit_source,
            VCKSS_DELETION_SOURCE_CELL_DEFAULT | VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
        ))
        || (value.deletion_mode == VCKSS_DELETION_OBSERVATION
            && request.v2.deletion_unit_source == VCKSS_DELETION_SOURCE_OBSERVATION_ROW);
    let reason = if value.request_schema != VCKSS_REQUEST_CAPABILITY_SCHEMA_V3 {
        VCKSS_REQUEST_REASON_UNKNOWN_SCHEMA
    } else if !matches!(
        value.algorithm,
        VCKSS_ALGORITHM_AUTO | VCKSS_ALGORITHM_EXACT | VCKSS_ALGORITHM_JLA
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_ALGORITHM
    } else if !matches!(
        value.deletion_mode,
        VCKSS_DELETION_MATCH | VCKSS_DELETION_OBSERVATION
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_DELETION
    } else if !matches!(
        value.nuisance_mode,
        VCKSS_NUISANCE_JOINT | VCKSS_NUISANCE_FIXED_OFFSET
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_NUISANCE
    } else if !matches!(
        value.solver_route,
        VCKSS_ROUTE_AUTO | VCKSS_ROUTE_EXACT | VCKSS_ROUTE_DIAGONAL_PCG | VCKSS_ROUTE_CMG_PCG
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_SOLVER_ROUTE
    } else if !matches!(value.rng_contract, VCKSS_RNG_NONE | VCKSS_RNG_COUNTER_V1) {
        VCKSS_REQUEST_REASON_UNKNOWN_RNG_CONTRACT
    } else if !matches!(
        value.frequency_use,
        VCKSS_REQUEST_FREQUENCY_UNIT | VCKSS_REQUEST_FREQUENCY_LITERAL
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_FREQUENCY_USE
    } else if !matches!(
        request.v2.engine,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED | VCKSS_ENGINE_COMPRESSED | VCKSS_ENGINE_GENERIC
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_ENGINE
    } else if !phase_modes_known {
        VCKSS_REQUEST_REASON_UNKNOWN_BATCH_MODE
    } else if !matches!(
        request.v2.batch_mode,
        VCKSS_BATCH_MODE_AUTO | VCKSS_BATCH_MODE_EXPLICIT | VCKSS_BATCH_MODE_INDEPENDENT
    ) || request.v2.batch_mode != expected_summary
    {
        VCKSS_REQUEST_REASON_BATCH_SUMMARY_MISMATCH
    } else if !matches!(
        request.v2.stayers_mode,
        VCKSS_STAYERS_MOVERS | VCKSS_STAYERS_ALL
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_STAYERS_MODE
    } else if !matches!(
        request.v2.target_weight_mode,
        VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT | VCKSS_TARGET_WEIGHT_STORED_ROW_EXPLICIT
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_TARGET_WEIGHT_MODE
    } else if !matches!(
        request.v2.deletion_unit_source,
        VCKSS_DELETION_SOURCE_CELL_DEFAULT
            | VCKSS_DELETION_SOURCE_MATCH_ID_EXPLICIT
            | VCKSS_DELETION_SOURCE_OBSERVATION_ROW
    ) {
        VCKSS_REQUEST_REASON_UNKNOWN_DELETION_UNIT_SOURCE
    } else if !deletion_source_matches {
        VCKSS_REQUEST_REASON_DELETION_UNIT_SOURCE_MISMATCH
    } else if request.v2.probeorder_supplied > 1 {
        VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED
    } else if request.v2.wallseconds_supplied > 1
        || (request.v2.wallseconds_supplied == 0 && request.wallseconds != 0.0)
        || (request.v2.wallseconds_supplied == 1
            && (!request.wallseconds.is_finite() || request.wallseconds <= 0.0))
    {
        VCKSS_REQUEST_REASON_WALLSECONDS_VALUE
    } else if request.allow_automatic_cmg_setup_fallback > 1
        || (request.allow_automatic_cmg_setup_fallback == 1
            && value.solver_route != VCKSS_ROUTE_AUTO)
    {
        VCKSS_REQUEST_REASON_FALLBACK_ROUTE_MISMATCH
    } else if request.v2.physical_limit == 0
        || request.v2.physical_limit > MAX_EXACT_BINARY64_INTEGER
    {
        VCKSS_REQUEST_REASON_PHYSICAL_LIMIT
    } else if value.controls_count > 32 {
        VCKSS_REQUEST_REASON_CONTROLS_LIMIT
    } else if value.algorithm == VCKSS_ALGORITHM_EXACT {
        if request.v2.engine == VCKSS_ENGINE_COMPRESSED {
            VCKSS_REQUEST_REASON_EXACT_ENGINE
        } else if request.v2.stayers_mode == VCKSS_STAYERS_ALL
            && value.deletion_mode != VCKSS_DELETION_MATCH
        {
            VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED
        } else if request.v2.stayers_mode == VCKSS_STAYERS_ALL
            && request.v2.probeorder_supplied != 0
        {
            VCKSS_REQUEST_REASON_PROBEORDER_UNSUPPORTED
        } else if request.v2.stayers_mode == VCKSS_STAYERS_ALL
            && request.v2.wallseconds_supplied != 0
        {
            VCKSS_REQUEST_REASON_WALLSECONDS_UNSUPPORTED
        } else if value.rng_contract != VCKSS_RNG_NONE {
            VCKSS_REQUEST_REASON_EXACT_RNG
        } else if !matches!(value.solver_route, VCKSS_ROUTE_AUTO | VCKSS_ROUTE_EXACT) {
            VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE
        } else {
            VCKSS_REQUEST_REASON_SUPPORTED
        }
    } else if value.algorithm == VCKSS_ALGORITHM_AUTO {
        if request.v2.stayers_mode != VCKSS_STAYERS_MOVERS {
            VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED
        } else if value.rng_contract != VCKSS_RNG_COUNTER_V1 {
            VCKSS_REQUEST_REASON_AUTO_RNG
        } else if value.solver_route != VCKSS_ROUTE_AUTO {
            VCKSS_REQUEST_REASON_EXACT_SOLVER_ROUTE
        } else if request.v2.engine == VCKSS_ENGINE_COMPRESSED {
            VCKSS_REQUEST_REASON_AUTO_ENGINE_COMPRESSED
        } else {
            VCKSS_REQUEST_REASON_SUPPORTED
        }
    } else if request.v2.stayers_mode != VCKSS_STAYERS_MOVERS {
        VCKSS_REQUEST_REASON_STAYERS_MODE_UNSUPPORTED
    } else if value.rng_contract != VCKSS_RNG_COUNTER_V1 {
        VCKSS_REQUEST_REASON_JLA_RNG
    } else if value.solver_route == VCKSS_ROUTE_EXACT {
        VCKSS_REQUEST_REASON_JLA_GENERIC_SOLVER_ROUTE
    } else if request.v2.engine == VCKSS_ENGINE_COMPRESSED
        && (value.deletion_mode != VCKSS_DELETION_MATCH || value.controls_count != 0)
    {
        VCKSS_REQUEST_REASON_COMPRESSED_SCIENTIFIC_INELIGIBILITY
    } else {
        VCKSS_REQUEST_REASON_SUPPORTED
    };
    let profile = if reason == VCKSS_REQUEST_REASON_SUPPORTED {
        VCKSS_REQUEST_PROFILE_PLANNED_V1
    } else {
        VCKSS_REQUEST_PROFILE_NONE
    };
    (reason, profile)
}

fn request_capability_signature_v3(request: VckssBackendRequestCapabilityRequestV3) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"VCKSS-REQUEST-CAPABILITY-V3" {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for value in [
        request.v2.v1.request_schema,
        request.v2.v1.algorithm,
        request.v2.v1.deletion_mode,
        request.v2.v1.nuisance_mode,
        request.v2.v1.solver_route,
        request.v2.v1.rng_contract,
        request.v2.v1.controls_count,
        request.v2.v1.frequency_use,
        request.v2.engine,
        request.v2.batch_mode,
        request.v2.stayers_mode,
        request.v2.target_weight_mode,
        request.v2.deletion_unit_source,
        request.v2.probeorder_supplied,
        request.v2.wallseconds_supplied,
        request.leverage_batch_mode,
        request.target_batch_mode,
        request.allow_automatic_cmg_setup_fallback,
    ] {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    for byte in request.v2.physical_limit.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for byte in request.wallseconds.to_bits().to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn option_usize(value: Option<usize>, label: &str) -> Result<u64> {
    value.map_or(Ok(0), |item| to_u64(item, label))
}

fn component_vector(value: VarianceComponents) -> VckssComponentVectorV1 {
    VckssComponentVectorV1 {
        worker: value.worker,
        firm: value.firm,
        covariance: value.covariance,
        total: value.total,
    }
}

fn mcse_vector(value: NumericalMcse) -> VckssComponentVectorV1 {
    VckssComponentVectorV1 {
        worker: value.worker,
        firm: value.firm,
        covariance: value.covariance,
        total: value.total,
    }
}

fn copy_columns(columns: &VckssEngineColumnsV1, rows: usize) -> Result<InputColumns> {
    copy_columns_with_interrupt(columns, rows, &mut NeverInterrupt)
}

fn copy_columns_with_interrupt(
    columns: &VckssEngineColumnsV1,
    rows: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<InputColumns> {
    copy_columns_with_identifier_mode_and_interrupt(columns, rows, false, interrupt)
}

fn copy_columns_with_identifier_mode_and_interrupt(
    columns: &VckssEngineColumnsV1,
    rows: usize,
    allow_signed_identifiers: bool,
    interrupt: &mut dyn InterruptCheck,
) -> Result<InputColumns> {
    Ok(InputColumns {
        worker: copy_integer_column(
            columns.worker,
            rows,
            "worker identifier",
            ErrorCode::InvalidIdentifier,
            allow_signed_identifiers,
            interrupt,
        )?,
        firm: copy_integer_column(
            columns.firm,
            rows,
            "firm identifier",
            ErrorCode::InvalidIdentifier,
            allow_signed_identifiers,
            interrupt,
        )?,
        deletion: copy_integer_column(
            columns.deletion,
            rows,
            "deletion identifier",
            ErrorCode::InvalidIdentifier,
            allow_signed_identifiers,
            interrupt,
        )?,
        outcome: copy_finite_column(columns.outcome, rows, "outcome", interrupt)?,
        frequency: copy_positive_integer_column(
            columns.frequency,
            rows,
            "frequency weight",
            ErrorCode::InvalidWeight,
            interrupt,
        )?,
        target_weight: copy_nonnegative_column(
            columns.target_weight,
            rows,
            "target weight",
            interrupt,
        )?,
        controls: Vec::new(),
    })
}

fn copy_columns_v2_with_interrupt(
    columns: &VckssEngineColumnsV2,
    rows: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<InputColumns> {
    copy_columns_v2_with_identifier_mode_and_interrupt(columns, rows, false, interrupt)
}

fn copy_columns_v2_with_identifier_mode_and_interrupt(
    columns: &VckssEngineColumnsV2,
    rows: usize,
    allow_signed_identifiers: bool,
    interrupt: &mut dyn InterruptCheck,
) -> Result<InputColumns> {
    let mut output = copy_columns_with_identifier_mode_and_interrupt(
        &columns.v1,
        rows,
        allow_signed_identifiers,
        interrupt,
    )?;
    let controls = usize::try_from(columns.controls_count).map_err(|_| {
        resource_error(
            "engine_prepare",
            "control-column count is not representable",
        )
    })?;
    if controls == 0 {
        if !columns.controls.is_null() {
            return Err(BackendError::invalid(
                "engine_prepare",
                "zero controls require a null control-pointer array",
            ));
        }
        return Ok(output);
    }
    if columns.controls.is_null() {
        return Err(BackendError::invalid(
            "engine_prepare",
            "positive control count requires a control-pointer array",
        ));
    }
    output.controls.reserve(controls);
    for control in 0..controls {
        checkpoint_chunk(interrupt, control, "engine_copy_control_descriptors")?;
        // SAFETY: the caller's V2 descriptor contract provides an array with
        // `controls_count` entries that remains valid for this synchronous
        // call.  Every pointed-to column is copied before returning.
        let pointer = unsafe { columns.controls.add(control).read_unaligned() };
        output
            .controls
            .push(copy_finite_column(pointer, rows, "control", interrupt)?);
    }
    Ok(output)
}

fn copy_stayer_augmentation_columns(
    columns: &VckssStayerAugmentationColumnsV1,
    rows: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<StayerAugmentationInput> {
    let controls = usize::try_from(columns.controls_count).map_err(|_| {
        resource_error(
            "engine_stayer_augmentation",
            "control-column count is not representable",
        )
    })?;
    if rows == 0 {
        if !columns.firm.is_null()
            || !columns.worker.is_null()
            || !columns.outcome.is_null()
            || !columns.frequency.is_null()
            || !columns.target_weight.is_null()
            || !columns.controls.is_null()
        {
            return Err(BackendError::invalid(
                "engine_stayer_augmentation",
                "zero stayer rows require null numeric and control pointers",
            ));
        }
        return Ok(StayerAugmentationInput {
            firm: Vec::new(),
            worker: Vec::new(),
            outcome: Vec::new(),
            frequency: Vec::new(),
            target_weight: Vec::new(),
            controls: vec![Vec::new(); controls],
        });
    }
    let mut control_values = Vec::with_capacity(controls);
    if controls == 0 {
        if !columns.controls.is_null() {
            return Err(BackendError::invalid(
                "engine_stayer_augmentation",
                "zero controls require a null control-pointer array",
            ));
        }
    } else {
        if columns.controls.is_null() {
            return Err(BackendError::invalid(
                "engine_stayer_augmentation",
                "positive control count requires a control-pointer array",
            ));
        }
        for control in 0..controls {
            checkpoint_chunk(interrupt, control, "engine_stayer_copy_control_descriptors")?;
            // SAFETY: the descriptor owns `controls_count` readable pointers
            // for this synchronous call; each column is copied immediately.
            let pointer = unsafe { columns.controls.add(control).read_unaligned() };
            control_values.push(copy_finite_column(
                pointer,
                rows,
                "stayer control",
                interrupt,
            )?);
        }
    }
    Ok(StayerAugmentationInput {
        firm: copy_positive_integer_column(
            columns.firm,
            rows,
            "stayer firm identifier",
            ErrorCode::InvalidIdentifier,
            interrupt,
        )?,
        worker: copy_positive_integer_column(
            columns.worker,
            rows,
            "stayer worker identifier",
            ErrorCode::InvalidIdentifier,
            interrupt,
        )?,
        outcome: copy_finite_column(columns.outcome, rows, "stayer outcome", interrupt)?,
        frequency: copy_positive_integer_column(
            columns.frequency,
            rows,
            "stayer frequency weight",
            ErrorCode::InvalidWeight,
            interrupt,
        )?,
        target_weight: copy_nonnegative_column(
            columns.target_weight,
            rows,
            "stayer target weight",
            interrupt,
        )?,
        controls: control_values,
    })
}

fn deletion_from_code(code: u32) -> Result<DeletionMode> {
    match code {
        VCKSS_DELETION_MATCH => Ok(DeletionMode::Match),
        VCKSS_DELETION_OBSERVATION => Ok(DeletionMode::Observation),
        _ => Err(BackendError::invalid(
            "engine_options",
            "unknown deletion-mode code",
        )),
    }
}

fn nuisance_from_code(code: u32) -> Result<NuisanceMode> {
    match code {
        VCKSS_NUISANCE_JOINT => Ok(NuisanceMode::Joint),
        VCKSS_NUISANCE_FIXED_OFFSET => Ok(NuisanceMode::FixedOffset),
        _ => Err(BackendError::invalid(
            "engine_options",
            "unknown nuisance-mode code",
        )),
    }
}

fn algorithm_from_code(code: u32) -> Result<u32> {
    match code {
        VCKSS_ALGORITHM_AUTO | VCKSS_ALGORITHM_EXACT | VCKSS_ALGORITHM_JLA => Ok(code),
        _ => Err(BackendError::invalid(
            "engine_solve",
            "unknown estimator algorithm code",
        )),
    }
}

fn algorithm_request_from_code(code: u32) -> Result<AlgorithmRequest> {
    match code {
        VCKSS_ALGORITHM_AUTO => Ok(AlgorithmRequest::Auto),
        VCKSS_ALGORITHM_EXACT => Ok(AlgorithmRequest::Exact),
        VCKSS_ALGORITHM_JLA => Ok(AlgorithmRequest::Jla),
        _ => Err(BackendError::invalid(
            "engine_solve",
            "unknown estimator algorithm code",
        )),
    }
}

fn engine_request_from_code(code: u32) -> Result<EngineRequest> {
    match code {
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED => Ok(EngineRequest::Auto),
        VCKSS_ENGINE_COMPRESSED => Ok(EngineRequest::Compressed),
        VCKSS_ENGINE_GENERIC => Ok(EngineRequest::Generic),
        _ => Err(BackendError::invalid(
            "engine_solve",
            "unknown estimator engine code",
        )),
    }
}

fn batch_request_from_code(code: u32, width: u32, phase: &str) -> Result<BatchRequest> {
    match code {
        VCKSS_BATCH_MODE_AUTO if width == 0 => Ok(BatchRequest::Auto),
        VCKSS_BATCH_MODE_EXPLICIT if width > 0 => usize::try_from(width)
            .map(BatchRequest::Explicit)
            .map_err(|_| resource_error("engine_solve", "batch width is not representable")),
        VCKSS_BATCH_MODE_AUTO => Err(BackendError::invalid(
            "engine_solve",
            format!("automatic {phase} batch mode requires a zero width"),
        )),
        VCKSS_BATCH_MODE_EXPLICIT => Err(BackendError::invalid(
            "engine_solve",
            format!("explicit {phase} batch mode requires a positive width"),
        )),
        _ => Err(BackendError::invalid(
            "engine_solve",
            format!("unknown {phase} batch mode"),
        )),
    }
}

fn validate_full_cmg_v2_request(
    request: VckssEngineSolveRequestV5,
) -> Result<Option<FullCmgPlanOptions>> {
    if request.v4.v3.v2.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV5>()? {
        return Err(abi_error("V5 solve request reports a short structure size"));
    }
    if request.reserved_5 != 0 || request.tolerance_supplied > 1 || request.full_cmg_v2 > 1 {
        return Err(abi_error(
            "invalid V5 solve request flags or reserved field",
        ));
    }
    if request.full_cmg_v2 == 0 {
        return Ok(None);
    }
    if request.threads == 0 {
        return Err(BackendError::invalid(
            "cmg_full_v2",
            "CMG_FULL_V2 requires a positive caller-declared thread count",
        ));
    }
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "cmg_full_v2",
            "CMG_FULL_V2 is currently qualified only on macOS and Linux",
        ));
    }
    let value = request.v4;
    let eligible = value.v3.v2.algorithm == VCKSS_ALGORITHM_JLA
        && value.v3.engine == VCKSS_ENGINE_AUTO_OR_UNSPECIFIED
        && value.v3.v2.v1.deletion_mode == VCKSS_DELETION_MATCH
        && value.v3.v2.nuisance_mode == VCKSS_NUISANCE_JOINT
        && value.v3.v2.v1.rng_contract == VCKSS_RNG_COUNTER_V1
        && value.v3.v2.v1.solver_route == VCKSS_ROUTE_AUTO
        && value.v3.batch_mode == VCKSS_BATCH_MODE_AUTO
        && value.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO
        && value.target_batch_mode == VCKSS_BATCH_MODE_AUTO
        && value.v3.v2.v1.leverage_batch_width == 0
        && value.v3.v2.v1.target_batch_width == 0
        && value.v3.stayers_mode == VCKSS_STAYERS_MOVERS
        && value.v3.target_weight_mode == VCKSS_TARGET_WEIGHT_FREQUENCY_DEFAULT
        && value.v3.deletion_unit_source == VCKSS_DELETION_SOURCE_CELL_DEFAULT
        && value.v3.probeorder_supplied == 1
        && value.v3.frequency_use == VCKSS_REQUEST_FREQUENCY_UNIT;
    if !eligible {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "cmg_full_v2",
            "CMG_FULL_V2 request is outside the qualified JLA/auto/match/joint/movers cell",
        ));
    }
    let probe_tolerance = (request.tolerance_supplied == 1).then_some(value.v3.v2.v1.pcg_tolerance);
    Ok(Some(FullCmgPlanOptions::production(
        usize::try_from(request.threads)
            .map_err(|_| resource_error("cmg_full_v2", "thread count is not representable"))?,
        value.v3.v2.v1.pcg_tolerance,
        probe_tolerance,
    )))
}

fn capability_request_v3_for_solve(
    request: VckssEngineSolveRequestV4,
    controls_count: u32,
) -> Result<VckssBackendRequestCapabilityRequestV3> {
    Ok(VckssBackendRequestCapabilityRequestV3 {
        v2: VckssBackendRequestCapabilityRequestV2 {
            v1: VckssBackendRequestCapabilityRequestV1 {
                abi_version: request.v3.v2.v1.abi_version,
                struct_size: struct_size_u32::<VckssBackendRequestCapabilityRequestV3>()?,
                request_schema: request.v3.capability_schema,
                algorithm: request.v3.v2.algorithm,
                deletion_mode: request.v3.v2.v1.deletion_mode,
                nuisance_mode: request.v3.v2.nuisance_mode,
                solver_route: request.v3.v2.v1.solver_route,
                rng_contract: request.v3.v2.v1.rng_contract,
                controls_count,
                frequency_use: request.v3.frequency_use,
                reserved: 0,
            },
            engine: request.v3.engine,
            batch_mode: request.v3.batch_mode,
            stayers_mode: request.v3.stayers_mode,
            target_weight_mode: request.v3.target_weight_mode,
            deletion_unit_source: request.v3.deletion_unit_source,
            probeorder_supplied: request.v3.probeorder_supplied,
            wallseconds_supplied: request.v3.wallseconds_supplied,
            reserved_2: 0,
            physical_limit: request.v3.physical_limit,
        },
        leverage_batch_mode: request.leverage_batch_mode,
        target_batch_mode: request.target_batch_mode,
        allow_automatic_cmg_setup_fallback: request.v3.v2.v1.allow_automatic_cmg_setup_fallback,
        reserved_3: 0,
        wallseconds: request.wallseconds,
        reserved_4: 0,
    })
}

fn model_routing_from_request(request: VckssEngineSolveRequestV1) -> Result<ModelRoutingOptions> {
    let route = match request.solver_route {
        VCKSS_ROUTE_AUTO => ModelSolverRoute::Auto,
        VCKSS_ROUTE_DIAGONAL_PCG => ModelSolverRoute::Diagonal,
        VCKSS_ROUTE_CMG_PCG => ModelSolverRoute::Cmg,
        _ => {
            return Err(BackendError::invalid(
                "engine_solve",
                "generic JLA requires auto, diagonal, or CMG routing",
            ))
        }
    };
    Ok(ModelRoutingOptions {
        route,
        cmg_minimum_dimension: to_usize(
            request.cmg_minimum_dimension,
            "engine_solve",
            "CMG minimum dimension",
        )?,
        allow_automatic_cmg_setup_fallback: request.allow_automatic_cmg_setup_fallback == 1,
        solver: ModelSolverOptions {
            pcg: PcgOptions {
                tolerance: request.pcg_tolerance,
                maximum_iterations: request.maximum_iterations,
                residual_replacement_interval: request.residual_replacement_interval,
            },
            rank_tolerance: request.rank_tolerance,
        },
        cmg: CmgOptions {
            terminal_vertices: to_usize(
                request.cmg_terminal_vertices,
                "engine_solve",
                "CMG terminal vertices",
            )?,
            dense_vertex_cap: to_usize(
                request.cmg_dense_vertex_cap,
                "engine_solve",
                "CMG dense vertex cap",
            )?,
            maximum_levels: to_usize(
                request.cmg_maximum_levels,
                "engine_solve",
                "CMG maximum levels",
            )?,
            aggregate_cap: to_usize(
                request.cmg_aggregate_cap,
                "engine_solve",
                "CMG aggregate cap",
            )?,
            minimum_reduction: request.cmg_minimum_reduction,
            jacobi_weight: request.cmg_jacobi_weight,
            pre_sweeps: request.cmg_pre_sweeps,
            post_sweeps: request.cmg_post_sweeps,
            maximum_edge_complexity: request.cmg_maximum_edge_complexity,
            maximum_vertex_complexity: request.cmg_maximum_vertex_complexity,
            memory_limit_bytes: request.cmg_memory_limit_bytes,
        },
    })
}

/// Validate every exact-routing option before the context enters its solving
/// state. This keeps invalid limits and tolerances ahead of dense allocation
/// and, for automatic requests that later select JLA, ahead of estimator RNG.
fn validate_exact_request_options(request: VckssEngineSolveRequestV2) -> Result<(usize, usize)> {
    if !request.v1.rank_tolerance.is_finite()
        || request.v1.rank_tolerance < 1.0e-14
        || request.v1.rank_tolerance >= 0.1
    {
        return Err(BackendError::invalid(
            "exact_estimator",
            "rank tolerance must be finite and lie in [1e-14, 0.1)",
        ));
    }
    if !request.v1.block_tolerance.is_finite()
        || request.v1.block_tolerance < 1.0e-14
        || request.v1.block_tolerance >= 1.0
    {
        return Err(BackendError::invalid(
            "exact_estimator",
            "block tolerance must be finite and lie in [1e-14, 1)",
        ));
    }
    if !request.v1.pcg_tolerance.is_finite()
        || request.v1.pcg_tolerance <= 0.0
        || request.v1.pcg_tolerance >= 0.1
    {
        return Err(BackendError::invalid(
            "exact_estimator",
            "solver tolerance must be finite and lie in (0, 0.1)",
        ));
    }
    let exact_limit = to_usize(
        request.exact_estimator_limit,
        "engine_solve",
        "exact estimator limit",
    )?;
    if !(2..=2_000).contains(&exact_limit) {
        return Err(BackendError::invalid(
            "exact_estimator",
            "exact limit must lie in [2, 2000]",
        ));
    }
    let blocksize_limit = to_usize(request.blocksize_limit, "engine_solve", "block-size limit")?;
    if !(1..=1_000_000).contains(&blocksize_limit) {
        return Err(BackendError::invalid(
            "exact_estimator",
            "block-size limit must lie in [1, 1000000]",
        ));
    }
    Ok((exact_limit, blocksize_limit))
}

fn validate_generic_request_options(request: VckssEngineSolveRequestV3) -> Result<usize> {
    let (_, blocksize_limit) = validate_exact_request_options(request.v2)?;
    if request.v2.v1.solver_route != VCKSS_ROUTE_DIAGONAL_PCG {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "generic JLA currently requires the diagonal PCG route",
        ));
    }
    if request.v2.v1.allow_automatic_cmg_setup_fallback != 0 {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "generic JLA does not permit solver fallback",
        ));
    }
    if request.v2.v1.rng_contract != VCKSS_RNG_COUNTER_V1 {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "engine_solve",
            "generic JLA requires Counter-V1 RNG",
        ));
    }
    if request.batch_mode != VCKSS_BATCH_MODE_EXPLICIT
        || request.v2.v1.leverage_batch_width == 0
        || request.v2.v1.target_batch_width == 0
    {
        return Err(BackendError::invalid(
            "engine_solve",
            "generic JLA requires positive explicit batch widths",
        ));
    }
    if request.v2.v1.probes < 2 {
        return Err(BackendError::invalid(
            "engine_solve",
            "generic JLA requires at least two probes",
        ));
    }
    if request.physical_limit == 0 || request.physical_limit > MAX_EXACT_BINARY64_INTEGER {
        return Err(BackendError::invalid(
            "engine_solve",
            "physical_limit must lie in [1, 2^53]",
        ));
    }
    PcgOptions {
        tolerance: request.v2.v1.pcg_tolerance,
        maximum_iterations: request.v2.v1.maximum_iterations,
        residual_replacement_interval: request.v2.v1.residual_replacement_interval,
    }
    .validate()?;
    Ok(blocksize_limit)
}

fn generic_rhs_export_memory(
    controls: u32,
    probes: u32,
    nuisance: NuisanceMode,
) -> Result<(u64, u64)> {
    let distinct_working = u64::from(nuisance == NuisanceMode::FixedOffset && controls != 0);
    let rows = u64::from(controls)
        .checked_add(1)
        .and_then(|value| value.checked_add(distinct_working))
        .and_then(|value| value.checked_add(u64::from(probes).checked_mul(3)?))
        .ok_or_else(|| resource_error("engine_solve", "generic RHS receipt count overflow"))?;
    let native_row = u64::try_from(size_of::<VckssEngineRhsReceiptV2>()).map_err(|_| {
        resource_error(
            "engine_solve",
            "native generic RHS receipt size is not representable",
        )
    })?;
    let caller_row = 15_u64.checked_mul(8).ok_or_else(|| {
        resource_error(
            "engine_solve",
            "caller generic RHS receipt row size overflow",
        )
    })?;
    let per_row = native_row.checked_add(caller_row).ok_or_else(|| {
        resource_error(
            "engine_solve",
            "combined generic RHS export row size overflow",
        )
    })?;
    let bytes = rows.checked_mul(per_row).ok_or_else(|| {
        resource_error(
            "engine_solve",
            "combined generic RHS export byte count overflow",
        )
    })?;
    Ok((rows, bytes))
}

const fn deletion_code(mode: DeletionMode) -> u32 {
    match mode {
        DeletionMode::Match => VCKSS_DELETION_MATCH,
        DeletionMode::Observation => VCKSS_DELETION_OBSERVATION,
    }
}

const fn nuisance_code(mode: NuisanceMode) -> u32 {
    match mode {
        NuisanceMode::Joint => VCKSS_NUISANCE_JOINT,
        NuisanceMode::FixedOffset => VCKSS_NUISANCE_FIXED_OFFSET,
    }
}

fn copy_positive_integer_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
    error_code: ErrorCode,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    copy_integer_column(pointer, rows, label, error_code, false, interrupt)
}

fn copy_integer_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
    error_code: ErrorCode,
    allow_signed: bool,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let source = copy_f64_slice(pointer, rows, label, interrupt)?;
    let mut output = Vec::with_capacity(rows);
    for (row, value) in source.into_iter().enumerate() {
        checkpoint_chunk(interrupt, row, "engine_ingest_integer")?;
        let in_range = if allow_signed {
            value >= -(MAX_EXACT_BINARY64_INTEGER as f64)
                && value <= MAX_EXACT_BINARY64_INTEGER as f64
        } else {
            value > 0.0 && value <= MAX_EXACT_BINARY64_INTEGER as f64
        };
        if !value.is_finite() || !in_range || value.fract() != 0.0 {
            return Err(BackendError::new(
                error_code,
                "engine_ingest",
                format!(
                    "{label} must be a {} exact binary64 integer at zero-based row {row}",
                    if allow_signed { "signed" } else { "positive" }
                ),
            ));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let encoded = if allow_signed {
            let signed = value as i64;
            (signed as u64) ^ (1_u64 << 63)
        } else {
            value as u64
        };
        output.push(encoded);
    }
    Ok(output)
}

fn copy_finite_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let source = copy_f64_slice(pointer, rows, label, interrupt)?;
    for (row, value) in source.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "engine_ingest_finite")?;
        if !value.is_finite() {
            return Err(BackendError::invalid(
                "engine_ingest",
                format!("{label} is nonfinite at zero-based row {row}"),
            ));
        }
    }
    Ok(source)
}

fn copy_nonnegative_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    let source = copy_f64_slice(pointer, rows, label, interrupt)?;
    for (row, value) in source.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "engine_ingest_nonnegative")?;
        if !value.is_finite() || *value < 0.0 {
            return Err(BackendError::new(
                ErrorCode::InvalidTargetWeight,
                "engine_ingest",
                format!("{label} is negative or nonfinite at zero-based row {row}"),
            ));
        }
    }
    Ok(source)
}

fn copy_f64_slice(
    pointer: *const f64,
    rows: usize,
    label: &str,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<f64>> {
    if pointer.is_null() {
        return Err(BackendError::invalid(
            "engine_ingest",
            format!("{label} pointer is null"),
        ));
    }
    if (pointer as usize) % align_of::<f64>() != 0 {
        return Err(BackendError::invalid(
            "engine_ingest",
            format!("{label} pointer is not aligned for binary64 values"),
        ));
    }
    if rows > isize::MAX as usize / size_of::<f64>() {
        return Err(resource_error(
            "engine_ingest",
            "input column byte length exceeds the addressable slice limit",
        ));
    }
    let mut output = Vec::with_capacity(rows);
    for begin in (0..rows).step_by(vckss_core::interrupt::INTERRUPT_CHECK_CHUNK) {
        interrupt.checkpoint("engine_ingest_copy")?;
        let end = begin
            .saturating_add(vckss_core::interrupt::INTERRUPT_CHECK_CHUNK)
            .min(rows);
        // SAFETY: the C contract requires `rows` readable aligned doubles.
        // The full extent and alignment were checked above; each bounded
        // subslice is copied immediately and no caller pointer escapes.
        let chunk =
            unsafe { std::slice::from_raw_parts(pointer.add(begin), end.saturating_sub(begin)) };
        output.extend_from_slice(chunk);
    }
    interrupt.checkpoint("engine_ingest_copy_final")?;
    Ok(output)
}

fn copy_request_struct<T: Copy>(pointer: *const T, label: &str) -> Result<T> {
    let reported = read_u32_at(pointer.cast::<u8>(), size_of::<u32>(), label)?;
    require_struct_size::<T>(reported, label)?;
    // SAFETY: the C input contract provides at least the reported byte count;
    // the required V1 prefix was validated before this unaligned value copy.
    Ok(unsafe { pointer.read_unaligned() })
}

fn copy_sized_struct<T: Copy>(pointer: *const T, label: &str) -> Result<T> {
    let reported = read_u32_at(pointer.cast::<u8>(), 0, label)?;
    require_struct_size::<T>(reported, label)?;
    // SAFETY: see `copy_request_struct`; no reference into caller memory forms.
    Ok(unsafe { pointer.read_unaligned() })
}

fn read_u32_at(pointer: *const u8, offset: usize, label: &str) -> Result<u32> {
    if pointer.is_null() {
        return Err(BackendError::invalid(
            "engine_ffi",
            format!("{label} pointer is null"),
        ));
    }
    // SAFETY: every ABI input must provide its fixed two-word header. Reading
    // unaligned permits byte-packed C call frames and creates no Rust reference.
    Ok(unsafe { pointer.wrapping_add(offset).cast::<u32>().read_unaligned() })
}

fn require_output_capacity<T>(pointer: *mut u8, capacity: u32, label: &str) -> Result<()> {
    let required = struct_size_u32::<T>()?;
    if pointer.is_null() {
        return Err(BackendError::invalid(
            "engine_ffi",
            format!("{label} pointer is null"),
        ));
    }
    if capacity < required {
        return Err(abi_error(format!(
            "{label} capacity {capacity} is below required size {required}"
        )));
    }
    Ok(())
}

fn write_output<T: Copy>(pointer: *mut T, value: T) {
    // SAFETY: the caller's output capacity was checked by the public function;
    // unaligned writes avoid forming a reference with Rust alignment demands.
    unsafe { pointer.write_unaligned(value) };
}

fn require_struct_size<T>(reported: u32, label: &str) -> Result<()> {
    let required = struct_size_u32::<T>()?;
    if reported < required {
        Err(abi_error(format!(
            "{label} size {reported} is below required size {required}"
        )))
    } else {
        Ok(())
    }
}

fn struct_size_u32<T>() -> Result<u32> {
    u32::try_from(size_of::<T>()).map_err(|_| {
        resource_error(
            "engine_ffi",
            "ABI structure size is not representable as u32",
        )
    })
}

fn to_usize(value: u64, phase: &'static str, label: &str) -> Result<usize> {
    usize::try_from(value)
        .map_err(|_| resource_error(phase, &format!("{label} is not representable as usize")))
}

fn to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value)
        .map_err(|_| resource_error("engine_receipt", &format!("{label} is not representable")))
}

fn to_u32(value: usize, label: &str) -> Result<u32> {
    u32::try_from(value)
        .map_err(|_| resource_error("engine_receipt", &format!("{label} is not representable")))
}

fn route_from_code(value: u32) -> Result<LinearSolverRoute> {
    match value {
        VCKSS_ROUTE_AUTO => Ok(LinearSolverRoute::Auto),
        VCKSS_ROUTE_EXACT => Ok(LinearSolverRoute::Exact),
        VCKSS_ROUTE_DIAGONAL_PCG => Ok(LinearSolverRoute::DiagonalPcg),
        VCKSS_ROUTE_CMG_PCG => Ok(LinearSolverRoute::CmgPcg),
        _ => Err(BackendError::invalid(
            "engine_solve",
            format!("unknown solver route code {value}"),
        )),
    }
}

const fn route_code(value: LinearSolverRoute) -> u32 {
    match value {
        LinearSolverRoute::Auto => VCKSS_ROUTE_AUTO,
        LinearSolverRoute::Exact => VCKSS_ROUTE_EXACT,
        LinearSolverRoute::DiagonalPcg => VCKSS_ROUTE_DIAGONAL_PCG,
        LinearSolverRoute::CmgPcg => VCKSS_ROUTE_CMG_PCG,
    }
}

fn state_code(state: ContextStateTag) -> u32 {
    match state {
        ContextStateTag::Empty => 0,
        ContextStateTag::Prepared => 1,
        ContextStateTag::Solving => 2,
        ContextStateTag::Solved => 3,
        ContextStateTag::Failed => 4,
        ContextStateTag::Poisoned => 5,
    }
}

fn require_abi(requested: u32) -> Result<()> {
    if requested == ABI_VERSION {
        Ok(())
    } else {
        Err(abi_error(format!(
            "requested ABI {requested} does not match plugin ABI {ABI_VERSION}"
        )))
    }
}

fn lock_engine(phase: &'static str) -> Result<std::sync::MutexGuard<'static, EngineState>> {
    engine().lock().map_err(|_| {
        BackendError::new(
            ErrorCode::ContextPoisoned,
            phase,
            "native numerical engine registry lock is poisoned",
        )
    })
}

fn clear_abandoned_before_replacement(cleanup_abandoned: u32) -> Result<()> {
    if cleanup_abandoned == 1 {
        lock_engine("engine_prepare_cleanup")?
            .registry
            .clear_abandoned();
    }
    Ok(())
}

fn abi_error(message: impl Into<String>) -> BackendError {
    BackendError::new(ErrorCode::AbiMismatch, "engine_ffi", message)
}

fn resource_error(phase: &'static str, message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, phase, message)
}

fn store_error_text(value: &str) {
    ENGINE_LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = cstring_without_nul(value);
    });
}

fn cstring_without_nul(value: &str) -> CString {
    let bytes = value
        .as_bytes()
        .iter()
        .copied()
        .map(|byte| if byte == 0 { b'?' } else { byte })
        .collect::<Vec<_>>();
    CString::new(bytes).unwrap_or_else(|_| CString::new("invalid string").expect("literal CString"))
}

#[cfg(test)]
mod tests {
    use super::{
        apply_prepared_memory_admission, component_identity_residual, copy_integer_column,
        VarianceComponents,
    };
    use vckss_core::engine::JlaEngineOptions;
    use vckss_core::error::ErrorCode;
    use vckss_core::interrupt::NeverInterrupt;

    #[test]
    fn actual_accounting_residual_detects_a_perturbed_identity() {
        let valid = VarianceComponents {
            worker: 3.0,
            firm: 5.0,
            covariance: -0.75,
            total: 6.5,
        };
        assert_eq!(component_identity_residual(valid), 0.0);
        let perturbed = VarianceComponents {
            total: valid.total + 0.125,
            ..valid
        };
        assert_eq!(component_identity_residual(perturbed), 0.125);
    }

    #[test]
    fn production_full_cmg_inherits_the_prepared_whole_command_limit() {
        let mut options = JlaEngineOptions::default();
        let legacy_cmg_limit = options.solver.cmg.memory_limit_bytes;
        let admitted_limit = 48_u64 << 30;
        let prepared_persistent = 3_u64 << 30;

        apply_prepared_memory_admission(&mut options, admitted_limit, prepared_persistent, true);

        assert_eq!(options.memory_limit_bytes, admitted_limit);
        assert_eq!(options.prepared_persistent_bytes, prepared_persistent);
        assert_eq!(options.solver.cmg.memory_limit_bytes, admitted_limit);
        assert_ne!(legacy_cmg_limit, admitted_limit);

        let mut legacy_options = JlaEngineOptions::default();
        apply_prepared_memory_admission(
            &mut legacy_options,
            admitted_limit,
            prepared_persistent,
            false,
        );
        assert_eq!(
            legacy_options.solver.cmg.memory_limit_bytes,
            legacy_cmg_limit
        );
    }

    #[test]
    fn signed_identifier_requires_explicit_implicit_match_ingest() {
        let values = [-2.0, 0.0, 1.0];
        let accepted = copy_integer_column(
            values.as_ptr(),
            values.len(),
            "test identifier",
            ErrorCode::InvalidIdentifier,
            true,
            &mut NeverInterrupt,
        )
        .expect("explicit implicit-match ingest should admit signed exact identifiers");
        assert_eq!(
            accepted,
            vec![
                (u64::MAX - 1) ^ (1_u64 << 63),
                1_u64 << 63,
                (1_u64 << 63) + 1,
            ]
        );

        let error = copy_integer_column(
            values.as_ptr(),
            values.len(),
            "test identifier",
            ErrorCode::InvalidIdentifier,
            false,
            &mut NeverInterrupt,
        )
        .expect_err("ordinary ingest must retain the positive-identifier contract");
        assert_eq!(error.code, ErrorCode::InvalidIdentifier);
        assert!(error
            .message
            .contains("must be a positive exact binary64 integer"));
    }
}
