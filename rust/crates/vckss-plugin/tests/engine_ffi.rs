// SPDX-License-Identifier: GPL-3.0-only

use std::ffi::{c_void, CStr};
use std::mem::{offset_of, size_of};
use std::ptr;
use std::sync::Mutex;

use vckss_core::error::ErrorCode;
use vckss_core::ABI_VERSION;
use vckss_plugin::ffi_engine::{
    vckss_rust_backend_capabilities_v1, vckss_rust_engine_admit_prepare_v2,
    vckss_rust_engine_clear_abandoned_v1, vckss_rust_engine_default_prepare_request_interrupt_v1,
    vckss_rust_engine_default_solve_request_interrupt_v1,
    vckss_rust_engine_default_solve_request_v1, vckss_rust_engine_detailed_receipt_v2,
    vckss_rust_engine_detailed_receipt_v3, vckss_rust_engine_last_error,
    vckss_rust_engine_preparation_receipt_v1, vckss_rust_engine_preparation_receipt_v2,
    vckss_rust_engine_preparation_receipt_v3, vckss_rust_engine_prepare_interrupt_v1,
    vckss_rust_engine_prepare_v1, vckss_rust_engine_prepare_v2, vckss_rust_engine_release_v1,
    vckss_rust_engine_result_v1, vckss_rust_engine_retained_mask_v1,
    vckss_rust_engine_rhs_receipts_v1, vckss_rust_engine_snapshot_v1,
    vckss_rust_engine_solve_interrupt_v1, vckss_rust_engine_solve_v1,
    vckss_rust_session_clear_abandoned_v1, vckss_rust_session_last_error,
    vckss_rust_session_preparation_receipt_v1, vckss_rust_session_prepare_v1,
    vckss_rust_session_release_v1, vckss_rust_session_snapshot_v1, VckssBackendCapabilitiesV1,
    VckssColumnsV1, VckssEngineColumnsV1, VckssEngineDetailedReceiptV1,
    VckssEngineDetailedReceiptV2, VckssEngineDetailedReceiptV3, VckssEnginePreparationReceiptV1,
    VckssEnginePreparationReceiptV2, VckssEnginePreparationReceiptV3,
    VckssEnginePrepareRequestInterruptV1, VckssEnginePrepareRequestV1, VckssEnginePrepareRequestV2,
    VckssEngineResultV1, VckssEngineRhsReceiptV1, VckssEngineSnapshotV1,
    VckssEngineSolveRequestInterruptV1, VckssEngineSolveRequestV1, VckssPreparationReceiptV1,
    VckssPrepareRequestV1, VckssSessionSnapshotV1, VCKSS_CORE_JLA_PLAN_READY,
    VCKSS_INTERRUPT_CONTINUE, VCKSS_INTERRUPT_USER_BREAK, VCKSS_ROUTE_EXACT,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
#[repr(C)]
struct PollState {
    calls: u32,
    stop_at: u32,
    terminal_status: i32,
}

unsafe extern "C" fn injected_poll(context: *mut c_void) -> i32 {
    // SAFETY: each test passes a live, uniquely borrowed PollState for the
    // duration of the synchronous solve and the engine never retains it.
    let state = unsafe { &mut *context.cast::<PollState>() };
    state.calls += 1;
    if state.calls >= state.stop_at {
        state.terminal_status
    } else {
        VCKSS_INTERRUPT_CONTINUE
    }
}

fn bytes<T>() -> u32 {
    u32::try_from(size_of::<T>()).expect("ABI structure size")
}

#[derive(Debug)]
struct OwnedColumns {
    worker: Vec<f64>,
    firm: Vec<f64>,
    deletion: Vec<f64>,
    outcome: Vec<f64>,
    frequency: Vec<f64>,
    target_weight: Vec<f64>,
}

impl OwnedColumns {
    fn dense() -> Self {
        let workers = 12_usize;
        let firms = 4_usize;
        let rows = workers * firms;
        let mut value = Self {
            worker: Vec::with_capacity(rows),
            firm: Vec::with_capacity(rows),
            deletion: Vec::with_capacity(rows),
            outcome: Vec::with_capacity(rows),
            frequency: Vec::with_capacity(rows),
            target_weight: Vec::with_capacity(rows),
        };
        for worker in 0..workers {
            for firm in 0..firms {
                value.worker.push((worker + 1) as f64);
                value.firm.push((firm + 1) as f64);
                value.deletion.push((worker * firms + firm + 1) as f64);
                let sign = if (worker + firm) % 2 == 0 { 1.0 } else { -1.0 };
                value.outcome.push(sign * (2 * worker + firm + 1) as f64);
                value.frequency.push((firm % 2 + 1) as f64);
                value.target_weight.push((worker % 3 + firm + 1) as f64);
            }
        }
        value
    }

    fn disconnected_components() -> Self {
        Self {
            worker: vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 4.0],
            firm: vec![1.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 3.0, 4.0, 5.0],
            deletion: (1..=10).map(f64::from).collect(),
            outcome: vec![1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 1.0, -1.0, 2.0, -2.0],
            frequency: vec![100.0, 100.0, 100.0, 100.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            target_weight: vec![1.0; 10],
        }
    }

    fn descriptor(&self) -> VckssEngineColumnsV1 {
        VckssEngineColumnsV1 {
            struct_size: bytes::<VckssEngineColumnsV1>(),
            reserved: 0,
            rows: self.worker.len() as u64,
            worker: self.worker.as_ptr(),
            firm: self.firm.as_ptr(),
            deletion: self.deletion.as_ptr(),
            outcome: self.outcome.as_ptr(),
            frequency: self.frequency.as_ptr(),
            target_weight: self.target_weight.as_ptr(),
        }
    }

    fn request(&self) -> VckssEnginePrepareRequestV1 {
        VckssEnginePrepareRequestV1 {
            abi_version: ABI_VERSION,
            struct_size: bytes::<VckssEnginePrepareRequestV1>(),
            rows: self.worker.len() as u64,
            cleanup_abandoned: 0,
            reserved: 0,
        }
    }

    fn request_v2(&self) -> VckssEnginePrepareRequestV2 {
        VckssEnginePrepareRequestV2 {
            abi_version: ABI_VERSION,
            struct_size: bytes::<VckssEnginePrepareRequestV2>(),
            rows: self.worker.len() as u64,
            cleanup_abandoned: 0,
            reserved: 0,
            memory_limit_bytes: 64_u64 << 20,
            caller_copy_bytes: self.worker.len() as u64 * 6 * 8,
        }
    }
}

fn reset() {
    assert_eq!(vckss_rust_engine_clear_abandoned_v1(), ErrorCode::Ok as i32);
}

#[test]
fn public_abi_layout_and_structured_capabilities_are_frozen() {
    assert_eq!(size_of::<VckssBackendCapabilitiesV1>(), 32);
    assert_eq!(size_of::<VckssEnginePrepareRequestV1>(), 24);
    assert_eq!(size_of::<VckssEnginePrepareRequestV2>(), 40);
    assert_eq!(size_of::<VckssEnginePrepareRequestInterruptV1>(), 64);
    assert_eq!(size_of::<VckssEngineColumnsV1>(), 64);
    assert_eq!(size_of::<VckssEngineSolveRequestV1>(), 176);
    assert_eq!(size_of::<VckssEngineSolveRequestInterruptV1>(), 200);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV1>(), 72);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV2>(), 248);
    assert_eq!(size_of::<VckssEnginePreparationReceiptV3>(), 256);
    assert_eq!(size_of::<VckssEngineResultV1>(), 144);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV1>(), 272);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV2>(), 360);
    assert_eq!(size_of::<VckssEngineDetailedReceiptV3>(), 384);
    assert_eq!(size_of::<VckssEngineRhsReceiptV1>(), 48);
    assert_eq!(size_of::<VckssEngineSnapshotV1>(), 24);
    assert_eq!(size_of::<VckssPrepareRequestV1>(), 24);
    assert_eq!(size_of::<VckssColumnsV1>(), 64);
    assert_eq!(size_of::<VckssPreparationReceiptV1>(), 72);
    assert_eq!(size_of::<VckssSessionSnapshotV1>(), 24);
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestV2, memory_limit_bytes),
        24
    );
    assert_eq!(offset_of!(VckssEnginePrepareRequestInterruptV1, options), 0);
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, interrupt_poll),
        40
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, interrupt_context),
        48
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, checkpoint_interval),
        56
    );
    assert_eq!(
        offset_of!(VckssEnginePrepareRequestInterruptV1, reserved),
        60
    );
    assert_eq!(
        offset_of!(VckssEnginePreparationReceiptV2, memory_limit_bytes),
        72
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestV1, cmg_memory_limit_bytes),
        168
    );
    assert_eq!(offset_of!(VckssEngineSolveRequestInterruptV1, options), 0);
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV1, interrupt_poll),
        176
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV1, interrupt_context),
        184
    );
    assert_eq!(
        offset_of!(VckssEngineSolveRequestInterruptV1, checkpoint_interval),
        192
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV1, cmg_dense_factor_bytes),
        264
    );
    assert_eq!(
        offset_of!(VckssEngineDetailedReceiptV2, full_fit_weighted_rss),
        272
    );
    assert_eq!(
        offset_of!(VckssEnginePreparationReceiptV3, target_weight_sum),
        248
    );
    assert_eq!(offset_of!(VckssEngineDetailedReceiptV3, rng_contract), 360);
    assert_eq!(offset_of!(VckssEngineRhsReceiptV1, reduced_residual), 32);

    let mut capabilities = VckssBackendCapabilitiesV1::default();
    assert_eq!(
        vckss_rust_backend_capabilities_v1(
            &mut capabilities,
            bytes::<VckssBackendCapabilitiesV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(capabilities.struct_size, 32);
    assert_eq!(capabilities.abi_version, ABI_VERSION);
    assert_ne!(capabilities.core_ready_flags & VCKSS_CORE_JLA_PLAN_READY, 0);
    assert_eq!(capabilities.support_flags, 38);
    assert_eq!(capabilities.deterministic_parallelism, 1);
    assert_eq!(capabilities.reserved, 0);
}

#[test]
fn frozen_session_aliases_share_the_engine_registry_and_error_slot() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let request = columns.request();
    let descriptor = columns.descriptor();

    let mut session_generation = 0_u64;
    assert_eq!(
        vckss_rust_session_prepare_v1(&request, &descriptor, &mut session_generation),
        ErrorCode::Ok as i32
    );
    let mut engine_snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut engine_snapshot, bytes::<VckssEngineSnapshotV1>(),),
        ErrorCode::Ok as i32
    );
    assert_eq!(engine_snapshot.generation, session_generation);
    assert_eq!(engine_snapshot.state, 1);
    let mut legacy_receipt = VckssPreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_session_preparation_receipt_v1(session_generation, &mut legacy_receipt),
        ErrorCode::Ok as i32
    );
    assert_eq!(legacy_receipt.generation, session_generation);
    assert_eq!(legacy_receipt.input_rows, columns.worker.len() as u64);
    assert_eq!(
        vckss_rust_engine_release_v1(session_generation),
        ErrorCode::Ok as i32
    );

    let engine_generation = prepare(&columns);
    let mut legacy_snapshot = VckssSessionSnapshotV1::default();
    assert_eq!(
        vckss_rust_session_snapshot_v1(&mut legacy_snapshot),
        ErrorCode::Ok as i32
    );
    assert_eq!(legacy_snapshot.generation, engine_generation);
    assert_eq!(legacy_snapshot.state, 1);
    assert_eq!(
        vckss_rust_session_release_v1(engine_generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_session_clear_abandoned_v1(),
        ErrorCode::Ok as i32
    );
    let error = unsafe { CStr::from_ptr(vckss_rust_session_last_error()) };
    assert_eq!(error.to_str().expect("session error utf-8"), "OK");
}

