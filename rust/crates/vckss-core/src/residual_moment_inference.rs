// SPDX-License-Identifier: GPL-3.0-only

//! Residual-moment attachments, including the direct residual-probe V4 policy.
//! The named structured model describes the variance basis; the separate
//! result diagnostic identifies residual-moment fitting rather than old CV.

use crate::component_inference::{
    prepare_grouped_structured_component_inference,
    prepare_structured_component_inference_with_interrupt, ComponentInferenceOptions,
    ComponentInferenceUnit, ComponentVarianceSource, PreparedComponentInference,
};
use crate::error::{BackendError, Result};
use crate::interrupt::InterruptCheck;
use crate::problem::CompressedProblem;
use crate::residual_moments::{
    memory_plan, GramMethod, ResidualMomentFit, ResidualMomentOptions,
    ResidualMomentPreparationDiagnostic,
};
use crate::structured_variance::StructuredVarianceOptions;

pub const ORDERING_CONTRACT: &str = "FEVC-OBSERVATION-OUTCOME-FREE-KEY-V1";
pub const DESIGN_ORDERING_CONTRACT: &str = "FEVC-OBSERVATION-DESIGN-ORDER-V1";
pub const MATCH_ORDERING_CONTRACT: &str = "FEVC-MATCH-DESIGN-ORDER-V1";

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub seed: u64,
    pub probes: usize,
    pub batch_width: usize,
    pub gram_method: GramMethod,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            seed: 1,
            probes: 512,
            batch_width: 16,
            gram_method: GramMethod::LegacyProjectedCovariance,
        }
    }
}

impl Options {
    pub(crate) fn moment_options(self) -> ResidualMomentOptions {
        ResidualMomentOptions {
            seed: self.seed,
            probes: self.probes,
            batch_width: self.batch_width,
            gram_method: self.gram_method,
            ..ResidualMomentOptions::default()
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ProjectionReceipt {
    pub probe: usize,
    pub iterations: u32,
    pub reduced_residual: f64,
    pub complete_residual: f64,
    pub full_residual_tolerance: f64,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub ordering_contract: &'static str,
    pub estimated_leverage_input: bool,
    pub preparation: ResidualMomentPreparationDiagnostic,
    pub gram: Vec<f64>,
    /// Original retained-row order, including raw predictions and floor counts.
    pub fit: ResidualMomentFit,
    pub projections: Vec<ProjectionReceipt>,
    /// Original basis columns retained in order; no outcome-dependent selection.
    pub basis_columns: Vec<usize>,
    pub basis_reconstruction_error: f64,
}

/// Prepare the explicit internal candidate. The caller certifies that the
/// unique probe-order key is fixed from design information before outcomes.
/// We can verify finite uniqueness, not the provenance of a caller's key.
pub fn prepare_with_interrupt(
    problem: &CompressedProblem,
    source: ComponentVarianceSource,
    inference: ComponentInferenceOptions,
    options: Options,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    let terms = match source {
        ComponentVarianceSource::StructuredCommon => 15,
        ComponentVarianceSource::StructuredLeverage => 3,
        ComponentVarianceSource::Oracle => {
            return Err(BackendError::invalid(
                "residual_moment_inference_prepare",
                "residual moments require a named structured basis",
            ))
        }
    };
    let mut moment = options.moment_options();
    // The complete generic-JLA request owns memory admission, before RNG.
    moment.additional_memory_limit_bytes = usize::MAX;
    memory_plan(problem.outcome.len(), terms, moment)?;
    crate::generic_jla::residual_moment_attachment::design_order(problem, interrupt)?;
    let mut prepared = prepare_structured_component_inference_with_interrupt(
        problem,
        source,
        inference,
        StructuredVarianceOptions::default(),
        interrupt,
    )?;
    prepared.residual_moments = Some(options);
    Ok(prepared)
}

/// Default for the additive public observation attachment. No caller-owned
/// unique key is required: structural rows determine numerical addresses.
pub fn prepare_default_with_interrupt(
    problem: &CompressedProblem,
    source: ComponentVarianceSource,
    inference: ComponentInferenceOptions,
    structured: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    let mut prepared = prepare_structured_component_inference_with_interrupt(
        problem, source, inference, structured, interrupt,
    )?;
    let options = Options {
        seed: inference.seed,
        batch_width: inference.batch_width.min(16),
        ..Options::default()
    };
    let mut moment = options.moment_options();
    moment.rank_tolerance = structured.rank_tolerance;
    moment.positivity_multiplier = structured.positivity_multiplier;
    moment.observations_per_term = structured.observations_per_term;
    moment.additional_memory_limit_bytes = usize::MAX;
    memory_plan(
        problem.outcome.len(),
        if source == ComponentVarianceSource::StructuredCommon {
            15
        } else {
            3
        },
        moment,
    )?;
    prepared.residual_moments = Some(options);
    prepared.individual_intervals = true;
    prepared.design_only_order = true;
    Ok(prepared)
}

/// Unified fitting policy; legacy constructors retain their original behavior.
pub fn prepare_unified_with_interrupt(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
    source: ComponentVarianceSource,
    inference: ComponentInferenceOptions,
    structured: StructuredVarianceOptions,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    let options = Options {
        seed: inference.seed,
        batch_width: inference.batch_width.min(16),
        ..Options::default()
    };
    prepare_unified_options_with_interrupt(
        problem, unit, source, inference, structured, options, interrupt,
    )
}

/// Current public policy. The count is validated before any estimator RNG;
/// all other numerical domains and the residual-moment fitting rule are fixed.
#[allow(clippy::too_many_arguments)]
pub fn prepare_direct_with_interrupt(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
    source: ComponentVarianceSource,
    inference: ComponentInferenceOptions,
    structured: StructuredVarianceOptions,
    gram_probes: u32,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    if !(512..=i32::MAX as u32).contains(&gram_probes) {
        return Err(BackendError::invalid(
            "residual_moment_inference_prepare",
            "Gram probes must be in [512,2147483647]",
        ));
    }
    let options = Options {
        seed: inference.seed,
        probes: gram_probes as usize,
        batch_width: inference.batch_width.min(16),
        gram_method: GramMethod::DirectResidualCovariance,
    };
    prepare_unified_options_with_interrupt(
        problem, unit, source, inference, structured, options, interrupt,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_unified_options_with_interrupt(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
    source: ComponentVarianceSource,
    inference: ComponentInferenceOptions,
    structured: StructuredVarianceOptions,
    options: Options,
    interrupt: &mut dyn InterruptCheck,
) -> Result<PreparedComponentInference> {
    let mut prepared = match unit {
        ComponentInferenceUnit::Observation => {
            prepare_structured_component_inference_with_interrupt(
                problem, source, inference, structured, interrupt,
            )?
        }
        ComponentInferenceUnit::Match => {
            prepare_grouped_structured_component_inference(problem, source, inference, structured)?
        }
    };
    let units = match unit {
        ComponentInferenceUnit::Observation => problem.outcome.len(),
        ComponentInferenceUnit::Match => problem.deletion_units(),
    };
    let mut moment = options.moment_options();
    moment.observations_per_term = structured.observations_per_term;
    moment.additional_memory_limit_bytes = usize::MAX;
    // At least the intercept must be identified. Full active-basis support is
    // checked after outcome-free reduction, not against redundant raw columns.
    memory_plan(units, 1, moment)?;
    prepared.residual_moments = Some(options);
    prepared.individual_intervals = true;
    prepared.design_only_order = unit == ComponentInferenceUnit::Observation;
    prepared.unified_variance_fit = true;
    Ok(prepared)
}
