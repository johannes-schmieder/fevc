// SPDX-License-Identifier: GPL-3.0-only

//! Four-arm paired diagnostic through unchanged internal grouped attachments.
//! Input design, outcomes, truths, and population oracles are owned by Python.

#[path = "common/q1_reference.rs"]
mod q1_reference;

use std::{env, fs};
use vckss_core::batch_plan::BatchRequest;
use vckss_core::cmg::CmgOptions;
use vckss_core::component_inference::{
    prepare_grouped_oracle_component_inference, prepare_grouped_structured_component_inference,
    q1_am_interval, ComponentInferenceOptions, ComponentQ1Status, ComponentReferenceDistribution,
    ComponentVarianceSource,
};
use vckss_core::generic_jla::{
    run_generic_jla_routed, run_generic_jla_routed_with_attachments_and_hybrid_interrupt,
    GenericJlaExecutionOptions, GenericJlaOptions, GenericJlaResult,
};
use vckss_core::interrupt::NeverInterrupt;
use vckss_core::jla::VarianceComponents;
use vckss_core::krylov::PcgOptions;
use vckss_core::model_solver::{ModelRoutingOptions, ModelSolverOptions, ModelSolverRoute};
use vckss_core::problem::CanonicalInput;
use vckss_core::structured_variance::StructuredVarianceOptions;
use vckss_core::types::{DeletionMode, InputColumns, NuisanceMode};

const TARGETS: [&str; 4] = ["worker", "firm", "covariance", "total"];

#[derive(Debug)]
struct Input {
    columns: InputColumns,
    variance: Vec<f64>,
    truth: [f64; 4],
    seed: u64,
    fold_seed: u64,
}

fn number<T: std::str::FromStr>(tokens: &mut std::str::SplitWhitespace<'_>) -> Result<T, String> {
    tokens
        .next()
        .ok_or_else(|| "truncated diagnostic input".to_owned())?
        .parse()
        .map_err(|_| "invalid diagnostic number".to_owned())
}

fn parse_input(payload: &str) -> Result<Input, String> {
    let mut tokens = payload.split_whitespace();
    if tokens.next() != Some("FEVC_OFFSET_INPUT_V1") {
        return Err("invalid diagnostic input schema".to_owned());
    }
    let n: usize = number(&mut tokens)?;
    let g: usize = number(&mut tokens)?;
    if !(1..=50_000).contains(&n) || !(1..=10_000).contains(&g) {
        return Err("invalid diagnostic dimensions".to_owned());
    }
    let seed = number(&mut tokens)?;
    let fold_seed = number(&mut tokens)?;
    let mut truth = [0.0; 4];
    for value in &mut truth {
        *value = number::<f64>(&mut tokens)?;
        if !value.is_finite() {
            return Err("nonfinite truth".to_owned());
        }
    }
    let mut columns = InputColumns {
        worker: Vec::with_capacity(n),
        firm: Vec::with_capacity(n),
        deletion: Vec::with_capacity(n),
        frequency: Vec::with_capacity(n),
        target_weight: Vec::with_capacity(n),
        outcome: Vec::with_capacity(n),
        controls: vec![Vec::with_capacity(n), Vec::with_capacity(n)],
    };
    for _ in 0..n {
        columns.worker.push(number(&mut tokens)?);
        columns.firm.push(number(&mut tokens)?);
        columns.deletion.push(number(&mut tokens)?);
        columns.frequency.push(number(&mut tokens)?);
        columns.target_weight.push(number(&mut tokens)?);
        columns.outcome.push(number(&mut tokens)?);
        columns.controls[0].push(number(&mut tokens)?);
        columns.controls[1].push(number(&mut tokens)?);
    }
    let mut variance = Vec::with_capacity(g);
    for _ in 0..g {
        let value = number::<f64>(&mut tokens)?;
        if !value.is_finite() || value <= 0.0 {
            return Err("nonpositive original aggregate variance".to_owned());
        }
        variance.push(value);
    }
    if tokens.next().is_some() {
        return Err("extra diagnostic input fields".to_owned());
    }
    columns.clone().validate().map_err(|e| e.to_string())?;
    Ok(Input {
        columns,
        variance,
        truth,
        seed,
        fold_seed,
    })
}

