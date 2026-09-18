// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use vckss_core::component_inference::{
    ComponentQ1TargetResult, ComponentSpectrumDiagnostics, PreparedComponentInference,
};
use vckss_core::generic_jla::run_generic_jla_with_diagonal_queue_attachments_interrupt as queued;
use vckss_core::memory::{MemoryBudget, MemoryCheck};

fn request(unit: ComponentInferenceUnit) -> GenericJlaExecutionOptions {
    let mut estimator = match unit {
        ComponentInferenceUnit::Observation => {
            options(DeletionMode::Observation, NuisanceMode::Joint)
        }
        ComponentInferenceUnit::Match => options(DeletionMode::Match, NuisanceMode::FixedOffset),
    };
    estimator.probes = 33;
    estimator.memory_budget = MemoryBudget::Unspecified;
    estimator.leverage_batch_width = 3;
    estimator.target_batch_width = 5;
    routed_options(estimator, ModelSolverRoute::Diagonal)
}

pub(super) fn inference_options() -> ComponentInferenceOptions {
    ComponentInferenceOptions {
        seed: 0x4d61_7463_6851_3101,
        probes: 129,
        spectrum_probes: 17,
        spectrum_iterations: 16,
        batch_width: 7,
        ..ComponentInferenceOptions::default()
    }
}

fn floats(left: impl IntoIterator<Item = f64>, right: impl IntoIterator<Item = f64>) {
    let left: Vec<_> = left.into_iter().collect();
    let right: Vec<_> = right.into_iter().collect();
    assert_eq!(left.len(), right.len());
    for (a, b) in left.into_iter().zip(right) {
        if a.is_nan() || b.is_nan() {
            assert!(a.is_nan() && b.is_nan(), "availability changed: {a} {b}");
        } else if a.is_infinite() || b.is_infinite() {
            assert_eq!(a, b);
        } else {
            assert_close(a, b, 1e-8);
        }
    }
}

fn spectrum(x: &ComponentSpectrumDiagnostics) -> [f64; 13] {
    [
        x.leading_eigenvalue,
        x.second_eigenvalue,
        x.trace_square_raw,
        x.trace_square,
        x.trace_square_mcse,
        x.trace_reconciliation,
        x.leading_share,
        x.leading_share_mcse_trace_only,
        x.remainder_leading_share,
        x.maximum_mode_weight_squared,
        x.leading_residual,
        x.second_residual,
        x.iterations as f64,
    ]
}

fn q1(x: &ComponentQ1TargetResult) -> [f64; 21] {
    [
        x.point_estimate,
        x.leading_score,
        x.leading_variance_correction,
        x.leading_variance,
        x.leading_recentered_component,
        x.remainder_estimate,
        x.remainder_identity_error,
        x.leading_remainder_covariance,
        x.remainder_variance,
        x.remainder_trace_mcse,
        x.standardized_determinant,
        x.remainder_influence_variance,
        x.remainder_trace_variance,
        x.curvature,
        x.critical_value,
        x.confidence_lower,
        x.confidence_upper,
        x.leading_f_statistic,
        x.remainder_influence_concentration,
        x.critical_draws as f64,
        x.status as u32 as f64,
    ]
}

pub(super) fn science(left: &GenericJlaResult, right: &GenericJlaResult) {
    science_impl(left, right, true);
}

pub(super) fn science_with_batch_order(left: &GenericJlaResult, right: &GenericJlaResult) {
    science_impl(left, right, false);
}

