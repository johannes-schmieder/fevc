// SPDX-License-Identifier: GPL-3.0-only
//! Private complete-run adapter. Python owns manifests, semantic keys and audits.
//! Each call uses the existing point executor with fresh solver preparation.

use std::{
    fmt::Write as _,
    io::{self, Read},
    time::Instant,
};
use vckss_core::batch_plan::BatchRequest;
use vckss_core::exact_estimator::ExactStayerHybridPlan;
use vckss_core::generic_jla::{
    run_generic_jla_reference_interrupt, GenericJlaExecutionOptions, GenericJlaOptions,
    ReferenceProbePlan,
};
use vckss_core::interrupt::NeverInterrupt;
use vckss_core::jla::VarianceComponents;
use vckss_core::krylov::PcgOptions;
use vckss_core::model_solver::{ModelRoutingOptions, ModelSolverOptions, ModelSolverRoute};
use vckss_core::numerical_mc::Status;
use vckss_core::problem::CanonicalInput;
use vckss_core::types::{DeletionMode, InputColumns, NuisanceMode};

fn number<T: std::str::FromStr>(tokens: &mut std::str::SplitWhitespace<'_>) -> Result<T, String> {
    tokens
        .next()
        .ok_or_else(|| "truncated reference input".to_owned())?
        .parse()
        .map_err(|_| "invalid reference input".to_owned())
}

fn scalar(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "null".to_owned()
    }
}

