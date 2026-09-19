# Stata plugin boundary

This crate builds the package-owned `vckss` Rust backend for `fevc` as an
ordinary Stata C plugin. It uses StataCorp's public SPI 3.0 compatibility files,
`stplugin.c` and `stplugin.h`, authenticated against the tracked hash manifest.
It does not require a separate Stata SDK.

The plugin is the native backend for the one public `fevc` command. Omitted
`backend()` and `backend(auto)` prefer a qualified plugin after complete
preflight, while `backend(rust)` remains strict and `backend(mata)` remains
explicit Mata. Native binaries are not shipped by the tracked source manifest.

## Boundary and lifecycle

The versioned boundary provides:

- capability request/receipt V3;
- prepare with retained-mask, graph, control, target, and memory receipts;
- solve/interrupt V4;
- exact, compressed-JLA, and generic-JLA result families;
- detailed execution-plan and numerical receipt V7;
- additive diagnostic performance receipt V1;
- execution-only V6 for explicitly selected generic diagonal queues and
  direct-CMG projection/component attachments, plus a separate complete-work
  receipt V1 (development candidate; qualification status is in the ledger);
- explicit memory-budget presence/check policy V1 and expected/admission
  forecast V1, preserving older ABI request layouts; and
- generation-safe result, release, clear, snapshot, and typed-error handling.

The Stata wrapper must reconcile the complete request, prepared generation,
selected family, execution plan, numerical diagnostics, memory, Counter facts,
and caller-state restoration before posting estimates. Failed or corrupt
receipts are typed failures, not fallback invitations.

The native planner resolves `algorithm(auto)` to exact or JLA and
`engine(auto)` to compressed, generic, or not-applicable. Public auto-exact,
broad effective-option admission, automatic JLA selection, `probeorder()`,
stayer augmentation, macOS arm64/Rosetta, and Linux/SCC are qualified for
their recorded source commits. The registered no-control match-JLA cell now
selects `CMG_FULL_V2` through explicit Rust or qualified macOS/Linux automatic
routing. Current qualification and release boundaries are recorded in
[`../../fevc/PLAN.md`](../../fevc/PLAN.md).

## Public and legacy memory policy

The additive V6 request is 304 bytes with an exact V4 prefix; its interrupt
request is 328 bytes. It selects diagonal queue (1) or direct attachments (2),
requires positive permitted threads and the existing explicit generic-JLA,
Counter-V1, no-fallback tuple. Direct attachments additionally require explicit
CMG and automatic point batches. Existing augmentation widths remain literal,
including eight; inference omission is not inferred from a numeric width.
Readiness bit 12 identifies this execution interface on Mac/Linux builds, not
a new statistical capability or a platform/performance qualification claim.
The Stata probe additionally exports `execution_api=3` for its matching C
selectors. Earlier FFI-only experimental binaries already advertised bit 12;
public V6 requests require both facts before preparation. Missing transport
metadata defaults to zero after clearing any cached scalar. This does not
change the native capability ABI or any statistical request signature.

The additive V8 request is 320 bytes (344 bytes with interruption) and retains
the exact V7/V6/V4 prefixes. Execution mode 3 preserves the original automatic
route and resolves queued diagonal versus direct CMG from the registered firm
and planned-RHS rule before estimator RNG. Its suffix records whether the user
supplied a tolerance so fit and probe tolerances retain their existing phase
semantics. Readiness bit 14 and `execution_api=3` are both required before
public preparation; older binaries fail closed without a native context.

The separate 160-byte `VckssGenericExecutionReceiptV1` counts fit, strict rank,
point, projection, component and Gram RHSs. Queued work excludes scalar diagonal
fits; direct work includes fits and extra complete-model refinements. Measured
diagonal concurrency and CMG's selected concurrency bound have distinct fields;
zero measured CMG concurrency means uninstrumented, not zero actual activity.
All V1--V5 solve layouts and meanings are frozen, including ignored V5 threads
when its full-CMG flag is zero. The point-only 56-byte model receipt rejects
attachment execution instead of silently mixing inference work into its counts.
The interface adds no public Stata option, automatic inference-width policy,
estimator fallback or installed package replacement. Qualification and remaining
integration are recorded in the optimization-parity experiment ledger.

