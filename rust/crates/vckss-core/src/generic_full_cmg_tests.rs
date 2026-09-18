// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use crate::interrupt::{CancellationInterrupt, CancellationToken};
use crate::memory::{MemoryBudget, MemoryCheck};
use crate::problem::CanonicalInput;
use crate::types::InputColumns;

pub(super) fn fixture(weighted: bool, repeated: bool) -> CompressedProblem {
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![],
    };
    for w in 0..112 {
        for p in 0..(2 + w % 7) {
            let f = (w + 3 * p) % 16;
            for copy in 0..if repeated { 2 } else { 1 } {
                let row = input.worker.len();
                input.worker.push(w + 1);
                input.firm.push(f + 1);
                input.deletion.push(w * 16 + f + 1);
                input.outcome.push(
                    (w % 11) as f64 * 0.3
                        + (f % 7) as f64 * 0.2
                        + ((row * 17 + 3) % 23) as f64 * 0.1
                        + copy as f64 * 0.2,
                );
                input
                    .frequency
                    .push(if weighted { 1 + row as u64 % 3 } else { 1 });
                input.target_weight.push(if weighted {
                    1.0 + (row % 5) as f64 * 0.25
                } else {
                    1.0
                });
            }
        }
    }
    let rows = input.worker.len();
    CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&vec![true; rows])
        .unwrap()
}

fn options(deletion: DeletionMode, probes: u32) -> GenericJlaExecutionOptions {
    GenericJlaExecutionOptions {
        estimator: GenericJlaOptions {
            deletion,
            probes,
            memory_budget: MemoryBudget::Unspecified,
            memory_limit_bytes: 0,
            ..Default::default()
        },
        routing: ModelRoutingOptions {
            route: ModelSolverRoute::Cmg,
            ..Default::default()
        },
        leverage_batch: BatchRequest::Auto,
        target_batch: BatchRequest::Auto,
        wallseconds: None,
    }
}

fn run(
    problem: &CompressedProblem,
    options: GenericJlaExecutionOptions,
    threads: usize,
) -> Result<GenericJlaResult> {
    run_generic_jla_with_direct_solver_interrupt(
        problem,
        options,
        None,
        None,
        None,
        Some(FullCmgPlanOptions::production(threads, 1e-10, Some(1e-10))),
        &mut CancellationInterrupt::new(CancellationToken::new()),
    )
}

pub(super) fn equivalent(a: &GenericJlaResult, b: &GenericJlaResult) {
    assert_eq!(a.receipt.execution.counter, b.receipt.execution.counter);
    let values = |v: crate::jla::VarianceComponents| [v.worker, v.firm, v.covariance, v.total];
    for (a, b) in values(a.corrected).into_iter().zip(values(b.corrected)) {
        assert!(
            (a - b).abs() <= 1e-8 * a.abs().max(b.abs()).max(1.0),
            "{a} != {b}"
        );
    }
    assert_eq!(
        a.receipt.leverage_probes_accepted,
        b.receipt.leverage_probes_accepted
    );
    assert_eq!(
        a.receipt.target_probes_accepted,
        b.receipt.target_probes_accepted
    );
}

