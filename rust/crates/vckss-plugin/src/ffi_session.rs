// SPDX-License-Identifier: GPL-3.0-only

//! Versioned C ABI for staged preparation from caller-owned numeric columns.
//!
//! The Stata C shim remains the only code that calls `SF_*`. It copies each
//! marked numeric column into ordinary contiguous buffers and passes read-only
//! pointers here. Rust copies and validates those buffers before returning, so
//! no pointer into Stata-managed memory survives the call.

use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};

use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::types::{InputColumns, MAX_EXACT_BINARY64_INTEGER};
use vckss_core::ABI_VERSION;

use crate::context::{ContextHandle, ContextRegistry, ContextStateTag};
use crate::session::{PreparationReceipt, PreparedProblem};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssPrepareRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub rows: u64,
    pub cleanup_abandoned: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssColumnsV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub rows: u64,
    pub worker: *const f64,
    pub firm: *const f64,
    pub deletion: *const f64,
    pub outcome: *const f64,
    pub frequency: *const f64,
    pub target_weight: *const f64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssPreparationReceiptV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub input_rows: u64,
    pub retained_rows: u64,
    pub workers: u64,
    pub firms: u64,
    pub cells: u64,
    pub deletion_units: u64,
    pub target_strata: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssSessionSnapshotV1 {
    pub struct_size: u32,
    pub state: u32,
    pub generation: u64,
    pub last_released_generation: u64,
}

#[derive(Debug)]
struct SessionState {
    registry: ContextRegistry<PreparedProblem, PreparationReceipt>,
    preparation: Option<(ContextHandle, PreparationReceipt)>,
}

impl SessionState {
    const fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
            preparation: None,
        }
    }

    fn clear_abandoned(&mut self) -> Option<ContextHandle> {
        let cleared = self.registry.clear_abandoned();
        if cleared.is_some() {
            self.preparation = None;
        }
        cleared
    }
}

// Raw input pointers are consumed synchronously under the caller thread and are
// never stored in `SessionState`. The owned state itself contains no raw
// pointers, so the mutex-protected registry is safe to share between calls.
static SESSION: OnceLock<Mutex<SessionState>> = OnceLock::new();
static SESSION_LAST_ERROR: OnceLock<Mutex<CString>> = OnceLock::new();

fn session() -> &'static Mutex<SessionState> {
    SESSION.get_or_init(|| Mutex::new(SessionState::new()))
}

fn error_slot() -> &'static Mutex<CString> {
    SESSION_LAST_ERROR.get_or_init(|| Mutex::new(cstring_without_nul("OK")))
}

