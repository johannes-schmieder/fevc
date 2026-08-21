// SPDX-License-Identifier: GPL-3.0-only

//! Versioned, panic-contained production engine ABI.
//!
//! The caller owns all pointer storage. Rust copies every input descriptor and
//! column before returning from preparation, retains no caller pointer, and
//! validates every output buffer capacity before writing. One generation-safe
//! registry owns the problem, preparation metadata, retained-row mask, result,
//! and numerical receipt for the complete command lifecycle.

use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::mem::{align_of, size_of};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};

use vckss_core::cmg::CmgOptions;
use vckss_core::engine::{run_jla_no_controls, JlaEngineOptions, JlaEngineResult, NumericalMcse};
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::jla::VarianceComponents;
use vckss_core::krylov::PcgOptions;
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute};
use vckss_core::types::{DeletionMode, InputColumns, RngContract, MAX_EXACT_BINARY64_INTEGER};
use vckss_core::ABI_VERSION;

use crate::context::{ContextHandle, ContextPayloadRef, ContextRegistry, ContextStateTag};
use crate::session::PreparationReceipt;
use crate::session_retained::PreparedProblemWithMask;

pub const VCKSS_DELETION_MATCH: u32 = 1;
pub const VCKSS_RNG_COUNTER_V1: u32 = 1;
pub const VCKSS_ROUTE_AUTO: u32 = 0;
pub const VCKSS_ROUTE_EXACT: u32 = 1;
pub const VCKSS_ROUTE_DIAGONAL_PCG: u32 = 2;
pub const VCKSS_ROUTE_CMG_PCG: u32 = 3;

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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEngineSnapshotV1 {
    pub struct_size: u32,
    pub state: u32,
    pub generation: u64,
    pub last_released_generation: u64,
}

#[derive(Debug)]
struct EngineSolved {
    result: JlaEngineResult,
    preparation: PreparationReceipt,
    retained: Vec<bool>,
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
        let rows = to_usize(request.rows, "engine_prepare", "row count")?;
        let input = copy_columns(&columns, rows)?;
        let prepared = PreparedProblemWithMask::from_columns(input)?;

        let mut state = lock_engine("engine_prepare")?;
        if request.cleanup_abandoned == 1 {
            state.registry.clear_abandoned();
        }
        let handle = state.registry.prepare(prepared)?;
        // SAFETY: capacity for one u64 was validated; unaligned C storage is
        // permitted and no Rust reference is formed.
        unsafe { output_handle.write_unaligned(handle.generation()) };
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
        let options = options_from_request(request)?;
        let handle = ContextHandle::from_generation(generation)?;
        let mut state = lock_engine("engine_solve")?;
        state.registry.solve_preserving(handle, |prepared| {
            let result = run_jla_no_controls(&prepared.problem, options)?;
            Ok(EngineSolved {
                result,
                preparation: prepared.receipt,
                retained: prepared.retained.clone(),
            })
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
        solver,
    })
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
    Ok(InputColumns {
        worker: copy_positive_integer_column(columns.worker, rows, "worker identifier")?,
        firm: copy_positive_integer_column(columns.firm, rows, "firm identifier")?,
        deletion: copy_positive_integer_column(columns.deletion, rows, "deletion identifier")?,
        outcome: copy_finite_column(columns.outcome, rows, "outcome")?,
        frequency: copy_positive_integer_column(columns.frequency, rows, "frequency weight")?,
        target_weight: copy_nonnegative_column(columns.target_weight, rows, "target weight")?,
        controls: Vec::new(),
    })
}

fn copy_positive_integer_column(pointer: *const f64, rows: usize, label: &str) -> Result<Vec<u64>> {
    let source = copy_f64_slice(pointer, rows, label)?;
    source
        .into_iter()
        .enumerate()
        .map(|(row, value)| {
            if !value.is_finite()
                || value <= 0.0
                || value.fract() != 0.0
                || value > MAX_EXACT_BINARY64_INTEGER as f64
            {
                return Err(BackendError::new(
                    ErrorCode::InvalidIdentifier,
                    "engine_ingest",
                    format!(
                        "{label} must be a positive exact binary64 integer at zero-based row {row}"
                    ),
                ));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(value as u64)
        })
        .collect()
}

fn copy_finite_column(pointer: *const f64, rows: usize, label: &str) -> Result<Vec<f64>> {
    let source = copy_f64_slice(pointer, rows, label)?;
    if let Some((row, _)) = source
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(BackendError::invalid(
            "engine_ingest",
            format!("{label} is nonfinite at zero-based row {row}"),
        ));
    }
    Ok(source)
}

fn copy_nonnegative_column(pointer: *const f64, rows: usize, label: &str) -> Result<Vec<f64>> {
    let source = copy_f64_slice(pointer, rows, label)?;
    if let Some((row, _)) = source
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite() || **value < 0.0)
    {
        return Err(BackendError::new(
            ErrorCode::InvalidTargetWeight,
            "engine_ingest",
            format!("{label} is negative or nonfinite at zero-based row {row}"),
        ));
    }
    Ok(source)
}

fn copy_f64_slice(pointer: *const f64, rows: usize, label: &str) -> Result<Vec<f64>> {
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
    // SAFETY: the C contract requires `rows` readable aligned doubles. The
    // bounds and alignment requirements are checked above, and the slice is
    // copied immediately so no caller pointer escapes this function.
    Ok(unsafe { std::slice::from_raw_parts(pointer, rows) }.to_vec())
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