fn components(value: VarianceComponents) -> [f64; 4] {
    [
        value.worker,
        value.firm,
        value.covariance,
        value.worker + value.firm + 2.0 * value.covariance,
    ]
}

fn routed(seed: u64) -> GenericJlaExecutionOptions {
    let estimator = GenericJlaOptions {
        seed: seed ^ 0xd07a_95c3_184e_b62f,
        probes: 256,
        leverage_batch_width: 16,
        target_batch_width: 16,
        deletion: DeletionMode::Match,
        nuisance: NuisanceMode::FixedOffset,
        rank_tolerance: 1.0e-10,
        block_tolerance: 1.0e-10,
        solver: ModelSolverOptions {
            pcg: PcgOptions {
                tolerance: 1.0e-11,
                maximum_iterations: 10_000,
                residual_replacement_interval: 37,
            },
            rank_tolerance: 1.0e-11,
        },
        ..GenericJlaOptions::default()
    };
    GenericJlaExecutionOptions {
        estimator,
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Diagonal,
            cmg_minimum_dimension: 1,
            allow_automatic_cmg_setup_fallback: false,
            solver: estimator.solver,
            cmg: CmgOptions::default(),
        },
        leverage_batch: BatchRequest::Explicit(16),
        target_batch: BatchRequest::Explicit(16),
        wallseconds: None,
    }
}

fn numeric(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.17e}")
    } else {
        "null".to_owned()
    }
}

fn emit_failure(arm: &str, seed: u64, code: &str, phase: &str, truth: &[f64; 4]) {
    for (target, value) in TARGETS.iter().zip(truth) {
        println!("{{\"schema\":\"fevc-offset-paired-row-v1\",\"arm\":\"{arm}\",\"seed\":{seed},\"target\":\"{target}\",\"truth\":{value:.17e},\"status\":\"backend_failure\",\"error_code\":\"{code}\",\"error_phase\":\"{phase}\"}}");
    }
}

