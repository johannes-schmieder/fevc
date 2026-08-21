// SPDX-License-Identifier: GPL-3.0-only

use std::ffi::CStr;
use std::mem::size_of;
use std::ptr;
use std::sync::Mutex;

use vckss_plugin::ffi_session::{
    vckss_rust_session_clear_abandoned_v1, vckss_rust_session_last_error,
    vckss_rust_session_prepare_v1, vckss_rust_session_preparation_receipt_v1,
    vckss_rust_session_release_v1, vckss_rust_session_snapshot_v1, VckssColumnsV1,
    VckssPreparationReceiptV1, VckssPrepareRequestV1, VckssSessionSnapshotV1,
};
use vckss_core::error::ErrorCode;
use vckss_core::ABI_VERSION;

static TEST_LOCK: Mutex<()> = Mutex::new(());

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
        for worker_index in 0..workers {
            for firm_index in 0..firms {
                value.worker.push(f64::from(
                    u32::try_from(worker_index + 1).expect("worker"),
                ));
                value.firm.push(f64::from(
                    u32::try_from(firm_index + 1).expect("firm"),
                ));
                value.deletion.push(f64::from(
                    u32::try_from(worker_index * firms + firm_index + 1).expect("deletion"),
                ));
                let sign = if (worker_index + firm_index) % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                value.outcome.push(
                    sign * f64::from(
                        u32::try_from(worker_index + firm_index + 1).expect("outcome"),
                    ),
                );
                value.frequency.push(f64::from(
                    u32::try_from(firm_index + 1).expect("frequency"),
                ));
                value.target_weight.push(f64::from(
                    u32::try_from((worker_index % 3) + firm_index + 1).expect("target"),
                ));
            }
        }
        value
    }

    fn descriptor(&self) -> VckssColumnsV1 {
        VckssColumnsV1 {
            struct_size: u32::try_from(size_of::<VckssColumnsV1>()).expect("structure size"),
            reserved: 0,
            rows: u64::try_from(self.worker.len()).expect("row count"),
            worker: self.worker.as_ptr(),
            firm: self.firm.as_ptr(),
            deletion: self.deletion.as_ptr(),
            outcome: self.outcome.as_ptr(),
            frequency: self.frequency.as_ptr(),
            target_weight: self.target_weight.as_ptr(),
        }
    }

    fn request(&self, cleanup_abandoned: u32) -> VckssPrepareRequestV1 {
        VckssPrepareRequestV1 {
            abi_version: ABI_VERSION,
            struct_size: u32::try_from(size_of::<VckssPrepareRequestV1>())
                .expect("structure size"),
            rows: u64::try_from(self.worker.len()).expect("row count"),
            cleanup_abandoned,
            reserved: 0,
        }
    }
}

fn reset() {
    assert_eq!(
        vckss_rust_session_clear_abandoned_v1(),
        ErrorCode::Ok as i32
    );
}

fn prepare(columns: &OwnedColumns, cleanup: u32) -> Result<u64, i32> {
    let request = columns.request(cleanup);
    let descriptor = columns.descriptor();
    let mut generation = 0_u64;
    let status = vckss_rust_session_prepare_v1(&request, &descriptor, &mut generation);
    if status == ErrorCode::Ok as i32 {
        Ok(generation)
    } else {
        Err(status)
    }
}

