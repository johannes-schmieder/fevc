// SPDX-License-Identifier: GPL-3.0-only

//! Standalone Rust side of the `vckss` Stata plugin.
//!
//! The platform C shim owns the official Stata SPI entry point. This crate
//! exports capability/self-test functions and the complete numerical engine ABI while
//! keeping every panic and owned native context behind a C-compatible boundary.

#[path = "../../crates/vckss-plugin/src/allocation_meter.rs"]
mod allocation_meter;
#[path = "../../crates/vckss-plugin/src/context.rs"]
pub mod context;
#[path = "../../crates/vckss-plugin/src/ffi_engine.rs"]
pub mod ffi_engine;
#[path = "../../crates/vckss-plugin/src/session.rs"]
pub mod session;
#[path = "../../crates/vckss-plugin/src/session_retained.rs"]
pub mod session_retained;

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

use vckss_core::error::{BackendError, ErrorCode};
use vckss_core::{selftest, Capabilities, ABI_VERSION, BACKEND_VERSION};

extern "C" {
    fn vckss_spi_pginit(plugin: *mut c_void) -> i32;
    fn vckss_stata_call_impl(argc: i32, argv: *mut *mut c_char) -> i32;
}

static VERSION: OnceLock<CString> = OnceLock::new();
static CAPABILITIES: OnceLock<CString> = OnceLock::new();

thread_local! {
    static LAST_ERROR: RefCell<CString> =
        RefCell::new(CString::new("OK").expect("literal CString"));
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
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn vckss_rust_selftest() -> i32 {
    ffi_status(selftest)
}

/// Stata SPI loader entry point. The C implementation records Stata's
/// function table; the Rust wrapper keeps the symbol on Cargo's export list.
///
/// # Safety
///
/// `plugin` must be the valid SPI function-table pointer supplied by Stata
/// during plugin loading and must remain valid for the process lifetime.
#[no_mangle]
pub unsafe extern "C" fn pginit(plugin: *mut c_void) -> i32 {
    // SAFETY: Stata supplies the SPI table pointer during plugin loading.
    unsafe { vckss_spi_pginit(plugin) }
}

/// Stata plugin dispatcher entry point retained through Cargo's export list.
///
/// # Safety
///
/// `argv` must reference `argc` valid, NUL-terminated strings owned by Stata
/// for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn stata_call(argc: i32, argv: *mut *mut c_char) -> i32 {
    // SAFETY: Stata owns the argument vector for the duration of this call.
    unsafe { vckss_stata_call_impl(argc, argv) }
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