#[test]
fn direct_pooled_hybrid_keeps_mover_and_physical_stayer_corrections() {
    use crate::stayer_hybrid::{prepare_exact_stayer_hybrid, StayerAugmentationInput};
    for weighted in [false, true] {
        for stayer_workers in [0, 160] {
            let mover = fixture(weighted, true);
            let mut stayers = StayerAugmentationInput {
                firm: vec![],
                worker: vec![],
                outcome: vec![],
                frequency: vec![],
                target_weight: vec![],
                controls: vec![],
            };
            for worker in 0..stayer_workers {
                for time in 0..2 + worker % 7 {
                    stayers.firm.push(1 + worker % 16);
                    stayers.worker.push(worker + 1);
                    stayers
                        .outcome
                        .push((worker % 11) as f64 * 0.2 + time as f64 * 0.1);
                    stayers
                        .frequency
                        .push(if weighted { 1 + time % 3 } else { 1 });
                    stayers.target_weight.push(if weighted {
                        1.0 + time as f64 * 0.1
                    } else {
                        1.0
                    });
                }
            }
            let pooled = prepare_exact_stayer_hybrid(&mover, stayers).unwrap();
            let request = options(DeletionMode::Match, 33);
            let reference = run_generic_jla_with_direct_solver_interrupt(
                &pooled.problem,
                request,
                None,
                None,
                Some(&pooled.plan),
                None,
                &mut NeverInterrupt,
            )
            .unwrap();
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                let candidate = run_generic_jla_with_direct_solver_interrupt(
                    &pooled.problem,
                    request,
                    None,
                    None,
                    Some(&pooled.plan),
                    Some(FullCmgPlanOptions::production(threads, 1e-10, Some(1e-10))),
                    &mut CancellationInterrupt::new(CancellationToken::new()),
                )
                .unwrap();
                equivalent(&candidate, &reference);
                let execution = &candidate.receipt.execution;
                assert_eq!(execution.threads.requested, threads);
                assert_eq!(execution.threads.used, threads);
                assert_eq!(
                    execution.memory.peak_bytes,
                    execution.batch.plan.selected_command_peak_bytes
                );
                let direct = execution.full_cmg.as_ref().unwrap();
                assert_eq!(
                    direct.setup.admitted_peak_bytes,
                    execution.memory.peak_bytes
                );
                assert!(direct.maximum_complete_residual <= 1e-9);
            }
        }
    }
}

#[test]
fn both_deletions_share_direct_solver_but_preserve_corrections() {
    for repeated in [false, true] {
        for weighted in [false, true] {
            let problem = fixture(weighted, repeated);
            for deletion in [DeletionMode::Match, DeletionMode::Observation] {
                let request = options(deletion, 33);
                let reference = run_generic_jla_routed(&problem, request).unwrap();
                for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                    let candidate = run(&problem, request, threads).unwrap();
                    equivalent(&candidate, &reference);
                    let execution = &candidate.receipt.execution;
                    let cmg = execution.full_cmg.as_ref().unwrap();
                    assert_eq!(execution.threads.requested, threads);
                    assert_eq!(execution.threads.used, threads);
                    assert_eq!(cmg.setup.vertices, 16 + 64);
                    assert_eq!(
                        cmg.setup.maximum_batch_rhs,
                        execution
                            .batch
                            .leverage_active_width
                            .max(2 * execution.batch.target_active_width)
                    );
                    assert!(cmg.maximum_complete_residual <= 1e-9);
                    assert_eq!(cmg.setup.admitted_peak_bytes, execution.memory.peak_bytes);
                }
            }
        }
    }
}

#[test]
fn explicit_budget_and_forbidden_batch_reject_before_rng() {
    let problem = fixture(false, true);
    let mut request = options(DeletionMode::Observation, 33);
    request.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: 1,
        check: MemoryCheck::Error,
    };
    assert_eq!(
        run(&problem, request, 7).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    request.estimator.memory_budget = MemoryBudget::Unspecified;
    request.leverage_batch = BatchRequest::Explicit(4);
    assert_eq!(
        run(&problem, request, 7).unwrap_err().code,
        ErrorCode::UnsupportedFeature
    );
    assert!(run(&problem, options(DeletionMode::Observation, 33), 7).is_ok());
}

