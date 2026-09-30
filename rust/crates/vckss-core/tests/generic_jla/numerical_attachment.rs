// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use vckss_core::generic_jla::run_generic_jla_with_numerical_mc_interrupt;
use vckss_core::memory::{MemoryBudget, MemoryCheck};
use vckss_core::numerical_mc::Status;

fn assert_matrix_relative(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) {
    let scale = a
        .iter()
        .chain(b.iter())
        .flatten()
        .fold(0.0_f64, |s, x| s.max(x.abs()));
    if scale == 0.0 {
        assert_eq!(a, b);
        return;
    }
    let norm = |m: [[f64; 3]; 3]| {
        m.iter()
            .flatten()
            .map(|x| (x / scale).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let difference = a
        .iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x / scale - y / scale).powi(2))
        .sum::<f64>()
        .sqrt();
    // Matrix Frobenius norm, with no absolute outcome-unit floor.
    assert!(difference / norm(a).max(norm(b)) <= 1e-11);
}

#[test]
fn current_queued_executor_keeps_point_work_and_reconciles_replay_work() {
    use vckss_core::full_cmg::FullCmgPlanOptions;
    use vckss_core::generic_jla::{
        run_generic_jla_with_numerical_mc_resolved_interrupt,
        run_generic_jla_with_resolved_execution_interrupt,
    };
    let problem = fixture(true);
    for threads in [1, 4, 7] {
        for deletion in [DeletionMode::Observation, DeletionMode::Match] {
            let mut opts = options(deletion, NuisanceMode::Joint);
            opts.probes = 17;
            let mut request = routed_options(opts, ModelSolverRoute::Auto);
            request.leverage_batch = BatchRequest::Auto;
            request.target_batch = BatchRequest::Auto;
            let cmg = FullCmgPlanOptions::production(threads, 1e-12, Some(1e-12));
            let base = run_generic_jla_with_resolved_execution_interrupt(
                &problem,
                request,
                None,
                None,
                None,
                threads,
                cmg,
                &mut NeverInterrupt,
            )
            .unwrap();
            let (point, diag) = run_generic_jla_with_numerical_mc_resolved_interrupt(
                &problem,
                request,
                None,
                threads,
                cmg,
                true,
                &mut NeverInterrupt,
            )
            .unwrap();
            assert_eq!(point.corrected, base.corrected);
            assert_eq!(point.numerical_mcse, base.numerical_mcse);
            assert_eq!(
                point.receipt.execution.counter,
                base.receipt.execution.counter
            );
            assert_eq!(diag.replay_rhs.len(), 17);
            assert_eq!(
                point
                    .receipt
                    .execution
                    .diagonal_queue
                    .unwrap()
                    .work
                    .completed_rhs,
                base.receipt
                    .execution
                    .diagonal_queue
                    .unwrap()
                    .work
                    .completed_rhs
                    + 17
            );
        }
    }
}