fn prepare(columns: &OwnedColumns) -> u64 {
    let mut generation = 0_u64;
    let request = columns.request_v2();
    assert_eq!(
        vckss_rust_engine_admit_prepare_v2(&request),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_prepare_v2(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    generation
}

fn prepare_interrupt_request(
    columns: &OwnedColumns,
    poll: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    context: *mut c_void,
    interval: u32,
) -> VckssEnginePrepareRequestInterruptV1 {
    let mut request = VckssEnginePrepareRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_prepare_request_interrupt_v1(
            &mut request,
            bytes::<VckssEnginePrepareRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    let options = columns.request_v2();
    request.options.abi_version = options.abi_version;
    request.options.rows = options.rows;
    request.options.cleanup_abandoned = options.cleanup_abandoned;
    request.options.memory_limit_bytes = options.memory_limit_bytes;
    request.options.caller_copy_bytes = options.caller_copy_bytes;
    request.interrupt_poll = poll;
    request.interrupt_context = context;
    request.checkpoint_interval = interval;
    request
}

#[test]
fn interrupt_prepare_default_and_contract_are_frozen() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let request = prepare_interrupt_request(&columns, None, ptr::null_mut(), 0);
    assert_eq!(request.options.struct_size, 64);
    assert!(request.interrupt_poll.is_none());
    assert!(request.interrupt_context.is_null());
    assert_eq!(request.checkpoint_interval, 0);
    assert_eq!(request.reserved, 0);
    assert_eq!(
        vckss_rust_engine_admit_prepare_v2(&request.options),
        ErrorCode::Ok as i32
    );

    let invalid = prepare_interrupt_request(&columns, None, ptr::null_mut(), 1);
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &invalid,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(generation, 0);

    let malformed_header = [ABI_VERSION, 8_u32];
    generation = 99;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            malformed_header
                .as_ptr()
                .cast::<VckssEnginePrepareRequestInterruptV1>(),
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(generation, 0);

    generation = 99;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            malformed_header
                .as_ptr()
                .cast::<VckssEnginePrepareRequestInterruptV1>(),
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>() - 1,
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(generation, 99);
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            malformed_header
                .as_ptr()
                .cast::<VckssEnginePrepareRequestInterruptV1>(),
            &columns.descriptor(),
            ptr::null_mut(),
            bytes::<u64>(),
        ),
        ErrorCode::InvalidInput as i32
    );

    let mut state = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: 77,
    };
    let unknown = prepare_interrupt_request(
        &columns,
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    generation = 99;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &unknown,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::InternalInvariantFailed as i32
    );
    assert_eq!(generation, 0);
}

