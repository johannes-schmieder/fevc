// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use queued_component::{inference_options, science};
use vckss_core::component_inference::PreparedComponentInference;
use vckss_core::full_cmg::FullCmgPlanOptions;
use vckss_core::generic_jla::{
    run_generic_jla_with_automatic_component_batches_interrupt as automatic,
    run_generic_jla_with_direct_attachments_interrupt as direct,
    run_generic_jla_with_direct_solver_interrupt as ordinary, ComponentBatchPolicy,
};

#[test]
fn automatic_component_widths_preserve_literal_eight_and_all_threads() {
    use queued_component::science_with_batch_order as science;
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            component_inference_fixture(true)
        } else {
            grouped_component_inference_fixture(7)
        };
        let prepared = oracle(
            &problem,
            unit,
            ComponentInferenceOptions {
                batch_width: 8,
                ..inference_options()
            },
        );
        let variance_pointer = prepared.variance.as_ptr();
        for cmg in [false, true] {
            let mut request = request(unit);
            request.routing.route = if cmg {
                ModelSolverRoute::Cmg
            } else {
                ModelSolverRoute::Diagonal
            };
            let baseline = reference(&problem, request, &prepared);
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                let result = automatic(
                    &problem,
                    request,
                    &prepared,
                    threads,
                    cmg.then(|| plan(threads)),
                    &mut NeverInterrupt,
                )
                .unwrap();
                science(&baseline, &result);
                let execution = &result.receipt.execution;
                let batch = execution.component_batch.unwrap();
                assert_eq!(batch.policy, ComponentBatchPolicy::Automatic);
                assert_eq!(
                    batch.selection_reason,
                    vckss_core::batch_plan::BatchSelectionReason::NoBudgetPerformanceChoice
                );
                assert_eq!(batch.declared_component_width, 8);
                assert_eq!(batch.component_width, 129.min(32.max(8 * threads)));
                assert_eq!(batch.gram_width, 0);
                assert_eq!(batch.permitted_threads, threads);
                assert!(execution.plan_frozen_before_rng);
                assert_eq!(execution.counter_atoms_before_plan_freeze, 0);
                assert_eq!(
                    execution.memory.peak_bytes,
                    execution.batch.plan.selected_command_peak_bytes
                );
                if cmg {
                    accounting(&result);
                } else {
                    let queue = execution.diagonal_queue.unwrap();
                    assert!(queue.plan.maximum_rhs >= batch.component_width);
                    assert_eq!(queue.plan.command_peak_bytes, execution.memory.peak_bytes);
                }
            }
        }
        let literal = run(&problem, request(unit), &prepared, 7).unwrap();
        let literal = literal.receipt.execution.component_batch.unwrap();
        assert_eq!(literal.policy, ComponentBatchPolicy::Literal);
        assert_eq!(literal.component_width, 8);
        assert_eq!(prepared.options.batch_width, 8);
        assert_eq!(prepared.variance.as_ptr(), variance_pointer);
    }
}