fn science_impl(left: &GenericJlaResult, right: &GenericJlaResult, same_batches: bool) {
    floats(components(left.corrected), components(right.corrected));
    floats(
        components(left.numerical_mcse),
        components(right.numerical_mcse),
    );
    assert_eq!(left.receipt.deletion_units, right.receipt.deletion_units);
    assert_eq!(left.receipt.target_strata, right.receipt.target_strata);
    assert_eq!(
        left.receipt.execution.counter,
        right.receipt.execution.counter
    );
    let a = left.component_inference.as_ref().unwrap();
    let b = right.component_inference.as_ref().unwrap();
    assert_eq!(a.counter_atoms, b.counter_atoms);
    assert_eq!(a.counter_words, b.counter_words);
    assert_eq!(a.inference_unit, b.inference_unit);
    assert_eq!(a.independent_units, b.independent_units);
    assert_eq!(a.joint_status, b.joint_status);
    assert_eq!(a.q0_status, b.q0_status);
    floats(a.covariance, b.covariance);
    floats(a.primitive_covariance, b.primitive_covariance);
    floats(a.trace_term, b.trace_term);
    floats(a.trace_mcse, b.trace_mcse);
    floats(a.influence_term, b.influence_term);
    floats(a.influence_concentration, b.influence_concentration);
    floats(a.leverage.iter().copied(), b.leverage.iter().copied());
    floats(
        a.maker_inverse.iter().copied(),
        b.maker_inverse.iter().copied(),
    );
    for (x, y) in a
        .target_diagonal
        .iter()
        .chain(&a.influence)
        .zip(b.target_diagonal.iter().chain(&b.influence))
    {
        floats(x.iter().copied(), y.iter().copied());
    }
    for (x, y) in a.spectrum.iter().zip(&b.spectrum) {
        assert_eq!(x.certified, y.certified);
        assert_eq!(x.probes, y.probes);
        floats(spectrum(x), spectrum(y));
    }
    assert_eq!(a.solve_receipts.len(), b.solve_receipts.len());
    let mut left_receipts: Vec<_> = a.solve_receipts.iter().collect();
    let mut right_receipts: Vec<_> = b.solve_receipts.iter().collect();
    if !same_batches {
        // Trace work stores each base batch followed by its four target
        // batches. Width changes alter receipt order, not the complete
        // (phase, logical probe) multiset, including repeated keys.
        left_receipts.sort_by_key(|r| (r.phase as u32, r.target_or_probe));
        right_receipts.sort_by_key(|r| (r.phase as u32, r.target_or_probe));
    }
    for (x, y) in left_receipts.into_iter().zip(right_receipts) {
        assert_eq!((x.phase, x.target_or_probe), (y.phase, y.target_or_probe));
        assert_eq!(x.full_residual_tolerance, y.full_residual_tolerance);
        assert!(y.complete_residual <= y.full_residual_tolerance);
    }
    assert_eq!(a.q1.is_some(), b.q1.is_some());
    if let (Some(x), Some(y)) = (&a.q1, &b.q1) {
        for (x, y) in x.iter().zip(y) {
            floats(q1(x), q1(y));
        }
    }
    if let (Some(x), Some(y)) = (&a.residual_moments, &b.residual_moments) {
        assert_eq!(x.ordering_contract, y.ordering_contract);
        assert_eq!(x.basis_columns, y.basis_columns);
        floats(x.gram.iter().copied(), y.gram.iter().copied());
        floats(
            x.fit.raw_variance.iter().copied(),
            y.fit.raw_variance.iter().copied(),
        );
        floats(
            x.fit.positive_variance.iter().copied(),
            y.fit.positive_variance.iter().copied(),
        );
        assert_eq!(x.projections.len(), y.projections.len());
        for (x, y) in x.projections.iter().zip(&y.projections) {
            assert_eq!(x.probe, y.probe);
            assert_eq!(x.full_residual_tolerance, y.full_residual_tolerance);
            assert!(y.complete_residual <= y.full_residual_tolerance);
        }
    } else {
        assert_eq!(a.residual_moments.is_some(), b.residual_moments.is_some());
    }
    if let (Some(x), Some(y)) = (&a.structured_variance, &b.structured_variance) {
        assert_eq!(x.outer_fold, y.outer_fold);
        assert_eq!(
            (x.counter_atoms, x.counter_words),
            (y.counter_atoms, y.counter_words)
        );
        floats(x.common.iter().copied(), y.common.iter().copied());
        floats(
            x.leverage_only.iter().copied(),
            y.leverage_only.iter().copied(),
        );
        assert_eq!(x.folds.len(), y.folds.len());
        for (x, y) in x.folds.iter().zip(&y.folds) {
            assert_eq!(
                (x.model, x.outer_fold, x.active_terms, x.floored_predictions),
                (y.model, y.outer_fold, y.active_terms, y.floored_predictions)
            );
            assert_eq!(x.selected_lambda, y.selected_lambda);
        }
    } else {
        assert_eq!(
            a.structured_variance.is_some(),
            b.structured_variance.is_some()
        );
    }
    assert!(b.maximum_complete_residual <= b.full_residual_tolerance);
}

