// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic fitted-variance qualification for structured component inference.
//!
//! The executable deliberately constructs its FEVC matrices independently of
//! the generic-JLA component attachment.  A diagonal-plus-low-rank
//! factorization evaluates the observation-deletion quadratic form and its
//! exact conditional covariance without either observation-by-observation
//! matrices or randomized covariance probes.  Tests retain a tiny dense oracle.

#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::env;
use std::fmt::Write as _;

use vckss_core::component_inference::{finish_q1_interval, finish_q1_target, q1_am_interval};
use vckss_core::interrupt::NeverInterrupt;
use vckss_core::structured_variance::{
    fit_structured_variance_with_interrupt, StructuredVarianceModel, StructuredVarianceOptions,
};

// Frozen campaign schemas count an unavailable target as a failed attempt.
// They must not serialize NaN intervals or treat a partial core result as success.
fn require_computed_q1(
    value: vckss_core::component_inference::ComponentQ1TargetResult,
) -> vckss_core::error::Result<vckss_core::component_inference::ComponentQ1TargetResult> {
    if value.status == vckss_core::component_inference::ComponentQ1Status::Computed {
        Ok(value)
    } else {
        Err(vckss_core::error::BackendError::new(
            vckss_core::error::ErrorCode::JlaConstraintFailed,
            "component_inference_q1",
            format!("target unavailable: {:?}", value.status),
        ))
    }
}

