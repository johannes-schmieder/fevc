# Active package plan

## Objective

Harden the private `fevc` `0.5.0-alpha.1` source into a clean release-candidate
checkpoint while retaining its accepted role as a fast, statistically
equivalent Stata alternative to maintained MATLAB KSS on compatible problems.
Do not change the version or release scope in this pass.  The owner-directed
exception is the registered MATLAB population-parity change below: the
existing mover/stayer mixed-deletion estimator is now the match-deletion
default and is supported by both exact and generic JLA computation.

Candidate promotion follows
[`docs/development_acceptance_v1.json`](docs/development_acceptance_v1.json).
Durable behavior and release decisions live in
[`docs/DECISIONS.md`](docs/DECISIONS.md), and current backend/platform
coverage lives in
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md).

Windows qualification, public tagging, and distribution remain deferred. A
possible `0.5.0-rc1` designation is an owner decision rather than an implied
version bump, tag, or public release.

## Active owner-directed benchmark campaign

The owner has authorized a new, source-bound FEVC--maintained-MATLAB benchmark
and complete paper refresh.  The registered implementation lives under
[`benchmarks/fevc_matlab_2026`](benchmarks/fevc_matlab_2026/README.md).  It uses
MATLAB R2026a, ordinary `welfgr` SCC space, homogeneous 28-core Broadwell nodes,
public Veneto only, and a limited manual-style Mata slice.  The campaign adds
absolute runtime/data/estimator memory decomposition, a conditioning continuum,
row-versus-graph-dimension separation, stayer/population audits, projection and
architecture sensitivities. Development uses a small 4-core end-to-end SCC
smoke. A clean source-bound campaign then uses one artifact build and scheduler
dependencies for smoke, worst-case pilot, and production; monitoring observes
the campaign and does not control stage release.

This owner instruction supersedes the earlier deferral of a replacement
comparative-scaling matrix for this campaign only.  It does not authorize an
estimator change, CZ18 use, release designation, tag, publication, or push.

### Campaign timeline

- On 2026-09-01, exact source
  `0077d9f02e083c944d923287166c581055dcbf97` submitted campaign
  `20260901T114600Z-0077d9f0-submit` as smoke job `7400008`, worst-case pilot
  `7400009.24`, and production array `7400010.1-24`.  Artifact preparation and
  both smoke applications completed, but the smoke validator rejected the
  documented compressed-engine success status
  `KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES` because it required the unrelated
  literal `OK`.  Smoke accounting is `failed=0`, `exit_status=1`; the pilot
  then failed closed without running its cell, and production is in the same
  fail-closed dependency path.  No result is accepted and no automatic retry
  is permitted for this schema/application failure.  The owner subsequently
  authorized the narrow validator correction and one focused smoke retest.
- Source `cec81d6dab92e3e3373d164f6e3d6760d27f47fb` submitted that single
  recovery smoke as run `20260901T125104Z-cec81d6d-smoke`, job `7400583`.
  Both applications again completed and the Rust numerical receipt reported
  the documented compressed engine, `CMG_FULL_V2`, 7,680 observations, and a
  `2.40412538805e-15` complete residual against a `1e-5` tolerance.  The
  validator nevertheless required the engine literal `rust` rather than
  `compressed`, so accounting is `failed=0`, `exit_status=1`.  The bounded
  wrapper-recovery allowance is exhausted: no evidence is accepted, no
  production campaign was resubmitted, and a second validator correction
  requires a new owner decision.
- The owner authorized that second narrow validator correction.  Clean source
  `e916d394c4efdd1b08373c59c1e40729922ceabc` submitted replacement campaign
  `20260901T130843Z-e916d394-submit` as smoke job `7400810`, worst-case pilot
  `7400811.24`, and production array `7400812.1-24`.  The effective scheduler
  receipts confirm 28-core `omp28` allocation, `E5-2680v4`, 8G/core, an
  8-hour limit, and linear 28-core affinity for the pilot and production;
  only the diagnostic smoke uses four cores.  Scheduler dependencies control
  progression.
