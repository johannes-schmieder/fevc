// SPDX-License-Identifier: GPL-3.0-only

//! Additive, opt-in numerical attachment. Frozen solve and receipt layouts
//! retain their meanings; this transport does not advertise a Stata capability.
use super::*;
use vckss_core::engine::CompressedNumericalMcResult;
use vckss_core::generic_jla::NumericalMcResult;
use vckss_core::numerical_mc::{self, Covariance, Status};

#[derive(Debug)]
pub(super) enum Attachment {
    Exact,
    Generic(NumericalMcResult),
    Compressed(CompressedNumericalMcResult),
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssNumericalRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub schema_version: u32,
    pub threads: u32,
    pub tolerance_supplied: u32,
    pub projection_requested: u32,
    pub component_requested: u32,
    pub reserved: u32,
    pub controls_count: u32,
    pub reserved_2: u32,
    pub point: VckssEngineSolveRequestV4,
}

impl Default for VckssNumericalRequestV1 {
    fn default() -> Self {
        let mut point = VckssEngineSolveRequestV4::default();
        point.v3.v2.algorithm = VCKSS_ALGORITHM_JLA;
        Self {
            abi_version: ABI_VERSION,
            struct_size: size_of::<Self>() as u32,
            schema_version: 1,
            threads: 1,
            tolerance_supplied: 0,
            projection_requested: 0,
            component_requested: 0,
            reserved: 0,
            controls_count: 0,
            reserved_2: 0,
            point,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VckssNumericalResultV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    /// 1 exact_zero, 2 ok_local, 3 ok_local_psd_adjusted, 4 unstable_nonpsd,
    /// 5 nonsmooth_adjustment, 6 nonfinite_derivative, 7 replay_failed.
    pub status: u32,
    pub engine: u32,
    pub leverage_probes: u32,
    pub target_probes: u32,
    pub target_fold_a: u32,
    pub target_fold_b: u32,
    pub certified_replay_rhs: u64,
    pub executed_replay_rhs: u64,
    pub attempted_replay_rhs: u64,
    pub replay_generator_words: u64,
    pub allocation_bound_bytes: u64,
    /// u32::MAX means no failed direction; otherwise zero-based.
    pub failed_replay_probe: u32,
    pub replay_error_code: u32,
    pub psd_adjustment: f64,
    pub minimum_constrained: f64,
    pub minimum_residual_margin: f64,
    pub maximum_sensitivity_ratio: f64,
    pub score_means: [[f64; 3]; 2],
    /// Matrices use row-major primitive worker, firm, covariance order.
    pub conditional: [[f64; 3]; 3],
    pub leverage: [[f64; 3]; 3],
    pub raw: [[f64; 3]; 3],
    pub usable: [[f64; 3]; 3],
    pub mcse: [f64; 4],
    pub point_rhs: u64,
    pub maximum_point_complete_residual: f64,
    pub maximum_replay_complete_residual: f64,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VckssNumericalRhsV1 {
    pub schema_version: u32,
    pub phase: u32,
    pub probe: u32,
    pub reserved: u32,
    pub complete_residual: f64,
}

const _: [(); 328] = [(); size_of::<VckssNumericalRequestV1>()];
const _: [(); 512] = [(); size_of::<VckssNumericalResultV1>()];
const _: [(); 24] = [(); size_of::<VckssNumericalRhsV1>()];

fn unsupported(message: &str) -> BackendError {
    BackendError::new(
        ErrorCode::UnsupportedFeature,
        "numerical_preflight",
        message,
    )
}

fn validate(request: VckssNumericalRequestV1) -> Result<GenericExecutionPlan> {
    validate_point(request, true)
}

fn validate_point(
    request: VckssNumericalRequestV1,
    point_only: bool,
) -> Result<GenericExecutionPlan> {
    require_abi(request.abi_version)?;
    require_abi(request.point.v3.v2.v1.abi_version)?;
    if request.schema_version != 1
        || request.reserved != 0
        || request.reserved_2 != 0
        || request.tolerance_supplied > 1
        || request.projection_requested > 1
        || request.component_requested > 1
    {
        return Err(abi_error("invalid numerical V1 schema or reserved fields"));
    }
    if point_only && vckss_rust_numerical_schema_v1() != 1 {
        return Err(unsupported(
            "numerical V1 is enabled only on macOS and Linux",
        ));
    }
    let point = request.point;
    require_struct_size::<VckssEngineSolveRequestV4>(
        point.v3.v2.v1.struct_size,
        "numerical point prefix",
    )?;
    if point.v3.reserved_3 != 0 || point.reserved_4 != 0 {
        return Err(abi_error(
            "reserved numerical point prefix fields must be zero",
        ));
    }
    if request.threads == 0 || point.v3.v2.v1.probes < 2 {
        return Err(BackendError::invalid(
            "numerical_preflight",
            "positive threads and at least two probes required",
        ));
    }
    if (point_only && (request.projection_requested != 0 || request.component_requested != 0))
        || !matches!(
            point.v3.v2.algorithm,
            VCKSS_ALGORITHM_JLA | VCKSS_ALGORITHM_AUTO
        )
        || !matches!(
            point.v3.engine,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED | VCKSS_ENGINE_GENERIC | VCKSS_ENGINE_COMPRESSED
        )
        || !matches!(
            point.v3.v2.v1.solver_route,
            VCKSS_ROUTE_AUTO | VCKSS_ROUTE_DIAGONAL_PCG | VCKSS_ROUTE_CMG_PCG
        )
        || point.v3.v2.v1.rng_contract != VCKSS_RNG_COUNTER_V1
    {
        return Err(unsupported("numerical V1 requires Counter-V1 JLA with no projection or component-inference attachment"));
    }
    let threads = usize::try_from(request.threads)
        .map_err(|_| resource_error("numerical_preflight", "thread count overflow"))?;
    validate_exact_request_options(point.v3.v2)?;
    deletion_from_code(point.v3.v2.v1.deletion_mode)?;
    nuisance_from_code(point.v3.v2.nuisance_mode)?;
    model_routing_from_request(point.v3.v2.v1)?.validate()?;
    batch_request_from_code(
        point.leverage_batch_mode,
        point.v3.v2.v1.leverage_batch_width,
        "leverage",
    )?;
    batch_request_from_code(
        point.target_batch_mode,
        point.v3.v2.v1.target_batch_width,
        "target",
    )?;
    let capability = capability_request_v3_for_solve(point, request.controls_count)?;
    let (reason, profile) = request_capability_classification_v3(capability);
    if point.v3.capability_schema != VCKSS_REQUEST_CAPABILITY_SCHEMA_V3
        || point.v3.capability_profile != VCKSS_REQUEST_PROFILE_PLANNED_V1
        || reason != VCKSS_REQUEST_REASON_SUPPORTED
        || profile != point.v3.capability_profile
        || request_capability_signature_v3(capability) != point.v3.request_signature
    {
        return Err(unsupported(
            "numerical point request does not reconcile with its planned capability",
        ));
    }
    Ok(GenericExecutionPlan::NumericalResolved {
        threads,
        full_cmg: FullCmgPlanOptions::production(
            threads,
            point.v3.v2.v1.pcg_tolerance,
            (request.tolerance_supplied == 1).then_some(point.v3.v2.v1.pcg_tolerance),
        ),
    })
}

/// V1 prefix is frozen; V2 adds executor intent without selecting a new point route.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssNumericalRequestV2 {
    pub v1: VckssNumericalRequestV1,
    pub execution_mode: u32,
    pub component_batch_mode: u32,
    pub full_cmg: u32,
    pub reserved: u32,
}

impl Default for VckssNumericalRequestV2 {
    fn default() -> Self {
        let v1 = VckssNumericalRequestV1 {
            struct_size: size_of::<Self>() as u32,
            schema_version: 2,
            ..VckssNumericalRequestV1::default()
        };
        Self {
            v1,
            execution_mode: 0,
            component_batch_mode: 0,
            full_cmg: 0,
            reserved: 0,
        }
    }
}
const _: [(); 344] = [(); size_of::<VckssNumericalRequestV2>()];

fn validate_v2(
    request: VckssNumericalRequestV2,
) -> Result<(Option<GenericExecutionPlan>, Option<FullCmgPlanOptions>)> {
    if request.v1.schema_version != 2
        || request.reserved != 0
        || request.full_cmg > 1
        || request.component_batch_mode > 1
        || vckss_rust_numerical_schema_v2() != 2
    {
        return Err(abi_error("invalid numerical V2 schema or executor intent"));
    }
    let mut prefix = request.v1;
    prefix.schema_version = 1;
    validate_point(prefix, false)?;
    generic_execution_api::numerical_execution_plan(
        prefix.point,
        prefix.threads,
        request.execution_mode,
        request.component_batch_mode,
        prefix.tolerance_supplied,
        request.full_cmg,
    )
}

#[no_mangle]
pub extern "C" fn vckss_rust_numerical_schema_v2() -> u32 {
    u32::from(cfg!(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    ))) * 2
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_numerical_preflight_v2(
    request: *const VckssNumericalRequestV2,
) -> i32 {
    ffi_status(|| {
        validate_v2(copy_request_struct(request, "numerical V2 preflight")?)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_numerical_interrupt_v2(
    generation: u64,
    request: *const VckssNumericalRequestV2,
    callback: VckssInterruptPollV1,
    context: *mut c_void,
    interval: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "numerical V2 solve")?;
        let (plan, cmg) = validate_v2(request)?;
        {
            let state = lock_engine("numerical_preflight")?;
            let handle = ContextHandle::from_generation(generation)?;
            if let ContextPayloadRef::Prepared(prepared) = state.registry.payload(handle)? {
                if request.v1.controls_count as usize != prepared.problem.controls.len()
                    || (request.v1.projection_requested != 0) != prepared.projection.is_some()
                    || (request.v1.component_requested != 0)
                        != prepared.component_inference.is_some()
                {
                    return Err(BackendError::invalid(
                        "numerical_preflight",
                        "numerical attachment intent differs from the prepared context",
                    ));
                }
            }
        }
        match CallbackInterrupt::new(callback, context, interval, 0, "numerical_mc")? {
            Some(mut interrupt) => solve_engine_v4_with_numerical_attachment(
                generation,
                request.v1.point,
                cmg,
                plan,
                true,
                V4SolveExecution::Coordinated(&mut interrupt),
            ),
            None => solve_engine_v4_with_numerical_attachment(
                generation,
                request.v1.point,
                cmg,
                plan,
                true,
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

fn cmg_plan(
    request: VckssNumericalRequestV1,
    plan: GenericExecutionPlan,
) -> Option<FullCmgPlanOptions> {
    match plan {
        GenericExecutionPlan::NumericalResolved { full_cmg, .. }
            if request.point.v3.v2.v1.solver_route != VCKSS_ROUTE_DIAGONAL_PCG
                && request.point.leverage_batch_mode == VCKSS_BATCH_MODE_AUTO
                && request.point.target_batch_mode == VCKSS_BATCH_MODE_AUTO =>
        {
            Some(full_cmg)
        }
        _ => None,
    }
}

/// Presence and version are checked before native preparation or estimator RNG.
#[no_mangle]
pub extern "C" fn vckss_rust_numerical_schema_v1() -> u32 {
    u32::from(cfg!(any(target_os = "macos", target_os = "linux")))
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_numerical_preflight_v1(
    request: *const VckssNumericalRequestV1,
) -> i32 {
    ffi_status(|| {
        validate(copy_request_struct(request, "numerical preflight")?)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_numerical_v1(
    generation: u64,
    request: *const VckssNumericalRequestV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "numerical solve")?;
        let plan = validate(request)?;
        solve_engine_v4_with_generic_execution(
            generation,
            request.point,
            cmg_plan(request, plan),
            Some(plan),
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_numerical_interrupt_v1(
    generation: u64,
    request: *const VckssNumericalRequestV1,
    callback: VckssInterruptPollV1,
    context: *mut c_void,
    interval: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "numerical interrupt solve")?;
        let plan = validate(request)?;
        let callback = CallbackInterrupt::new(callback, context, interval, 0, "numerical_solve")?;
        match callback {
            Some(mut callback) => solve_engine_v4_with_generic_execution(
                generation,
                request.point,
                cmg_plan(request, plan),
                Some(plan),
                V4SolveExecution::Coordinated(&mut callback),
            ),
            None => solve_engine_v4_with_generic_execution(
                generation,
                request.point,
                cmg_plan(request, plan),
                Some(plan),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

fn covariance(value: &mut VckssNumericalResultV1, covariance: &Covariance) {
    value.status = match covariance.status {
        Status::ExactZero => 1,
        Status::OkLocal => 2,
        Status::OkLocalPsdAdjusted => 3,
        Status::UnstableNonPsd => 4,
        Status::NonsmoothAdjustment => 5,
        Status::NonfiniteDerivative => 6,
        Status::ReplayFailed => 7,
    };
    value.conditional = covariance.conditional;
    value.leverage = covariance.leverage;
    value.raw = covariance.raw;
    value.usable = covariance.usable.unwrap_or([[f64::NAN; 3]; 3]);
    value.mcse = covariance.mcse.unwrap_or([f64::NAN; 4]);
    value.psd_adjustment = covariance.psd_adjustment;
}

fn result(generation: u64, solved: &EngineSolved) -> Result<VckssNumericalResultV1> {
    let mut value = VckssNumericalResultV1 {
        struct_size: size_of::<VckssNumericalResultV1>() as u32,
        schema_version: 1,
        generation,
        engine: solved.engine_selected,
        failed_replay_probe: u32::MAX,
        ..VckssNumericalResultV1::default()
    };
    match solved
        .numerical
        .as_ref()
        .ok_or_else(|| unsupported("result has no numerical V1 attachment"))?
    {
        Attachment::Exact => {
            let mut zero = numerical_mc::finalize([[0.0; 3]; 3], [[0.0; 3]; 3]);
            zero.status = Status::ExactZero;
            covariance(&mut value, &zero);
        }
        Attachment::Generic(a) => {
            covariance(&mut value, &a.covariance);
            value.leverage_probes = a.leverage_probes;
            value.target_probes = a.target_probes;
            [value.target_fold_a, value.target_fold_b] = a.folds;
            value.certified_replay_rhs = to_u64(a.replay_rhs.len(), "certified replay RHS")?;
            value.attempted_replay_rhs =
                to_u64(a.replay_attempted_rhs_count, "attempted replay RHS")?;
            value.executed_replay_rhs = to_u64(a.replay_executed_rhs_count, "executed replay RHS")?;
            value.replay_generator_words = a.replay_generator_word_evaluations;
            value.allocation_bound_bytes = a.allocation_bound_bytes;
            value.failed_replay_probe = a.failed_replay_probe.unwrap_or(u32::MAX);
            value.replay_error_code = a.replay_error.as_ref().map_or(0, |e| e.code as u32);
            value.score_means = a.replay_score_means;
            value.minimum_constrained = a.minimum_constrained;
            value.minimum_residual_margin = a.minimum_residual_margin;
            value.maximum_sensitivity_ratio = a.maximum_sensitivity_ratio;
            value.maximum_replay_complete_residual = a
                .replay_rhs
                .iter()
                .fold(0.0_f64, |x, r| x.max(r.complete_residual));
        }
        Attachment::Compressed(a) => {
            covariance(&mut value, &a.covariance);
            value.leverage_probes = a.leverage_probes;
            value.target_probes = a.target_probes;
            [value.target_fold_a, value.target_fold_b] = a.folds;
            value.certified_replay_rhs = to_u64(a.replay_rhs.len(), "compressed replay RHS")?;
            value.attempted_replay_rhs =
                to_u64(a.replay_attempted_rhs_count, "attempted replay RHS")?;
            value.executed_replay_rhs = to_u64(
                a.replay_executed_rhs_count,
                "completed compressed replay RHS",
            )?;
            value.replay_generator_words = a.replay_generator_word_evaluations;
            value.allocation_bound_bytes = a.allocation_bound_bytes;
            value.failed_replay_probe = a.failed_replay_probe.unwrap_or(u32::MAX);
            value.replay_error_code = a.replay_error.as_ref().map_or(0, |e| e.code as u32);
            value.score_means = a.score_means;
            value.minimum_constrained = a.minimum_constrained;
            value.minimum_residual_margin = a.minimum_residual_margin;
            value.maximum_sensitivity_ratio = a.maximum_sensitivity_ratio;
            value.maximum_replay_complete_residual = a
                .replay_rhs
                .iter()
                .fold(0.0_f64, |x, r| x.max(r.solve.complete_residual));
        }
    }
    match &solved.result {
        EngineEstimate::GenericJla(r) => {
            value.point_rhs = to_u64(r.receipt.rhs.len(), "point RHS")?;
            value.maximum_point_complete_residual = r.receipt.maximum_complete_residual;
        }
        EngineEstimate::Jla(r) => {
            value.point_rhs = to_u64(
                1 + r.receipt.leverage_rhs.len() + r.receipt.target_rhs.len(),
                "point RHS",
            )?;
            value.maximum_point_complete_residual = r.receipt.max_complete_residual;
        }
        EngineEstimate::Exact(_) => {
            if value.status != 1 {
                return Err(BackendError::invariant(
                    "numerical_result",
                    "exact attachment status mismatch",
                ));
            }
        }
    }
    Ok(value)
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_numerical_result_v1(
    generation: u64,
    output: *mut VckssNumericalResultV1,
    capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssNumericalResultV1>(
            output.cast(),
            capacity_bytes,
            "numerical result",
        )?;
        let state = lock_engine("numerical_result")?;
        let solved = state
            .registry
            .result(ContextHandle::from_generation(generation)?)?;
        write_output(output, result(generation, solved)?);
        Ok(())
    })
}

/// Validate the complete destination before writing; no R-sized export scratch.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vckss_rust_engine_numerical_rhs_v1(
    generation: u64,
    output: *mut VckssNumericalRhsV1,
    capacity: u64,
    written: *mut u64,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<u64>(written.cast(), 8, "numerical RHS count")?;
        let state = lock_engine("numerical_rhs")?;
        let solved = state
            .registry
            .result(ContextHandle::from_generation(generation)?)?;
        let a = solved
            .numerical
            .as_ref()
            .ok_or_else(|| unsupported("result has no numerical attachment"))?;
        let n = match a {
            Attachment::Exact => 0,
            Attachment::Generic(a) => a.replay_rhs.len(),
            Attachment::Compressed(a) => a.replay_rhs.len(),
        };
        let bytes = capacity
            .checked_mul(size_of::<VckssNumericalRhsV1>() as u64)
            .ok_or_else(|| resource_error("numerical_rhs", "RHS capacity overflow"))?;
        if bytes > isize::MAX as u64 || capacity < n as u64 || (n != 0 && output.is_null()) {
            return Err(abi_error("invalid or short numerical RHS destination"));
        }
        let start = output as usize;
        let end = start
            .checked_add(n * size_of::<VckssNumericalRhsV1>())
            .ok_or_else(|| abi_error("numerical RHS address overflow"))?;
        let count_start = written as usize;
        let count_end = count_start
            .checked_add(size_of::<u64>())
            .ok_or_else(|| abi_error("numerical count address overflow"))?;
        if start < count_end && count_start < end {
            return Err(abi_error("numerical RHS and count destinations overlap"));
        }
        for i in 0..n {
            let (probe, residual) = match a {
                Attachment::Exact => unreachable!("exact attachment has no replay RHS"),
                Attachment::Generic(a) => {
                    (a.replay_rhs[i].probe, a.replay_rhs[i].complete_residual)
                }
                Attachment::Compressed(a) => (
                    a.replay_rhs[i].probe,
                    a.replay_rhs[i].solve.complete_residual,
                ),
            };
            // SAFETY: caller owns the validated writable capacity; no reference
            // into its memory forms, including for packed C destinations.
            unsafe {
                output.add(i).write_unaligned(VckssNumericalRhsV1 {
                    schema_version: 1,
                    phase: 1,
                    probe,
                    reserved: 0,
                    complete_residual: residual,
                });
            }
        }
        write_output(written, n as u64);
        Ok(())
    })
}

/// Separate symbol and semantic receipt for CMG work including opted-in replay.
/// The frozen point-only full-CMG model symbol rejects these generations.
#[no_mangle]
pub extern "C" fn vckss_rust_engine_numerical_cmg_work_v1(
    generation: u64,
    output: *mut VckssFullCmgModelReceiptV1,
    capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssFullCmgModelReceiptV1>(
            output.cast(),
            capacity_bytes,
            "numerical CMG work",
        )?;
        let state = lock_engine("numerical_cmg_work")?;
        let solved = state
            .registry
            .result(ContextHandle::from_generation(generation)?)?;
        let numerical = result(generation, solved)?;
        let receipt = solved
            .full_cmg
            .as_ref()
            .ok_or_else(|| unsupported("numerical result did not use full CMG"))?;
        let logical = numerical
            .point_rhs
            .checked_add(numerical.executed_replay_rhs)
            .ok_or_else(|| resource_error("numerical_cmg_work", "logical work overflow"))?;
        let d = &receipt.model_diagnostics;
        if logical.checked_add(d.control_refinement_rhs_count) != Some(receipt.rhs_count)
            || d.explicit_options_rhs_count != u64::from(solved.controls_count)
            || d.controlled_rhs_count > numerical.point_rhs
        {
            return Err(BackendError::invariant(
                "numerical_cmg_work",
                "CMG point/replay work does not reconcile",
            ));
        }
        write_output(
            output,
            VckssFullCmgModelReceiptV1 {
                struct_size: size_of::<VckssFullCmgModelReceiptV1>() as u32,
                schema_version: 1,
                generation,
                controls_count: solved.controls_count,
                nuisance_mode: nuisance_code(solved.nuisance),
                logical_rhs_count: logical,
                explicit_options_rhs_count: d.explicit_options_rhs_count,
                controlled_rhs_count: d.controlled_rhs_count,
                control_refinement_rhs_count: d.control_refinement_rhs_count,
            },
        );
        Ok(())
    })
}

/// Numerical schema V2 on the frozen legacy compressed point request.
/// The request and all point receipts retain their original ABI meanings.
#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_numerical_legacy_v2(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV2,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "numerical legacy V2 request")?;
        require_abi(request.options.v1.abi_version)?;
        let callback = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "numerical_legacy",
        )?;
        match callback {
            Some(mut callback) => solve_engine_v2_execution(
                generation,
                request.options,
                None,
                true,
                V4SolveExecution::Caller(&mut callback),
            ),
            None => solve_engine_v2_execution(
                generation,
                request.options,
                None,
                true,
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

/// Numerical schema V2 on the frozen legacy generic point request.
#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_numerical_generic_legacy_v2(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV3,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "numerical generic legacy V2 request")?;
        require_abi(request.options.v2.v1.abi_version)?;
        let callback = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "numerical_legacy",
        )?;
        match callback {
            Some(mut callback) => {
                solve_engine_v3_numerical(generation, request.options, true, &mut callback)
            }
            None => {
                solve_engine_v3_numerical(generation, request.options, true, &mut NeverInterrupt)
            }
        }
    })
}