fn equivalent(left: &GenericJlaResult, right: &GenericJlaResult) {
    science(left, right);
    let b = right.component_inference.as_ref().unwrap();
    let execution = &right.receipt.execution;
    let queue = execution.diagonal_queue.unwrap();
    assert_eq!(execution.selected_route, ModelSolverRoute::Diagonal);
    assert!(execution.full_cmg.is_none());
    assert_eq!(execution.memory.peak_bytes, queue.plan.command_peak_bytes);
    assert_eq!(
        execution.batch.plan.selected_command_peak_bytes,
        execution.memory.peak_bytes
    );
    assert_eq!(
        queue.work.completed_rhs,
        3 * 33
            + right.receipt.control_rank.projection_rhs.len()
            + b.solve_receipts.len()
            + b.residual_moments
                .as_ref()
                .map_or(0, |fit| fit.projections.len())
    );
    assert!(queue.work.maximum_active_workers <= queue.plan.workers);
    assert!(execution.plan_frozen_before_rng);
    assert_eq!(execution.counter_atoms_before_plan_freeze, 0);
}

fn run(
    problem: &CompressedProblem,
    options: GenericJlaExecutionOptions,
    prepared: &PreparedComponentInference,
    threads: usize,
) -> Result<GenericJlaResult> {
    queued(
        problem,
        options,
        None,
        Some(prepared),
        None,
        threads,
        &mut NeverInterrupt,
    )
}

fn reference(
    problem: &CompressedProblem,
    options: GenericJlaExecutionOptions,
    prepared: &PreparedComponentInference,
) -> GenericJlaResult {
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        problem,
        options,
        None,
        Some(prepared),
        None,
        &mut NeverInterrupt,
    )
    .unwrap()
}

#[test]
fn queued_component_oracle_thread_matrix_and_partial_groups() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            component_inference_fixture(true)
        } else {
            grouped_component_inference_fixture(7)
        };
        let options = request(unit);
        let mut prepared = if unit == ComponentInferenceUnit::Observation {
            prepare_oracle_component_inference(
                &problem,
                &vec![0.04; problem.outcome.len()],
                inference_options(),
            )
        } else {
            prepare_grouped_oracle_component_inference(
                &problem,
                &vec![0.04; problem.deletion_units()],
                inference_options(),
            )
        }
        .unwrap();
        // Availability is compared target by target, not conditioned on a
        // positive joint covariance in this small engineering fixture.
        prepared.individual_intervals = true;
        let baseline = reference(&problem, options, &prepared);
        for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
            equivalent(
                &baseline,
                &run(&problem, options, &prepared, threads).unwrap(),
            );
        }
        let mut tiny = options;
        tiny.leverage_batch = BatchRequest::Explicit(1);
        tiny.target_batch = BatchRequest::Explicit(1);
        let candidate = run(&problem, tiny, &prepared, 7).unwrap();
        assert_eq!(
            candidate
                .receipt
                .execution
                .diagonal_queue
                .unwrap()
                .plan
                .maximum_rhs,
            2
        );
        equivalent(&baseline, &candidate);
    }
}

#[test]
fn queued_component_q1_preserves_computed_and_unavailable_targets() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            component_inference_fixture(true)
        } else {
            grouped_component_inference_fixture(7)
        };
        let options = request(unit);
        for certified_fixture in [true, false] {
            let inference_options = ComponentInferenceOptions {
                probes: 2048,
                spectrum_probes: 128,
                spectrum_iterations: if certified_fixture { 96 } else { 2 },
                spectrum_tolerance: if certified_fixture { 1e-2 } else { 1e-10 },
                reference_distribution: ComponentReferenceDistribution::Q1,
                critical_simulations: 4000,
                ..inference_options()
            };
            let mut prepared = if unit == ComponentInferenceUnit::Observation {
                prepare_oracle_component_inference(
                    &problem,
                    &vec![0.04; problem.outcome.len()],
                    inference_options,
                )
            } else {
                prepare_grouped_oracle_component_inference(
                    &problem,
                    &vec![0.04; problem.deletion_units()],
                    inference_options,
                )
            }
            .unwrap();
            prepared.individual_intervals = true;
            let baseline = reference(&problem, options, &prepared);
            let targets = baseline.component_inference.as_ref().unwrap().q1.unwrap();
            if certified_fixture {
                assert!(targets.iter().any(|target| target.critical_draws > 0));
            } else {
                assert!(targets.iter().any(|target| target.critical_draws == 0));
            }
            for threads in [1, 7] {
                equivalent(
                    &baseline,
                    &run(&problem, options, &prepared, threads).unwrap(),
                );
            }
        }
    }
}