The public wrapper now uses the private `solveexecution` selector for explicit
generic/JLA/diagonal requests (automatic or literal point batches), and for
explicit generic/JLA/CMG projection/component attachments with automatic point
batches. The C shim validates the complete 160-byte receipt before exporting
23 exact scalar fields; the Ado `executionreceipt` helper returns their matrix
and clears transport scalars. Public reconciliation checks actual work,
admitted memory, original-system residuals and caller state before posting
`e(rust_execution_receipt)`. V3 request signatures are independently reproduced
before preparation. Legacy solve selectors and V5 point-only accounting remain
unchanged. Effective automatic routes and inference omission intent still need
completion; do not interpret this explicit-route hookup as all-path parity.

Public `fevc` uses the additive memory-policy interface. An omitted
`memory_gib()` carries explicit absence; it is not a 4-GiB default or an
arbitrarily large cap. Explicit budgets warn by default, while `error` opts
into strict forecast admission and `off` suppresses warnings/rejection.
Budget-driven automatic planning requires an explicit budget. The old native
and private diagnostic entrypoints retain their prior strict numeric defaults.

The preparation peak measures requested heap payload while preparation runs,
including accounted C-buffer overlap. Actual CMG setup refines the solve
forecast before estimator RNG. The forecast receipt separates expected peak
from admission peak and conditional reserve; it excludes total process RSS.
Warnings do not waive structural receipt checks or actual allocation failures.
See [the current memory contract](../../fevc/docs/MEMORY.md).

## Qualify a local macOS candidate

On Apple Silicon with licensed Stata 18 or newer at the standard path, run from
the repository root:

```bash
rust/stata_backend/qualify_macos.sh \
  --receipt /private/tmp/vckss-macos-candidate-receipt.txt \
  --artifacts-dir /private/tmp/vckss-macos-sanitized-evidence
```

Use `--stata /absolute/path/to/stata-mp` for another installation. The receipt
path must not already exist. The optional artifacts directory must exist and
be empty.

The qualifier:

1. authenticates the pinned SPI sources;
2. runs locked Rust formatting, strict Clippy, tests, and C shim/ABI gates;
3. builds thin arm64 and x86_64 slices and a universal binary from one source
   manifest;
4. audits architectures, deployment floors, install IDs, dependencies,
   signatures, and required exports;
5. runs fresh licensed-Stata plugin lifecycle, shared-atom, exact, compressed,
   generic, routing, fault, corrupt-receipt, and clean-install tests on arm64;
6. repeats architecture-sensitive coverage under Rosetta when available; and
7. writes source, SPI, binary, and sanitized-artifact hashes only after every
   required PASS marker is present.

Raw Stata logs and temporary installs are deleted. Sanitized transcripts begin
at the first batch prompt and omit startup/license banners. A dirty worktree is
labelled as a local checkpoint, not a clean qualification. Rosetta is
compatibility evidence on Apple Silicon, not native Intel qualification.

The local qualification alias is:

```bash
./ci/run_stata_ci.sh plugin-build
```

Read the exact-SHA receipt under `.ci/stata/results/` and inspect the local
Rust/C and licensed-Stata outputs. See [`../TEST_PLAN.md`](../TEST_PLAN.md).

## Manual macOS build

For iterative development without a candidate receipt:

