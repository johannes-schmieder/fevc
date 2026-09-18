// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use vckss_plugin::ffi_engine::*;

fn ok(status: i32) {
    assert_eq!(
        status,
        0,
        "{}",
        unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
    );
}

fn request(
    deletion: u32,
    nuisance: u32,
    controls: u32,
    executor: u32,
    threads: u32,
    explicit: bool,
) -> VckssEngineSolveRequestV6 {
    let route = if executor == VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE {
        VCKSS_ROUTE_DIAGONAL_PCG
    } else {
        VCKSS_ROUTE_CMG_PCG
    };
    let mode = if explicit {
        VCKSS_BATCH_MODE_EXPLICIT
    } else {
        VCKSS_BATCH_MODE_AUTO
    };
    let capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        route,
        deletion,
        nuisance,
        controls,
        mode,
        mode,
    );
    let mut v4 = planned_solve_request(
        capability,
        if explicit { 3 } else { 0 },
        if explicit { 2 } else { 0 },
    );
    v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestV6>();
    v4.v3.v2.v1.probes = 17;
    VckssEngineSolveRequestV6 {
        v4,
        execution_mode: executor,
        threads,
        reserved_6: 0,
    }
}

fn work(generation: u64) -> VckssGenericExecutionReceiptV1 {
    let mut value = VckssGenericExecutionReceiptV1::default();
    ok(vckss_rust_engine_generic_execution_receipt_v1(
        generation, &mut value, 160,
    ));
    assert_eq!(value.struct_size, 160);
    assert_eq!(value.schema_version, 1);
    assert_eq!(value.generation, generation);
    assert_eq!(value.reserved, 0);
    assert_eq!(
        value.logical_rhs_count,
        value.fit_rhs_count
            + value.control_projection_rhs_count
            + value.point_probe_rhs_count
            + value.projection_rhs_count
            + value.component_rhs_count
            + value.gram_rhs_count
    );
    assert!(value.maximum_active_workers <= value.planned_workers);
    assert!(value.planned_workers <= value.permitted_threads);
    assert!(value.maximum_complete_residual <= 1e-11);
    assert!(value.command_peak_forecast_bytes > 0);
    if value.execution_mode == VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE {
        assert_eq!(
            value.queued_rhs_count + value.fit_rhs_count,
            value.logical_rhs_count
        );
        assert_eq!(value.cmg_rhs_count, 0);
        assert_eq!(value.cmg_selected_concurrency, 0);
    } else {
        assert_eq!(
            value.cmg_rhs_count,
            value.logical_rhs_count + value.control_refinement_rhs_count
        );
        assert_eq!(value.queued_rhs_count, 0);
        assert_eq!(value.maximum_active_workers, 0);
        assert!(value.cmg_selected_concurrency > 0);
        assert!(value.cmg_selected_concurrency <= value.permitted_threads);
    }
    value
}

fn point(generation: u64) -> [f64; 4] {
    let mut value = VckssEngineResultV1::default();
    ok(vckss_rust_engine_result_v1(
        generation,
        &mut value,
        bytes::<VckssEngineResultV1>(),
    ));
    component_bits(value.corrected).map(f64::from_bits)
}

fn close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (&actual, &expected) in actual.iter().zip(expected) {
        assert!(actual.is_finite() && expected.is_finite());
        assert!(
            (actual - expected).abs() <= 1e-8 * expected.abs().max(1.0),
            "{actual} != {expected}"
        );
    }
}

