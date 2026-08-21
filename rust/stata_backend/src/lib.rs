// SPDX-License-Identifier: GPL-3.0-only

//! Standalone Rust side of the `varcomp_kss` Stata plugin.
//!
//! The platform C shim owns the official Stata SPI entry point. This crate
//! exports capability/self-test functions and the staged preparation ABI while
//! keeping every panic and owned native context behind a C-compatible boundary.

#[path = "../../crates/vckss-plugin/src/context.rs"]
pub mod context;
#[path = "../../crates/vckss-plugin/src/context_ffi.rs"]
mod context_ffi;
#[path = "../../crates/vckss-plugin/src/session.rs"]
pub mod session;
#[path = "../../crates/vckss-plugin/src/ffi_session.rs"]
pub mod ffi_session;

use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, OnceLock};

use vckss_core::error::{BackendError, ErrorCode};
use vckss_core::{selftest, Capabilities, ABI_VERSION, BACKEND_VERSION};

static LAST_ERROR: OnceLock<Mutex<CString>> = OnceLock::new();
static VERSION: OnceLock<CString> = OnceLock::new();
static CAPABILITIES: OnceLock<CString> = OnceLock::new();

fn cstring_without_nul(value: &str) -> CString {
    let bytes = value
        .as_bytes()
        .iter()
        .copied()
        .map(|byte| if byte == 0 { b'?' } else { byte })
        .collect::<Vec<_>>();
    CString::new(bytes).unwrap_or_else(|_| CString::new("invalid string").expect("literal CString"))
}

fn error_slot() -> &'static Mutex<CString> {
    LAST_ERROR.get_or_init(|| Mutex::new(cstring_without_nul("OK")))
}

fn store_error(error: &BackendError) {
    if let Ok(mut slot) = error_slot().lock() {
        *slot = cstring_without_nul(&error.to_string());
    }
}

fn clear_error() {
    if let Ok(mut slot) = error_slot().lock() {
        *slot = cstring_without_nul("OK");
    }
}

fn ffi_status(function: impl FnOnce() -> Result<(), BackendError>) -> i32 {
    match catch_unwind(AssertUnwindSafe(function)) {
        Ok(Ok(())) => {
            clear_error();
            ErrorCode::Ok as i32
        }
        Ok(Err(error)) => {
            let code = error.code as i32;
            store_error(&error);
            code
        }
        Err(_) => {
            let error = BackendError::new(
                ErrorCode::Panic,
                "ffi",
                "Rust panic was contained at the standalone C ABI boundary",
            );
            store_error(&error);
            ErrorCode::Panic as i32
        }
    }
}

#[no_mangle]
pub extern "C" fn vckss_rust_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn vckss_rust_backend_version() -> *const c_char {
    VERSION
        .get_or_init(|| cstring_without_nul(BACKEND_VERSION))
        .as_ptr()
}

#[no_mangle]
pub extern "C" fn vckss_rust_capabilities_json() -> *const c_char {
    CAPABILITIES
        .get_or_init(|| cstring_without_nul(&Capabilities::current().to_json()))
        .as_ptr()
}

#[no_mangle]
pub extern "C" fn vckss_rust_last_error() -> *const c_char {
    match error_slot().lock() {
        Ok(slot) => slot.as_ptr(),
        Err(_) => c"CONTEXT_POISONED [ffi]: error lock poisoned".as_ptr(),
    }
}

#[no_mangle]
pub extern "C" fn vckss_rust_selftest() -> i32 {
    ffi_status(selftest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn standalone_selftest_passes() {
        assert_eq!(vckss_rust_selftest(), ErrorCode::Ok as i32);
    }

    #[test]
    fn capability_and_version_pointers_are_process_stable() {
        assert_eq!(
            vckss_rust_capabilities_json(),
            vckss_rust_capabilities_json()
        );
        assert_eq!(vckss_rust_backend_version(), vckss_rust_backend_version());
        // SAFETY: both pointers reference process-lifetime OnceLock CStrings.
        let capabilities = unsafe { CStr::from_ptr(vckss_rust_capabilities_json()) };
        // SAFETY: both pointers reference process-lifetime OnceLock CStrings.
        let version = unsafe { CStr::from_ptr(vckss_rust_backend_version()) };
        assert!(capabilities.to_bytes().starts_with(b"{"));
        assert_eq!(version.to_string_lossy(), BACKEND_VERSION);
    }
}
