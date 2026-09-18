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
    hybrid: bool,
    threads: u32,
) -> VckssExactExecutionRequestV1 {
    let mut capability = planned_capability_request(
        VCKSS_ALGORITHM_EXACT,
        VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
        VCKSS_ROUTE_AUTO,
        deletion,
        nuisance,
        1,
        VCKSS_BATCH_MODE_AUTO,
        VCKSS_BATCH_MODE_AUTO,
    );
    if hybrid && deletion == VCKSS_DELETION_MATCH {
        capability.v2.stayers_mode = VCKSS_STAYERS_ALL;
    }
    let mut v4 = planned_solve_request(capability, 0, 0);
    v4.v3.v2.v1.struct_size = bytes::<VckssExactExecutionRequestV1>();
    VckssExactExecutionRequestV1 {
        v4,
        threads,
        reserved: 0,
    }
}

fn prepare(deletion: u32, hybrid: bool, budget: u64) -> u64 {
    let mut columns = OwnedColumns::generic_dense();
    // Observation stayers are part of the ordinary prepared population;
    // the separate augmentation represents mixed match/observation deletion.
    if hybrid && deletion == VCKSS_DELETION_OBSERVATION {
        columns.worker.extend([13.0, 13.0, 14.0]);
        columns.firm.extend([1.0, 1.0, 2.0]);
        columns.deletion.extend([97.0, 98.0, 99.0]);
        columns.outcome.extend([0.25, 1.75, -0.5]);
        columns.frequency.extend([1.0, 1.0, 2.0]);
        columns.target_weight.extend([0.75, 1.25, 2.5]);
    }
    let control = one_generic_control(&columns);
    let generation = prepare_with_controls_memory(&columns, &[control], deletion, budget);
    if hybrid && deletion == VCKSS_DELETION_MATCH {
        let firm = [1.0, 1.0, 2.0];
        let worker = [1.0, 1.0, 2.0];
        let outcome = [0.25, 1.75, -0.5];
        let frequency = [1.0, 1.0, 2.0];
        let weight = [0.75, 1.25, 2.5];
        let control = [0.5, 1.5, -0.25];
        let pointers = [control.as_ptr()];
        let columns = VckssStayerAugmentationColumnsV1 {
            struct_size: bytes::<VckssStayerAugmentationColumnsV1>(),
            reserved: 0,
            rows: 3,
            firm: firm.as_ptr(),
            worker: worker.as_ptr(),
            outcome: outcome.as_ptr(),
            frequency: frequency.as_ptr(),
            target_weight: weight.as_ptr(),
            controls: pointers.as_ptr(),
            controls_count: 1,
            reserved_2: 0,
        };
        let request = VckssStayerAugmentationRequestV1 {
            rows: 3,
            controls_count: 1,
            caller_copy_bytes: 3 * 6 * 8,
            ..Default::default()
        };
        ok(vckss_rust_engine_augment_stayers_v1(
            generation, &request, &columns,
        ));
    }
    generation
}

fn result(generation: u64, hybrid: bool) -> [u64; 4] {
    if hybrid {
        let mut value = VckssStayerHybridResultV1::default();
        ok(vckss_rust_engine_stayer_hybrid_result_v1(
            generation,
            &mut value,
            bytes::<VckssStayerHybridResultV1>(),
        ));
        assert!(value.accounting_residual <= 1e-12);
        component_bits(value.corrected)
    } else {
        let mut value = VckssEngineResultV1::default();
        ok(vckss_rust_engine_result_v1(
            generation,
            &mut value,
            bytes::<VckssEngineResultV1>(),
        ));
        component_bits(value.corrected)
    }
}

