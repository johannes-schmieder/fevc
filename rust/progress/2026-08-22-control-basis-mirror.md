# Rust CI mirror checkpoint: control basis

Date: 2026-08-22

Private source checkpoint under qualification:

- repository: `johannes-schmieder/varcomp_kss`
- branch: `codex/rust-backend-completion`
- source commit: `ce76810348b96d377c1b52907a2fd9fdb76f4909`
- source Rust tree: `8e939cb3fb1b28f501668459e839fd5a427bf2c2`

Public GitHub Actions controller:

- repository: `johannes-schmieder/playground`
- restored path: `crates/vckss-core/src/control_basis.rs`
- expected and verified Git blob: `ea68dd44a872966594302a3934ecf4830c5e96b2`
- expected and verified byte count: `52600`

The file was reconstructed from committed source-bound ranges by a fail-closed GitHub Actions workflow. A separate verification workflow created its success marker only after checking both `git hash-object` and `wc -c`; that success marker was then consumed through the GitHub connector using the known empty-blob SHA. This verifies byte identity of the public test copy.

No Rust code was compiled locally. Stata was not executed. The public mirror is not yet complete, so no full-workspace green claim is made.

Next resume point:

1. restore `model_solver.rs` exactly;
2. restore `cmg/hierarchy.rs`, `generic_jla.rs`, and plugin `ffi_engine.rs`;
3. restore remaining plugin modules and integration tests;
4. run the complete Ubuntu/macOS/Windows by Rust 1.81/stable matrix;
5. fix source-bound failures on `codex/rust-backend-completion`, resync, and repeat until green.