fn vector(values: &[f64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|&v| scalar(v))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn matrix(values: [[f64; 3]; 3]) -> String {
    if values.iter().flatten().any(|v| !v.is_finite()) {
        return "null".to_owned();
    }
    format!(
        "[{}]",
        values
            .iter()
            .map(|v| vector(v))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn primitive(value: VarianceComponents) -> String {
    vector(&[value.worker, value.firm, value.covariance])
}

fn status(value: Status) -> &'static str {
    match value {
        Status::ExactZero => "exact_zero",
        Status::OkLocal => "ok_local",
        Status::OkLocalPsdAdjusted => "ok_local_psd_adjusted",
        Status::UnstableNonPsd => "unstable_nonpsd",
        Status::NonsmoothAdjustment => "nonsmooth_adjustment",
        Status::NonfiniteDerivative => "nonfinite_derivative",
        Status::ReplayFailed => "replay_failed",
    }
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())?;
    let mut tokens = input.split_whitespace();
    if tokens.next() != Some("FEVC_ALL_PROBE_NATIVE_INPUT_V1") {
        return Err("invalid native input schema".to_owned());
    }
    let rows: usize = number(&mut tokens)?;
    let controls: usize = number(&mut tokens)?;
    let r: u32 = number(&mut tokens)?;
    let t: u32 = number(&mut tokens)?;
    let attempts: usize = number(&mut tokens)?;
    let deletion = match tokens.next() {
        Some("observation") => DeletionMode::Observation,
        Some("match") => DeletionMode::Match,
        _ => return Err("invalid deletion".to_owned()),
    };
    let nuisance = match tokens.next() {
        Some("joint") => NuisanceMode::Joint,
        Some("fixedoffset") => NuisanceMode::FixedOffset,
        _ => return Err("invalid nuisance".to_owned()),
    };
    let mover_groups: usize = number(&mut tokens)?;
    let hybrid: u32 = number(&mut tokens)?;
    if rows == 0
        || rows > 512
        || controls > 32
        || !(2..=400).contains(&r)
        || !(2..=400).contains(&t)
        || attempts == 0
        || attempts > 100_000
        || hybrid > 1
    {
        return Err("private small-reference limits exceeded".to_owned());
    }
    let mut data = InputColumns {
        worker: Vec::new(),
        firm: Vec::new(),
        deletion: Vec::new(),
        outcome: Vec::new(),
        frequency: Vec::new(),
        target_weight: Vec::new(),
        controls: vec![Vec::new(); controls],
    };
    for _ in 0..rows {
        data.worker.push(number(&mut tokens)?);
        data.firm.push(number(&mut tokens)?);
        data.deletion.push(number(&mut tokens)?);
        data.frequency.push(number(&mut tokens)?);
        data.target_weight.push(number(&mut tokens)?);
        data.outcome.push(number(&mut tokens)?);
        for column in &mut data.controls {
            column.push(number(&mut tokens)?);
        }
    }
    let canonical = CanonicalInput::from_validated(data.validate().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let problem = canonical
        .compress(&vec![true; rows])
        .map_err(|e| e.to_string())?;
    let plan = (hybrid == 1).then(|| ExactStayerHybridPlan {
        stayer_rows: problem
            .row_deletion
            .iter()
            .map(|&g| g as usize >= mover_groups)
            .collect(),
        mover_deletion_units: mover_groups,
    });
    let solver = ModelSolverOptions {
        pcg: PcgOptions {
            tolerance: 1e-12,
            maximum_iterations: 5000,
            residual_replacement_interval: 17,
        },
        ..ModelSolverOptions::default()
    };
    let options = GenericJlaExecutionOptions {
        estimator: GenericJlaOptions {
            probes: r,
            deletion,
            nuisance,
            solver,
            ..GenericJlaOptions::default()
        },
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Diagonal,
            solver,
            ..ModelRoutingOptions::default()
        },
        leverage_batch: BatchRequest::Explicit(7),
        target_batch: BatchRequest::Explicit(5),
        wallseconds: None,
    };
    for _ in 0..attempts {
        let k: usize = number(&mut tokens)?;
        let l: usize = number(&mut tokens)?;
        let leverage_seed: u64 = number(&mut tokens)?;
        let target_seed: u64 = number(&mut tokens)?;
        let probe_plan = ReferenceProbePlan {
            leverage_seed,
            target_seed,
            leverage_probes: r,
            target_probes: t,
        };
        let start = Instant::now();
        let result = run_generic_jla_reference_interrupt(
            &problem,
            options,
            plan.as_ref(),
            probe_plan,
            true,
            &mut NeverInterrupt,
        );
        let mut output = format!("{{\"key\":[{k},{l}],\"leverage_seed\":{leverage_seed},\"target_seed\":{target_seed},\"R\":{r},\"T\":{t},\"fold_a\":{},\"fold_b\":{},\"seconds\":{},\"route\":\"generic_diagonal\"",
                                 t.div_ceil(2), t / 2, start.elapsed().as_secs_f64());
        match result {
            Err(error) => {
                write!(output, ",\"point_status\":\"failed\",\"diagnostic_status\":\"point_failed\",\"point_error_code\":{},\"point_error_phase\":\"{}\"}}", error.code as u32, error.phase)
                    .map_err(|e| e.to_string())?;
            }
            Ok((point, Some(diagnostic))) => {
                let c = diagnostic.covariance;
                let mcse = c.mcse.map_or_else(|| "null".to_owned(), |v| vector(&v));
                let usable = c.usable.map_or_else(|| "null".to_owned(), matrix);
                let replay_residual = diagnostic
                    .replay_rhs
                    .iter()
                    .fold(0.0_f64, |m, rhs| m.max(rhs.complete_residual));
                let counter = point.receipt.execution.counter;
                write!(output, ",\"point_status\":\"ok\",\"point\":{},\"plugin\":{},\"correction\":{},\"conditional_mcse\":{},\"conditional\":{},\"leverage\":{},\"all_raw\":{},\"all_usable\":{},\"all_mcse\":{},\"diagnostic_status\":\"{}\",\"psd_adjustment\":{},\"minimum_margin\":{},\"minimum_constrained\":{},\"sensitivity_ratio\":{},\"maximum_complete_residual\":{},\"maximum_replay_complete_residual\":{},\"rhs\":{},\"replay_rhs\":{},\"replay_executed_rhs\":{},\"replay_attempted_rhs\":{},\"replay_generator_word_evaluations\":{},\"counter_unique_packed_words\":{},\"counter_physical_trials\":{},\"allocation_bound_bytes\":{},\"failed_replay_probe\":{}}}",
                    primitive(point.corrected), primitive(point.plugin), primitive(point.correction),
                    vector(&[point.numerical_mcse.worker, point.numerical_mcse.firm, point.numerical_mcse.covariance, point.numerical_mcse.total]),
                    matrix(c.conditional), matrix(c.leverage), matrix(c.raw), usable, mcse, status(c.status),
                    scalar(c.psd_adjustment), scalar(diagnostic.minimum_residual_margin), scalar(diagnostic.minimum_constrained),
                    scalar(diagnostic.maximum_sensitivity_ratio), scalar(point.receipt.maximum_complete_residual), scalar(replay_residual),
                    point.receipt.rhs.len(), diagnostic.replay_rhs.len(), diagnostic.replay_executed_rhs_count, diagnostic.replay_attempted_rhs_count,
                    diagnostic.replay_generator_word_evaluations, counter.total.actual_unique_packed_words,
                    counter.total.actual_physical_bernoulli_trials, diagnostic.allocation_bound_bytes,
                    diagnostic.failed_replay_probe.map_or_else(|| "null".to_owned(), |p| p.to_string()))
                    .map_err(|e| e.to_string())?;
            }
            Ok((_, None)) => return Err("lost requested numerical attachment".to_owned()),
        }
        println!("{output}");
    }
    if tokens.next().is_some() {
        return Err("unexpected input trailer".to_owned());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
