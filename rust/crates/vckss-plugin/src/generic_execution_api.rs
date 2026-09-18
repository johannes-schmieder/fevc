// SPDX-License-Identifier: GPL-3.0-only

//! Additive execution-only interface. Statistical capabilities, augmentation
//! widths and V1--V5 solve meanings are deliberately unchanged.

use super::*;

pub const VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE: u32 = 1;
pub const VCKSS_GENERIC_EXECUTION_DIRECT_ATTACHMENTS: u32 = 2;
pub const VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO: u32 = 3;

/// V4 is an exact prefix. Unlike V5, V6 explicitly selects an executor for
/// generic JLA and always consumes a positive permitted thread count. Numeric
/// point/attachment batch choices remain explicit; no literal width means auto.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV6 {
    pub v4: VckssEngineSolveRequestV4,
    pub execution_mode: u32,
    pub threads: u32,
    pub reserved_6: u64,
}

impl Default for VckssEngineSolveRequestV6 {
    fn default() -> Self {
        let mut v4 = VckssEngineSolveRequestV4::default();
        v4.v3.v2.v1.struct_size = u32::try_from(size_of::<Self>()).expect("V6 solve size");
        v4.v3.v2.algorithm = VCKSS_ALGORITHM_JLA;
        v4.v3.engine = VCKSS_ENGINE_GENERIC;
        v4.v3.v2.v1.solver_route = VCKSS_ROUTE_DIAGONAL_PCG;
        v4.v3.v2.v1.allow_automatic_cmg_setup_fallback = 0;
        Self {
            v4,
            execution_mode: VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE,
            threads: 1,
            reserved_6: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV6 {
    pub options: VckssEngineSolveRequestV6,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEngineSolveRequestInterruptV6 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV6::default();
        options.v4.v3.v2.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V6 interrupt solve size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

/// Actual logical work includes strict rank projections, point, projection,
/// component and residual-Gram RHSs. Queued RHSs exclude scalar diagonal fits;
/// CMG RHSs include fits and additional complete-model refinement solves.
/// Active workers are measured only for the diagonal queue (zero for CMG).
/// CMG reports selected concurrency separately, not measured busy workers.
/// Memory is the admitted allocation forecast, not physical process RSS.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VckssGenericExecutionReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub execution_mode: u32,
    pub permitted_threads: u32,
    pub planned_workers: u32,
    pub maximum_active_workers: u32,
    pub cmg_selected_concurrency: u32,
    pub reserved: u32,
    pub maximum_rhs_capacity: u64,
    pub leverage_batch_width: u64,
    pub target_batch_width: u64,
    pub fit_rhs_count: u64,
    pub control_projection_rhs_count: u64,
    pub point_probe_rhs_count: u64,
    pub projection_rhs_count: u64,
    pub component_rhs_count: u64,
    pub gram_rhs_count: u64,
    pub logical_rhs_count: u64,
    pub queued_rhs_count: u64,
    pub cmg_rhs_count: u64,
    pub control_refinement_rhs_count: u64,
    pub command_peak_forecast_bytes: u64,
    pub maximum_complete_residual: f64,
}

const _: [(); 304] = [(); size_of::<VckssEngineSolveRequestV6>()];
const _: [(); 328] = [(); size_of::<VckssEngineSolveRequestInterruptV6>()];
const _: [(); 160] = [(); size_of::<VckssGenericExecutionReceiptV1>()];
const _: [(); 288] = [(); std::mem::offset_of!(VckssEngineSolveRequestV6, execution_mode)];
const _: [(); 304] = [(); std::mem::offset_of!(VckssEngineSolveRequestInterruptV6, interrupt_poll)];
const _: [(); 64] = [(); std::mem::offset_of!(VckssGenericExecutionReceiptV1, fit_rhs_count)];

/// V7 extends the frozen V6 prefix solely to distinguish automatic
/// component-inference batching from a literal numeric attachment width.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV7 {
    pub v6: VckssEngineSolveRequestV6,
    pub component_batch_mode: u32,
    pub reserved_7: u32,
}

impl Default for VckssEngineSolveRequestV7 {
    fn default() -> Self {
        let mut v6 = VckssEngineSolveRequestV6::default();
        v6.v4.v3.v2.v1.struct_size = u32::try_from(size_of::<Self>()).expect("V7 solve size");
        Self {
            v6,
            component_batch_mode: 1,
            reserved_7: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV7 {
    pub options: VckssEngineSolveRequestV7,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEngineSolveRequestInterruptV7 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV7::default();
        options.v6.v4.v3.v2.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V7 interrupt solve size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

const _: [(); 312] = [(); size_of::<VckssEngineSolveRequestV7>()];
const _: [(); 336] = [(); size_of::<VckssEngineSolveRequestInterruptV7>()];
const _: [(); 304] = [(); std::mem::offset_of!(VckssEngineSolveRequestV7, component_batch_mode)];
const _: [(); 312] = [(); std::mem::offset_of!(VckssEngineSolveRequestInterruptV7, interrupt_poll)];

/// V8 preserves the V7 prefix and the caller's original V3 request. The
/// resolver may select exact or compressed, in which case no generic executor
/// or generic-work receipt is created.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV8 {
    pub v7: VckssEngineSolveRequestV7,
    pub resolved_execution_mode: u32,
    pub tolerance_supplied: u32,
}

impl Default for VckssEngineSolveRequestV8 {
    fn default() -> Self {
        let mut v7 = VckssEngineSolveRequestV7::default();
        v7.v6.v4.v3.v2.v1.struct_size = u32::try_from(size_of::<Self>()).expect("V8 solve size");
        v7.v6.execution_mode = VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO;
        v7.component_batch_mode = 0;
        Self {
            v7,
            resolved_execution_mode: VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO,
            tolerance_supplied: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestInterruptV8 {
    pub options: VckssEngineSolveRequestV8,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssEngineSolveRequestInterruptV8 {
    fn default() -> Self {
        let mut options = VckssEngineSolveRequestV8::default();
        options.v7.v6.v4.v3.v2.v1.struct_size =
            u32::try_from(size_of::<Self>()).expect("V8 interrupt solve size");
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

const _: [(); 320] = [(); size_of::<VckssEngineSolveRequestV8>()];
const _: [(); 344] = [(); size_of::<VckssEngineSolveRequestInterruptV8>()];
const _: [(); 312] = [(); std::mem::offset_of!(VckssEngineSolveRequestV8, resolved_execution_mode)];
const _: [(); 320] = [(); std::mem::offset_of!(VckssEngineSolveRequestInterruptV8, interrupt_poll)];

/// Separate diagnostic receipt; the frozen 160-byte generic-work receipt is
/// deliberately not extended or reinterpreted.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VckssComponentBatchReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub policy: u32,
    pub selection_reason: u32,
    pub generation: u64,
    pub declared_component_width: u64,
    pub declared_gram_width: u64,
    pub component_width: u64,
    pub gram_width: u64,
    pub automatic_cap: u64,
    pub permitted_threads: u64,
    pub maximum_rhs_capacity: u64,
    pub command_peak_forecast_bytes: u64,
}

const _: [(); 88] = [(); size_of::<VckssComponentBatchReceiptV1>()];

#[derive(Clone, Copy)]
pub(super) enum GenericExecutionPlan {
    /// Only the separate exact-execution entrypoint can construct this intent.
    ExactParallel(usize),
    DiagonalQueue(usize),
    DirectAttachments,
    AutomaticComponentDiagonalQueue(usize),
    AutomaticComponentDirectAttachments(usize),
    ResolvedExecution {
        threads: usize,
        full_cmg: FullCmgPlanOptions,
    },
}

fn validate(
    request: VckssEngineSolveRequestV6,
) -> Result<(GenericExecutionPlan, Option<FullCmgPlanOptions>)> {
    let v4 = request.v4;
    require_abi(v4.v3.v2.v1.abi_version)?;
    if v4.v3.v2.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV6>()?
        || request.reserved_6 != 0
    {
        return Err(abi_error(
            "invalid V6 solve structure size or reserved field",
        ));
    }
    if request.threads == 0 {
        return Err(BackendError::invalid(
            "generic_execution",
            "V6 requires positive permitted threads",
        ));
    }
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(unsupported(
            "V6 execution is enabled only on macOS and Linux",
        ));
    }
    if v4.v3.v2.algorithm != VCKSS_ALGORITHM_JLA
        || v4.v3.engine != VCKSS_ENGINE_GENERIC
        || v4.v3.v2.v1.rng_contract != VCKSS_RNG_COUNTER_V1
        || v4.v3.v2.v1.allow_automatic_cmg_setup_fallback != 0
    {
        return Err(unsupported(
            "V6 requires explicit generic JLA, Counter-V1 and no fallback",
        ));
    }
    let threads = usize::try_from(request.threads)
        .map_err(|_| resource_error("generic_execution", "thread count is not representable"))?;
    match request.execution_mode {
        VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE
            if v4.v3.v2.v1.solver_route == VCKSS_ROUTE_DIAGONAL_PCG =>
        {
            Ok((GenericExecutionPlan::DiagonalQueue(threads), None))
        }
        VCKSS_GENERIC_EXECUTION_DIRECT_ATTACHMENTS
            if v4.v3.v2.v1.solver_route == VCKSS_ROUTE_CMG_PCG
                && v4.v3.batch_mode == VCKSS_BATCH_MODE_AUTO
                && v4.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO
                && v4.target_batch_mode == VCKSS_BATCH_MODE_AUTO
                && v4.v3.v2.v1.leverage_batch_width == 0
                && v4.v3.v2.v1.target_batch_width == 0 =>
        {
            Ok((
                GenericExecutionPlan::DirectAttachments,
                Some(FullCmgPlanOptions::production(
                    threads,
                    v4.v3.v2.v1.pcg_tolerance,
                    Some(v4.v3.v2.v1.pcg_tolerance),
                )),
            ))
        }
        _ => Err(unsupported(
            "unknown V6 executor or incompatible solver/batch selection",
        )),
    }
}

fn unsupported(message: &str) -> BackendError {
    BackendError::new(ErrorCode::UnsupportedFeature, "generic_execution", message)
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v6(
    output: *mut VckssEngineSolveRequestV6,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV6>(
            output.cast(),
            output_capacity_bytes,
            "V6 solve default",
        )?;
        write_output(output, VckssEngineSolveRequestV6::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v6(
    output: *mut VckssEngineSolveRequestInterruptV6,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV6>(
            output.cast(),
            output_capacity_bytes,
            "V6 interrupt solve default",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV6::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v6(
    generation: u64,
    request: *const VckssEngineSolveRequestV6,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V6 solve request")?;
        let (plan, full_cmg) = validate(request)?;
        solve_engine_v4_with_generic_execution(
            generation,
            request.v4,
            full_cmg,
            Some(plan),
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v6(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV6,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V6 interrupt solve request")?;
        let (plan, full_cmg) = validate(request.options)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            // Both owned executors observe the coordinator's cancellation
            // token. Only this caller invokes the foreign polling callback.
            Some(mut interrupt) => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v4,
                full_cmg,
                Some(plan),
                V4SolveExecution::Coordinated(&mut interrupt),
            ),
            None => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v4,
                full_cmg,
                Some(plan),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

fn validate_v7(
    request: VckssEngineSolveRequestV7,
) -> Result<(GenericExecutionPlan, Option<FullCmgPlanOptions>)> {
    if request.v6.v4.v3.v2.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV7>()?
        || request.component_batch_mode != 1
        || request.reserved_7 != 0
    {
        return Err(abi_error(
            "invalid V7 solve structure or component batch mode",
        ));
    }
    let (plan, full_cmg) = validate(request.v6)?;
    let plan = match plan {
        GenericExecutionPlan::DiagonalQueue(threads) => {
            GenericExecutionPlan::AutomaticComponentDiagonalQueue(threads)
        }
        GenericExecutionPlan::DirectAttachments => {
            GenericExecutionPlan::AutomaticComponentDirectAttachments(
                usize::try_from(request.v6.threads).map_err(|_| {
                    resource_error("generic_execution", "thread count is not representable")
                })?,
            )
        }
        _ => return Err(unsupported("invalid V7 executor")),
    };
    Ok((plan, full_cmg))
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v7(
    output: *mut VckssEngineSolveRequestV7,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV7>(
            output.cast(),
            output_capacity_bytes,
            "V7 solve default",
        )?;
        write_output(output, VckssEngineSolveRequestV7::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v7(
    output: *mut VckssEngineSolveRequestInterruptV7,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV7>(
            output.cast(),
            output_capacity_bytes,
            "V7 interrupt solve default",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV7::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v7(
    generation: u64,
    request: *const VckssEngineSolveRequestV7,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V7 solve request")?;
        let (plan, full_cmg) = validate_v7(request)?;
        solve_engine_v4_with_generic_execution(
            generation,
            request.v6.v4,
            full_cmg,
            Some(plan),
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v7(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV7,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V7 interrupt solve request")?;
        let (plan, full_cmg) = validate_v7(request.options)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v6.v4,
                full_cmg,
                Some(plan),
                V4SolveExecution::Coordinated(&mut interrupt),
            ),
            None => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v6.v4,
                full_cmg,
                Some(plan),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

fn validate_v8(request: VckssEngineSolveRequestV8) -> Result<GenericExecutionPlan> {
    let v7 = request.v7;
    let v6 = v7.v6;
    let v4 = v6.v4;
    require_abi(v4.v3.v2.v1.abi_version)?;
    if v4.v3.v2.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestV8>()?
        || request.tolerance_supplied > 1
        || v7.reserved_7 != 0
        || v7.component_batch_mode != 0
        || v6.reserved_6 != 0
        || request.resolved_execution_mode != VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO
    {
        return Err(abi_error("invalid V8 resolved execution request"));
    }
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(unsupported(
            "V8 execution is enabled only on macOS and Linux",
        ));
    }
    if v6.threads == 0
        || v6.execution_mode != VCKSS_GENERIC_EXECUTION_RESOLVED_AUTO
        || !matches!(
            v4.v3.v2.algorithm,
            VCKSS_ALGORITHM_JLA | VCKSS_ALGORITHM_AUTO
        )
        || !matches!(
            v4.v3.engine,
            VCKSS_ENGINE_GENERIC | VCKSS_ENGINE_AUTO_OR_UNSPECIFIED
        )
        || !matches!(
            v4.v3.v2.v1.solver_route,
            VCKSS_ROUTE_DIAGONAL_PCG | VCKSS_ROUTE_AUTO
        )
        || v4.v3.v2.v1.rng_contract != VCKSS_RNG_COUNTER_V1
        || v4.v3.v2.v1.allow_automatic_cmg_setup_fallback
            != u32::from(v4.v3.v2.v1.solver_route == VCKSS_ROUTE_AUTO)
    {
        return Err(unsupported(
            "V8 requires original auto/JLA, auto/generic engine and a diagonal or automatic solver tuple",
        ));
    }
    let threads = usize::try_from(v6.threads)
        .map_err(|_| resource_error("generic_execution", "thread count is not representable"))?;
    Ok(GenericExecutionPlan::ResolvedExecution {
        threads,
        full_cmg: FullCmgPlanOptions::production(
            threads,
            v4.v3.v2.v1.pcg_tolerance,
            (request.tolerance_supplied == 1).then_some(v4.v3.v2.v1.pcg_tolerance),
        ),
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_v8(
    output: *mut VckssEngineSolveRequestV8,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestV8>(
            output.cast(),
            output_capacity_bytes,
            "V8 solve default",
        )?;
        write_output(output, VckssEngineSolveRequestV8::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_default_solve_request_interrupt_v8(
    output: *mut VckssEngineSolveRequestInterruptV8,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssEngineSolveRequestInterruptV8>(
            output.cast(),
            output_capacity_bytes,
            "V8 interrupt solve default",
        )?;
        write_output(output, VckssEngineSolveRequestInterruptV8::default());
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v8(
    generation: u64,
    request: *const VckssEngineSolveRequestV8,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V8 solve request")?;
        let plan = validate_v8(request)?;
        solve_engine_v4_with_generic_execution(
            generation,
            request.v7.v6.v4,
            None,
            Some(plan),
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_interrupt_v8(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV8,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "V8 interrupt solve request")?;
        let plan = validate_v8(request.options)?;
        let interrupt = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "solve",
        )?;
        match interrupt {
            Some(mut interrupt) => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v7.v6.v4,
                None,
                Some(plan),
                V4SolveExecution::Coordinated(&mut interrupt),
            ),
            None => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v7.v6.v4,
                None,
                Some(plan),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_generic_execution_receipt_v1(
    generation: u64,
    output: *mut VckssGenericExecutionReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssGenericExecutionReceiptV1>(
            output.cast(),
            output_capacity_bytes,
            "generic execution receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("generic_execution")?;
        let solved = state.registry.result(handle)?;
        write_output(output, receipt(generation, solved)?);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_component_batch_receipt_v1(
    generation: u64,
    output: *mut VckssComponentBatchReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssComponentBatchReceiptV1>(
            output.cast(),
            output_capacity_bytes,
            "component batch receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("component_batch_receipt")?;
        let solved = state.registry.result(handle)?;
        let EngineEstimate::GenericJla(result) = &solved.result else {
            return Err(unsupported("component batch receipt requires generic JLA"));
        };
        let batch = result
            .receipt
            .execution
            .component_batch
            .ok_or_else(|| unsupported("result has no component batch receipt"))?;
        let reason = match batch.selection_reason {
            BatchSelectionReason::ExplicitWidth => 1,
            BatchSelectionReason::NoBudgetPerformanceChoice => 2,
            BatchSelectionReason::LargestAdmissibleCandidate => 3,
            BatchSelectionReason::MinimumMemoryOverBudget => 4,
        };
        write_output(
            output,
            VckssComponentBatchReceiptV1 {
                struct_size: struct_size_u32::<VckssComponentBatchReceiptV1>()?,
                schema_version: 1,
                policy: u32::from(
                    batch.policy == vckss_core::generic_jla::ComponentBatchPolicy::Automatic,
                ),
                selection_reason: reason,
                generation,
                declared_component_width: to_u64(
                    batch.declared_component_width,
                    "declared component width",
                )?,
                declared_gram_width: to_u64(batch.declared_gram_width, "declared Gram width")?,
                component_width: to_u64(batch.component_width, "component width")?,
                gram_width: to_u64(batch.gram_width, "Gram width")?,
                automatic_cap: to_u64(batch.automatic_cap, "automatic component cap")?,
                permitted_threads: to_u64(batch.permitted_threads, "component threads")?,
                maximum_rhs_capacity: receipt(generation, solved)?.maximum_rhs_capacity,
                command_peak_forecast_bytes: result.receipt.execution.memory.peak_bytes,
            },
        );
        Ok(())
    })
}

fn receipt(generation: u64, solved: &EngineSolved) -> Result<VckssGenericExecutionReceiptV1> {
    let EngineEstimate::GenericJla(result) = &solved.result else {
        return Err(unsupported(
            "generic execution receipt requires a V6 generic result",
        ));
    };
    let execution = &result.receipt.execution;
    let gram = result
        .component_inference
        .as_ref()
        .and_then(|c| c.residual_moments.as_ref());
    let mut value = VckssGenericExecutionReceiptV1 {
        struct_size: struct_size_u32::<VckssGenericExecutionReceiptV1>()?,
        schema_version: 1,
        generation,
        leverage_batch_width: to_u64(execution.batch.leverage_active_width, "leverage width")?,
        target_batch_width: to_u64(execution.batch.target_active_width, "target width")?,
        fit_rhs_count: 1 + u64::from(
            solved.controls_count > 0 && solved.nuisance == NuisanceMode::FixedOffset,
        ),
        control_projection_rhs_count: u64::from(solved.controls_count),
        point_probe_rhs_count: u64::from(solved.probes) * 3,
        projection_rhs_count: to_u64(
            result.projection.as_ref().map_or(0, |p| p.columns),
            "projection RHS count",
        )?,
        component_rhs_count: to_u64(
            result
                .component_inference
                .as_ref()
                .map_or(0, |c| c.solve_receipts.len()),
            "component RHS count",
        )?,
        gram_rhs_count: to_u64(
            result
                .component_inference
                .as_ref()
                .and_then(|c| c.residual_moments.as_ref())
                .map_or(0, |r| r.projections.len()),
            "Gram RHS count",
        )?,
        command_peak_forecast_bytes: execution.memory.peak_bytes,
        maximum_complete_residual: gram.map_or(result.receipt.maximum_complete_residual, |fit| {
            fit.projections
                .iter()
                .fold(result.receipt.maximum_complete_residual, |max, rhs| {
                    max.max(rhs.complete_residual)
                })
        }),
        ..VckssGenericExecutionReceiptV1::default()
    };
    value.logical_rhs_count = [
        value.fit_rhs_count,
        value.control_projection_rhs_count,
        value.point_probe_rhs_count,
        value.projection_rhs_count,
        value.component_rhs_count,
        value.gram_rhs_count,
    ]
    .into_iter()
    .try_fold(0_u64, |total, count| total.checked_add(count))
    .ok_or_else(|| resource_error("generic_execution", "logical RHS count overflow"))?;
    let (permitted, workers, active, capacity) = if let Some(queue) = &execution.diagonal_queue {
        value.execution_mode = VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE;
        value.queued_rhs_count = to_u64(queue.work.completed_rhs, "queued RHS count")?;
        if value.queued_rhs_count.checked_add(value.fit_rhs_count) != Some(value.logical_rhs_count)
        {
            return Err(BackendError::invariant(
                "generic_execution",
                "queue work does not reconcile",
            ));
        }
        (
            queue.plan.permitted_threads,
            queue.plan.workers,
            queue.work.maximum_active_workers,
            queue.plan.maximum_rhs,
        )
    } else if let (Some(attachment), Some(cmg)) =
        (&execution.direct_attachments, &execution.full_cmg)
    {
        value.execution_mode = VCKSS_GENERIC_EXECUTION_DIRECT_ATTACHMENTS;
        value.cmg_rhs_count = cmg.rhs_count;
        value.control_refinement_rhs_count = attachment.control_refinement_rhs;
        if value.logical_rhs_count != to_u64(attachment.logical_rhs, "direct logical RHS count")?
            || value
                .logical_rhs_count
                .checked_add(value.control_refinement_rhs_count)
                != Some(value.cmg_rhs_count)
        {
            return Err(BackendError::invariant(
                "generic_execution",
                "CMG work does not reconcile",
            ));
        }
        value.cmg_selected_concurrency =
            to_u32(cmg.maximum_concurrency, "CMG selected concurrency")?;
        (
            cmg.setup.threads,
            cmg.setup.threads,
            0,
            cmg.setup.maximum_batch_rhs,
        )
    } else {
        return Err(unsupported(
            "legacy solve has no V6 generic execution receipt",
        ));
    };
    if active > workers
        || workers > permitted
        || workers == 0
        || u64::from(value.cmg_selected_concurrency) > to_u64(permitted, "permitted threads")?
    {
        return Err(BackendError::invariant(
            "generic_execution",
            "worker counts do not reconcile",
        ));
    }
    value.permitted_threads = to_u32(permitted, "permitted threads")?;
    value.planned_workers = to_u32(workers, "planned workers")?;
    value.maximum_active_workers = to_u32(active, "observed workers")?;
    value.maximum_rhs_capacity = to_u64(capacity, "RHS capacity")?;
    Ok(value)
}
