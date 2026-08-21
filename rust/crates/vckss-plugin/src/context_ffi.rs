// SPDX-License-Identifier: GPL-3.0-only

//! Checked conversion from a C ABI generation value to the opaque Rust handle.

use vckss_core::error::{BackendError, ErrorCode, Result};

use crate::context::ContextHandle;

impl ContextHandle {
    pub fn from_generation(generation: u64) -> Result<Self> {
        if generation == 0 {
            return Err(BackendError::new(
                ErrorCode::StaleContext,
                "context_ffi",
                "native context generation zero is invalid",
            ));
        }
        // SAFETY: `ContextHandle` is `repr(transparent)` over one `u64`, so
        // every nonzero u64 has exactly the same representation. The semantic
        // generation check remains the registry's responsibility.
        Ok(unsafe { std::mem::transmute::<u64, Self>(generation) })
    }
}