#[test]
fn automatic_component_gram_widths_budgets_breaks_and_reuse() {
    use queued_component::science_with_batch_order as science;
    struct NoPool;
    impl InterruptCheck for NoPool {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            assert!(!matches!(
                phase,
                "generic_diagonal_queue_admitted" | "generic_full_cmg_attachments_admitted"
            ));
            Ok(())
        }
    }
    struct BreakAt(&'static str);
    impl InterruptCheck for BreakAt {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.0 {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "automatic-batch test break",
                ))
            } else {
                Ok(())
            }
        }
    }
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
        let prepared = vckss_core::residual_moment_inference::prepare_direct_with_interrupt(
            &problem,
            unit,
            ComponentVarianceSource::StructuredCommon,
            ComponentInferenceOptions {
                batch_width: 8,
                ..inference_options()
            },
            StructuredVarianceOptions::default(),
            513,
            &mut NeverInterrupt,
        )
        .unwrap();
        for cmg in [false, true] {
            let mut request = request(unit);
            request.routing.route = if cmg {
                ModelSolverRoute::Cmg
            } else {
                ModelSolverRoute::Diagonal
            };
            let baseline = reference(&problem, request, &prepared);
            let run = |request, interrupt: &mut dyn InterruptCheck| {
                automatic(
                    &problem,
                    request,
                    &prepared,
                    7,
                    cmg.then(|| plan(7)),
                    interrupt,
                )
            };
            let free = run(request, &mut NeverInterrupt).unwrap();
            science(&baseline, &free);
            let receipt = free.receipt.execution.component_batch.unwrap();
            assert_eq!((receipt.component_width, receipt.gram_width), (56, 56));
            assert_eq!(
                (
                    receipt.declared_component_width,
                    receipt.declared_gram_width
                ),
                (8, 8)
            );
            for check in [MemoryCheck::Warn, MemoryCheck::Off] {
                request.estimator.memory_budget = MemoryBudget::Explicit { bytes: 1, check };
                let minimum = run(request, &mut NeverInterrupt).unwrap();
                science(&baseline, &minimum);
                let receipt = minimum.receipt.execution.component_batch.unwrap();
                assert_eq!((receipt.component_width, receipt.gram_width), (1, 1));
                assert_eq!(
                    receipt.selection_reason,
                    vckss_core::batch_plan::BatchSelectionReason::MinimumMemoryOverBudget
                );
                let boundary = minimum.receipt.execution.memory.peak_bytes;
                request.estimator.memory_budget = MemoryBudget::Explicit {
                    bytes: boundary,
                    check: MemoryCheck::Error,
                };
                let exact = run(request, &mut NeverInterrupt).unwrap();
                science(&baseline, &exact);
                assert!(exact.receipt.execution.memory.peak_bytes <= boundary);
                assert_eq!(
                    exact
                        .receipt
                        .execution
                        .component_batch
                        .unwrap()
                        .selection_reason,
                    vckss_core::batch_plan::BatchSelectionReason::LargestAdmissibleCandidate
                );
                let intermediate =
                    boundary + (free.receipt.execution.memory.peak_bytes - boundary) / 2;
                request.estimator.memory_budget = MemoryBudget::Explicit {
                    bytes: intermediate,
                    check: MemoryCheck::Error,
                };
                let reduced = run(request, &mut NeverInterrupt).unwrap();
                science(&baseline, &reduced);
                assert!(reduced.receipt.execution.memory.peak_bytes <= intermediate);
                assert!(
                    reduced.receipt.execution.memory.peak_bytes
                        < free.receipt.execution.memory.peak_bytes
                );
                request.estimator.memory_budget = MemoryBudget::Explicit {
                    bytes: boundary - 1,
                    check: MemoryCheck::Error,
                };
                assert_eq!(
                    run(request, &mut NoPool).unwrap_err().code,
                    ErrorCode::ResourceLimit
                );
            }
            request.estimator.memory_budget = MemoryBudget::Unspecified;
            for phase in [
                if cmg {
                    "generic_full_cmg_attachments_admitted"
                } else {
                    "generic_diagonal_queue_admitted"
                },
                "residual_moment_projection_batch",
                "residual_moment_gaussian",
                "residual_moment_rhs",
                "residual_moment_prediction",
                "residual_moment_reduce",
                "generic_jla_component_spectrum_trace_batch",
                "generic_jla_component_covariance_batch",
            ] {
                assert_eq!(
                    run(request, &mut BreakAt(phase)).unwrap_err().code,
                    ErrorCode::UserBreak
                );
                science(&baseline, &run(request, &mut NeverInterrupt).unwrap());
            }
            // Automatic inference planning must not rewrite either explicit
            // point width; full CMG continues to reject explicit point batches.
            request.leverage_batch = BatchRequest::Explicit(7);
            request.target_batch = BatchRequest::Explicit(8);
            let result = run(request, &mut NeverInterrupt);
            if cmg {
                assert_eq!(result.unwrap_err().code, ErrorCode::UnsupportedFeature);
            } else {
                let result = result.unwrap();
                science(&baseline, &result);
                assert_eq!(result.receipt.execution.batch.leverage_active_width, 7);
                assert_eq!(result.receipt.execution.batch.target_active_width, 8);
                request.estimator.memory_budget = MemoryBudget::Explicit {
                    bytes: 1,
                    check: MemoryCheck::Warn,
                };
                let result = run(request, &mut NeverInterrupt).unwrap();
                science(&baseline, &result);
                assert_eq!(result.receipt.execution.batch.leverage_active_width, 7);
                assert_eq!(result.receipt.execution.batch.target_active_width, 8);
                assert_eq!(
                    result
                        .receipt
                        .execution
                        .component_batch
                        .unwrap()
                        .component_width,
                    1
                );
            }
        }
    }
}
use vckss_core::memory::{MemoryBudget, MemoryCheck};

