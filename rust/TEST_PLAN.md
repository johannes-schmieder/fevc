# Rust backend test plan

This is the active native test plan. Dated implementation and CI snapshots are
kept under [`progress/`](progress/) and must not be treated as current
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

## Routine source gates

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
   and covariance gates plus maintained-MATLAB `lincom_KSS` standard-error
   gates; and
5. pre-RNG CMG memory failure and lifecycle restoration, plus a pre-native
   failure test proving that automatic solver routing is not admitted by the
   public projection tuple; and
6. focused 6,000-, 24,000-, and 96,000-row convergence probes before any
   MATLAB-relative or cross-platform performance claim.

These local gates do not substitute for a source-bound `plugin-build` release
profile or establish large-data performance. The next performance evidence is
the focused six-cell FEVC/MATLAB protocol in
the [archived scalable-projection protocol](../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference), not a broad
SCC or platform matrix.

## Supported explicit matrix-free component-inference gate

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

The local q=0 foundation at source
`77177a6497891d8f6e1cab0aca89366f4e4ca4ad` passed those focused tests, the
complete workspace and standalone-backend gates, and exact-source macOS
arm64/Rosetta licensed-Stata qualification. Its sanitized packet is in
[`qualification/evidence/MATCH-FIXEDOFFSET-Q0-MACOS/`](qualification/evidence/MATCH-FIXEDOFFSET-Q0-MACOS/).
This is implementation/build evidence, not the registered grouped coverage or
misspecification campaign and not authority to start q=1.

The first grouped q0 campaign is registered in
`../fevc/docs/match_inference_q0_campaign_v1.json` and its pre-result
`../fevc/docs/match_inference_q0_campaign_v1_amendment1.json`. Before any development
profile, run its full tiny pipeline with the release example and the focused
adversarial tests, then submit only its one-task smoke profile through the SCC
build-task-aggregate launcher. Preflight must precede manifest creation; task
rows, receipts, semantic seeds, binary/source bindings, scheduler accounting,
and the exact output inventory must all reconcile. The local preflight binary
and SCC task binary are platform-specific: local aggregation requires the
preflight binary, while SCC aggregation requires the task binary to match its
exact-source Linux build receipt.

Both prerequisite smokes pass at exact implementation source `983ed37`. The
local path produced all 56 expected target-attempt rows; the exact-source SCC
build/task/aggregate chain produced all eight expected smoke rows with one
slot per stage, `failed=0`, and `exit_status=0`. The complete hashes,
inventories, and limitations are recorded in
`../fevc/docs/MATCH_INFERENCE_Q0_CAMPAIGN_SMOKE_2026-09-04.md`. The registered
amendment's complete local tiny path and replacement clean exact-source SCC
smoke also pass. The unchanged 400-replication-per-cell development profile
then completed all 280 tasks and 22,400 target attempts at source `c26a7ee`
with no registered gate failure. Its exact result and accounting are in
`../fevc/docs/match_inference_q0_campaign_v1_result.json`. This is accepted
development evidence, not independent confirmation or public promotion. A
separate grouped q1 derivation and tiny oracle suite may now begin; no q1
implementation or campaign is implied by the q0 result.

```bash
cargo build --release --locked --manifest-path rust/Cargo.toml \
  -p vckss-core --example match_inference_q0_development
./.venv/bin/python fevc/tools/run_match_inference_q0_campaign.py \
  run-preflight --profile tiny --root . --output-dir RUN/preflight \
  --binary rust/target/release/examples/match_inference_q0_development
./.venv/bin/python fevc/tools/run_match_inference_q0_campaign.py \
  create-manifest --profile tiny --root . --output RUN/manifest.json \
  --preflight-receipt RUN/preflight/receipt.json
./.venv/bin/python -m pytest -q \
  fevc/tests/python/test_match_inference_q0_campaign.py
```

The original fitted-model gate is executable as:

```bash
cargo run --release --manifest-path rust/Cargo.toml -p vckss-core \
  --example structured_inference_qualification --locked -- development > results.csv
./.venv/bin/python fevc/tools/validate_structured_inference_qualification.py \
  results.csv --profile development

cargo run --release --manifest-path rust/Cargo.toml -p vckss-core \
  --example structured_inference_qualification --locked -- confirmation > results.csv
./.venv/bin/python fevc/tools/validate_structured_inference_qualification.py \
  results.csv --profile confirmation
```