#[test]
fn automatic_direct_replay_late_certification_preserves_point_and_recorded_work() {
    use vckss_core::full_cmg::FullCmgPlanOptions;
    use vckss_core::generic_jla::{
        run_generic_jla_with_numerical_mc_resolved_interrupt,
        run_generic_jla_with_resolved_execution_interrupt,
    };
    let mut columns = InputColumns {
        worker: Vec::new(),
        firm: Vec::new(),
        deletion: Vec::new(),
        outcome: Vec::new(),
        frequency: Vec::new(),
        target_weight: Vec::new(),
        controls: Vec::new(),
    };
    // Cross the registered Auto threshold without a dense observation matrix.
    for worker in 0..8_u64 {
        for firm in 0..256_u64 {
            for copy in 0..2_u64 {
                let row = columns.worker.len() as u64;
                columns.worker.push(worker);
                columns.firm.push(firm);
                columns.deletion.push(worker * 256 + firm);
                columns.frequency.push(1 + row % 2);
                columns.target_weight.push(0.7 + (row % 7) as f64 / 5.0);
                columns.outcome.push(
                    0.3 * worker as f64 - 0.02 * firm as f64
                        + ((row * 37 + copy * 11) % 101) as f64 / 50.0
                        - 1.0,
                );
            }
        }
    }
    let rows = columns.worker.len();
    let problem = CanonicalInput::from_validated(columns.validate().unwrap())
        .unwrap()
        .compress(&vec![true; rows])
        .unwrap();
    let mut opts = options(DeletionMode::Match, NuisanceMode::Joint);
    opts.probes = 33;
    let mut request = routed_options(opts, ModelSolverRoute::Auto);
    request.leverage_batch = BatchRequest::Auto;
    request.target_batch = BatchRequest::Auto;
    let cmg = FullCmgPlanOptions::production(1, 1e-12, Some(1e-12));
    let baseline = run_generic_jla_with_resolved_execution_interrupt(
        &problem,
        request,
        None,
        None,
        None,
        1,
        cmg,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert!(baseline.receipt.execution.direct_attachments.is_some());
    struct LateCertification {
        batches: usize,
        fail_batch: usize,
    }
    impl InterruptCheck for LateCertification {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == "numerical_mc_replay" {
                self.batches += 1;
            }
            if self.batches >= self.fail_batch && phase == "model_full_residual" {
                Err(BackendError::new(
                    ErrorCode::FullResidualFailed,
                    phase,
                    "injected rejection after lower CMG certification",
                ))
            } else {
                Ok(())
            }
        }
    }
    for fail_batch in [1, 2] {
        let (point, diagnostic) = run_generic_jla_with_numerical_mc_resolved_interrupt(
            &problem,
            request,
            None,
            1,
            cmg,
            true,
            &mut LateCertification {
                batches: 0,
                fail_batch,
            },
        )
        .unwrap();
        assert_eq!(point.corrected, baseline.corrected);
        assert_eq!(point.numerical_mcse, baseline.numerical_mcse);
        assert_eq!(
            point.receipt.execution.counter,
            baseline.receipt.execution.counter
        );
        assert_eq!(diagnostic.covariance.status, Status::ReplayFailed);
        assert!(diagnostic.covariance.usable.is_none());
        assert_eq!(diagnostic.replay_rhs.len(), 4 * (fail_batch - 1));
        assert_eq!(
            diagnostic.failed_replay_probe,
            Some((4 * (fail_batch - 1)) as u32)
        );
        assert_eq!(diagnostic.replay_executed_rhs_count, 4 * fail_batch);
        assert_eq!(diagnostic.replay_attempted_rhs_count, 4 * fail_batch);
        assert_eq!(
            point.receipt.execution.full_cmg.unwrap().rhs_count,
            baseline
                .receipt
                .execution
                .full_cmg
                .as_ref()
                .unwrap()
                .rhs_count
                + 4 * fail_batch as u64
        );
    }
    let (reused, diagnostic) = run_generic_jla_with_numerical_mc_resolved_interrupt(
        &problem,
        request,
        None,
        1,
        cmg,
        true,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(reused.corrected, baseline.corrected);
    assert_eq!(diagnostic.replay_rhs.len(), 33);
    assert_eq!(diagnostic.replay_executed_rhs_count, 33);
}

#[test]
fn private_probe_plan_keeps_public_mapping_and_independent_stage_identity() {
    use vckss_core::generic_jla::{run_generic_jla_reference_interrupt, ReferenceProbePlan};
    let problem = fixture(true);
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        let mut opts = options(deletion, NuisanceMode::Joint);
        opts.probes = 17;
        let request = routed_options(opts, ModelSolverRoute::Diagonal);
        let public = run_generic_jla_with_numerical_mc_interrupt(
            &problem,
            request,
            None,
            &mut NeverInterrupt,
        )
        .unwrap();
        let mut plan = ReferenceProbePlan {
            leverage_seed: opts.seed,
            target_seed: opts.seed,
            leverage_probes: 17,
            target_probes: 17,
        };
        let private = run_generic_jla_reference_interrupt(
            &problem,
            request,
            None,
            plan,
            true,
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(private.0.corrected, public.0.corrected);
        assert_eq!(
            private.0.receipt.execution.counter,
            public.0.receipt.execution.counter
        );
        assert_matrix_relative(private.1.unwrap().covariance.raw, public.1.covariance.raw);
        for (r, t) in [(7, 19), (19, 7)] {
            plan.leverage_probes = r;
            plan.target_probes = t;
            plan.target_seed = 991;
            let (point, diagnostic) = run_generic_jla_reference_interrupt(
                &problem,
                request,
                None,
                plan,
                true,
                &mut NeverInterrupt,
            )
            .unwrap();
            let baseline = run_generic_jla_reference_interrupt(
                &problem,
                request,
                None,
                plan,
                false,
                &mut NeverInterrupt,
            )
            .unwrap()
            .0;
            assert_eq!(point.corrected, baseline.corrected);
            assert_eq!(point.numerical_mcse, baseline.numerical_mcse);
            assert_eq!(
                point.receipt.execution.counter,
                baseline.receipt.execution.counter
            );
            assert_eq!(
                (
                    point.receipt.leverage_probes_accepted,
                    point.receipt.target_probes_accepted
                ),
                (r, t)
            );
            assert_eq!(
                point.receipt.rhs.len(),
                problem.controls.len() + 1 + r as usize + 2 * t as usize
            );
            let diagnostic = diagnostic.unwrap();
            assert_eq!(diagnostic.folds, [t.div_ceil(2), t / 2]);
            assert_eq!(diagnostic.replay_rhs.len(), r as usize);
            let leverage = point
                .receipt
                .rhs
                .iter()
                .filter(|s| s.phase == GenericJlaRhsPhase::Leverage)
                .map(|s| s.complete_residual)
                .collect::<Vec<_>>();
            plan.target_seed = 992;
            let next = run_generic_jla_reference_interrupt(
                &problem,
                request,
                None,
                plan,
                false,
                &mut NeverInterrupt,
            )
            .unwrap()
            .0;
            assert_eq!(
                leverage,
                next.receipt
                    .rhs
                    .iter()
                    .filter(|s| s.phase == GenericJlaRhsPhase::Leverage)
                    .map(|s| s.complete_residual)
                    .collect::<Vec<_>>()
            );
            assert_ne!(point.corrected, next.corrected);
        }
    }
}

#[test]
fn explicit_current_direct_cmg_preserves_point_route_and_work() {
    use vckss_core::full_cmg::FullCmgPlanOptions;
    use vckss_core::generic_jla::{
        run_generic_jla_with_direct_solver_interrupt,
        run_generic_jla_with_numerical_mc_resolved_interrupt,
    };
    for controls in [false, true] {
        let problem = fixture(controls);
        for deletion in [DeletionMode::Observation, DeletionMode::Match] {
            for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
                let mut opts = options(deletion, nuisance);
                opts.probes = 17;
                let mut request = routed_options(opts, ModelSolverRoute::Cmg);
                request.leverage_batch = BatchRequest::Auto;
                request.target_batch = BatchRequest::Auto;
                let cmg = FullCmgPlanOptions::production(3, 1e-12, Some(1e-12));
                let base = run_generic_jla_with_direct_solver_interrupt(
                    &problem,
                    request,
                    None,
                    None,
                    None,
                    Some(cmg),
                    &mut NeverInterrupt,
                )
                .unwrap();
                let (point, diag) = run_generic_jla_with_numerical_mc_resolved_interrupt(
                    &problem,
                    request,
                    None,
                    3,
                    cmg,
                    true,
                    &mut NeverInterrupt,
                )
                .unwrap();
                assert_eq!(point.corrected, base.corrected);
                assert_eq!(point.numerical_mcse, base.numerical_mcse);
                assert_eq!(
                    point.receipt.execution.counter,
                    base.receipt.execution.counter
                );
                assert_eq!(
                    point.receipt.execution.selected_route,
                    ModelSolverRoute::Cmg
                );
                assert_eq!(diag.replay_rhs.len(), 17);
                let point_work = point.receipt.execution.full_cmg.unwrap();
                let base_work = base.receipt.execution.full_cmg.unwrap();
                let extra_refinements = point_work.model_diagnostics.control_refinement_rhs_count
                    - base_work.model_diagnostics.control_refinement_rhs_count;
                assert_eq!(
                    point_work.rhs_count,
                    base_work.rhs_count + 17 + extra_refinements
                );
                for r in diag.replay_rhs {
                    assert!(r.complete_residual <= point.receipt.full_residual_tolerance);
                }
            }
        }
    }
}