#[test]
fn c_abi_prepares_exports_and_releases_owned_state() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let generation = prepare(&columns, 0).expect("prepare");
    assert!(generation > 0);

    let mut receipt = VckssPreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_session_preparation_receipt_v1(generation, &mut receipt),
        ErrorCode::Ok as i32
    );
    assert_eq!(receipt.generation, generation);
    assert_eq!(receipt.input_rows, 48);
    assert_eq!(receipt.retained_rows, 48);
    assert_eq!(receipt.workers, 12);
    assert_eq!(receipt.firms, 4);
    assert_eq!(receipt.cells, 48);
    assert_eq!(receipt.deletion_units, 48);
    assert_eq!(receipt.target_strata, 48);

    let mut snapshot = VckssSessionSnapshotV1::default();
    assert_eq!(
        vckss_rust_session_snapshot_v1(&mut snapshot),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 1);
    assert_eq!(snapshot.generation, generation);

    assert_eq!(
        vckss_rust_session_release_v1(generation),
        ErrorCode::Ok as i32
    );
    assert_eq!(
        vckss_rust_session_release_v1(generation),
        ErrorCode::Ok as i32
    );
}

#[test]
fn fractional_identifiers_are_rejected_before_context_creation() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let mut columns = OwnedColumns::dense();
    columns.worker[7] = 1.5;
    assert_eq!(
        prepare(&columns, 0).expect_err("fractional identifier must fail"),
        ErrorCode::InvalidIdentifier as i32
    );
    // SAFETY: the pointer references a process-lifetime CString in the module.
    let message = unsafe { CStr::from_ptr(vckss_rust_session_last_error()) };
    assert!(
        message.to_string_lossy().contains("worker identifier"),
        "unexpected session error: {}",
        message.to_string_lossy()
    );
    let mut snapshot = VckssSessionSnapshotV1::default();
    assert_eq!(
        vckss_rust_session_snapshot_v1(&mut snapshot),
        ErrorCode::Ok as i32
    );
    assert_eq!(snapshot.state, 0);
}

#[test]
fn abi_mismatch_and_short_structures_are_typed_failures() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let descriptor = columns.descriptor();
    let mut request = columns.request(0);
    let mut generation = 0_u64;
    request.abi_version = ABI_VERSION + 1;
    assert_eq!(
        vckss_rust_session_prepare_v1(&request, &descriptor, &mut generation),
        ErrorCode::AbiMismatch as i32
    );
    request.abi_version = ABI_VERSION;
    request.struct_size = 1;
    assert_eq!(
        vckss_rust_session_prepare_v1(&request, &descriptor, &mut generation),
        ErrorCode::AbiMismatch as i32
    );
}

#[test]
fn abandoned_cleanup_is_explicit_and_generation_checked() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let first = prepare(&columns, 0).expect("first prepare");
    assert_eq!(
        prepare(&columns, 0).expect_err("second prepare must fail"),
        ErrorCode::ContextPoisoned as i32
    );
    let second = prepare(&columns, 1).expect("cleanup and reprepare");
    assert!(second > first);

    let mut receipt = VckssPreparationReceiptV1::default();
    assert_eq!(
        vckss_rust_session_preparation_receipt_v1(first, &mut receipt),
        ErrorCode::StaleContext as i32
    );
    assert_eq!(
        vckss_rust_session_release_v1(first),
        ErrorCode::StaleContext as i32
    );
    assert_eq!(
        vckss_rust_session_release_v1(second),
        ErrorCode::Ok as i32
    );
}

#[test]
fn null_input_and_output_pointers_never_dereference() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset();
    let columns = OwnedColumns::dense();
    let request = columns.request(0);
    let descriptor = columns.descriptor();
    let mut generation = 0_u64;
    assert_eq!(
        vckss_rust_session_prepare_v1(ptr::null(), &descriptor, &mut generation),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(
        vckss_rust_session_prepare_v1(&request, ptr::null(), &mut generation),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(
        vckss_rust_session_prepare_v1(&request, &descriptor, ptr::null_mut()),
        ErrorCode::InvalidInput as i32
    );
    let generation = prepare(&columns, 0).expect("valid prepare");
    assert_eq!(
        vckss_rust_session_preparation_receipt_v1(generation, ptr::null_mut()),
        ErrorCode::InvalidInput as i32
    );
    assert_eq!(
        vckss_rust_session_release_v1(generation),
        ErrorCode::Ok as i32
    );
}
