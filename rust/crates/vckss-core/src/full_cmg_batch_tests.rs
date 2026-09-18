// SPDX-License-Identifier: GPL-3.0-only
//! Composed regressions: a correct cap alone does not prove the executed width.

#[test]
fn selected_capacity_tracks_budget_reductions_before_any_solve() {
    let problem = fixture(false, None);
    let free = run_jla_no_controls_planned(&problem, options(200, 64)).unwrap();
    let peak = free.execution.memory.solve_peak_forecast_bytes;
    let mut reduced = false;
    for step in 1..20 {
        let mut request = options(200, 64);
        request.estimator.memory_budget = MemoryBudget::Explicit {
            bytes: peak * (100 - step * 4) / 100,
            check: MemoryCheck::Error,
        };
        match run_jla_no_controls_planned(&problem, request) {
            Ok(result) => {
                let batch = result.execution.batch;
                let setup = result.execution.full_cmg.as_ref().unwrap().setup;
                assert_eq!(
                    setup.maximum_batch_rhs,
                    batch
                        .leverage_active_width
                        .max(2 * batch.target_active_width)
                );
                assert_eq!(setup.workspace_count, setup.maximum_batch_rhs.min(64));
                assert!(
                    result.execution.memory.solve_peak_forecast_bytes
                        <= request.estimator.memory_budget.limit(0).unwrap()
                );
                assert_eq!(
                    result.execution.memory.solve_peak_forecast_bytes,
                    batch.plan.selected_command_peak_bytes
                );
                reduced |= setup.maximum_batch_rhs
                    < free
                        .execution
                        .full_cmg
                        .as_ref()
                        .unwrap()
                        .setup
                        .maximum_batch_rhs;
            }
            Err(error) => assert_eq!(error.code, ErrorCode::ResourceLimit),
        }
    }
    assert!(reduced, "fixture must actually exercise a reduced capacity");
}

#[test]
fn one_byte_budget_fails_before_pool_admission_and_rng() {
    struct Check {
        admitted: bool,
    }
    impl InterruptCheck for Check {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            self.admitted |= phase == "cmg_full_v2_pools_admitted";
            Ok(())
        }
    }
    let problem = fixture(false, None);
    let mut request = options(33, 7);
    request.estimator.memory_budget = MemoryBudget::Explicit {
        bytes: 1,
        check: MemoryCheck::Error,
    };
    let mut check = Check { admitted: false };
    let error =
        run_jla_no_controls_planned_with_interrupt(&problem, request, &mut check).unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert!(!check.admitted);
}
use super::*;
use crate::memory::{MemoryBudget, MemoryCheck};
use crate::problem::CanonicalInput;
use crate::types::InputColumns;