#[test]
fn semantic_probe_order_preserves_all_probe_outputs_under_row_permutation() {
    let mut forward = fixture(true);
    let n = forward.outcome.len();
    forward.probe_order = Some((0..n).map(|r| r as f64).collect());
    let mut reverse = CanonicalInput::from_validated(
        InputColumns {
            worker: forward
                .row_worker
                .iter()
                .rev()
                .map(|&x| u64::from(x) + 1)
                .collect(),
            firm: forward
                .row_firm
                .iter()
                .rev()
                .map(|&x| u64::from(x) + 1)
                .collect(),
            deletion: forward
                .row_deletion
                .iter()
                .rev()
                .map(|&x| u64::from(x) + 1)
                .collect(),
            outcome: forward.outcome.iter().rev().copied().collect(),
            frequency: forward.frequency.iter().rev().copied().collect(),
            target_weight: forward.target_weight.iter().rev().copied().collect(),
            controls: forward
                .controls
                .iter()
                .map(|c| c.iter().rev().copied().collect())
                .collect(),
        }
        .validate()
        .unwrap(),
    )
    .unwrap()
    .compress(&vec![true; n])
    .unwrap();
    reverse.probe_order = Some((0..n).rev().map(|r| r as f64).collect());
    for deletion in [DeletionMode::Observation, DeletionMode::Match] {
        let request = routed_options(
            options(deletion, NuisanceMode::Joint),
            ModelSolverRoute::Diagonal,
        );
        let (a, ad) = run_generic_jla_with_numerical_mc_interrupt(
            &forward,
            request,
            None,
            &mut NeverInterrupt,
        )
        .unwrap();
        let (b, bd) = run_generic_jla_with_numerical_mc_interrupt(
            &reverse,
            request,
            None,
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(a.receipt.execution.counter, b.receipt.execution.counter);
        for (x, y) in components(a.corrected)
            .into_iter()
            .zip(components(b.corrected))
        {
            assert!((x - y).abs() <= 1e-11 * x.abs().max(y.abs()));
        }
        assert_matrix_relative(ad.covariance.conditional, bd.covariance.conditional);
        assert_matrix_relative(ad.covariance.leverage, bd.covariance.leverage);
        assert_matrix_relative(ad.covariance.raw, bd.covariance.raw);
    }
}

fn check_point_and_attachment(
    problem: &CompressedProblem,
    execution: GenericJlaExecutionOptions,
    hybrid: Option<&vckss_core::exact_estimator::ExactStayerHybridPlan>,
) {
    let baseline = run_generic_jla_routed_with_projection_and_hybrid_interrupt(
        problem,
        execution,
        None,
        hybrid,
        &mut NeverInterrupt,
    )
    .unwrap();
    let (point, attachment) = run_generic_jla_with_numerical_mc_interrupt(
        problem,
        execution,
        hybrid,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(components(point.corrected), components(baseline.corrected));
    assert_eq!(
        components(point.numerical_mcse),
        components(baseline.numerical_mcse)
    );
    assert_eq!(
        point.receipt.execution.counter,
        baseline.receipt.execution.counter
    );
    assert_eq!(point.receipt.rhs.len(), baseline.receipt.rhs.len());
    assert_eq!(
        point.receipt.execution.selected_route,
        baseline.receipt.execution.selected_route
    );
    assert_eq!(
        attachment.replay_rhs.len(),
        execution.estimator.probes as usize
    );
    assert!(matches!(
        attachment.covariance.status,
        Status::OkLocal | Status::OkLocalPsdAdjusted | Status::UnstableNonPsd
    ));
    assert!(attachment
        .covariance
        .raw
        .iter()
        .flatten()
        .all(|v| v.is_finite()));
    for receipt in &attachment.replay_rhs {
        assert!(receipt.complete_residual <= point.receipt.full_residual_tolerance);
    }
    let conditional = attachment.covariance.conditional;
    let mcse = components(point.numerical_mcse);
    for i in 0..3 {
        assert!((conditional[i][i] - mcse[i] * mcse[i]).abs() < 1e-13);
    }
    let total = conditional[0][0]
        + conditional[1][1]
        + 4.0 * conditional[2][2]
        + 2.0 * conditional[0][1]
        + 4.0 * conditional[0][2]
        + 4.0 * conditional[1][2];
    assert!((total - mcse[3] * mcse[3]).abs() < 1e-13);
    for mean in attachment.replay_score_means.into_iter().flatten() {
        assert!(mean.abs() < 1e-10);
    }
    assert!(point.receipt.peak_forecast_bytes > baseline.receipt.peak_forecast_bytes);
}

#[test]
fn generic_all_probe_preserves_point_conditional_rng_and_original_residuals() {
    for controls in [false, true] {
        for deletion in [DeletionMode::Observation, DeletionMode::Match] {
            for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
                for route in [ModelSolverRoute::Diagonal, ModelSolverRoute::Cmg] {
                    let problem = fixture(controls);
                    let mut opts = options(deletion, nuisance);
                    opts.probes = 33; // odd logical parity, partial point batches
                    check_point_and_attachment(&problem, routed_options(opts, route), None);
                }
            }
        }
    }
}

#[test]
fn mixed_stayer_all_probe_uses_one_global_direction() {
    let mover = fixture(true);
    let prepared = prepare_exact_stayer_hybrid(
        &mover,
        StayerAugmentationInput {
            firm: vec![1, 1, 2, 2],
            worker: vec![1, 1, 2, 2],
            outcome: vec![0.4, 0.9, -0.2, 0.35],
            frequency: vec![1, 2, 1, 2],
            target_weight: vec![0.7, 1.1, 0.6, 0.9],
            controls: vec![vec![-0.6, 0.8, 0.2, -0.4], vec![0.3, -0.5, 0.9, -0.1]],
        },
    )
    .unwrap();
    for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
        let mut opts = options(DeletionMode::Match, nuisance);
        opts.probes = 33;
        check_point_and_attachment(
            &prepared.problem,
            routed_options(opts, ModelSolverRoute::Diagonal),
            Some(&prepared.plan),
        );
    }
}

