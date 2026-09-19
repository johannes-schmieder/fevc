// SPDX-License-Identifier: GPL-3.0-only

//! Panic-contained C ABI for the `vckss` Rust backend.
//!
//! The Stata-facing `stata_call()` shim is added as a separately compiled C
//! translation unit. These exports make capability negotiation and isolated
//! self-tests available before any dataset is read.

mod allocation_meter;
pub mod context;
pub mod ffi_engine;
pub mod progress_api;
pub mod session;
pub mod session_retained;

use std::cell::RefCell;
use std::ffi::{c_char, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

use vckss_core::error::{BackendError, ErrorCode};
use vckss_core::{selftest, Capabilities, ABI_VERSION, BACKEND_VERSION};

static VERSION: OnceLock<CString> = OnceLock::new();
static CAPABILITIES: OnceLock<CString> = OnceLock::new();

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("OK").expect("literal CString"));
}

fn cstring_without_nul(value: &str) -> CString {
    let bytes: Vec<u8> = value
        .as_bytes()
        .iter()
        .copied()
        .map(|byte| if byte == 0 { b'?' } else { byte })
        .collect();
    CString::new(bytes).unwrap_or_else(|_| CString::new("invalid string").expect("literal CString"))
}

fn store_error(error: &BackendError) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = cstring_without_nul(&error.to_string()));
}

fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = cstring_without_nul("OK"));
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
                "Rust panic was contained at the C ABI boundary",
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
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
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
    fn ffi_selftest_passes() {
        assert_eq!(vckss_rust_selftest(), ErrorCode::Ok as i32);
    }

    #[test]
    fn capability_pointer_is_stable() {
        let first = vckss_rust_capabilities_json();
        let second = vckss_rust_capabilities_json();
        assert_eq!(first, second);
        // SAFETY: the pointer references a process-lifetime OnceLock CString.
        let text = unsafe { CStr::from_ptr(first) };
        assert!(text.to_bytes().starts_with(b"{"));
    }
}
