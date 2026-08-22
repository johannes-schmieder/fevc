# Remote Rust CI status

Last updated: 2026-08-22

Development branch: `codex/rust-backend-completion`
Baseline source commit: `ce76810348b96d377c1b52907a2fd9fdb76f4909`
Public compiler/test controller: `johannes-schmieder/playground`

## Current checkpoint

The first public six-cell matrix run (`32586393479`) did not test a complete
copy of the baseline Rust tree. The cargo commands ran, but the public snapshot
was missing several large modules, including `engine.rs`, `exact_estimator.rs`,
`generic_jla.rs`, `model_solver.rs`, and the plugin `ffi_engine.rs`; some other
large files were older blobs. Therefore the red result is a snapshot-controller
failure and is not qualification evidence for or against the private source.

The public mirror now has complete staged chunks for the baseline versions of:

- `crates/vckss-core/src/engine.rs` (expected Git blob
  `63e4db1028b462096f9be9fa0de6ec26ca5af206`, 132633 bytes); and
- `crates/vckss-core/src/exact_estimator.rs` (expected Git blob
  `e101b16b9b6d10f2b6e80a73da7d4756df07007f`, 99101 bytes).

A two-file hash-fenced assembly was triggered from public commit
`505924e1ec02f0e141d5ac1e160e09053fc3d3f5`. The assembler independently checks
both byte length and the Git blob SHA before writing either target. This status
file intentionally does not call those files verified until the assembly
workflow result and resulting public blobs have been read back.

The remaining known mirror mismatches are `generic_jla.rs`, `model_solver.rs`,
`control_basis.rs`, `cmg/hierarchy.rs`, `solver.rs`, the new core integration
tests, the plugin sources (especially `ffi_engine.rs`), and plugin integration
tests. These must all be exact before a Rust matrix result is scientifically
usable.

## Evidence policy

- Rust has not been compiled locally in the ChatGPT execution environment.
- GitHub Actions is the only Rust compiler and test runner used here.
- Stata has not been executed in this environment.
- No Rust checkpoint is called green until `.ci/latest.json` is read back from
  `playground`, names the exact private source SHA, and reports every required
  matrix command successful.
- A red run against an incomplete or mixed source snapshot is classified as a
  controller failure, not as a source failure.

## Exact resume point

1. Read back the two-file assembly result and verify the resulting public blob
   SHAs for `engine.rs` and `exact_estimator.rs`.
2. Stage and hash-assemble `model_solver.rs`, then `generic_jla.rs`.
3. Replace the stale medium core modules and add all missing core tests.
4. Replace the plugin source and integration tests, including the large
   `ffi_engine.rs` and `engine_ffi.rs` blobs.
5. Verify the complete public Rust tree against the private baseline tree.
6. Run and inspect the Ubuntu/macOS/Windows by Rust-1.81/stable matrix; fix
   actual source failures in small private commits and mirror each tested SHA.
7. Only after a green exact-source baseline, finish private Ado solve-V4
   dispatch and detailed V7 reconciliation.
8. Add static/C/Rust boundary tests for the V4/V7 lifecycle; keep licensed
   Stata execution explicitly pending.
9. Expose the planned routing and receipt surface, implement Rust exact stayer
   parity, then profile and optimize large-N memory traffic, batched PCG, CMG
   application, and deterministic parallel kernels.

Update this file at each diagnostically useful or green remote checkpoint so a
future thread can resume without relying on chat history.
