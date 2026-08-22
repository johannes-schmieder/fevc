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

## Current controller diagnosis

The first public six-cell matrix run (`32586393479`) used an incomplete and
mixed-version Rust tree. Large modules were absent and several retained modules
were stale. Its red result is not source qualification evidence.

Subsequent assembly attempts correctly failed closed on the authoritative
`engine.rs` object:

- expected Git blob: `63e4db1028b462096f9be9fa0de6ec26ca5af206`;
- expected length: 132633 bytes;
- an early mixed snapshot produced 136981 bytes;
- replacing only one stale tail made the mixed-version problem explicit;
- a complete restaging then produced a candidate eight bytes short;
- restoring five stripped part-boundary newlines produced the latest readable
  diagnostic candidate at 132630 bytes, Git blob
  `58f18fa5812cf77e690eb423fdc96818ff672162`, three bytes short of the
  authoritative object.

The public source parts must not be called exact merely because their stated
line ranges look complete. At least one separately staged tail fragment was
visibly truncated mid-token. Text-copy staging is therefore retired.

## Artifact-based recovery checkpoint

The public assembly workflow was updated in commit
`a59e37d6e7936b4db48a3c0693e829fd6578dd34` to upload every assembled candidate
as a GitHub Actions artifact, whether verification succeeds or fails. The
workflow still fails closed and still commits its machine-readable diagnostic;
the artifact is diagnostic only and is never installed as verified source.

Artifact-enabled assembly attempt 7 was triggered by public commit
`56efffb0da7431a866ed59eed4a83b76accf4c60` with the exact private baseline SHA
recorded in `.sync/READY`. The next action is to inspect that workflow, download
the candidate artifact through the GitHub connector, verify its local size and
Git blob SHA, and compare only targeted source ranges against immutable private
content. This avoids further blind source transcription.

A separate small-file experiment established that exact base64 Git-object
copying is supported by the connector: recreating baseline `rust/Cargo.toml`
returned the identical blob SHA
`97a16758ec3239edaf78f0ca811694f2536ca796`. Large connector responses are
truncated at the response boundary, so a base64 payload is accepted only when
it can be decoded completely and independently hash-verified.

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

1. Inspect the workflow triggered by public commit `56efffb0`, read its jobs and
   logs, and download its source-candidate artifact.
2. Establish the candidate's byte length and Git blob SHA locally. Do not call
   this local inspection a Rust build or test.
3. Locate and repair the remaining three-byte `engine.rs` discrepancy using
   targeted immutable private ranges; rerun the SHA fence until exact.
4. Transfer and hash-fence every remaining mismatched Rust and integration-test
   object.
5. Verify the full public Rust tree against the private baseline tree.
6. Run the required six-cell matrix: `cargo fmt --all -- --check`, strict
   workspace/all-target Clippy, debug tests, release tests, and release builds,
   plus the standalone Stata-boundary/C/ABI gates.
7. Fix actual Rust failures in small commits on
   `codex/rust-backend-completion`; after each coherent repair, mirror the exact
   tested SHA and update this file with the public run ID and result.
8. Only after a green exact-source baseline, finish private Ado solve-V4
   dispatch and detailed V7 reconciliation. Licensed Stata lifecycle testing
   remains explicitly pending.
9. Then expose the planned routing/receipt surface, implement Rust exact stayer
   parity, and profile large-N memory traffic, batched PCG, CMG application, and
   deterministic parallel kernels.

Update this file at every diagnostically useful or green checkpoint so recovery
does not depend on chat history.
