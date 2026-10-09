# Stata plugin boundary

This crate builds the package-owned `vckss` Rust backend for `fevc` as an
ordinary Stata C plugin. It uses StataCorp's public SPI 3.0 compatibility files,
`stplugin.c` and `stplugin.h`, authenticated against the tracked hash manifest.
It does not require a separate Stata SDK.

The plugin is the native backend for the one public `fevc` command. Omitted
`backend()` and `backend(auto)` prefer a qualified plugin after complete
preflight, while `backend(rust)` remains strict and `backend(mata)` remains
explicit Mata. The nested portable source manifest omits native binaries;
the root repository installation manifest includes them.

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

## Additive outcome-centering API 1

The existing C ABI structures and flag meanings are unchanged. The header
exports these additive functions inside its `extern "C"` block:

```c
uint32_t vckss_rust_centering_schema_v1(void);
int32_t vckss_rust_engine_centering_v1(uint64_t generation, uint32_t mode);
```

The schema function returns 1. Modes are 0=None, 1=Mean and 2=Corrected.
A newly prepared internal generation defaults to None; the public `fevc`
frontend explicitly configures Mean when `centering()` is omitted.
Configuration is tied to that
generation, occurs before solve/RNG, rejects conflicts with unsupported
attachments, and is rechecked during V4 solve. Stale generations and invalid configurations
fail through the existing typed-error path.

The shim exposes `centeringv1 generation mode` for the internal wrapper and
reports `centering_api` in its probe. `fevc_rust probe` returns
`r(centering_api)`. Active native centering requires 1; absent/zero capability
is a structural preflight failure. `backend(auto)` may select Mata before
preparation/RNG under existing consent rules; strict Rust or explicit
Counter-V1 requests fail with `RUST_BACKEND_UNAVAILABLE`. No centering failure
permits fallback after stochastic work.

Exact, generic/compressed JLA and existing hybrid point executors receive
the configured mode. Corrected JLA requires even probes of at least four
and uses the existing full/two-half leverage pools. Mean MCSE fixes the
observed mean; Corrected fixes the added increment too and returns Mean's
MCSE/covariance. Mean projection and Mean component highrank/q1 inference
are supported on their existing tuples; Corrected with either attachment is
rejected by the frontend and native boundary.
Projection uses the same frequency-weighted retained working-outcome mean as
point centering. Only the outcome factor in covariance contractions changes;
projection coefficients and residual-squared naive covariance are unchanged.

Numerical API 2 and centering API 1 are separate capabilities. Point-centering,
Mean-projection and Mean-component checks are required against the exact installed candidate;
capability exposure alone is not runtime qualification. The local candidate
status is recorded below and in [native provenance](../../native/README.md).

## Additive projection-centering API 1

The native probe exposes `projection_centering_api`; the Stata wrapper returns
`r(projection_centering_api)`. Mean plus projection requires this capability to
equal 1 in addition to centering API 1. None projection does not require it.
This separate capability prevents an older plugin with point-centering support
from accepting a projection request whose covariance it cannot center. Missing
capability is a structural preflight failure under the usual strict Rust and
automatic-Mata routing rules, before preparation/RNG.

The five payloads published at `66d0278b` support all three centering capabilities.
Mac builds retain full and installed Mean qualification at `b9f80ce9`; Linux
retains it at `63757839`. Windows at `240ad74d` passes hosted checks and private
smoke/full qualification, including installed Mean and Corrected exact tests.
The original Mac/Linux binaries and source archives retain their identities;
reuse is limited to unchanged public routes and excludes the unrepaired
lower-level serial exact Corrected path. The new Windows binary contains that
repair. The [adoption manifest](../../native/mean-component-20261008/manifest.json)
binds sources, tested routes and compatibility. Local installation checks
pass at `240ad74d`. At published source `66d0278b`, the four public `net`/`github`
fresh/replacement cases have status PASS on Mac arm64,
with all 61 final files, five plugin hashes and eight Mean cells per case. The
[public-install summary](../../native/mean-component-20261008/evidence/packaging/public-install-66d0278b.json)
and [exact binding](../../native/mean-component-20261008/evidence/packaging/public-install-binding-66d0278b.json)
retain this scope separately from other platform qualification. Earlier
failures and receipts are preserved. Existing ABI
structures are unchanged.

## Additive component-centering API 1

