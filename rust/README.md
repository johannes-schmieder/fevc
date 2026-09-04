# Rust backend

This directory contains the optional native backend for `fevc`. It is a
package-owned implementation, not a separate public command. Omitted
`backend()` and `backend(auto)` prefer Rust after a complete preflight
capability check, with Mata fallback allowed only before preparation and RNG.

Rust is either an explicit opt-in or a fully preflighted automatic selection
and must cross the compositional Stata/plugin boundary: request capability,
prepare, solve, result, and release. The native result and
its execution-plan, numerical, memory, counter, and cleanup receipts must all
reconcile before Stata posts estimates.

Backend development compares corrected statistical results rather than exact
floating-point paths. The active promotion thresholds are registered in
[`../fevc/docs/development_acceptance_v1.json`](../fevc/docs/development_acceptance_v1.json);
bitwise identity and legacy fixed roundoff gates are diagnostic when hard
correctness and corrected-result equivalence pass.

## Source map

- `crates/vckss-core/`: estimator, graph, exact/JLA, solver, CMG adapter,
  planning, memory, wall, counter, component-inference, and receipt logic.
- `crates/vckss-plugin/`: generation-safe native lifecycle and versioned ABI.
- `stata_backend/`: Stata C-plugin shim, build, packaging, and macOS qualifier.
- `RNG_CONTRACT.md`: Counter-V1 contract.
- `SOURCE_PROVENANCE.md`: native-source provenance.
- `TEST_PLAN.md`: active native validation gates.
- `progress/`: dated, source-bound checkpoints. These are evidence, not current
  instructions.

The active cross-language milestone is maintained in
[`../fevc/PLAN.md`](../fevc/PLAN.md). Do not create a second
current plan in this directory.

## Implemented result families

The native core implements three result families:

1. **Exact** — deterministic dense exact calculation for eligible retained
   dimensions. Estimator RNG, probe batches, and iterative preconditioning are
   not applicable.
2. **Compressed JLA** — the registered no-control match specialization with
   separate coefficient-cell, deletion-unit, target-stratum, and Counter-V1
   semantics.
3. **Generic JLA** — the general worker--firm/control estimator with planned
   diagonal or CMG PCG, independent batch widths, memory admission, and
   complete per-RHS receipts.

Algorithm, engine, route, fallback, batch, memory, and wall planning are frozen
before estimator RNG. A later rank, setup, resource, convergence, or numerical
failure cannot choose a different scientific estimator. Explicit CMG fails
closed; automatic setup fallback to diagonal is allowed only before RNG and is
recorded.

## Public boundary

The package exposes explicit Rust exact and planned compressed/generic JLA.
The V3/V4/V7 public `algorithm(auto)` path is qualified when its frozen plan
selects exact, including the exact-family poster and zero-RNG reconciliation.

The planned generic-JLA lifecycle also has an additive sparse fixed-effect
projection attachment for positive-integer-frequency observation or match
deletion, including the mixed eligible-stayer partition. Before Counter-V1
work, it reduces the synchronous project columns
to a small Gram and coefficient-space RHSs, admitting both C/Rust column copies
and all preparation work. The live model solver applies one inverse action per
projection column through explicit diagonal PCG or forced generic CMG. Forced
CMG shares one admitted hierarchy between the full and FE-only solvers and
never falls back after selection. KSS and residual-squared score covariances
are then streamed into `q`-square accumulators. The V7 result retains phase-6
RHS, complete original-system residual, conditioning, PSD,
persistent/result, and whole-command memory receipts. No dense observation-by-
parameter design or full inverse is constructed. Covariance accumulation uses
the uncentered observation identity or symmetrized mover-match block identity,
with physical-observation stayer blocks. The CMG composition has
focused local exact/diagonal and 6,000/24,000/96,000 convergence evidence; it
does not yet add a same-host MATLAB speed or new cross-platform performance
claim. Projection CMG is the planned generic preconditioner, not the
specialized compressed `CMG_FULL_V2` route.

