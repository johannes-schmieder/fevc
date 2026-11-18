# Testing and qualification

This file is the active package testing guide. Dated performance reports and
Rust checkpoints are evidence, not instructions. Start with
[`PLAN.md`](PLAN.md) for the current milestone and
[`docs/README.md`](docs/README.md) for the documentation map.

## Gate selection and evidence reuse

Select the minimum sufficient gates from the behavior a change can affect.
Do not treat a new commit SHA as an automatic reason to rerun every test, a
full native/platform profile, or a large SCC job. Documentation, tests, CI,
packaging, provenance, and evidence-workflow changes normally need focused
static or unit checks. A production, ABI, build, dependency, input-generator,
timing, resource-measurement, or acceptance-semantics change needs the relevant
integration or measurement gate, escalating through a small pilot before a
large run when possible.

Existing exact-source evidence may support a later source when a compatibility
review records the tested and current identities, changed paths, affected
surface, unchanged relevant production/build/binary/input/acceptance identities,
focused checks, reused claims, and limitations. Historical receipts remain
immutable. Large benchmarks and broad qualification runs require a material
affected-surface reason, an active benchmarking or release need, risk that
focused checks cannot bound, or an explicit owner request.

## Source gates

Use the repository interpreter:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

The Python gate covers package, CMG, packaging, retained comparator, and static
contract tests. Generated CMG source must remain drift-free.
Documentation-inventory tests use recorded expectations. Frozen-evidence
audits still read their exact historical commits: shallow clones explicitly
skip only unavailable history. Set `FEVC_REQUIRE_HISTORY=1` to require those
audits; full-history source CI does so. Missing evidence in available commits
always fails.

The integrated release-hardening checks are also individually available:

```bash
./.venv/bin/python fevc/tools/check_fevc_public_identity.py
./.venv/bin/python fevc/tools/license_audit.py
./.venv/bin/python fevc/tools/render_rust_mata_parity.py --check
./.venv/bin/python fevc/tools/build_release_artifact.py --check
```

The artifact check is read-only and deterministic. To construct the portable
source artifact from a clean committed checkout without publishing it, run:

```bash
./.venv/bin/python fevc/tools/build_release_artifact.py \
  --output-dir /private/tmp/fevc-release-artifact
```

The archive contains only `stata.toc`, `fevc.pkg`, and the manifest-listed
portable runtime, help, license, and notice files under one `fevc/` directory.
Its adjacent JSON receipt records the exact source commit and SHA-256 inventory.

For Rust source changes also run:

```bash
rustup run 1.85.1 cargo fmt --manifest-path rust/Cargo.toml --all -- --check
rustup run 1.85.1 cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
rustup run 1.85.1 cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
rustup run 1.85.1 cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

## Licensed Stata profiles

Licensed Stata gates are local-only or run in explicitly private
infrastructure. Public GitHub Actions must not target self-hosted licensed
runners, store licensed logs, or write qualification receipts back to the
repository.

From the repository root:

```bash
./ci/run_stata_ci.sh quick
./ci/run_stata_ci.sh full
```

The integrated local command is:

```bash
./.venv/bin/python fevc/tools/run_checks.py
```

A successful suite prints its explicit terminal PASS marker. Stata 18 batch
launchers can return shell status zero after a do-file error, so the marker and
machine-readable `stata_rc` are authoritative.

The integrated checker also tests helper migration from a legacy underscore
installation: uninstall the registered old package, install the renamed
package through ordinary and SSC-shaped source layouts, verify all helper
locations and obsolete-file removal, and execute both deletion modes. These
fixtures use temporary PLUS directories and do not modify the user's installation.

The quick suite is the ordinary source gate. The full suite is required for a
substantial package or numerical change. Both include routing, sample,
restoration, result-equivalence, and typed-failure coverage appropriate to
their profile.

`tests/stata/test_pooled_deletion.do` checks original deletion-unit mover
classification against literal block-deletion QR fits, including one-firm
multi-block workers, physical copies, target weights, controls, both populations
and nuisance modes, and positive/indefinite projection covariances. Current
plugins run the native exact and JLA routes; older plugins must fall back or
withhold before preparation. The full profile also checks 256 fixed-seed outcome
draws with every attempt counted. The native match-component test separately
requires q0/q1 success for the new mover case. Rust's `deletion_unit_graph`
test independently enumerates 728 small multigraphs and checks the fixed point.

`tests/stata/test_subsample_equivalence.do` compares `if`/`in` fits with
physically compact selected datasets in both backends and exact/JLA routes.
It checks retained row identities, sample counts and target mass, results and
numerical MCSE, data/order/RNG restoration, and idle native state. Fixtures
include controls, frequency and target weights, pooled deletion IDs, excluded
movers, missing excluded inputs, eligible stayers, and graph exclusions. It
also preserves the typed failure for missing inputs inside requested matches.
The test runs in both source profiles and qualified installed-package checks.

## Stata 15 compatibility

The exact Mata leading-direction calculation scales each symmetric target to
unit magnitude for the eigensolve and rescales its eigenvalues. This avoids
missing eigensystem results observed on Stata 15 with finite, rank-deficient
targets. For a repeated leading eigenvalue, a coordinate projection into its
eigenspace selects a reproducible direction instead of the solver's arbitrary
basis. A closed-form spectrum checks the direction and eigenvalue at scales
1e-12, 1 and 1e12; the independent dense oracle and literal frequency-expansion
regression retain their original tolerances.

Native RHS diagnostic tables are allocated through Mata on all runtimes.
Stata 15's `st_matrix()` export ignores `matsize`; its diagnostic row ceiling
is kept at 65,534 for MP, or the edition's `max_matsize` otherwise. Projection
expressions still respect the caller's legacy `matsize`. The unchanged
4,096-probe literal-deletion oracle tests receipt transport beyond 11,000 rows.

The Stata 15 compatibility regression also checks all four native diagnostic
row formats against literal expected rows, including a table longer than
11,000 rows. The pooled-deletion oracle retains its 4,096/8,192-probe settings
and checks complete native/public tables across the legacy subscript boundary.

Use the installed older executable with its installation root explicitly set:

```bash
SYSDIR_STATA="/Applications/Stata 15/" \
STATA_BIN="/Applications/Stata 15/StataMP.app/Contents/MacOS/stata-mp" \
./ci/run_stata_ci.sh full
```

Run the default-inference regression separately without the point-only suite
switch. `test_stata_compatibility.do` checks sort-state replay and preservation
of the caller's returned results, the runtime-specific RNG identity and native
matrix-dimension admission. The existing simulation, RNG, lifecycle, restoration,
inference and installed-package tests must also pass. On this Apple Silicon
Mac, Stata 15.1 executes the Intel plugin under Rosetta. Stata 16 is not installed;
the installed Stata 17 license is expired. Neither version has runtime-test
evidence from this compatibility work.

## Focused outcome-centering checks

The small implementation is covered by `test_centering_mean.do` (fixed-mean
point/MCSE/covariance oracle), `test_centering_exact.do` (independent dense
estimated-mean oracle), `test_centering_jla.do` (Corrected/Mean MCSE
agreement), `test_centering_jla_map.do` (independent three-pool map) and
`test_centering_options.do` (default Mean equivalence for point/MCSE/covariance
and sample, explicit modes, restrictions, RNG/caller-state guards).
`test_projection_mean.do` independently refits deleted samples in 48 exact
projection cells, covering controls, both deletion/population modes, both
effect dimensions, frequency/target projection weights, Mean/None, shifts
and literal-copy expansion. `test_projection_mean_native.do` checks the
explicit JLA projection attachment with the new native capability, including
diagonal/CMG agreement and the same mean convention. These point/projection
checks do not qualify the standard errors of the components below. Corrected
remains unsupported for projections; the standard errors of the components
accept all three modes and are checked by `test_repeat_measurement_se.do`.
The map oracle is Mata-specific; the other files accept package root and
backend arguments. They are part of the ordinary Stata runner.

For focused checks, pass the absolute source package path and `mata` or
`rust` to the applicable file, for example:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do \
  fevc/tests/stata/test_centering_mean.do /absolute/path/to/fevc/fevc mata
```

Strict Rust tests require the matching local centering API 1 plugin, not
the unchanged distributed binaries. The new Rust unit test checks the three-pool correction map against an
independent dense construction. Shared exact systems and public mode/MCSE
behavior are checked by the Stata oracles above. Existing native lifecycle,
resource and ABI regression suites remain part of the completion checks;
they are not new centering-specific unit coverage.

The local completion record includes both-backend focused tests, dense
oracles, explicit-CMG cases, the full Rust workspace and integrated source/
Stata/install checks. Small complete-command timings compare all three
centering modes for exact/JLA and MCSE off/all. Preserve the first run and
quiet repeat; do not reinterpret them as platform or representative-scale
qualification. [CENTERING.md](docs/CENTERING.md) records the exact scope.
Documentation-only updates reuse these scientific/runtime results with
focused source and documentation checks under the evidence rules above.

## Standard errors of the components