fn request(unit: ComponentInferenceUnit) -> GenericJlaExecutionOptions {
    let (deletion, nuisance) = match unit {
        ComponentInferenceUnit::Observation => (DeletionMode::Observation, NuisanceMode::Joint),
        ComponentInferenceUnit::Match => (DeletionMode::Match, NuisanceMode::FixedOffset),
    };
    let mut estimator = options(deletion, nuisance);
    estimator.probes = 33;
    estimator.memory_budget = MemoryBudget::Unspecified;
    let mut result = routed_options(estimator, ModelSolverRoute::Cmg);
    result.leverage_batch = BatchRequest::Auto;
    result.target_batch = BatchRequest::Auto;
    result
}

fn plan(threads: usize) -> FullCmgPlanOptions {
    // Deliberately leave the loose point-only defaults supplied. Attachments
    // must select the existing strict model options even with zero controls.
    FullCmgPlanOptions::production(threads, 1e-10, None)
}

fn run(
    problem: &CompressedProblem,
    request: GenericJlaExecutionOptions,
    prepared: &PreparedComponentInference,
    threads: usize,
) -> Result<GenericJlaResult> {
    direct(
        problem,
        request,
        None,
        Some(prepared),
        None,
        plan(threads),
        &mut NeverInterrupt,
    )
}

fn reference(
    problem: &CompressedProblem,
    request: GenericJlaExecutionOptions,
    prepared: &PreparedComponentInference,
) -> GenericJlaResult {
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        problem,
        request,
        None,
        Some(prepared),
        None,
        &mut NeverInterrupt,
    )
    .unwrap()
}

fn accounting(result: &GenericJlaResult) {
    let execution = &result.receipt.execution;
    let cmg = execution.full_cmg.as_ref().unwrap();
    let work = execution.direct_attachments.unwrap();
    assert!(execution.diagonal_queue.is_none());
    assert_eq!(execution.selected_route, ModelSolverRoute::Cmg);
    assert_eq!(cmg.setup.admitted_peak_bytes, execution.memory.peak_bytes);
    assert_eq!(
        execution.batch.plan.selected_command_peak_bytes,
        execution.memory.peak_bytes
    );
    assert_eq!(
        work.logical_rhs,
        work.fit_rhs
            + work.control_projection_rhs
            + work.point_probe_rhs
            + work.projection_rhs
            + work.component_rhs
            + work.gram_rhs
    );
    assert_eq!(
        cmg.rhs_count,
        work.logical_rhs as u64 + work.control_refinement_rhs
    );
    assert_eq!(
        cmg.model_diagnostics.explicit_options_rhs_count,
        work.control_projection_rhs as u64
    );
    assert_eq!(cmg.setup.probe_effective_tolerance, 1e-12);
    assert!(result.receipt.maximum_complete_residual <= result.receipt.full_residual_tolerance);
    assert!(execution.plan_frozen_before_rng);
    assert_eq!(execution.counter_atoms_before_plan_freeze, 0);
}