#[test]
fn v8_resolves_original_auto_requests_before_optional_diagonal_queue() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(bytes::<VckssEngineSolveRequestV8>(), 320);
    assert_eq!(bytes::<VckssEngineSolveRequestInterruptV8>(), 344);
    let mut default = VckssEngineSolveRequestV8::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v8(&mut default, 319),
        ErrorCode::AbiMismatch as i32
    );
    ok(vckss_rust_engine_default_solve_request_v8(
        &mut default,
        320,
    ));
    let mut interrupt = VckssEngineSolveRequestInterruptV8::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v8(&mut interrupt, 343),
        ErrorCode::AbiMismatch as i32
    );
    ok(vckss_rust_engine_default_solve_request_interrupt_v8(
        &mut interrupt,
        344,
    ));
    let columns = OwnedColumns::generic_dense();
    for (algorithm, engine, deletion, exact_limit, selected_engine) in [
        (
            VCKSS_ALGORITHM_AUTO,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            VCKSS_DELETION_OBSERVATION,
            500,
            VCKSS_ENGINE_NOT_APPLICABLE,
        ),
        (
            VCKSS_ALGORITHM_JLA,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            VCKSS_DELETION_MATCH,
            2,
            VCKSS_ENGINE_COMPRESSED,
        ),
        (
            VCKSS_ALGORITHM_AUTO,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            VCKSS_DELETION_OBSERVATION,
            2,
            VCKSS_ENGINE_GENERIC,
        ),
        (
            VCKSS_ALGORITHM_JLA,
            VCKSS_ENGINE_GENERIC,
            VCKSS_DELETION_MATCH,
            2,
            VCKSS_ENGINE_GENERIC,
        ),
    ] {
        let capability = planned_capability_request(
            algorithm,
            engine,
            VCKSS_ROUTE_AUTO,
            deletion,
            VCKSS_NUISANCE_JOINT,
            0,
            VCKSS_BATCH_MODE_AUTO,
            VCKSS_BATCH_MODE_AUTO,
        );
        let mut baseline = planned_solve_request(capability, 0, 0);
        baseline.v3.v2.exact_estimator_limit = exact_limit;
        baseline.v3.v2.v1.probes = if selected_engine == VCKSS_ENGINE_GENERIC {
            200
        } else {
            17
        };
        let baseline_generation = prepare_with_controls(&columns, &[], deletion);
        ok(vckss_rust_engine_solve_v4(baseline_generation, &baseline));
        let baseline_point = point(baseline_generation);
        ok(vckss_rust_engine_release_v1(baseline_generation));

        let mut candidate = VckssEngineSolveRequestV8::default();
        candidate.v7.v6.v4 = baseline;
        candidate.v7.v6.v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestV8>();
        candidate.v7.v6.threads = 7;
        let mut generation = prepare_with_controls(&columns, &[], deletion);
        if selected_engine == VCKSS_ENGINE_GENERIC {
            let mut wrong_signature = candidate;
            wrong_signature.v7.v6.v4.v3.request_signature ^= 1;
            assert_ne!(vckss_rust_engine_solve_v8(generation, &wrong_signature), 0);
            let mut snapshot = VckssEngineSnapshotV1::default();
            ok(vckss_rust_engine_snapshot_v1(
                &mut snapshot,
                bytes::<VckssEngineSnapshotV1>(),
            ));
            // Signature reconciliation runs inside the fail-closed solving
            // transaction. It leaves a failed generation, never a reusable one.
            assert_eq!(snapshot.state, 4);
            ok(vckss_rust_engine_release_v1(generation));
            generation = prepare_with_controls(&columns, &[], deletion);
        }
        ok(vckss_rust_engine_solve_v8(generation, &candidate));
        let mut plan = VckssExecutionPlanReceiptV1::default();
        ok(vckss_rust_engine_execution_plan_receipt_v1(
            generation,
            &mut plan,
            bytes::<VckssExecutionPlanReceiptV1>(),
        ));
        assert_eq!(plan.request_signature, baseline.v3.request_signature);
        assert_eq!(plan.resolution.algorithm_requested, algorithm);
        assert_eq!(plan.resolution.engine_requested, engine);
        assert_eq!(plan.resolution.engine_selected, selected_engine);
        close(&point(generation), &baseline_point);
        if selected_engine == VCKSS_ENGINE_GENERIC {
            assert_eq!(plan.solver.requested_route, VCKSS_ROUTE_AUTO);
            assert_eq!(plan.solver.selected_route, VCKSS_ROUTE_DIAGONAL_PCG);
            let receipt = work(generation);
            assert_eq!(
                receipt.execution_mode,
                VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE
            );
            assert_eq!(receipt.permitted_threads, 7);
            assert!(receipt.queued_rhs_count > 0);
            assert_eq!(receipt.leverage_batch_width, 56);
            assert_eq!(receipt.target_batch_width, 32);
            assert_eq!(receipt.maximum_rhs_capacity, 64);
        } else {
            let mut receipt = VckssGenericExecutionReceiptV1::default();
            assert_ne!(
                vckss_rust_engine_generic_execution_receipt_v1(
                    generation,
                    &mut receipt,
                    bytes::<VckssGenericExecutionReceiptV1>(),
                ),
                0
            );
            assert_eq!(receipt.generation, 0);
        }
        ok(vckss_rust_engine_release_v1(generation));
    }
}