const LEVEL: f64 = 0.95;
const DEVELOPMENT_SEED: u64 = 20_261_001;
const CONFIRMATION_SEED: u64 = 20_262_001;
const INTERVAL_SEED: u64 = 8_675_309;
const V4_CALIBRATION_SEED: u64 = 0x6f9a_31c2_074d_85e1;
const V4_EVALUATION_SEED: u64 = 0xbd42_7619_a05e_3cf8;
const V4_REFERENCE_CALIBRATION_SEED: u64 = 0x293e_8cb7_54a1_f602;
const V4_REFERENCE_EVALUATION_SEED: u64 = 0xe517_40ad_9b63_28c4;
const V5_CONFIRMATION_SEED: u64 = 0x7a93_d4c1_52e8_6b0f;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Profile {
    Development,
    Confirmation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reference {
    Q0,
    Q1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VarianceDgp {
    Homoskedastic,
    Common,
    Leverage,
    MildFunctional,
    MildOmitted,
    SevereOmitted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErrorDgp {
    Gaussian,
    StudentT8,
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    name: &'static str,
    k: usize,
    controls: bool,
    dominant: bool,
    variance: VarianceDgp,
    model: StructuredVarianceModel,
    error: ErrorDgp,
    reference: Reference,
    beta_zero: bool,
    replications: usize,
    gate: &'static str,
}

#[derive(Clone, Debug)]
struct FactorizedDesign {
    rows: usize,
    parameters: usize,
    x: Vec<f64>,
    sparse_x: Vec<Vec<(usize, f64)>>,
    information_inverse: Vec<f64>,
    leverage: Vec<f64>,
    maker_inverse: Vec<f64>,
    kernel_factor: [Vec<f64>; 4],
    ratio: [Vec<f64>; 4],
    remainder_factor: [Vec<f64>; 4],
    remainder_ratio: [Vec<f64>; 4],
    target_diagonal: [Vec<f64>; 3],
    fold_entity: Vec<u64>,
    variance_ranks: [Vec<f64>; 4],
    hidden_driver: Vec<f64>,
    beta: Vec<f64>,
    truth: [f64; 4],
    leading_value: [f64; 4],
    leading_mode: [Vec<f64>; 4],
    leading_share: [f64; 4],
    remainder_share: [f64; 4],
}

#[derive(Clone, Copy, Debug, Default)]
struct Summary {
    attempts: usize,
    successes: usize,
    covered: usize,
    sum_error: f64,
    sum_error_square: f64,
    sum_se: f64,
    sum_variance: f64,
    sum_floor_share: f64,
    sum_boundary_share: f64,
}

impl Summary {
    fn push(&mut self, error: f64, standard_error: f64, covered: bool, floor: f64, boundary: f64) {
        self.attempts += 1;
        if !error.is_finite() || !standard_error.is_finite() || standard_error <= 0.0 {
            return;
        }
        self.successes += 1;
        self.covered += usize::from(covered);
        self.sum_error += error;
        self.sum_error_square += error * error;
        self.sum_se += standard_error;
        self.sum_variance += standard_error * standard_error;
        self.sum_floor_share += floor;
        self.sum_boundary_share += boundary;
    }

    fn values(self) -> [f64; 10] {
        let n = self.successes as f64;
        if self.successes < 2 {
            return [f64::NAN; 10];
        }
        let bias = self.sum_error / n;
        let centered = ((self.sum_error_square - n * bias * bias) / (n - 1.0)).max(0.0);
        let empirical_sd = centered.sqrt();
        let coverage = self.covered as f64 / n;
        [
            bias,
            empirical_sd / n.sqrt(),
            coverage,
            (coverage * (1.0 - coverage) / n).sqrt(),
            empirical_sd,
            self.sum_se / n,
            empirical_sd / (self.sum_se / n),
            self.sum_variance / n,
            self.sum_floor_share / n,
            self.sum_boundary_share / n,
        ]
    }
}

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.first().map(String::as_str) == Some("confirmation-matrix-v5") {
        run_confirmation_matrix(&arguments[1..]);
        return;
    }
    if arguments.first().map(String::as_str) == Some("diagnostic-q1-v4") {
        run_q1_reference_diagnostic(&arguments[1..]);
        return;
    }
    if arguments.first().map(String::as_str) == Some("diagnostic-q1-v3") {
        run_q1_diagnostic(&arguments[1..]);
        return;
    }
    let profile = parse_profile();
    println!(
        "schema,profile,cell,gate,k,controls,dominant,variance_model,variance_dgp,error_dgp,reference,beta,target,replications,successes,bias,bias_mcse,coverage,coverage_mcse,empirical_sd,mean_se,se_ratio,mean_estimated_variance,leading_share,remainder_share,mean_floor_share,mean_boundary_share"
    );
    for cell in cells(profile) {
        run_cell(profile, cell);
    }
}

fn run_confirmation_matrix(arguments: &[String]) {
    assert_eq!(
        arguments.len(),
        4,
        "confirmation-matrix-v5 requires CELL K START REPLICATIONS"
    );
    let name = arguments[0].as_str();
    let k = arguments[1].parse::<usize>().expect("K is an integer");
    let start = arguments[2].parse::<usize>().expect("START is an integer");
    let replications = arguments[3]
        .parse::<usize>()
        .expect("REPLICATIONS is an integer");
    let cell = cells(Profile::Confirmation)
        .into_iter()
        .find(|candidate| candidate.name == name && candidate.k == k)
        .expect("CELL and K identify a registered V5 matrix cell");
    assert!(
        replications > 0
            && start
                .checked_add(replications)
                .is_some_and(|end| end <= cell.replications),
        "invalid confirmation-matrix-v5 replication range"
    );
    run_confirmation_matrix_cell(cell, start, replications);
}

fn emit_confirmation_failure(
    cell: Cell,
    replication: usize,
    target: usize,
    semantic_seed: u64,
    status: &str,
) {
    println!(
        "{{\"schema\":\"fevc-structured-inference-confirmation-v5\",\"cell\":\"{}\",\"k\":{},\"replication\":{},\"target\":\"{}\",\"semantic_seed\":{},\"status\":\"{}\"}}",
        cell.name,
        cell.k,
        replication,
        target_name(target),
        semantic_seed,
        status,
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_confirmation_success(
    cell: Cell,
    replication: usize,
    target: usize,
    semantic_seed: u64,
    truth: f64,
    point_error: f64,
    estimated_sd: f64,
    lower: f64,
    upper: f64,
    leading_variance: f64,
    remainder_variance: f64,
    leading_remainder_covariance: f64,
    remainder_identity_error: f64,
    critical: f64,
    floor_share: f64,
    boundary_share: f64,
    maximum_boundary_excess: f64,
    maximum_prediction_leverage: f64,
    minimum_fitted_rcond: f64,
    leading_share: f64,
    remainder_share: f64,
) {
    println!(
        "{{\"schema\":\"fevc-structured-inference-confirmation-v5\",\"cell\":\"{}\",\"gate\":\"{}\",\"k\":{},\"controls\":{},\"dominant\":{},\"variance_model\":\"{}\",\"variance_dgp\":\"{:?}\",\"error_dgp\":\"{:?}\",\"reference\":\"{:?}\",\"beta\":\"{}\",\"replication\":{},\"target\":\"{}\",\"semantic_seed\":{semantic_seed},\"status\":\"success\",\"point_error\":{point_error:.17e},\"estimated_sd\":{estimated_sd:.17e},\"covered\":{},\"lower_miss\":{},\"upper_miss\":{},\"interval_width\":{:.17e},\"leading_variance\":{leading_variance:.17e},\"remainder_variance\":{remainder_variance:.17e},\"leading_remainder_covariance\":{leading_remainder_covariance:.17e},\"remainder_identity_error\":{remainder_identity_error:.17e},\"critical\":{critical:.17e},\"leading_share\":{:.17e},\"remainder_share\":{:.17e},\"floor_share\":{floor_share:.17e},\"boundary_share\":{boundary_share:.17e},\"maximum_boundary_excess\":{maximum_boundary_excess:.17e},\"maximum_prediction_leverage\":{maximum_prediction_leverage:.17e},\"minimum_fitted_rcond\":{minimum_fitted_rcond:.17e}}}",
        cell.name,
        cell.gate,
        cell.k,
        cell.controls,
        cell.dominant,
        match cell.model {
            StructuredVarianceModel::Common => "structured_common",
            StructuredVarianceModel::LeverageOnly => "structured_leverage",
        },
        cell.variance,
        cell.error,
        cell.reference,
        if cell.beta_zero { "zero" } else { "nonzero" },
        replication,
        target_name(target),
        truth >= lower && truth <= upper,
        truth < lower,
        truth > upper,
        upper - lower,
        leading_share,
        remainder_share,
    );
}

fn run_confirmation_matrix_cell(cell: Cell, start: usize, replications: usize) {
    let design = make_design(cell.k, cell.controls, cell.dominant, cell.beta_zero);
    let variance = make_variance(&design, cell.variance);
    let mean = matrix_vector(&design.x, design.rows, design.parameters, &design.beta);
    for replication in start..start + replications {
        let replication_seed = semantic_seed(V5_CONFIRMATION_SEED, cell.name, cell.k, replication);
        let mut rng = IndependentRng::new(replication_seed);
        let error = (0..design.rows)
            .map(|row| variance[row].sqrt() * rng.standardized_error(cell.error))
            .collect::<Vec<_>>();
        let outcome = mean
            .iter()
            .zip(error)
            .map(|(left, right)| left + right)
            .collect::<Vec<_>>();
        let residual = maker_action(&design, &outcome);
        let proxy = (0..design.rows)
            .map(|row| outcome[row] * residual[row] * design.maker_inverse[row])
            .collect::<Vec<_>>();
        let Ok(fitted) = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &design.maker_inverse,
            &design.leverage,
            &design.target_diagonal,
            &design.fold_entity,
            StructuredVarianceOptions {
                seed: INTERVAL_SEED,
                ..StructuredVarianceOptions::default()
            },
            &mut NeverInterrupt,
        ) else {
            for target in 0..4 {
                emit_confirmation_failure(
                    cell,
                    replication,
                    target,
                    replication_seed,
                    "variance_fit_failed",
                );
            }
            continue;
        };
        let selected = fitted.selected(cell.model);
        let model_row = usize::from(cell.model == StructuredVarianceModel::LeverageOnly);
        let fit = fitted.summary[model_row];
        for target in 0..4 {
            let influence = kernel_action(
                &design,
                &design.kernel_factor[target],
                &design.ratio[target],
                &outcome,
            );
            let point = dot(&outcome, &influence);
            let point_error = point - design.truth[target];
            let full_trace = factorized_trace_variance(
                &design,
                &design.kernel_factor[target],
                &design.ratio[target],
                selected,
            );
            let full_linear = 4.0
                * influence
                    .iter()
                    .zip(selected)
                    .map(|(left, right)| left * left * right)
                    .sum::<f64>();
            let full_variance = full_linear - full_trace;
            if !full_variance.is_finite() || full_variance <= 0.0 {
                emit_confirmation_failure(
                    cell,
                    replication,
                    target,
                    replication_seed,
                    "q0_variance_failed",
                );
                continue;
            }
            match cell.reference {
                Reference::Q0 => {
                    let estimated_sd = full_variance.sqrt();
                    let critical = 1.959_963_984_540_054;
                    emit_confirmation_success(
                        cell,
                        replication,
                        target,
                        replication_seed,
                        design.truth[target],
                        point_error,
                        estimated_sd,
                        point - critical * estimated_sd,
                        point + critical * estimated_sd,
                        0.0,
                        full_variance,
                        0.0,
                        0.0,
                        critical,
                        fit.floor_share,
                        fit.boundary_share,
                        fit.maximum_boundary_excess,
                        fit.maximum_prediction_leverage,
                        fit.minimum_fitted_rcond,
                        design.leading_share[target],
                        design.remainder_share[target],
                    );
                }
                Reference::Q1 => {
                    let lambda = design.leading_value[target];
                    let mode = &design.leading_mode[target];
                    let score = dot(mode, &outcome);
                    let remainder_influence = kernel_action(
                        &design,
                        &design.remainder_factor[target],
                        &design.remainder_ratio[target],
                        &outcome,
                    );
                    let trace = factorized_trace_variance(
                        &design,
                        &design.remainder_factor[target],
                        &design.remainder_ratio[target],
                        selected,
                    );
                    let leading_variance_correction = mode
                        .iter()
                        .zip(&proxy)
                        .map(|(mode, proxy)| mode * mode * proxy)
                        .sum::<f64>();
                    let direct_remainder_estimate = dot(&outcome, &remainder_influence);
                    let result = finish_q1_target(
                        point,
                        score,
                        leading_variance_correction,
                        direct_remainder_estimate,
                        1.0e-9,
                        lambda,
                        mode,
                        &remainder_influence,
                        selected,
                        trace,
                        0.0,
                        1.0e-8,
                    )
                    .and_then(require_computed_q1);
                    let Ok(result) = result else {
                        emit_confirmation_failure(
                            cell,
                            replication,
                            target,
                            replication_seed,
                            "q1_failed",
                        );
                        continue;
                    };
                    let critical = reference_q1_critical(result.curvature, LEVEL);
                    let interval = q1_am_interval(
                        [result.leading_score, result.remainder_estimate],
                        [
                            result.leading_variance,
                            result.leading_remainder_covariance,
                            result.leading_remainder_covariance,
                            result.remainder_variance,
                        ],
                        critical,
                        lambda,
                    );
                    let Ok(interval) = interval else {
                        emit_confirmation_failure(
                            cell,
                            replication,
                            target,
                            replication_seed,
                            "q1_interval_failed",
                        );
                        continue;
                    };
                    emit_confirmation_success(
                        cell,
                        replication,
                        target,
                        replication_seed,
                        design.truth[target],
                        point_error,
                        full_variance.sqrt(),
                        interval[0],
                        interval[1],
                        result.leading_variance,
                        result.remainder_variance,
                        result.leading_remainder_covariance,
                        result.remainder_identity_error,
                        critical,
                        fit.floor_share,
                        fit.boundary_share,
                        fit.maximum_boundary_excess,
                        fit.maximum_prediction_leverage,
                        fit.minimum_fitted_rcond,
                        design.leading_share[target],
                        design.remainder_share[target],
                    );
                }
            }
        }
    }
}

fn run_q1_diagnostic(arguments: &[String]) {
    assert_eq!(
        arguments.len(),
        3,
        "diagnostic-q1-v3 requires K START REPLICATIONS"
    );
    let k = arguments[0].parse::<usize>().expect("K is an integer");
    let start = arguments[1].parse::<usize>().expect("START is an integer");
    let replications = arguments[2]
        .parse::<usize>()
        .expect("REPLICATIONS is an integer");
    assert!(
        k >= 8 && replications > 0,
        "invalid diagnostic-q1-v3 bounds"
    );
    for (name, variance_dgp, model, error_dgp) in [
        (
            "dominant_leverage",
            VarianceDgp::Leverage,
            StructuredVarianceModel::LeverageOnly,
            ErrorDgp::Gaussian,
        ),
        (
            "dominant_common_t8",
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::StudentT8,
        ),
    ] {
        run_q1_diagnostic_cell(name, k, start, replications, variance_dgp, model, error_dgp);
    }
}

#[allow(clippy::too_many_arguments)]
fn run_q1_diagnostic_cell(
    name: &str,
    k: usize,
    start: usize,
    replications: usize,
    variance_dgp: VarianceDgp,
    model: StructuredVarianceModel,
    error_dgp: ErrorDgp,
) {
    const TARGET: usize = 1;
    let design = make_design(k, false, true, false);
    let true_variance = make_variance(&design, variance_dgp);
    let mean = matrix_vector(&design.x, design.rows, design.parameters, &design.beta);
    let mode_mean = dot(&design.leading_mode[TARGET], &mean);
    let remainder_truth =
        design.truth[TARGET] - design.leading_value[TARGET] * mode_mean * mode_mean;
    for replication in start..start + replications {
        let outcome_seed = semantic_seed(CONFIRMATION_SEED, name, k, replication);
        let mut rng = IndependentRng::new(outcome_seed);
        let outcome = mean
            .iter()
            .zip(&true_variance)
            .map(|(mean, variance)| mean + variance.sqrt() * rng.standardized_error(error_dgp))
            .collect::<Vec<_>>();
        let residual = maker_action(&design, &outcome);
        let proxy = (0..design.rows)
            .map(|row| outcome[row] * residual[row] * design.maker_inverse[row])
            .collect::<Vec<_>>();
        let fitted = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &design.maker_inverse,
            &design.leverage,
            &design.target_diagonal,
            &design.fold_entity,
            StructuredVarianceOptions {
                seed: INTERVAL_SEED,
                ..StructuredVarianceOptions::default()
            },
            &mut NeverInterrupt,
        );
        emit_q1_diagnostic(
            name,
            k,
            replication,
            "oracle",
            &design,
            &outcome,
            &proxy,
            &true_variance,
            mode_mean,
            remainder_truth,
            0.0,
            0.0,
        );
        match fitted {
            Ok(value) => {
                let model_row = usize::from(model == StructuredVarianceModel::LeverageOnly);
                emit_q1_diagnostic(
                    name,
                    k,
                    replication,
                    "fitted",
                    &design,
                    &outcome,
                    &proxy,
                    value.selected(model),
                    mode_mean,
                    remainder_truth,
                    value.summary[model_row].floor_share,
                    value.summary[model_row].boundary_share,
                );
            }
            Err(_) => emit_q1_failure(name, k, replication, "fitted", "variance_fit_failed"),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_q1_diagnostic(
    name: &str,
    k: usize,
    replication: usize,
    variance_source: &str,
    design: &FactorizedDesign,
    outcome: &[f64],
    variance_proxy: &[f64],
    variance: &[f64],
    mode_mean: f64,
    remainder_truth: f64,
    floor_share: f64,
    boundary_share: f64,
) {
    const TARGET: usize = 1;
    let influence = kernel_action(
        design,
        &design.kernel_factor[TARGET],
        &design.ratio[TARGET],
        outcome,
    );
    let point = dot(outcome, &influence);
    let full_trace = factorized_trace_variance(
        design,
        &design.kernel_factor[TARGET],
        &design.ratio[TARGET],
        variance,
    );
    let full_linear = 4.0
        * influence
            .iter()
            .zip(variance)
            .map(|(influence, variance)| influence * influence * variance)
            .sum::<f64>();
    let full_variance = full_linear - full_trace;
    let score = dot(&design.leading_mode[TARGET], outcome);
    let remainder_influence = kernel_action(
        design,
        &design.remainder_factor[TARGET],
        &design.remainder_ratio[TARGET],
        outcome,
    );
    let remainder_trace = factorized_trace_variance(
        design,
        &design.remainder_factor[TARGET],
        &design.remainder_ratio[TARGET],
        variance,
    );
    let leading_variance_correction = design.leading_mode[TARGET]
        .iter()
        .zip(variance_proxy)
        .map(|(mode, proxy)| mode * mode * proxy)
        .sum::<f64>();
    let direct_remainder_estimate = dot(outcome, &remainder_influence);
    let target_result = finish_q1_target(
        point,
        score,
        leading_variance_correction,
        direct_remainder_estimate,
        1.0e-9,
        design.leading_value[TARGET],
        &design.leading_mode[TARGET],
        &remainder_influence,
        variance,
        remainder_trace,
        0.0,
        1.0e-8,
    )
    .and_then(require_computed_q1);
    let Ok(mut result) = target_result else {
        emit_q1_failure(name, k, replication, variance_source, "q1_failed");
        return;
    };
    let critical = reference_q1_critical(result.curvature, LEVEL);
    let covariance = [
        result.leading_variance,
        result.leading_remainder_covariance,
        result.leading_remainder_covariance,
        result.remainder_variance,
    ];
    let interval = q1_am_interval(
        [result.leading_score, result.remainder_estimate],
        covariance,
        critical,
        design.leading_value[TARGET],
    );
    let legacy_remainder =
        point - design.leading_value[TARGET] * (score * score - result.leading_variance);
    let legacy_interval = q1_am_interval(
        [result.leading_score, legacy_remainder],
        covariance,
        critical,
        design.leading_value[TARGET],
    );
    let (Ok(interval), Ok(legacy_interval)) = (interval, legacy_interval) else {
        emit_q1_failure(name, k, replication, variance_source, "q1_interval_failed");
        return;
    };
    result.critical_value = critical;
    result.confidence_lower = interval[0];
    result.confidence_upper = interval[1];
    if !full_variance.is_finite() || full_variance <= 0.0 {
        emit_q1_failure(
            name,
            k,
            replication,
            variance_source,
            "full_variance_failed",
        );
        return;
    }
    let maximum_mode_share = design.leading_mode[TARGET]
        .iter()
        .zip(variance)
        .map(|(mode, variance)| mode * mode * variance / result.leading_variance)
        .fold(0.0_f64, f64::max);
    let covered = design.truth[TARGET] >= result.confidence_lower
        && design.truth[TARGET] <= result.confidence_upper;
    let legacy_covered =
        design.truth[TARGET] >= legacy_interval[0] && design.truth[TARGET] <= legacy_interval[1];
    let lower_miss = design.truth[TARGET] < result.confidence_lower;
    let upper_miss = design.truth[TARGET] > result.confidence_upper;
    println!(
        "{{\"schema\":\"fevc-q1-diagnostic-v3\",\"cell\":\"{name}\",\"k\":{k},\"replication\":{replication},\"variance_source\":\"{variance_source}\",\"status\":\"success\",\"point_error\":{:.17e},\"estimated_sd\":{:.17e},\"covered\":{},\"legacy_covered\":{},\"lower_miss\":{},\"upper_miss\":{},\"leading_variance\":{:.17e},\"leading_variance_correction\":{:.17e},\"remainder_identity_error\":{:.17e},\"remainder_variance\":{:.17e},\"leading_remainder_covariance\":{:.17e},\"curvature\":{:.17e},\"reference_critical\":{:.17e},\"interval_width\":{:.17e},\"score_error\":{:.17e},\"remainder_error\":{:.17e},\"leading_share\":{:.17e},\"remainder_share\":{:.17e},\"maximum_mode_share\":{:.17e},\"remainder_influence_concentration\":{:.17e},\"floor_share\":{:.17e},\"boundary_share\":{:.17e}}}",
        point - design.truth[TARGET],
        full_variance.sqrt(),
        covered,
        legacy_covered,
        lower_miss,
        upper_miss,
        result.leading_variance,
        result.leading_variance_correction,
        result.remainder_identity_error,
        result.remainder_variance,
        result.leading_remainder_covariance,
        result.curvature,
        result.critical_value,
        result.confidence_upper - result.confidence_lower,
        score - mode_mean,
        result.remainder_estimate - remainder_truth,
        design.leading_share[TARGET],
        design.remainder_share[TARGET],
        maximum_mode_share,
        result.remainder_influence_concentration,
        floor_share,
        boundary_share,
    );
}

fn emit_q1_failure(name: &str, k: usize, replication: usize, variance_source: &str, reason: &str) {
    let mut output = String::new();
    write!(
        output,
        "{{\"schema\":\"fevc-q1-diagnostic-v3\",\"cell\":\"{name}\",\"k\":{k},\"replication\":{replication},\"variance_source\":\"{variance_source}\",\"status\":\"{reason}\"}}"
    )
    .expect("write to string");
    println!("{output}");
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum V4Sample {
    Calibration,
    Evaluation,
}

impl V4Sample {
    fn parse(value: &str) -> Self {
        match value {
            "calibration" => Self::Calibration,
            "evaluation" => Self::Evaluation,
            _ => panic!("diagnostic-q1-v4 SAMPLE must be calibration or evaluation"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Calibration => "calibration",
            Self::Evaluation => "evaluation",
        }
    }

    const fn outcome_seed(self) -> u64 {
        match self {
            Self::Calibration => V4_CALIBRATION_SEED,
            Self::Evaluation => V4_EVALUATION_SEED,
        }
    }

    const fn reference_seed(self) -> u64 {
        match self {
            Self::Calibration => V4_REFERENCE_CALIBRATION_SEED,
            Self::Evaluation => V4_REFERENCE_EVALUATION_SEED,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct V4Covariance {
    leading: f64,
    cross: f64,
    remainder: f64,
}

impl V4Covariance {
    fn curvature(self, eigenvalue: f64) -> Option<f64> {
        if !self.leading.is_finite()
            || !self.cross.is_finite()
            || !self.remainder.is_finite()
            || self.leading <= 0.0
            || self.remainder <= 0.0
        {
            return None;
        }
        let conditional = self.remainder - self.cross * self.cross / self.leading;
        (conditional.is_finite() && conditional > 0.0)
            .then(|| 2.0 * eigenvalue.abs() * self.leading / conditional.sqrt())
    }

    const fn array(self) -> [f64; 4] {
        [self.leading, self.cross, self.cross, self.remainder]
    }
}

#[derive(Clone, Copy, Debug)]
struct V4Interval {
    lower: f64,
    upper: f64,
    critical: f64,
    covered: bool,
    lower_miss: bool,
    upper_miss: bool,
}

fn v4_interval(
    center: [f64; 2],
    covariance: V4Covariance,
    eigenvalue: f64,
    truth: f64,
) -> Option<V4Interval> {
    let curvature = covariance.curvature(eigenvalue)?;
    let critical = reference_q1_critical(curvature, LEVEL);
    let interval = q1_am_interval(center, covariance.array(), critical, eigenvalue).ok()?;
    Some(V4Interval {
        lower: interval[0],
        upper: interval[1],
        critical,
        covered: truth >= interval[0] && truth <= interval[1],
        lower_miss: truth < interval[0],
        upper_miss: truth > interval[1],
    })
}

fn v4_write_interval(output: &mut String, name: &str, interval: V4Interval) {
    write!(
        output,
        ",\"{name}_lower\":{:.17e},\"{name}_upper\":{:.17e},\"{name}_width\":{:.17e},\"{name}_critical\":{:.17e},\"{name}_covered\":{},\"{name}_lower_miss\":{},\"{name}_upper_miss\":{}",
        interval.lower,
        interval.upper,
        interval.upper - interval.lower,
        interval.critical,
        interval.covered,
        interval.lower_miss,
        interval.upper_miss,
    )
    .expect("write interval JSON");
}

fn run_q1_reference_diagnostic(arguments: &[String]) {
    assert_eq!(
        arguments.len(),
        4,
        "diagnostic-q1-v4 requires SAMPLE K START REPLICATIONS"
    );
    let sample = V4Sample::parse(&arguments[0]);
    let k = arguments[1].parse::<usize>().expect("K is an integer");
    let start = arguments[2].parse::<usize>().expect("START is an integer");
    let replications = arguments[3]
        .parse::<usize>()
        .expect("REPLICATIONS is an integer");
    assert!(
        k >= 8 && replications > 0,
        "invalid diagnostic-q1-v4 bounds"
    );

    let design = make_design(k, false, true, false);
    let variance = make_variance(&design, VarianceDgp::Common);
    let mean = matrix_vector(&design.x, design.rows, design.parameters, &design.beta);
    let leading_mean = dot(&design.leading_mode[1], &mean);
    let remainder_truth = design.truth[1] - design.leading_value[1] * leading_mean * leading_mean;
    let population = v4_population_covariance(&design, &variance);

    for replication in start..start + replications {
        let paired_seed =
            semantic_seed(sample.outcome_seed(), "dominant_common_v4", k, replication);
        let mut gaussian = Vec::with_capacity(design.rows);
        let mut student_t8 = Vec::with_capacity(design.rows);
        for row in 0..design.rows {
            let (normal, t8) = v4_paired_errors(paired_seed, row);
            gaussian.push(mean[row] + variance[row].sqrt() * normal);
            student_t8.push(mean[row] + variance[row].sqrt() * t8);
        }
        emit_v4_outcome(
            sample,
            "gaussian",
            k,
            replication,
            paired_seed,
            &design,
            &gaussian,
            &variance,
            population,
            leading_mean,
            remainder_truth,
        );
        emit_v4_outcome(
            sample,
            "student_t8",
            k,
            replication,
            paired_seed,
            &design,
            &student_t8,
            &variance,
            population,
            leading_mean,
            remainder_truth,
        );
        emit_v4_reference(
            sample,
            k,
            replication,
            "gaussian_reference",
            false,
            &design,
            population,
            leading_mean,
            remainder_truth,
        );
        emit_v4_reference(
            sample,
            k,
            replication,
            "gaussian_reference_vertex",
            true,
            &design,
            population,
            leading_mean,
            remainder_truth,
        );
    }
}

fn v4_population_covariance(design: &FactorizedDesign, variance: &[f64]) -> (V4Covariance, f64) {
    const TARGET: usize = 1;
    let mean = matrix_vector(&design.x, design.rows, design.parameters, &design.beta);
    let leading = design.leading_mode[TARGET]
        .iter()
        .zip(variance)
        .map(|(mode, variance)| mode * mode * variance)
        .sum::<f64>();
    let remainder_influence = kernel_action(
        design,
        &design.remainder_factor[TARGET],
        &design.remainder_ratio[TARGET],
        &mean,
    );
    let remainder_trace = factorized_trace_variance(
        design,
        &design.remainder_factor[TARGET],
        &design.remainder_ratio[TARGET],
        variance,
    );
    let remainder = remainder_trace
        + 4.0
            * remainder_influence
                .iter()
                .zip(variance)
                .map(|(influence, variance)| influence * influence * variance)
                .sum::<f64>();
    let cross = 2.0
        * design.leading_mode[TARGET]
            .iter()
            .zip(variance)
            .zip(&remainder_influence)
            .map(|((mode, variance), influence)| mode * variance * influence)
            .sum::<f64>();
    let full_influence = kernel_action(
        design,
        &design.kernel_factor[TARGET],
        &design.ratio[TARGET],
        &mean,
    );
    let full_trace = factorized_trace_variance(
        design,
        &design.kernel_factor[TARGET],
        &design.ratio[TARGET],
        variance,
    );
    let full = full_trace
        + 4.0
            * full_influence
                .iter()
                .zip(variance)
                .map(|(influence, variance)| influence * influence * variance)
                .sum::<f64>();
    assert!(leading > 0.0 && remainder > 0.0 && full > 0.0);
    (
        V4Covariance {
            leading,
            // The leave-out remainder kernel has zero diagonal, so its
            // quadratic error term has zero covariance with the leading
            // score even without a zero-third-moment assumption.
            cross,
            remainder,
        },
        full,
    )
}

fn v4_paired_errors(seed: u64, row: usize) -> (f64, f64) {
    let mut rng = IndependentRng::new(splitmix64(seed ^ splitmix64(row as u64)));
    let numerator = rng.normal();
    let chi_square = (0..8).map(|_| rng.normal().powi(2)).sum::<f64>();
    let student_t8 = (6.0 / 8.0_f64).sqrt() * numerator / (chi_square / 8.0).sqrt();
    (numerator, student_t8)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn emit_v4_outcome(
    sample: V4Sample,
    error_dgp: &str,
    k: usize,
    replication: usize,
    paired_seed: u64,
    design: &FactorizedDesign,
    outcome: &[f64],
    variance: &[f64],
    population: (V4Covariance, f64),
    leading_mean: f64,
    remainder_truth: f64,
) {
    const TARGET: usize = 1;
    let (population_covariance, population_full_variance) = population;
    let residual = maker_action(design, outcome);
    let proxy = outcome
        .iter()
        .zip(&residual)
        .zip(&design.maker_inverse)
        .map(|((outcome, residual), maker_inverse)| outcome * residual * maker_inverse)
        .collect::<Vec<_>>();
    let full_influence = kernel_action(
        design,
        &design.kernel_factor[TARGET],
        &design.ratio[TARGET],
        outcome,
    );
    let point = dot(outcome, &full_influence);
    let full_trace = factorized_trace_variance(
        design,
        &design.kernel_factor[TARGET],
        &design.ratio[TARGET],
        variance,
    );
    let full_linear = 4.0
        * full_influence
            .iter()
            .zip(variance)
            .map(|(influence, variance)| influence * influence * variance)
            .sum::<f64>();
    let production_full_variance = full_linear - full_trace;
    let score = dot(&design.leading_mode[TARGET], outcome);
    let remainder_influence = kernel_action(
        design,
        &design.remainder_factor[TARGET],
        &design.remainder_ratio[TARGET],
        outcome,
    );
    let remainder_trace = factorized_trace_variance(
        design,
        &design.remainder_factor[TARGET],
        &design.remainder_ratio[TARGET],
        variance,
    );
    let leading_variance_correction = design.leading_mode[TARGET]
        .iter()
        .zip(&proxy)
        .map(|(mode, proxy)| mode * mode * proxy)
        .sum::<f64>();
    let direct_remainder = dot(outcome, &remainder_influence);
    let result = finish_q1_target(
        point,
        score,
        leading_variance_correction,
        direct_remainder,
        1.0e-9,
        design.leading_value[TARGET],
        &design.leading_mode[TARGET],
        &remainder_influence,
        variance,
        remainder_trace,
        0.0,
        1.0e-8,
    )
    .and_then(require_computed_q1);
    if !production_full_variance.is_finite() || production_full_variance <= 0.0 {
        println!(
            "{{\"schema\":\"fevc-q1-reference-diagnostic-v4\",\"kind\":\"outcome\",\"sample\":\"{}\",\"error_dgp\":\"{error_dgp}\",\"k\":{k},\"replication\":{replication},\"semantic_seed\":{paired_seed},\"status\":\"q0_variance_failed\"}}",
            sample.label()
        );
        return;
    }
    let Ok(result) = result else {
        println!(
            "{{\"schema\":\"fevc-q1-reference-diagnostic-v4\",\"kind\":\"outcome\",\"sample\":\"{}\",\"error_dgp\":\"{error_dgp}\",\"k\":{k},\"replication\":{replication},\"semantic_seed\":{paired_seed},\"status\":\"q1_failed\"}}",
            sample.label()
        );
        return;
    };
    let production_covariance = V4Covariance {
        leading: result.leading_variance,
        cross: result.leading_remainder_covariance,
        remainder: result.remainder_variance,
    };
    let leading_population_covariance = V4Covariance {
        leading: population_covariance.leading,
        ..production_covariance
    };
    let remainder_population_covariance = V4Covariance {
        remainder: population_covariance.remainder,
        ..production_covariance
    };
    let cross_population_covariance = V4Covariance {
        cross: population_covariance.cross,
        ..production_covariance
    };
    let center = [result.leading_score, result.remainder_estimate];
    let truth = design.truth[TARGET];
    let required_radius = v4_required_radius(
        center,
        population_covariance,
        design.leading_value[TARGET],
        truth,
    );
    let maximum_mode_share = design.leading_mode[TARGET]
        .iter()
        .zip(variance)
        .map(|(mode, variance)| mode * mode * variance / result.leading_variance)
        .fold(0.0_f64, f64::max);
    let maximum_full_influence_share = full_influence
        .iter()
        .zip(variance)
        .map(|(influence, variance)| 4.0 * influence * influence * variance / full_linear)
        .fold(0.0_f64, f64::max);
    let mut output = String::new();
    write!(
        output,
        "{{\"schema\":\"fevc-q1-reference-diagnostic-v4\",\"kind\":\"outcome\",\"sample\":\"{}\",\"error_dgp\":\"{error_dgp}\",\"k\":{k},\"replication\":{replication},\"semantic_seed\":{paired_seed},\"status\":\"success\",\"truth\":{truth:.17e},\"point_error\":{:.17e},\"score_error\":{:.17e},\"remainder_error\":{:.17e},\"leading_variance_estimated\":{:.17e},\"leading_variance_population\":{:.17e},\"remainder_variance_estimated\":{:.17e},\"remainder_variance_population\":{:.17e},\"cross_covariance_estimated\":{:.17e},\"cross_covariance_population\":{:.17e},\"full_variance_estimated\":{production_full_variance:.17e},\"full_variance_population\":{population_full_variance:.17e},\"leading_variance_correction\":{leading_variance_correction:.17e},\"remainder_identity_error\":{:.17e},\"leading_share\":{:.17e},\"remainder_share\":{:.17e},\"maximum_mode_share\":{maximum_mode_share:.17e},\"maximum_full_influence_share\":{maximum_full_influence_share:.17e},\"maximum_remainder_influence_share\":{:.17e},\"required_radius_population\":{required_radius:.17e}",
        sample.label(),
        point - truth,
        score - leading_mean,
        result.remainder_estimate - remainder_truth,
        result.leading_variance,
        population_covariance.leading,
        result.remainder_variance,
        population_covariance.remainder,
        result.leading_remainder_covariance,
        population_covariance.cross,
        result.remainder_identity_error,
        design.leading_share[TARGET],
        design.remainder_share[TARGET],
        result.remainder_influence_concentration,
    )
    .expect("write V4 outcome JSON");
    if sample == V4Sample::Evaluation {
        let intervals = [
            ("production_q1", production_covariance),
            ("fixed_population_q1", population_covariance),
            ("leading_population_q1", leading_population_covariance),
            ("remainder_population_q1", remainder_population_covariance),
            ("cross_population_q1", cross_population_covariance),
        ];
        for (name, covariance) in intervals {
            let Some(interval) =
                v4_interval(center, covariance, design.leading_value[TARGET], truth)
            else {
                println!(
                    "{{\"schema\":\"fevc-q1-reference-diagnostic-v4\",\"kind\":\"outcome\",\"sample\":\"{}\",\"error_dgp\":\"{error_dgp}\",\"k\":{k},\"replication\":{replication},\"semantic_seed\":{paired_seed},\"status\":\"{name}_failed\"}}",
                    sample.label()
                );
                return;
            };
            v4_write_interval(&mut output, name, interval);
        }
        for (name, variance_value) in [
            ("production_q0", production_full_variance),
            ("fixed_population_q0", population_full_variance),
        ] {
            if !variance_value.is_finite() || variance_value <= 0.0 {
                println!(
                    "{{\"schema\":\"fevc-q1-reference-diagnostic-v4\",\"kind\":\"outcome\",\"sample\":\"{}\",\"error_dgp\":\"{error_dgp}\",\"k\":{k},\"replication\":{replication},\"semantic_seed\":{paired_seed},\"status\":\"{name}_failed\"}}",
                    sample.label()
                );
                return;
            }
            let radius = 1.959_963_984_540_054 * variance_value.sqrt();
            let lower = point - radius;
            let upper = point + radius;
            let interval = V4Interval {
                lower,
                upper,
                critical: 1.959_963_984_540_054,
                covered: truth >= lower && truth <= upper,
                lower_miss: truth < lower,
                upper_miss: truth > upper,
            };
            v4_write_interval(&mut output, name, interval);
        }
    }
    output.push('}');
    println!("{output}");
}

fn emit_v4_reference(
    sample: V4Sample,
    k: usize,
    replication: usize,
    error_dgp: &str,
    vertex: bool,
    design: &FactorizedDesign,
    population: (V4Covariance, f64),
    design_leading_mean: f64,
    design_remainder_mean: f64,
) {
    const TARGET: usize = 1;
    let covariance = population.0;
    let semantic_seed = semantic_seed(sample.reference_seed(), "q1_reference_v4", k, replication);
    let mut rng = IndependentRng::new(semantic_seed);
    let first = rng.normal();
    let second = rng.normal();
    let leading_error = covariance.leading.sqrt() * first;
    let conditional =
        covariance.remainder - covariance.cross * covariance.cross / covariance.leading;
    let remainder_error =
        covariance.cross / covariance.leading.sqrt() * first + conditional.sqrt() * second;
    let (leading_mean, remainder_mean, truth) = if vertex {
        (0.0, 0.0, 0.0)
    } else {
        (
            design_leading_mean,
            design_remainder_mean,
            design.truth[TARGET],
        )
    };
    let center = [
        leading_mean + leading_error,
        remainder_mean + remainder_error,
    ];
    let required_radius =
        v4_required_radius(center, covariance, design.leading_value[TARGET], truth);
    let curvature = covariance
        .curvature(design.leading_value[TARGET])
        .expect("registered population covariance is positive definite");
    let critical = reference_q1_critical(curvature, LEVEL);
    let mut output = String::new();
    write!(
        output,
        "{{\"schema\":\"fevc-q1-reference-diagnostic-v4\",\"kind\":\"reference\",\"sample\":\"{}\",\"error_dgp\":\"{error_dgp}\",\"k\":{k},\"replication\":{replication},\"semantic_seed\":{semantic_seed},\"status\":\"success\",\"truth\":{truth:.17e},\"leading_mean\":{leading_mean:.17e},\"remainder_mean\":{remainder_mean:.17e},\"score_error\":{leading_error:.17e},\"remainder_error\":{remainder_error:.17e},\"leading_variance_population\":{:.17e},\"remainder_variance_population\":{:.17e},\"cross_covariance_population\":{:.17e},\"curvature_population\":{curvature:.17e},\"theoretical_critical\":{critical:.17e},\"required_radius_population\":{required_radius:.17e}",
        sample.label(),
        covariance.leading,
        covariance.remainder,
        covariance.cross,
    )
    .expect("write V4 reference JSON");
    if sample == V4Sample::Evaluation {
        let interval = q1_am_interval(
            center,
            covariance.array(),
            critical,
            design.leading_value[TARGET],
        )
        .expect("reference-law confidence ellipse is valid");
        let diagnostic = V4Interval {
            lower: interval[0],
            upper: interval[1],
            critical,
            covered: truth >= interval[0] && truth <= interval[1],
            lower_miss: truth < interval[0],
            upper_miss: truth > interval[1],
        };
        v4_write_interval(&mut output, "reference_q1", diagnostic);
    }
    output.push('}');
    println!("{output}");
}

fn v4_required_radius(
    center: [f64; 2],
    covariance: V4Covariance,
    eigenvalue: f64,
    truth: f64,
) -> f64 {
    let leading = covariance.leading;
    let slope = covariance.cross / leading;
    let conditional = covariance.remainder - covariance.cross * covariance.cross / leading;
    assert!(leading > 0.0 && conditional > 0.0);
    if eigenvalue.abs() <= 1.0e-14 {
        return (center[1] - truth).abs() / covariance.remainder.sqrt();
    }
    let constant = center[1] - truth - slope * center[0];
    let cubic = [
        2.0 * eigenvalue * eigenvalue / conditional,
        3.0 * eigenvalue * slope / conditional,
        leading.recip() + (slope * slope + 2.0 * eigenvalue * constant) / conditional,
        -center[0] / leading + slope * constant / conditional,
    ];
    let roots = v4_cubic_real_roots(cubic);
    let objective = |candidate: f64| {
        let first = center[0] - candidate;
        let second = center[1] - (truth - eigenvalue * candidate * candidate) - slope * first;
        first * first / leading + second * second / conditional
    };
    let minimum = roots
        .into_iter()
        .map(objective)
        .fold(f64::INFINITY, f64::min);
    assert!(minimum.is_finite() && minimum >= -1.0e-12);
    minimum.max(0.0).sqrt()
}

fn v4_cubic_real_roots(coefficient: [f64; 4]) -> Vec<f64> {
    let [a3, a2, a1, a0] = coefficient;
    assert!(a3.is_finite() && a3 > 0.0);
    let bound = 1.0 + (a2 / a3).abs().max((a1 / a3).abs()).max((a0 / a3).abs());
    let polynomial = |value: f64| ((a3 * value + a2) * value + a1) * value + a0;
    let derivative_discriminant = 4.0 * a2 * a2 - 12.0 * a3 * a1;
    let mut boundary = vec![-bound, bound];
    if derivative_discriminant >= 0.0 {
        let root = derivative_discriminant.sqrt();
        for value in [
            (-2.0 * a2 - root) / (6.0 * a3),
            (-2.0 * a2 + root) / (6.0 * a3),
        ] {
            if value > -bound && value < bound {
                boundary.push(value);
            }
        }
    }
    boundary.sort_by(f64::total_cmp);
    boundary.dedup_by(|left, right| {
        (*left - *right).abs() <= 1.0e-14 * left.abs().max(right.abs()).max(1.0)
    });
    let scale = a3.abs() * bound.powi(3) + a2.abs() * bound.powi(2) + a1.abs() * bound + a0.abs();
    let zero_tolerance = 1.0e-13 * scale.max(1.0);
    let mut roots = Vec::new();
    for &value in &boundary {
        if polynomial(value).abs() <= zero_tolerance {
            roots.push(value);
        }
    }
    for interval in boundary.windows(2) {
        let mut left = interval[0];
        let mut right = interval[1];
        let mut left_value = polynomial(left);
        let right_value = polynomial(right);
        if left_value == 0.0 || right_value == 0.0 || left_value.signum() == right_value.signum() {
            continue;
        }
        for _ in 0..128 {
            let midpoint = 0.5 * (left + right);
            let midpoint_value = polynomial(midpoint);
            if midpoint_value == 0.0 {
                left = midpoint;
                right = midpoint;
                break;
            }
            if left_value.signum() == midpoint_value.signum() {
                left = midpoint;
                left_value = midpoint_value;
            } else {
                right = midpoint;
            }
        }
        roots.push(0.5 * (left + right));
    }
    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|left, right| {
        (*left - *right).abs() <= 1.0e-11 * left.abs().max(right.abs()).max(1.0)
    });
    assert!(!roots.is_empty());
    roots
}

#[path = "common/q1_reference.rs"]
mod q1_reference;
#[cfg(test)]
use q1_reference::reference_normal_cdf;
use q1_reference::reference_q1_critical;

fn parse_profile() -> Profile {
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        None | Some("development") => Profile::Development,
        Some("confirmation") => Profile::Confirmation,
        Some(value) => panic!("unknown qualification profile: {value}"),
    }
}

fn cells(profile: Profile) -> Vec<Cell> {
    let reps = |k: usize| match profile {
        Profile::Development => 100,
        Profile::Confirmation => match k {
            8 => 5_000,
            11 => 4_000,
            _ => 2_500,
        },
    };
    let mut output = Vec::new();
    let sequence: &[usize] = match profile {
        Profile::Development => &[12],
        Profile::Confirmation => &[12, 16],
    };
    for &k in sequence {
        output.push(Cell {
            name: "diffuse_common",
            k,
            controls: false,
            dominant: false,
            variance: VarianceDgp::Common,
            model: StructuredVarianceModel::Common,
            error: ErrorDgp::Gaussian,
            reference: Reference::Q0,
            beta_zero: false,
            replications: reps(k),
            gate: "correct",
        });
        output.push(Cell {
            name: "dominant_common",
            k,
            controls: false,
            dominant: true,
            variance: VarianceDgp::Common,
            model: StructuredVarianceModel::Common,
            error: ErrorDgp::Gaussian,
            reference: Reference::Q1,
            beta_zero: false,
            replications: reps(k),
            gate: "correct",
        });
    }
    let k = *sequence.last().expect("qualification sequence is nonempty");
    let extra = [
        (
            "diffuse_homoskedastic",
            false,
            false,
            VarianceDgp::Homoskedastic,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            false,
            "correct",
        ),
        (
            "diffuse_leverage",
            false,
            false,
            VarianceDgp::Leverage,
            StructuredVarianceModel::LeverageOnly,
            ErrorDgp::Gaussian,
            Reference::Q0,
            false,
            "correct",
        ),
        (
            "dominant_leverage",
            false,
            true,
            VarianceDgp::Leverage,
            StructuredVarianceModel::LeverageOnly,
            ErrorDgp::Gaussian,
            Reference::Q1,
            false,
            "correct",
        ),
        (
            "diffuse_common_t8",
            false,
            false,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::StudentT8,
            Reference::Q0,
            false,
            "correct",
        ),
        (
            "dominant_common_t8",
            false,
            true,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::StudentT8,
            Reference::Q1,
            false,
            "correct",
        ),
        (
            "diffuse_common_controls",
            true,
            false,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            false,
            "correct",
        ),
        (
            "dominant_common_controls",
            true,
            true,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q1,
            false,
            "correct",
        ),
        (
            "diffuse_mild_functional",
            false,
            false,
            VarianceDgp::MildFunctional,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            false,
            "mild",
        ),
        (
            "dominant_mild_functional",
            false,
            true,
            VarianceDgp::MildFunctional,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q1,
            false,
            "mild",
        ),
        (
            "diffuse_mild_omitted",
            false,
            false,
            VarianceDgp::MildOmitted,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            false,
            "mild",
        ),
        (
            "dominant_mild_omitted",
            false,
            true,
            VarianceDgp::MildOmitted,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q1,
            false,
            "mild",
        ),
        (
            "diffuse_severe_omitted",
            false,
            false,
            VarianceDgp::SevereOmitted,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            false,
            "descriptive",
        ),
        (
            "dominant_severe_omitted",
            false,
            true,
            VarianceDgp::SevereOmitted,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q1,
            false,
            "descriptive",
        ),
        (
            "diffuse_common_null",
            false,
            false,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            true,
            "diagnostic",
        ),
        (
            "dominant_common_null_q0",
            false,
            true,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q0,
            true,
            "diagnostic",
        ),
        (
            "dominant_common_null",
            false,
            true,
            VarianceDgp::Common,
            StructuredVarianceModel::Common,
            ErrorDgp::Gaussian,
            Reference::Q1,
            true,
            "diagnostic",
        ),
    ];
    for (name, controls, dominant, variance, model, error, reference, beta_zero, gate) in extra {
        output.push(Cell {
            name,
            k,
            controls,
            dominant,
            variance,
            model,
            error,
            reference,
            beta_zero,
            replications: reps(k),
            gate,
        });
    }
    output
}

fn run_cell(profile: Profile, cell: Cell) {
    let design = make_design(cell.k, cell.controls, cell.dominant, cell.beta_zero);
    let variance = make_variance(&design, cell.variance);
    let mut summaries = [Summary::default(); 4];
    let outcome_seed = match profile {
        Profile::Development => DEVELOPMENT_SEED,
        Profile::Confirmation => CONFIRMATION_SEED,
    } ^ hash_label(cell.name);
    for replication in 0..cell.replications {
        let mut rng = IndependentRng::new(outcome_seed ^ replication as u64);
        let error = (0..design.rows)
            .map(|row| variance[row].sqrt() * rng.standardized_error(cell.error))
            .collect::<Vec<_>>();
        let mean = matrix_vector(&design.x, design.rows, design.parameters, &design.beta);
        let outcome = mean
            .iter()
            .zip(error)
            .map(|(left, right)| left + right)
            .collect::<Vec<_>>();
        let residual = maker_action(&design, &outcome);
        let proxy = (0..design.rows)
            .map(|row| outcome[row] * residual[row] * design.maker_inverse[row])
            .collect::<Vec<_>>();
        let fitted = fit_structured_variance_with_interrupt(
            &proxy,
            &residual,
            &design.maker_inverse,
            &design.leverage,
            &design.target_diagonal,
            &design.fold_entity,
            StructuredVarianceOptions {
                seed: INTERVAL_SEED,
                ..StructuredVarianceOptions::default()
            },
            &mut NeverInterrupt,
        );
        let Ok(fitted) = fitted else {
            for summary in &mut summaries {
                summary.attempts += 1;
            }
            continue;
        };
        let selected = fitted.selected(cell.model);
        let model_row = usize::from(cell.model == StructuredVarianceModel::LeverageOnly);
        let floor = fitted.summary[model_row].floor_share;
        let boundary = fitted.summary[model_row].boundary_share;
        for target in 0..4 {
            let influence = kernel_action(
                &design,
                &design.kernel_factor[target],
                &design.ratio[target],
                &outcome,
            );
            let point = dot(&outcome, &influence);
            let point_error = point - design.truth[target];
            match cell.reference {
                Reference::Q0 => {
                    let trace = factorized_trace_variance(
                        &design,
                        &design.kernel_factor[target],
                        &design.ratio[target],
                        selected,
                    );
                    let linear = 4.0
                        * influence
                            .iter()
                            .zip(selected)
                            .map(|(left, right)| left * left * right)
                            .sum::<f64>();
                    let estimated = linear - trace;
                    if estimated > 0.0 && estimated.is_finite() {
                        let se = estimated.sqrt();
                        summaries[target].push(
                            point_error,
                            se,
                            point_error.abs() <= 1.959_963_984_540_054 * se,
                            floor,
                            boundary,
                        );
                    } else {
                        summaries[target].attempts += 1;
                    }
                }
                Reference::Q1 => {
                    let full_trace = factorized_trace_variance(
                        &design,
                        &design.kernel_factor[target],
                        &design.ratio[target],
                        selected,
                    );
                    let full_linear = 4.0
                        * influence
                            .iter()
                            .zip(selected)
                            .map(|(left, right)| left * left * right)
                            .sum::<f64>();
                    let full_variance = full_linear - full_trace;
                    let lambda = design.leading_value[target];
                    let mode = &design.leading_mode[target];
                    let score = dot(mode, &outcome);
                    let remainder_influence = kernel_action(
                        &design,
                        &design.remainder_factor[target],
                        &design.remainder_ratio[target],
                        &outcome,
                    );
                    let trace = factorized_trace_variance(
                        &design,
                        &design.remainder_factor[target],
                        &design.remainder_ratio[target],
                        selected,
                    );
                    let leading_variance_correction = mode
                        .iter()
                        .zip(&proxy)
                        .map(|(mode, proxy)| mode * mode * proxy)
                        .sum::<f64>();
                    let direct_remainder_estimate = dot(&outcome, &remainder_influence);
                    let target_result = finish_q1_target(
                        point,
                        score,
                        leading_variance_correction,
                        direct_remainder_estimate,
                        1.0e-9,
                        lambda,
                        mode,
                        &remainder_influence,
                        selected,
                        trace,
                        0.0,
                        1.0e-8,
                    )
                    .and_then(require_computed_q1)
                    .and_then(|value| {
                        finish_q1_interval(
                            value,
                            INTERVAL_SEED ^ replication as u64,
                            target,
                            lambda,
                            LEVEL,
                            2_000,
                        )
                    });
                    match target_result {
                        Ok(result) if full_variance.is_finite() && full_variance > 0.0 => {
                            summaries[target].push(
                                point_error,
                                full_variance.sqrt(),
                                design.truth[target] >= result.confidence_lower
                                    && design.truth[target] <= result.confidence_upper,
                                floor,
                                boundary,
                            );
                        }
                        _ => summaries[target].attempts += 1,
                    }
                }
            }
        }
    }
    for (target, summary) in summaries.into_iter().enumerate() {
        let value = summary.values();
        println!(
            "1,{},{},{},{},{},{},{},{:?},{:?},{:?},{},{},{},{},{},{:.12e},{:.12e},{:.8},{:.12e},{:.12e},{:.12e},{:.8},{:.12e},{:.8},{:.8},{:.8}",
            match profile { Profile::Development => "development", Profile::Confirmation => "confirmation" },
            cell.name,
            cell.gate,
            cell.k,
            usize::from(cell.controls),
            usize::from(cell.dominant),
            match cell.model { StructuredVarianceModel::Common => "structured_common", StructuredVarianceModel::LeverageOnly => "structured_leverage" },
            cell.variance,
            cell.error,
            cell.reference,
            if cell.beta_zero { "zero" } else { "nonzero" },
            target_name(target),
            summary.attempts,
            summary.successes,
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
            design.leading_share[target], design.remainder_share[target], value[8], value[9],
        );
    }
}

fn make_design(k: usize, controls: bool, dominant: bool, beta_zero: bool) -> FactorizedDesign {
    let mut observations = Vec::new();
    for worker in 0..k {
        for firm in 0..k {
            let design_hash = splitmix64(((worker as u64) << 32) ^ firm as u64 ^ 20_260_904);
            let copies = if dominant {
                let half = k / 2;
                let same_cluster = (worker < half) == (firm < half);
                if same_cluster {
                    3 + usize::try_from(design_hash % 3).expect("copy count fits usize")
                } else if firm == (worker + half) % k {
                    1
                } else {
                    0
                }
            } else {
                2 + usize::try_from(design_hash % 4).expect("copy count fits usize")
            };
            for copy in 0..copies {
                observations.push((worker, firm, copy));
            }
        }
    }
    let rows = observations.len();
    let control_count = if controls { 2 } else { 0 };
    let parameters = k + k - 1 + control_count;
    let mut x = vec![0.0; rows * parameters];
    let mut fold_entity = vec![0; rows];
    for (row, &(worker, firm, copy)) in observations.iter().enumerate() {
        x[row * parameters + worker] = 1.0;
        if firm + 1 < k {
            x[row * parameters + k + firm] = 1.0;
        }
        if controls {
            let phase = ((worker + 1) * (firm + 2)) as f64 + 0.37 * (copy + 1) as f64;
            x[row * parameters + 2 * k - 1] = phase.sin();
            x[row * parameters + 2 * k] = (0.73 * phase + 0.11 * worker as f64).cos();
            fold_entity[row] = row as u64;
        } else {
            fold_entity[row] = (worker * k + firm) as u64;
        }
    }
    let sparse_x = (0..rows)
        .map(|row| {
            (0..parameters)
                .filter_map(|column| {
                    let value = x[row * parameters + column];
                    (value != 0.0).then_some((column, value))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let information = sparse_crossproduct(&sparse_x, parameters, None);
    let information_inverse = inverse(&information, parameters);
    let leverage = sparse_x
        .iter()
        .map(|row| sparse_quadratic(row, &information_inverse, parameters))
        .collect::<Vec<_>>();
    let maker_inverse = leverage
        .iter()
        .map(|value| (1.0 - value).recip())
        .collect::<Vec<_>>();
    let target = targets(k, parameters, &observations);
    let mut kernel_factor: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
    let mut ratio: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
    let mut remainder_factor: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
    let mut remainder_ratio: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
    let mut diagonal: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
    let mut leading_value = [0.0; 4];
    let mut leading_mode: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
    let mut leading_share = [0.0; 4];
    let mut remainder_share = [0.0; 4];
    let mut covariance_remainder_beta = vec![0.0; parameters];
    let cholesky = cholesky(&information, parameters);
    let cholesky_inverse = inverse_lower(&cholesky, parameters);
    for target_index in 0..4 {
        let inv_target = multiply(
            &information_inverse,
            parameters,
            parameters,
            &target[target_index],
            parameters,
        );
        let inv_target_inv = multiply(
            &inv_target,
            parameters,
            parameters,
            &information_inverse,
            parameters,
        );
        diagonal[target_index] = sparse_x
            .iter()
            .map(|row| sparse_quadratic(row, &inv_target_inv, parameters))
            .collect();
        ratio[target_index] = (0..rows)
            .map(|row| diagonal[target_index][row] * maker_inverse[row])
            .collect::<Vec<_>>();
        kernel_factor[target_index] =
            observation_kernel_factor(&inv_target_inv, &information_inverse, parameters);
        let whitened = multiply(
            &multiply(
                &cholesky_inverse,
                parameters,
                parameters,
                &target[target_index],
                parameters,
            ),
            parameters,
            parameters,
            &transpose(&cholesky_inverse, parameters, parameters),
            parameters,
        );
        let (values, vectors) = jacobi_eigen(whitened, parameters);
        let mut order = (0..parameters).collect::<Vec<_>>();
        order.sort_by(|left, right| values[*right].abs().total_cmp(&values[*left].abs()));
        let first = order[0];
        let second = order[1];
        leading_value[target_index] = values[first];
        let gamma = (0..parameters)
            .map(|row| vectors[row * parameters + first])
            .collect::<Vec<_>>();
        let mut coefficients = solve_upper_from_lower(&cholesky, parameters, &gamma);
        let mut mode = matrix_vector(&x, rows, parameters, &coefficients);
        let mode_norm = dot(&mode, &mode).sqrt();
        for value in &mut coefficients {
            *value /= mode_norm;
        }
        for value in &mut mode {
            *value /= mode_norm;
        }
        leading_mode[target_index] = mode;
        let mut remainder_a = inv_target_inv.clone();
        for row in 0..parameters {
            for column in 0..parameters {
                remainder_a[row * parameters + column] -=
                    values[first] * coefficients[row] * coefficients[column];
            }
        }
        remainder_ratio[target_index] = (0..rows)
            .map(|row| {
                (diagonal[target_index][row]
                    - values[first]
                        * leading_mode[target_index][row]
                        * leading_mode[target_index][row])
                    * maker_inverse[row]
            })
            .collect();
        remainder_factor[target_index] =
            observation_kernel_factor(&remainder_a, &information_inverse, parameters);
        if dominant && target_index == 2 {
            let retained = parameters.min(8);
            let amplitude = 4.0 * (rows as f64 / (retained - 1) as f64).sqrt();
            for (position, &eigen_index) in order.iter().take(retained).enumerate().skip(1) {
                if values[eigen_index].abs() <= 1.0e-10 {
                    continue;
                }
                let gamma = (0..parameters)
                    .map(|row| vectors[row * parameters + eigen_index])
                    .collect::<Vec<_>>();
                let coefficients = solve_upper_from_lower(&cholesky, parameters, &gamma);
                let sign = if position % 2 == 0 { -1.0 } else { 1.0 };
                for (beta, coefficient) in covariance_remainder_beta.iter_mut().zip(coefficients) {
                    *beta += sign * amplitude * coefficient;
                }
            }
        }
        let trace_square = values.iter().map(|value| value * value).sum::<f64>();
        leading_share[target_index] = values[first] * values[first] / trace_square;
        let remainder = (trace_square - values[first] * values[first]).max(0.0);
        remainder_share[target_index] = if remainder > 0.0 {
            values[second] * values[second] / remainder
        } else {
            0.0
        };
    }
    let target_diagonal = [
        diagonal[0].clone(),
        diagonal[1].clone(),
        diagonal[2].clone(),
    ];
    let variance_ranks = [
        midranks(&leverage),
        midranks(&target_diagonal[0]),
        midranks(&target_diagonal[1]),
        midranks(&target_diagonal[2]),
    ];
    let beta = if beta_zero {
        vec![0.0; parameters]
    } else if dominant {
        covariance_remainder_beta
    } else {
        let mut value = (0..parameters)
            .map(|index| ((index + 1) as f64 * 0.73).sin())
            .collect::<Vec<_>>();
        if controls {
            value[parameters - 2] = 0.2;
            value[parameters - 1] = -0.15;
        }
        value
    };
    let mean = matrix_vector(&x, rows, parameters, &beta);
    let omitted_raw = mean
        .iter()
        .zip(&fold_entity)
        .map(|(value, entity)| {
            let jitter =
                (splitmix64(*entity ^ 20_260_903) >> 11) as f64 / 9_007_199_254_740_992.0 - 0.5;
            value + 0.1 * jitter
        })
        .collect::<Vec<_>>();
    let hidden_driver = omitted_driver(&variance_ranks, &omitted_raw);
    let truth = core::array::from_fn(|index| quadratic(&beta, &target[index], parameters));
    FactorizedDesign {
        rows,
        parameters,
        x,
        sparse_x,
        information_inverse,
        leverage,
        maker_inverse,
        kernel_factor,
        ratio,
        remainder_factor,
        remainder_ratio,
        target_diagonal,
        fold_entity,
        variance_ranks,
        hidden_driver,
        beta,
        truth,
        leading_value,
        leading_mode,
        leading_share,
        remainder_share,
    }
}

fn targets(k: usize, parameters: usize, observations: &[(usize, usize, usize)]) -> [Vec<f64>; 4] {
    let mass = (observations.len() as f64).recip();
    let mut worker_mean = vec![0.0; parameters];
    let mut firm_mean = vec![0.0; parameters];
    for &(worker, firm, _) in observations {
        worker_mean[worker] += mass;
        if firm + 1 < k {
            firm_mean[k + firm] += mass;
        }
    }
    let mut output: [Vec<f64>; 4] = core::array::from_fn(|_| vec![0.0; parameters * parameters]);
    for &(worker, firm, _) in observations {
        let mut worker_loading = worker_mean.iter().map(|value| -value).collect::<Vec<_>>();
        let mut firm_loading = firm_mean.iter().map(|value| -value).collect::<Vec<_>>();
        worker_loading[worker] += 1.0;
        if firm + 1 < k {
            firm_loading[k + firm] += 1.0;
        }
        for row in 0..parameters {
            for column in 0..parameters {
                let index = row * parameters + column;
                output[0][index] += mass * worker_loading[row] * worker_loading[column];
                output[1][index] += mass * firm_loading[row] * firm_loading[column];
                output[2][index] += 0.5
                    * mass
                    * (worker_loading[row] * firm_loading[column]
                        + firm_loading[row] * worker_loading[column]);
            }
        }
    }
    for index in 0..parameters * parameters {
        output[3][index] = output[0][index] + output[1][index] + 2.0 * output[2][index];
    }
    output
}

fn make_variance(design: &FactorizedDesign, dgp: VarianceDgp) -> Vec<f64> {
    let h = &design.variance_ranks[0];
    let worker = &design.variance_ranks[1];
    let firm = &design.variance_ranks[2];
    let covariance = &design.variance_ranks[3];
    let mut output = (0..design.rows)
        .map(|row| {
            let common = 1.0
                + 0.25 * h[row]
                + 0.15 * worker[row] * worker[row]
                + 0.10 * firm[row] * covariance[row];
            match dgp {
                VarianceDgp::Homoskedastic => 1.0,
                VarianceDgp::Common => common,
                VarianceDgp::Leverage => 1.0 + 0.35 * h[row] + 0.20 * h[row] * h[row],
                VarianceDgp::MildFunctional => (0.25 * h[row]
                    + 0.15 * worker[row] * worker[row]
                    + 0.10 * firm[row] * covariance[row])
                    .exp(),
                VarianceDgp::MildOmitted => {
                    common * (1.5_f64.ln() * design.hidden_driver[row]).exp()
                }
                VarianceDgp::SevereOmitted => {
                    common * (16.0_f64.ln() * design.hidden_driver[row]).exp()
                }
            }
        })
        .collect::<Vec<_>>();
    let mean = output.iter().sum::<f64>() / output.len() as f64;
    for value in &mut output {
        *value /= mean;
        assert!(value.is_finite() && *value > 0.0);
    }
    output
}

fn omitted_driver(ranks: &[Vec<f64>; 4], raw: &[f64]) -> Vec<f64> {
    let rows = raw.len();
    let basis = (0..rows)
        .flat_map(|row| basis_row(ranks, row))
        .collect::<Vec<_>>();
    let bt = transpose(&basis, rows, 15);
    let mut gram = multiply(&bt, 15, rows, &basis, 15);
    let ridge = 1.0e-10 * rows as f64;
    for diagonal in 0..15 {
        gram[diagonal * 15 + diagonal] += ridge;
    }
    let coefficients = matrix_vector(
        &inverse(&gram, 15),
        15,
        15,
        &matrix_vector(&bt, 15, rows, raw),
    );
    let fitted = matrix_vector(&basis, rows, 15, &coefficients);
    let mut residual = raw
        .iter()
        .zip(fitted)
        .map(|(left, right)| left - right)
        .collect::<Vec<_>>();
    let scale = (residual.iter().map(|value| value * value).sum::<f64>() / rows as f64).sqrt();
    for value in &mut residual {
        *value = (*value / scale).clamp(-2.0, 2.0);
    }
    residual
}

fn basis_row(ranks: &[Vec<f64>; 4], row: usize) -> [f64; 15] {
    let x = [ranks[0][row], ranks[1][row], ranks[2][row], ranks[3][row]];
    let mut output = [0.0; 15];
    output[0] = 1.0;
    output[1..5].copy_from_slice(&x);
    for index in 0..4 {
        output[5 + index] = x[index] * x[index];
    }
    let mut term = 9;
    for left in 0..4 {
        for right in left + 1..4 {
            output[term] = x[left] * x[right];
            term += 1;
        }
    }
    output
}

fn sparse_crossproduct(
    rows: &[Vec<(usize, f64)>],
    dimension: usize,
    weights: Option<&[f64]>,
) -> Vec<f64> {
    if let Some(weights) = weights {
        assert_eq!(weights.len(), rows.len());
    }
    let mut output = vec![0.0; dimension * dimension];
    for (row_index, row) in rows.iter().enumerate() {
        let weight = weights.map_or(1.0, |value| value[row_index]);
        for &(left, left_value) in row {
            for &(right, right_value) in row {
                output[left * dimension + right] += weight * left_value * right_value;
            }
        }
    }
    output
}

fn sparse_quadratic(row: &[(usize, f64)], matrix: &[f64], dimension: usize) -> f64 {
    let mut output = 0.0;
    for &(left, left_value) in row {
        for &(right, right_value) in row {
            output += left_value * matrix[left * dimension + right] * right_value;
        }
    }
    output
}

fn observation_kernel_factor(
    target_inverse: &[f64],
    information_inverse: &[f64],
    parameters: usize,
) -> Vec<f64> {
    let dimension = 2 * parameters;
    let mut output = vec![0.0; dimension * dimension];
    for row in 0..parameters {
        for column in 0..parameters {
            output[row * dimension + column] = target_inverse[row * parameters + column];
            let half_inverse = 0.5 * information_inverse[row * parameters + column];
            output[row * dimension + parameters + column] = half_inverse;
            output[(parameters + row) * dimension + column] = half_inverse;
        }
    }
    output
}

fn maker_action(design: &FactorizedDesign, vector: &[f64]) -> Vec<f64> {
    assert_eq!(vector.len(), design.rows);
    let mut transpose_product = vec![0.0; design.parameters];
    for (value, row) in vector.iter().zip(&design.sparse_x) {
        for &(column, loading) in row {
            transpose_product[column] += loading * value;
        }
    }
    let coefficients = matrix_vector(
        &design.information_inverse,
        design.parameters,
        design.parameters,
        &transpose_product,
    );
    let projection = matrix_vector(&design.x, design.rows, design.parameters, &coefficients);
    vector
        .iter()
        .zip(projection)
        .map(|(value, fitted)| value - fitted)
        .collect()
}

fn kernel_action(
    design: &FactorizedDesign,
    factor: &[f64],
    ratio: &[f64],
    vector: &[f64],
) -> Vec<f64> {
    assert_eq!(factor.len(), 4 * design.parameters * design.parameters);
    assert_eq!(ratio.len(), design.rows);
    assert_eq!(vector.len(), design.rows);
    let mut transpose_product = vec![0.0; 2 * design.parameters];
    for ((value, row), ratio) in vector.iter().zip(&design.sparse_x).zip(ratio) {
        for &(column, loading) in row {
            transpose_product[column] += loading * value;
            transpose_product[design.parameters + column] += loading * ratio * value;
        }
    }
    let coefficients = matrix_vector(
        factor,
        2 * design.parameters,
        2 * design.parameters,
        &transpose_product,
    );
    let first = matrix_vector(
        &design.x,
        design.rows,
        design.parameters,
        &coefficients[..design.parameters],
    );
    let second = matrix_vector(
        &design.x,
        design.rows,
        design.parameters,
        &coefficients[design.parameters..],
    );
    (0..design.rows)
        .map(|row| first[row] + ratio[row] * second[row] - ratio[row] * vector[row])
        .collect()
}

fn factorized_trace_variance(
    design: &FactorizedDesign,
    factor: &[f64],
    ratio: &[f64],
    variance: &[f64],
) -> f64 {
    assert_eq!(ratio.len(), design.rows);
    assert_eq!(variance.len(), design.rows);
    let dimension = 2 * design.parameters;
    assert_eq!(factor.len(), dimension * dimension);
    let mut weighted_gram = vec![0.0; dimension * dimension];
    let mut diagonal_gram = vec![0.0; dimension * dimension];
    let mut diagonal_square = 0.0;
    for row_index in 0..design.rows {
        let row = &design.sparse_x[row_index];
        let row_ratio = ratio[row_index];
        let row_variance = variance[row_index];
        let diagonal = -row_variance * row_ratio;
        diagonal_square += diagonal * diagonal;
        let diagonal_weight = -row_variance * row_variance * row_ratio;
        for &(left, left_value) in row {
            for &(right, right_value) in row {
                let product = left_value * right_value;
                let left_scaled = [1.0, row_ratio];
                let right_scaled = [1.0, row_ratio];
                for left_block in 0..2 {
                    for right_block in 0..2 {
                        let index = (left_block * design.parameters + left) * dimension
                            + right_block * design.parameters
                            + right;
                        let scale = left_scaled[left_block] * right_scaled[right_block] * product;
                        weighted_gram[index] += row_variance * scale;
                        diagonal_gram[index] += diagonal_weight * scale;
                    }
                }
            }
        }
    }
    let factor_gram = multiply(factor, dimension, dimension, &weighted_gram, dimension);
    let mut cross = 0.0;
    let mut low_rank_square = 0.0;
    for row in 0..dimension {
        for column in 0..dimension {
            cross += factor[row * dimension + column] * diagonal_gram[column * dimension + row];
            low_rank_square +=
                factor_gram[row * dimension + column] * factor_gram[column * dimension + row];
        }
    }
    let trace_square = diagonal_square + 2.0 * cross + low_rank_square;
    let scale = diagonal_square.abs() + (2.0 * cross).abs() + low_rank_square.abs();
    assert!(trace_square >= -1.0e-10 * scale.max(1.0));
    2.0 * trace_square.max(0.0)
}

fn midranks(values: &[f64]) -> Vec<f64> {
    let mut order = (0..values.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        values[*left]
            .total_cmp(&values[*right])
            .then(left.cmp(right))
    });
    let mut output = vec![0.0; values.len()];
    let mut begin = 0;
    while begin < order.len() {
        let mut end = begin + 1;
        while end < order.len() && values[order[begin]].total_cmp(&values[order[end]]).is_eq() {
            end += 1;
        }
        let midrank = 0.5 * ((begin + 1 + end) as f64);
        let value = 2.0 * ((midrank - 0.5) / values.len() as f64) - 1.0;
        for &row in &order[begin..end] {
            output[row] = value;
        }
        begin = end;
    }
    output
}

#[cfg(test)]
fn trace_variance(kernel: &[f64], variance: &[f64], rows: usize) -> f64 {
    let mut output = 0.0;
    for row in 0..rows {
        for column in 0..rows {
            let value = kernel[row * rows + column];
            output += 2.0 * variance[row] * value * variance[column] * value;
        }
    }
    output
}

fn quadratic(vector: &[f64], matrix: &[f64], dimension: usize) -> f64 {
    dot(vector, &matrix_vector(matrix, dimension, dimension, vector))
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn matrix_vector(matrix: &[f64], rows: usize, columns: usize, vector: &[f64]) -> Vec<f64> {
    assert_eq!(matrix.len(), rows * columns);
    assert_eq!(vector.len(), columns);
    (0..rows)
        .map(|row| {
            (0..columns)
                .map(|column| matrix[row * columns + column] * vector[column])
                .sum()
        })
        .collect()
}

fn multiply(
    left: &[f64],
    left_rows: usize,
    shared: usize,
    right: &[f64],
    right_columns: usize,
) -> Vec<f64> {
    assert_eq!(left.len(), left_rows * shared);
    assert_eq!(right.len(), shared * right_columns);
    let mut output = vec![0.0; left_rows * right_columns];
    for row in 0..left_rows {
        for inner in 0..shared {
            let scale = left[row * shared + inner];
            for column in 0..right_columns {
                output[row * right_columns + column] +=
                    scale * right[inner * right_columns + column];
            }
        }
    }
    output
}

fn transpose(value: &[f64], rows: usize, columns: usize) -> Vec<f64> {
    let mut output = vec![0.0; value.len()];
    for row in 0..rows {
        for column in 0..columns {
            output[column * rows + row] = value[row * columns + column];
        }
    }
    output
}

fn inverse(value: &[f64], dimension: usize) -> Vec<f64> {
    let mut augmented = vec![0.0; dimension * 2 * dimension];
    for row in 0..dimension {
        for column in 0..dimension {
            augmented[row * 2 * dimension + column] = value[row * dimension + column];
        }
        augmented[row * 2 * dimension + dimension + row] = 1.0;
    }
    for pivot in 0..dimension {
        let selected = (pivot..dimension)
            .max_by(|left, right| {
                augmented[*left * 2 * dimension + pivot]
                    .abs()
                    .total_cmp(&augmented[*right * 2 * dimension + pivot].abs())
            })
            .expect("nonempty pivot range");
        assert!(augmented[selected * 2 * dimension + pivot].abs() > 1.0e-12);
        if selected != pivot {
            for column in 0..2 * dimension {
                augmented.swap(
                    pivot * 2 * dimension + column,
                    selected * 2 * dimension + column,
                );
            }
        }
        let scale = augmented[pivot * 2 * dimension + pivot];
        for column in 0..2 * dimension {
            augmented[pivot * 2 * dimension + column] /= scale;
        }
        for row in 0..dimension {
            if row == pivot {
                continue;
            }
            let scale = augmented[row * 2 * dimension + pivot];
            for column in 0..2 * dimension {
                augmented[row * 2 * dimension + column] -=
                    scale * augmented[pivot * 2 * dimension + column];
            }
        }
    }
    let mut output = vec![0.0; dimension * dimension];
    for row in 0..dimension {
        output[row * dimension..(row + 1) * dimension].copy_from_slice(
            &augmented[row * 2 * dimension + dimension..(row + 1) * 2 * dimension],
        );
    }
    output
}

fn cholesky(value: &[f64], dimension: usize) -> Vec<f64> {
    let mut output = vec![0.0; dimension * dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = value[row * dimension + column];
            for inner in 0..column {
                value -= output[row * dimension + inner] * output[column * dimension + inner];
            }
            output[row * dimension + column] = if row == column {
                assert!(value > 1.0e-12);
                value.sqrt()
            } else {
                value / output[column * dimension + column]
            };
        }
    }
    output
}

fn inverse_lower(value: &[f64], dimension: usize) -> Vec<f64> {
    let mut output = vec![0.0; dimension * dimension];
    for column in 0..dimension {
        for row in 0..dimension {
            let rhs = f64::from(row == column);
            let prior = (0..row)
                .map(|inner| value[row * dimension + inner] * output[inner * dimension + column])
                .sum::<f64>();
            output[row * dimension + column] = (rhs - prior) / value[row * dimension + row];
        }
    }
    output
}

fn solve_upper_from_lower(lower: &[f64], dimension: usize, rhs: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; dimension];
    for offset in 0..dimension {
        let row = dimension - 1 - offset;
        let prior = (row + 1..dimension)
            .map(|column| lower[column * dimension + row] * output[column])
            .sum::<f64>();
        output[row] = (rhs[row] - prior) / lower[row * dimension + row];
    }
    output
}

fn jacobi_eigen(mut matrix: Vec<f64>, dimension: usize) -> (Vec<f64>, Vec<f64>) {
    let mut vectors = vec![0.0; dimension * dimension];
    for index in 0..dimension {
        vectors[index * dimension + index] = 1.0;
    }
    for _ in 0..100 * dimension * dimension {
        let mut row = 0;
        let mut column = 1;
        let mut maximum = 0.0;
        for left in 0..dimension {
            for right in left + 1..dimension {
                let value = matrix[left * dimension + right].abs();
                if value > maximum {
                    maximum = value;
                    row = left;
                    column = right;
                }
            }
        }
        if maximum <= 1.0e-12 {
            break;
        }
        let diagonal_difference =
            matrix[column * dimension + column] - matrix[row * dimension + row];
        let angle = 0.5 * (2.0 * matrix[row * dimension + column]).atan2(diagonal_difference);
        let cosine = angle.cos();
        let sine = angle.sin();
        for index in 0..dimension {
            let left = matrix[row * dimension + index];
            let right = matrix[column * dimension + index];
            matrix[row * dimension + index] = cosine * left - sine * right;
            matrix[column * dimension + index] = sine * left + cosine * right;
        }
        for index in 0..dimension {
            let left = matrix[index * dimension + row];
            let right = matrix[index * dimension + column];
            matrix[index * dimension + row] = cosine * left - sine * right;
            matrix[index * dimension + column] = sine * left + cosine * right;
            let vector_left = vectors[index * dimension + row];
            let vector_right = vectors[index * dimension + column];
            vectors[index * dimension + row] = cosine * vector_left - sine * vector_right;
            vectors[index * dimension + column] = sine * vector_left + cosine * vector_right;
        }
    }
    let values = (0..dimension)
        .map(|index| matrix[index * dimension + index])
        .collect();
    (values, vectors)
}

fn target_name(target: usize) -> &'static str {
    ["worker", "firm", "covariance", "total"][target]
}

fn hash_label(value: &str) -> u64 {
    value.bytes().fold(0xcbf2_9ce4_8422_2325, |state, byte| {
        (state ^ u64::from(byte)).wrapping_mul(0x1000_0000_01b3)
    })
}

fn semantic_seed(master: u64, cell: &str, k: usize, replication: usize) -> u64 {
    splitmix64(
        master
            ^ splitmix64(hash_label(cell))
            ^ splitmix64(k as u64)
            ^ splitmix64(replication as u64),
    )
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Clone, Copy, Debug)]
struct IndependentRng {
    state: u64,
}

impl IndependentRng {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn uniform(&mut self) -> f64 {
        self.state = splitmix64(self.state);
        (((self.state >> 11) as f64) + 0.5) / 9_007_199_254_740_992.0
    }

    fn normal(&mut self) -> f64 {
        (-2.0 * self.uniform().ln()).sqrt() * (core::f64::consts::TAU * self.uniform()).cos()
    }

    fn standardized_error(&mut self, dgp: ErrorDgp) -> f64 {
        match dgp {
            ErrorDgp::Gaussian => self.normal(),
            ErrorDgp::StudentT8 => {
                let chi_square = (0..8).map(|_| self.normal().powi(2)).sum::<f64>();
                (6.0 / 8.0_f64).sqrt() * self.normal() / (chi_square / 8.0).sqrt()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v5_confirmation_seed_keys_are_deterministic() {
        assert_eq!(
            semantic_seed(V5_CONFIRMATION_SEED, "diffuse_common", 12, 0),
            6_180_164_651_401_935_216
        );
        assert_eq!(
            semantic_seed(V5_CONFIRMATION_SEED, "dominant_common_t8", 16, 2_499),
            2_983_714_797_850_544_688
        );
    }

    fn simpson_q1_cdf(distance: f64, curvature: f64) -> f64 {
        if curvature <= 1.0e-10 {
            return 2.0 * reference_normal_cdf(distance) - 1.0;
        }
        const PANELS: usize = 131_072;
        let inverse = curvature.recip();
        let width = distance / PANELS as f64;
        let integrand = |x: f64| {
            let y_squared = ((distance - x) * (2.0 * inverse + distance + x)).max(0.0);
            (2.0 / core::f64::consts::PI).sqrt()
                * (-0.5 * x * x).exp()
                * (2.0 * reference_normal_cdf(y_squared.sqrt()) - 1.0).max(0.0)
        };
        let mut sum = integrand(0.0) + integrand(distance);
        for panel in 1..PANELS {
            sum += if panel % 2 == 0 { 2.0 } else { 4.0 } * integrand(panel as f64 * width);
        }
        sum * width / 3.0
    }

    fn dense_maker(design: &FactorizedDesign) -> Vec<f64> {
        let transpose = transpose(&design.x, design.rows, design.parameters);
        let projection = multiply(
            &multiply(
                &design.x,
                design.rows,
                design.parameters,
                &design.information_inverse,
                design.parameters,
            ),
            design.rows,
            design.parameters,
            &transpose,
            design.rows,
        );
        (0..design.rows * design.rows)
            .map(|index| {
                let row = index / design.rows;
                let column = index % design.rows;
                f64::from(row == column) - projection[index]
            })
            .collect()
    }

    fn dense_kernel(
        design: &FactorizedDesign,
        factor: &[f64],
        ratio: &[f64],
        maker: &[f64],
    ) -> Vec<f64> {
        let parameters = design.parameters;
        let dimension = 2 * parameters;
        let target_inverse = (0..parameters * parameters)
            .map(|index| {
                let row = index / parameters;
                let column = index % parameters;
                factor[row * dimension + column]
            })
            .collect::<Vec<_>>();
        let transpose = transpose(&design.x, design.rows, design.parameters);
        let target = multiply(
            &multiply(
                &design.x,
                design.rows,
                parameters,
                &target_inverse,
                parameters,
            ),
            design.rows,
            parameters,
            &transpose,
            design.rows,
        );
        (0..design.rows * design.rows)
            .map(|index| {
                let row = index / design.rows;
                let column = index % design.rows;
                target[index] - 0.5 * (ratio[row] * maker[index] + maker[index] * ratio[column])
            })
            .collect()
    }

    fn assert_close(left: f64, right: f64) {
        let scale = left.abs().max(right.abs()).max(1.0);
        assert!(
            (left - right).abs() <= 1.0e-9 * scale,
            "left={left:.16e} right={right:.16e} difference={:.16e}",
            left - right
        );
    }

    #[test]
    fn factorized_maker_kernels_traces_and_q1_interval_match_dense_oracle() {
        for (dominant, controls) in [(false, false), (true, false), (true, true)] {
            let design = make_design(8, controls, dominant, false);
            let maker = dense_maker(&design);
            let variance = make_variance(&design, VarianceDgp::Common);
            let outcome = (0..design.rows)
                .map(|row| (0.37 * (row + 1) as f64).sin() + 0.2 * (row % 3) as f64)
                .collect::<Vec<_>>();
            let dense_residual = matrix_vector(&maker, design.rows, design.rows, &outcome);
            let factorized_residual = maker_action(&design, &outcome);
            for (&left, &right) in dense_residual.iter().zip(&factorized_residual) {
                assert_close(left, right);
            }
            for target in 0..4 {
                let dense = dense_kernel(
                    &design,
                    &design.kernel_factor[target],
                    &design.ratio[target],
                    &maker,
                );
                let dense_action = matrix_vector(&dense, design.rows, design.rows, &outcome);
                let factorized_action = kernel_action(
                    &design,
                    &design.kernel_factor[target],
                    &design.ratio[target],
                    &outcome,
                );
                for (&left, &right) in dense_action.iter().zip(&factorized_action) {
                    assert_close(left, right);
                }
                assert_close(
                    trace_variance(&dense, &variance, design.rows),
                    factorized_trace_variance(
                        &design,
                        &design.kernel_factor[target],
                        &design.ratio[target],
                        &variance,
                    ),
                );

                let dense_remainder = dense_kernel(
                    &design,
                    &design.remainder_factor[target],
                    &design.remainder_ratio[target],
                    &maker,
                );
                let dense_remainder_action =
                    matrix_vector(&dense_remainder, design.rows, design.rows, &outcome);
                let factorized_remainder_action = kernel_action(
                    &design,
                    &design.remainder_factor[target],
                    &design.remainder_ratio[target],
                    &outcome,
                );
                for (&left, &right) in dense_remainder_action
                    .iter()
                    .zip(&factorized_remainder_action)
                {
                    assert_close(left, right);
                }
                let dense_trace = trace_variance(&dense_remainder, &variance, design.rows);
                let factorized_trace = factorized_trace_variance(
                    &design,
                    &design.remainder_factor[target],
                    &design.remainder_ratio[target],
                    &variance,
                );
                assert_close(dense_trace, factorized_trace);

                let point = dot(&outcome, &dense_action);
                let score = dot(&design.leading_mode[target], &outcome);
                let residual = maker_action(&design, &outcome);
                let proxy = outcome
                    .iter()
                    .zip(&residual)
                    .zip(&design.maker_inverse)
                    .map(|((outcome, residual), maker_inverse)| outcome * residual * maker_inverse)
                    .collect::<Vec<_>>();
                let leading_variance_correction = design.leading_mode[target]
                    .iter()
                    .zip(&proxy)
                    .map(|(mode, proxy)| mode * mode * proxy)
                    .sum::<f64>();
                let dense_remainder_estimate = dot(&outcome, &dense_remainder_action);
                let factorized_remainder_estimate = dot(&outcome, &factorized_remainder_action);
                let dense_result = finish_q1_target(
                    point,
                    score,
                    leading_variance_correction,
                    dense_remainder_estimate,
                    1.0e-9,
                    design.leading_value[target],
                    &design.leading_mode[target],
                    &dense_remainder_action,
                    &variance,
                    dense_trace,
                    0.0,
                    1.0e-8,
                )
                .and_then(require_computed_q1)
                .and_then(|value| {
                    finish_q1_interval(value, 91, target, design.leading_value[target], 0.95, 2_000)
                });
                let factorized_result = finish_q1_target(
                    point,
                    score,
                    leading_variance_correction,
                    factorized_remainder_estimate,
                    1.0e-9,
                    design.leading_value[target],
                    &design.leading_mode[target],
                    &factorized_remainder_action,
                    &variance,
                    factorized_trace,
                    0.0,
                    1.0e-8,
                )
                .and_then(require_computed_q1)
                .and_then(|value| {
                    finish_q1_interval(value, 91, target, design.leading_value[target], 0.95, 2_000)
                });
                match (dense_result, factorized_result) {
                    (Ok(left), Ok(right)) => {
                        assert_close(left.remainder_variance, right.remainder_variance);
                        assert_close(
                            left.leading_remainder_covariance,
                            right.leading_remainder_covariance,
                        );
                        assert_close(left.confidence_lower, right.confidence_lower);
                        assert_close(left.confidence_upper, right.confidence_upper);
                    }
                    (Err(left), Err(right)) => assert_eq!(left.code, right.code),
                    (left, right) => {
                        panic!("dense/factorized q=1 status mismatch: {left:?} {right:?}")
                    }
                }
            }
        }
    }

    #[test]
    fn q1_reference_quadrature_matches_high_resolution_simpson_oracle() {
        for curvature in [0.0, 0.05, 0.5, 5.0, 25.0] {
            let critical = reference_q1_critical(curvature, 0.95);
            let cdf = simpson_q1_cdf(critical, curvature);
            assert!(
                (cdf - 0.95).abs() < 2.0e-6,
                "curvature={curvature}, critical={critical}, Simpson CDF={cdf}"
            );
        }
    }

    #[test]
    fn production_counter_critical_values_match_independent_reference_probability() {
        // Predeclared probability-scale allowance comfortably exceeds the
        // Monte Carlo error of 100,000 draws, including all twenty cells.
        for curvature in [0.0, 0.05, 0.5, 5.0, 25.0] {
            for target in 0..4 {
                let critical = vckss_core::component_inference::q1_critical_value(
                    0x482a_13cd_995e_07b1,
                    target,
                    curvature,
                    0.95,
                    100_000,
                )
                .unwrap();
                let probability = simpson_q1_cdf(critical, curvature);
                assert!(
                    (probability - 0.95).abs() < 0.005,
                    "curvature={curvature}, target={target}, probability={probability}"
                );
            }
        }
    }

    #[test]
    fn v4_population_covariance_and_required_radius_match_dense_oracles() {
        let design = make_design(8, false, true, false);
        let variance = make_variance(&design, VarianceDgp::Common);
        let maker = dense_maker(&design);
        let (population, full_variance) = v4_population_covariance(&design, &variance);
        let dense_full = dense_kernel(&design, &design.kernel_factor[1], &design.ratio[1], &maker);
        let dense_remainder = dense_kernel(
            &design,
            &design.remainder_factor[1],
            &design.remainder_ratio[1],
            &maker,
        );
        for row in 0..design.rows {
            assert!(
                dense_full[row * design.rows + row].abs() <= 2.0e-12
                    && dense_remainder[row * design.rows + row].abs() <= 2.0e-12,
                "leave-out population covariance requires zero-diagonal kernels"
            );
        }
        let mean = matrix_vector(&design.x, design.rows, design.parameters, &design.beta);
        let dense_full_mean = matrix_vector(&dense_full, design.rows, design.rows, &mean);
        let dense_remainder_mean = matrix_vector(&dense_remainder, design.rows, design.rows, &mean);
        let expected_full = trace_variance(&dense_full, &variance, design.rows)
            + 4.0
                * dense_full_mean
                    .iter()
                    .zip(&variance)
                    .map(|(influence, variance)| influence * influence * variance)
                    .sum::<f64>();
        let expected_remainder = trace_variance(&dense_remainder, &variance, design.rows)
            + 4.0
                * dense_remainder_mean
                    .iter()
                    .zip(&variance)
                    .map(|(influence, variance)| influence * influence * variance)
                    .sum::<f64>();
        let expected_cross = 2.0
            * design.leading_mode[1]
                .iter()
                .zip(&variance)
                .zip(&dense_remainder_mean)
                .map(|((mode, variance), influence)| mode * variance * influence)
                .sum::<f64>();
        assert_close(full_variance, expected_full);
        assert_close(population.remainder, expected_remainder);
        assert_close(
            population.leading,
            design.leading_mode[1]
                .iter()
                .zip(&variance)
                .map(|(mode, variance)| mode * mode * variance)
                .sum(),
        );
        assert_close(population.cross, expected_cross);

        for (center, covariance, eigenvalue, truth) in [
            (
                [0.4, -0.7],
                V4Covariance {
                    leading: 1.2,
                    cross: 0.25,
                    remainder: 0.9,
                },
                0.35,
                0.2,
            ),
            (
                [-1.1, 0.3],
                V4Covariance {
                    leading: 0.7,
                    cross: -0.4,
                    remainder: 1.8,
                },
                -0.8,
                -0.5,
            ),
            (
                [0.05, -0.02],
                V4Covariance {
                    leading: 0.2,
                    cross: 0.01,
                    remainder: 0.4,
                },
                2.2,
                0.0,
            ),
        ] {
            let observed = v4_required_radius(center, covariance, eigenvalue, truth);
            let leading = covariance.leading;
            let slope = covariance.cross / leading;
            let conditional = covariance.remainder - covariance.cross * covariance.cross / leading;
            let objective = |candidate: f64| {
                let first = center[0] - candidate;
                let second =
                    center[1] - (truth - eigenvalue * candidate * candidate) - slope * first;
                first * first / leading + second * second / conditional
            };
            const GRID: usize = 1_000_000;
            let bound = 20.0;
            let mut oracle = f64::INFINITY;
            for index in 0..=GRID {
                let candidate = -bound + 2.0 * bound * index as f64 / GRID as f64;
                oracle = oracle.min(objective(candidate));
            }
            assert!(
                (observed - oracle.sqrt()).abs() < 5.0e-5,
                "required radius {observed} versus dense-grid oracle {}",
                oracle.sqrt()
            );
            for radius in [0.9 * observed, 1.1 * observed.max(1.0e-8)] {
                let interval = q1_am_interval(center, covariance.array(), radius, eigenvalue)
                    .expect("dense ellipse image");
                assert_eq!(
                    truth >= interval[0] && truth <= interval[1],
                    radius >= observed
                );
            }
        }
    }

    #[test]
    fn v4_paired_error_draws_are_semantically_deterministic() {
        let seed = semantic_seed(V4_EVALUATION_SEED, "dominant_common_v4", 64, 91);
        let first = (0..32)
            .map(|row| v4_paired_errors(seed, row))
            .collect::<Vec<_>>();
        let repeated = (0..32)
            .map(|row| v4_paired_errors(seed, row))
            .collect::<Vec<_>>();
        for (left, right) in first.iter().zip(repeated) {
            assert_eq!(left.0.to_bits(), right.0.to_bits());
            assert_eq!(left.1.to_bits(), right.1.to_bits());
        }
        assert_ne!(
            semantic_seed(V4_CALIBRATION_SEED, "dominant_common_v4", 64, 91),
            seed
        );
        assert_ne!(
            semantic_seed(V4_REFERENCE_EVALUATION_SEED, "q1_reference_v4", 64, 91),
            seed
        );
    }
}