fn oracle(
    problem: &CompressedProblem,
    unit: ComponentInferenceUnit,
    options: ComponentInferenceOptions,
) -> PreparedComponentInference {
    let mut prepared = match unit {
        ComponentInferenceUnit::Observation => {
            prepare_oracle_component_inference(problem, &vec![0.04; problem.outcome.len()], options)
        }
        ComponentInferenceUnit::Match => prepare_grouped_oracle_component_inference(
            problem,
            &vec![0.04; problem.deletion_units()],
            options,
        ),
    }
    .unwrap();
    prepared.individual_intervals = true;
    prepared
}

#[test]
fn direct_component_oracle_threads_and_strict_options() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        for controls in [false, true] {
            let mut problem = if unit == ComponentInferenceUnit::Observation {
                component_inference_fixture(controls)
            } else {
                grouped_component_inference_fixture(7)
            };
            if !controls {
                problem.controls.clear();
            }
            let request = request(unit);
            let prepared = oracle(&problem, unit, inference_options());
            let baseline = reference(&problem, request, &prepared);
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                let candidate = run(&problem, request, &prepared, threads).unwrap();
                if baseline.component_inference.as_ref().unwrap().q0_status
                    != candidate.component_inference.as_ref().unwrap().q0_status
                {
                    eprintln!("status mismatch unit={unit:?} controls={controls} threads={threads}\nbaseline={:?}\ncandidate={:?}",
                        baseline.component_inference.as_ref().unwrap().spectrum,
                        candidate.component_inference.as_ref().unwrap().spectrum);
                }
                science(&baseline, &candidate);
                accounting(&candidate);
                assert_eq!(candidate.receipt.execution.threads.requested, threads);
                if controls {
                    assert_eq!(
                        candidate.receipt.control_rank.projection_pcg_tolerance,
                        1e-13
                    );
                    assert!(candidate.receipt.control_rank.maximum_projection_residual <= 1e-11);
                }
            }
            // An old point-only caller must still reject, not silently opt in.
            assert_eq!(
                ordinary(
                    &problem,
                    request,
                    None,
                    Some(&prepared),
                    None,
                    Some(plan(4)),
                    &mut NeverInterrupt
                )
                .unwrap_err()
                .code,
                ErrorCode::UnsupportedFeature
            );
        }
    }
}

#[test]
fn direct_component_q1_and_structured_models() {
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            component_inference_fixture(true)
        } else {
            grouped_component_inference_fixture(7)
        };
        let request = request(unit);
        for certified in [true, false] {
            let prepared = oracle(
                &problem,
                unit,
                ComponentInferenceOptions {
                    probes: 2048,
                    spectrum_probes: 128,
                    spectrum_iterations: if certified { 96 } else { 2 },
                    spectrum_tolerance: if certified { 1e-2 } else { 1e-10 },
                    reference_distribution: ComponentReferenceDistribution::Q1,
                    critical_simulations: 4000,
                    ..inference_options()
                },
            );
            let baseline = reference(&problem, request, &prepared);
            assert!(baseline
                .component_inference
                .as_ref()
                .unwrap()
                .q1
                .unwrap()
                .iter()
                .any(|target| (target.critical_draws > 0) == certified));
            for threads in [1, 7] {
                let result = run(&problem, request, &prepared, threads).unwrap();
                science(&baseline, &result);
                accounting(&result);
                let automatic = automatic(
                    &problem,
                    request,
                    &prepared,
                    threads,
                    Some(plan(threads)),
                    &mut NeverInterrupt,
                )
                .unwrap();
                queued_component::science_with_batch_order(&baseline, &automatic);
                accounting(&automatic);
                let mut diagonal = request;
                diagonal.routing.route = ModelSolverRoute::Diagonal;
                let diagonal_reference = reference(&problem, diagonal, &prepared);
                let automatic =
                    automatic_component_diagonal(&problem, diagonal, &prepared, threads);
                queued_component::science_with_batch_order(&diagonal_reference, &automatic);
            }
        }
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
        for source in [
            ComponentVarianceSource::StructuredCommon,
            ComponentVarianceSource::StructuredLeverage,
        ] {
            let mut prepared = match unit {
                ComponentInferenceUnit::Observation => prepare_structured_component_inference(
                    &problem,
                    source,
                    inference_options(),
                    StructuredVarianceOptions::default(),
                ),
                ComponentInferenceUnit::Match => prepare_grouped_structured_component_inference(
                    &problem,
                    source,
                    inference_options(),
                    StructuredVarianceOptions::default(),
                ),
            }
            .unwrap();
            prepared.individual_intervals = true;
            let baseline = reference(&problem, request, &prepared);
            let result = run(&problem, request, &prepared, 7).unwrap();
            science(&baseline, &result);
            accounting(&result);
        }
    }
}

