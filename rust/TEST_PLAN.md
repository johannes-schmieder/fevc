# Rust backend test plan

This is the active native test plan. Dated implementation and CI snapshots are
kept under [`progress/`](https://github.com/johannes-schmieder/fevc/tree/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/rust/progress) and must not be treated as current
instructions. The authoritative milestone order is
[`../fevc/PLAN.md`](../fevc/PLAN.md).

## Evidence rules

- Bind new evidence to the exact tested source SHA and profile. A later source
  may reuse unaffected claims through the compatibility review required by the
  development acceptance policy; a commit change alone does not trigger a full
  native, platform, or scale rerun.
- A licensed Stata receipt does not by itself prove Rust formatting, Clippy, or
  workspace tests; inspect the workflow jobs as well.
- A green quick profile is not plugin qualification.
- Stata batch process status is not authoritative by itself; require the
  profile receipt and explicit PASS markers.
- Preserve failures and exact diagnostics. Apply the registered
  [`development acceptance policy`](../fevc/docs/development_acceptance_v1.json):
  do not change the requested estimator merely to obtain green output, but do
  not treat harmless bitwise/ULP or legacy fixed-roundoff differences as
  promotion blockers when corrected-result equivalence and hard correctness
  pass.
- Receipt-only `[skip ci]` commits are bookkeeping, not the tested source.

## Controlled direct point integration

`v5_controlled_point_model_accounting_and_strict_rank_gates` compares legacy
and direct weighted joint/fixed-offset estimates for both deletion modes,
checks the additive 56-byte model receipt, strict per-RHS rank gates, short
buffers and stale handles. `test_rust_controlled_full_cmg.do` exercises pooled
and mover-only populations, omitted and explicit tolerances, automatic routes,
weights, declared deletion IDs, empty controls, strict budgets, caller-state
restoration and successful reuse. Run it in each Mac native profile and in
isolated installs. The separate stale-model-runtime test must reject bit-10-only
runtimes before preparation/RNG. These extend, not replace, core dense-oracle,
Q32 allocation, rank, cancellation and deliberate-refinement tests.

## Routine source gates

Match movers count distinct original deletion IDs, including parallel blocks
at one model firm. `deletion_unit_graph.rs` independently enumerates all 728
nonempty three-worker/two-firm multigraphs and checks retained rows against
literal vertex/edge removal. `test_pooled_deletion.do` checks physical-copy
and deleted-QR oracles, both populations and nuisance modes, weighted controls,
positive projection covariance, exact and randomized native routes, and old
plugin fallback before preparation/RNG. Its `full` profile adds 256 outcome
draws. Run it in native qualification and isolated installations. The match
component suite separately requires q0/q1 success for a one-firm worker with
twenty original blocks. Readiness bit 15 is additive; observation semantics,
worker-articulation pruning, rank/residual gates and ABI layouts stay fixed.

The additive native V6 execution boundary has dedicated `generic_execution`
FFI suites. Freeze the 304/328-byte request and 160-byte work receipt, validate
short buffers/headers/reserved fields, stale generations and request/solver
incompatibility, and prove old V5 flag-zero threads remain ignored. Compare
both deletion/nuisance modes, controls, explicit/automatic point widths and
1/2/3/4/7/14/28/64 threads. Test observation/joint and match/fixed-offset
structured inference with 513 Gram probes, unchanged literal seven/eight
attachment widths, strict phase gates and separate logical/refinement counts.
Projection checks preserve both successful coefficients/covariances and typed
PSD withholding; never select a different tolerance or hide failures to obtain
green results. Check exact/one-byte-short and omitted/advisory/off memory policy,
foreign callbacks confined to the caller, cancellation and reuse. Source-bound
native exports/builds must include V6. The new `test_rust_execution_paths.do`
must run in every supported Mac profile and isolated installation. It covers
72 point cells across both deletion/nuisance/population modes, 1/4/7 permitted
threads and automatic/seven/eight point batches; projection and both component
deletion modes; all 23 corrupt work fields; interruption/reuse; omitted and
explicit budget policy; and independent V3 signature vectors, including binary64
wall values. The legacy public generic suite independently rejects corrupt
capabilities before preparation and keeps its private scalar reference.
The separate stale-runtime test must reject before preparation/RNG for both
bit-11-only and FFI-only bit-12 binaries. The C transport's `execution_api=3`
is required independently of native readiness. Test absent metadata and a
cached false readiness scalar; a rejected call must have last_released=0.
Automatic inference-width intent and effective automatic-route coverage still
require implementation and qualification before the timing screen.

The subsequent internal automatic-width entrypoint has a distinct
`ComponentBatchPolicy` and borrowed prepared-data view. Its focused gate checks
literal eight versus automatic widths at 1/2/3/4/7/14/28/64 threads, independent
covariance/spectrum/Gram probe caps, the preserved legacy Gram cap, and complete
work-key multisets when wider batches change trace receipt storage order.
Unchanged-width comparisons retain their ordered check. Require identical RNG
accounting and target availability, corrected-result equivalence, every original
residual gate, q0/q1, both structured models, partial 513-Gram batches, explicit
point preservation, omitted/advisory/off/exact/intermediate budgets, pre-pool
failure, checked overflow, cancellation and reuse. Eight separate-process Q=32
heap cases cover both solvers/deletion modes at one/seven threads, separating
diagonal stack reservations from allocator payload. This source-only gate does
not qualify the still-pending native/public omission-intent handoff.

The isolated diagonal queue adds weighted degree-2--8 row-level normal-equation
oracles, zero RHSs, odd/partial work and 1/2/3/4/7/14/28/64 thread settings with
unchanged ordinary and strict phase tolerances. Check deterministic error order,
worker panic transport, caller-thread-only cancellation, parent-token handling,
successful reuse, exact-budget/one-byte-short boundaries, advisory/omitted
budgets, retained queue capacities and allocation overflow. A coordinated
four-worker test proves concurrency without requiring tiny numerical tasks to
overlap under test-suite contention. The 64-thread allocation is unit coverage,
not a 64-core performance claim. The separate tracking-allocator test bounds
incremental heap payload excluding stack reservations; it does not measure RSS
or admit the caller's prepared model. The subsequent internal estimator tests
compare both deletion modes, joint/fixed offsets, weights/controls, pooled
stayers and projection output with the existing diagonal path. They reconcile
pre-RNG fixed-k=2 widths, queue RHS counts, strict rank projections, explicit
widths, exact budgets, automatic budget reduction, omitted budgets and breaks.
A 32-control test requires strict preparation to use a four-RHS capacity, and
a separate-process tracking allocator covers the estimator lifetime in both
deletion modes at one/seven threads. V6 and the explicit public generic/diagonal
route now select the queue; historical existing-route-only native receipts do
not qualify this new dispatch.
The internal component attachment tests compare observation/joint and
match/fixed-offset inference, controls/targets and applicable frequency weights,
oracle and both structured variance models, q0/q1 results, computed versus
unavailable targets, covariance/coherent fourth target, strict per-RHS gates,
Counter counts and partial groups across all requested thread settings.
Direct residual tests include an odd 513-probe count, seven-column callbacks
split through capacity two/four, pre-pool exact-budget rejection, omitted and
advisory budgets, automatic batch reduction, cancellation and successful reuse.
A separate-process allocator checks complete inference heap at one/seven
permitted threads, with stack reservations reported separately. Direct-CMG
inference, native execution/caller-state checks and measured performance parity
remain prerequisites for public integration/paper timing. The direct-CMG
attachment continuation adds strict Q=0 and controlled oracle comparisons,
q0/q1 availability, both structured fits, partial Gram groups, joint automatic
budget planning and distinct solve accounting. Candidate-capacity forecasts must
be pure and agree with the realized workspace pool; Q=32 separate-process heap
tests force rank/Gram batches through capacity two. Weighted mixed-degree pooled
projection tests cover both deletion and nuisance modes. A deterministic signed
2-by-2 near-diagonal matrix regression requires small eigen-residuals before
and after final Ritz rotation; it protects the shared cancellation repair
without relaxing the component certification threshold. Existing public/native
routes require regression, but that does not establish native execution of the
new internal entrypoint or a performance/coverage claim.

Choose these gates by affected surface. Rust production, ABI, or native-boundary
changes require their applicable commands; documentation, tests, CI, packaging,
or evidence-workflow-only changes normally require only focused checks. Do not
run this whole list mechanically for every commit.

Run from a clean checkout:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

When Stata is available, also run:

```bash
./ci/run_stata_ci.sh quick
./ci/run_stata_ci.sh full
```

The integrated package command is:

```bash
./.venv/bin/python fevc/tools/run_checks.py
```

## Plugin qualification

A meaningful change to Rust estimator code, the plugin ABI, the Ado native
boundary, result posting, packaging, or native route tests requires:

```bash
./ci/run_ci_profile.sh plugin-build
```

The comprehensive macOS profile runs locked Rust and C gates, SPI
authentication, ABI/layout checks, thin and universal builds, binary audits,
native arm64 lifecycle and clean-install tests, and Rosetta x86_64 coverage
when available. The exact profile and source SHA must be present in
`.ci/stata/results/<sha>.json` with `status == "success"`.

This evidence qualifies only the recorded macOS candidate and routes. It does
not qualify native Intel hardware, Linux, Windows, production scale, or public
release.

## Focused scalable-projection gate

The sparse `project()` route is deliberately bounded to explicit Rust generic
JLA, Counter-V1, observation or match deletion, the registered mover/stayer
partition, positive integer frequency weights interpreted as literal physical copies, and explicit
diagonal PCG or forced generic CMG. Automatic projection routing remains
withheld. Its affected-surface gate requires:

1. pinned formatting, strict Clippy, workspace tests, C-shim transport tests,
   and ABI-header compilation;
2. an exact-boundary native memory test charging the C and Rust project-column
   copies, coefficient-space preparation, retained RHSs, solve peak, result,
   and caller export;
3. public Stata firm/frequency and worker/target comparisons against the
   independent exact Mata route for observation and match blocks, including a
   mixed eligible-stayer case, compressed-weight versus literal-
   expansion equality, forced-CMG versus common-draw diagonal equality,
   phase-6 RHS and complete-residual, PSD, lifecycle, and existing
   `e(projection_*)` schema checks;
4. the immutable 1,002-row inference fixture with deterministic coefficient
   and covariance gates plus KSS Matlab `lincom_KSS` standard-error
   gates; and
5. pre-RNG CMG memory failure and lifecycle restoration, plus a pre-native
   failure test proving that automatic solver routing is not admitted by the
   public projection tuple; and
6. focused 6,000-, 24,000-, and 96,000-row convergence probes before any
   KSS Matlab-relative or cross-platform performance claim.

These local gates do not substitute for a source-bound `plugin-build` release
profile or establish large-data performance. The next performance evidence is
the focused six-cell FEVC/KSS Matlab protocol in
the [archived scalable-projection protocol](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference), not a broad
SCC or platform matrix.

## Supported explicit matrix-free component-inference gate

For the individual-inference candidate, add
`test_rust_individual_inference.do` to native and isolated-install gates.
It covers both deletion modes and references at omitted/default 200 probes,
real fitter metadata, invalid-joint withholding, target-local availability,
malformed receipts and caller cleanup. The new observation geometry tests
cover outcome changes, batches, diagonal/CMG, memory admission and identical
design rows. The separately registered 64-call duplicate-ordering diagnostic
is run explicitly in release mode. These checks do not substitute for the
bounded saved-draw assessment registered in
`../fevc/docs/inference_completion_v1.json`. The current direct V4 route
also requires independent physical-row/collapsed-match moment identities,
redundant-basis and conditioning checks, independent-match support and Counter
accounting, and requested Gram solves (default 2,048) reconciled through native and Stata
receipts. Same-probe dense covariance, independent weighted collapse and
512/1,024/2,048 precision/count/point invariance checks are required.
V1–V3 augmentations retain their older policies; V4 exports retain
strict legacy semantics. Historical gate descriptions below refer to their
original interfaces, not qualification of the unified candidate.

The new explicit fixed-offset match interface also requires
`test_rust_match_component_inference.do` in the source-local native, integrated
and isolated-install profiles. It checks unchanged point results, physical
frequency-copy equivalence, target mass, declared match IDs, fixed controls,
diagonal/CMG, batch/order invariance, units and omitted-offset metadata,
pre-RNG tuple rejection, corrupted unit receipts and caller-state cleanup.
The additive native V1 unit receipt has size/capacity/signature tests; legacy
observation augmentation and statistical result V4 remain unchanged. These
are engineering gates, not a new Monte Carlo campaign or a waiver of the
corrected observation-q1 confirmation failure. Exact-source plugin-build is
required for the affected public boundary. Source
`53f22a109effee87467b4ef0602b21d0b8ec1ca9` passes those macOS arm64/Rosetta
and isolated-install gates; the current result and limitations are in
`../fevc/docs/FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md`.

The oracle-variance layer is internal; the structured-variance observation-
deletion attachment is a supported explicit public route, not a default. Its
focused source gate requires independent dense identities for `H^{-1}`, `P`,
the three `B_t` and zero-diagonal `C_t` kernels, combined influence vectors,
and the common one-solve Gaussian scalar; joint trace-covariance Monte Carlo
agreement within reported MCSE; bitwise unchanged point results; exact three-
to-four covariance coherence; inference-probe batch invariance; explicit
diagonal and CMG residual gates; typed rejection of every out-of-scope tuple;
cancellation; and the admitted-memory boundary. The spectral extension also
requires dense generalized-eigen and trace-square agreement, leading and
remainder concentration cases, numerical residual receipts, probe doubling,
and batch invariance. The `q=1` route additionally requires dense rank-one
remainder influence/probe/covariance identities, exact point decomposition,
critical-value determinism, and the confidence-ellipse image. Both references
report maximum linear-influence concentration. A bounded deterministic
heteroskedastic conditional experiment checks oracle-variance `q=0` coverage;
the fitted structured model requires a separate misspecification and coverage
gate.
Grouped-match work begins with two independent dense oracles: one constructs
the original physical-row match blocks and one constructs a single collapsed
scalar row per declared match. Under `nuisance(fixedoffset)` they must agree on
the FE fit, plug-ins, whole-match deleted residual contraction, point
correction, and zero-block-diagonal kernels. The first matrix-free `q=0` slice
then requires one Gaussian draw per independent match (never one per unit of
`F_g`), exact point-estimate invariance, structured match-variance fitting,
match-mass and influence diagnostics, spectrum/residual/PSD gates, and typed
failure for cross-coordinate or nonestimable matches. The supported
observation route retains the registered positive common regression,
leverage-only sensitivity, versioned plugin/Stata lifecycle, return, spectral,
misspecification, and typed-failure source gates.

## Performance and platform qualification

Select new runs using the affected-surface rules in
[`../fevc/TESTING.md`](../fevc/TESTING.md). Use independent oracles and
complete-command measurements. Freeze the source, input identities, semantic
RNG keys, repetitions, acceptance thresholds, and output inventory before a
confirmation campaign. Historical failures remain failures.

Use the real macOS/Linux/Windows installation paths and test the final binary
bytes, not only a build-tree library. Require licensed-Stata success markers,
clean native installation, ABI/lifecycle checks, and source/binary hashes.
Public workflows must not run on licensed self-hosted runners.

Completed milestone narratives and per-run receipts are outside the active
checkout; see [historical material](../docs/ARCHIVE.md). The complete previous
test plan, including the in-progress engineering notes, was also preserved in
the local cleanup archive's `previous-guides/rust/TEST_PLAN.md`.

## Hard acceptance contracts

Differential promotion uses the four corrected targets and the registered
`development_acceptance_v1.json` thresholds. Plug-in/correction path identity,
bitwise/ULP equality, equal iterations, and internal reduction order are
secondary diagnostics. They do not replace the hard contracts below and do
not block a statistically equivalent faster candidate.

All accepted native routes must preserve:

- the retained sample, target population, deletion unit, nuisance convention,
  frequency and target semantics, and four target columns;
- identification, control-basis, deletion-rank, reciprocal/maker, accounting,
  and finite-output gates;
- the complete original-system residual for every RHS at
  `max(1e-11,10*effective_phase_tolerance)`, using the receipted fit or probe
  tolerance as applicable;
- structural pre-RNG algorithm, engine, route, fallback, batch, memory, wall,
  and Counter receipts;
- optional budget presence and `warn`/`error`/`off` policy reconciliation;
  forecast rejection only for an explicit `memory_gib()` with
  `memorycheck(error)`, no memory-based planning when omitted, and exact
  preservation of explicit batches;
- exact request/selection/result-family reconciliation; and
- caller data, `e(sample)`, RNG algorithm/stream/state, sort state, release,
  and idle registry restoration on success, error, and UserBreak.

Point-only requests must not post `e(V)`. Explicit, capability-gated component
inference posts econometric covariance under its documented assumptions;
projection inference uses separate `e(projection_*)` returns. Probe dispersion
is numerical Monte Carlo error, never an econometric standard error.

## Deferred qualification

Windows Stata/plugin qualification, native Intel hardware qualification, a
command-surviving native cache, package distribution, tagging, and final
release mathematical/license/provenance approval remain deferred. The alpha
qualification packet includes the representative-scale RSS/performance,
rendered benchmark report, and exact-source macOS/SCC/supply-chain receipts.
