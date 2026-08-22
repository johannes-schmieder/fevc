# Rust backend checkpoint: public matrix green

Date: 2026-08-22

Source under test:

- repository: `johannes-schmieder/varcomp_kss`
- branch: `codex/rust-backend-completion`
- source commit: `ce76810348b96d377c1b52907a2fd9fdb76f4909`
- Rust tree: `8e939cb3fb1b28f501668459e839fd5a427bf2c2`

The byte-exact public mirror in `johannes-schmieder/playground` was repaired and independently checked against private Git blob IDs. In particular, `control_basis.rs` was restored as blob `ea68dd44a872966594302a3934ecf4830c5e96b2` with 52,600 bytes. Independent markers also confirmed the expected private blobs for the major core and plugin modules, including `model_solver.rs`, `cmg/hierarchy.rs`, `generic_jla.rs`, `ffi_engine.rs`, and the plugin session modules.

A separate fail-closed receipt verifier then accepted the committed `.ci/latest.json` only after checking:

- `overall == success` and `matrix_result == success`;
- six expected and six observed jobs;
- Ubuntu, macOS, and Windows under Rust 1.81.0 and stable;
- every recorded command result was `success`;
- the source commit and Rust-tree bindings above; and
- the tested public SHA was an ancestor of current `playground/main`.

The matrix commands are:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --locked -- -D warnings`;
- `cargo test --workspace --all-targets --locked`;
- `cargo test --workspace --all-targets --locked --release`;
- `cargo build --workspace --all-targets --locked --release`; and
- `cargo build --release --locked`.

This is GitHub Actions compilation/test evidence only. Rust was not compiled locally. Stata was not executed.

Next resume point:

1. finish solve V4 dispatch in `varcomp_kss/varcomp_kss_rust.ado`;
2. reconcile and export the complete V7 execution-plan receipt;
3. add source-static and Rust/C corruption/lifecycle tests that can run without Stata;
4. push each checkpoint to this private branch, sync the exact Rust tree to `playground`, and rerun the public matrix;
5. expose the planned automatic routing only after the private boundary is complete.