#[test]
fn prepare_user_breaks_leave_zero_generation_and_empty_registry() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();

    let mut counting = PollState {
        calls: 0,
        stop_at: u32::MAX,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let request = prepare_interrupt_request(
        &columns,
        Some(injected_poll),
        (&mut counting as *mut PollState).cast(),
        1,
    );
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    let total = counting.calls;
    assert!(total > 16, "fixture must cross multiple preparation phases");
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let stops = [1, 2, total / 4, total / 2, total - 1];
    for stop_at in stops {
        let mut state = PollState {
            calls: 0,
            stop_at,
            terminal_status: VCKSS_INTERRUPT_USER_BREAK,
        };
        let request = prepare_interrupt_request(
            &columns,
            Some(injected_poll),
            (&mut state as *mut PollState).cast(),
            1,
        );
        generation = 99;
        assert_eq!(
            vckss_rust_engine_prepare_interrupt_v1(
                &request,
                &columns.descriptor(),
                &mut generation,
                bytes::<u64>(),
            ),
            ErrorCode::UserBreak as i32,
            "stop_at={stop_at}"
        );
        assert_eq!(generation, 0);
        assert_eq!(state.calls, stop_at);
        let mut snapshot = VckssEngineSnapshotV1::default();
        assert_eq!(
            vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
            ErrorCode::Ok as i32
        );
        assert_eq!(snapshot.state, 0);
        assert_eq!(snapshot.generation, 0);
    }
}

