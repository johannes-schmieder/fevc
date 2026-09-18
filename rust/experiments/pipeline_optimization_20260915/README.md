# Complete-pipeline optimization — September 15, 2026

Owner-approved scope: finish optimization of all supported Rust point, exact,
projection and component-inference paths, equally for observation and match
deletion, including controls, weights and eligible stayers. Preserve the fast
compressed path, statistical semantics and every numerical/resource/lifecycle
gate. Mata remains the reference/fallback. Fusion remains experimental.

The starting comparison is the accepted September 15 paper runtime, native
source manifest `6d8e86aafbe1224e29217ce4565a7c91c22fcc60583bd97b2805b910298ae18d`.
Freeze the current worktree before edits and verify its relevant source against
that manifest; a commit alone is not the identity of this dirty baseline.
New development artifacts live in `.local/pipeline-optimization-20260915/`.
Do not modify accepted evidence, install PLUS, change the paper, commit, push,
create a branch/worktree, change CMG dependencies or run at 6.4m.

## Work order

1. Freeze baseline; add development-only phase/allocation/work diagnostics.
2. Share bounded, cancellation-safe ordered execution using existing pools.
3. Optimize RNG, RHS assembly, leverage moments, target contractions, original
   residual certification and controlled recovery across both deletion modes.
4. Extend shared improvements to supported attachments, exact estimation and
   preparation. Exact threading needs an additive capability/receipt; old ABI
   meanings remain unchanged. Keep dependent recurrences and all residual gates.
5. Pass numerical, memory, failure, cancellation/reuse and native/install gates;
   run the bounded performance screens; reprofile and report remaining hotspots.

Use thread-aware batches; freeze structural crossover rules before screening.
Omitted memory imposes no budget planning. Admit owned scratch, packed RNG,
outputs, queue and pool lifetimes before allocation; do not allocate all
probes-by-rows or multiply dense model workspaces unnecessarily.

## Approved measured screens

- Twelve supported-path profiles at 1/4/7 threads: the eight existing
  optimization-parity profiles plus mover observation, supported match projection
  and both exact modes. Baseline/candidate, three measured repetitions (216
  calls); existing 100k point/8k attachment/small exact dimensions.
- 400k, five paper graph classes, both deletion modes, 4/28 threads:
  baseline/candidate and Matlab additionally at 28; three repetitions (150).
- 1.6m, degree-four bottleneck, mixed segmented, degree-five well-mixed,
  both deletions at 28 threads: baseline/candidate/Matlab, five repetitions (90).
- Full existing private Veneto, both deletions at four native threads:
  baseline/candidate, three repetitions (12); raw/detail data stays private.

Total 468 measured calls plus separate warm-ups, not an authorization to submit
before prerequisite gates pass. Each variant has one warm-up per configuration;
rotate measured order, run variants sequentially on one host, freeze source,
input, commands, counts, seeds and expected outputs before each stage.
Use 200 point probes, omitted tolerance and existing attachment probe counts.
Keep setup-inclusive, estimator-only and process time separate; compare
estimator-only Matlab performance without hiding pool startup or RNG caveats.

SCC defaults: 4x3G/25min smoke, 14x3G/45min small FEVC-only screens,
28x3G/45min comparisons/private checks, unrestricted eligible host/queue and
project welfgr. Retain small FEVC 20 GiB, large/private FEVC 45 GiB and Matlab
72 GiB physical-RSS guards. Stata processors=min(4,T); native threads=T through
the verified isolated benchmark adapter. Check live jobs/capacity before use.
One narrow operational repair/retest is allowed; another operational or any
scientific failure stops advancement. Do not automatically increase resources.

## Acceptance

Follow `fevc/docs/development_acceptance_v1.json`. Keep bundles with >=3%
complete-command gain on their intended workload and no covered median
regression >5%; prefer simpler implementations within 2%. Target >=20%
aggregate 400k observation improvement without losing match performance.
Matlab competitiveness is <=1 and the development goal is <=0.5 candidate/
Matlab, never a reason to weaken correctness. Confirm gains separately by
deletion at 1.6m. Every final phase >=5% of command time needs an optimization
or a measured/dependency-based reason for remaining serial.

## Checkpoint

The first source slice implements shared bounded statistical execution on the
already-owned CMG/diagonal pools, parallel counter/RHS preparation, disjoint
leverage accumulators, combined target contractions, target-diagonal sweeps and
independent generic original-system certification/control completion. Every
original-system gate and the serial, ordered controlled-refinement ladder
remain. No public options, capability claims or native layouts changed.