fn receipt(generation: u64) -> VckssExactExecutionReceiptV1 {
    let mut value = VckssExactExecutionReceiptV1::default();
    ok(vckss_rust_engine_exact_execution_receipt_v1(
        generation, &mut value, 64,
    ));
    assert_eq!(value.struct_size, 64);
    assert_eq!(value.schema_version, 1);
    assert_eq!(value.generation, generation);
    assert!(value.worker_limit > 0 && value.worker_limit <= value.requested_threads);
    assert!(value.maximum_original_fit_residual <= 1e-9);
    assert!(value.incremental_forecast_bytes > 0);
    let mut plan = VckssExecutionPlanReceiptV1::default();
    ok(vckss_rust_engine_execution_plan_receipt_v1(
        generation,
        &mut plan,
        bytes::<VckssExecutionPlanReceiptV1>(),
    ));
    assert_eq!(plan.solver.threads_requested, value.requested_threads);
    assert_eq!(plan.solver.threads_used, value.worker_limit);
    assert_eq!(
        u64::from(plan.solver.parallel_regions),
        value.parallel_regions
    );
    assert_eq!(plan.counter.rng_contract, VCKSS_RNG_NONE);
    let mut details = VckssEngineDetailedReceiptV5::default();
    ok(vckss_rust_engine_detailed_receipt_v5(
        generation,
        &mut details,
        bytes::<VckssEngineDetailedReceiptV5>(),
    ));
    assert_eq!(
        details.v4.v3.v2.solve_peak_forecast_bytes,
        plan.memory.command_peak_bytes
    );
    assert_eq!(
        details.v4.exact_peak_forecast_bytes,
        plan.memory.command_peak_bytes
    );
    assert_eq!(details.fit_peak_forecast_bytes, plan.memory.fit_peak_bytes);
    assert_eq!(
        details.correction_peak_forecast_bytes,
        plan.memory.correction_peak_bytes
    );
    assert_eq!(plan.counter.total.actual_logical_atoms, 0);
    assert_eq!(plan.solver.logical_atoms_before_plan_freeze, 0);
    value
}

#[test]
fn exact_opt_in_preserves_legacy_and_all_thread_results_including_stayers() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(vckss_rust_exact_execution_schema_v1(), 1);
    assert_eq!(bytes::<VckssExactExecutionRequestV1>(), 296);
    assert_eq!(bytes::<VckssExactExecutionRequestInterruptV1>(), 320);
    for deletion in [VCKSS_DELETION_OBSERVATION, VCKSS_DELETION_MATCH] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            for hybrid in [false, true] {
                let generation = prepare(deletion, hybrid, 512 << 20);
                let mut old = request(deletion, nuisance, hybrid, 1).v4;
                old.v3.v2.v1.struct_size = bytes::<VckssEngineSolveRequestV4>();
                ok(vckss_rust_engine_solve_v4(generation, &old));
                let mixed_deletion = hybrid && deletion == VCKSS_DELETION_MATCH;
                let expected = result(generation, mixed_deletion);
                let mut value = VckssExactExecutionReceiptV1 {
                    generation: 123,
                    ..Default::default()
                };
                assert_eq!(
                    vckss_rust_engine_exact_execution_receipt_v1(generation, &mut value, 64),
                    ErrorCode::UnsupportedFeature as i32
                );
                assert_eq!(value.generation, 123);
                ok(vckss_rust_engine_release_v1(generation));
                for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                    let generation = prepare(deletion, hybrid, 512 << 20);
                    ok(vckss_rust_engine_solve_exact_execution_v1(
                        generation,
                        &request(deletion, nuisance, hybrid, threads),
                    ));
                    assert_eq!(
                        result(generation, mixed_deletion),
                        expected,
                        "deletion={deletion} nuisance={nuisance} hybrid={hybrid} T={threads}"
                    );
                    let value = receipt(generation);
                    assert_eq!(value.estimator_passes, if mixed_deletion { 2 } else { 1 });
                    assert_eq!(value.parallel_regions == 0, threads == 1);
                    ok(vckss_rust_engine_release_v1(generation));
                }
            }
        }
    }
}

