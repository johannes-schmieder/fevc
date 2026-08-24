# Remote Rust CI status

Last updated: 2026-08-22

Development branch: `codex/rust-backend-completion`
Baseline source commit: `ce76810348b96d377c1b52907a2fd9fdb76f4909`
Public compiler/test controller: `johannes-schmieder/playground`

## Evidence policy

- Rust has not been compiled locally in the ChatGPT execution environment.
- GitHub Actions is the only Rust compiler and test runner used here.
- Stata has not been executed in this environment.
- No Rust checkpoint is called green until `.ci/latest.json` is read back from
  `playground`, names the exact private source SHA, and reports every required
  Ubuntu/macOS/Windows and Rust-1.81/stable command successful.
- A red run against an incomplete or mixed source snapshot is a controller
  failure, not evidence for or against the private numerical source.
- Every mirrored source file must match both its authoritative byte length and
  Git blob SHA before Cargo output is considered source-bound evidence.
- Connector output that is merely labelled with a source range is not trusted
  unless its beginning, end, byte count, and reconstructed Git identity are all
  independently complete and verified.

## Current controller diagnosis

The first public six-cell matrix run (`32586393479`) used an incomplete and
mixed-version Rust tree. Large modules were absent and several retained modules
were stale. Its red result is not source qualification evidence.

The authoritative baseline `engine.rs` object is:

- Git blob `63e4db1028b462096f9be9fa0de6ec26ca5af206`;
- 132633 bytes; and
- 3636 newline-terminated source lines.

The latest assembled diagnostic candidate is:

- Git blob `34dfcf183c9924e95b4cb11e46e230980f128640`;
- 132630 bytes; and
- the same 3636-line structure.

Artifact-enabled assembly run `32593805270` published that failed candidate as
artifact `9481016826`. The artifact was downloaded and independently checked in
the working container. This was source inspection only, not Rust compilation.

Rust-1.81 rustfmt diagnostic run `32596758161`, triggered by public commit
`148a146bc2a52007872a446d8646c9c9a416983e`, parsed and formatted the candidate
successfully but left it byte-for-byte unchanged at blob `34dfcf...` and 132630
bytes. Therefore the three missing bytes are not recoverable formatting and are
part of the source content or a connector-transfer truncation.

The only localized equality currently established is baseline versus candidate
lines 1--250: both are 7864 bytes and byte-identical. No later range is yet
certified equal or different.

## Retracted diagnostic

A prior apparent localization to lines 2776--2850 is invalid and must not be
used. The connector response copied into public blob
`c2ffdca5a9976a57dff8edd0c040d67abd1c0dfa` began mid-token with
`anned_unique_packed_words,`; it was itself truncated before transfer. Public
commit `c7252ab69522b90bbbf8463f0863a4fe76b6af6f` linked that invalid diagnostic
at `.sync/exact-range/engine-2776-2850.rs`. The file is not authoritative source
and should be removed or explicitly quarantined before further assembly work.

This incident establishes the safe transfer rule: do not copy displayed
base64/text from a connector response unless the complete payload is visibly
present and the reconstructed object independently matches an expected Git
blob SHA. Small-file exact transfer remains proven for `rust/Cargo.toml`, whose
recreated object matched blob `97a16758ec3239edaf78f0ca811694f2536ca796`.

## Remaining exact-mirror work

After `engine.rs`, the known missing or stale baseline objects include:

- `exact_estimator.rs`;
- `generic_jla.rs`;
- `model_solver.rs`;
- `control_basis.rs`;
- `cmg/hierarchy.rs`;
- `solver.rs`;
- the new core integration tests;
- plugin source, especially `ffi_engine.rs`; and
- plugin integration tests, especially `engine_ffi.rs`.

The complete public Rust tree must be compared against the private baseline
Git tree before the compiler matrix is treated as meaningful.

## Exact resume order

1. Remove or quarantine the invalid public diagnostic range from commit
   `c7252ab...`; do not use it in any source manifest.
2. Diagnose `engine.rs` only with small complete private windows around source
   part boundaries, or replace the mirror with a direct authenticated checkout
   mechanism. Require exact 132633-byte and `63e4db...` admission.
3. Transfer and hash-fence every remaining mismatched Rust and integration-test
   object.
4. Verify the full public Rust tree against the private baseline tree.
5. Run the required six-cell matrix: `cargo fmt --all -- --check`, strict
   workspace/all-target Clippy, debug tests, release tests, and release builds,
   plus the standalone Stata-boundary/C/ABI gates.
6. Fix actual Rust failures in small commits on
   `codex/rust-backend-completion`; after each coherent repair, mirror the exact
   tested SHA and update this file with the public run ID and result.
7. Only after a green exact-source baseline, finish private Ado solve-V4
   dispatch and detailed V7 reconciliation. Licensed Stata lifecycle testing
   remains explicitly pending.
8. Then expose the planned routing/receipt surface, implement Rust exact stayer
   parity, and profile large-N memory traffic, batched PCG, CMG application, and
   deterministic parallel kernels.

Update this file at every diagnostically useful or green checkpoint so recovery
does not depend on chat history.