Frozen `slice-03` passed full workspace/all-target tests, formatting, strict
Clippy with/without diagnostics, and isolated macOS native/Stata qualification
(arm64, Rosetta x86, thin/universal). Its native manifest is
`37b3bbff24cd9b591fac38f2057ca37940dd7b75ccd84cf9e129675e424e8371`.
It has 273 core unit tests and includes packed four-probe Philox generation.
This RNG packing is not experimental fused CMG execution. An earlier queue
header test failure was repaired by removing unused payload, not raising the
admitted bound. `slice-01` and `slice-02` omitted native inputs and are not
qualification snapshots; required-manifest completeness is now enforced.

All 45 candidate 100k diagnostics pass (five graphs, observation, compressed
match and generic match, 1/4/7 threads); maximum absolute corrected-target gap
against 30 baseline calls is 4.44e-16. Seven-thread observation core speedups
are 1.24–2.27x. Single timings include regressions elsewhere and do not establish
native-command acceptance. See `diagnostic-03-report.md` under the artifacts.

The subsequent core-only exact prototype adds ordered parallel matrix products
and deletion corrections, including hybrid stayers. Concurrent scratch is
admitted before pool creation. Dense buffers are fallible and triangular solve
scratch is reused across inverse columns. All 275 core units pass, including
thread caps 1/2/3/4/7/14/28/64, controls, weights, maker shapes, exact-budget
boundaries and cancellation/reuse. No native entrypoint selects the prototype.
Later exact changes are not covered by the `slice-03` native qualification.

All twelve `path-diagnostic-02` attachment checks pass from `slice-05` after a
diagnostic wrapper's missing projection-export admission was repaired. The
failed `path-diagnostic-01` remains preserved. At seven threads residual Gram
and spectral work account for about 84% of observation inference and 80% of
match inference. Projection statistics are below 1% in these small cases.

The next source slice parallelizes residual Gaussian probes and moment work,
RHS packing and prediction on the owned pool. Centered covariance recurrence
remains in probe order; prediction writes directly into admitted outputs.
Existing attachment/budget/cancellation tests pass; added token-enabled checks
cover both kernels/deletions and all thread caps. The test initially requested
129 Gram probes (the contract requires at least 512) and then incorrectly
compared different solver kernels bitwise (maximum Gram gap 2.13e-14, with all
registered scientific checks already passing). The corrected test uses 513
probes, checks legacy-vs-direct with the unchanged scientific policy, and
requires bit-identical Gram/fitted variance across threads of the same kernel.
Both failed test logs are retained. Frozen `slice-06` subsequently passed the
full workspace, strict Clippy/format and isolated arm64/Rosetta/universal native
Stata qualification. Native manifest:
`06985e5f53cbf0b58c452a2f6d4a97a55a1782a74cc88b0edaeabd31a504d860`.
The first qualifier attempt lacked its required empty artifact directory;
the successful receipt is `slice-06/native-receipt-02.txt`.

All twelve `path-diagnostic-03` cases pass with zero corrected-target gaps at
reported precision and unchanged q=0 computed/withheld statuses. Seven-thread
component core time changes 0.766612→0.510957 seconds for observation and
0.326586→0.270852 for match. Gram work falls 0.319175→0.071739 and
0.086815→0.031294 respectively. Spectrum remains the largest phase. These
single instrumented calls are not complete-command acceptance. The source-bound
comparison is `path-diagnostic-03-report.md`, generated by `analyze_paths.py`.

`slice-07` adds an explicit experimental exact entrypoint, capability and
64-byte receipt (296-byte request, 320-byte interrupt request). Both base and
optional hybrid passes are admitted before either executes. Existing V1–V8
entrypoints and ordinary Stata exact reconciliation remain serial and unchanged.
The opt-in is reachable only through separately named native/C selectors;
`exact_transport_smoke.do` exercises them without changing the public command.
Observation stayers use the ordinary population; only match uses the mixed
deletion augmentation. All 64 weighted/control/deletion/stayer/thread test
combinations match serial targets bitwise; strict budget edges, caller-only
callbacks, cancellation/reuse, invalid requests and C ABI/transport tests pass.
Full workspace/Clippy/format and ordinary arm64/Rosetta native qualification also
pass. Opt-in-specific Stata validation now passes all 32 cases on arm64 and
all 32 under Rosetta (both deletions/nuisance conventions/populations and
legacy/1/4/7-thread execution). Exact source/binary/test hashes and failed
attempts are recorded in `exact-transport-qualified-01/receipt.json`.
This is thin-binary local transport validation, not public-command integration,
Windows/Linux or representative-scale performance evidence.
Preserved failed tests cover
test-only type/constant errors, an incorrect observation augmentation fixture,
and a late cancellation poll that tiny inputs can finish before. The first
direct Stata smoke ran before the thin plugin was staged and failed at probe.
Further smoke-only errors were raw negative-exponent plugin arguments (the
isolated `argument_probe` proved that Stata passes bare `1e-12` as `1e`) and
stayer identifiers not separately dense. The passing test uses decimal-form
numeric arguments and the established dense augmentation identifiers; neither
fix changes estimation or public parsing.