#[test]
fn v8_automatic_cmg_selection_uses_direct_execution() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let columns = OwnedColumns::structured_component_sized(8, 256);
    let capability = planned_capability_request(
        VCKSS_ALGORITHM_JLA,
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    let mut baseline = planned_solve_request(capability, 0, 0);
    baseline.v3.v2.v1.probes = 9;
    let baseline_generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    ok(vckss_rust_engine_solve_v4(baseline_generation, &baseline));
    let reference = point(baseline_generation);
    ok(vckss_rust_engine_release_v1(baseline_generation));

    let mut candidate = VckssEngineSolveRequestV8::default();
    candidate.v7.v6.v4 = baseline;
    candidate.v7.v6.v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestV8>();
    candidate.v7.v6.threads = 7;
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    ok(vckss_rust_engine_solve_v8(generation, &candidate));
    close(&point(generation), &reference);
    let mut plan = VckssExecutionPlanReceiptV1::default();
    ok(vckss_rust_engine_execution_plan_receipt_v1(
        generation,
        &mut plan,
        bytes::<VckssExecutionPlanReceiptV1>(),
    ));
    assert_eq!(plan.solver.requested_route, VCKSS_ROUTE_AUTO);
    assert_eq!(plan.solver.selected_route, VCKSS_ROUTE_CMG_PCG);
    let mut receipt = VckssGenericExecutionReceiptV1::default();
    ok(vckss_rust_engine_generic_execution_receipt_v1(
        generation,
        &mut receipt,
        bytes::<VckssGenericExecutionReceiptV1>(),
    ));
    assert_eq!(
        receipt.execution_mode,
        VCKSS_GENERIC_EXECUTION_DIRECT_ATTACHMENTS
    );
    assert_eq!(receipt.permitted_threads, 7);
    assert_eq!(receipt.logical_rhs_count, 28);
    assert_eq!(receipt.cmg_rhs_count, 28);
    assert_eq!(receipt.queued_rhs_count, 0);
    ok(vckss_rust_engine_release_v1(generation));
}

#[test]
fn v8_resolved_queue_interrupts_on_caller_and_reuses_after_release() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let columns = OwnedColumns::generic_dense();
    let capability = planned_capability_request(
        VCKSS_ALGORITHM_AUTO,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    let mut request = VckssEngineSolveRequestInterruptV8::default();
    request.options.v7.v6.v4 = planned_solve_request(capability, 0, 0);
    request.options.v7.v6.v4.v3.v2.exact_estimator_limit = 2;
    request.options.v7.v6.v4.v3.v2.v1.probes = 17;
    request.options.v7.v6.v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV8>();
    request.options.v7.v6.threads = 7;
    let mut poll = CallerPoll {
        inner: PollState {
            calls: 0,
            stop_at: 1,
            terminal_status: VCKSS_INTERRUPT_USER_BREAK,
        },
        caller: std::thread::current().id(),
        wrong_thread: false,
    };
    request.interrupt_poll = Some(caller_poll);
    request.interrupt_context = (&mut poll as *mut CallerPoll).cast();
    request.checkpoint_interval = 1;
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v8(generation, &request),
        ErrorCode::UserBreak as i32
    );
    assert!(poll.inner.calls > 0);
    assert!(!poll.wrong_thread);
    ok(vckss_rust_engine_release_v1(generation));

    poll.inner.calls = 0;
    poll.inner.stop_at = u32::MAX;
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    ok(vckss_rust_engine_solve_interrupt_v8(generation, &request));
    assert!(!poll.wrong_thread);
    work(generation);
    ok(vckss_rust_engine_release_v1(generation));
}

#[test]
fn v6_diagonal_point_batches_threads_and_legacy_meanings() {
    let _guard = TEST_LOCK.lock().unwrap();
    let columns = OwnedColumns::generic_dense();
    for deletion in [VCKSS_DELETION_MATCH, VCKSS_DELETION_OBSERVATION] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            for q in [0, 1] {
                let controls = if q == 0 {
                    vec![]
                } else {
                    vec![one_generic_control(&columns)]
                };
                for explicit in [false, true] {
                    let mut reference = None;
                    let mut counter = None;
                    for threads in [0, 1, 2, 3, 4, 7, 14, 28, 64] {
                        reset();
                        let generation =
                            prepare_with_controls_memory(&columns, &controls, deletion, 1 << 30);
                        let request = request(
                            deletion,
                            nuisance,
                            q,
                            VCKSS_GENERIC_EXECUTION_DIAGONAL_QUEUE,
                            threads,
                            explicit,
                        );
                        if threads == 0 {
                            // V5 flag=0 still ignores threads, even u32::MAX.
                            let old = VckssEngineSolveRequestV5 {
                                v4: request.v4,
                                threads: u32::MAX,
                                tolerance_supplied: 0,
                                full_cmg_v2: 0,
                                reserved_5: 0,
                            };
                            ok(vckss_rust_engine_solve_v5(generation, &old));
                            let mut work = VckssGenericExecutionReceiptV1::default();
                            assert_eq!(
                                vckss_rust_engine_generic_execution_receipt_v1(
                                    generation, &mut work, 160
                                ),
                                ErrorCode::UnsupportedFeature as i32
                            );
                            reference = Some(point(generation));
                        } else {
                            ok(vckss_rust_engine_solve_v6(generation, &request));
                            let work = work(generation);
                            assert_eq!(work.permitted_threads, threads);
                            assert_eq!(work.control_projection_rhs_count, u64::from(q));
                            assert_eq!(
                                work.fit_rhs_count,
                                1 + u64::from(q > 0 && nuisance == VCKSS_NUISANCE_FIXED_OFFSET)
                            );
                            assert_eq!(work.point_probe_rhs_count, 51);
                            assert_eq!(work.leverage_batch_width, if explicit { 3 } else { 17 });
                            assert_eq!(work.target_batch_width, if explicit { 2 } else { 17 });
                            assert_eq!(work.maximum_rhs_capacity, if explicit { 4 } else { 34 });
                            close(&point(generation), &reference.unwrap());
                        }
                        let mut detail = VckssEngineDetailedReceiptV7::default();
                        ok(vckss_rust_engine_detailed_receipt_v7(
                            generation,
                            &mut detail,
                            bytes::<VckssEngineDetailedReceiptV7>(),
                        ));
                        let atoms = detail.execution.counter.total.actual_logical_atoms;
                        assert_eq!(atoms, *counter.get_or_insert(atoms));
                        assert_eq!(detail.execution.counter.completed, 1);
                        ok(vckss_rust_engine_release_v1(generation));
                    }
                }
            }
        }
    }
}

