// SPDX-License-Identifier: GPL-3.0-only

//! Rust CMG public module.
//!
//! The source-informed hybrid graph is generated from a checked `.rs.in`
//! template. Hierarchy, V-cycle, routing, and workspace code live in separate
//! ordinary Rust modules.

mod implementation {
    include!(concat!(env!("OUT_DIR"), "/cmg_impl_generated.rs"));
}

pub use implementation::*;