The Rust core advertises `VCKSS_CORE_COMPONENT_CENTERING_V1_READY` as bit 16
of `core_ready_flags` in the existing capabilities struct/export. The C shim
exposes `component_centering_api=1` only when this bit is set, and
`fevc_rust probe` returns `r(component_centering_api)`. Both the matching C
transport and Rust readiness are required: absent metadata is zero, with
cached scalars cleared before and after a probe. This adds no DLL export and
changes no request, result or capabilities layout.

Mean component inference requires this capability and point-centering API 1
before preparation/RNG. Native Mean projection separately requires
projection-centering API 1. Existing combined exact-Mata requests support Mean;
native combined component/projection requests remain unsupported for both
Mean and None. No platform tuple is added. Explicit None retains its previous
capability requirements. Validate configuration both before and after
attachment; Corrected remains a typed pre-RNG failure. No late fallback is
permitted.

Mean holds the observed retained physical-frequency working-outcome mean
fixed. Realized influences and q1 leading/remainder terms consume the centered
outcome; the residual-moment variance fit uses unchanged residuals. Gaussian
inference error probes are never shifted by the observed mean or their own
sample means. Fixed-offset match collapse uses `sqrt(F_g)(ubar_g-c)` and one
error draw per independent declared match. The approximation omits
mean-estimation uncertainty, separately from estimated nuisance-offset
uncertainty, and does not imply conditional validity given the observed mean.
The public metadata are `e(inference_centering)` and
`e(inference_mean_omitted)` on active component requests.

The exact-Mata counterpart uses the runtime build identity
`vckss-inference-api2-q1-target-status-projection-mean1-component-mean1` to
prevent stale installed inference code from silently using the old calculation.
It retains target-specific smoothing, unlike the native residual-moment fit.

`test_component_centering_exact.do` and `test_component_centering_native.do`
run in affected source, native and isolated-install profiles. Core/FFI and C
transport tests cover the bit, missing/stale probe metadata, lifecycle and
unchanged layouts. Qualification must bind the new source and exact intended
payloads; earlier point/projection receipts and capability exposure do not
qualify this extension. See [the inference contract](../../fevc/docs/INFERENCE.md)
and [native test plan](../TEST_PLAN.md).

## All-probe numerical attachment

The frontend defaults to `mcse(all)`; `mcse(off)` omits additional derivative,
fold and replay work. `mcse(conditional)` is a developer option and
`numericalmcse()` remains a compatibility alias. Supported projection and
component-inference attachments keep their existing computation and results;
MCSE covers only main point estimates.

Numerical V2 adds a 344-byte request carrying the original point executor,
automatic component-width intent and prepared attachment flags, plus a
176-byte combined work receipt with the frozen 160-byte point-work prefix and
separate replay work. Numerical V1's 328-byte request retains its platform and
point-only restrictions. The 512-byte numerical result and 24-byte replay RHS
layouts remain unchanged. Additive legacy V2 numerical selectors consume the
frozen compressed V2 and generic V3 interrupt requests, preserving their point
receipts and literal batches. Native V2 is enabled on macOS, Linux and existing
supported Windows routes; this does not enable new Windows solver/inference
routes. Runtime probe `numerical_api=2` is distinct from qualification.

An implicit default with an older runtime preserves point routing and reports
MCSE unavailable. Explicit all requires V2 before preparation/RNG. The original
V1 work getter refuses numerical-attached contexts; V2 reconciles total work
without changing V1 meanings. Exact-artifact platform qualification and adopted
binary identities are recorded separately. See the
[default/interface decision](../../fevc/docs/MCSE_DEFAULT_INTERFACE_2026-09-30.md)
and [implementation status](../../fevc/docs/ALL_PROBE_MCSE_STATUS.md).

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
qualified separately on SCC. Current all-platform build/runtime qualification
and repository distribution are recorded in [`native/README.md`](../../native/README.md);
release tags and archives remain separate owner decisions.

The Linux qualifier requires point-, projection- and component-centering
API 1 on both the staged candidate and its isolated installation. It runs
the eight-cell Mean-projection test and both component-centering regressions
against both copies, checks the installed plugin hash, and runs the four
point-centering tests with explicit `rust` against the installed package.
These checks supplement the full suite, whose ordinary point-centering tests
use Mata. The receipt records their results separately; adding a gate does
not qualify an older Linux payload for either attachment. Mac qualification
also runs the component regressions for thin and universal arm64/Rosetta
installs; the Windows full runtime profile includes them for its isolated
installation. These are required checks, not statements of completed
qualification.

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
