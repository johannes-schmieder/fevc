// SPDX-License-Identifier: GPL-3.0-only

use super::*;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VckssMemoryPolicyV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub budget_present: u32,
    /// 1 = error, 2 = warn, 3 = off. An absent budget never admits/rejects.
    pub check_mode: u32,
    pub budget_bytes: u64,
}

impl VckssMemoryPolicyV1 {
    fn budget(self) -> Result<MemoryBudget> {
        if self.struct_size != struct_size_u32::<Self>()?
            || self.schema_version != 1
            || self.budget_present > 1
        {
            return Err(abi_error("invalid memory policy header or budget presence"));
        }
        let check = match self.check_mode {
            1 => MemoryCheck::Error,
            2 => MemoryCheck::Warn,
            3 => MemoryCheck::Off,
            _ => return Err(abi_error("unknown memory check policy")),
        };
        let budget = if self.budget_present == 0 {
            if self.budget_bytes != 0 {
                return Err(abi_error("absent memory budget carries bytes"));
            }
            MemoryBudget::Unspecified
        } else {
            MemoryBudget::Explicit {
                bytes: self.budget_bytes,
                check,
            }
        };
        budget.validate(0)?;
        Ok(budget)
    }
}

fn request_budget(
    policy: *const VckssMemoryPolicyV1,
    request: VckssEnginePrepareRequestV4,
) -> Result<MemoryBudget> {
    let policy = copy_sized_struct(policy, "memory policy")?;
    let budget = policy.budget()?;
    if request.v3.v2.memory_limit_bytes != policy.budget_bytes {
        return Err(abi_error("preparation and memory policy budgets disagree"));
    }
    Ok(budget)
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_admit_prepare_memory_v1(
    request: *const VckssEnginePrepareRequestV4,
    policy: *const VckssMemoryPolicyV1,
    probeorder_supplied: u32,
) -> i32 {
    ffi_status(|| {
        let request = copy_request_struct(request, "memory-policy preparation")?;
        if probeorder_supplied > 1 {
            return Err(abi_error("invalid probe-order presence"));
        }
        let budget = request_budget(policy, request)?;
        validate_prepare_request_v4_with_budget(request, probeorder_supplied == 1, budget)?;
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_prepare_memory_interrupt_v1(
    request: *const VckssEnginePrepareRequestInterruptV3,
    columns: *const VckssEngineColumnsV3,
    policy: *const VckssMemoryPolicyV1,
    output_handle: *mut u64,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<u64>(
            output_handle.cast(),
            output_capacity_bytes,
            "memory-policy output handle",
        )?;
        write_output(output_handle, 0);
        let request = copy_request_struct(request, "memory-policy interrupt preparation")?;
        let budget = request_budget(policy, request.options)?;
        let columns = copy_sized_struct(columns, "memory-policy columns")?;
        let (deletion, implicit_match, memory) = validate_prepare_request_v4_with_budget(
            request.options,
            columns.probeorder_supplied == 1,
            budget,
        )?;
        let mut callback = CallbackInterrupt::new(
            request.interrupt_poll,
            request.interrupt_context,
            request.checkpoint_interval,
            request.reserved,
            "prepare",
        )?;
        let mut inert = NeverInterrupt;
        let interrupt: &mut dyn InterruptCheck = match callback.as_mut() {
            Some(value) => value,
            None => &mut inert,
        };
        prepare_columns_value(
            request.options.v3.v2,
            request.options.v3.controls_count,
            deletion,
            implicit_match,
            memory,
            columns,
            output_handle,
            output_capacity_bytes,
            interrupt,
        )
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_memory_policy_v1(
    generation: u64,
    output: *mut VckssMemoryPolicyV1,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssMemoryPolicyV1>(
            output.cast(),
            output_capacity_bytes,
            "memory policy receipt",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("memory_policy_receipt")?;
        let memory = match state.registry.payload(handle)? {
            ContextPayloadRef::Prepared(prepared) => prepared.receipt.memory,
            ContextPayloadRef::Solved(solved) => solved.preparation.memory,
        };
        let limit = memory.budget.limit(memory.hard_limit_bytes);
        write_output(
            output,
            VckssMemoryPolicyV1 {
                struct_size: struct_size_u32::<VckssMemoryPolicyV1>()?,
                schema_version: 1,
                budget_present: u32::from(limit.is_some()),
                check_mode: match memory.budget.check() {
                    MemoryCheck::Error => 1,
                    MemoryCheck::Warn => 2,
                    MemoryCheck::Off => 3,
                },
                budget_bytes: limit.unwrap_or(0),
            },
        );
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memory_policy_layout_and_presence_are_explicit() {
        assert_eq!(size_of::<VckssMemoryPolicyV1>(), 24);
        assert_eq!(std::mem::offset_of!(VckssMemoryPolicyV1, budget_bytes), 16);
        let policy = VckssMemoryPolicyV1 {
            struct_size: 24,
            schema_version: 1,
            budget_present: 0,
            check_mode: 2,
            budget_bytes: 0,
        };
        assert_eq!(policy.budget().unwrap(), MemoryBudget::Unspecified);
        assert!(VckssMemoryPolicyV1 {
            budget_bytes: 1,
            ..policy
        }
        .budget()
        .is_err());
        assert!(VckssMemoryPolicyV1 {
            budget_present: 1,
            ..policy
        }
        .budget()
        .is_err());
        assert!(VckssMemoryPolicyV1 {
            check_mode: 9,
            ..policy
        }
        .budget()
        .is_err());
        let mut request = VckssEnginePrepareRequestV4::default();
        request.v3.v2.memory_limit_bytes = 42;
        assert!(request_budget(&policy, request).is_err());
        request.v3.v2.memory_limit_bytes = 0;
        assert_eq!(
            request_budget(&policy, request).unwrap(),
            MemoryBudget::Unspecified
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VckssMemoryForecastV1 {
    pub struct_size: u32,
    pub schema_version: u32,
    pub expected_peak_bytes: u64,
    pub admission_peak_bytes: u64,
    pub conditional_reserve_bytes: u64,
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_memory_forecast_v1(
    generation: u64,
    output: *mut VckssMemoryForecastV1,
    capacity: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssMemoryForecastV1>(
            output.cast(),
            capacity,
            "memory forecast",
        )?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("memory_forecast")?;
        let (expected, admission, preparation) =
            match state.registry.payload(handle)? {
                ContextPayloadRef::Prepared(prepared) => {
                    let peak = prepared.receipt.memory.preparation_peak_forecast_bytes;
                    (peak, peak, peak)
                }
                ContextPayloadRef::Solved(solved) => {
                    let (expected, admission) = match &solved.result {
                        EngineEstimate::Jla(result) => (
                            result.receipt.memory.expected_peak_forecast_bytes,
                            result.receipt.memory.solve_peak_forecast_bytes,
                        ),
                        EngineEstimate::GenericJla(result) => (
                            result.receipt.peak_forecast_bytes,
                            result.receipt.peak_forecast_bytes,
                        ),
                        EngineEstimate::Exact(result) => (
                            result.receipt.peak_forecast_bytes,
                            result.receipt.peak_forecast_bytes,
                        ),
                    };
                    let hybrid = solved
                        .stayer_hybrid
                        .as_ref()
                        .map_or(0, |value| value.estimator.receipt.peak_forecast_bytes);
                    (
                        expected.max(hybrid),
                        admission.max(hybrid),
                        solved
                            .preparation
                            .memory
                            .preparation_peak_forecast_bytes
                            .max(solved.stayer_augmentation.as_ref().map_or(0, |receipt| {
                                receipt.memory.augmentation_peak_forecast_bytes
                            })),
                    )
                }
            };
        let expected = expected.max(preparation);
        let admission = admission.max(preparation);
        write_output(
            output,
            VckssMemoryForecastV1 {
                struct_size: struct_size_u32::<VckssMemoryForecastV1>()?,
                schema_version: 1,
                expected_peak_bytes: expected,
                admission_peak_bytes: admission,
                conditional_reserve_bytes: admission.saturating_sub(expected),
            },
        );
        Ok(())
    })
}
