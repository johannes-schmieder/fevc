// SPDX-License-Identifier: GPL-3.0-only

//! Additive individual-result transport. Legacy result ABIs remain strict.
use super::*;
use vckss_core::component_inference::{ComponentJointStatus, ComponentQ0Status};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssComponentInferenceResultReceiptV5 {
    pub v4: VckssComponentInferenceResultReceiptV4,
    pub joint_status: u32,
    /// 1: legacy cross-fitted match/observation; 2: residual moments.
    pub variance_fit: u32,
    pub q0_status: [u32; 4],
    pub gram_probes: u32,
    /// 0: legacy; 1: unique key; 2: canonical observation rows; 3: canonical matches.
    pub ordering: u32,
    pub gram_rcond: f64,
    pub gram_inverse_relres: f64,
    pub variance_fit_relres: f64,
    pub positivity_floor: f64,
    pub floored_predictions: u64,
    pub nonpositive_predictions: u64,
}

const _: [(); 288] = [(); size_of::<VckssComponentInferenceResultReceiptV5>()];

#[no_mangle]
pub extern "C" fn vckss_rust_component_inference_interface_version() -> u32 {
    4
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vckss_rust_engine_component_inference_result_v5(
    generation: u64,
    primitive: *mut f64,
    primitive_capacity: u64,
    covariance: *mut f64,
    covariance_capacity: u64,
    mcse: *mut f64,
    mcse_capacity: u64,
    spectrum: *mut f64,
    spectrum_capacity: u64,
    q1: *mut f64,
    q1_capacity: u64,
    summaries: *mut f64,
    summaries_capacity: u64,
    folds: *mut f64,
    folds_capacity: u64,
    cv: *mut f64,
    cv_capacity: u64,
    targets: *mut f64,
    targets_capacity: u64,
    output: *mut VckssComponentInferenceResultReceiptV5,
    output_capacity_bytes: u32,
) -> i32 {
    ffi_status(|| {
        require_output_capacity::<VckssComponentInferenceResultReceiptV5>(
            output.cast::<u8>(),
            output_capacity_bytes,
            "component V5 receipt",
        )?;
        // Validate the complete caller inventory before writing any matrix.
        for (buffer, capacity, count, label) in [
            (primitive, primitive_capacity, 9, "component primitive"),
            (covariance, covariance_capacity, 16, "component covariance"),
            (mcse, mcse_capacity, 9, "component MCSE"),
            (spectrum, spectrum_capacity, 60, "component spectrum"),
            (summaries, summaries_capacity, 24, "component summaries"),
            (folds, folds_capacity, 150, "component folds"),
            (cv, cv_capacity, 490, "component CV"),
            (targets, targets_capacity, 8, "component target status"),
        ] {
            require_component_output(buffer, capacity, count, label)?;
        }
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_component_inference_result_v5")?;
        let solved = state.registry.result(handle)?;
        let EngineEstimate::GenericJla(generic) = &solved.result else {
            return Err(BackendError::invalid(
                "component_result_v5",
                "generic JLA required",
            ));
        };
        let result = generic.component_inference.as_ref().ok_or_else(|| {
            BackendError::invalid("component_result_v5", "no component attachment")
        })?;
        if result.q1.is_some() {
            require_component_output(q1, q1_capacity, 80, "component q1")?;
        } else if q1_capacity != 0 {
            return Err(BackendError::invalid(
                "component_result_v5",
                "q0 requires zero q1 capacity",
            ));
        }
        let receipt = receipt_v5(generation, result)?;
        let joint = result.joint_status == ComponentJointStatus::Computed;
        copy_component_output(
            primitive,
            primitive_capacity,
            if joint {
                &result.primitive_covariance
            } else {
                &[f64::NAN; 9]
            },
            "component primitive",
        )?;
        copy_component_output(
            covariance,
            covariance_capacity,
            if joint {
                &result.covariance
            } else {
                &[f64::NAN; 16]
            },
            "component covariance",
        )?;
        let target_values: [f64; 8] = core::array::from_fn(|index| {
            let target = index / 2;
            if index % 2 == 0 {
                result.covariance[target * 4 + target]
            } else {
                f64::from(result.q0_status[target] as u32)
            }
        });
        copy_component_output(
            targets,
            targets_capacity,
            &target_values,
            "component target status",
        )?;
        copy_component_output(mcse, mcse_capacity, &result.trace_mcse, "component MCSE")?;
        copy_component_output(
            spectrum,
            spectrum_capacity,
            &component_spectrum_values(&result.spectrum, &result.influence_concentration),
            "component spectrum",
        )?;
        if let Some(values) = &result.q1 {
            copy_component_output(
                q1,
                q1_capacity,
                &component_q1_values_v4(values),
                "component q1",
            )?;
        }
        if let Some(fit) = &result.structured_variance {
            copy_component_output(
                summaries,
                summaries_capacity,
                &component_variance_summary_values(&fit.summary),
                "component summaries",
            )?;
            copy_component_output(
                folds,
                folds_capacity,
                &component_fold_values(&fit.folds),
                "component folds",
            )?;
            copy_component_output(
                cv,
                cv_capacity,
                &component_cv_values(&fit.cv),
                "component CV",
            )?;
        } else {
            // Explicitly inapplicable, never fictitious successful CV results.
            copy_component_output(
                summaries,
                summaries_capacity,
                &[f64::NAN; 24],
                "component summaries",
            )?;
            copy_component_output(folds, folds_capacity, &[f64::NAN; 150], "component folds")?;
            copy_component_output(cv, cv_capacity, &[f64::NAN; 490], "component CV")?;
        }
        write_output(output, receipt);
        Ok(())
    })
}

fn receipt_v5(
    generation: u64,
    result: &ComponentInferenceResult,
) -> Result<VckssComponentInferenceResultReceiptV5> {
    let mut base = component_inference_result_receipt(
        generation,
        result,
        result.structured_variance.as_ref(),
    )?;
    base.schema_version = 5;
    let q1_present = result.q1.is_some();
    let computed_targets = result.q1.as_ref().map_or_else(
        || {
            result
                .q0_status
                .iter()
                .filter(|&&status| status == ComponentQ0Status::Computed)
                .count()
        },
        |targets| {
            targets
                .iter()
                .filter(|target| target.status == ComponentQ1Status::Computed)
                .count()
        },
    );
    let mut receipt = VckssComponentInferenceResultReceiptV5 {
        v4: VckssComponentInferenceResultReceiptV4 {
            v3: VckssComponentInferenceResultReceiptV3 {
                v2: base,
                q1_columns: if q1_present { 20 } else { 0 },
                critical_simulations: result.critical_simulations,
                maximum_remainder_identity_error: result.q1.as_ref().map_or(0.0, |targets| {
                    targets
                        .iter()
                        .map(|target| target.remainder_identity_error)
                        .fold(0.0, f64::max)
                }),
                reserved: 0,
            },
            computed_targets: to_u32(computed_targets, "computed targets")?,
            solver_columns: to_u32(
                result.solve_receipts.len()
                    + result
                        .residual_moments
                        .as_ref()
                        .map_or(0, |fit| fit.projections.len()),
                "inference solver columns",
            )?,
            critical_draws: result.q1.as_ref().map_or(0, |targets| {
                targets
                    .iter()
                    .map(|target| u64::from(target.critical_draws))
                    .sum()
            }),
        },
        joint_status: result.joint_status as u32,
        variance_fit: 1,
        q0_status: result.q0_status.map(|status| status as u32),
        gram_rcond: f64::NAN,
        gram_inverse_relres: f64::NAN,
        variance_fit_relres: f64::NAN,
        positivity_floor: f64::NAN,
        ..VckssComponentInferenceResultReceiptV5::default()
    };
    if let Some(fit) = &result.residual_moments {
        receipt.variance_fit = 2;
        receipt.gram_probes = to_u32(fit.preparation.probes, "Gram probes")?;
        receipt.ordering = if fit.ordering_contract
            == vckss_core::residual_moment_inference::DESIGN_ORDERING_CONTRACT
        {
            2
        } else if fit.ordering_contract
            == vckss_core::residual_moment_inference::MATCH_ORDERING_CONTRACT
        {
            3
        } else {
            1
        };
        receipt.gram_rcond = fit.preparation.gram_rcond;
        receipt.gram_inverse_relres = fit.preparation.gram_inverse_relres;
        receipt.variance_fit_relres = fit.fit.moment_relative_residual;
        receipt.positivity_floor = fit.fit.positivity_floor;
        receipt.floored_predictions = to_u64(fit.fit.floored_predictions, "floored predictions")?;
        receipt.nonpositive_predictions =
            to_u64(fit.fit.nonpositive_predictions, "nonpositive predictions")?;
    } else if result.structured_variance.is_none() {
        return Err(BackendError::invalid(
            "component_result_v5",
            "public inference requires fitted structured variances",
        ));
    }
    Ok(receipt)
}