#[test]
fn exact_opt_in_preflights_complete_budget_and_does_not_write_invalid_receipts() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    for deletion in [VCKSS_DELETION_OBSERVATION, VCKSS_DELETION_MATCH] {
        for hybrid in [false, true] {
            let req = request(deletion, VCKSS_NUISANCE_JOINT, hybrid, 7);
            let generation = prepare(deletion, hybrid, 512 << 20);
            ok(vckss_rust_engine_solve_exact_execution_v1(generation, &req));
            let peak = receipt(generation).command_peak_forecast_bytes;
            let mut value = VckssExactExecutionReceiptV1 {
                generation: 321,
                ..Default::default()
            };
            assert_eq!(
                vckss_rust_engine_exact_execution_receipt_v1(generation, &mut value, 63),
                ErrorCode::AbiMismatch as i32
            );
            assert_eq!(value.generation, 321);
            ok(vckss_rust_engine_release_v1(generation));
            for budget in [peak, peak - 1] {
                let generation = prepare(deletion, hybrid, budget);
                let status = vckss_rust_engine_solve_exact_execution_v1(generation, &req);
                if budget == peak {
                    ok(status);
                    receipt(generation);
                } else {
                    assert_eq!(status, ErrorCode::ResourceLimit as i32);
                    assert_ne!(
                        vckss_rust_engine_exact_execution_receipt_v1(generation, &mut value, 64),
                        0
                    );
                    assert_eq!(value.generation, 321);
                }
                ok(vckss_rust_engine_release_v1(generation));
            }
        }
    }
    let generation = prepare(VCKSS_DELETION_MATCH, false, 512 << 20);
    let original = request(VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, false, 7);
    for kind in 0..4 {
        let mut req = original;
        match kind {
            0 => req.threads = 0,
            1 => req.reserved = 1,
            2 => req.v4.v3.v2.v1.struct_size = 295,
            _ => req.v4.v3.v2.algorithm = VCKSS_ALGORITHM_JLA,
        }
        assert_ne!(
            vckss_rust_engine_solve_exact_execution_v1(generation, &req),
            0
        );
    }
    ok(vckss_rust_engine_solve_exact_execution_v1(
        generation, &original,
    ));
    receipt(generation);
    ok(vckss_rust_engine_release_v1(generation));
}

struct Caller {
    thread: std::thread::ThreadId,
    calls: u32,
    stop: u32,
    wrong_thread: bool,
}

