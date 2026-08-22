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

The first two-file assembly run (`32591178869`) correctly failed its Git-blob
fence. It expected baseline `engine.rs` blob
`63e4db1028b462096f9be9fa0de6ec26ca5af206` at 132633 bytes, but the staged
parts produced blob `12a444458dd017e85b61c07eb312bd13dc8515bd` at 136981 bytes.

An initial hypothesis assigned the 4348-byte overrun to old chunk 8. Replacing
that chunk with baseline lines 2801--3200 produced the second fenced failure
(`32591909432`): blob `0f210ccb7d98c40439bb4f9eb4de923a3135a4c6` at
141122 bytes. This proved the earlier chunks were mixed-version as well. Exact
accounting showed staged chunks 1--7 overstated the current first 2800 lines by
8489 bytes, while the old chunk 8 understated current lines 2801--3200 by 4141
bytes. The net original discrepancy was therefore 4348 bytes.

The entire baseline `engine.rs` was then rebuilt from nonoverlapping immutable
source ranges. Assembly attempt 4 (`32593467393`, trigger commit
`bc5b0bb52208289738dd93ef71fc923fda0af137`) reduced the discrepancy to exactly
eight bytes: candidate blob `bf5bf001cd3a1903c7eaa777d0b1397799089364`
at 132625 bytes versus expected blob
`63e4db1028b462096f9be9fa0de6ec26ca5af206` at 132633 bytes. No candidate was
accepted or written because the SHA fence remained closed.

The public assembler was strengthened at commit
`0c3e3b4bd44ec45cb09135f0f1b8bcc4a353d69c`. It now records every part's byte
length, line count, and Git blob SHA and, only for the exact eight-byte deficit,
searches every byte offset for one insertion of eight spaces. A candidate is
accepted only if that operation reproduces the complete immutable expected Git
blob SHA. The successful-assembly path no longer deletes `.sync`, preserving
staged recovery material for later large modules.

Diagnostic attempt 5 was triggered by public commit
`4e7685c57824ae7cf85c7ccaa4c71dc569f11fe6`. Its result is pending read-back;
this file deliberately makes no success claim before `.ci/assembly-latest.json`
and the resulting public `engine.rs` blob are verified.

A separate transfer experiment established that exact base64 Git-object copying
is available through the connector: recreating baseline `rust/Cargo.toml` in
`playground` returned the identical blob SHA
`97a16758ec3239edaf78f0ca811694f2536ca796`. This will replace manual source
transcription where the connector response size permits it; every transfer is
admitted only by exact SHA equality.

The remaining known mirror mismatches are `exact_estimator.rs`,
`generic_jla.rs`, `model_solver.rs`, `control_basis.rs`, `cmg/hierarchy.rs`,
`solver.rs`, the new core integration tests, the plugin sources (especially
`ffi_engine.rs`), and plugin integration tests. These must all be exact before
a Rust matrix result is scientifically usable.

## Evidence policy

- Rust has not been compiled locally in the ChatGPT execution environment.
- GitHub Actions is the only Rust compiler and test runner used here.
- Stata has not been executed in this environment.
- No Rust checkpoint is called green until `.ci/latest.json` is read back from
  `playground`, names the exact private source SHA, and reports every required
  matrix command successful.
- A red run against an incomplete or mixed source snapshot is classified as a
  controller failure, not as a source failure.
- No source chunk, inferred insertion, or recreated blob is trusted merely
  because it parses; byte length and the complete Git blob SHA must both match
  the private source object.

## Exact resume point

1. Read back diagnostic attempt 5 and verify any inferred insertion by the full
   expected Git blob SHA; otherwise use its per-part diagnostics to isolate the
   remaining eight-byte transcription loss.
2. Read back the resulting public `engine.rs` blob and confirm exact equality.
3. Verify or replace the complete staged `exact_estimator.rs`; do not assume its
   earlier chunks are current merely because they are complete.
4. Stage and hash-assemble `model_solver.rs`, then `generic_jla.rs`.
5. Replace the stale medium core modules and add all missing core tests.
6. Replace the plugin source and integration tests, including the large
   `ffi_engine.rs` and `engine_ffi.rs` blobs.
7. Verify the complete public Rust tree against the private baseline tree.
8. Run and inspect the Ubuntu/macOS/Windows by Rust-1.81/stable matrix; fix
   actual source failures in small private commits and mirror each tested SHA.
9. Only after a green exact-source baseline, finish private Ado solve-V4
   dispatch and detailed V7 reconciliation.
10. Add static/C/Rust boundary tests for the V4/V7 lifecycle; keep licensed
    Stata execution explicitly pending.
11. Expose the planned routing and receipt surface, implement Rust exact stayer
    parity, then profile and optimize large-N memory traffic, batched PCG, CMG
    application, and deterministic parallel kernels.

Update this file at each diagnostically useful or green remote checkpoint so a
future thread can resume without relying on chat history.