The confirmation profile is fixed at 2,500 replications per cell and dimensions
12 and 16. It fails closed on correct-model bias, coverage, standard-error
calibration, or execution rate; it separately bounds mild misspecification and
requires severe omitted-driver cells to reveal the model limitation. Spectral
fixture thresholds validate the designed diffuse, one-mode, and q>1 cases but
do not route public requests. The 2026-09-03 dirty-checkpoint confirmation
failed the predeclared coverage gate for the correctly specified q=1 firm
target under leverage heteroskedasticity (0.9348) and t8 errors (0.9336).
That transient output did not become source-bound qualification evidence; it
was superseded only through the separately registered V2--V5 sequence.

The registered V3 diagnosis retains the V2 factorized oracle and tests the
corrected raw leave-out q=1 recenter without restoring the moderate-dimension
dense-storage bottleneck. The qualification example represents
each observation-deletion kernel as a diagonal plus a coefficient-space
low-rank factor, evaluates its conditional covariance by an exact factorized
trace, and retains no observation-by-observation matrix. Its unit test compares
maker actions, full and rank-one-remainder kernel actions, traces, q=1
covariance terms, and interval endpoints with independently constructed dense
matrices at a scale-relative tolerance of `1e-9`.

The source-bound campaign interface is:

```bash
./.venv/bin/python fevc/tools/run_structured_inference_campaign.py \
  create-manifest --profile smoke --root . --output manifest.json
./.venv/bin/python fevc/tools/run_structured_inference_campaign.py \
  run-task manifest.json 1 task-output \
  --binary rust/target/release/examples/structured_inference_qualification
./.venv/bin/python fevc/tools/run_structured_inference_campaign.py \
  aggregate manifest.json task-output aggregate-output
```

`diagnostic` fixes dimensions 16, 24, 32, 48, and 64 and 1,000 replications
per cell/dimension; `confirmation` fixes dimensions 32, 48, and 64 and 5,000
replications. Confirmation manifests require a clean committed source. The
manifest binds the source tree, every task owns a disjoint semantic replication
range, task and aggregate writes are atomic and non-overwriting, and aggregation
validates the complete inventory and every receipt hash before applying the
registered gates. `rust/stata_backend/scc/deploy_structured_inference_campaign.sh`
and `submit_structured_inference_campaign.sh` deploy an immutable source archive
and submit build, array, and aggregation jobs with scheduler dependencies. The
wrappers explicitly load SCC `python3/3.13.8`; they do not rely on the login
node's older default interpreter, and task/aggregate receipts record the actual
runtime. A
1--4-core real-entrypoint smoke must pass before either registered campaign.
These results are assumption-conditional evidence for the named structured
models, never unrestricted-heteroskedastic KSS evidence.

V3 requires the leading square to use
`sum_i v_i^2 y_i e_(i,-i)` while the positive structured variance vector is
used only for the joint covariance. Each result records the direct-remainder
identity error and the legacy modeled-variance recenter only as a non-gating
coverage diagnostic. Qualification critical values use deterministic
Gauss--Legendre inversion; focused tests compare that rule with Simpson
integration, production Counter-V1 quantiles with direct integration, and the
ellipse image with a dense angular oracle. The public attachment uses at least
100,000 critical draws.

The V2 development run at source `37f9798` completed all 50 tasks and 20,000
rows with zero scheduler or process failures. It failed only
`dominant_common_t8/64/oracle: coverage`: 0.972 with MCSE 0.0052. The fitted
path covered 0.964. Maximum leading-mode and remainder-influence shares both
declined from dimensions 32 through 64, so the registered classification is
`q1_reference_or_remainder_problem`. The V3 source corrects the discovered
center/covariance mismatch and was registered before new moderate-dimension
coverage was inspected. The clean source-bound V3 campaign at `7b92cf1`
likewise completed all 50 tasks and 20,000 rows with zero process failures.
The direct-remainder identity held, but the same oracle t8 firm cell at
dimension 64 covered 0.972 and failed the frozen gate. Its leading and
remainder variance ratios were 0.982 and 0.989, so the registered
classification remains a q=1 reference/remainder problem rather than a
structured-smoother failure. At that historical checkpoint confirmation
remained unauthorized. Exact V2 and
V3 development receipts and hashes are indexed by
`../fevc/docs/structured_inference_diagnostic_v2_result.json` and
`../fevc/docs/structured_inference_diagnostic_v3_result.json`.