fn attach(generation: u64, grouped: bool, width: u32) {
    let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
    augmentation.options.seed = 8_675_309;
    augmentation.options.probes = 33;
    augmentation.options.batch_width = width;
    augmentation.options.spectrum_probes = 17;
    augmentation.options.spectrum_iterations = 64;
    let function = if grouped {
        vckss_rust_engine_augment_match_component_inference_interrupt_v4
    } else {
        vckss_rust_engine_augment_component_inference_interrupt_v4
    };
    ok(function(generation, &augmentation, 513));
}

fn attach_automatic(generation: u64, grouped: bool) {
    let mut augmentation = VckssComponentInferenceAugmentationRequestInterruptV1::default();
    augmentation.options.seed = 8_675_309;
    augmentation.options.probes = 33;
    augmentation.options.batch_width = 0;
    augmentation.options.spectrum_probes = 17;
    augmentation.options.spectrum_iterations = 64;
    let function = if grouped {
        vckss_rust_engine_augment_match_component_inference_interrupt_v5
    } else {
        vckss_rust_engine_augment_component_inference_interrupt_v5
    };
    ok(function(generation, &augmentation, 513));
}

#[test]
fn v7_automatic_component_batches_are_distinct_from_literal_eight() {
    let _guard = TEST_LOCK.lock().unwrap();
    for grouped in [false, true] {
        let deletion = if grouped {
            VCKSS_DELETION_MATCH
        } else {
            VCKSS_DELETION_OBSERVATION
        };
        let nuisance = if grouped {
            VCKSS_NUISANCE_FIXED_OFFSET
        } else {
            VCKSS_NUISANCE_JOINT
        };
        let mut columns = OwnedColumns::structured_component();
        if grouped {
            for (row, key) in columns.deletion.iter_mut().enumerate() {
                *key = (row / 2 + 1) as f64;
            }
        }
        let mut reference: Option<(Vec<f64>, [f64; 4])> = None;
        for (executor, threads, automatic) in [
            (1, 1, false),
            (1, 1, true),
            (1, 7, true),
            (2, 1, true),
            (2, 7, true),
        ] {
            reset();
            let generation = prepare_with_controls_memory(&columns, &[], deletion, 1 << 30);
            if automatic {
                attach_automatic(generation, grouped);
            } else {
                attach(generation, grouped, 8);
            }
            let mut old = request(deletion, nuisance, 0, executor, threads, false);
            old.v4.v3.v2.v1.probes = 17;
            old.v4.v3.v2.v1.seed = 8_675_309;
            if automatic {
                let mut new = VckssEngineSolveRequestV7 {
                    v6: old,
                    ..Default::default()
                };
                new.v6.v4.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestV7>();
                ok(vckss_rust_engine_solve_v7(generation, &new));
            } else {
                ok(vckss_rust_engine_solve_v6(generation, &old));
            }
            let work = work(generation);
            let mut batch = VckssComponentBatchReceiptV1::default();
            ok(vckss_rust_engine_component_batch_receipt_v1(
                generation,
                &mut batch,
                bytes::<VckssComponentBatchReceiptV1>(),
            ));
            assert_eq!(batch.struct_size, 88);
            assert_eq!(batch.generation, generation);
            assert_eq!(batch.policy, u32::from(automatic));
            assert_eq!(
                batch.declared_component_width,
                if automatic { 1 } else { 8 }
            );
            assert_eq!(batch.declared_gram_width, if automatic { 1 } else { 8 });
            assert_eq!(
                batch.component_width,
                if automatic {
                    (8 * threads).clamp(32, 33) as u64
                } else {
                    8
                }
            );
            assert_eq!(
                batch.gram_width,
                if automatic {
                    (8 * threads).clamp(32, 513) as u64
                } else {
                    8
                }
            );
            assert_eq!(batch.maximum_rhs_capacity, work.maximum_rhs_capacity);
            if automatic && threads == 7 {
                assert!(work.maximum_rhs_capacity > 34);
            }
            assert_eq!(
                batch.command_peak_forecast_bytes,
                work.command_peak_forecast_bytes
            );
            let science = (inference(generation).0, point(generation));
            if let Some((expected, expected_point)) = &reference {
                close(&science.0, expected);
                close(&science.1, expected_point);
            } else {
                reference = Some(science);
            }
            ok(vckss_rust_engine_release_v1(generation));
        }
    }
}