#[test]
fn legacy_exact_parallel_preserves_v2_results_and_ignored_tuning() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(vckss_rust_exact_legacy_execution_schema_v1(), 1);
    for deletion in [VCKSS_DELETION_OBSERVATION, VCKSS_DELETION_MATCH] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            let mut options = request(deletion, nuisance, false, 1).v4.v3.v2;
            options.v1.struct_size = bytes::<VckssEngineSolveRequestV2>();
            options.v1.leverage_batch_width = 3;
            options.v1.target_batch_width = 7;
            options.v1.probes = 11;
            let generation = prepare(deletion, false, 512 << 20);
            ok(vckss_rust_engine_solve_v2(generation, &options));
            let reference = result(generation, false);
            ok(vckss_rust_engine_release_v1(generation));
            for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                options.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV2>();
                let mut caller = Caller {
                    thread: std::thread::current().id(),
                    calls: 0,
                    stop: 1,
                    wrong_thread: false,
                };
                let req = VckssEngineSolveRequestInterruptV2 {
                    options,
                    interrupt_poll: Some(poll),
                    interrupt_context: (&mut caller as *mut Caller).cast(),
                    checkpoint_interval: 1,
                    reserved: 0,
                };
                let generation = prepare(deletion, false, 512 << 20);
                assert_eq!(
                    vckss_rust_engine_solve_exact_legacy_execution_interrupt_v1(
                        generation, &req, threads
                    ),
                    ErrorCode::UserBreak as i32
                );
                // Failed generations are deliberately poisoned. Reuse means
                // release, prepare a fresh generation, then execute successfully.
                ok(vckss_rust_engine_release_v1(generation));
                let generation = prepare(deletion, false, 512 << 20);
                caller.stop = u32::MAX;
                ok(vckss_rust_engine_solve_exact_legacy_execution_interrupt_v1(
                    generation, &req, threads,
                ));
                assert!(!caller.wrong_thread && caller.calls > 1);
                assert_eq!(result(generation, false), reference);
                let mut receipt = VckssExactExecutionReceiptV1::default();
                ok(vckss_rust_engine_exact_execution_receipt_v1(
                    generation,
                    &mut receipt,
                    bytes::<VckssExactExecutionReceiptV1>(),
                ));
                assert_eq!(receipt.requested_threads, threads);
                assert_eq!(receipt.estimator_passes, 1);
                ok(vckss_rust_engine_release_v1(generation));
            }
        }
    }
}
unsafe extern "C" fn poll(context: *mut c_void) -> i32 {
    // The synchronous ABI call borrows this live, uniquely owned test state.
    let caller = unsafe { &mut *context.cast::<Caller>() };
    caller.wrong_thread |= caller.thread != std::thread::current().id();
    caller.calls += 1;
    if caller.calls >= caller.stop {
        VCKSS_INTERRUPT_USER_BREAK
    } else {
        VCKSS_INTERRUPT_CONTINUE
    }
}

#[test]
fn legacy_parallel_exact_rejects_planned_hybrid_attachment() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let generation = prepare(VCKSS_DELETION_MATCH, true, 512 << 20);
    let mut options = request(VCKSS_DELETION_MATCH, VCKSS_NUISANCE_JOINT, false, 4)
        .v4
        .v3
        .v2;
    options.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV2>();
    let mut caller = Caller {
        thread: std::thread::current().id(),
        calls: 0,
        stop: u32::MAX,
        wrong_thread: false,
    };
    let req = VckssEngineSolveRequestInterruptV2 {
        options,
        interrupt_poll: Some(poll),
        interrupt_context: (&mut caller as *mut Caller).cast(),
        checkpoint_interval: 1,
        reserved: 0,
    };
    assert_eq!(
        vckss_rust_engine_solve_exact_legacy_execution_interrupt_v1(generation, &req, 4),
        ErrorCode::UnsupportedFeature as i32
    );
    let mut receipt = VckssExactExecutionReceiptV1::default();
    assert_ne!(
        vckss_rust_engine_exact_execution_receipt_v1(generation, &mut receipt, 64),
        0
    );
    ok(vckss_rust_engine_release_v1(generation));
}

#[test]
fn exact_opt_in_cancels_only_on_caller_and_successfully_reuses() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    for deletion in [VCKSS_DELETION_OBSERVATION, VCKSS_DELETION_MATCH] {
        let mut caller = Caller {
            thread: std::thread::current().id(),
            calls: 0,
            stop: 1,
            wrong_thread: false,
        };
        let mut req = VckssExactExecutionRequestInterruptV1 {
            options: request(deletion, VCKSS_NUISANCE_JOINT, true, 7),
            interrupt_poll: Some(poll),
            interrupt_context: (&mut caller as *mut Caller).cast(),
            checkpoint_interval: 1,
            reserved: 0,
        };
        req.options.v4.v3.v2.v1.struct_size = 320;
        // Native coordination polls by elapsed time, not estimator phases;
        // tiny solves can finish before an arbitrary later poll. Core tests
        // separately inject phase-specific breaks inside parallel regions.
        for stop in [1, 2, u32::MAX] {
            caller.calls = 0;
            caller.stop = stop;
            let generation = prepare(deletion, true, 512 << 20);
            let status = vckss_rust_engine_solve_exact_execution_interrupt_v1(generation, &req);
            if stop == u32::MAX {
                ok(status);
                receipt(generation);
            } else {
                assert_eq!(status, ErrorCode::UserBreak as i32);
            }
            assert!(caller.calls > 0 && !caller.wrong_thread);
            ok(vckss_rust_engine_release_v1(generation));
            ok(vckss_rust_engine_release_v1(generation));
        }
    }
}