V4 subsequently established that the remaining modest overcoverage was the
expected behavior of the KSS/Andrews--Mikusheva maximal-curvature,
at-least-nominal uniform construction, not a production statistical defect.
The clean preregistered V5 confirmation at source `8e3596a` completed 200 SCC
tasks and 200,000 target-replication attempts with every frozen scientific,
spectral, schema, execution, and inventory gate passing. Supported `q=0`
therefore still requires diffuse target and influence contributions; supported
`q=1` removes one leading mode and requires a diffuse remainder. The
deliberately multi-mode covariance target remains outside the `q=1` coverage
claim. The separate public-boundary promotion source
`7608942a09c643fcb78fb52885d3b87333c5429f` passed exact-source macOS
arm64/Rosetta native/plugin and licensed-Stata qualification. The recorded
compatibility review confirms that its statistical core, ABI, RNG, solvers,
build, DGPs, and acceptance rules are unchanged, so no new Monte Carlo
campaign was required.

## Completed focused milestone: planned auto-exact

Commit `6954da6e190680a65ac271b71a33ece8d0fcfab1` closes these gates locally:

1. Direct V4 exact result export reconciles through
   `_fevc_rust_reconcile_exact_v7` with the exact limit, algorithm/engine
   selection reasons, plan complexity, memory, residual, accounting, and zero
   pre-RNG counter facts intact.
2. The public Rust option predicate admits only the intended planned
   `algorithm(auto)` tuple and preserves explicit backend/RNG consent.
3. `_fevc_rust_generic_planned` recognizes the exact result family before its
   compressed/generic switch and constructs the exact preparation, graph, and
   capability contexts.
4. `_fevc_rust_post_exact_v7` posts exact results, V7 plan fields, and
   `e(sample)` and releases the native handle exactly once.
5. A public command test covers
   `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)` selecting exact.
6. The test verifies requested/selected algorithm and engine, result family,
   exact identities, no estimator RNG consumption, caller RNG/data/sort
   restoration, clean install, and typed corrupt/inconsistent receipt failure.
7. Quick, full, and plugin-build profiles pass at the exact final source SHA.

The focused direct test retains `EXACT_V7_RECONCILE_FAIL` and the returned
receipt list on any future reconciliation failure. The clean source-local
`plugin-build` receipt is green for thin arm64, thin x86_64/Rosetta, and
universal execution under both architectures, including clean installs.

## Completed foundation: exact stayer augmentation

Commit `c199bf017719f09f00fbe7e476e93448747d8a83` closes the native exact
`stayers(both)` gap on macOS:

1. A versioned native augmentation lifecycle constructs the frozen
   mixed-deletion stayer correction. The current command promotes that
   combined result to the headline; the historical exact receipt established
   the augmentation and source-split foundation.
2. The public exact poster reconciles preparation, graph, capability,
   augmentation, result, accounting, memory, and V7 plan receipts before
   posting.
3. Independent dense and Mata differential oracles cover pooled targets,
   physical frequency splits, controls, weights, deletion IDs, and exact
   zero-RNG behavior.
4. Success, typed failure, corrupt receipt, resource failure, and UserBreak
   retain exactly-once release and idle-registry restoration.
5. The exact-SHA macOS `plugin-build` passes the public hybrid suite for thin
   and universal arm64 and x86-64/Rosetta candidates, plus clean installation.

## Qualified production full-CMG gate

The normal build owns vendored CMG commit `92a12f2` and identifies the scalar
direct hybrid route as `CMG_FULL_V2`. The completed explicit-route gate
requires:

1. strict Clippy, workspace and standalone tests run under pinned Rust 1.85.1,
   including the vendored CMG parallel feature tests;
