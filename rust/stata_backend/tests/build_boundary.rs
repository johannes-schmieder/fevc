// SPDX-License-Identifier: GPL-3.0-only

// Compile the build helper as an ordinary test module so its hash, input, and
// platform-policy tests run under Cargo with resolved build dependencies.
#[allow(dead_code)]
#[path = "../build.rs"]
mod build_script;