#[test]
fn interrupt_prepare_preserves_cleanup_abandoned_contract() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let abandoned = prepare(&columns);

    let mut request = prepare_interrupt_request(&columns, None, ptr::null_mut(), 0);
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::ContextPoisoned as i32
    );
    assert_eq!(generation, 0);

    request.options.cleanup_abandoned = 1;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_ne!(generation, abandoned);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn cleanup_releases_the_old_generation_before_replacement_ingest_can_fail() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let abandoned = prepare(&columns);
    let mut state = PollState {
        calls: 0,
        stop_at: 2,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let mut request = prepare_interrupt_request(
        &columns,
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    request.options.cleanup_abandoned = 1;
    let mut generation = 99_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(generation, 0);
    assert_eq!(state.calls, 2);
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 0);
    assert_eq!(snapshot.generation, 0);
    assert_eq!(snapshot.last_released_generation, abandoned);
}

#[test]
fn no_callback_prepare_schema_matches_v2_receipt_and_mask() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut expected_receipt = VckssEnginePreparationReceiptV2::default();
    let mut expected_mask = vec![0_u8; columns.worker.len()];
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut expected_receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(
            generation,
            expected_mask.as_mut_ptr(),
            expected_mask.len() as u64,
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let request = prepare_interrupt_request(&columns, None, ptr::null_mut(), 0);
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_interrupt_v1(
            &request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut actual_receipt = VckssEnginePreparationReceiptV2::default();
    let mut actual_mask = vec![0_u8; columns.worker.len()];
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut actual_receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(
            generation,
            actual_mask.as_mut_ptr(),
            actual_mask.len() as u64,
        ),
        ErrorCode::Ok as i32
    );
    expected_receipt.generation = 0;
    actual_receipt.generation = 0;
    assert_eq!(expected_receipt, actual_receipt);
    assert_eq!(expected_mask, actual_mask);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn one_engine_generation_exports_science_mask_and_fixed_receipts() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);

    let mut preparation = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.generation, generation);
    assert_eq!(preparation.input_rows, 48);
    assert_eq!(preparation.retained_rows, 48);
    assert_eq!(preparation.memory_limit_bytes, 64_u64 << 20);
    assert_eq!(preparation.caller_copy_bytes, 48 * 6 * 8);
    assert!(preparation.preparation_peak_forecast_bytes >= preparation.caller_copy_bytes);
    assert!(preparation.prepared_resident_bytes > 0);
    assert_eq!(preparation.graph_input_rows, 48);
    assert_eq!(preparation.graph_retained_rows, 48);
    assert_eq!(preparation.graph_input_physical_mass, 72);
    assert_eq!(preparation.graph_retained_physical_mass, 72);
    assert_eq!(preparation.graph_initial_components, 1);
    assert_eq!(preparation.graph_maximum_components, 1);
    assert_eq!(preparation.graph_initial_component_rows, 48);
    assert_eq!(preparation.graph_mover_input_rows, 48);
    assert_eq!(preparation.graph_initial_deletion_edges, 48);
    assert_eq!(preparation.graph_retained_deletion_edges, 48);

    let mut preparation_v3 = VckssEnginePreparationReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v3(
            generation,
            &mut preparation_v3,
            bytes::<VckssEnginePreparationReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation_v3.v2, preparation);
    assert_eq!(
        preparation_v3.target_weight_sum,
        columns.target_weight.iter().sum()
    );

    let mut mask = vec![0_u8; 48];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));

    let mut solve = VckssEngineSolveRequestV1::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_v1(
            &mut solve,
            bytes::<VckssEngineSolveRequestV1>(),
        ),
        ErrorCode::Ok as i32
    );
    solve.seed = 91_827;
    solve.probes = 6;
    solve.leverage_batch_width = 3;
    solve.target_batch_width = 2;
    solve.solver_route = VCKSS_ROUTE_EXACT;
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &solve),
        ErrorCode::Ok as i32
    );

    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>(),),
        ErrorCode::Ok as i32
    );
    for ((plugin, correction), corrected) in [
        result.plugin.worker,
        result.plugin.firm,
        result.plugin.covariance,
        result.plugin.total,
    ]
    .into_iter()
    .zip([
        result.correction.worker,
        result.correction.firm,
        result.correction.covariance,
        result.correction.total,
    ])
    .zip([
        result.corrected.worker,
        result.corrected.firm,
        result.corrected.covariance,
        result.corrected.total,
    ]) {
        assert!((plugin - correction - corrected).abs() <= 1.0e-10);
    }
    assert!(result.numerical_mcse.worker.is_finite());
    assert!(result.numerical_mcse.worker >= 0.0);

    let mut receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.seed, solve.seed);
    assert_eq!(receipt.probes_requested, solve.probes);
    assert_eq!(receipt.solver_requested, VCKSS_ROUTE_EXACT);
    assert_eq!(receipt.solver_selected, VCKSS_ROUTE_EXACT);
    assert_eq!(receipt.leverage_batch_width, 3);
    assert_eq!(receipt.target_batch_width, 2);
    assert_eq!(receipt.leverage_rhs_count, 6);
    assert_eq!(receipt.target_rhs_count, 12);
    assert!(receipt.full_fit_weighted_rss.is_finite());
    assert!(receipt.full_fit_weighted_rss > 0.0);
    assert_eq!(receipt.memory_limit_bytes, preparation.memory_limit_bytes);
    assert_eq!(receipt.caller_copy_bytes, preparation.caller_copy_bytes);
    assert_eq!(
        receipt.prepared_resident_bytes,
        preparation.prepared_resident_bytes
    );
    assert_eq!(
        receipt.command_peak_forecast_bytes,
        receipt
            .preparation_peak_forecast_bytes
            .max(receipt.solve_peak_forecast_bytes)
    );
    assert!(receipt.command_peak_forecast_bytes <= receipt.memory_limit_bytes);

    let mut receipt_v3 = VckssEngineDetailedReceiptV3::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v3(
            generation,
            &mut receipt_v3,
            bytes::<VckssEngineDetailedReceiptV3>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt_v3.v2, receipt);
    assert_eq!(receipt_v3.rng_contract, 1);
    assert_eq!(receipt_v3.rhs_receipt_rows, 19);
    assert_eq!(receipt_v3.caller_result_copy_bytes, 19 * 14 * 8);
    assert!(receipt_v3.v2.result_forecast_bytes >= receipt_v3.caller_result_copy_bytes);

    let mut rhs = vec![VckssEngineRhsReceiptV1::default(); 19];
    assert_eq!(
        vckss_rust_engine_rhs_receipts_v1(generation, rhs.as_mut_ptr(), rhs.len() as u64),
        ErrorCode::Ok as i32
    );
    assert_eq!((rhs[0].phase, rhs[0].side, rhs[0].probe), (1, 0, -1));
    for (probe, row) in rhs[1..7].iter().enumerate() {
        assert_eq!((row.phase, row.side, row.probe), (2, 0, probe as i64));
    }
    for probe in 0..6 {
        let worker = rhs[7 + 2 * probe];
        let firm = rhs[8 + 2 * probe];
        assert_eq!(
            (worker.phase, worker.side, worker.probe),
            (3, 1, probe as i64)
        );
        assert_eq!((firm.phase, firm.side, firm.probe), (3, 2, probe as i64));
    }
    assert!(rhs.iter().all(|row| row.route == VCKSS_ROUTE_EXACT));
    assert!(rhs.iter().all(|row| row.complete_residual.is_finite()));

    // Preparation metadata remains in the same generation after solve.
    preparation = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    mask.fill(0);
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));

    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn v2_memory_admission_fails_before_column_descriptor_access() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let mut request = columns.request_v2();
    request.memory_limit_bytes = request.caller_copy_bytes;
    assert_eq!(
        vckss_rust_engine_admit_prepare_v2(&request),
        ErrorCode::ResourceLimit as i32
    );

    let mut generation = 77_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(&request, ptr::null(), &mut generation, bytes::<u64>(),),
        ErrorCode::ResourceLimit as i32
    );
    assert_eq!(generation, 0);
    // SAFETY: engine error strings are thread-local and valid until this
    // thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().contains("simultaneous C/Rust"));
}