Generic JLA additionally has an internal oracle layer and a supported explicit
structured-variance observation-deletion component-inference attachment. Its
`q=0`
path reuses retained leverage and three primitive target
diagonals, adds three combined influence solves and one shared full-model solve
per Gaussian covariance probe, streams joint moments, and derives the fourth
target by an exact linear map. Matrix-free trace-square probes and a fixed-
iteration two-vector generalized eigensolver report leading and remainder
spectral concentration for all four targets. The `q=1` path subtracts one
leading mode, recenters its square with the raw observation leave-out mode
variance product, and returns its joint leading/remainder covariance and
curvature, Counter-V1 critical value, remainder-identity diagnostic, and
Andrews--Mikusheva confidence interval. The positive structured variance
vector enters covariance estimation only. It is limited to observation
deletion, mover-only unit-frequency
data, joint controls, and explicit diagonal or CMG routes. Versioned plugin
augmentation and result ABIs expose `structured_common` and
`structured_leverage` only through a separately named Stata option; omission
preserves the existing exact Mata target-specific behavior. The strict
unrestricted-KSS variance-product construction remains unimplemented and is
not a reserved parser mode. A deterministic dense fitted-variance harness and
validator cover exact structured DGPs, mild and severe misspecification,
controls, t8 errors, and diffuse/dominant spectra. The initial 2026-09-03
moderate-dimension confirmation withheld promotion after two `q=1` firm gates
failed. The subsequent factorized V2
diagnosis isolated one oracle t8 failure at dimension 64. Result ABI V3 and
the registered V3 campaign test the corrected raw recenter at dimensions
16--64, with independent numerical-integration and ellipse-image oracles and a
100,000-draw minimum on the public q=1 path. The source-bound V3 development
run completed all 20,000 attempts and satisfied the remainder identity, but
the oracle t8 firm cell at dimension 64 again covered 0.972 and failed its
fixed gate. V4 isolated that result as expected modest conservatism from the
maximal-curvature, at-least-nominal reference law rather than a recentering,
covariance, studentization, or ellipse-image defect. The clean preregistered V5
confirmation then passed every frozen `q=0`, eligible one-mode `q=1`,
misspecification, execution, and inventory gate. The supported surface remains
explicit and target-specific: no automatic `q` selection is added, severe
omitted variance drivers can invalidate inference, and a successful
multi-mode calculation lies outside the `q=1` coverage claim.
Promotion source `7608942a09c643fcb78fb52885d3b87333c5429f` passed the
source-bound macOS arm64/Rosetta and licensed-Stata plugin qualifier. Its
formal V5 compatibility review and sanitized evidence packet are indexed from
`../fevc/docs/STRUCTURED_OBSERVATION_INFERENCE_PROMOTION_2026-09-04.md`.

The next internal layer implements the first fixed-offset collapsed-match
`q=0` foundation without changing that public surface. Conditional on the
full-sample control offset, one declared match becomes one scalar FE inference
row. Its regression mass is retained algebraically rather than expanded into
independent inference copies. The internal attachment supports aggregate-match
oracle variances and a separately registered structured match-variance fit,
uses one Gaussian draw per independent match, preserves the existing
whole-match point estimate, and reports match-mass, maker, influence, spectral,
solver, PSD, and nuisance-conditioning diagnostics. It remains mover-only,
requires explicit match deletion plus `nuisance(fixedoffset)` and an explicit
diagonal or CMG route, and rejects q=1, mixed stayers, projection composition,
automatic routing, and public invocation. This is development infrastructure,
not qualified or released match inference.

Implementation source `77177a6497891d8f6e1cab0aca89366f4e4ca4ad`
passed the source-local macOS arm64/Rosetta and licensed-Stata plugin profile.
That qualification protects the native build and unchanged public routes; the
internal grouped path itself is covered by Rust integration and independent
dense-oracle tests and still has no coverage claim. The sanitized packet is
under `qualification/evidence/MATCH-FIXEDOFFSET-Q0-MACOS/`.

The source-bound campaign executable is
`crates/vckss-core/examples/match_inference_q0_development.rs`; orchestration is
owned by `../fevc/tools/run_match_inference_q0_campaign.py` and frozen by
`../fevc/docs/match_inference_q0_campaign_v1.json` plus its pre-result
`../fevc/docs/match_inference_q0_campaign_v1_amendment1.json`. The amendment
repairs only the one-mode outcome-free fixture and the tiny numerical
resolution after the development preflight stopped before manifest creation.
The preflight validates
target-specific diffuse, one-mode, and multi-mode regimes before creating a
manifest. The complete tiny path and exact-source one-core SCC Linux
build/task/aggregate smoke pass at source `983ed37`; their source, inventory,
receipt, artifact, and `qacct` record is in
`../fevc/docs/MATCH_INFERENCE_Q0_CAMPAIGN_SMOKE_2026-09-04.md`. These are
non-evidentiary pipeline checks for the original source. The amended complete
local tiny path also passes; a new clean exact-source SCC smoke is required
before the larger registered development profile runs. The campaign remains
internal and implements q0 only.