- That replacement campaign reached terminal failure before any 28-core cell
  ran.  Smoke `7400810` completed artifact preparation and both applications,
  with `failed=0`, `exit_status=1`; its Rust receipt reported the documented
  status, compressed engine, `CMG_FULL_V2`, sample count, a
  `2.40412538805e-15` complete residual against `1e-5`, forecast-bounded
  memory, and successful restoration flags.  Validation then rejected empty
  legacy phase-timing
  fields such as `selection_seconds`, even though the populated native phase
  family is present.  Pilot `7400811.24` and production `7400812.1-24` failed
  closed through their scheduler dependencies.  No 28-core timing is accepted
  and no automatic retry is permitted for this repeated validator-schema
  failure.
- The owner authorized a correction that permits blank legacy Rust phase
  timings while still requiring every native Rust phase to be finite.  Exact
  replay of the failed smoke passed the complete application validator, and
  source `847bbf87b92a818207c38149a68f5133a647ca19` then passed focused
  4-core smoke `20260901T144714Z-847bbf87-smoke` as job `7403342`, including
  terminal accounting (`failed=0`, `exit_status=0`) and collection.  The same
  clean source submitted replacement campaign
  `20260901T145443Z-847bbf87-submit` as smoke `7403413`, worst-case pilot
  `7403414.24`, and production `7403415.1-24`.  Effective receipts again
  confirm 28-core `omp28`, `E5-2680v4`, 8G/core, an 8-hour limit, and linear
  28-core affinity for pilot and production.

## Current accepted state

- Point estimation remains the default. Exact-observation component inference
  and fixed-effect projection inference are explicit, capability-gated
  requests.
- Match deletion and `nuisance(joint)` remain the defaults. Omitting
  `algorithm()` selects the MATLAB-like 200-probe JLA route; explicit
  `algorithm(auto)` retains exact-small/JLA-large structural planning.
- Match deletion now also follows the maintained MATLAB population default:
  one pooled mover/eligible-stayer target with mixed match/observation
  deletion. `stayers(movers)` remains the explicit mover-only convention.
- Portable Mata and qualified Rust routes share the registered estimator,
  sample, target, weighting, failure, and complete-residual contracts without
  requiring pathwise floating-point identity.
- Rust-preferred automatic routing may fall back to Mata only during structural
  preflight, before native preparation and estimator RNG. Selected native
  failures fail closed.
- Exact, compressed JLA, generic JLA, and the package-owned `CMG_FULL_V2`
  route have source-bound qualification on their declared surfaces. A green
  quick lane alone is not native qualification.
- The accepted alpha evidence covers macOS arm64/Rosetta and SCC Linux x86-64.
  The completed comparative-scaling study and full-CMG production evidence
  remain source-bound; they need not be rerun for unrelated changes.
- The human package-boundary, corresponding-source, notice, provenance, and
  data-exclusion review is complete. Public release still requires a fresh
  exact-artifact decision.

Supporting contracts and evidence are indexed by
[`docs/README.md`](docs/README.md).

## Terminal scientific checkpoint: scalable projection preconditioning

The explicit Rust generic-JLA projection route supports mover-only observation
deletion, literal-copy positive integer frequency weights, and explicit
diagonal PCG or forced generic CMG. It preserves the established
`e(projection_*)` result schema and uses Mata exact as the independent oracle.
Automatic projection routing remains withheld.

This checkpoint is terminal and deferred, not the active implementation
milestone. Its immutable results define a future numerical-research boundary;
they do not authorize another SCC run or an estimator change during release
hardening.

Exact source `96e7a66` completed the registered focused SCC comparison:

- all 6,000- and 24,000-row repetitions passed the statistical gates;
- maximum covariance-diagonal differences were below 0.48% and standard-error
  differences below 0.24%;
