// SPDX-License-Identifier: GPL-3.0-only

//! Default-options end-to-end numerical session ABI.
//!
//! This V1 surface intentionally freezes only scientific outputs already
//! required by the public estimator: exact retained rows and plugin,
//! correction, and corrected variance-component vectors. Solver/probe tuning
//! is added through later request versions without changing this layout.

use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};

use vckss_core::engine::{run_jla_no_controls, JlaEngineOptions, JlaEngineResult};
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::jla::VarianceComponents;
use vckss_core::problem::CompressedProblem;
use vckss_core::types::{InputColumns, MAX_EXACT_BINARY64_INTEGER};
use vckss_core::ABI_VERSION;

use crate::context::{ContextHandle, ContextRegistry, ContextStateTag};
use crate::session::PreparationReceipt;
use crate::session_retained::PreparedProblemWithMask;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEnginePrepareRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub rows: u64,
    pub cleanup_abandoned: u32,
    pub reserved: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineColumnsV1 {
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

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VckssEngineSolveRequestV1 {
    pub abi_version: u32,
    pub struct_size: u32,
    pub reserved_0: u64,
    pub reserved_1: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEnginePreparationReceiptV1 {
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssComponentVectorV1 {
    pub worker: f64,
    pub firm: f64,
    pub covariance: f64,
    pub total: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct VckssEngineResultV1 {
    pub struct_size: u32,
    pub reserved: u32,
    pub generation: u64,
    pub plugin: VckssComponentVectorV1,
    pub correction: VckssComponentVectorV1,
    pub corrected: VckssComponentVectorV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct VckssEngineSnapshotV1 {
    pub struct_size: u32,
    pub state: u32,
    pub generation: u64,
    pub last_released_generation: u64,
}

#[derive(Debug)]
struct EnginePrepared {
    problem: CompressedProblem,
    receipt: PreparationReceipt,
}

#[derive(Debug)]
struct EngineSolved {
    result: JlaEngineResult,
}

#[derive(Debug)]
struct EngineState {
    registry: ContextRegistry<EnginePrepared, EngineSolved>,
    preparation: Option<(ContextHandle, PreparationReceipt)>,
    retained: Option<(ContextHandle, Vec<bool>)>,
}

impl EngineState {
    const fn new() -> Self {
        Self {
            registry: ContextRegistry::new(),
            preparation: None,
            retained: None,
        }
    }

    fn clear_abandoned(&mut self) -> Option<ContextHandle> {
        let cleared = self.registry.clear_abandoned();
        if cleared.is_some() {
            self.preparation = None;
            self.retained = None;
        }
        cleared
    }

    fn release(&mut self, generation: u64) -> Result<()> {
        let handle = ContextHandle::from_generation(generation)?;
        let released = self.registry.release(handle)?;
        if released
            || self
                .preparation
                .is_some_and(|(stored, _)| stored.generation() == generation)
            || self
                .retained
                .as_ref()
                .is_some_and(|(stored, _)| stored.generation() == generation)
        {
            self.preparation = None;
            self.retained = None;
        }
        Ok(())
    }
}

static ENGINE: OnceLock<Mutex<EngineState>> = OnceLock::new();
static ENGINE_LAST_ERROR: OnceLock<Mutex<CString>> = OnceLock::new();

fn engine() -> &'static Mutex<EngineState> {
    ENGINE.get_or_init(|| Mutex::new(EngineState::new()))
}

fn error_slot() -> &'static Mutex<CString> {
    ENGINE_LAST_ERROR.get_or_init(|| Mutex::new(cstring_without_nul("OK")))
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
                "engine_ffi",
                "Rust panic was contained at the numerical session ABI boundary",
            );
            store_error_text(&error.to_string());
            ErrorCode::Panic as i32
        }
    }
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_prepare_v1(
    request: *const VckssEnginePrepareRequestV1,
    columns: *const VckssEngineColumnsV1,
    output_handle: *mut u64,
) -> i32 {
    ffi_status(|| {
        let request = read_struct(request, "engine prepare request")?;
        let columns = read_struct(columns, "engine column descriptor")?;
        require_output_pointer(output_handle, "engine output handle")?;
        require_struct_size::<VckssEnginePrepareRequestV1>(
            request.struct_size,
            "engine prepare request",
        )?;
        require_struct_size::<VckssEngineColumnsV1>(
            columns.struct_size,
            "engine column descriptor",
        )?;
        if request.abi_version != ABI_VERSION {
            return Err(abi_mismatch(request.abi_version));
        }
        if request.rows == 0 || request.rows != columns.rows {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "engine_prepare",
                "request and column row counts must agree and be positive",
            ));
        }
        if request.cleanup_abandoned > 1 {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "engine_prepare",
                "cleanup_abandoned must be zero or one",
            ));
        }
        let rows = usize::try_from(request.rows).map_err(|_| resource_error(
            "engine_prepare",
            "row count is not representable on this platform",
        ))?;
        let columns = copy_columns(columns, rows)?;
        let prepared = PreparedProblemWithMask::from_columns(columns)?;
        let receipt = prepared.receipt;
        let retained = prepared.retained;
        let owned = EnginePrepared {
            problem: prepared.problem,
            receipt,
        };

        let mut state = lock_engine("engine_prepare")?;
        if request.cleanup_abandoned == 1 {
            state.clear_abandoned();
        }
        let handle = state.registry.prepare(owned)?;
        state.preparation = Some((handle, receipt));
        state.retained = Some((handle, retained));
        // SAFETY: non-null caller-owned output storage was checked above.
        unsafe { output_handle.write(handle.generation()) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_preparation_receipt_v1(
    generation: u64,
    output: *mut VckssEnginePreparationReceiptV1,
) -> i32 {
    ffi_status(|| {
        require_output_pointer(output, "engine preparation receipt")?;
        let state = lock_engine("engine_receipt")?;
        let (handle, receipt) = state.preparation.ok_or_else(|| {
            stale_context(
                "engine_receipt",
                generation,
                state.registry.snapshot().generation,
            )
        })?;
        require_generation(handle, generation, "engine_receipt")?;
        let value = VckssEnginePreparationReceiptV1 {
            struct_size: struct_size::<VckssEnginePreparationReceiptV1>()?,
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
        // SAFETY: non-null caller-owned output storage was checked above.
        unsafe { output.write(value) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_retained_mask_v1(
    generation: u64,
    output: *mut u8,
    output_length: u64,
) -> i32 {
    ffi_status(|| {
        if output.is_null() {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "engine_retained",
                "retained-mask output pointer is null",
            ));
        }
        let state = lock_engine("engine_retained")?;
        let (handle, retained) = state.retained.as_ref().ok_or_else(|| {
            stale_context(
                "engine_retained",
                generation,
                state.registry.snapshot().generation,
            )
        })?;
        require_generation(*handle, generation, "engine_retained")?;
        if output_length != u64::try_from(retained.len()).map_err(|_| {
            resource_error(
                "engine_retained",
                "retained-mask length is not representable as u64",
            )
        })? {
            return Err(BackendError::new(
                ErrorCode::InvalidInput,
                "engine_retained",
                "retained-mask output length does not equal the marked-row count",
            ));
        }
        for (index, &kept) in retained.iter().enumerate() {
            // SAFETY: the caller promises output_length writable bytes and the
            // exact length was checked before this bounded loop.
            unsafe { output.add(index).write(u8::from(kept)) };
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_solve_v1(
    generation: u64,
    request: *const VckssEngineSolveRequestV1,
) -> i32 {
    ffi_status(|| {
        let request = read_struct(request, "engine solve request")?;
        require_struct_size::<VckssEngineSolveRequestV1>(
            request.struct_size,
            "engine solve request",
        )?;
        if request.abi_version != ABI_VERSION {
            return Err(abi_mismatch(request.abi_version));
        }
        if request.reserved_0 != 0 || request.reserved_1 != 0 {
            return Err(BackendError::new(
                ErrorCode::AbiMismatch,
                "engine_solve",
                "reserved solve-request fields must be zero",
            ));
        }
        let handle = ContextHandle::from_generation(generation)?;
        let mut state = lock_engine("engine_solve")?;
        state.registry.solve(handle, |prepared| {
            let result = run_jla_no_controls(&prepared.problem, JlaEngineOptions::default())?;
            Ok(EngineSolved { result })
        })
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_result_v1(
    generation: u64,
    output: *mut VckssEngineResultV1,
) -> i32 {
    ffi_status(|| {
        require_output_pointer(output, "engine result")?;
        let handle = ContextHandle::from_generation(generation)?;
        let state = lock_engine("engine_result")?;
        let result = &state.registry.result(handle)?.result;
        result.plugin.verify_accounting(1.0e-10)?;
        result.corrected.verify_accounting(1.0e-10)?;
        let correction = subtract_components(result.plugin, result.corrected);
        correction.verify_accounting(1.0e-9)?;
        let value = VckssEngineResultV1 {
            struct_size: struct_size::<VckssEngineResultV1>()?,
            reserved: 0,
            generation,
            plugin: component_vector(result.plugin),
            correction: component_vector(correction),
            corrected: component_vector(result.corrected),
        };
        // SAFETY: non-null caller-owned output storage was checked above.
        unsafe { output.write(value) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_release_v1(generation: u64) -> i32 {
    ffi_status(|| lock_engine("engine_release")?.release(generation))
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_clear_abandoned_v1() -> i32 {
    ffi_status(|| {
        lock_engine("engine_clear")?.clear_abandoned();
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_snapshot_v1(
    output: *mut VckssEngineSnapshotV1,
) -> i32 {
    ffi_status(|| {
        require_output_pointer(output, "engine snapshot")?;
        let state = lock_engine("engine_snapshot")?;
        let snapshot = state.registry.snapshot();
        let value = VckssEngineSnapshotV1 {
            struct_size: struct_size::<VckssEngineSnapshotV1>()?,
            state: state_code(snapshot.state),
            generation: snapshot.generation.unwrap_or(0),
            last_released_generation: snapshot.last_released_generation,
        };
        // SAFETY: non-null caller-owned output storage was checked above.
        unsafe { output.write(value) };
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn vckss_rust_engine_last_error() -> *const c_char {
    match error_slot().lock() {
        Ok(slot) => slot.as_ptr(),
        Err(_) => c"CONTEXT_POISONED [engine_ffi]: error lock poisoned".as_ptr(),
    }
}

fn subtract_components(
    plugin: VarianceComponents,
    corrected: VarianceComponents,
) -> VarianceComponents {
    VarianceComponents {
        worker: plugin.worker - corrected.worker,
        firm: plugin.firm - corrected.firm,
        covariance: plugin.covariance - corrected.covariance,
        total: plugin.total - corrected.total,
    }
}

fn component_vector(value: VarianceComponents) -> VckssComponentVectorV1 {
    VckssComponentVectorV1 {
        worker: value.worker,
        firm: value.firm,
        covariance: value.covariance,
        total: value.total,
    }
}

fn copy_columns(columns: &VckssEngineColumnsV1, rows: usize) -> Result<InputColumns> {
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
                    ErrorCode::InvalidIdentifier,
                    "engine_ingest",
                    format!(
                        "{label} must be a positive exact binary64 integer at zero-based row {row}"
                    ),
                ));
            }
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
            "engine_ingest",
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
            "engine_ingest",
            format!("{label} is negative or nonfinite at zero-based row {row}"),
        ));
    }
    Ok(source.to_vec())
}

fn read_slice<'a>(pointer: *const f64, rows: usize, label: &str) -> Result<&'a [f64]> {
    if pointer.is_null() {
        return Err(BackendError::new(
            ErrorCode::InvalidInput,
            "engine_ingest",
            format!("{label} pointer is null"),
        ));
    }
    // SAFETY: the caller guarantees `rows` initialized values for this
    // synchronous call; Rust copies the complete slice before returning.
    Ok(unsafe { std::slice::from_raw_parts(pointer, rows) })
}

fn read_struct<'a, T>(pointer: *const T, label: &str) -> Result<&'a T> {
    if pointer.is_null() {
        return Err(BackendError::new(
            ErrorCode::InvalidInput,
            "engine_ffi",
            format!("{label} pointer is null"),
        ));
    }
    // SAFETY: the caller guarantees one initialized T for this synchronous
    // call. The reported structure size is checked before optional fields.
    Ok(unsafe { &*pointer })
}

fn require_output_pointer<T>(pointer: *mut T, label: &str) -> Result<()> {
    if pointer.is_null() {
        Err(BackendError::new(
            ErrorCode::InvalidInput,
            "engine_ffi",
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
            "engine_ffi",
            format!("{label} size {reported} is below required size {required}"),
        ))
    } else {
        Ok(())
    }
}

fn struct_size<T>() -> Result<u32> {
    u32::try_from(std::mem::size_of::<T>()).map_err(|_| {
        resource_error(
            "engine_ffi",
            "ABI structure size is not representable as u32",
        )
    })
}

fn require_generation(
    handle: ContextHandle,
    generation: u64,
    phase: &'static str,
) -> Result<()> {
    if handle.generation() == generation {
        Ok(())
    } else {
        Err(stale_context(
            phase,
            generation,
            Some(handle.generation()),
        ))
    }
}

fn lock_engine(phase: &'static str) -> Result<std::sync::MutexGuard<'static, EngineState>> {
    engine().lock().map_err(|_| {
        BackendError::new(
            ErrorCode::ContextPoisoned,
            phase,
            "native numerical session registry lock is poisoned",
        )
    })
}

fn abi_mismatch(requested: u32) -> BackendError {
    BackendError::new(
        ErrorCode::AbiMismatch,
        "engine_ffi",
        format!(
            "requested ABI {requested} does not match plugin ABI {ABI_VERSION}"
        ),
    )
}

fn resource_error(phase: &'static str, message: &str) -> BackendError {
    BackendError::new(ErrorCode::ResourceLimit, phase, message)
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