Since 0.6.0 the command reports repeat-measurement standard errors by
default ([INFERENCE.md](docs/INFERENCE.md)). The registered Stata suites
were written for the earlier point-only default and keep it through
`FEVC_DEFAULT_INFERENCE=none`; the files below name `inference()` or check
the default explicitly.

- `test_repeat_measurement_se.do` compares the Mata routes, and the native
  route when the loaded plugin carries the method, with an independent
  dense calculation under both deletion modes, with controls, the
  mover-only population and every centering mode; it also checks the
  leading directions and the interval of every component, a variance
  estimate that is not positive, and the messages for the removed models.
- `test_default_inference.do` checks the command's own default, natively
  and in Mata, and restores the suite setting.
- `test_help_examples.do` runs the help's examples through `fevc_run`,
  including Examples 6 (`inference(highrank)`: normal intervals, `e(V)` and
  the leading directions' shares) and 7 (`inference(q1)` on the bottleneck
  of `ex4`: `q = 1` intervals and no `e(V)`), and 8 (the default with
  `estat decomposition, full`, `estat diagnostics` and `estat inference`),
  and checks what the help says the default reports on the same data.
- `test_inference_display.do` checks the display on `ex4` on Mata JLA,
  Mata exact and, with a capable plugin, natively: the same table in the
  three modes (columns, no p-values, the interval of every row and its lead
  share against `e(inference_intervals)`, the heading's rule or request,
  the lines under the table), every line within 80 columns, the stored
  results of `inference(highrank)` (normal intervals, the search, and every
  result but the regime equal to that of `inference(auto)`), the numerical
  MCSE line with its ratio to the standard errors and its warning (forced
  with `probes(8)`), `estat inference` and `estat diagnostics`, and that
  the default output shows no live progress while `verbose` does. It also
  forces the search to stop under `inference(highrank)` (a native failure
  of the attachment that carries it, injected through
  `fevc__rust_public_call`, and a Mata decomposition whose function is
  replaced in memory) and checks that the rerun without the search keeps
  the standard errors, `e(V)`, estimates and status, records status 7 and
  shows no shares; that an unresolved direction shows no share under
  `inference(highrank)` but keeps its share under the default; that
  `inference(q1)` stays strict; and that a variance estimate that is not
  positive still stops `inference(highrank)` on Mata.

The Monte Carlo harness of the interval rule is `tools/default_interval/`;
its run records are in `docs/default_interval/`. Run a focused file with the
package root, for example:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do \
  fevc/tests/stata/test_repeat_measurement_se.do /absolute/path/to/fevc/fevc