fn emit_result(
    arm: &str,
    input: &Input,
    result: &GenericJlaResult,
    export: bool,
) -> Result<(), String> {
    let points = components(result.corrected);
    let inference = result
        .component_inference
        .as_ref()
        .ok_or("missing attachment")?;
    let values = inference.q1.as_ref().ok_or("missing q1 result")?;
    let fold_hash = inference.structured_variance.as_ref().map(|fit| {
        fit.outer_fold
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |state, &fold| {
                (state ^ u64::from(fold)).wrapping_mul(0x100_0000_01b3)
            })
    });
    let variance = inference
        .structured_variance
        .as_ref()
        .map_or(&input.variance, |fit| &fit.common);
    if export {
        println!("{{\"kind\":\"state\",\"arm\":\"{arm}\",\"target_diagonal\":{:?},\"maker_inverse\":{:?},\"variance\":{:?},\"folds\":{:?}}}",
                 inference.target_diagonal, inference.maker_inverse, variance,
                 inference.structured_variance.as_ref().map_or(&[][..], |fit| &fit.outer_fold));
    }
    for target in 0..4 {
        let value = values[target];
        let spectrum = inference.spectrum[target];
        let sd = inference.covariance[target * 4 + target].sqrt();
        let (lower, upper, critical) = if value.status == ComponentQ1Status::Computed {
            let critical = q1_reference::reference_q1_critical(value.curvature, 0.95);
            let interval = q1_am_interval(
                [value.leading_score, value.remainder_estimate],
                [
                    value.leading_variance,
                    value.leading_remainder_covariance,
                    value.leading_remainder_covariance,
                    value.remainder_variance,
                ],
                critical,
                spectrum.leading_eigenvalue,
            )
            .map_err(|e| e.to_string())?;
            (interval[0], interval[1], critical)
        } else {
            (f64::NAN, f64::NAN, f64::NAN)
        };
        let mut fields = vec![
            "\"schema\":\"fevc-offset-paired-row-v1\"".to_owned(),
            format!("\"arm\":\"{arm}\""),
            format!("\"target\":\"{}\"", TARGETS[target]),
            format!("\"seed\":{}", input.seed),
            format!(
                "\"fold_hash\":{}",
                fold_hash.map_or_else(|| "null".to_owned(), |v| format!("\"{v:016x}\""))
            ),
            format!(
                "\"status\":\"{}\"",
                if value.status == ComponentQ1Status::Computed {
                    "success"
                } else {
                    "target_unavailable"
                }
            ),
            format!("\"q1_status\":{}", value.status as u32),
        ];
        for (name, number) in [
            ("truth", input.truth[target]),
            ("point", points[target]),
            ("sd", sd),
            ("lower", lower),
            ("upper", upper),
            ("critical", critical),
            ("production_critical", value.critical_value),
            ("q0_lower", points[target] - 1.959_963_984_540_054 * sd),
            ("q0_upper", points[target] + 1.959_963_984_540_054 * sd),
            ("leading_score", value.leading_score),
            ("leading_variance", value.leading_variance),
            (
                "leading_variance_correction",
                value.leading_variance_correction,
            ),
            ("remainder", value.remainder_estimate),
            ("remainder_variance", value.remainder_variance),
            ("cross_covariance", value.leading_remainder_covariance),
            (
                "remainder_influence_variance",
                value.remainder_influence_variance,
            ),
            ("remainder_trace_variance", value.remainder_trace_variance),
            ("remainder_trace_mcse", value.remainder_trace_mcse),
            ("standardized_determinant", value.standardized_determinant),
            ("curvature", value.curvature),
            ("leading_share", spectrum.leading_share),
            ("remainder_share", spectrum.remainder_leading_share),
            ("leading_eigenvalue", spectrum.leading_eigenvalue),
            ("remainder_identity_error", value.remainder_identity_error),
            (
                "point_identity_error",
                (value.point_estimate - points[target]).abs(),
            ),
            (
                "maximum_complete_residual",
                inference.maximum_complete_residual,
            ),
            ("maximum_mode_weight", spectrum.maximum_mode_weight_squared),
            (
                "remainder_influence_concentration",
                value.remainder_influence_concentration,
            ),
        ] {
            fields.push(format!("\"{name}\":{}", numeric(number)));
        }
        let mean_variance = variance.iter().sum::<f64>() / variance.len() as f64;
        fields.push(format!("\"mean_variance\":{}", numeric(mean_variance)));
        fields.push(format!(
            "\"variance_floor_share\":{}",
            inference.structured_variance.as_ref().map_or_else(
                || "null".to_owned(),
                |fit| numeric(fit.summary[0].floor_share)
            )
        ));
        println!("{{{}}}", fields.join(","));
    }
    Ok(())
}