fn automatic_component_diagonal(
    problem: &CompressedProblem,
    request: GenericJlaExecutionOptions,
    prepared: &PreparedComponentInference,
    threads: usize,
) -> GenericJlaResult {
    automatic(
        problem,
        request,
        prepared,
        threads,
        None,
        &mut NeverInterrupt,
    )
    .unwrap()
}

#[test]
fn residual_statistical_executor_matches_scalar_for_every_thread_cap() {
    use vckss_core::interrupt::{CancellationInterrupt, CancellationToken};
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
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
        for cmg in [false, true] {
            let mut request = request(unit);
            request.routing.route = if cmg {
                ModelSolverRoute::Cmg
            } else {
                ModelSolverRoute::Diagonal
            };
            let baseline = reference(&problem, request, &prepared);
            // Legacy and direct CMG use different numerical solve kernels;
            // their contract is the unchanged scaled science tolerance.
            // Bitwise statistics comparisons require the same solve kernel.
            let same_kernel = automatic(
                &problem,
                request,
                &prepared,
                1,
                cmg.then(|| plan(1)),
                &mut CancellationInterrupt::new(CancellationToken::new()),
            )
            .unwrap();
            queued_component::science_with_batch_order(&baseline, &same_kernel);
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                let result = automatic(
                    &problem,
                    request,
                    &prepared,
                    threads,
                    cmg.then(|| plan(threads)),
                    &mut CancellationInterrupt::new(CancellationToken::new()),
                )
                .unwrap();
                queued_component::science_with_batch_order(&baseline, &result);
                let expected = same_kernel
                    .component_inference
                    .as_ref()
                    .unwrap()
                    .residual_moments
                    .as_ref()
                    .unwrap();
                let actual = result
                    .component_inference
                    .as_ref()
                    .unwrap()
                    .residual_moments
                    .as_ref()
                    .unwrap();
                assert_eq!(expected.gram, actual.gram);
                assert_eq!(expected.fit.positive_variance, actual.fit.positive_variance);
                assert_eq!(
                    expected.preparation.counter_atoms,
                    actual.preparation.counter_atoms
                );
                assert_eq!(
                    expected.preparation.counter_words,
                    actual.preparation.counter_words
                );
                if cmg {
                    accounting(&result);
                }
            }
        }
    }
}