Effective default admission, `algorithm(auto)` selecting JLA, and semantic
`probeorder()` tie breaking are qualified on macOS arm64 and Rosetta. Match
deletion defaults to `stayers(both)`: a combined target with mover-match and
stayer-observation corrections. Exact and generic JLA use the versioned native
augmentation lifecycle; `stayers(movers)` retains the compressed and ordinary
mover-only paths. Exact has separate-source reconciliation and a zero-RNG
contract; generic JLA uses one joint fit and leverage sketch. The
platform and bounded safety gates are qualified: macOS arm64/Rosetta, SCC
Linux x86-64, Miri, C-shim ASan/UBSan, malformed-ABI fuzzing, RustSec audits,
license inventory, and CycloneDX SBOMs have source-bound evidence under
`qualification/evidence/`. The alpha qualification packet adds exact-source
macOS, SCC Linux, supply-chain, representative-performance, and rendered
benchmark evidence without making a Windows or binary-release claim.

M5 instrumentation uses the additive
`VckssEnginePerformanceReceiptV1`, separate from the frozen V7 numerical and
pre-RNG plan receipt. It reports native ingress, canonicalization, graph,
compression, plan, stayer-augmentation, solve, and summed wall-clock phases.
These diagnostics never enter admission, routing, numerical work, RNG
accounting, or result reconciliation decisions.

The productionization wave vendors standalone CMG commit `92a12f2` under
`vendor/cmg`, pins Rust 1.85.1 (MSRV 1.85), and assigns the scalar direct
hybrid solver the normal-build identity `CMG_FULL_V2`. The registered
no-control match-JLA cell is available through explicit `backend(rust)` and,
after accepted macOS and SCC evidence, through `backend(auto) rng(auto)` on
qualified macOS and Linux builds. Structurally unsupported requests retain
their existing routes; a failure after full-CMG selection never falls back.
Windows retains its prior route selection and has no full-CMG qualification
claim. Full-CMG setup and solves run on an owned coordinator worker while the
Stata caller
thread polls UserBreak every 5 ms. Rayon workers see only an atomic
cancellation flag. The generation remains owned until the worker joins, and
success, cancellation, typed failure, or panic produces one terminal state
followed by idempotent release. Whole-command memory admission, actual-retained
reconciliation, and the residual-refinement schedule are frozen before
Counter-V1 begins. The eligible production path also uses the additive V4
preparation ABI to request the certified implicit worker-firm match key. That
request is structural rather than environment-driven, requires match deletion,
no controls, and an explicit probe order, and charges two row-capacity `u64`
vectors in the pre-RNG preparation forecast. Older preparation ABIs retain
their original identifier and deletion semantics.