#[test]
fn compressed_attachment_preserves_its_point_route_and_matches_generic_overlap() {
    use vckss_core::engine::run_jla_no_controls_with_numerical_mc_interrupt;
    let problem = fixture(false);
    for route in [
        LinearSolverRoute::Exact,
        LinearSolverRoute::DiagonalPcg,
        LinearSolverRoute::CmgPcg,
    ] {
        let request = PlannedJlaEngineOptions {
            estimator: JlaEngineOptions {
                seed: 0x55aa_1122_3344_7788,
                probes: 33,
                leverage_batch_width: 7,
                target_batch_width: 5,
                solver: LinearSolverOptions {
                    route,
                    full_residual_tolerance: 1e-11,
                    pcg: PcgOptions {
                        tolerance: 1e-12,
                        maximum_iterations: 5000,
                        residual_replacement_interval: 17,
                    },
                    ..LinearSolverOptions::default()
                },
                ..JlaEngineOptions::default()
            },
            leverage_batch: BatchRequest::Explicit(7),
            target_batch: BatchRequest::Explicit(5),
            ..PlannedJlaEngineOptions::default()
        };
        let baseline = run_jla_no_controls_planned(&problem, request).unwrap();
        let (point, diagnostic) =
            run_jla_no_controls_with_numerical_mc_interrupt(&problem, request, &mut NeverInterrupt)
                .unwrap();
        assert_eq!(point.estimator.corrected, baseline.estimator.corrected);
        assert_eq!(
            point.estimator.numerical_mcse,
            baseline.estimator.numerical_mcse
        );
        assert_eq!(point.execution.counter, baseline.execution.counter);
        assert_eq!(
            point.execution.selected_solver_route,
            baseline.execution.selected_solver_route
        );
        assert_eq!(diagnostic.replay_rhs.len(), 33);
        let mut opts = options(DeletionMode::Match, NuisanceMode::Joint);
        opts.probes = 33;
        let (_, generic) = run_generic_jla_with_numerical_mc_interrupt(
            &problem,
            routed_options(opts, ModelSolverRoute::Diagonal),
            None,
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_matrix_relative(
            diagnostic.covariance.conditional,
            generic.covariance.conditional,
        );
        assert_matrix_relative(diagnostic.covariance.leverage, generic.covariance.leverage);
        assert_matrix_relative(diagnostic.covariance.raw, generic.covariance.raw);
    }
}

struct BreakReplay;
impl InterruptCheck for BreakReplay {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == "numerical_mc_replay" {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "test replay break",
            ))
        } else {
            Ok(())
        }
    }
}