The subsequent spectral prototype uses bounded statistical jobs for target RHS
preparation and prediction, charges additional concurrent RHS scratch and queue
metadata, reuses total-target buffers and removes an unnecessary iteration
vector clone. The fixed recurrence/count, target map and scientific gates are
unchanged. Its new cancellation regression fails before implementation and
passes afterward; all seven focused direct-component tests pass, including
both kernels/deletions, all thread caps and budget/withholding/reuse checks.
Frozen `slice-08` passed the full Rust workspace, strict Clippy and formatting.
All twelve diagnostic calls pass with unchanged corrected targets and interval
statuses, but this single small-input timing does not show a spectral gain:
seven-thread observation component time is 0.541368 versus 0.510957 seconds,
and match is 0.288477 versus 0.270852. Several unchanged phases also drifted.
Do not discard these regressions or infer a performance win from parallelism;
the new spectral scheduling remains experimental pending paired screening.
This source is not covered by the earlier native qualifications. Report:
`path-diagnostic-04-report.md`.

The next continuation parallelizes compressed leverage RHS construction and
routes its statistical phases through fallible ordered jobs with interior
cancellation. Explicitly inert core callers retain parallel execution; custom
host callbacks without a token stay on the caller. Probe addresses and each
output's arithmetic order are unchanged. Target directions reuse their output,
both ordinary/full-CMG forecasts charge scratch and queue metadata, and native
planned compressed solves borrow the existing preparation-time semantic plan.
Weighted/partial/all-thread, cancellation/reuse, result/counter/memory identity
and overflow checks pass. The first complete `slice-09` check failed in a new
allocator test's overlapping measurement windows, followed by poisoned test
locks. Separate processes repair the test; all thirteen allocator suites pass.

`slice-10` deliberately selects the verified pre-prototype spectrum code/test
from `slice-07`, recording four per-file origins and excluded working hashes.
The experimental `slice-08` remains intact. Its 1,368-file snapshot hash is
`82aecd7ec9b92627cdaab57f4d7e8e873be3cffa82b2f892ac9c19283d3b89ba`.
Full pinned workspace/all-targets (278 core units), strict Clippy/formatting and
arm64/Rosetta/thin/universal native Stata plus isolated installs pass. Native
source manifest: `d2a9980d4ea14f63445baee7e6ee6c2553b367ed9b9519163affc3cbea791ea8`.
The Mac qualifier truthfully records explicit Command Line Tools selection;
ordinary Xcode failures still fail closed. No license was accepted. An old
source-bundle allowlist rejected new public experiment/test files; the focused
extension retains private-data rejection and passes fourteen tests. The later
699-test package/diagnostic/inventory run passes; unchanged CMG/scale tests also
passed in the initial full Python run. Later harness/docs do not alter the
qualified native sources. Current working-tree spectral code is not covered by
this excluded-prototype candidate's qualification.

`paired_compressed.py` defines a capture-free two-graph 100k diagnostic at one
and seven threads: 24 measured calls plus eight warm-ups, rotated execution,
fixed source/input/binary hashes, unchanged corrected-target gates and explicit
failed/unattempted states. Its three harness tests pass. It excludes Stata,
native plan-reuse savings, Matlab, SCC and RSS; it cannot promote a candidate.

All 24 measured calls and eight warm-ups pass with identical reported corrected
targets. Median paired seven-thread core runtime falls 7.96% for degree-four
bottleneck and 1.64% for mixed segmented, incrementally versus `slice-07`.
One-thread changes are 2.19% slower and 0.08% slower. Do not substitute the
ratio of independent medians for these paired ratios. Three repetitions and
core-only timing do not establish complete-command acceptance. All attempted
calls, exact binaries, public inputs and failed validation attempts are archived
in `.local/pipeline-optimization-20260915/continuation-10-validation/REPORT.md`.
The final combined Python suite passes all 817 tests with the recorded explicit
CLT/Homebrew environment. An invocation that omitted that environment failed
136 tests on the existing Xcode-license issue; both logs are retained. No source
or system settings changed for the passing rerun.

`campaign.py` defines all 468 measured calls, separate warm-ups and rotations;
two inventory/resource/identity tests pass. It submits nothing. Remaining:
the rest of the preparation audit, public exact integration, the guarded
twelve-profile screen, 400k, 1.6m and private Veneto gates, and the final report.
No cluster campaign has started or bundle passed complete-command acceptance.