fn inference(generation: u64) -> (Vec<f64>, VckssComponentInferenceResultReceiptV5) {
    let mut primitive = [0.; 9];
    let mut covariance = [0.; 16];
    let mut mcse = [0.; 9];
    let mut spectrum = [0.; 60];
    let mut summaries = [0.; 24];
    let mut folds = [0.; 150];
    let mut cv = [0.; 490];
    let mut targets = [0.; 8];
    let mut receipt = VckssComponentInferenceResultReceiptV5::default();
    ok(vckss_rust_engine_component_inference_result_v5(
        generation,
        primitive.as_mut_ptr(),
        9,
        covariance.as_mut_ptr(),
        16,
        mcse.as_mut_ptr(),
        9,
        spectrum.as_mut_ptr(),
        60,
        ptr::null_mut(),
        0,
        summaries.as_mut_ptr(),
        24,
        folds.as_mut_ptr(),
        150,
        cv.as_mut_ptr(),
        490,
        targets.as_mut_ptr(),
        8,
        &mut receipt,
        bytes::<VckssComponentInferenceResultReceiptV5>(),
    ));
    (
        primitive
            .into_iter()
            .chain(covariance)
            .chain(targets)
            .collect(),
        receipt,
    )
}

#[test]
fn v6_component_direct_gram_and_queue_native_equivalence() {
    let _guard = TEST_LOCK.lock().unwrap();
    for grouped in [false, true] {
        let deletion = if grouped {
            VCKSS_DELETION_MATCH
        } else {
            VCKSS_DELETION_OBSERVATION
        };
        let nuisance = if grouped {
            VCKSS_NUISANCE_FIXED_OFFSET
        } else {
            VCKSS_NUISANCE_JOINT
        };
        let mut columns = OwnedColumns::structured_component();
        if grouped {
            for (row, key) in columns.deletion.iter_mut().enumerate() {
                *key = (row / 2 + 1) as f64;
            }
        }
        for q in [0, 1] {
            let controls = if q == 0 {
                vec![]
            } else {
                vec![one_generic_control(&columns)]
            };
            for width in [7, 8] {
                let mut reference: Option<(Vec<f64>, [f64; 4])> = None;
                for (executor, threads) in [(1, 0), (1, 1), (1, 7), (2, 1), (2, 7)] {
                    reset();
                    let generation =
                        prepare_with_controls_memory(&columns, &controls, deletion, 1 << 30);
                    attach(generation, grouped, width);
                    let mut request = request(deletion, nuisance, q, executor, threads, false);
                    request.v4.v3.v2.v1.probes = 200;
                    request.v4.v3.v2.v1.seed = 8_675_309;
                    if threads == 0 {
                        ok(vckss_rust_engine_solve_v4(generation, &request.v4));
                    } else {
                        ok(vckss_rust_engine_solve_v6(generation, &request));
                        let work = work(generation);
                        assert_eq!(work.point_probe_rhs_count, 600);
                        assert_eq!(work.gram_rhs_count, 513);
                        assert!(work.component_rhs_count > 0);
                        assert_eq!(work.projection_rhs_count, 0);
                        if executor == 2 {
                            let mut old = VckssFullCmgModelReceiptV1::default();
                            assert_eq!(
                                vckss_rust_engine_full_cmg_model_receipt_v1(
                                    generation, &mut old, 56
                                ),
                                ErrorCode::UnsupportedFeature as i32
                            );
                            // All fields are numeric scalars or byte arrays.
                            let mut cmg: VckssFullCmgReceiptV1 = unsafe { std::mem::zeroed() };
                            ok(vckss_rust_engine_full_cmg_receipt_v1(
                                generation, &mut cmg, 400,
                            ));
                            assert_eq!(cmg.probe_effective_tolerance, 1e-12);
                            assert_eq!(cmg.rhs_count, work.cmg_rhs_count);
                        }
                    }
                    let (actual, receipt) = inference(generation);
                    assert_eq!(
                        receipt.v4.v3.v2.reference_distribution,
                        VCKSS_COMPONENT_REFERENCE_Q0
                    );
                    if let Some((expected, point_ref)) = &reference {
                        close(&actual, expected);
                        close(&point(generation), point_ref);
                    } else {
                        reference = Some((actual, point(generation)));
                    }
                    ok(vckss_rust_engine_release_v1(generation));
                }
            }
        }
    }
}