struct FailReplay {
    armed: bool,
    code: ErrorCode,
}
impl InterruptCheck for FailReplay {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == "numerical_mc_replay" {
            self.armed = true;
        }
        if self.armed && phase == "model_solver_batch" {
            Err(BackendError::new(
                self.code,
                phase,
                "injected uncertified replay",
            ))
        } else {
            Ok(())
        }
    }
}

#[test]
fn uncertified_replay_withholds_only_diagnostic_and_keeps_valid_point() {
    let problem = fixture(false);
    let mut opts = options(DeletionMode::Observation, NuisanceMode::Joint);
    opts.probes = 17;
    let request = routed_options(opts, ModelSolverRoute::Diagonal);
    let base = run_generic_jla_routed(&problem, request).unwrap();
    for code in [
        ErrorCode::FullResidualFailed,
        ErrorCode::PcgMaxIterations,
        ErrorCode::PcgStagnation,
        ErrorCode::PcgCurvatureBreakdown,
        ErrorCode::PcgPreconditionerBreakdown,
    ] {
        let (point, diag) = run_generic_jla_with_numerical_mc_interrupt(
            &problem,
            request,
            None,
            &mut FailReplay { armed: false, code },
        )
        .unwrap();
        assert_eq!(point.corrected, base.corrected);
        assert_eq!(point.numerical_mcse, base.numerical_mcse);
        assert_eq!(diag.covariance.status, Status::ReplayFailed);
        assert!(diag.covariance.usable.is_none());
        assert_eq!(diag.replay_error.unwrap().code, code);
        assert!(diag.replay_rhs.is_empty());
        assert_eq!(diag.failed_replay_probe, Some(0));
    }
    for code in [
        ErrorCode::AllocationFailed,
        ErrorCode::InternalInvariantFailed,
        ErrorCode::UserBreak,
    ] {
        assert_eq!(
            run_generic_jla_with_numerical_mc_interrupt(
                &problem,
                request,
                None,
                &mut FailReplay { armed: false, code }
            )
            .unwrap_err()
            .code,
            code
        );
    }
    run_generic_jla_with_numerical_mc_interrupt(&problem, request, None, &mut NeverInterrupt)
        .unwrap();
}