```

## Mean component inference checks (models removed in 0.6.0)

This section describes the checks of the fixed-observed-mean component
models of 0.5.2, which 0.6.0 removed from the command, and records their
evidence. The Stata files it names now drive the repeat-measurement model.

`test_component_centering_exact.do` and
`test_component_centering_native.do` cover the public exact and structured
routes, respectively. Core `generic_jla/component_centering.rs` checks the
centered native execution path alongside FFI lifecycle/transport tests.

Mean component inference extends every existing supported highrank/q1 tuple
with the observed retained physical-frequency working-outcome mean fixed.
Run independent exact observation and physical-block/collapsed-match oracles
for the centered point, influence, primitive cross-covariances, q1 raw
recenter, leading/remainder covariance and remainder identity. The exact
Mata oracle must retain target-specific smoothing and polarization; native
oracles must separately cover both residual-moment variance models. Do not
center inference error probes by either the observed mean or their own means.

Check default/explicit Mean equivalence; Mean versus None on externally
shifted outcomes; original None regressions; outcome translation and unit
rescaling; literal-copy match frequency weights and target weights; controls,
pooled deletion IDs, coherent fourth-target accounting, highrank `lincom`,
existing supported projection combinations and point/MCSE attachment
equivalence under the registered numerical tolerance rules. Cover q1
eligibility and target-local unavailable statuses, strict rank/positivity/PSD
withholding, partial Gram batches, semantic RNG domains, memory boundaries,
cancellation/reuse, and data/sample/sort/RNG restoration on every exit.

Public tests must check `e(inference_centering)` and
`e(inference_mean_omitted)` only on active component requests, their absence
after failures and irrelevant requests, the fixed-mean output/replay note,
and continued Corrected rejection before RNG. Exact tests require build
identity `vckss-inference-api2-q1-target-status-projection-mean1-component-mean1`.
Native tests additionally require `component_centering_api=1` from matching
C transport and Rust core readiness bit 16, stale/missing cached capability rejection
before preparation/RNG and unchanged None compatibility. Check combined
exact-Mata requests with Mean, and continued rejection of native combined
component/projection requests for Mean and None. Register the focused regressions in
source quick/full, affected native and isolated-install profiles.

The [accepted plan](docs/MEAN_COMPONENT_INFERENCE_PLAN.md) also requires exact
Gaussian moments of the actual Mean kernel and a fresh bounded sampling
assessment. Freeze the source/input/semantic-seed/task inventory before final
outcomes. Recompute the observed mean on every outcome replication, never
within inference error probes. Report every attempted replication, all
unavailable targets and atomic failures, bias/true SD, approximation error,
SE calibration, coverage and numerical sensitivity. Validate missing,
duplicate, malformed, partial and scientifically failing outputs and
schedule-invariant aggregation. A bounded assessment is not a uniform
coverage theorem; historical None confirmations do not qualify Mean.

Qualification must bind the new source and final intended binaries on every
claimed platform. Dirty-tree development successes, old centering receipts
and passing source gates do not substitute for clean native qualification.
Preserve all failed gates and record limitations before promotion.

The [October 8 assessment](docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
is complete: 48,000 primary calls, 192,000 target rows and all 960 task receipts
are accounted for. Its 22 exact-Mata availability-screen failures remain
failures; eight native cells pass the broad descriptive screens. Stress and
fixed-outcome numerical profiles retain unavailable targets and material
numerical sensitivity. The final 32 harness tests pass against byte-verified
`b9f80ce9` harness files, identical at `63757839`.
Mac full and four-alias installed Mean checks pass at `b9f80ce9`; Linux full
and installed checks pass at `63757839`. The Windows artifact at `240ad74d`
passes hosted checks and private smoke/full qualification, including 12 exact
Mean, eight native Mean and 12 Corrected exact installed cells. All five
payloads are published at `66d0278b`. The original Mac/Linux bytes and archives are
reused only for unchanged public routes under an affected-route review; they
do not contain the lower-level serial Corrected fix in `240ad74d`.
The [adoption manifest](../native/mean-component-20261008/manifest.json)
binds sources, routes and preserved earlier failures.

Local HTTP fresh/replacement checks at `240ad74d` pass on Mac arm64, with all
61 installed file hashes, five payload hashes and eight Mean-component cells
per case. At published source `66d0278b`, the four public `net`/`github`
fresh/replacement cases have status PASS on Mac arm64,
with all 61 final files, five plugin hashes and eight Mean cells per case. See
the [public-install summary](../native/mean-component-20261008/evidence/packaging/public-install-66d0278b.json)
and [exact binding](../native/mean-component-20261008/evidence/packaging/public-install-binding-66d0278b.json).
Other platform runtime and native Intel hardware are outside this check. The earlier integrated source
checker remains staged-development evidence, not clean-SHA integrated
qualification. Engineering gates do not remove the assessment's limitations.

## Manual referee checks

The self-contained Stata entry points under [`tests/manual/`](tests/manual/)
simulate their own data and leave readable result datasets. The validation
suite checks exact repeatability, the JLA numerical envelope, optional native
parity, and a KSS Matlab behavioral comparison. The benchmark records
`fevc` and KSS Matlab estimator command time separately from MATLAB
process, pool, and optional run-local MEX setup. These runs are accessible
diagnostics, not release qualification or substitutes for source-bound
benchmark evidence. See [`tests/manual/README.md`](tests/manual/README.md) for
requirements and setup.

## Native plugin qualification

A meaningful change to Rust estimator code, the ABI/C shim, native Ado
boundary, exact/compressed/generic posting, package native helpers, or native
route tests requires:

```bash
./ci/run_ci_profile.sh plugin-build
```

The source-local qualifier runs locked Rust/C gates, authenticates the Stata
SPI, builds and audits thin/universal macOS candidates, exercises native arm64
and available Rosetta paths, and performs isolated clean-install tests. See
[`../rust/stata_backend/README.md`](../rust/stata_backend/README.md) and
[`../rust/TEST_PLAN.md`](../rust/TEST_PLAN.md).

A green quick receipt is not native qualification. Require
`.ci/stata/results/<tested-sha>.json` with the exact profile and status, then
inspect the source-local Rust/C and Stata qualification outputs. Historical
receipt-only `[skip ci]` commits are bookkeeping rather than the tested source.

## Current alpha regressions

The completed direct exact-V7 test remains in
`tests/stata/test_rust_planned_compressed_post.do` and prints
`EXACT_V7_RECONCILE_FAIL` plus the full returned receipt on any future
regression. Public auto-exact, its matching poster, zero-RNG execution, and
clean-install coverage remain permanent gates.

Rust-preferred effective-option routing, preflight-only fallback, automatic
JLA selection, `probeorder()`, exact stayer behavior, and the declared native
platform surfaces retain their permanent regressions. Current feature and
platform status is generated in
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md); the active checkpoint
and its impact-selected gates are defined only in [`PLAN.md`](PLAN.md).

## Hard acceptance checks

Candidate comparison follows
[`docs/development_acceptance_v1.json`](docs/development_acceptance_v1.json).
The four corrected targets are primary. Deterministic differences up to
`1e-8*max(1,abs(a),abs(b))` pass. Common-draw randomized comparisons also
allow ten percent of the larger reported numerical MCSE; independent-draw
comparisons allow six times combined numerical MCSE. Bitwise/ULP equality,
equal iteration counts, identical reduction order, and legacy fixed roundoff
gates are diagnostics, not blockers.

Tests must preserve:

- supported input, retained sample, target population, deletion unit, frequency
  and target semantics;
- exact and JLA corrected-result agreement with independent small-design
  oracles under the registered equivalence rule where their contracts coincide;
- identification, canonical control-basis, deletion-rank, inverse/maker, and
  finite-output gates without hidden regularization;
- every accepted RHS's complete original worker-plus-firm or
  worker-plus-firm-plus-control residual at
  `max(1e-11,10*effective_phase_tolerance)`;
- `total = worker + firm + 2*covariance` and
  `corrected = plugin - correction`;
- structural pre-RNG algorithm, engine, route, batch, memory, wall, fallback,
  and Counter receipts;
- optional strict admission within an explicitly supplied `memory_gib()`; and
- caller data, `e(sample)`, RNG algorithm/stream/state, and sort restoration on
  success, typed failure, and UserBreak.

Exact estimation does not require production RNG registration. JLA uses
separate runtime-scoped leverage and target domains. Pathwise draws must be
stable under row permutation, batching, route, processor count, and scheduling
within the same observed-ID design. Arbitrary ID relabeling may change a valid
draw.

## Structural routing and resources

`preconditioner(auto)` is structural. It does not run pilot RHSs or use a
projected-work withholding cutoff. Explicit CMG fails closed; automatic setup
fallback to diagonal is permitted only before estimator RNG and must be
recorded. Actual solves still must converge and pass the complete residual
gate.

`memory_gib()` is an optional direct-allocation budget. Only
`memorycheck(error)` makes its forecast an admission gate; the default is
`memorycheck(warn)`. Omission declares no budget. Wall forecasts, the
registered 30-percent memory headroom, 50-percent wall allowance, timing
models, and automatic batch percentages are planning diagnostics or selection
heuristics, not scientific withholding gates. No budget means no memory-based
batch, concurrency or route adjustment; explicit batches remain unchanged.
Conditional refinement storage is included in the admission forecast and is
separate from percentage headroom. Malformed receipts and actual allocation
failures still fail closed.

The focused public policy test is `tests/stata/test_memory_policy.do`; it
checks both backends, absent budgets under every policy, explicit budgets and
batches, CMG, invalid options, RNG restoration and idle native lifecycle.
Native allocation-layout tests check retained boolean storage and strict
one-byte boundaries. `compressed_jla_memory` independently measures four
full-CMG core fixtures in debug and release. Its registered 5%/1-MiB excess
gate is a bounded allocation-accuracy test, not total RSS or an all-regime
precision guarantee. Keep new accuracy measurements separate from historical
receipts; see [MEMORY.md](docs/MEMORY.md).

## Optional performance and cluster evidence

Performance work must use complete-command, source-order-controlled evidence
and preserve hard correctness plus registered corrected-result equivalence
before interpreting time or RSS. A harmless pathwise numerical difference is
recorded but does not stop a promising performance lane.
The retained source-bound reports are indexed in
[`docs/README.md`](docs/README.md). Do not rewrite an old report for newer
source.

When SCC evidence is useful, submit one scalar SGE job containing one Stata
process. Freeze slots, Stata processors, memory per core, wall time, fixture,
dimensions, probes, seed, batch, source hash, and input hash. Restricted data
remain under authorized storage and must never enter this repository.

Historical comparator timings are descriptive unless the estimator, sample,
weights, targets, RNG, and tolerances coincide. Preserve an unweakened failure
rather than modifying the comparator or input to obtain a number.

Workspace cleanup uses `tools/clean_workspace.py --dry-run` first and
`--apply` only after reviewing the protected allowlist. Tracked source, `.venv`, local archives, and source-bound evidence are never
eligible. Historical retention checks were retired with the corresponding
checkout material; see [the archive note](../docs/ARCHIVE.md).
