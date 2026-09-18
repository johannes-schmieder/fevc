// SPDX-License-Identifier: GPL-3.0-only

//! Opt-in experimental exact executor. No V1--V8 entrypoint selects it.
use super::*;
use vckss_core::exact_estimator::{parallel, ExactThreadReceipt};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssExactExecutionRequestV1 {
    pub v4: VckssEngineSolveRequestV4,
    pub threads: u32,
    pub reserved: u32,
}

impl Default for VckssExactExecutionRequestV1 {
    fn default() -> Self {
        let mut v4 = VckssEngineSolveRequestV4::default();
        v4.v3.v2.v1.struct_size = size_of::<Self>() as u32;
        v4.v3.v2.algorithm = VCKSS_ALGORITHM_EXACT;
        Self {
            v4,
            threads: 1,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssExactExecutionRequestInterruptV1 {
    pub options: VckssExactExecutionRequestV1,
    pub interrupt_poll: VckssInterruptPollV1,
    pub interrupt_context: *mut c_void,
    pub checkpoint_interval: u32,
    pub reserved: u32,
}

impl Default for VckssExactExecutionRequestInterruptV1 {
    fn default() -> Self {
        let mut options = VckssExactExecutionRequestV1::default();
        options.v4.v3.v2.v1.struct_size = size_of::<Self>() as u32;
        Self {
            options,
            interrupt_poll: None,
            interrupt_context: std::ptr::null_mut(),
            checkpoint_interval: 0,
            reserved: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VckssExactExecutionReceiptV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub generation: u64,
    pub requested_threads: u32,
    /// Selected worker bound, not a measurement of simultaneously busy cores.
    pub worker_limit: u32,
    pub parallel_regions: u64,
    pub estimator_passes: u64,
    pub incremental_forecast_bytes: u64,
    pub command_peak_forecast_bytes: u64,
    pub maximum_original_fit_residual: f64,
}

const _: [(); 296] = [(); size_of::<VckssExactExecutionRequestV1>()];
const _: [(); 320] = [(); size_of::<VckssExactExecutionRequestInterruptV1>()];
const _: [(); 64] = [(); size_of::<VckssExactExecutionReceiptV1>()];
const _: [(); 288] = [(); std::mem::offset_of!(VckssExactExecutionRequestV1, threads)];

fn validate(request: VckssExactExecutionRequestV1, algorithm: u32) -> Result<usize> {
    require_abi(request.v4.v3.v2.v1.abi_version)?;
    if request.v4.v3.v2.v1.struct_size < struct_size_u32::<VckssExactExecutionRequestV1>()?
        || request.reserved != 0
    {
        return Err(abi_error(
            "invalid exact-execution structure size or reserved field",
        ));
    }
    if !cfg!(any(target_os = "macos", target_os = "linux")) {
        return Err(BackendError::new(
            ErrorCode::UnsupportedFeature,
            "exact_execution",
            "experimental exact execution is enabled only on macOS and Linux",
        ));
    }
    if request.threads == 0 || request.v4.v3.v2.algorithm != algorithm {
        return Err(BackendError::invalid(
            "exact_execution",
            "positive threads and the selector's exact/auto algorithm required",
        ));
    }
    usize::try_from(request.threads)
        .map_err(|_| resource_error("exact_execution", "thread count overflow"))
}

/// Separate opt-in capability; it does not change the legacy core-ready mask.
#[no_mangle]
pub extern "C" fn vckss_rust_exact_execution_schema_v1() -> u32 {
    u32::from(cfg!(any(target_os = "macos", target_os = "linux")))
}

/// Additive legacy-request transport: original V2 payload, V1 capability,
/// and ignored exact tuning values remain unchanged. No old selector opts in.
#[no_mangle]
pub extern "C" fn vckss_rust_exact_legacy_execution_schema_v1() -> u32 {
    vckss_rust_exact_execution_schema_v1()
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_exact_legacy_execution_interrupt_v1(
    generation: u64,
    request: *const VckssEngineSolveRequestInterruptV2,
    threads: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "legacy exact execution request")?;
        require_abi(request.options.v1.abi_version)?;
        if request.options.v1.struct_size < struct_size_u32::<VckssEngineSolveRequestInterruptV2>()?
        {
            return Err(abi_error("short legacy exact execution request"));
        }
        if vckss_rust_exact_legacy_execution_schema_v1() != 1 {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "exact_execution",
                "unsupported platform",
            ));
        }
        if threads == 0 || request.options.algorithm != VCKSS_ALGORITHM_EXACT {
            return Err(BackendError::invalid(
                "exact_execution",
                "explicit exact and positive threads required",
            ));
        }
        let threads = usize::try_from(threads)
            .map_err(|_| resource_error("exact_execution", "thread count overflow"))?;
        let callback = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "exact_execution",
        )?;
        match callback {
            Some(mut callback) => solve_engine_v2_execution(
                generation,
                request.options,
                Some(threads),
                V4SolveExecution::Coordinated(&mut callback),
            ),
            None => solve_engine_v2_execution(
                generation,
                request.options,
                Some(threads),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_exact_execution_v1(
    generation: u64,
    request: *const VckssExactExecutionRequestV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "exact execution request")?;
        let threads = validate(request, VCKSS_ALGORITHM_EXACT)?;
        solve_engine_v4_with_generic_execution(
            generation,
            request.v4,
            None,
            Some(GenericExecutionPlan::ExactParallel(threads)),
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_exact_execution_interrupt_v1(
    generation: u64,
    request: *const VckssExactExecutionRequestInterruptV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "interruptible exact execution request")?;
        if request.options.v4.v3.v2.v1.struct_size
            < struct_size_u32::<VckssExactExecutionRequestInterruptV1>()?
        {
            return Err(abi_error("short interruptible exact execution request"));
        }
        let threads = validate(request.options, VCKSS_ALGORITHM_EXACT)?;
        let callback = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "exact_execution",
        )?;
        match callback {
            Some(mut callback) => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v4,
                None,
                Some(GenericExecutionPlan::ExactParallel(threads)),
                V4SolveExecution::Coordinated(&mut callback),
            ),
            None => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v4,
                None,
                Some(GenericExecutionPlan::ExactParallel(threads)),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

/// Resolved-auto exact has a distinct capability and selectors. V1 remains
/// explicit-exact only, and all legacy V4--V8 entrypoints retain their meaning.
#[no_mangle]
pub extern "C" fn vckss_rust_exact_resolved_execution_schema_v2() -> u32 {
    2 * u32::from(cfg!(any(target_os = "macos", target_os = "linux")))
}

/// V2 uses the same frozen payload layout; the unchanged V3 capability
/// signature binds the original auto request. The common solve boundary
/// rejects a non-exact resolved plan before any estimator work or RNG.
#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_exact_resolved_execution_v2(
    generation: u64,
    request: *const VckssExactExecutionRequestV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "resolved exact execution request")?;
        let threads = validate(request, VCKSS_ALGORITHM_AUTO)?;
        solve_engine_v4_with_generic_execution(
            generation,
            request.v4,
            None,
            Some(GenericExecutionPlan::ExactParallel(threads)),
            V4SolveExecution::Caller(&mut NeverInterrupt),
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_exact_resolved_execution_interrupt_v2(
    generation: u64,
    request: *const VckssExactExecutionRequestInterruptV1,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "interruptible resolved exact request")?;
        if request.options.v4.v3.v2.v1.struct_size
            < struct_size_u32::<VckssExactExecutionRequestInterruptV1>()?
        {
            return Err(abi_error("short interruptible resolved exact request"));
        }
        let threads = validate(request.options, VCKSS_ALGORITHM_AUTO)?;
        let callback = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "exact_execution",
        )?;
        match callback {
            Some(mut callback) => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v4,
                None,
                Some(GenericExecutionPlan::ExactParallel(threads)),
                V4SolveExecution::Coordinated(&mut callback),
            ),
            None => solve_engine_v4_with_generic_execution(
                generation,
                request.options.v4,
                None,
                Some(GenericExecutionPlan::ExactParallel(threads)),
                V4SolveExecution::Caller(&mut NeverInterrupt),
            ),
        }
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_exact_execution_receipt_v1(
    generation: u64,
    output: *mut VckssExactExecutionReceiptV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssExactExecutionReceiptV1>(
            output.cast(),
            output_capacity_bytes,
            "exact execution receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("exact_execution")?;
        let solved = state.registry.result(handle)?;
        let receipt = solved.exact_execution.ok_or_else(|| {
            BackendError::new(
                ErrorCode::UnsupportedFeature,
                "exact_execution",
                "result has no experimental exact execution receipt",
            )
        })?;
        write_output(output, receipt);
        Ok(())
    })
}

pub(super) fn execute(
    generation: u64,
    problem: &vckss_core::problem::CompressedProblem,
    hybrid: Option<(
        &vckss_core::problem::CompressedProblem,
        &vckss_core::exact_estimator::ExactStayerHybridPlan,
    )>,
    options: ExactEstimatorOptions,
    wallseconds: Option<f64>,
    threads: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(
    vckss_core::exact_estimator::PlannedExactEstimatorResult,
    Option<ExactStayerHybridResult>,
    VckssExactExecutionReceiptV1,
)> {
    use vckss_core::counter_accounting::combine_counter_phases;
    use vckss_core::exact_estimator::{ExactPlanApplicability, PlannedExactEstimatorResult};
    // New native receipt plus its result-transition copies. Base/hybrid scalar
    // results are small but coexist, so keep them inside the admitted envelope.
    let mut options = options;
    options.prepared_persistent_bytes = options
        .prepared_persistent_bytes
        .checked_add(4096)
        .ok_or_else(|| resource_error("exact_execution", "receipt envelope overflow"))?;
    parallel::preflight(problem, options, None, threads, interrupt)?;
    let mut work = parallel::wall_work(problem, options)?;
    if let Some((problem, plan)) = hybrid {
        parallel::preflight(problem, options, Some(plan), threads, interrupt)?;
        let extra = parallel::wall_work(problem, options)?;
        for (value, addition) in [
            (&mut work.preparation, extra.preparation),
            (&mut work.engine_setup, extra.engine_setup),
            (&mut work.full_fit, extra.full_fit),
            (&mut work.leverage, extra.leverage),
            (&mut work.target, extra.target),
            (&mut work.result_export, extra.result_export),
        ] {
            *value = value
                .checked_add(addition)
                .ok_or_else(|| resource_error("exact_execution", "wall work overflow"))?;
        }
    }
    let wall = vckss_core::wall_plan::wall_work_receipt(
        work,
        wallseconds,
        vckss_core::wall_plan::WallCalibration::Uncalibrated,
    )?;
    let with_receipt = |bytes: u64| {
        bytes.checked_add(4096).ok_or_else(|| {
            resource_error("exact_execution", "incremental receipt forecast overflow")
        })
    };
    let base = parallel::run_with_interrupt(problem, options, None, threads, interrupt)?;
    let mut receipt = VckssExactExecutionReceiptV1 {
        struct_size: size_of::<VckssExactExecutionReceiptV1>() as u32,
        schema_version: 1,
        generation,
        requested_threads: to_u32(threads, "exact threads")?,
        worker_limit: to_u32(base.worker_limit, "exact worker limit")?,
        parallel_regions: to_u64(base.parallel_regions, "exact parallel regions")?,
        estimator_passes: 1,
        incremental_forecast_bytes: with_receipt(base.incremental_forecast_bytes)?,
        command_peak_forecast_bytes: base.estimator.receipt.peak_forecast_bytes,
        maximum_original_fit_residual: base
            .estimator
            .receipt
            .full_fit_relres
            .max(base.estimator.receipt.working_fit_relres),
    };
    let hybrid = hybrid
        .map(|(problem, plan)| {
            let value =
                parallel::run_with_interrupt(problem, options, Some(plan), threads, interrupt)?;
            receipt.worker_limit = receipt
                .worker_limit
                .max(to_u32(value.worker_limit, "exact worker limit")?);
            receipt.parallel_regions = receipt
                .parallel_regions
                .checked_add(to_u64(value.parallel_regions, "exact regions")?)
                .ok_or_else(|| {
                    resource_error("exact_execution", "parallel region count overflow")
                })?;
            receipt.estimator_passes += 1;
            receipt.incremental_forecast_bytes = receipt
                .incremental_forecast_bytes
                .max(with_receipt(value.incremental_forecast_bytes)?);
            receipt.command_peak_forecast_bytes = receipt
                .command_peak_forecast_bytes
                .max(value.estimator.receipt.peak_forecast_bytes);
            receipt.maximum_original_fit_residual = receipt
                .maximum_original_fit_residual
                .max(value.estimator.receipt.full_fit_relres)
                .max(value.estimator.receipt.working_fit_relres);
            let [mover_correction, stayer_correction] =
                value.correction_sources.ok_or_else(|| {
                    BackendError::invariant("exact_execution", "hybrid correction sources missing")
                })?;
            Ok::<_, BackendError>(ExactStayerHybridResult {
                estimator: value.estimator,
                mover_correction,
                stayer_correction,
            })
        })
        .transpose()?;
    let execution = ExactExecutionReceipt {
        schema_version: 2,
        selected_engine: SelectedEngine::NotApplicable,
        solver_route: ExactPlanApplicability::NotApplicable,
        batches: ExactPlanApplicability::NotApplicable,
        peak_forecast_bytes: receipt.command_peak_forecast_bytes,
        fit_peak_forecast_bytes: base.estimator.receipt.fit_peak_forecast_bytes.max(
            hybrid
                .as_ref()
                .map_or(0, |h| h.estimator.receipt.fit_peak_forecast_bytes),
        ),
        correction_peak_forecast_bytes: base.estimator.receipt.correction_peak_forecast_bytes.max(
            hybrid
                .as_ref()
                .map_or(0, |h| h.estimator.receipt.correction_peak_forecast_bytes),
        ),
        wall,
        counter: combine_counter_phases(Default::default(), Default::default())?,
        plan_frozen_before_execution: true,
        logical_atoms_before_plan_freeze: 0,
        unique_packed_words_before_plan_freeze: 0,
        physical_trials_before_plan_freeze: 0,
        threads: ExactThreadReceipt {
            requested: threads,
            used: receipt.worker_limit as usize,
            parallel_regions: usize::try_from(receipt.parallel_regions)
                .map_err(|_| resource_error("exact_execution", "region count overflow"))?,
        },
    };
    Ok((
        PlannedExactEstimatorResult {
            estimator: base.estimator,
            execution,
        },
        hybrid,
        receipt,
    ))
}
