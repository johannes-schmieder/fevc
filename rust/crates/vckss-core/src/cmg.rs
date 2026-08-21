// SPDX-License-Identifier: GPL-3.0-only

//! Rust CMG public module.
//!
//! The source-informed hybrid graph is kept in its own implementation unit so
//! hierarchy, V-cycle, routing, and workspace code can evolve independently.

mod implementation {
    include!("cmg_impl.rs");

    // The implementation deliberately leaves one CSR pointer vector's numeric
    // type to its first checked arithmetic operation. Defaulting an otherwise
    // unconstrained `Vec` to `u64` makes that storage contract explicit while
    // preserving ordinary inference for every other `Vec<T>` in the module.
    type Vec<T = u64> = std::vec::Vec<T>;
}

pub use implementation::*;