#[test]
fn queued_component_legacy_structured_fits_are_preserved() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
        let options = request(unit);
        for source in [
            ComponentVarianceSource::StructuredCommon,
            ComponentVarianceSource::StructuredLeverage,
        ] {
            let mut prepared = if unit == ComponentInferenceUnit::Observation {
                prepare_structured_component_inference(
                    &problem,
                    source,
                    inference_options(),
                    StructuredVarianceOptions::default(),
                )
            } else {
                prepare_grouped_structured_component_inference(
                    &problem,
                    source,
                    inference_options(),
                    StructuredVarianceOptions::default(),
                )
            }
            .unwrap();
            prepared.individual_intervals = true;
            let baseline = reference(&problem, options, &prepared);
            for threads in [1, 7] {
                equivalent(
                    &baseline,
                    &run(&problem, options, &prepared, threads).unwrap(),
                );
            }
        }
    }
}

#[test]
fn queued_component_direct_gram_both_units_budgets_and_breaks() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
        let mut options = request(unit);
        let prepared = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
            &problem,
            unit,
            ComponentVarianceSource::StructuredCommon,
            inference_options(),
            StructuredVarianceOptions::default(),
            513,
            &mut NeverInterrupt,
        )
        .unwrap();
        let baseline = reference(&problem, options, &prepared);
        let free = run(&problem, options, &prepared, 7).unwrap();
        equivalent(&baseline, &free);
        let fit = free
            .component_inference
            .as_ref()
            .unwrap()
            .residual_moments
            .as_ref()
            .unwrap();
        assert_eq!(fit.projections.len(), 513);
        for threads in [1, 4] {
            equivalent(
                &baseline,
                &run(&problem, options, &prepared, threads).unwrap(),
            );
        }
        let boundary = free.receipt.execution.memory.peak_bytes;
        for (bytes, passes) in [(boundary, true), (boundary - 1, false)] {
            options.estimator.memory_budget = MemoryBudget::Explicit {
                bytes,
                check: MemoryCheck::Error,
            };
            let result = run(&problem, options, &prepared, 7);
            if passes {
                equivalent(&free, &result.unwrap());
            } else {
                assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
            }
        }
        struct NoPool;
        impl InterruptCheck for NoPool {
            fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
                assert_ne!(
                    phase, "generic_diagonal_queue_admitted",
                    "under-budget call reached pool setup"
                );
                Ok(())
            }
        }
        let rejected = queued(
            &problem,
            options,
            None,
            Some(&prepared),
            None,
            7,
            &mut NoPool,
        )
        .unwrap_err();
        assert_eq!(rejected.code, ErrorCode::ResourceLimit);
        for check in [MemoryCheck::Warn, MemoryCheck::Off] {
            options.estimator.memory_budget = MemoryBudget::Explicit { bytes: 1, check };
            equivalent(&free, &run(&problem, options, &prepared, 7).unwrap());
        }
        // Automatic widths must jointly include inference and callback memory.
        options.leverage_batch = BatchRequest::Auto;
        options.target_batch = BatchRequest::Auto;
        let reduced = run(&problem, options, &prepared, 7).unwrap();
        assert_eq!(
            reduced
                .receipt
                .execution
                .diagonal_queue
                .unwrap()
                .plan
                .maximum_rhs,
            2
        );
        equivalent(&free, &reduced);
        options.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: reduced.receipt.execution.memory.peak_bytes,
            check: MemoryCheck::Error,
        };
        equivalent(&free, &run(&problem, options, &prepared, 7).unwrap());
        options.estimator.memory_budget = MemoryBudget::Unspecified;
        struct BreakAt(&'static str);
        impl InterruptCheck for BreakAt {
            fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
                if phase == self.0 {
                    Err(BackendError::new(
                        ErrorCode::UserBreak,
                        phase,
                        "inference test break",
                    ))
                } else {
                    Ok(())
                }
            }
        }
        for phase in [
            "generic_diagonal_queue_admitted",
            "residual_moment_projection_batch",
            "generic_jla_component_spectrum_trace_batch",
            "generic_jla_component_spectrum_iteration",
            "generic_jla_component_spectrum_pack",
            "generic_jla_component_spectrum_unpack",
            "generic_jla_component_covariance_batch",
        ] {
            let error = queued(
                &problem,
                options,
                None,
                Some(&prepared),
                None,
                7,
                &mut BreakAt(phase),
            )
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::UserBreak);
        }
        equivalent(&free, &run(&problem, options, &prepared, 7).unwrap());
        options.estimator.nuisance = if unit == ComponentInferenceUnit::Match {
            NuisanceMode::Joint
        } else {
            NuisanceMode::FixedOffset
        };
        assert_eq!(
            run(&problem, options, &prepared, 7).unwrap_err().code,
            ErrorCode::UnsupportedFeature
        );
    }
}