2. checked memory overflow, exact-boundary admission, actual-retained
   reconciliation, and pre-RNG resource rejection are typed and receipted;
3. residual failure refines only failing columns on the frozen same-route
   tolerance ladder, with no estimator fallback;
4. the caller-thread coordinator polls Stata while hierarchy, plan, PCG,
   V-cycle, batch, extraction, recovery, and certification workers observe only
   the shared atomic cancellation token;
5. successful, cancelled, failed, panicked, stale, and corrupt generations
   retain exactly-once/idempotent release and return the registry to idle; and
6. an ordinary source-local and clean-install Stata request selects
   `CMG_FULL_V2`, reconciles the 46-field receipt, restores caller data/RNG/sort
   state, and requires no private environment or standalone checkout.

The production preparation boundary is additive V4. Focused tests freeze its
64-byte request and 88-byte interrupt-request layouts, admit the exact memory
forecast but reject one byte less, and reject implicit-match requests without
match deletion, without explicit probe order, or with controls. The caller
passes this bit only for the registered full-CMG cell. Runtime source
`4b6874e` passes the explicit macOS and SCC numerical, memory, lifecycle,
clean-install, and performance gates. Commit `61dba32` therefore admits the identical effective
cell through `backend(auto) rng(auto)` on qualified macOS and Linux builds.
Focused tests require truthful requested/selected metadata, explicit Mata
availability, fail-closed post-selection resource errors, and pre-preparation
fallback when the runtime is genuinely missing. Unsupported tuples retain
their existing routes and cannot claim
`e(cmg_backend) == "CMG_FULL_V2"`. Windows remains unclaimed.

## Active comparative-scaling evidence gate

