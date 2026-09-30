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
    engine: u32,
    route: u32,
    deletion: u32,
    nuisance: u32,
    controls: u32,
) -> VckssNumericalRequestV1 {
    let mut point = planned_solve_request(
        planned_capability_request(
            VCKSS_ALGORITHM_JLA,
            engine,
            route,
            deletion,
            nuisance,
            controls,
            VCKSS_BATCH_MODE_AUTO,
            VCKSS_BATCH_MODE_AUTO,
        ),
        0,
        0,
    );
    point.v3.v2.v1.probes = 33;
    VckssNumericalRequestV1 {
        point,
        controls_count: controls,
        threads: 4,
        ..VckssNumericalRequestV1::default()
    }
}

fn result(generation: u64) -> VckssNumericalResultV1 {
    let mut value = VckssNumericalResultV1::default();
    ok(vckss_rust_engine_numerical_result_v1(
        generation,
        &mut value,
        bytes::<VckssNumericalResultV1>(),
    ));
    value
}

fn point(generation: u64) -> VckssEngineResultV1 {
    let mut value = VckssEngineResultV1::default();
    ok(vckss_rust_engine_result_v1(
        generation,
        &mut value,
        bytes::<VckssEngineResultV1>(),
    ));
    value
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn close(a: f64, b: f64) {
    assert!(a.is_finite() && b.is_finite());
    assert!(
        (a - b).abs() <= 1e-11 * a.abs().max(b.abs()).max(1.0),
        "{a} != {b}"
    );
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn numerical_v1_routes_preserve_points_conditional_and_owned_replay() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(vckss_rust_numerical_schema_v1(), 1);
    assert_eq!(bytes::<VckssNumericalRequestV1>(), 328);
    assert_eq!(bytes::<VckssNumericalResultV1>(), 512);
    let columns = OwnedColumns::generic_dense();
    for (engine, route, deletion, controls) in [
        (
            VCKSS_ENGINE_GENERIC,
            VCKSS_ROUTE_DIAGONAL_PCG,
            VCKSS_DELETION_OBSERVATION,
            1,
        ),
        (
            VCKSS_ENGINE_GENERIC,
            VCKSS_ROUTE_AUTO,
            VCKSS_DELETION_MATCH,
            1,
        ),
        (
            VCKSS_ENGINE_GENERIC,
            VCKSS_ROUTE_CMG_PCG,
            VCKSS_DELETION_MATCH,
            0,
        ),
        (
            VCKSS_ENGINE_COMPRESSED,
            VCKSS_ROUTE_DIAGONAL_PCG,
            VCKSS_DELETION_MATCH,
            0,
        ),
    ] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            let c = [one_generic_control(&columns)];
            let c = if controls == 0 { &[][..] } else { &c[..] };
            let generation = prepare_with_controls(&columns, c, deletion);
            let request = request(engine, route, deletion, nuisance, controls);
            ok(vckss_rust_engine_numerical_preflight_v1(&request));
            ok(vckss_rust_engine_solve_v4(generation, &request.point));
            let old = point(generation);
            ok(vckss_rust_engine_release_v1(generation));
            let generation = prepare_with_controls(&columns, c, deletion);
            ok(vckss_rust_engine_solve_numerical_v1(generation, &request));
            let new = point(generation);
            for (a, b) in component_bits(old.corrected)
                .into_iter()
                .zip(component_bits(new.corrected))
            {
                close(f64::from_bits(a), f64::from_bits(b));
            }
            for (a, b) in component_bits(old.numerical_mcse)
                .into_iter()
                .zip(component_bits(new.numerical_mcse))
            {
                close(f64::from_bits(a), f64::from_bits(b));
            }
            let value = result(generation);
            assert_eq!(
                vckss_rust_engine_full_cmg_model_receipt_v1(
                    generation,
                    &mut VckssFullCmgModelReceiptV1::default(),
                    bytes::<VckssFullCmgModelReceiptV1>()
                ),
                ErrorCode::UnsupportedFeature as i32
            );
            if route == VCKSS_ROUTE_CMG_PCG || route == VCKSS_ROUTE_AUTO {
                let mut work = VckssFullCmgModelReceiptV1::default();
                ok(vckss_rust_engine_numerical_cmg_work_v1(
                    generation,
                    &mut work,
                    bytes::<VckssFullCmgModelReceiptV1>(),
                ));
                assert_eq!(
                    work.logical_rhs_count,
                    value.point_rhs + value.executed_replay_rhs
                );
                assert_eq!(work.controls_count, controls);
            }
            assert_eq!((value.generation, value.engine), (generation, engine));
            assert!(matches!(value.status, 2..=4));
            assert_eq!(
                (
                    value.leverage_probes,
                    value.target_probes,
                    value.target_fold_a,
                    value.target_fold_b
                ),
                (33, 33, 17, 16)
            );
            assert_eq!(
                (value.certified_replay_rhs, value.executed_replay_rhs),
                (33, 33)
            );
            assert_eq!(value.failed_replay_probe, u32::MAX);
            assert_eq!(value.replay_error_code, 0);
            assert!(value.allocation_bound_bytes > 0 && value.replay_generator_words > 0);
            assert!(value.minimum_constrained > 0.0 && value.minimum_residual_margin > 0.0);
            assert!(
                value.maximum_point_complete_residual <= 1e-11
                    && value.maximum_replay_complete_residual <= 1e-11
            );
            let conditional = component_bits(new.numerical_mcse).map(f64::from_bits);
            for (i, mcse) in conditional[..3].iter().enumerate() {
                close(value.conditional[i][i], mcse * mcse);
            }
            for i in 0..3 {
                for j in 0..3 {
                    assert_eq!(
                        value.raw[i][j],
                        value.conditional[i][j] + value.leverage[i][j]
                    );
                    close(value.raw[i][j], value.raw[j][i]);
                }
            }
            if value.status == 4 {
                assert!(value.usable.iter().flatten().all(|v| v.is_nan()));
                assert!(value.mcse.iter().all(|v| v.is_nan()));
            } else {
                assert!(value.mcse.iter().all(|v| v.is_finite()));
            }
            let mut rhs = [VckssNumericalRhsV1::default(); 33];
            let mut written = 99;
            ok(vckss_rust_engine_numerical_rhs_v1(
                generation,
                rhs.as_mut_ptr(),
                33,
                &mut written,
            ));
            assert_eq!(written, 33);
            let before = rhs;
            assert_eq!(
                vckss_rust_engine_numerical_rhs_v1(
                    generation,
                    rhs.as_mut_ptr(),
                    33,
                    rhs.as_mut_ptr().cast::<u64>()
                ),
                ErrorCode::AbiMismatch as i32
            );
            for (a, b) in rhs.iter().zip(before) {
                assert_eq!(a.probe, b.probe);
                assert_eq!(a.complete_residual, b.complete_residual);
            }
            assert_eq!(value.attempted_replay_rhs, 33);
            for (i, r) in rhs.iter().enumerate() {
                assert_eq!(
                    (r.schema_version, r.phase, r.probe, r.reserved),
                    (1, 1, i as u32, 0)
                );
                assert!(r.complete_residual <= 1e-11);
            }
            assert_eq!(
                vckss_rust_engine_numerical_rhs_v1(generation, rhs.as_mut_ptr(), 32, &mut written),
                ErrorCode::AbiMismatch as i32
            );
            assert_eq!(written, 33);
            assert_eq!(
                vckss_rust_engine_numerical_rhs_v1(
                    generation,
                    rhs.as_mut_ptr(),
                    u64::MAX,
                    &mut written
                ),
                ErrorCode::ResourceLimit as i32
            );
            assert_eq!(
                vckss_rust_engine_numerical_result_v1(
                    generation,
                    &mut VckssNumericalResultV1::default(),
                    511
                ),
                ErrorCode::AbiMismatch as i32
            );
            ok(vckss_rust_engine_release_v1(generation));
            assert_ne!(
                vckss_rust_engine_numerical_result_v1(
                    generation,
                    &mut VckssNumericalResultV1::default(),
                    512
                ),
                0
            );
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn numerical_preflight_rejects_bad_counts_stale_schema_and_unqualified_intersections() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let valid = request(
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        0,
    );
    for field in 0..17 {
        let mut bad = valid;
        match field {
            0 => bad.struct_size = 327,
            1 => bad.schema_version = 0,
            2 => bad.reserved = 1,
            3 => bad.threads = 0,
            4 => bad.point.v3.v2.v1.probes = 1,
            5 => bad.projection_requested = 1,
            6 => bad.component_requested = 1,
            7 => bad.point.v3.capability_schema = 0,
            8 => bad.point.v3.v2.v1.pcg_tolerance = f64::NAN,
            9 => bad.point.target_batch_mode = 99,
            10 => bad.point.v3.v2.v1.deletion_mode = 99,
            11 => bad.point.v3.v2.nuisance_mode = 99,
            12 => bad.point.v3.v2.v1.struct_size = 0,
            13 => bad.point.reserved_4 = 1,
            14 => bad.point.v3.request_signature ^= 1,
            15 => bad.controls_count = 1,
            _ => bad.reserved_2 = 1,
        }
        assert_ne!(vckss_rust_engine_numerical_preflight_v1(&bad), 0);
    }
    for route in [VCKSS_ROUTE_AUTO, VCKSS_ROUTE_CMG_PCG] {
        let point = planned_solve_request(
            planned_capability_request(
                VCKSS_ALGORITHM_JLA,
                VCKSS_ENGINE_GENERIC,
                route,
                VCKSS_DELETION_MATCH,
                VCKSS_NUISANCE_JOINT,
                0,
                VCKSS_BATCH_MODE_EXPLICIT,
                VCKSS_BATCH_MODE_EXPLICIT,
            ),
            3,
            5,
        );
        let bad = VckssNumericalRequestV1 { point, ..valid };
        ok(vckss_rust_engine_numerical_preflight_v1(&bad));
    }
    assert_ne!(vckss_rust_engine_numerical_preflight_v1(ptr::null()), 0);
    let generation =
        prepare_with_controls(&OwnedColumns::generic_dense(), &[], VCKSS_DELETION_MATCH);
    let mut interrupted = PollState {
        calls: 0,
        stop_at: 2,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    assert_eq!(
        vckss_rust_engine_solve_numerical_interrupt_v1(
            generation,
            &valid,
            Some(injected_poll),
            (&mut interrupted as *mut PollState).cast(),
            1
        ),
        ErrorCode::UserBreak as i32
    );
    ok(vckss_rust_engine_release_v1(generation));
    let generation =
        prepare_with_controls(&OwnedColumns::generic_dense(), &[], VCKSS_DELETION_MATCH);
    ok(vckss_rust_engine_solve_numerical_v1(generation, &valid));
    assert_eq!(result(generation).executed_replay_rhs, 33);
    ok(vckss_rust_engine_release_v1(generation));
    let generation =
        prepare_with_controls(&OwnedColumns::generic_dense(), &[], VCKSS_DELETION_MATCH);
    ok(vckss_rust_engine_solve_v4(generation, &valid.point));
    assert_eq!(
        vckss_rust_engine_numerical_result_v1(
            generation,
            &mut VckssNumericalResultV1::default(),
            512
        ),
        ErrorCode::UnsupportedFeature as i32
    );
    ok(vckss_rust_engine_release_v1(generation));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn numerical_auto_exact_is_zero_without_probe_or_replay_work() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let columns = OwnedColumns::generic_dense();
    let mut point = planned_solve_request(
        planned_capability_request(
            VCKSS_ALGORITHM_AUTO,
            VCKSS_ENGINE_AUTO_OR_UNSPECIFIED,
            VCKSS_ROUTE_AUTO,
            VCKSS_DELETION_MATCH,
            VCKSS_NUISANCE_JOINT,
            0,
            VCKSS_BATCH_MODE_AUTO,
            VCKSS_BATCH_MODE_AUTO,
        ),
        0,
        0,
    );
    point.v3.v2.exact_estimator_limit = 500;
    let request = VckssNumericalRequestV1 {
        point,
        ..VckssNumericalRequestV1::default()
    };
    ok(vckss_rust_engine_numerical_preflight_v1(&request));
    let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
    ok(vckss_rust_engine_solve_numerical_v1(generation, &request));
    let value = result(generation);
    assert_eq!(
        (value.status, value.engine),
        (1, VCKSS_ENGINE_NOT_APPLICABLE)
    );
    assert_eq!(
        (value.leverage_probes, value.target_probes, value.point_rhs),
        (0, 0, 0)
    );
    assert_eq!(
        (
            value.certified_replay_rhs,
            value.executed_replay_rhs,
            value.attempted_replay_rhs,
            value.replay_generator_words,
            value.allocation_bound_bytes
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(value.raw, [[0.0; 3]; 3]);
    assert_eq!(value.mcse, [0.0; 4]);
    let mut written = 99;
    ok(vckss_rust_engine_numerical_rhs_v1(
        generation,
        ptr::null_mut(),
        0,
        &mut written,
    ));
    assert_eq!(written, 0);
    ok(vckss_rust_engine_release_v1(generation));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn numerical_compressed_literal_cmg_preserves_the_point_route() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let columns = OwnedColumns::generic_dense();
    for route in [VCKSS_ROUTE_AUTO, VCKSS_ROUTE_CMG_PCG] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            let mut request = request(
                VCKSS_ENGINE_COMPRESSED,
                route,
                VCKSS_DELETION_MATCH,
                nuisance,
                0,
            );
            request.point = planned_solve_request(
                planned_capability_request(
                    VCKSS_ALGORITHM_JLA,
                    VCKSS_ENGINE_COMPRESSED,
                    route,
                    VCKSS_DELETION_MATCH,
                    nuisance,
                    0,
                    VCKSS_BATCH_MODE_EXPLICIT,
                    VCKSS_BATCH_MODE_EXPLICIT,
                ),
                7,
                5,
            );
            request.point.v3.v2.v1.probes = 33;
            let generation =
                prepare_with_controls_memory(&columns, &[], VCKSS_DELETION_MATCH, 1_u64 << 30);
            ok(vckss_rust_engine_solve_v4(generation, &request.point));
            let old = point(generation);
            ok(vckss_rust_engine_release_v1(generation));
            let generation =
                prepare_with_controls_memory(&columns, &[], VCKSS_DELETION_MATCH, 1_u64 << 30);
            ok(vckss_rust_engine_solve_numerical_v1(generation, &request));
            let new = point(generation);
            for (a, b) in component_bits(old.corrected)
                .into_iter()
                .zip(component_bits(new.corrected))
            {
                close(f64::from_bits(a), f64::from_bits(b));
            }
            for (a, b) in component_bits(old.numerical_mcse)
                .into_iter()
                .zip(component_bits(new.numerical_mcse))
            {
                close(f64::from_bits(a), f64::from_bits(b));
            }
            let numerical = result(generation);
            assert_eq!(numerical.engine, VCKSS_ENGINE_COMPRESSED);
            assert_eq!(numerical.certified_replay_rhs, 33);
            assert!(numerical.maximum_replay_complete_residual <= 1e-11);
            ok(vckss_rust_engine_release_v1(generation));
        }
    }
}

#[test]
fn numerical_v2_schema_preflight_intent_and_short_buffers_are_atomic() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(bytes::<VckssNumericalRequestV2>(), 344);
    assert_eq!(bytes::<VckssGenericExecutionReceiptV2>(), 176);
    assert_eq!(vckss_rust_numerical_schema_v2(), 2);
    let prefix = request(
        VCKSS_ENGINE_GENERIC,
        VCKSS_ROUTE_DIAGONAL_PCG,
        VCKSS_DELETION_MATCH,
        VCKSS_NUISANCE_JOINT,
        1,
    );
    let mut good = VckssNumericalRequestV2 {
        v1: prefix,
        execution_mode: if cfg!(target_os = "windows") { 0 } else { 1 },
        ..VckssNumericalRequestV2::default()
    };
    good.v1.struct_size = bytes::<VckssNumericalRequestV2>();
    good.v1.schema_version = 2;
    ok(vckss_rust_engine_numerical_preflight_v2(&good));
    for field in 0..10 {
        let mut bad = good;
        match field {
            0 => bad.v1.struct_size = 343,
            1 => bad.v1.schema_version = 1,
            2 => bad.v1.reserved = 1,
            3 => bad.v1.reserved_2 = 1,
            4 => bad.reserved = 1,
            5 => bad.execution_mode = 4,
            6 => bad.component_batch_mode = 2,
            7 => bad.full_cmg = 2,
            8 => bad.v1.threads = 0,
            _ => bad.v1.point.v3.request_signature ^= 1,
        }
        assert_ne!(vckss_rust_engine_numerical_preflight_v2(&bad), 0);
        assert_eq!(numerical_state(), 0);
    }
    let columns = OwnedColumns::generic_dense();
    let c = [one_generic_control(&columns)];
    let generation = prepare_with_controls(&columns, &c, VCKSS_DELETION_MATCH);
    let mut bad = good;
    bad.v1.projection_requested = 1;
    assert_eq!(
        vckss_rust_engine_solve_numerical_interrupt_v2(
            generation,
            &bad,
            None,
            std::ptr::null_mut(),
            1
        ),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(numerical_state(), 1);
    ok(vckss_rust_engine_solve_numerical_interrupt_v2(
        generation,
        &good,
        None,
        std::ptr::null_mut(),
        0,
    ));
    assert_eq!(result(generation).executed_replay_rhs, 33);
    ok(vckss_rust_engine_release_v1(generation));
}

#[test]
fn numerical_v2_legacy_executors_preserve_points_and_conditional_receipts() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    let columns = OwnedColumns::generic_dense();
    // Independently exercise the frozen V2 compressed executor with unequal
    // literal widths, rather than comparing two planned execution wrappers.
    let mut request = VckssEngineSolveRequestInterruptV2::default();
    request.options.algorithm = VCKSS_ALGORITHM_JLA;
    request.options.v1.probes = 33;
    request.options.v1.leverage_batch_width = 2;
    request.options.v1.target_batch_width = 3;
    let mut reference: Option<VckssEngineResultV1> = None;
    for numerical in [false, true] {
        let generation = prepare_with_controls(&columns, &[], VCKSS_DELETION_MATCH);
        ok(if numerical {
            vckss_rust_engine_solve_numerical_legacy_v2(generation, &request)
        } else {
            vckss_rust_engine_solve_interrupt_v2(generation, &request)
        });
        let value = point(generation);
        if let Some(old) = reference {
            assert_eq!(
                component_bits(value.corrected),
                component_bits(old.corrected)
            );
            assert_eq!(
                component_bits(value.numerical_mcse),
                component_bits(old.numerical_mcse)
            );
            assert_eq!(result(generation).executed_replay_rhs, 33);
        } else {
            reference = Some(value);
        }
        ok(vckss_rust_engine_release_v1(generation));
    }
    for deletion in [VCKSS_DELETION_MATCH, VCKSS_DELETION_OBSERVATION] {
        for nuisance in [VCKSS_NUISANCE_JOINT, VCKSS_NUISANCE_FIXED_OFFSET] {
            let c = [one_generic_control(&columns)];
            let (_, mut options) = generic_solve_request(deletion, nuisance, 1, 33);
            options.v2.v1.struct_size = bytes::<VckssEngineSolveRequestInterruptV3>();
            let request = VckssEngineSolveRequestInterruptV3 {
                options,
                ..VckssEngineSolveRequestInterruptV3::default()
            };
            let mut reference: Option<VckssEngineResultV1> = None;
            for numerical in [false, true] {
                let generation = prepare_with_controls(&columns, &c, deletion);
                ok(if numerical {
                    vckss_rust_engine_solve_numerical_generic_legacy_v2(generation, &request)
                } else {
                    vckss_rust_engine_solve_interrupt_v3(generation, &request)
                });
                let value = point(generation);
                if let Some(old) = reference {
                    assert_eq!(
                        component_bits(value.corrected),
                        component_bits(old.corrected)
                    );
                    assert_eq!(
                        component_bits(value.numerical_mcse),
                        component_bits(old.numerical_mcse)
                    );
                    assert_eq!(result(generation).executed_replay_rhs, 33);
                } else {
                    reference = Some(value);
                }
                ok(vckss_rust_engine_release_v1(generation));
            }
        }
    }
}

fn numerical_state() -> u32 {
    let mut value = VckssEngineSnapshotV1::default();
    ok(vckss_rust_engine_snapshot_v1(
        &mut value,
        bytes::<VckssEngineSnapshotV1>(),
    ));
    value.state
}

#[test]
fn numerical_v2_coordinated_interrupt_stays_on_caller_and_releases() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    struct Poll {
        caller: std::thread::ThreadId,
        calls: usize,
        wrong_thread: bool,
    }
    unsafe extern "C" fn poll(context: *mut std::ffi::c_void) -> i32 {
        let state = unsafe { &mut *context.cast::<Poll>() };
        state.calls += 1;
        state.wrong_thread |= std::thread::current().id() != state.caller;
        1
    }
    let columns = OwnedColumns::generic_dense();
    let c = [one_generic_control(&columns)];
    let generation = prepare_with_controls(&columns, &c, VCKSS_DELETION_MATCH);
    let mut request = VckssNumericalRequestV2 {
        v1: request(
            VCKSS_ENGINE_GENERIC,
            VCKSS_ROUTE_DIAGONAL_PCG,
            VCKSS_DELETION_MATCH,
            VCKSS_NUISANCE_JOINT,
            1,
        ),
        execution_mode: if cfg!(target_os = "windows") { 0 } else { 1 },
        ..VckssNumericalRequestV2::default()
    };
    request.v1.struct_size = bytes::<VckssNumericalRequestV2>();
    request.v1.schema_version = 2;
    let mut state = Poll {
        caller: std::thread::current().id(),
        calls: 0,
        wrong_thread: false,
    };
    assert_eq!(
        vckss_rust_engine_solve_numerical_interrupt_v2(
            generation,
            &request,
            Some(poll),
            (&mut state as *mut Poll).cast(),
            1
        ),
        ErrorCode::UserBreak as i32
    );
    assert!(state.calls > 0 && !state.wrong_thread);
    ok(vckss_rust_engine_release_v1(generation));
    let generation = prepare_with_controls(&columns, &c, VCKSS_DELETION_MATCH);
    ok(vckss_rust_engine_solve_numerical_interrupt_v2(
        generation,
        &request,
        None,
        std::ptr::null_mut(),
        0,
    ));
    assert_eq!(result(generation).executed_replay_rhs, 33);
    ok(vckss_rust_engine_release_v1(generation));
}

#[cfg(target_os = "windows")]
#[test]
fn numerical_windows_keeps_v1_disabled_while_v2_is_available() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset();
    assert_eq!(vckss_rust_numerical_schema_v1(), 0);
    assert_eq!(vckss_rust_numerical_schema_v2(), 2);
    assert_eq!(
        vckss_rust_engine_numerical_preflight_v1(&VckssNumericalRequestV1::default()),
        ErrorCode::UnsupportedFeature as i32
    );
}