#[test]
fn budgets_reduce_widths_and_exact_boundary_is_reconciled() {
    let problem = fixture(false, true);
    let free = run(&problem, options(DeletionMode::Observation, 200), 28).unwrap();
    let peak = free.receipt.execution.memory.peak_bytes;
    let mut reduced = false;
    for percent in [100, 95, 90, 80, 70, 60, 50, 40] {
        let mut request = options(DeletionMode::Observation, 200);
        let limit = peak * percent / 100;
        request.estimator.memory_limit_bytes = limit;
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: limit,
            check: MemoryCheck::Error,
        };
        match run(&problem, request, 28) {
            Ok(result) => {
                equivalent(&result, &free);
                let execution = &result.receipt.execution;
                assert!(execution.memory.peak_bytes <= limit);
                assert_eq!(
                    execution.batch.plan.selected_command_peak_bytes,
                    execution.memory.peak_bytes
                );
                reduced |= execution.batch.leverage_active_width
                    < free.receipt.execution.batch.leverage_active_width
                    || execution.batch.target_active_width
                        < free.receipt.execution.batch.target_active_width;
            }
            Err(error) => assert_eq!(error.code, ErrorCode::ResourceLimit),
        }
    }
    assert!(
        reduced,
        "budget fixture must exercise reduced automatic widths"
    );
    // For this low-thread cell, construction dominates even width one. The
    // final bound must accept exactly and reject one byte below it.
    let mut request = options(DeletionMode::Observation, 33);
    let boundary = run(&problem, request, 1)
        .unwrap()
        .receipt
        .execution
        .memory
        .peak_bytes;
    request.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: boundary,
        check: MemoryCheck::Error,
    };
    request.estimator.memory_limit_bytes = boundary;
    assert!(run(&problem, request, 1).is_ok());
    request.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: boundary - 1,
        check: MemoryCheck::Error,
    };
    request.estimator.memory_limit_bytes = boundary - 1;
    assert_eq!(
        run(&problem, request, 1).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn cancellation_at_each_direct_phase_is_atomic_and_allows_reuse() {
    struct BreakAt(&'static str);
    impl InterruptCheck for BreakAt {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.0 {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "deliberate test break",
                ))
            } else {
                Ok(())
            }
        }
    }
    let problem = fixture(false, true);
    for phase in [
        "generic_full_cmg_pools_admitted",
        "cmg_full_v2_rhs",
        "generic_jla_observation_leverage_batch",
        "generic_jla_target_batch",
    ] {
        let error = run_generic_jla_with_direct_solver_interrupt(
            &problem,
            options(DeletionMode::Observation, 33),
            None,
            None,
            None,
            Some(FullCmgPlanOptions::production(4, 1e-10, None)),
            &mut BreakAt(phase),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::UserBreak, "{phase}");
        assert!(run(&problem, options(DeletionMode::Observation, 33), 4).is_ok());
    }
}

pub(super) fn controlled_fixture() -> CompressedProblem {
    let mut problem = fixture(true, true);
    let rows = problem.outcome.len();
    problem.controls = (0..2)
        .map(|control| {
            (0..rows)
                .map(|row| ((row * (13 + 4 * control) + 11) as f64 * 0.37).sin())
                .collect()
        })
        .collect();
    for (row, outcome) in problem.outcome.iter_mut().enumerate() {
        *outcome += row as f64 * 0.000001;
    }
    problem
}

#[test]
fn direct_controlled_point_preserves_strict_rank_and_estimation_solves() {
    let problem = controlled_fixture();
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let mut request = options(deletion, 5);
            request.estimator.nuisance = nuisance;
            let reference = run_generic_jla_routed(&problem, request).unwrap();
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                // A loose ordinary point-probe default must not leak into
                // controlled estimation or the stricter rank projections.
                let candidate = run_generic_jla_with_direct_solver_interrupt(
                    &problem,
                    request,
                    None,
                    None,
                    None,
                    Some(FullCmgPlanOptions::production(threads, 1e-10, None)),
                    &mut CancellationInterrupt::new(CancellationToken::new()),
                )
                .unwrap();
                equivalent(&candidate, &reference);
                let rank = &candidate.receipt.control_rank;
                assert_eq!(rank.projection_rhs.len(), 2);
                assert_eq!(rank.projection_pcg_tolerance, 1e-13);
                assert_eq!(rank.projection_residual_gate, 1e-11);
                assert!(rank.maximum_projection_residual <= 1e-11);
                assert!(candidate.receipt.maximum_complete_residual <= 1e-9);
                assert_eq!(candidate.receipt.full_residual_tolerance, 1e-9);
                let execution = &candidate.receipt.execution;
                let cmg = execution.full_cmg.as_ref().unwrap();
                assert_eq!(cmg.setup.probe_effective_tolerance, 1e-10);
                assert_eq!(cmg.setup.admitted_peak_bytes, execution.memory.peak_bytes);
                assert_eq!(cmg.setup.threads, threads);
                assert!(cmg.rhs_count >= 1 + 2 + 3 * 5);
            }
        }
    }
}