- diagonal PCG was 18.97 times slower than MATLAB at 24,000 rows; and
- the 96,000-row Rust solve failed to converge after 20,000 iterations, while
  the independently reconstructed MATLAB fit also reached its iteration limit.

The diagonal route is therefore numerically qualified on its accepted cells
but is not promoted for comparable large-data reach. The evidence and
interpretation are in
[archived scalable-projection record](../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference).

The forced-CMG composition now reuses the generic-JLA model hierarchy already
implemented and qualified for planned generic solves. That hierarchy is shared
between the full W+F+Q and FE-only solvers, including the certified control
block. This is deliberately not `CMG_FULL_V2`, whose direct compressed hybrid
API is tied to the no-control match-deletion point-estimation cell.

The first local promotion probes preserve the common Counter-V1 formula path:

- at 6,000 rows, CMG and diagonal differ by at most `2.28e-11` across the
  three projection coefficients and full 3-by-3 covariance;
- 6,000-, 24,000-, and 96,000-row CMG commands all pass residual, PSD, and
  memory gates;
- worst projection-solve iterations are 17, 38, and 79 rather than 161, 625,
  and diagonal nonconvergence; and
- the 96,000-row CMG command completes locally in 338.7 seconds with an
  `8.85e-11` maximum projection complete residual and a 39.6 MB projection
  memory forecast.

The same three probes were then bound to exact source `33ede86`.  Every
statistical and numerical field was identical to the candidate run.  Command
times were 6.627, 42.609, and 344.04 seconds, while complete-command peak RSS
was 186.4, 303.5, and 451.4 MB.  These are exact-source local convergence and
resource observations, not a same-host MATLAB performance comparison or a
cross-platform reach claim.  Compact receipts are under
[archived under the exact-source local forced-CMG receipts](../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference).

The first 480,000-row paired feasibility attempt at workflow source `3b46a11`
completed with clean scheduler accounting but no accepted role. Forced-CMG
model PCG reached the registered 20,000-iteration limit at reduced residual
`1.9074751337948694e-10` in both core cells, while maintained MATLAB
independently rejected its grounded fit.

