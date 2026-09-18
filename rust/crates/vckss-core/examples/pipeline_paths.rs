// SPDX-License-Identifier: GPL-3.0-only

//! Bounded public-input attachment diagnostics, not native command timings.
use std::{error::Error, time::Instant};
use vckss_core::component_inference::{
    ComponentInferenceOptions, ComponentInferenceUnit, ComponentVarianceSource,
};
use vckss_core::generic_jla::{
    run_generic_jla_with_automatic_component_batches_interrupt as component_run,
    run_generic_jla_with_diagonal_queue_attachments_interrupt as projection_run,
    GenericJlaExecutionOptions, GenericJlaOptions,
};
use vckss_core::interrupt::{CancellationInterrupt, CancellationToken};
use vckss_core::memory::MemoryBudget;
use vckss_core::model_solver::{ModelRoutingOptions, ModelSolverRoute};
use vckss_core::problem::CanonicalInput;
use vckss_core::projection::{prepare_projection, ProjectionEffect, ProjectionWeight};
use vckss_core::residual_moment_inference::prepare_direct_with_interrupt;
use vckss_core::structured_variance::StructuredVarianceOptions;
use vckss_core::types::{DeletionMode, InputColumns, NuisanceMode};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: pipeline_paths public-8k.csv observation-projection|match-projection|observation-component|match-component threads".into());
    }
    let threads: usize = args[3].parse()?;
    if ![1, 4, 7].contains(&threads) {
        return Err("this diagnostic permits only 1, 4, or 7 threads".into());
    }
    let (matched, component) = match args[2].as_str() {
        "observation-projection" => (false, false),
        "match-projection" => (true, false),
        "observation-component" => (false, true),
        "match-component" => (true, true),
        _ => return Err("unregistered diagnostic path".into()),
    };
    let text = std::fs::read_to_string(&args[1])?;
    let mut lines = text.lines();
    if lines.next() != Some("observation_key,worker,firm,period,match,y,frequency,target_weight,control_1,control_2,projection") {
        return Err("not the public development-input schema".into());
    }
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![]],
    };
    let mut projection = Vec::new();
    for line in lines {
        let f: Vec<_> = line.split(',').collect();
        if f.len() != 11 {
            return Err("invalid public row".into());
        }
        input.worker.push(f[1].parse()?);
        input.firm.push(f[2].parse()?);
        input.deletion.push(f[4].parse()?);
        input.outcome.push(f[5].parse()?);
        input.frequency.push(if matched && component {
            f[6].parse()?
        } else {
            1
        });
        input.target_weight.push(f[7].parse()?);
        input.controls[0].push(f[8].parse()?);
        projection.push(f[10].parse()?);
    }
    if input.outcome.len() != 8_000 {
        return Err("expected exactly 8,000 public rows".into());
    }
    let start = Instant::now();
    let canonical = CanonicalInput::from_validated(input.validate()?)?;
    let problem = canonical.compress(&vec![true; 8_000])?;
    let preparation = start.elapsed().as_secs_f64();
    let mut request = GenericJlaExecutionOptions {
        estimator: GenericJlaOptions {
            deletion: if matched {
                DeletionMode::Match
            } else {
                DeletionMode::Observation
            },
            nuisance: if matched && component {
                NuisanceMode::FixedOffset
            } else {
                NuisanceMode::Joint
            },
            seed: 104729,
            probes: 200,
            memory_budget: MemoryBudget::Unspecified,
            memory_limit_bytes: 0,
            ..Default::default()
        },
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Diagonal,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut interrupt = CancellationInterrupt::new(CancellationToken::new());
    let start = Instant::now();
    let result = if component {
        let prepared = prepare_direct_with_interrupt(
            &problem,
            if matched {
                ComponentInferenceUnit::Match
            } else {
                ComponentInferenceUnit::Observation
            },
            ComponentVarianceSource::StructuredCommon,
            ComponentInferenceOptions {
                seed: 104729,
                probes: 129,
                ..Default::default()
            },
            StructuredVarianceOptions::default(),
            513,
            &mut interrupt,
        )?;
        component_run(&problem, request, &prepared, threads, None, &mut interrupt)?
    } else {
        let prepared = prepare_projection(
            &problem,
            &[projection],
            ProjectionEffect::Firm,
            ProjectionWeight::Target,
            1e-10,
        )?;
        request.estimator.projection_columns = prepared.columns;
        // The projection contract requires both result and export admission.
        // Retain that conservative envelope even in a core-only diagnostic.
        request.estimator.projection_result_bytes = 4096;
        request.estimator.projection_export_bytes = 4096;
        projection_run(
            &problem,
            request,
            Some(&prepared),
            None,
            None,
            threads,
            &mut interrupt,
        )?
    };
    if result.receipt.maximum_complete_residual > result.receipt.full_residual_tolerance {
        return Err("complete original residual gate failed".into());
    }
    let c = result.corrected;
    println!("DIAGNOSTIC_PATH_PASS mode={} threads={threads} prepare={preparation:.9} estimate={:.9} worker={:.17} firm={:.17} covariance={:.17} total={:.17} residual={:.17e} gate={:.17e} memory={}",
        args[2], start.elapsed().as_secs_f64(), c.worker,c.firm,c.covariance,c.total,
        result.receipt.maximum_complete_residual, result.receipt.full_residual_tolerance,
        result.receipt.execution.memory.peak_bytes);
    if let Some(component) = result.component_inference {
        println!(
            "DIAGNOSTIC_ATTACHMENT q0_status={:?} residual={:.17e} gate={:.17e} receipts={}",
            component.q0_status,
            component.maximum_complete_residual,
            component.full_residual_tolerance,
            component.solve_receipts.len()
        );
    }
    Ok(())
}