#[test]
fn controlled_admission_boundaries_and_cancellation_remain_pre_rng() {
    struct BreakAt(&'static str);
    impl InterruptCheck for BreakAt {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.0 {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "controlled test cancellation",
                ))
            } else {
                Ok(())
            }
        }
    }
    let problem = controlled_fixture();
    let mut request = options(DeletionMode::Observation, 5);
    let free = run(&problem, request, 1).unwrap();
    let boundary = free.receipt.execution.memory.peak_bytes;
    for (bytes, success) in [(boundary, true), (boundary - 1, false)] {
        request.estimator.memory_limit_bytes = bytes;
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes,
            check: MemoryCheck::Error,
        };
        let result = run(&problem, request, 1);
        if success {
            equivalent(&result.unwrap(), &free);
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
    for check in [MemoryCheck::Warn, MemoryCheck::Off] {
        request.estimator.memory_budget = MemoryBudget::Explicit { bytes: 1, check };
        request.estimator.memory_limit_bytes = 1;
        let result = run(&problem, request, 7).unwrap();
        equivalent(&result, &free);
        assert_eq!(result.receipt.execution.batch.leverage_active_width, 1);
        assert_eq!(result.receipt.execution.batch.target_active_width, 1);
    }
    request.estimator.memory_budget = MemoryBudget::Unspecified;
    for phase in [
        "generic_full_cmg_controls_admitted",
        "model_rank_projection_rhs",
        "model_direct_control_rhs",
    ] {
        let error = run_generic_jla_with_direct_solver_interrupt(
            &problem,
            request,
            None,
            None,
            None,
            Some(FullCmgPlanOptions::production(4, 1e-10, None)),
            &mut BreakAt(phase),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::UserBreak, "{phase}");
        equivalent(&run(&problem, request, 4).unwrap(), &free);
    }
    let mut singular = problem.clone();
    singular.controls[1] = singular.controls[0].clone();
    let reference = run_generic_jla_routed(&singular, request).unwrap_err();
    assert_eq!(run(&singular, request, 4).unwrap_err().code, reference.code);
}

#[test]
fn controlled_direct_pooled_stayers_preserve_hybrid_estimand() {
    use crate::stayer_hybrid::{prepare_exact_stayer_hybrid, StayerAugmentationInput};
    let movers = controlled_fixture();
    let mut stayers = StayerAugmentationInput {
        firm: vec![],
        worker: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![vec![], vec![]],
    };
    for worker in 0..64 {
        for time in 0..3 + worker % 5 {
            let row = stayers.outcome.len();
            stayers.firm.push(1 + worker % 16);
            stayers.worker.push(worker + 1);
            stayers
                .outcome
                .push(0.5 * (worker % 7) as f64 + 0.2 * time as f64 + row as f64 * 0.000003);
            stayers.frequency.push(1 + time % 3);
            stayers.target_weight.push(1.0 + time as f64 * 0.1);
            for j in 0..2 {
                stayers.controls[j].push(((row * (13 + 4 * j) + 11) as f64 * 0.37).sin());
            }
        }
    }
    let pooled = prepare_exact_stayer_hybrid(&movers, stayers).unwrap();
    let request = options(DeletionMode::Match, 33);
    let reference = run_generic_jla_with_direct_solver_interrupt(
        &pooled.problem,
        request,
        None,
        None,
        Some(&pooled.plan),
        None,
        &mut NeverInterrupt,
    )
    .unwrap();
    for threads in [1, 7, 28] {
        let candidate = run_generic_jla_with_direct_solver_interrupt(
            &pooled.problem,
            request,
            None,
            None,
            Some(&pooled.plan),
            Some(FullCmgPlanOptions::production(threads, 1e-10, None)),
            &mut NeverInterrupt,
        )
        .unwrap();
        equivalent(&candidate, &reference);
        assert_eq!(
            candidate.receipt.deletion_units,
            reference.receipt.deletion_units
        );
        assert_eq!(candidate.receipt.parameters, reference.receipt.parameters);
        assert!(candidate.receipt.maximum_complete_residual <= 1e-9);
    }
}