fn ffi_status(function: impl FnOnce() -> Result<()>) -> i32 {
    match catch_unwind(AssertUnwindSafe(function)) {
        Ok(Ok(())) => {
            store_error_text("OK");
            ErrorCode::Ok as i32
        }
        Ok(Err(error)) => {
            let code = error.code as i32;
            store_error_text(&error.to_string());
            code
        }
        Err(_) => {
            let error = BackendError::new(
                ErrorCode::Panic,
                "session_ffi",
                "Rust panic was contained at the staged session ABI boundary",
            );
            store_error_text(&error.to_string());
            ErrorCode::Panic as i32
        }
    }
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_prepare_v1(
    request: *const VckssPrepareRequestV1,
    columns: *const VckssColumnsV1,
    output_handle: *mut u64,
) -> i32 {
    ffi_status(|| {
        let request = read_struct(request, "prepare request")?;
        let columns = read_struct(columns, "column descriptor")?;
        require_output_pointer(output_handle, "output handle")?;
        require_struct_size::<VckssPrepareRequestV1>(request.struct_size, "prepare request")?;
        require_struct_size::<VckssColumnsV1>(columns.struct_size, "column descriptor")?;
        if request.abi_version != ABI_VERSION {
            return Err(BackendError::new(
                ErrorCode::AbiMismatch,
                "session_prepare",
                format!(
                    "requested ABI {} does not match plugin ABI {}",
                    request.abi_version, ABI_VERSION
                ),
            ));
        }
        if request.rows == 0 || request.rows != columns.rows {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "session_prepare",
                "request and column row counts must agree and be positive",
            ));
        }
        if request.cleanup_abandoned > 1 {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "session_prepare",
                "cleanup_abandoned must be zero or one",
            ));
        }
        let rows = usize::try_from(request.rows).map_err(|_| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "session_prepare",
                "row count is not representable on this platform",
            )
        })?;
        let input = copy_columns(columns, rows)?;
        let prepared = PreparedProblem::from_columns(input)?;
        let receipt = prepared.receipt;

        let mut state = session().lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_prepare",
                "native session registry lock is poisoned",
            )
        })?;
        if request.cleanup_abandoned == 1 {
            state.clear_abandoned();
        }
        let handle = state.registry.prepare(prepared)?;
        state.preparation = Some((handle, receipt));
        // SAFETY: the pointer was checked non-null and points to caller-owned
        // writable storage for one u64 under the C ABI contract.
        unsafe { output_handle.write(handle.generation()) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_preparation_receipt_v1(
    generation: u64,
    output: *mut VckssPreparationReceiptV1,
) -> i32 {
    ffi_status(|| {
        require_output_pointer(output, "preparation receipt")?;
        let state = session().lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_receipt",
                "native session registry lock is poisoned",
            )
        })?;
        let (handle, receipt) = state.preparation.ok_or_else(|| {
            stale_context("session_receipt", generation, state.registry.snapshot().generation)
        })?;
        if handle.generation() != generation {
            return Err(stale_context(
                "session_receipt",
                generation,
                Some(handle.generation()),
            ));
        }
        let value = VckssPreparationReceiptV1 {
            struct_size: struct_size::<VckssPreparationReceiptV1>()?,
            reserved: 0,
            generation,
            input_rows: receipt.input_rows,
            retained_rows: receipt.retained_rows,
            workers: receipt.workers,
            firms: receipt.firms,
            cells: receipt.cells,
            deletion_units: receipt.deletion_units,
            target_strata: receipt.target_strata,
        };
        // SAFETY: the pointer was checked non-null and the caller provides
        // writable storage for one complete receipt structure.
        unsafe { output.write(value) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_release_v1(generation: u64) -> i32 {
    ffi_status(|| {
        let mut state = session().lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_release",
                "native session registry lock is poisoned",
            )
        })?;
        let handle = ContextHandle::from_generation(generation)?;
        let released = state.registry.release(handle)?;
        if released
            || state
                .preparation
                .is_some_and(|(stored, _)| stored.generation() == generation)
        {
            state.preparation = None;
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_clear_abandoned_v1() -> i32 {
    ffi_status(|| {
        let mut state = session().lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_clear",
                "native session registry lock is poisoned",
            )
        })?;
        state.clear_abandoned();
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_snapshot_v1(
    output: *mut VckssSessionSnapshotV1,
) -> i32 {
    ffi_status(|| {
        require_output_pointer(output, "session snapshot")?;
        let state = session().lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "session_snapshot",
                "native session registry lock is poisoned",
            )
        })?;
        let snapshot = state.registry.snapshot();
        let value = VckssSessionSnapshotV1 {
            struct_size: struct_size::<VckssSessionSnapshotV1>()?,
            state: state_code(snapshot.state),
            generation: snapshot.generation.unwrap_or(0),
            last_released_generation: snapshot.last_released_generation,
        };
        // SAFETY: the pointer was checked non-null and the caller provides
        // writable storage for one complete snapshot structure.
        unsafe { output.write(value) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_session_last_error() -> *const c_char {
    match error_slot().lock() {
        Ok(slot) => slot.as_ptr(),
        Err(_) => c"CONTEXT_POISONED [session_ffi]: error lock poisoned".as_ptr(),
    }
}

fn copy_columns(columns: &VckssColumnsV1, rows: usize) -> Result<InputColumns> {
    Ok(InputColumns {
        worker: copy_positive_integer_column(columns.worker, rows, "worker identifier")?,
        firm: copy_positive_integer_column(columns.firm, rows, "firm identifier")?,
        deletion: copy_positive_integer_column(
            columns.deletion,
            rows,
            "deletion identifier",
        )?,
        outcome: copy_finite_column(columns.outcome, rows, "outcome")?,
        frequency: copy_positive_integer_column(columns.frequency, rows, "frequency weight")?,
        target_weight: copy_nonnegative_column(columns.target_weight, rows, "target weight")?,
        controls: Vec::new(),
    })
}

fn copy_positive_integer_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
) -> Result<Vec<u64>> {
    let source = read_slice(pointer, rows, label)?;
    source
        .iter()
        .enumerate()
        .map(|(row, &value)| {
            if !value.is_finite()
                || value <= 0.0
                || value.fract() != 0.0
                || value > MAX_EXACT_BINARY64_INTEGER as f64
            {
                return Err(BackendError::new(
                    ErrorCode::InvalidId,
                    "session_ingest",
                    format!(
                        "{label} must be a positive exact binary64 integer at zero-based row {row}"
                    ),
                ));
            }
            // The range and exact-integrality checks above make this cast exact.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(value as u64)
        })
        .collect()
}

fn copy_finite_column(pointer: *const f64, rows: usize, label: &str) -> Result<Vec<f64>> {
    let source = read_slice(pointer, rows, label)?;
    if let Some((row, _)) = source.iter().enumerate().find(|(_, value)| !value.is_finite()) {
        return Err(BackendError::new(
            ErrorCode::InvalidInput,
            "session_ingest",
            format!("{label} is nonfinite at zero-based row {row}"),
        ));
    }
    Ok(source.to_vec())
}

fn copy_nonnegative_column(
    pointer: *const f64,
    rows: usize,
    label: &str,
) -> Result<Vec<f64>> {
    let source = read_slice(pointer, rows, label)?;
    if let Some((row, _)) = source
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite() || **value < 0.0)
    {
        return Err(BackendError::new(
            ErrorCode::InvalidTargetWeight,
            "session_ingest",
            format!("{label} is negative or nonfinite at zero-based row {row}"),
        ));
    }
    Ok(source.to_vec())
}

fn read_slice<'a>(pointer: *const f64, rows: usize, label: &str) -> Result<&'a [f64]> {
    if pointer.is_null() {
        return Err(BackendError::new(
            ErrorCode::InvalidInput,
            "session_ingest",
            format!("{label} pointer is null"),
        ));
    }
    // SAFETY: the C caller guarantees that `pointer` addresses at least `rows`
    // initialized f64 values for the duration of this synchronous call. The
    // resulting slice is copied before the function returns and is never stored.
    Ok(unsafe { std::slice::from_raw_parts(pointer, rows) })
}

fn read_struct<'a, T>(pointer: *const T, label: &str) -> Result<&'a T> {
    if pointer.is_null() {
        return Err(BackendError::new(
            ErrorCode::InvalidInput,
            "session_ffi",
            format!("{label} pointer is null"),
        ));
    }
    // SAFETY: the C caller guarantees one initialized structure of type T for
    // the duration of this synchronous call. Structure-size checks follow.
    Ok(unsafe { &*pointer })
}

fn require_output_pointer<T>(pointer: *mut T, label: &str) -> Result<()> {
    if pointer.is_null() {
        Err(BackendError::new(
            ErrorCode::InvalidInput,
            "session_ffi",
            format!("{label} pointer is null"),
        ))
    } else {
        Ok(())
    }
}

fn require_struct_size<T>(reported: u32, label: &str) -> Result<()> {
    let required = struct_size::<T>()?;
    if reported < required {
        Err(BackendError::new(
            ErrorCode::AbiMismatch,
            "session_ffi",
            format!("{label} size {reported} is below required size {required}"),
        ))
    } else {
        Ok(())
    }
}

fn struct_size<T>() -> Result<u32> {
    u32::try_from(std::mem::size_of::<T>()).map_err(|_| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "session_ffi",
            "ABI structure size is not representable as u32",
        )
    })
}

fn state_code(state: ContextStateTag) -> u32 {
    match state {
        ContextStateTag::Empty => 0,
        ContextStateTag::Prepared => 1,
        ContextStateTag::Solving => 2,
        ContextStateTag::Solved => 3,
        ContextStateTag::Failed => 4,
        ContextStateTag::Poisoned => 5,
    }
}

fn stale_context(phase: &'static str, requested: u64, active: Option<u64>) -> BackendError {
    BackendError::new(
        ErrorCode::StaleContext,
        phase,
        active.map_or_else(
            || format!("stale generation {requested}; no native context is active"),
            |generation| {
                format!("stale generation {requested}; active generation is {generation}")
            },
        ),
    )
}

fn store_error_text(value: &str) {
    if let Ok(mut slot) = error_slot().lock() {
        *slot = cstring_without_nul(value);
    }
}

fn cstring_without_nul(value: &str) -> CString {
    let bytes = value
        .as_bytes()
        .iter()
        .copied()
        .map(|byte| if byte == 0 { b'?' } else { byte })
        .collect::<Vec<_>>();
    CString::new(bytes).unwrap_or_else(|_| CString::new("invalid string").expect("literal CString"))
}