#[test]
fn resolved_exact_preserves_auto_signature_and_rejects_jla_before_rng() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(vckss_rust_exact_execution_schema_v1(), 1);
    assert_eq!(vckss_rust_exact_resolved_execution_schema_v2(), 2);
    for deletion in [VCKSS_DELETION_OBSERVATION, VCKSS_DELETION_MATCH] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            for hybrid in [false, true] {
                let mut capability = planned_capability_request(
                    VCKSS_ALGORITHM_AUTO,
                    VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
                    VCKSS_ROUTE_AUTO,
                    deletion,
                    nuisance,
                    1,
                    VCKSS_BATCH_MODE_AUTO,
                    VCKSS_BATCH_MODE_AUTO,
                );
                if hybrid && deletion == VCKSS_DELETION_MATCH {
                    capability.v2.stayers_mode = VCKSS_STAYERS_ALL;
                }
                let v4 = planned_solve_request(capability, 0, 0);
                let generation = prepare(deletion, hybrid, 512 << 20);
                ok(vckss_rust_engine_solve_v4(generation, &v4));
                let expected = result(generation, hybrid && deletion == VCKSS_DELETION_MATCH);
                ok(vckss_rust_engine_release_v1(generation));
                for threads in [1, 4, 7] {
                    let generation = prepare(deletion, hybrid, 512 << 20);
                    let mut req = VckssExactExecutionRequestV1 {
                        v4,
                        threads,
                        reserved: 0,
                    };
                    req.v4.v3.v2.v1.struct_size = bytes::<VckssExactExecutionRequestV1>();
                    assert_eq!(
                        vckss_rust_engine_solve_exact_execution_v1(generation, &req),
                        ErrorCode::InvalidInput as i32
                    );
                    ok(vckss_rust_engine_solve_exact_resolved_execution_v2(
                        generation, &req,
                    ));
                    assert_eq!(
                        result(generation, hybrid && deletion == VCKSS_DELETION_MATCH),
                        expected
                    );
                    receipt(generation);
                    let mut plan = VckssExecutionPlanReceiptV1::default();
                    ok(vckss_rust_engine_execution_plan_receipt_v1(
                        generation,
                        &mut plan,
                        bytes::<VckssExecutionPlanReceiptV1>(),
                    ));
                    assert_eq!(plan.request_signature, v4.v3.request_signature);
                    assert_eq!(plan.resolution.algorithm_requested, VCKSS_ALGORITHM_AUTO);
                    assert_eq!(plan.resolution.algorithm_selected, VCKSS_ALGORITHM_EXACT);
                    ok(vckss_rust_engine_release_v1(generation));
                }
                let generation = prepare(deletion, hybrid, 512 << 20);
                let mut req = VckssExactExecutionRequestV1 {
                    v4,
                    threads: 4,
                    reserved: 0,
                };
                req.v4.v3.v2.v1.struct_size = bytes::<VckssExactExecutionRequestV1>();
                req.v4.v3.v2.exact_estimator_limit = 2;
                assert_eq!(
                    vckss_rust_engine_solve_exact_resolved_execution_v2(generation, &req),
                    ErrorCode::UnsupportedFeature as i32,
                    "{}",
                    unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) }.to_string_lossy()
                );
                let mut value = VckssExactExecutionReceiptV1::default();
                assert_ne!(
                    vckss_rust_engine_exact_execution_receipt_v1(generation, &mut value, 64),
                    0
                );
                ok(vckss_rust_engine_release_v1(generation));
            }
        }
    }
}
