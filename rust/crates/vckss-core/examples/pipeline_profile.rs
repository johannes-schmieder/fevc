// SPDX-License-Identifier: GPL-3.0-only

//! Diagnostic replay of already-prepared public paper inputs, not a native
//! complete-command performance gate or a replacement for sample preparation.

use std::error::Error;
use std::time::Instant;
use vckss_core::engine::{run_jla_no_controls_planned, JlaEngineOptions, PlannedJlaEngineOptions};
use vckss_core::full_cmg::FullCmgPlanOptions;
use vckss_core::generic_jla::{
    run_generic_jla_with_direct_solver_interrupt, GenericJlaExecutionOptions, GenericJlaOptions,
};
use vckss_core::interrupt::{CancellationInterrupt, CancellationToken};
use vckss_core::memory::MemoryBudget;
use vckss_core::model_solver::{ModelRoutingOptions, ModelSolverRoute};
use vckss_core::problem::CanonicalInput;
use vckss_core::types::{DeletionMode, InputColumns};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: pipeline_profile public.csv observation|generic-match|match threads".into(),
        );
    }
    let threads: usize = args[3].parse()?;
    let text = std::fs::read_to_string(&args[1])?;
    let mut lines = text.lines();
    if lines.next() != Some("observation_key,worker,firm,period,match,y") {
        return Err("not the frozen public paper CSV schema".into());
    }
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![],
    };
    for line in lines {
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 6 {
            return Err("invalid CSV row".into());
        }
        input.worker.push(fields[1].parse()?);
        input.firm.push(fields[2].parse()?);
        input.deletion.push(fields[4].parse()?);
        input.outcome.push(fields[5].parse()?);
        input.frequency.push(1);
        input.target_weight.push(1.0);
    }
    let rows = input.outcome.len();
    let start = Instant::now();
    let canonical = CanonicalInput::from_validated(input.validate()?)?;
    let problem = canonical.compress(&vec![true; rows])?;
    let prepare_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut cmg = FullCmgPlanOptions::production(threads, 1e-10, None);
    cmg.memory_budget = MemoryBudget::Unspecified;
    let corrected = if args[2] == "match" {
        run_jla_no_controls_planned(
            &problem,
            PlannedJlaEngineOptions {
                estimator: JlaEngineOptions {
                    seed: 104729,
                    memory_budget: MemoryBudget::Unspecified,
                    memory_limit_bytes: 0,
                    ..Default::default()
                },
                full_cmg: Some(cmg),
                ..Default::default()
            },
        )?
        .estimator
        .corrected
    } else {
        let deletion = match args[2].as_str() {
            "observation" => DeletionMode::Observation,
            "generic-match" => DeletionMode::Match,
            _ => return Err("invalid deletion mode".into()),
        };
        run_generic_jla_with_direct_solver_interrupt(
            &problem,
            GenericJlaExecutionOptions {
                estimator: GenericJlaOptions {
                    deletion,
                    seed: 104729,
                    memory_budget: MemoryBudget::Unspecified,
                    memory_limit_bytes: 0,
                    ..Default::default()
                },
                routing: ModelRoutingOptions {
                    route: ModelSolverRoute::Cmg,
                    ..Default::default()
                },
                ..Default::default()
            },
            None,
            None,
            None,
            Some(cmg),
            &mut CancellationInterrupt::new(CancellationToken::new()),
        )?
        .corrected
    };
    println!("DIAGNOSTIC_CORE_PASS rows={rows} threads={threads} prepare={prepare_seconds:.9} estimate={:.9} worker={:.17} firm={:.17} covariance={:.17} total={:.17}",
        start.elapsed().as_secs_f64(), corrected.worker, corrected.firm, corrected.covariance, corrected.total);
    Ok(())
}