#[test]
fn v6_invalid_requests_are_atomic_and_receipts_are_generation_safe() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(bytes::<VckssEngineSolveRequestV6>(), 304);
    assert_eq!(bytes::<VckssEngineSolveRequestInterruptV6>(), 328);
    assert_eq!(bytes::<VckssGenericExecutionReceiptV1>(), 160);
    let mut default = VckssEngineSolveRequestV6::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v6(&mut default, 303),
        ErrorCode::AbiMismatch as i32
    );
    ok(vckss_rust_engine_default_solve_request_v6(
        &mut default,
        304,
    ));
    let mut interrupt = VckssEngineSolveRequestInterruptV6::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v6(&mut interrupt, 327),
        ErrorCode::AbiMismatch as i32
    );
    ok(vckss_rust_engine_default_solve_request_interrupt_v6(
        &mut interrupt,
        328,
    ));
    let columns = OwnedColumns::generic_dense();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    let good = request(
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        1,
        7,
        true,
    );
    for mutation in 0..9 {
        let mut bad = good;
        match mutation {
            0 => bad.reserved_6 = 1,
            1 => bad.v4.v3.v2.v1.struct_size = 303,
            2 => bad.threads = 0,
            3 => bad.execution_mode = 0,
            4 => bad.execution_mode = u32::MAX,
            5 => bad.v4.v3.v2.v1.solver_route = VCKSS_ROUTE_CMG_PCG,
            6 => bad.v4.v3.engine = VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            7 => bad.v4.v3.v2.algorithm = VCKSS_ALGORITHM_EXACT,
            _ => bad.v4.v3.v2.v1.allow_automatic_cmg_setup_fallback = 1,
        }
        assert_ne!(vckss_rust_engine_solve_v6(generation, &bad), 0);
        let mut snapshot = VckssEngineSnapshotV1::default();
        ok(vckss_rust_engine_snapshot_v1(
            &mut snapshot,
            bytes::<VckssEngineSnapshotV1>(),
        ));
        assert_eq!(snapshot.state, 1);
    }
    ok(vckss_rust_engine_solve_v6(generation, &good));
    let mut value = VckssGenericExecutionReceiptV1 {
        generation: 42,
        ..Default::default()
    };
    assert_eq!(
        vckss_rust_engine_generic_execution_receipt_v1(generation, &mut value, 159),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(value.generation, 42);
    work(generation);
    ok(vckss_rust_engine_release_v1(generation));
    assert_ne!(
        vckss_rust_engine_generic_execution_receipt_v1(generation, &mut value, 160),
        0
    );
    assert_eq!(value.generation, 42);
}

#[test]
fn v6_interrupts_both_executors_on_caller_and_reuses_after_release() {
    let _guard = TEST_LOCK.lock().unwrap();
    let columns = OwnedColumns::structured_component();
    for executor in [1, 2] {
        reset();
        let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
        attach(generation, false, 7);
        let mut request = VckssEngineSolveRequestInterruptV6 {
            options: request(
                VCKSS_DELETION_OBSERVATION,
                VCKSS_NUISANCE_JOINT,
                0,
                executor,
                7,
                false,
            ),
            ..Default::default()
        };
        request.options.v4.v3.v2.v1.struct_size = 328;
        let mut poll = CallerPoll {
            inner: PollState {
                calls: 0,
                stop_at: 1,
                terminal_status: VCKSS_INTERRUPT_USER_BREAK,
            },
            caller: std::thread::current().id(),
            wrong_thread: false,
        };
        request.interrupt_poll = Some(caller_poll);
        request.interrupt_context = (&mut poll as *mut CallerPoll).cast();
        request.checkpoint_interval = 1;
        assert_eq!(
            vckss_rust_engine_solve_interrupt_v6(generation, &request),
            ErrorCode::UserBreak as i32
        );
        assert!(poll.inner.calls > 0);
        assert!(!poll.wrong_thread);
        ok(vckss_rust_engine_release_v1(generation));
        ok(vckss_rust_engine_release_v1(generation));
        let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
        attach(generation, false, 7);
        request.options.v4.v3.v2.v1.probes = 200;
        request.options.v4.v3.v2.v1.seed = 8_675_309;
        poll.inner.calls = 0;
        poll.inner.stop_at = u32::MAX;
        ok(vckss_rust_engine_solve_interrupt_v6(generation, &request));
        assert!(!poll.wrong_thread);
        work(generation);
        ok(vckss_rust_engine_release_v1(generation));
    }
}

