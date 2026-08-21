// SPDX-License-Identifier: GPL-3.0-only

use std::ffi::CStr;
use std::mem::size_of;
use std::ptr;
use std::sync::Mutex;

use vckss_core::error::ErrorCode;
use vckss_core::ABI_VERSION;
use vckss_plugin::ffi_engine::{
    vckss_rust_engine_clear_abandoned_v1, vckss_rust_engine_detailed_receipt_v1,
    vckss_rust_engine_last_error, vckss_rust_engine_preparation_receipt_v1,
    vckss_rust_engine_prepare_v1, vckss_rust_engine_release_v1, vckss_rust_engine_result_v1,
    vckss_rust_engine_retained_mask_v1, vckss_rust_engine_snapshot_v1, vckss_rust_engine_solve_v1,
    VckssEngineColumnsV1, VckssEngineDetailedReceiptV1, VckssEnginePreparationReceiptV1,
    VckssEnginePrepareRequestV1, VckssEngineResultV1, VckssEngineSnapshotV1,
    VckssEngineSolveRequestV1, VCKSS_ROUTE_EXACT,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

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
}

fn reset() {
    assert_eq!(vckss_rust_engine_clear_abandoned_v1(), ErrorCode::Ok as i32);
}

fn prepare(columns: &OwnedColumns) -> u64 {
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_engine_prepare_v1(
            &columns.request(),
            &columns.descriptor(),
            &mut generation,
            bytes::<u64>(),
        ),
        ErrorCode::Ok as i32
    );
    generation
}

#[test]
fn one_engine_generation_exports_science_mask_and_fixed_receipts() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns);

    let mut preparation = VckssEnginePreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v1(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV1>(),
        ),
        ErrorCode::Ok as i32
    );
    assert_eq!(preparation.generation, generation);
    assert_eq!(preparation.input_rows, 48);
    assert_eq!(preparation.retained_rows, 48);

    let mut mask = vec![0_u8; 48];
    assert_eq!(
        vckss_rust_engine_retained_mask_v1(generation, mask.as_mut_ptr(), mask.len() as u64),
        ErrorCode::Ok as i32
    );
    assert!(mask.iter().all(|&value| value == 1));

    let solve = VckssEngineSolveRequestV1 {
        seed: 91_827,
        probes: 6,
        leverage_batch_width: 3,
        target_batch_width: 2,
        solver_route: VCKSS_ROUTE_EXACT,
        ..VckssEngineSolveRequestV1::default()
    };
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

    let mut receipt = VckssEngineDetailedReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_detailed_receipt_v1(
            generation,
            &mut receipt,
            bytes::<VckssEngineDetailedReceiptV1>(),
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

    // Preparation metadata remains in the same generation after solve.
    preparation = VckssEnginePreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_engine_preparation_receipt_v1(
            generation,
            &mut preparation,
            bytes::<VckssEnginePreparationReceiptV1>(),
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