```bash
rust/stata_backend/fetch_stata_spi.sh
vckss_cargo_185=$(rustup which --toolchain 1.85.1 cargo)
vckss_rustc_185=$(rustup which --toolchain 1.85.1 rustc)
vckss_rust_bin_185=$(dirname -- "${vckss_rustc_185}")
env PATH="${vckss_rust_bin_185}:${PATH}" RUSTC="${vckss_rustc_185}" \
  "${vckss_cargo_185}" test --manifest-path rust/stata_backend/Cargo.toml \
  --locked --all-targets
env PATH="${vckss_rust_bin_185}:${PATH}" RUSTC="${vckss_rustc_185}" \
  "${vckss_cargo_185}" clippy --manifest-path rust/stata_backend/Cargo.toml \
  --locked --all-targets -- -D warnings
env PATH="${vckss_rust_bin_185}:${PATH}" RUSTC="${vckss_rustc_185}" \
  "${vckss_cargo_185}" build --manifest-path rust/stata_backend/Cargo.toml \
  --locked --release
cp rust/stata_backend/target/release/libvckss_stata.dylib \
  fevc/fevc_rust_macos_arm64.plugin
```

Resolving the exact Cargo and `rustc` executables is intentional: some rustup
installations do not expose the selected toolchain's sibling compiler to
Cargo's child process. `VCKSS_STATA_SPI_DIR` may point the SPI fetch/build to a
separate directory. Local SPI files, Cargo output, and plugin binaries are
ignored by Git.

A manual host build is architecture-specific. Do not rename it to the universal
plugin name; only the qualifier constructs and audits a universal candidate.

## Focused Stata tests

For one test during development:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  fevc/tests/stata/test_rust_planned_compressed_post.do \
  /absolute/path/to/checkout/fevc/fevc
```

Other native tests live beside it under `fevc/tests/stata/`. Always
check the explicit terminal PASS marker, the native registry's idle state, and
caller RNG/data/sort restoration. Do not commit raw Stata logs.

## Qualification boundary

The macOS qualifier makes no Linux, Windows, native-Intel,
representative-scale, production, inference, or public-release claim. Linux is
qualified separately on SCC for the alpha; Windows and public distribution
remain deferred pending their own gates and human review.

## Runtime reporting

The optional `progress_api=2` probe field identifies the `reportv2` Stata
selector and includes support for the existing `reportv1` selector and
`vckss_rust_report_call_v1` synchronous scope. Its 32-byte
`VckssProgressOptionsV1` supplies schema 1 or 2, display level (0/1/2), a zero
reserved field, and a caller-owned callback/context. Level zero requires a
null display/context. Each callback receives a borrowed 96-byte update,
elapsed milliseconds, and the display level; return values are Stata statuses.
Schema 1 retains phase-relative times and separate probe messages. Schema 2
reports elapsed time for the entire synchronous call and coalesces paired
leverage/target counts in update values 0--3; value 4 is zero while targets
are pending. The C formatter adds the command's elapsed time before this call,
supplied by `reportv2 level elapsed_ms operation ...`.
Existing estimator requests, signatures, receipts, and entrypoints are unchanged.

The scope owns fixed stack storage. Numerical coordinators borrow it through
joined thread scopes; no worker receives a host pointer or calls Stata. Counts
are published at batch boundaries, not inside numerical kernels. The existing
host poll drains coalesced updates, with final draining only on success. Errors
and Break retain their existing cleanup path. Publishing never changes
`is_inert()`, cancellation timing, pool selection, or memory admission.

The Ado reporting policy uses the existing command-local context pattern,
cleared on entry and every captured exit, so internal hybrid `nodisplay` calls
do not overwrite the public display choice. Direct `fevc_rust` calls remain
silent unless invoked inside that public scope. Old probe fields default to
zero after cached transport scalars are cleared.

The public wrapper owns an unused Stata timer for the command's lifetime,
including native preparation, solving, inference, Ado validation, and cleanup.
Running and accumulated caller timers are never borrowed. At native call
boundaries it samples that timer; Rust uses its monotonic clock within the
call. The timer is cleared on success, error, and Break. If no reporting timer
is free, estimation continues with one notice instead of live updates.