struct CallerPoll {
    inner: PollState,
    caller: std::thread::ThreadId,
    wrong_thread: bool,
}

unsafe extern "C" fn caller_poll(context: *mut c_void) -> i32 {
    // The synchronous test call retains unique ownership of this state.
    let state = unsafe { &mut *context.cast::<CallerPoll>() };
    state.wrong_thread |= std::thread::current().id() != state.caller;
    unsafe { injected_poll((&mut state.inner as *mut PollState).cast()) }
}

#[test]
fn v6_projection_outputs_and_psd_withholding_match_legacy() {
    let _guard = TEST_LOCK.lock().unwrap();
    // Existing public test_rust_projection.do fixture, including its declared
    // outcome location. This is an interface oracle, not a coverage campaign.
    let mut columns = OwnedColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
    };
    let mut controls = vec![vec![]];
    let mut projects = [vec![]];
    for row in 0..240 {
        let worker = (row / 6) as f64;
        let time = (row % 6) as f64;
        let firm = ((row / 6 + (row % 6) / 2) % 20) as f64;
        let control = time - 2.5;
        let noise = 0.25 * (((row + 1) * 17) as f64 / 11.).sin()
            + 0.15 * (((row + 1) * 7) as f64 / 13.).cos();
        columns.worker.push(worker + 1.);
        columns.firm.push(firm + 1.);
        columns.deletion.push(worker * 20. + firm + 1.);
        columns
            .outcome
            .push(-4. + 0.08 * worker - 0.12 * firm + 0.3 * control + noise);
        columns.frequency.push(1.);
        columns.target_weight.push(1.);
        controls[0].push(control);
        // The projection preparation adds its own intercept.
        projects[0].push((worker / 5.).sin() + (firm / 3.).cos() + time / 20.);
    }
    let pointers = projects.each_ref().map(|column| column.as_ptr());
    let rows = columns.worker.len() as u64;
    let mut augmentation = VckssProjectionAugmentationRequestInterruptV1::default();
    augmentation.options.rows = rows;
    augmentation.options.project_count = 1;
    augmentation.options.effect = VCKSS_PROJECTION_EFFECT_FIRM;
    augmentation.options.caller_copy_bytes = rows * 8;
    let descriptor = VckssProjectionColumnsV1 {
        struct_size: bytes::<VckssProjectionColumnsV1>(),
        reserved: 0,
        rows,
        project: pointers.as_ptr(),
        project_count: 1,
        reserved_2: 0,
    };
    let mut successful_cells = 0;
    let mut withheld_cells = 0;
    for deletion in [VCKSS_DELETION_OBSERVATION, VCKSS_DELETION_MATCH] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            let mut reference: Option<Vec<f64>> = None;
            let mut expected_status = None;
            for (executor, threads) in [(1, 0), (1, 1), (1, 7), (2, 1), (2, 7)] {
                reset();
                let generation =
                    prepare_with_controls_memory(&columns, &controls, deletion, 1 << 30);
                ok(vckss_rust_engine_augment_projection_interrupt_v1(
                    generation,
                    &augmentation,
                    &descriptor,
                ));
                let mut request = request(deletion, nuisance, 1, executor, threads, false);
                request.v4.v3.v2.v1.probes = 200;
                let status = if threads == 0 {
                    vckss_rust_engine_solve_v4(generation, &request.v4)
                } else {
                    vckss_rust_engine_solve_v6(generation, &request)
                };
                println!("projection deletion={deletion} nuisance={nuisance} executor={executor} threads={threads} status={status}");
                assert_eq!(status, *expected_status.get_or_insert(status));
                if status != 0 {
                    // This existing fixture is not a positive-covariance
                    // oracle in every nuisance mode. Preserve the same typed
                    // withholding in every executor, never relax the PSD gate.
                    assert_eq!(status, ErrorCode::JlaConstraintFailed as i32);
                    assert!(unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }
                        .to_string_lossy()
                        .contains("projection_covariance_psd"));
                    withheld_cells += usize::from(threads == 0);
                    ok(vckss_rust_engine_release_v1(generation));
                    continue;
                }
                successful_cells += usize::from(threads == 0);
                if threads > 0 {
                    assert_eq!(work(generation).projection_rhs_count, 2);
                }
                let mut beta = [0.; 2];
                let mut covariance = [0.; 4];
                let mut naive = [0.; 4];
                let mut receipt = VckssProjectionResultReceiptV1::default();
                ok(vckss_rust_engine_projection_result_v1(
                    generation,
                    beta.as_mut_ptr(),
                    2,
                    covariance.as_mut_ptr(),
                    4,
                    naive.as_mut_ptr(),
                    4,
                    &mut receipt,
                    152,
                ));
                assert!(receipt.maximum_complete_residual <= receipt.full_residual_tolerance);
                let actual: Vec<_> = beta.into_iter().chain(covariance).chain(naive).collect();
                if let Some(reference) = &reference {
                    close(&actual, reference);
                } else {
                    reference = Some(actual);
                }
                ok(vckss_rust_engine_release_v1(generation));
            }
        }
    }
    assert!(successful_cells > 0);
    assert!(withheld_cells > 0);
}

