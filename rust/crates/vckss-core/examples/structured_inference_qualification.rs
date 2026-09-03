// SPDX-License-Identifier: GPL-3.0-only

//! Deterministic fitted-variance qualification for structured component inference.
//!
//! The executable deliberately constructs its dense FEVC matrices independently
//! of the generic-JLA component attachment.  It uses the production structured
//! variance fitter, then evaluates the observation-deletion quadratic form and
//! its exact conditional covariance without randomized covariance probes.

#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::env;

use vckss_core::component_inference::{finish_q1_interval, finish_q1_target};
use vckss_core::interrupt::NeverInterrupt;
use vckss_core::structured_variance::{
    fit_structured_variance_with_interrupt, StructuredVarianceModel, StructuredVarianceOptions,
};

const LEVEL: f64 = 0.95;
const DEVELOPMENT_SEED: u64 = 20_261_001;
const CONFIRMATION_SEED: u64 = 20_262_001;
const INTERVAL_SEED: u64 = 8_675_309;

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
struct DenseDesign {
    rows: usize,
    parameters: usize,
    x: Vec<f64>,
    maker: Vec<f64>,
    leverage: Vec<f64>,
    maker_inverse: Vec<f64>,
    kernel: [Vec<f64>; 4],
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
    let profile = parse_profile();
    println!(
        "schema,profile,cell,gate,k,controls,dominant,variance_model,variance_dgp,error_dgp,reference,beta,target,replications,successes,bias,bias_mcse,coverage,coverage_mcse,empirical_sd,mean_se,se_ratio,mean_estimated_variance,leading_share,remainder_share,mean_floor_share,mean_boundary_share"
    );
    for cell in cells(profile) {
        run_cell(profile, cell);
    }
}

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
        let residual = matrix_vector(&design.maker, design.rows, design.rows, &outcome);
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
            let point = quadratic(&outcome, &design.kernel[target], design.rows);
            let point_error = point - design.truth[target];
            let influence =
                matrix_vector(&design.kernel[target], design.rows, design.rows, &outcome);
            match cell.reference {
                Reference::Q0 => {
                    let trace = trace_variance(&design.kernel[target], selected, design.rows);
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
                    let full_trace = trace_variance(&design.kernel[target], selected, design.rows);
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
                    let mode_ratio = mode
                        .iter()
                        .zip(&design.maker_inverse)
                        .map(|(mode, maker_inverse)| lambda * mode * mode * maker_inverse)
                        .collect::<Vec<_>>();
                    let remainder_kernel = design.kernel[target]
                        .iter()
                        .enumerate()
                        .map(|(index, value)| {
                            let row = index / design.rows;
                            let column = index % design.rows;
                            value - lambda * mode[row] * mode[column]
                                + 0.5
                                    * (mode_ratio[row] * design.maker[index]
                                        + design.maker[index] * mode_ratio[column])
                        })
                        .collect::<Vec<_>>();
                    let remainder_influence =
                        matrix_vector(&remainder_kernel, design.rows, design.rows, &outcome);
                    let trace = trace_variance(&remainder_kernel, selected, design.rows);
                    let target_result = finish_q1_target(
                        point,
                        score,
                        lambda,
                        mode,
                        &remainder_influence,
                        selected,
                        trace,
                        0.0,
                        1.0e-8,
                    )
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

fn make_design(k: usize, controls: bool, dominant: bool, beta_zero: bool) -> DenseDesign {
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
    let xt = transpose(&x, rows, parameters);
    let information = multiply(&xt, parameters, rows, &x, parameters);
    let information_inverse = inverse(&information, parameters);
    let projection = multiply(
        &multiply(&x, rows, parameters, &information_inverse, parameters),
        rows,
        parameters,
        &xt,
        rows,
    );
    let maker = (0..rows * rows)
        .map(|index| {
            let row = index / rows;
            let column = index % rows;
            f64::from(row == column) - projection[index]
        })
        .collect::<Vec<_>>();
    let leverage = (0..rows)
        .map(|row| projection[row * rows + row])
        .collect::<Vec<_>>();
    let maker_inverse = leverage
        .iter()
        .map(|value| (1.0 - value).recip())
        .collect::<Vec<_>>();
    let target = targets(k, parameters, &observations);
    let mut kernel: [Vec<f64>; 4] = core::array::from_fn(|_| Vec::new());
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
        let b = multiply(
            &multiply(&x, rows, parameters, &inv_target_inv, parameters),
            rows,
            parameters,
            &xt,
            rows,
        );
        diagonal[target_index] = (0..rows).map(|row| b[row * rows + row]).collect();
        let ratio = (0..rows)
            .map(|row| diagonal[target_index][row] * maker_inverse[row])
            .collect::<Vec<_>>();
        kernel[target_index] = (0..rows * rows)
            .map(|index| {
                let row = index / rows;
                let column = index % rows;
                b[index] - 0.5 * (ratio[row] * maker[index] + maker[index] * ratio[column])
            })
            .collect();
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
        let coefficients = solve_upper_from_lower(&cholesky, parameters, &gamma);
        let mode = matrix_vector(&x, rows, parameters, &coefficients);
        leading_mode[target_index] = normalize(mode);
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
    DenseDesign {
        rows,
        parameters,
        x,
        maker,
        leverage,
        maker_inverse,
        kernel,
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

fn make_variance(design: &DenseDesign, dgp: VarianceDgp) -> Vec<f64> {
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

fn normalize(mut value: Vec<f64>) -> Vec<f64> {
    let norm = dot(&value, &value).sqrt();
    for entry in &mut value {
        *entry /= norm;
    }
    value
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