fn fixture(weighted: bool, fixed_degree: Option<usize>) -> CompressedProblem {
    let mut input = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![],
    };
    for worker in 0..112 {
        let degree = fixed_degree.unwrap_or(2 + worker % 7);
        for period in 0..degree {
            let firm = (worker + period * 3) % 16;
            let row = input.worker.len();
            input.worker.push(worker as u64 + 1);
            input.firm.push(firm as u64 + 1);
            input.deletion.push(row as u64 + 1);
            input.outcome.push(
                (worker % 11) as f64 * 0.3
                    + (firm % 7) as f64 * 0.2
                    + ((row * 17 + 3) % 23) as f64 * 0.1,
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
    let rows = input.worker.len();
    CanonicalInput::from_validated(input.validate().unwrap())
        .unwrap()
        .compress(&vec![true; rows])
        .unwrap()
}

fn options(probes: u32, threads: usize) -> PlannedJlaEngineOptions {
    PlannedJlaEngineOptions {
        estimator: JlaEngineOptions {
            probes,
            memory_budget: MemoryBudget::Unspecified,
            memory_limit_bytes: 0,
            solver: LinearSolverOptions {
                route: LinearSolverRoute::CmgPcg,
                ..Default::default()
            },
            ..Default::default()
        },
        leverage_batch: BatchRequest::Auto,
        target_batch: BatchRequest::Auto,
        wallseconds: None,
        full_cmg: Some(FullCmgPlanOptions::production(threads, 1e-10, None)),
    }
}

#[test]
fn desired_widths_survive_composed_planning_without_a_budget() {
    for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
        for probes in [1, 3, 5, 31, 32, 33, 63, 64, 65, 199, 200, 201] {
            for k in [1, 2] {
                let (leverage, target, _) =
                    crate::full_cmg_batch_policy::caps(probes, threads, k).unwrap();
                for width in [leverage, target] {
                    let receipt = crate::batch_plan::plan_full_cmg_batches_with_forecasts(
                        BatchRequest::Auto,
                        BatchRequest::Auto,
                        BatchPlannerCaps {
                            probes,
                            declared_threads: 1,
                            columns_per_thread: width,
                            route_width_cap: width,
                            non_batched_peak_bytes: 10,
                            hard_memory_bytes: 0,
                            memory_budget: MemoryBudget::Unspecified,
                        },
                        |w| Ok(10 + w as u64 * 8),
                        |w| Ok(10 + w as u64 * 16),
                    )
                    .unwrap();
                    assert_eq!(receipt.leverage.selected_width, width);
                    assert_eq!(receipt.target.selected_width, width);
                }
            }
        }
    }
}

#[test]
fn actual_estimator_widths_rhs_counts_and_hybrid_vertices_reconcile() {
    let problem = fixture(false, None);
    let mut cases = vec![];
    for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
        cases.push((200, threads));
    }
    for probes in [3, 5, 33, 65, 199, 201] {
        cases.push((probes, 7));
    }
    for (probes, threads) in cases {
        let result = run_jla_no_controls_planned(&problem, options(probes, threads)).unwrap();
        let (leverage, target, maximum) = crate::full_cmg_batch_policy::caps(
            probes as usize,
            threads,
            crate::full_cmg_batch_policy::SELECTED_K,
        )
        .unwrap();
        let execution = &result.execution;
        assert_eq!(execution.batch.leverage_active_width, leverage);
        assert_eq!(execution.batch.target_active_width, target);
        let cmg = execution.full_cmg.as_ref().unwrap();
        assert_eq!(cmg.setup.vertices, 16 + 64);
        assert_eq!(cmg.setup.maximum_batch_rhs, maximum);
        // Initial fit, leverage and target batches, plus separately counted refinements.
        assert_eq!(
            cmg.batch_calls - cmg.refinement_attempts,
            1 + (probes as u64).div_ceil(leverage as u64) + (probes as u64).div_ceil(target as u64)
        );
        assert_eq!(cmg.rhs_count, 1 + 3 * u64::from(probes));
        assert!(cmg.maximum_complete_residual <= 1e-5);
        assert!(execution.plan_frozen_before_rng);
        assert_eq!(execution.physical_trials_before_plan_freeze, 0);
        assert_eq!(
            execution.memory.solve_peak_forecast_bytes,
            execution.batch.plan.selected_command_peak_bytes
        );
    }
}

#[test]
fn budget_candidates_include_desired_width_and_preserve_warn_error_off() {
    for check in [MemoryCheck::Error, MemoryCheck::Warn, MemoryCheck::Off] {
        for (bytes, expected) in [(2240, 224), (2239, 128), (1280, 128), (1279, 64)] {
            let result = crate::batch_plan::plan_full_cmg_batches_with_forecasts(
                BatchRequest::Auto,
                BatchRequest::Auto,
                BatchPlannerCaps {
                    probes: 300,
                    declared_threads: 1,
                    columns_per_thread: 224,
                    route_width_cap: 224,
                    non_batched_peak_bytes: 1,
                    hard_memory_bytes: 0,
                    memory_budget: MemoryBudget::Explicit { bytes, check },
                },
                |w| Ok(w as u64 * 10),
                |w| Ok(w as u64 * 10),
            )
            .unwrap();
            assert_eq!(result.leverage.selected_width, expected);
            assert_eq!(result.target.selected_width, expected);
        }
    }
}

#[test]
fn heterogeneous_weights_and_each_degree_preserve_complete_solve_gates() {
    for degree in [
        None,
        Some(2),
        Some(3),
        Some(4),
        Some(5),
        Some(6),
        Some(7),
        Some(8),
    ] {
        let problem = fixture(true, degree);
        let result = run_jla_no_controls_planned(&problem, options(33, 3)).unwrap();
        let cmg = result.execution.full_cmg.unwrap();
        let auxiliaries = degree.map_or(64, |d| if d > 4 { 112 } else { 0 });
        assert_eq!(cmg.setup.vertices, 16 + auxiliaries);
        assert!(cmg.maximum_complete_residual <= 1e-5);
    }
}