fn run(input: &Input, export: bool) -> Result<(), String> {
    let mut previous_folds = None;
    for offset in ["known", "estimated"] {
        let mut columns = input.columns.clone();
        if offset == "known" {
            for row in 0..columns.outcome.len() {
                columns.outcome[row] -=
                    0.72 * columns.controls[0][row] - 0.43 * columns.controls[1][row];
            }
            columns.controls.clear();
        }
        let rows = columns.outcome.len();
        let problem =
            CanonicalInput::from_validated(columns.validate().map_err(|e| e.to_string())?)
                .and_then(|value| value.compress(&vec![true; rows]))
                .map_err(|e| e.to_string())?;
        if problem.deletion_units() != input.variance.len() {
            return Err("aggregate variance/deletion inventory mismatch".to_owned());
        }
        let baseline = run_generic_jla_routed(&problem, routed(input.seed));
        for model in ["known", "fitted"] {
            let arm = format!("{offset}_{model}");
            let Ok(baseline) = &baseline else {
                let error = baseline.as_ref().unwrap_err();
                emit_failure(
                    &arm,
                    input.seed,
                    error.code.as_str(),
                    error.phase,
                    &input.truth,
                );
                continue;
            };
            let options = ComponentInferenceOptions {
                seed: input.seed ^ 0x347c_2a91_e85f_d60b,
                probes: 512,
                batch_width: 16,
                spectrum_probes: 128,
                spectrum_iterations: 256,
                spectrum_tolerance: 0.02,
                reference_distribution: ComponentReferenceDistribution::Q1,
                critical_simulations: 4000,
                ..ComponentInferenceOptions::default()
            };
            let prepared = if model == "known" {
                prepare_grouped_oracle_component_inference(&problem, &input.variance, options)
            } else {
                prepare_grouped_structured_component_inference(
                    &problem,
                    ComponentVarianceSource::StructuredCommon,
                    options,
                    StructuredVarianceOptions {
                        seed: input.fold_seed,
                        ..StructuredVarianceOptions::default()
                    },
                )
            };
            let result = prepared.and_then(|prepared| {
                run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
                    &problem,
                    routed(input.seed),
                    None,
                    Some(&prepared),
                    None,
                    &mut NeverInterrupt,
                )
            });
            match result {
                Err(error) => emit_failure(
                    &arm,
                    input.seed,
                    error.code.as_str(),
                    error.phase,
                    &input.truth,
                ),
                Ok(result) => {
                    if components(result.corrected).map(f64::to_bits)
                        != components(baseline.corrected).map(f64::to_bits)
                        || components(result.plugin).map(f64::to_bits)
                            != components(baseline.plugin).map(f64::to_bits)
                    {
                        return Err("attachment changed the point estimate".to_owned());
                    }
                    if let Some(fit) = result
                        .component_inference
                        .as_ref()
                        .and_then(|value| value.structured_variance.as_ref())
                    {
                        if let Some(folds) = &previous_folds {
                            if folds != &fit.outer_fold {
                                return Err("fitted arms changed outcome-free folds".to_owned());
                            }
                        }
                        previous_folds = Some(fit.outer_fold.clone());
                    }
                    emit_result(&arm, input, &result, export)?;
                }
            }
        }
    }
    Ok(())
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = if args.len() == 1 || (args.len() == 2 && args[1] == "export-state") {
        fs::read_to_string(&args[0])
            .map_err(|e| e.to_string())
            .and_then(|text| parse_input(&text))
            .and_then(|input| run(&input, args.len() == 2))
    } else {
        Err("usage: fixed_offset_paired INPUT [export-state]".to_owned())
    };
    if let Err(error) = result {
        eprintln!("FIXED_OFFSET_DIAGNOSTIC_ERROR: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_input_is_rejected_before_estimation() {
        for text in [
            "",
            "WRONG 1 1",
            "FEVC_OFFSET_INPUT_V1 0 1",
            "FEVC_OFFSET_INPUT_V1 1 0",
            "FEVC_OFFSET_INPUT_V1 1 1 1 1 NaN 1 1 1",
            "FEVC_OFFSET_INPUT_V1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 -1",
        ] {
            assert!(parse_input(text).is_err());
        }
    }

    #[test]
    fn known_offset_is_an_explicit_input_operation_only() {
        let input =
            parse_input("FEVC_OFFSET_INPUT_V1 1 1 7 8 1 2 3 9 100 200 300 2 1 5 2 3 0.25").unwrap();
        assert_eq!(input.columns.controls, vec![vec![2.0], vec![3.0]]);
        assert_eq!(input.variance, vec![0.25]);
        assert_eq!(input.seed, 7);
        assert_eq!(input.fold_seed, 8);
        assert!(parse_input(
            "FEVC_OFFSET_INPUT_V1 1 1 7 8 1 2 3 9 100 200 300 2 1 5 2 3 0.25 EXTRA"
        )
        .is_err());
    }
}