#[test]
fn invalid_frequency_weight_has_the_weight_error_code() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let mut columns = OwnedColumns::dense();
    columns.frequency[0] = -1.0;
    let request = columns.request_v2();
    let descriptor = columns.descriptor();
    let mut generation = 77_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(&request, &descriptor, &mut generation, bytes::<u64>(),),
        ErrorCode::InvalidWeight as i32
    );
    assert_eq!(generation, 0);
    // SAFETY: engine error strings are thread-local and valid until this
    // thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().starts_with("INVALID_WEIGHT "));
    assert!(error.to_string_lossy().contains("frequency weight"));
}

#[test]
fn headers_and_every_output_capacity_fail_before_full_value_access_or_write() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let descriptor = columns.descriptor();
    let short_request = [ABI_VERSION, 4_u32];
    let mut generation = 77_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v1(
            short_request.as_ptr().cast::<VckssEnginePrepareRequestV1>(),
            &descriptor,
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(generation, 77);

    let request = columns.request();
    assert_eq!(
        vckss_rust_engine_prepare_v1(&request, &descriptor, &mut generation, 7),
        ErrorCode::AbiMismatch as i32
    );
    generation = prepare(&columns);

    let mut receipt = VckssEnginePreparationReceiptV1 {
        generation: 123,
        ..VckssEnginePreparationReceiptV1::default()
    };
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v1(
            generation,
            &mut receipt,
            bytes::<VckssEnginePreparationReceiptV1>() - 1,
        ),
        ErrorCode::AbiMismatch as i32
    );
    assert_eq!(receipt.generation, 123);
    let mut short_mask = vec![9_u8; columns.worker.len() - 1];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(
            generation,
            short_mask.as_mut_ptr(),
            short_mask.len() as u64,
        ),
        ErrorCode::InvalidInput as i32
    );
    assert!(short_mask.iter().all(|&value| value == 9));
    assert_eq!(
        vckss_rust_engine_result_v1(generation, ptr::null_mut(), 0),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn failed_solve_preserves_authoritative_preparation_and_typed_error() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let solve = VckssEngineSolveRequestV1 {
        probes: 1,
        ..VckssEngineSolveRequestV1::default()
    };
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &solve),
        ErrorCode::InvalidInput as i32
    );
    // SAFETY: engine error strings are thread-local and valid until this
    // thread's next engine ABI call.
    let error = unsafe { CStr::from_ptr(vckss_rust_engine_last_error()) };
    assert!(error.to_string_lossy().contains("at least two"));

    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>(),),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 4);
    let mut preparation = VckssEnginePreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v1(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.input_rows, 48);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