The source-bound harness in
[archived comparative-scaling harness](../docs/history/VCKSS_ARCHIVE.md#performance-records)
tests the already-qualified public route without changing it. Local tests
freeze the 300-task manifest, graph/input hashes, strict Rust and Mata request
strings, omitted `tolerance()`, CPU-affinity selection, dynamic MATLAB worker
monitoring, estimator-phase memory markers, timeout/failure preservation,
independent-probe MCSE gate, maintained MATLAB PCG convergence/rejection,
FEVC and full-CMG phase timers, estimator/full-process RSS, 900-row
aggregation, and a headless vector-report build. The report builder has passed
a warning-free synthetic cardinality/layout exercise; real claims remain
blocked on accepted small/worst pilots and the complete 300-task collection.

SCC acceptance additionally requires exact source and plugin/MEX hashes,
`qacct failed=0` and `exit_status=0`, validated wrapper/node receipts, and all
successful-call route, residual, accounting, `e(sample)`, caller-state, memory,
and lifecycle checks. Prototype jobs may use any eligible SCC host rather than
waiting for a fixed queue, CPU model, or exclusive node. All three routes run
sequentially within each task on the same receipted host, with rotated order;
paired within-task ratios are primary. Absolute timing and cross-core scaling
are stratified by CPU identity or treated as descriptive until a smaller
homogeneous-host confirmation. Memory limits must fit the scheduler request but
are safety envelopes rather than a fixed development target. The small and
worst-case pilots precede the 300-task production array. A cell is rankable
only when every route passes in all three repetitions; failures and timeouts
remain explicit results.

## Private alpha qualification

The generated feature ledger is
[`../fevc/docs/RUST_MATA_PARITY.md`](../fevc/docs/RUST_MATA_PARITY.md).
Every alpha-required row must be `qualified` on its claimed platform before
the candidate is called complete. This milestone deliberately creates no tag
or public prerelease.

### Historical M5 direct full-CMG decision spike

Commit `98a486c` introduces `VckssEnginePerformanceReceiptV1` as an additive
96-byte C ABI surface. It does not extend or replace the frozen V7
science/execution-plan receipt. Rust and C compile-time layout fences require
the exact size and timing offset, while FFI tests reconcile generation,
selected algorithm/engine, applicability, and the saturating phase sum for
exact, compressed, and generic planned solves.

The Stata result bridge validates the receipt before posting and exposes
`e(rust_phase_profile)` with schema `VCKSS-NATIVE-PHASE-PERF-V1` in seconds.
Focused planned V4, public compressed, and public auto-exact tests require
nonnegative phases, a reconciled native total, caller-state restoration, and
an idle registry. A clean exact-SHA `plugin-build` remains required before the
instrumentation is qualified; timings are diagnostics, never routing inputs.

The historical private lane added `CMG_FULL_SPIKE_V1` without changing the
public ABI or advertised capability. Its archived source now lives outside the
normal workspaces under `rust/experiments/full_cmg_spike/`. Focused Rust tests
required fixed-order batch
results, one shared solver/plan/workspace pool, bounded four-thread execution,
no Stata calls from Rayon workers, independent complete-system residuals, and
repeated corrected results within the registered equivalence rule. The
registered local 8,192-firm/200-probe case passes
state and residual gates and is 12.35% faster end-to-end than source `4124b34`.
Its covariance and corrected covariance differ by about `2.22e-12`, which
failed the legacy `2e-12` comparison but easily passes the active development-
equivalence policy. An inner `1e-15` diagnostic produces the same result gap at
higher runtime, so the difference is retained as a nonblocking diagnosis.

Accepted same-host SCC job `7306628` is the original decision gate. The synthetic
headline takes 329.260 seconds for A, 223.232 seconds for C, and 171.733
seconds for maintained MATLAB R2025b. C is 32.20% faster than A but 29.99%
slower than MATLAB. Its two covariance fields fail the historical receipt's
fixed A/C threshold, but under the active policy it is
statistically equivalent; the original route was rejected because it was
slower than MATLAB. Preserve the local and SCC receipts.

The owner-authorized renewed direct-route wave is source-bound at `598a08d`.
Packed/ordered-parallel Counter-V1 generation, direct leverage RHSs,
independent target preparation, parallel moment accumulation, and parallel
recovery/certification reduce one clean macOS headline to 81.145 seconds,
0.473 times the registered MATLAB comparator. The corrected targets remain
bit-identical to the preceding source-bound checkpoint; the maximum complete
residual is `6.97e-6` under the `1e-5` probe gate. This crosses the 2x target
only as a single development run. Do not harden or expose the route until one
cold plus five alternating warm A/C/MATLAB runs and the fixed CZ18 P200 gate
both pass. SCC synthetic job `7311964` measures 156.455 seconds versus
MATLAB's 260.928 seconds. Accepted same-host job `7312041` keeps direct at
151.351 seconds and disables fused f64 after its 239.610-second regression.
Direct is 1.669x MATLAB and misses the 2x threshold by 25.079 seconds; the
official full-CMG repeated solve is the dominant cost. Fixed-CZ18 P20
job `7314745` now passes all 61 numerical solves, result reconciliation and
posting, state restoration, scheduler accounting, and the pinned validator at
maximum complete residual `7.993e-6`. It remains smoke only: candidate command
time is 91.596 seconds versus MATLAB's 47.154 seconds. The scalar pass-fusion
lane at `08565be` passes deterministic CMG tests and unchanged scientific
gates, but its favorable clean local observation is only about 2% faster and
private admitted memory is 2.6% higher. Preserve it behind explicit private
consent and keep it disabled; no SCC matrix is justified. The first CZ18 P200
attempt, job `7314843` at `c1ae402`, correctly fails the unchanged `1e-5`
complete residual gate on target probe 56 (`1.563e-5`) before MATLAB. The next
attempt pre-registers private inner tolerance `1e-9` for P200 while P20 remains
`1e-8`; effective probe tolerance stays `1e-6`. It must pass wrapper,
application, qacct, and the pinned validator before any interpretation. It is still a
single-run decision, not qualification. Do not return to the simplified
hierarchy.

Fixed-CZ18 P200 SCC job `7317771` at source `3daa465` passes one cold and five
position-balanced warm A/C/MATLAB rounds. Warm medians are 253.771 seconds for
A, 33.942 seconds for C, and 70.147118 seconds for maintained MATLAB R2025b;
C is 2.0667x MATLAB. Candidate median peak RSS is 2,530,940 KiB versus
MATLAB's 4,310,024 KiB. Complete residual, common-probe corrected-target,
repeatability, application/state, MATLAB process-tree, wrapper, and qacct
gates all pass. The pinned validator initially stopped on blank optional
legacy phase diagnostics; commit `5a6daaf` changes only the parser and adds a
self-hash, then passes against the unchanged job evidence. Record this as
`CZ18_P200_WARM_MEDIAN_TWO_X_CHECKPOINT_ONLY`; it did not authorize hardening
without the subsequent synthetic decision.

Synthetic P200 SCC job `7318114` at source `787327f` passes the complete six-
round application, common-probe corrected-target, complete-residual,
repeatability, caller-state, MATLAB process-tree, wrapper, and qacct suite.
Warm medians are 490.209 seconds for A, 123.633 seconds for C, and 183.017703
seconds for MATLAB. Record this as
`SYNTHETIC_P200_WARM_MEDIAN_NOT_PROMOTED`: C/MATLAB is `0.6755`, not `<=0.5`,
and candidate maximum process RSS is 4,134,164 KiB versus MATLAB's 3,847,076
KiB. The official repeated solve is 98.664913 seconds across 601 RHSs and must
fall materially before another full matrix. Preserve the full run and compact
decision and, at that source, kept the route private. The later scalar
production wave and runtime source `4b6874e` supersede that decision while
retaining this history and its receipts.

The required new suites cover:

1. Rust-preferred omitted/automatic backend routing and `rng(auto)` with only
   missing-runtime or structurally unsupported preflight fallback to Mata.
2. Strict explicit backend/RNG mismatches and fail-closed stale, corrupt,
   preparation, memory, convergence, numerical, resource, and UserBreak exits.
3. Keep the qualified effective-option admission regression for automatic
   exact/compressed/generic result families, controls, deletion modes, weights,
   targets, deletion IDs, `probeorder()`, route, batch, and wall advisories.
4. Keep exact and generic-JLA `stayers(both)` augmentation regressions with
   independent dense/Mata oracles, combined-primary and zero-stayer behavior,
   pooled target accounting, memory admission, and lifecycle stress.
5. Linux x86-64 clean installation and real Stata MP 19 execution on SCC,
   bound to the exact source SHA and successful SGE accounting.
6. Miri, malformed-ABI fuzzing, C-shim sanitizers, dependency/license/SBOM,
   and registered performance/accuracy gates.

### Completed bounded Miri and C-shim sanitizer gate

Source `041a89e9fbaa0b4893564b3ca8984bc701e2003b` has a passing Darwin
arm64 receipt under
[`qualification/evidence/M4-SAFETY/`](qualification/evidence/M4-SAFETY/).
The registered gate uses the date-pinned nightly toolchain, strict-provenance
Miri for bounded unsafe FFI/registry/lifecycle cases, and Clang
AddressSanitizer plus UndefinedBehaviorSanitizer for the C shim. macOS leak
detection is explicitly disabled because that sanitizer is unsupported on the
platform; AddressSanitizer and UndefinedBehaviorSanitizer remain enabled.

Miri does not replace native numerical evidence. The full numerical suite
showed a software-interpreter one-ULP floating-point difference, and the dense
exact-stayer spectral solve is impractically slow under interpretation. Those
paths retain native Rust corrected-result equivalence and licensed-Stata
lifecycle gates; the one-ULP difference is diagnostic under the active policy.
The same directory contains a passing bounded malformed-ABI fuzz
receipt for source `a091b37123c16697750af5578c905b7ff172b710`. The
locked `cargo-fuzz 0.12.0` target uses the same date-pinned nightly, runs
nightly Clippy first, preserves only the registered seed corpus, and requires
zero crash/timeout/leak artifacts. Its V1/V2/V3 capability inputs cover
malformed headers and sizes, unknown semantic tuples, null pointers, and
adversarial output capacities.

### Completed SCC Linux qualification

Source `86e0711b1b1d07cff3058461c558838f2233f1d1` has a passing Linux
x86-64 receipt under
[`qualification/evidence/M4-LINUX-SCC/`](qualification/evidence/M4-LINUX-SCC/).
SGE job `7305794` built the ELF candidate from a deterministic source bundle,
ran the locked Rust/C/ABI gates, authenticated Stata MP 19, passed the full
public Stata suite, and repeated the public Rust route from an isolated clean
installation. The candidate, bundle, staged-source, Stata binary, environment,
and required exports are hash/receipt bound; `qacct` records `failed=0` and
`exit_status=0`. This is Linux x86-64 evidence, not representative-scale
benchmarking or a Windows/macOS claim.

### Completed dependency, license-inventory, and SBOM gate

Source `2888b838ac744ec72c90373ff54402ab025d7e9b` has a passing automated
supply-chain receipt under
[`qualification/evidence/M4-SUPPLY-CHAIN/`](qualification/evidence/M4-SUPPLY-CHAIN/).
The source-clean gate pins `cargo-audit 0.22.2` and `cargo-cyclonedx 0.5.9`,
audits the workspace, Stata-backend, and fuzz lockfiles against one RustSec
snapshot, rejects vulnerabilities and warnings, and emits deterministic
CycloneDX 1.5 SBOMs. The normalized evidence requires registry checksums,
Cargo package URLs, a reviewed license allowlist, no unregistered git
dependencies, and no machine-local paths. The registered
`MIT/Apache-2.0 -> MIT OR Apache-2.0` bridge is limited to the spelling
published by `version_check 0.9.5`. Final human license/provenance approval is
still required before any public release.

### Completed alpha macOS and supply-chain gates

macOS and supply-chain receipt source
`4dafec6734af4b8d3c25785f268f19f69f780684`
has passing exact-source evidence under
[`qualification/evidence/M5-FULL-CMG-MACOS/`](qualification/evidence/M5-FULL-CMG-MACOS/)
and
[`qualification/evidence/M5-SUPPLY-CHAIN/`](qualification/evidence/M5-SUPPLY-CHAIN/).
The ordinary plugin build passes thin arm64 and x86-64, universal execution,
Rosetta, clean installation, ABI, cancellation/lifecycle, explicit full-CMG,
and eligible automatic routing. The SCC qualifier links its Linux
error-transport fixture with `libm` and invokes the pinned `cargo-fmt`,
`rustfmt`, and `cargo-clippy` binaries directly. The source-clean supply-chain
gate reports zero RustSec vulnerabilities and warnings, four deterministic
CycloneDX 1.5 SBOMs, and 53 registered components. These receipts qualify
only their exact source and do not replace final human license/provenance
review.

### Completed alpha SCC Linux gate

SCC source `992eba0947ca155c534f532750fc202e41ecf978`, deterministic
bundle `633758a3e784c4c48d35d421cf4627d915e63ad1db4e59f4116c34e3ad5d348e`,
and job `7330577` have passing exact-source evidence under
[`qualification/evidence/M5-FULL-CMG-LINUX-SCC/`](qualification/evidence/M5-FULL-CMG-LINUX-SCC/).
The ordinary x86-64 plugin passes the pinned Rust 1.85.1 format, strict
Clippy, Rust, C-shim, ABI, Stata/MP 19 full-suite, public-route, clean-install,
caller-state, cancellation/lifecycle, explicit full-CMG, and eligible
automatic-routing gates. The wrapper and qualifier receipts bind the source,
bundle, source manifest, candidate hash, platform, and commands; scheduler
accounting records `failed=0` and `exit_status=0`. Job `7330293` remains
preserved as a rejected infrastructure-only attempt whose direct
`cargo-clippy` invocation omitted the literal `clippy` subcommand. The compact
accepted packet excludes the native binary and raw or licensed Stata logs.

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
- direct allocation within `memory_gib()`;
- exact request/selection/result-family reconciliation; and
- caller data, `e(sample)`, RNG algorithm/stream/state, sort state, release,
  and idle registry restoration on success, error, and UserBreak.

Point estimates only are implemented. Probe dispersion is numerical Monte
Carlo error, not an econometric standard error, and `e(V)` must not be posted.

## Deferred qualification

Windows Stata/plugin qualification, native Intel hardware qualification, a
command-surviving native cache, package distribution, tagging, and final
release mathematical/license/provenance approval remain deferred. The alpha
qualification packet includes the representative-scale RSS/performance,
rendered benchmark report, and exact-source macOS/SCC/supply-chain receipts.