#[test]
fn automatic_component_invalid_and_overflow_requests_stop_before_work() {
    struct NoWork;
    impl InterruptCheck for NoWork {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            panic!("invalid request reached {phase}")
        }
    }
    let problem = component_inference_fixture(true);
    let mut prepared = oracle(
        &problem,
        ComponentInferenceUnit::Observation,
        inference_options(),
    );
    let mut request = request(ComponentInferenceUnit::Observation);
    for (threads, code) in [
        (0, ErrorCode::InvalidInput),
        (usize::MAX, ErrorCode::ResourceLimit),
    ] {
        assert_eq!(
            automatic(
                &problem,
                request,
                &prepared,
                threads,
                Some(plan(threads)),
                &mut NoWork
            )
            .unwrap_err()
            .code,
            code
        );
    }
    assert_eq!(
        automatic(&problem, request, &prepared, 4, Some(plan(7)), &mut NoWork)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    request.routing.route = ModelSolverRoute::Auto;
    assert_eq!(
        automatic(&problem, request, &prepared, 4, Some(plan(4)), &mut NoWork)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    request.routing.route = ModelSolverRoute::Diagonal;
    prepared.options.probes = 0;
    assert_eq!(
        automatic(&problem, request, &prepared, 4, None, &mut NoWork)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn direct_component_residual_gram_admission_and_cancellation() {
    struct BreakAt(&'static str);
    impl InterruptCheck for BreakAt {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == self.0 {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "test cancellation",
                ))
            } else {
                Ok(())
            }
        }
    }
    struct NoPool;
    impl InterruptCheck for NoPool {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            assert_ne!(phase, "generic_full_cmg_attachments_admitted");
            Ok(())
        }
    }
    for unit in [
        ComponentInferenceUnit::Observation,
        ComponentInferenceUnit::Match,
    ] {
        let problem = if unit == ComponentInferenceUnit::Observation {
            structured_component_inference_fixture()
        } else {
            grouped_component_inference_fixture(14)
        };
        let mut request = request(unit);
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
        let baseline = reference(&problem, request, &prepared);
        for threads in [1, 4, 7] {
            let result = run(&problem, request, &prepared, threads).unwrap();
            science(&baseline, &result);
            accounting(&result);
            assert_eq!(
                result
                    .receipt
                    .execution
                    .direct_attachments
                    .unwrap()
                    .gram_rhs,
                513
            );
        }
        // Advisory tiny budget freezes the minimum capacity, two RHSs. The
        // seven-column Gram callbacks and 28-column trace groups must split.
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: 1,
            check: MemoryCheck::Warn,
        };
        let minimum = run(&problem, request, &prepared, 7).unwrap();
        science(&baseline, &minimum);
        assert_eq!(
            minimum
                .receipt
                .execution
                .full_cmg
                .as_ref()
                .unwrap()
                .setup
                .maximum_batch_rhs,
            2
        );
        let boundary = minimum.receipt.execution.memory.peak_bytes;
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: boundary,
            check: MemoryCheck::Error,
        };
        let accepted = run(&problem, request, &prepared, 7).unwrap();
        science(&baseline, &accepted);
        assert!(accepted.receipt.execution.memory.peak_bytes <= boundary);
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: boundary - 1,
            check: MemoryCheck::Error,
        };
        assert_eq!(
            direct(
                &problem,
                request,
                None,
                Some(&prepared),
                None,
                plan(7),
                &mut NoPool
            )
            .unwrap_err()
            .code,
            ErrorCode::ResourceLimit
        );
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: 1,
            check: MemoryCheck::Off,
        };
        science(&baseline, &run(&problem, request, &prepared, 7).unwrap());
        request.estimator.memory_budget = MemoryBudget::Unspecified;
        for phase in [
            "generic_full_cmg_attachments_admitted",
            "residual_moment_projection_batch",
            "generic_jla_component_spectrum_trace_batch",
            "generic_jla_component_spectrum_iteration",
            "generic_jla_component_spectrum_pack",
            "generic_jla_component_spectrum_rhs",
            "generic_jla_component_spectrum_predict",
            "generic_jla_component_spectrum_unpack",
            "generic_jla_component_covariance_batch",
        ] {
            let error = direct(
                &problem,
                request,
                None,
                Some(&prepared),
                None,
                plan(4),
                &mut BreakAt(phase),
            )
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::UserBreak, "{phase}");
            science(&baseline, &run(&problem, request, &prepared, 4).unwrap());
        }
        request.leverage_batch = BatchRequest::Explicit(8);
        assert_eq!(
            run(&problem, request, &prepared, 4).unwrap_err().code,
            ErrorCode::UnsupportedFeature
        );
        request.leverage_batch = BatchRequest::Auto;
        request.estimator.nuisance = if unit == ComponentInferenceUnit::Observation {
            NuisanceMode::FixedOffset
        } else {
            NuisanceMode::Joint
        };
        assert_eq!(
            run(&problem, request, &prepared, 4).unwrap_err().code,
            ErrorCode::UnsupportedFeature
        );
    }
}