The accepted alpha production gate is source-bound to runtime source
`4b6874e`. On macOS, five alternating warm runs take a median 74.774 seconds
versus 104.489 seconds for MATLAB R2024b Update 5 and 81.145 seconds for the
private winner. On SCC's fixed CZ18 case, the corresponding medians are 19.097
seconds versus 33.058 seconds for MATLAB R2024b Update 3 and 33.942 seconds for
the private winner. Both cases pass the corrected-target, complete-residual,
memory, wrapper, and process gates. Commit `61dba32` admits the same effective
cell through the automatic backend on qualified platforms; all other
automatic cells remain unchanged. Compact receipts and the CMG-style report
are indexed in the [archived performance records](../docs/history/VCKSS_ARCHIVE.md#performance-records).

The historical performance experiment is the private `CMG_FULL_SPIKE_V1` direct
hybrid-Laplacian batch route under
`experiments/full_cmg_spike/full_cmg_spike.rs`.
That directory is excluded from both normal workspaces; fused, mixed-precision,
and pass-fused sources are retained only as source-bound historical evidence.
It links exact standalone CMG source `92a12f2`, requires explicit private
environment consent, uses one `ParallelPcgSolver` and one bounded Rayon pool,
places transformed firm RHS values on firm vertices with zeros on auxiliary
worker vertices, extracts and recenters firm solutions, recovers workers, and
applies the independent complete original-system residual gate. It is not a
public capability or ABI. Its first registered local hard-case timing improves
the prior backend by 12.35%. Its approximately `2.22e-12` covariance difference
failed the legacy pathwise gate but passes the active development-equivalence
policy and is not a scientific blocker. Accepted SCC job `7306628` measured
223.232 seconds for the candidate versus 329.260 seconds for the matched
baseline and 171.733 seconds for maintained MATLAB on the same node. C is
29.99% slower than MATLAB; its 153.432-second direct solve is the dominant
phase. That original route was rejected for performance, not numerical parity.

The owner-authorized renewed direct-route wave preserves that evidence and
adds ordered parallel Counter-V1 generation, direct leverage RHSs, independent
target preparation, parallel moment accumulation, and bounded parallel
recovery and complete-residual certification. Clean source `598a08d` completes
the same macOS headline in 81.145 seconds, or 0.473 times the registered MATLAB
command, with bit-identical corrected targets and a `6.97e-6` maximum complete
residual. This first 2.12x single run is a development checkpoint only.
Alternating warm medians and the checksum-bound fixed CZ18 SCC case remain
mandatory before vendoring, hardening, or exposing the route. Further
optimization of the simplified embedded hierarchy remains stopped.

On SCC, synthetic job `7311964` reaches 156.455 seconds versus MATLAB's
260.928 seconds, while accepted direct-versus-fused job `7312041` reaches
151.351 seconds for direct and 239.610 seconds for fused f64. Direct is kept;
fused f64 is disabled. The direct result is 1.669x faster than MATLAB but
25.079 seconds short of the 2x threshold, with bit-identical direct/fused
targets and a `6.971259e-6` maximum complete residual. The official full-CMG
repeated solve is the dominant remaining cost. Fixed-CZ18 P20 completes its
61 numerical solves at a `7.993e-6` maximum residual after commit `83d2284`
repairs private fit receipt reconciliation without weakening the independent
complete-system gate. SCC job `7314745` passes posting, state restoration,
qacct, and final validation, but remains a P20 smoke: candidate command time is
91.596 seconds versus MATLAB's 47.154 seconds. Private scalar pass fusion at
`08565be` retains the official hierarchy, independent recurrence, and final
certification, yet its favorable clean local observation improves command and
solve time by only about 2% and raises private admitted memory 2.6%. It is
preserved and disabled without an SCC matrix. The first CZ18 P200 attempt,
job `7314843` at exact source `c1ae402`, correctly fails the unchanged `1e-5`
complete residual gate on target probe 56 (`1.563e-5`) before MATLAB. The next
attempt keeps effective probe tolerance `1e-6` and pre-registers a tighter
private P200 inner solve at `1e-9`; P20 remains `1e-8`. It is not qualification
or promotion evidence until wrapper, qacct, and the pinned validator pass.

Fixed-CZ18 P200 SCC job `7317771` at source `3daa465` supplies the required
alternating hard-case checkpoint. Across five position-balanced warm rounds,
candidate command median is 33.942 seconds versus 70.147118 seconds for
maintained MATLAB R2025b, or 2.0667x MATLAB. Candidate median peak RSS is
2,530,940 KiB versus MATLAB's 4,310,024 KiB. All complete residual, corrected-
target, repeatability, state, process-tree, wrapper, and qacct gates pass. The
initial post-job validator failure was limited to blank optional legacy phase
diagnostics; repair `5a6daaf` validates the unchanged evidence and records its
own hash. This checkpoint did not authorize hardening on its own; the
subsequent alternating synthetic decision below controls promotion.

The alternating synthetic matrix is accepted at SCC job `7318114`, source
`787327f`, but is not promoted. Warm medians are 490.209 seconds for A,
123.633 seconds for C, and 183.017703 seconds for MATLAB. C is 3.9650x A and
1.4803x MATLAB, with all statistical, complete-residual, caller-state,
process-tree, wrapper, and qacct gates passing. It misses the 2x threshold by
32.124 seconds and its maximum process RSS is 4,134,164 KiB versus MATLAB's
3,847,076 KiB. The official full-CMG repeated solve is the bottleneck at
98.664913 seconds across 601 RHSs. At that source the route remained private
and vendoring, hardening, public admission, default-auto, and report work were
deferred. The later scalar production wave and runtime source `4b6874e`
supersede that decision while preserving its evidence. Do not return to the
simplified hierarchy.

Result families must continue to use their matching posters and all receipts
remain mandatory.

## Development gates

From the repository root, use the pinned toolchain and locked manifests:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

Public push and pull-request workflows run hosted source checks only. Native
route qualification requires an explicitly local or private source-local
plugin profile and its exact-SHA receipt; a green quick receipt alone is
insufficient. See [`TEST_PLAN.md`](TEST_PLAN.md) and
[`stata_backend/README.md`](stata_backend/README.md).

## Release boundary

The tracked package ships portable source and Ado boundary helpers, not native
binaries. macOS and SCC Linux plugin artifacts are qualification products,
not a public binary release. Windows, native Intel hardware, package tagging,
binary distribution, and the final mathematical/license/provenance sign-off
remain outside the public-source prerelease claim.