The owner-directed continuation at exact source `30d49fd` is also terminal.
Preparation job `7373800` and the 6,000-row gate job `7374823` passed their
complete scheduler, source, input, build, application, numerical, and resource
checks. Both tasks in 480,000-row array `7374830` have clean scheduler
accounting (`failed=0`, `exit_status=0`), but VCkss reaches the registered
40,000 model-PCG iteration limit at the identical reduced residual
`4.412307557090175e-10`. The observed failed-role walls were 9,733.567 seconds
at four application cores and 9,719.729 seconds at sixteen; they are diagnostic
failed-run durations, not accepted VCkss completion times. Maintained MATLAB
is correctly reason-coded `RIGHT_CENSORED/MATLAB_FIT_NONCONVERGENCE` in both
cells, with diagnostic failed-command walls of 492.664 and 297.322 seconds.
Those observations are not completion times, are not imputed, and do not form
a speed ratio. The committed collector records `COMPLETE_NONPASS`; no larger
stage was submitted and paper performance claims remain unchanged. Compact
source-bound evidence is under
[archived with the projection/AKM scaling evidence](../docs/history/VCKSS_ARCHIVE.md#performance-records).

The affected-surface implementation gates are green: pinned Rust formatting,
strict Clippy, and workspace/all-target tests; generated-CMG checks and its 28
tests; focused Python formula, weight, packaging, and parity tests; C shim and
ABI checks; source-local and isolated-install Stata projection tests; and the
1,002-row exact/MATLAB oracle. The two repository-wide harness defects formerly
recorded here are closed: the scale-bundle allowlist contains the complete
installed runtime, and the Stata 19 planned-route brace problem was repaired
during the hard rename. Their permanent regressions and the rename
qualification are green.

## Active checkpoint: release hardening

The pass starts from clean source
`acead96e6032f116bc192c5229d8446855c8f74e`, with `main` equal to
`origin/main`. The rename had already passed quick/full Stata, clean install,
benchmark, CMG/B1, separation, MATLAB-bridge, and predecessor-equivalence
checks; all 465 Python tests and the generated-CMG check passed.

1. Reconcile the active plan, README, help, changelog, manifest, install
   instructions, versions, examples, defaults, supported tuples, platform
   coverage, and failure behavior without changing runtime semantics.
2. Run the public-identity, legacy-name, preserved-history, package, license,
   provenance, source-supply, generated-parity, and source-layout audits.
3. Provide and test a deterministic, non-publishing artifact constructor driven
   by `fevc.pkg`, with an exact source/file/hash receipt.
4. Strengthen clean-install coverage for help lookup, point estimation, Mata
   fallback, and graceful unavailable-plugin preflight behavior.
5. Carry forward unaffected scientific, performance, native, and platform
   evidence through
   [`docs/RELEASE_HARDENING_2026-08-31.md`](docs/RELEASE_HARDENING_2026-08-31.md)
   rather than rerunning SCC or plugin qualification.

## Release checklist

### Completed in this pass

- Reconciled this plan with the completed rename qualification and repaired
  harness gates while preserving projection as a terminal scientific
  checkpoint.
- Audited public identity, deliberate private/historical identities, versions,
  defaults, examples, paths, package contents, licenses, notices, provenance,
  and generated parity documentation.
- Added a deterministic manifest-driven portable source-artifact procedure and
  exact receipt, with focused regressions.
- Strengthened clean-install and cleanup-safety coverage without changing the
  installed runtime or native payload.
- Ran the affected-surface source and licensed-Stata gates recorded in the
  release-hardening review. Native source, ABI, build, helpers, and native
  packaging did not change, so plugin requalification was not selected.

### Larger technical work deliberately deferred

- Windows qualification, a public tag, and public distribution.
- Match-cluster inference and unsupported frequency/stayer inference tuples.
- A command-surviving native cache or new public result interface.
- Changes to the estimator, target definitions, finite-projection formula,
  default point-only behavior, or accepted historical evidence.
- A replacement comparative-scaling array, broad platform matrix, or paper
  claim before a stronger projection route earns focused qualification.
- Diagnosis or redesign of the 480,000-row model-PCG stagnation.
- The registered inference simulation study and coefficient-one mathematical
  review.

### Owner decisions still required

- Whether to designate the hardened source `0.5.0-rc1` without changing the
  package version in this pass.
- Whether the exact constructed source artifact passes the final human
  conveyance review.
- Whether and when to tag, publish, change visibility, create a GitHub release,
  or distribute source or binaries.
- Whether any deferred platform or scientific work should become an RC gate.

## Acceptance and qualification

- Statistical equivalence, tolerance behavior, hard correctness, performance
  priority, qualification selection, and evidence reuse follow the registered
  development policy.
- Every accepted solve must pass identification, accounting, finite-output,
  complete original-system residual, direct-memory, typed-failure/UserBreak,
  and caller-state/lifecycle gates.
- Performance claims require compatible requests, samples, graph structure,
  probe count, tolerances, hardware/affinity records, and complete-command
  measurements. Kernel-only wins are insufficient.
- Keep independent oracles independent and historical receipts immutable.
- Documentation-only changes receive focused checks and carry forward
  unaffected scientific/performance evidence through impact review.

## Completion gates

The active checkpoint closes only when:

1. Python, generated CMG, package, identity, history, license/provenance,
   parity, artifact, and source-supply checks pass;
2. the integrated Stata quick/full, clean-install, benchmark, CMG/B1, and
   separation checks print their explicit PASS markers;
3. the artifact is reproducible from the clean committed checkout and contains
   only the catalog, manifest, and manifest-listed portable source;
4. the compatibility review shows why source-bound scientific, performance,
   native, and platform evidence remains unaffected; and
5. `git diff --check`, complete diff review, and final status are clean, with
   no tag, publication, or release action inferred from the private alpha.