fn interrupt_request(
    poll: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
    context: *mut c_void,
    interval: u32,
) -> VckssEngineSolveRequestInterruptV1 {
    let mut request = VckssEngineSolveRequestInterruptV1::default();
    assert_eq!(
        vckss_rust_engine_default_solve_request_interrupt_v1(
            &mut request,
            bytes::<VckssEngineSolveRequestInterruptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    request.options.seed = 91_827;
    request.options.probes = 20;
    request.options.leverage_batch_width = 4;
    request.options.target_batch_width = 4;
    request.options.solver_route = VCKSS_ROUTE_EXACT;
    request.interrupt_poll = poll;
    request.interrupt_context = context;
    request.checkpoint_interval = interval;
    request
}

#[test]
fn immediate_user_break_preserves_context_until_release_and_rerun() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut state = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let request = interrupt_request(
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &request),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(state.calls, 1);

    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 4);
    assert_eq!(snapshot.generation, generation);

    let mut preparation = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    let mut mask = vec![0_u8; columns.worker.len()];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64,),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));
    let mut result = VckssEngineResultV1::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut result, bytes::<VckssEngineResultV1>()),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let rerun_generation = prepare(&columns);
    let mut continue_state = PollState {
        calls: 0,
        stop_at: u32::MAX,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let rerun = interrupt_request(
        Some(injected_poll),
        (&mut continue_state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(rerun_generation, &rerun),
        ErrorCode::Ok as i32
    );
    assert!(continue_state.calls > 1);
    assert_eq!(
        vckss_rust_engine_release_v1(rerun_generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn deep_user_break_stops_at_the_requested_checkpoint() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut state = PollState {
        calls: 0,
        stop_at: 40,
        terminal_status: VCKSS_INTERRUPT_USER_BREAK,
    };
    let request = interrupt_request(
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &request),
        ErrorCode::UserBreak as i32
    );
    assert_eq!(state.calls, 40);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn invalid_and_unknown_callback_contracts_fail_closed() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);

    let invalid = interrupt_request(None, ptr::null_mut(), 1);
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &invalid),
        ErrorCode::InvalidInput as i32
    );
    let mut snapshot = VckssEngineSnapshotV1::default();
    assert_eq!(
        vckss_rust_engine_snapshot_v1(&mut snapshot, bytes::<VckssEngineSnapshotV1>()),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 1);

    let mut state = PollState {
        calls: 0,
        stop_at: 1,
        terminal_status: 77,
    };
    let unknown = interrupt_request(
        Some(injected_poll),
        (&mut state as *mut PollState).cast(),
        1,
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &unknown),
        ErrorCode::InternalInvariantFailed as i32
    );
    assert_eq!(state.calls, 1);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn no_callback_interrupt_schema_is_bitwise_identical_to_v1() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let v1 = VckssEngineSolveRequestV1 {
        seed: 91_827,
        probes: 20,
        leverage_batch_width: 4,
        target_batch_width: 4,
        solver_route: VCKSS_ROUTE_EXACT,
        ..VckssEngineSolveRequestV1::default()
    };
    assert_eq!(
        vckss_rust_engine_solve_v1(generation, &v1),
        ErrorCode::Ok as i32
    );
    let mut v1_result = VckssEngineResultV1::default();
    let mut v1_receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_result_v1(generation, &mut v1_result, bytes::<VckssEngineResultV1>(),),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut v1_receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let generation = prepare(&columns);
    let interrupt = interrupt_request(None, ptr::null_mut(), 0);
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &interrupt),
        ErrorCode::Ok as i32
    );
    let mut interrupt_result = VckssEngineResultV1::default();
    let mut interrupt_receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_result_v1(
            generation,
            &mut interrupt_result,
            bytes::<VckssEngineResultV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut interrupt_receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    v1_result.generation = 0;
    interrupt_result.generation = 0;
    v1_receipt.generation = 0;
    interrupt_receipt.generation = 0;
    assert_eq!(v1_result, interrupt_result);
    assert_eq!(v1_receipt, interrupt_receipt);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn v2_exports_nontrivial_graph_selection_receipt() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::disconnected_components();
    let generation = prepare(&columns);
    let mut receipt = VckssEnginePreparationReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEnginePreparationReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.graph_input_rows, 10);
    assert_eq!(receipt.graph_retained_rows, 6);
    assert_eq!(receipt.graph_input_physical_mass, 406);
    assert_eq!(receipt.graph_retained_physical_mass, 6);
    assert_eq!(receipt.graph_initial_components, 2);
    assert_eq!(receipt.graph_maximum_components, 2);
    assert_eq!(receipt.graph_initial_component_rows, 6);
    assert_eq!(receipt.graph_mover_input_rows, 6);
    assert_eq!(receipt.graph_initial_deletion_edges, 6);
    assert_eq!(receipt.graph_retained_deletion_edges, 6);
    let mut mask = vec![0_u8; 10];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), 10),
        ErrorCode::Ok as i32
    );
    assert_eq!(mask, vec![0, 0, 0, 0, 1, 1, 1, 1, 1, 1]);
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn tight_memory_limit_covers_single_mask_and_allocation_free_finalization() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);
    let mut solve = interrupt_request(None, ptr::null_mut(), 0);
    solve.options.probes = 256;
    solve.options.leverage_batch_width = 8;
    solve.options.target_batch_width = 8;
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &solve),
        ErrorCode::Ok as i32
    );
    let mut receipt = VckssEngineDetailedReceiptV2::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v2(
            generation,
            &mut receipt,
            bytes::<VckssEngineDetailedReceiptV2>(),
        ),
        ErrorCode::Ok as i32
    );
    let exact_limit = receipt.command_peak_forecast_bytes;
    assert_eq!(exact_limit, receipt.solve_peak_forecast_bytes);
    assert!(
        receipt.solve_peak_forecast_bytes > receipt.preparation_peak_forecast_bytes,
        "fixture must exercise solve-time admission"
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    let mut prepare_request = columns.request_v2();
    prepare_request.memory_limit_bytes = exact_limit;
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(
            &prepare_request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &solve),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );

    prepare_request.memory_limit_bytes = exact_limit - 1;
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v2(
            &prepare_request,
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_engine_solve_interrupt_v1(generation, &solve),
        ErrorCode::ResourceLimit as i32
    );
    assert_eq!(
        vckss_rust_engine_release_v1(generation),
        ErrorCode::Ok as i32
    );
}