#[test]
fn thread_aware_batches_preserve_corrected_targets_and_counter_work() {
    for weighted in [false, true] {
        let problem = fixture(weighted, None);
        let baseline = run_jla_no_controls_planned(&problem, options(65, 1)).unwrap();
        let a = baseline.estimator.corrected;
        for threads in [2, 3, 4, 7, 14, 28, 64] {
            let result = run_jla_no_controls_planned(&problem, options(65, threads)).unwrap();
            let b = result.estimator.corrected;
            for (left, right) in [
                (a.worker, b.worker),
                (a.firm, b.firm),
                (a.covariance, b.covariance),
                (a.total, b.total),
            ] {
                assert!((left - right).abs() <= 1e-8 * left.abs().max(right.abs()).max(1.0));
            }
            let receipt = &result.estimator.receipt;
            assert_eq!(receipt.seed, baseline.estimator.receipt.seed);
            assert_eq!(receipt.leverage_probes_accepted, 65);
            assert_eq!(receipt.target_probes_accepted, 65);
            assert_eq!(
                receipt.topology_checksum,
                baseline.estimator.receipt.topology_checksum
            );
            assert!(receipt.accounting_residual < 1e-10);
        }
    }
}

#[test]
fn cancellation_at_admitted_pool_boundary_precedes_rng_and_allows_reuse() {
    struct CancelAtPool;
    impl InterruptCheck for CancelAtPool {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == "cmg_full_v2_pools_admitted" {
                return Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "test cancellation",
                ));
            }
            assert!(!phase.starts_with("jla_rng"));
            Ok(())
        }
    }
    let problem = fixture(false, None);
    let error =
        run_jla_no_controls_planned_with_interrupt(&problem, options(33, 7), &mut CancelAtPool)
            .unwrap_err();
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert!(run_jla_no_controls_planned(&problem, options(33, 7)).is_ok());
}

#[test]
fn compressed_statistics_poll_inside_work_on_the_original_caller() {
    struct BreakInside {
        phase: &'static str,
        owner: std::thread::ThreadId,
    }
    impl InterruptCheck for BreakInside {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            assert_eq!(std::thread::current().id(), self.owner);
            if phase == self.phase {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "inside compressed work",
                ))
            } else {
                Ok(())
            }
        }
    }
    let problem = fixture(true, None);
    for phase in [
        "jla_leverage_rhs",
        "jla_leverage_moment_column",
        "jla_target_direction_strata",
        "jla_target_balance",
        "jla_target_prepare_copy",
        "jla_target_contraction",
    ] {
        let mut check = BreakInside {
            phase,
            owner: std::thread::current().id(),
        };
        let result =
            run_jla_no_controls_planned_with_interrupt(&problem, options(33, 7), &mut check);
        assert!(
            matches!(&result, Err(error) if error.code == ErrorCode::UserBreak),
            "phase={phase}, error={:?}",
            result.as_ref().err()
        );
        assert!(run_jla_no_controls_planned(&problem, options(33, 7)).is_ok());
    }
}

#[test]
fn retained_plan_is_reused_without_rng_memory_or_result_changes() {
    struct RejectBuild;
    impl InterruptCheck for RejectBuild {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            if phase == "jla_plan_entry" {
                Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "unexpected plan rebuild",
                ))
            } else {
                Ok(())
            }
        }
    }
    for weighted in [false, true] {
        let problem = fixture(weighted, None);
        let plan = JlaPlan::build_no_controls(&problem).unwrap();
        for threads in [1, 4, 7] {
            let baseline = run_jla_no_controls_planned(&problem, options(33, threads)).unwrap();
            let reused = run_jla_no_controls_planned_with_plan_and_interrupt(
                &problem,
                &plan,
                options(33, threads),
                &mut RejectBuild,
            )
            .unwrap();
            assert_eq!(
                format!("{:?}", baseline.estimator),
                format!("{:?}", reused.estimator)
            );
            assert_eq!(
                format!("{:?}", baseline.execution.counter),
                format!("{:?}", reused.execution.counter)
            );
            assert_eq!(
                baseline.execution.memory.solve_peak_forecast_bytes,
                reused.execution.memory.solve_peak_forecast_bytes
            );
        }
        let mut invalid = plan.clone();
        invalid.deletion.physical_count[0] = 0;
        assert!(run_jla_no_controls_planned_with_plan_and_interrupt(
            &problem,
            &invalid,
            options(33, 7),
            &mut RejectBuild
        )
        .is_err());
    }
}