#[test]
fn v6_direct_requires_attachment_and_rejects_explicit_point_widths() {
    let _guard = TEST_LOCK.lock().unwrap();
    let columns = OwnedColumns::generic_dense();
    reset();
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_OBSERVATION);
    let explicit = request(
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        2,
        7,
        true,
    );
    assert_eq!(
        vckss_rust_engine_solve_v6(generation, &explicit),
        ErrorCode::UnsupportedFeature as i32
    );
    let automatic = request(
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        2,
        7,
        false,
    );
    assert_eq!(
        vckss_rust_engine_solve_v6(generation, &automatic),
        ErrorCode::InvalidInput as i32
    );
    ok(vckss_rust_engine_release_v1(generation));
}

fn prepare_policy(columns: &OwnedColumns, budget: Option<u64>, mode: u32) -> u64 {
    let mut request = VckssEnginePrepareRequestInterruptV3::default();
    request.options.v3.v2.rows = columns.worker.len() as u64;
    request.options.v3.v2.memory_limit_bytes = budget.unwrap_or(0);
    request.options.v3.v2.caller_copy_bytes = columns.worker.len() as u64 * 48;
    request.options.v3.deletion_mode = VCKSS_DELETION_OBSERVATION;
    let descriptor = VckssEngineColumnsV3 {
        v2: VckssEngineColumnsV2 {
            v1: VckssEngineColumnsV1 {
                struct_size: bytes::<VckssEngineColumnsV3>(),
                ..columns.descriptor()
            },
            controls: ptr::null(),
            controls_count: 0,
            reserved_2: 0,
        },
        probe_order: ptr::null(),
        probeorder_supplied: 0,
        reserved_3: 0,
    };
    let policy = VckssMemoryPolicyV1 {
        struct_size: 24,
        schema_version: 1,
        budget_present: u32::from(budget.is_some()),
        check_mode: mode,
        budget_bytes: budget.unwrap_or(0),
    };
    let mut generation = 0;
    ok(vckss_rust_engine_prepare_memory_interrupt_v1(
        &request,
        &descriptor,
        &policy,
        &mut generation,
        8,
    ));
    generation
}

#[test]
fn v6_native_memory_policy_explicit_boundary_and_omitted_budget() {
    let _guard = TEST_LOCK.lock().unwrap();
    let columns = OwnedColumns::generic_dense();
    let request = request(
        VCKSS_DELETION_OBSERVATION,
        VCKSS_NUISANCE_JOINT,
        0,
        1,
        7,
        true,
    );
    reset();
    let generation = prepare_policy(&columns, None, 1);
    ok(vckss_rust_engine_solve_v6(generation, &request));
    let baseline = work(generation);
    ok(vckss_rust_engine_release_v1(generation));
    for (budget, mode, succeeds) in [
        (baseline.command_peak_forecast_bytes, 1, true),
        (baseline.command_peak_forecast_bytes - 1, 1, false),
        (1, 2, true),
        (1, 3, true),
    ] {
        let generation = prepare_policy(&columns, Some(budget), mode);
        let status = vckss_rust_engine_solve_v6(generation, &request);
        if succeeds {
            ok(status);
            let actual = work(generation);
            assert_eq!(actual.leverage_batch_width, 3);
            assert_eq!(actual.target_batch_width, 2);
            assert_eq!(
                actual.command_peak_forecast_bytes,
                baseline.command_peak_forecast_bytes
            );
        } else {
            assert_eq!(status, ErrorCode::ResourceLimit as i32);
        }
        ok(vckss_rust_engine_release_v1(generation));
    }
}
