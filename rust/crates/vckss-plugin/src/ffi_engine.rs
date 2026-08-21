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

use vckss_core::cmg::CmgOptions;
use vckss_core::engine::{
    run_jla_no_controls_with_interrupt, run_jla_no_controls_with_plan_and_interrupt,
    JlaEngineOptions, JlaEngineResult, JlaRhsReceipt, JlaRhsSide, JlaSolvePhase, NumericalMcse,
};
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::interrupt::{checkpoint_chunk, InterruptCheck, NeverInterrupt};
use vckss_core::jla::VarianceComponents;
use vckss_core::krylov::PcgOptions;
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute};
use vckss_core::types::{DeletionMode, InputColumns, RngContract, MAX_EXACT_BINARY64_INTEGER};
use vckss_core::{Capabilities, ABI_VERSION};

use crate::context::{ContextHandle, ContextPayloadRef, ContextRegistry, ContextStateTag};
use crate::session::{admit_prepare_memory, PreparationMemoryReceipt, PreparationReceipt};
use crate::session_retained::PreparedProblemWithMask;

pub const VCKSS_DELETION_MATCH: u32 = 1;
pub const VCKSS_RNG_COUNTER_V1: u32 = 1;
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
pub const VCKSS_INTERRUPT_CONTINUE: i32 = 0;
pub const VCKSS_INTERRUPT_USER_BREAK: i32 = 1;

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
    pub full_fit_reduced_residual: f64,
    pub full_fit_complete_residual: f64,
    pub full_fit_zero_rhs: u32,
    pub reserved_1: u32,
    pub leverage_rhs_count: u64,
    pub target_rhs_count: u64,
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
    pub full_fit_reduced_residual: f64,
    pub full_fit_complete_residual: f64,
    pub full_fit_zero_rhs: u32,
    pub reserved_1: u32,
    pub leverage_rhs_count: u64,
    pub target_rhs_count: u64,
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
struct EngineSolved {
    result: JlaEngineResult,
    preparation: PreparationReceipt,
    retained: Arc<Vec<bool>>,
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
        let capabilities = Capabilities::current();
        write_output(
            output,
            VckssBackendCapabilitiesV1 {
                struct_size: struct_size_u32::<VckssBackendCapabilitiesV1>()?,
                abi_version: capabilities.abi_version,
                core_ready_flags: core_ready_flags(capabilities),
                support_flags: support_flags(capabilities),
                deterministic_parallelism: u32::from(capabilities.deterministic_parallelism),
                reserved: 0,
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
        let input = copy_columns(&columns, rows)?;
        let prepared = PreparedProblemWithMask::from_columns(input)?;

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
    let input = copy_columns_with_interrupt(&columns, rows, interrupt)?;
    let prepared =
        PreparedProblemWithMask::from_columns_with_memory_and_interrupt(input, memory, interrupt)?;
    interrupt.checkpoint("engine_prepare_final")?;

    let mut state = lock_engine("engine_prepare")?;
    let handle = state.registry.prepare(prepared)?;
    write_output(output_handle, handle.generation());
    Ok(())
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

fn solve_engine(
    generation: u64,
    request: VckssEngineSolveRequestV1,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let options = options_from_request(request)?;
    let handle = ContextHandle::from_generation(generation)?;
    let mut state = lock_engine("engine_solve")?;
    state.registry.solve_preserving(handle, |prepared| {
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
        let result = if prepared.receipt.memory.hard_limit_bytes == 0 {
            run_jla_no_controls_with_interrupt(&prepared.problem, admitted_options, interrupt)?
        } else {
            run_jla_no_controls_with_plan_and_interrupt(
                &prepared.problem,
                &prepared.plan,
                admitted_options,
                interrupt,
            )?
        };
        Ok(EngineSolved {
            result,
            preparation: prepared.receipt,
            retained: Arc::clone(&prepared.retained),
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
        result.plugin.verify_accounting(1.0e-10)?;
        result.correction.verify_accounting(1.0e-10)?;
        result.corrected.verify_accounting(1.0e-10)?;
        write_output(
            output,
            VckssEngineResultV1 {
                struct_size: struct_size_u32::<VckssEngineResultV1>()?,
                reserved: 0,
                generation,
                plugin: component_vector(result.plugin),
                correction: component_vector(result.correction),
                corrected: component_vector(result.corrected),
                numerical_mcse: mcse_vector(result.numerical_mcse),
            },
        );
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
        let receipt = &state.registry.result(handle)?.result.receipt;
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
        let rows = rhs_receipt_count(&solved.result.receipt)?;
        let caller_result_copy_bytes = rows
            .checked_mul(14)
            .and_then(|value| value.checked_mul(8))
            .ok_or_else(|| {
                resource_error(
                    "engine_detailed_receipt",
                    "caller RHS receipt matrix byte count overflow",
                )
            })?;
        write_output(
            output,
            VckssEngineDetailedReceiptV3 {
                v2: detailed_receipt_v2(generation, solved)?,
                rng_contract: rng_contract_code(solved.result.receipt.rng),
                reserved_2: 0,
                rhs_receipt_rows: rows,
                caller_result_copy_bytes,
            },
        );
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
        let receipt = &state.registry.result(handle)?.result.receipt;
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

fn detailed_receipt_v2(
    generation: u64,
    solved: &EngineSolved,
) -> Result<VckssEngineDetailedReceiptV2> {
    let base = detailed_receipt(generation, &solved.result.receipt)?;
    let preparation_memory = solved.preparation.memory;
    let solve_memory = solved.result.receipt.memory;
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
        full_fit_weighted_rss: solved.result.weighted_rss,
        memory_limit_bytes: preparation_memory.hard_limit_bytes,
        caller_copy_bytes: preparation_memory.caller_copy_bytes,
        preparation_peak_forecast_bytes: preparation_memory.preparation_peak_forecast_bytes,
        prepared_resident_bytes: preparation_memory.prepared_resident_bytes,
        solver_setup_forecast_bytes: solve_memory.solver_setup_forecast_bytes,
        leverage_phase_forecast_bytes: solve_memory.leverage_phase_forecast_bytes,
        target_phase_forecast_bytes: solve_memory.target_phase_forecast_bytes,
        result_forecast_bytes: solve_memory.result_forecast_bytes,
        solve_peak_forecast_bytes: solve_memory.solve_peak_forecast_bytes,
        command_peak_forecast_bytes: preparation_memory
            .preparation_peak_forecast_bytes
            .max(solve_memory.solve_peak_forecast_bytes),
    })
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

fn core_ready_flags(capabilities: Capabilities) -> u64 {
    enabled_flag(
        capabilities.core_match_graph_ready,
        VCKSS_CORE_MATCH_GRAPH_READY,
    ) | enabled_flag(capabilities.core_exact_ready, VCKSS_CORE_EXACT_READY)
        | enabled_flag(
            capabilities.core_diagonal_pcg_ready,
            VCKSS_CORE_DIAGONAL_PCG_READY,
        )
        | enabled_flag(
            capabilities.core_batched_pcg_ready,
            VCKSS_CORE_BATCHED_PCG_READY,
        )
        | enabled_flag(
            capabilities.core_cmg_graph_ready,
            VCKSS_CORE_CMG_GRAPH_READY,
        )
        | enabled_flag(
            capabilities.core_solver_router_ready,
            VCKSS_CORE_SOLVER_ROUTER_READY,
        )
        | enabled_flag(
            capabilities.core_counter_rng_ready,
            VCKSS_CORE_COUNTER_RNG_READY,
        )
        | enabled_flag(capabilities.core_jla_plan_ready, VCKSS_CORE_JLA_PLAN_READY)
}

fn support_flags(capabilities: Capabilities) -> u64 {
    enabled_flag(capabilities.supports_exact, VCKSS_SUPPORT_EXACT)
        | enabled_flag(capabilities.supports_jla, VCKSS_SUPPORT_JLA)
        | enabled_flag(
            capabilities.supports_match_deletion,
            VCKSS_SUPPORT_MATCH_DELETION,
        )
        | enabled_flag(
            capabilities.supports_observation_deletion,
            VCKSS_SUPPORT_OBSERVATION_DELETION,
        )
        | enabled_flag(capabilities.supports_controls, VCKSS_SUPPORT_CONTROLS)
        | enabled_flag(capabilities.supports_diagonal, VCKSS_SUPPORT_DIAGONAL)
        | enabled_flag(capabilities.supports_cmg, VCKSS_SUPPORT_CMG)
}

const fn enabled_flag(enabled: bool, flag: u64) -> u64 {
    if enabled {
        flag
    } else {
        0
    }
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
    Ok(InputColumns {
        worker: copy_positive_integer_column(
            columns.worker,
            rows,
            "worker identifier",
            ErrorCode::InvalidIdentifier,
            interrupt,
        )?,
        firm: copy_positive_integer_column(
            columns.firm,
            rows,
            "firm identifier",
            ErrorCode::InvalidIdentifier,
            interrupt,
        )?,
        deletion: copy_positive_integer_column(
            columns.deletion,
            rows,
            "deletion identifier",
            ErrorCode::InvalidIdentifier,
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

fn copy_positive_integer_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
    error_code: ErrorCode,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<u64>> {
    let source = copy_f64_slice(pointer, rows, label, interrupt)?;
    let mut output = Vec::with_capacity(rows);
    for (row, value) in source.into_iter().enumerate() {
        checkpoint_chunk(interrupt, row, "engine_ingest_integer")?;
        if !value.is_finite()
            || value <= 0.0
            || value.fract() != 0.0
            || value > MAX_EXACT_BINARY64_INTEGER as f64
        {
            return Err(BackendError::new(
                error_code,
                "engine_ingest",
                format!(
                    "{label} must be a positive exact binary64 integer at zero-based row {row}"
                ),
            ));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        output.push(value as u64);
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