#[test]
fn combined_memory_boundary_rejects_before_point_atoms_and_replay_break_reuses() {
    let problem = fixture(false);
    let mut opts = options(DeletionMode::Observation, NuisanceMode::Joint);
    opts.probes = 17;
    let execution = routed_options(opts, ModelSolverRoute::Diagonal);
    let (point, _) =
        run_generic_jla_with_numerical_mc_interrupt(&problem, execution, None, &mut NeverInterrupt)
            .unwrap();
    let peak = point.receipt.peak_forecast_bytes;
    let mut exact = execution;
    exact.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: peak,
        check: MemoryCheck::Error,
    };
    run_generic_jla_with_numerical_mc_interrupt(&problem, exact, None, &mut NeverInterrupt)
        .unwrap();
    exact.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: peak - 1,
        check: MemoryCheck::Error,
    };
    assert_eq!(
        run_generic_jla_with_numerical_mc_interrupt(&problem, exact, None, &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        run_generic_jla_with_numerical_mc_interrupt(&problem, execution, None, &mut BreakReplay)
            .unwrap_err()
            .code,
        ErrorCode::UserBreak
    );
    run_generic_jla_with_numerical_mc_interrupt(&problem, execution, None, &mut NeverInterrupt)
        .unwrap();
}

#[test]
fn numerical_export_overlap_and_compressed_prepared_sum_set_strict_boundaries() {
    use vckss_core::engine::run_jla_no_controls_with_numerical_mc_interrupt;
    let problem = fixture(false);
    let mut opts = options(DeletionMode::Match, NuisanceMode::Joint);
    opts.probes = 33;
    opts.rhs_export_bytes = 10_000_000;
    let execution = routed_options(opts, ModelSolverRoute::Diagonal);
    let baseline = run_generic_jla_routed(&problem, execution).unwrap();
    let (point, _) =
        run_generic_jla_with_numerical_mc_interrupt(&problem, execution, None, &mut NeverInterrupt)
            .unwrap();
    assert!(
        point.receipt.result_export_peak_forecast_bytes
            > baseline.receipt.result_export_peak_forecast_bytes
    );
    assert_eq!(
        point.receipt.peak_forecast_bytes,
        point.receipt.result_export_peak_forecast_bytes
    );
    let mut strict = execution;
    strict.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: baseline.receipt.peak_forecast_bytes,
        check: MemoryCheck::Error,
    };
    assert_eq!(
        run_generic_jla_with_numerical_mc_interrupt(&problem, strict, None, &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    strict.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: point.receipt.peak_forecast_bytes,
        check: MemoryCheck::Error,
    };
    run_generic_jla_with_numerical_mc_interrupt(&problem, strict, None, &mut NeverInterrupt)
        .unwrap();
    for check in [MemoryCheck::Warn, MemoryCheck::Off] {
        strict.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: baseline.receipt.peak_forecast_bytes,
            check,
        };
        let (warned, _) = run_generic_jla_with_numerical_mc_interrupt(
            &problem,
            strict,
            None,
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(warned.corrected, point.corrected);
        assert_eq!(
            warned.receipt.execution.counter,
            point.receipt.execution.counter
        );
    }

    let request = PlannedJlaEngineOptions {
        estimator: JlaEngineOptions {
            probes: 33,
            ..JlaEngineOptions::default()
        },
        leverage_batch: BatchRequest::Explicit(3),
        target_batch: BatchRequest::Explicit(5),
        ..PlannedJlaEngineOptions::default()
    };
    let baseline = run_jla_no_controls_planned(&problem, request).unwrap();
    let (point, numerical) =
        run_jla_no_controls_with_numerical_mc_interrupt(&problem, request, &mut NeverInterrupt)
            .unwrap();
    assert_eq!(
        point.estimator.receipt.memory.prepared_persistent_bytes,
        baseline.estimator.receipt.memory.prepared_persistent_bytes
    );
    assert!(
        point.estimator.receipt.memory.solve_peak_forecast_bytes
            >= baseline.estimator.receipt.memory.solve_peak_forecast_bytes
                + numerical.allocation_bound_bytes
    );
    let mut strict = request;
    strict.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: baseline.estimator.receipt.memory.solve_peak_forecast_bytes,
        check: MemoryCheck::Error,
    };
    assert_eq!(
        run_jla_no_controls_with_numerical_mc_interrupt(&problem, strict, &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    for check in [MemoryCheck::Warn, MemoryCheck::Off] {
        strict.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: baseline.estimator.receipt.memory.solve_peak_forecast_bytes,
            check,
        };
        let (warned, _) =
            run_jla_no_controls_with_numerical_mc_interrupt(&problem, strict, &mut NeverInterrupt)
                .unwrap();
        assert_eq!(warned.estimator.corrected, point.estimator.corrected);
        assert_eq!(warned.execution.counter, point.execution.counter);
    }
}