#[test]
fn compressed_statistical_pool_preserves_order_cancellation_and_inert_parallelism() {
    use crate::interrupt::{CancellationInterrupt, CancellationToken};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    let problem = fixture(true, None);
    let plan = JlaPlan::build_no_controls(&problem).unwrap();
    for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
        let mut cmg = FullCmgPlanOptions::production(threads, 1e-10, None);
        cmg.maximum_batch_rhs = 66;
        let solver = PreparedTwoWaySolver::prepare_full_cmg_v2_with_interrupt(
            &problem,
            options(33, threads).estimator.solver,
            cmg,
            &mut NeverInterrupt,
        )
        .unwrap();
        for width in [1, 3, 5, 33] {
            let atoms = rademacher_atoms_with_interrupt(
                CounterRng::new(17),
                ProbeDomain::Leverage,
                3,
                width,
                &plan.deletion.semantic_rank,
                &plan.deletion.physical_count,
                &mut NeverInterrupt,
            )
            .unwrap();
            let mut expected = vec![0.0; problem.cells() * width];
            for column in 0..width {
                for group in 0..plan.deletion_units() {
                    expected[column * problem.cells() + plan.deletion.cell[group] as usize] +=
                        atoms[column * plan.deletion_units() + group] as f64;
                }
            }
            let expected =
                transpose_cell_rhs_with_interrupt(&problem, &expected, width, &mut NeverInterrupt)
                    .unwrap();
            let actual = leverage_rhs_with_interrupt(
                &problem,
                &plan,
                &atoms,
                width,
                &solver,
                &mut NeverInterrupt,
            )
            .unwrap();
            assert_eq!(expected, actual);
        }
        let owner = std::thread::current().id();
        let workers = Mutex::new(Vec::new());
        let mut values = [usize::MAX; 17];
        solver
            .statistical_work(
                values.iter_mut(),
                "inert_pool",
                |index, output, _| {
                    *output = index;
                    workers.lock().unwrap().push(std::thread::current().id());
                    Ok(())
                },
                &mut NeverInterrupt,
            )
            .unwrap();
        assert_eq!(values, core::array::from_fn(|index| index));
        assert!(workers
            .lock()
            .unwrap()
            .iter()
            .any(|&thread| thread != owner));
        let error = solver
            .statistical_work(
                0..17,
                "ordered_error",
                |index, _, _| {
                    if index == 2 || index == 9 {
                        Err(BackendError::invalid("ordered_error", index.to_string()))
                    } else {
                        Ok(())
                    }
                },
                &mut NeverInterrupt,
            )
            .unwrap_err();
        assert_eq!(error.message, "2");
        let token = CancellationToken::new();
        let calls = AtomicUsize::new(0);
        let error = solver
            .statistical_work(
                0..17,
                "inside_token",
                |_, _, check| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    token.cancel();
                    check.checkpoint("inside_compressed_worker")
                },
                &mut CancellationInterrupt::new(token.clone()),
            )
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert!(calls.load(Ordering::Relaxed) > 0);
        solver
            .statistical_work(0..3, "reuse", |_, _, _| Ok(()), &mut NeverInterrupt)
            .unwrap();
        solver
            .statistical_work(0..0, "empty", |_, _, _| unreachable!(), &mut NeverInterrupt)
            .unwrap();
    }
    assert!(statistical_metadata(u64::MAX).is_err());
    assert!(target_assembly_forecast(u64::MAX, 1, 1, 1, 1).is_err());
    assert!(target_assembly_forecast(1, 1, u64::MAX, 1, 1).is_err());
    assert!(statistical_buffer(usize::MAX, 0_u64, &mut NeverInterrupt).is_err());
}
