# Deletion-mode solver symmetry — September 12, 2026

## Scope and implementation

Owner-authorized implementation on the existing worktree above `fbf8dcd`.
This is not a release, commit, push or paper rerun. Native qualification stages
the tested Mac developer plugins in the source package; it does not replace the
installed Stata PLUS package or distribute public binaries.
Fusion remains experimental. Existing inference and controlled-model capabilities
are unchanged.

Eligible no-control, unit-frequency, default-target, joint/mover JLA point calls
with automatic engine/preconditioner/batches and explicit `probeorder()` now share
the unfused full-CMG solver for **both observation and match deletion**. Match
retains its compressed statistical estimator; observation retains generic JLA's
physical-observation probes and correction. Only match requests the implicit
worker–firm deletion key. The observation route does not reuse match corrections.

The shared solver uses degree-four elimination, the ordered work queue, and
fixed k=2 widths based on actual permitted threads T: leverage
`min(P,max(32,8*T))`, target `min(P,max(32,4*T))`. Workspace and maximum RHS
capacity derive from selected widths. Explicit budgets can narrow automatic
widths; omitted memory imposes no memory budget. Public explicit full-CMG batches
remain unsupported. Fit/probe defaults are 1e-10/1e-6; explicit tolerance overrides
both. Original-system certification, scalar refinement and cancellation remain.

No-control generic model solvers share one owned direct hierarchy and pool; fit
and probe phases remain separate. The generic original operator independently
checks returned solutions as well as the direct solver's complete-system gate.
Queue metadata, returned vectors, residual work, retained operators and deferred
pools are included in the generic statistical engine's phase forecasts. Setup
also includes the pinned CMG construction estimate: dense terminal work can be
larger than the subsequently retained hierarchy. Public ABI layouts are unchanged.

This does **not** claim parallel acceleration for controls, weighted public
requests, explicit generic routing, stayers(both), or inference. Those retain
their existing capabilities. Positive integer frequency and nonuniform target
weights are nevertheless covered by the shared core's numerical tests, to avoid
building a solver that is intrinsically match-only or unit-weight-only.

## Local correctness and qualification

- Shared-core corrected-target/common-counter comparisons cover both deletion
  modes, mixed worker degrees 2–8, repeated observations and heterogeneous
  frequency/target weights, at 1/2/3/4/7/14/28/64 threads. This is unit coverage,
  not a 64-core performance claim.
- Tests cover budget reduction, exact budget acceptance/one-byte rejection,
  forbidden batches, cancellation during setup/leverage/target and successful
  reuse. Existing direct-solver queue, residual/refinement and allocation tests
  remain part of the full Rust suite.
- All 560 top-level Rust tests passed (one pre-existing ignored diagnostic),
  including the additional memory test's 12 isolated subprocess measurements.
  The allocator test was then serialized against other allocator tests and its
  six-test integration binary rerun successfully. Strict Clippy, formatting,
  805 Python tests and generated CMG checks pass.
- Isolated public Stata tests pass for both modes, corrected targets, retained
  sample, phase-specific tolerances, actual threads and batch capacity, RNG and
  data restoration, typed memory failure and idle/released lifecycle.
- Broad macOS arm64/Rosetta thin/universal qualification passes, including four
  native observation/full-CMG tests and the unchanged controlled/inference
  surfaces. Its receipt truthfully identifies `LOCAL_CHECKPOINT_DIRTY_TREE`
  above the baseline commit. Native source manifest:
  `d66523c875156118999bcd420879c4dca0bc552f345aafb742b2b56985186ba1`.
- The original qualifier's printed scope/command list omitted the newly run
  observation tests, but all four sanitized application logs contain PASS.
  Later edits repair only those printed descriptions and pass its self-test.
  An independent audit rehashes all 185 native inputs and every frozen source
  file, allowing only enumerated documentation and receipt-description changes.
  The runtime and staged binary bytes remain identical to the tested versions.
- Final source/history/licensing, portable-package and generated parity checks
  pass. The Python suite passes again after the documentation updates (805
  tests). The integrated quick-suite marker was verified; full-suite and
  clean-install checks have separate retained successful process/application
  receipts (501.688s and 1.913s). Capturing those final receipts required a
  focused repeat because the earlier combined console output was truncated;
  no missing console text is treated as evidence of a pass.

Observed principal heap payload across 12 high-degree construction tests is
0.63–4.57 MB; final forecasts 1.12–8.21 MB cover every measurement. These are
allocator payload measurements, not process RSS or scheduler virtual memory.

Development failures are preserved: observation deletion-source guard mismatch;
setup-peak export and width-one reconciliation omissions; a construction peak
underestimate; a stale manual benchmark anchor; and a last-bit scalar-vs-local
macro tolerance comparison on an unchanged inference route. The latter now uses
numeric scalars and the affected inference test passes. The memory test's first
multi-pool measurement also exposed asynchronous thread-local teardown crossing
tracking epochs; each measurement now uses an isolated process.

## Bounded performance checks

Frozen baseline: `fbf8dcd6187351d09cb3de735c148ef608ec0219`.
Frozen candidate manifest SHA256:
`c8c7d6252b801f4958b3a42d7e594253bcd44df51607e8ffb8999aa01bfd45c5`.
No upstream source or fusion changes.

- Smoke: 8k observations, repeated matches, four native threads; four old/new
  estimator calls and one deliberately failed call. Local gate passes.
- 100k: degree-four singleton matches and mixed-degree repeated matches,
  at 1 and 7 threads. Both deletion modes; three rotated paired repetitions
  plus separate warm-ups (48 timed calls, 16 warm-ups).
- 400k, only after 100k validation: well-mixed and segmented mixed-degree
  repeated matches, at 7 and 28 threads; same repetition design.

All pairs use identical inputs, seeds, 200 probes and explicit 1e-10 tolerance,
sequentially on the same host. This isolates execution changes from different
default tolerances. Corrected-target gaps must satisfy 1e-8 times the common
scale; samples and accounting must agree. Full-command timings, phase receipts,
RSS, output hashes and every attempted call are retained. Warm-ups are excluded
from performance summaries. No Matlab or 1.6m/6.4m campaign is authorized here.

The inherited launcher initially exposed Stata returning a zero shell status
after an application error. A tested receipt-checking shell boundary now
propagates that error, and the complete local smoke passes. The initial local
RSS monitor was blocked by the sandbox; the rerun used authorized process
inspection. These failed local development attempts remain separate from SCC.

SCC smoke job **7537411** passes scheduler, application and independent output
checks: four successful calls plus deliberate application-failure propagation.
Resources: 4 slots × 3G, 25 minutes, unrestricted eligible
queue/host. Later bounded cells request 28 slots × 3G, 45 minutes; no automatic
resource increases. SCC root:
`/projectnb/welfgr/vckss/runs/20260912T211000Z-deletion-symmetry`.
The first 100k input-preparation launcher (**7537423**) failed before generating
inputs because the inline binary-job shell arguments were not preserved. An
attempt to cancel it was made before accounting showed its exit status 1.
The one permitted operational repair replaced that inline command with a real
batch script; focused retest **7537430** passes. The frozen estimator packages,
generator and acceptance rules were unchanged. The same script prepares 400k
successfully in **7537467**. All accounting and failed launcher evidence remain.

### Accepted 100k results

Array **7537447.1–4** passes all scheduler/application/output checks. Its 64
attempts comprise 48 timed calls and 16 warm-ups. Median paired speedups below
exclude warm-ups and compare candidate to baseline FEVC, not Matlab.

| Input | Threads | Observation old → new (seconds) | Observation speedup | Match old → new (seconds) |
|---|---:|---:|---:|---:|
| Degree four, one row/match | 1 | 29.842 → 7.760 | 3.85× | 5.378 → 5.347 |
| Degree four, one row/match | 7 | 26.674 → 4.844 | 5.51× | 1.834 → 1.636 |
| Mixed degree 2–8, two rows/match | 1 | 13.337 → 9.481 | 1.41× | 7.354 → 7.236 |
| Mixed degree 2–8, two rows/match | 7 | 13.760 → 4.814 | 2.84× | 1.974 → 1.966 |

Observation geometric-mean paired runtime ratio is 0.3291; match is 0.9692.
No cell has a median regression. Treat the small match differences as noise,
not as an additional solver optimization: that route was already optimized.
Different thread cells use different hosts, so this is not a controlled
cross-core scaling claim. Maximum scaled corrected-target gap is 1.127e-13
(match gaps exactly zero), maximum complete residual 3.647e-10 against 1e-9,
and maximum physical process-tree RSS 0.527 GiB. RSS is distinct from charged
allocator payload and scheduler virtual memory.

### Accepted 400k results and decision

Array **7537473.1–4** passes all four scheduler records (failed=0/exit=0),
application checks and independent audits on SCC and locally. Its 64 attempts
again comprise 48 timed calls and 16 warm-ups, all successful.

| Mixed-degree input, two rows/match | Threads | Observation old → new (seconds) | Observation speedup | Match old → new (seconds) |
|---|---:|---:|---:|---:|
| Well-mixed | 7 | 60.559 → 17.741 | 3.41× | 7.055 → 7.098 |
| Well-mixed | 28 | 68.015 → 21.206 | 3.22× | 5.717 → 5.420 |
| Segmented | 7 | 64.006 → 22.749 | 2.81× | 11.571 → 11.634 |
| Segmented | 28 | 63.950 → 16.267 | 3.82× | 4.887 → 4.881 |

Speedups are medians of paired ratios, not ratios of the displayed medians.
Observation geometric-mean paired ratio is 0.3019 (69.8% less command time).
Match ratio is 0.9908; the largest paired median regression is only 0.10%.
Maximum scaled corrected-target gap is 7.189e-14 and maximum complete residual
4.140e-10, below 1e-9. Maximum physical process-tree RSS is 1.573 GiB candidate
and 1.598 GiB baseline; scheduler maxvmem reaches 11.367G and is not physical
RAM. Resources were never increased. No subsequent scheduled-job or scientific
failure occurred after the one preparation repair.

The same-host native solve-phase medians fall from 58.715/65.571/62.032/61.956s
to 15.731/17.587/20.661/14.182s in table order. Candidate direct PCG wall time
alone is 4.344/2.906/8.435/2.662s. Thus shared parallel solving materially helps,
but generic observation statistics, RHS assembly, independent certification
and surrounding command work remain substantial. Fusion cannot be assumed to
remove that remaining cost. Retained solver allocations are approximately
46.1/82.3/62.7/101.6 MB, workspace pools 12.1/48.2/12.9/51.8 MB, and admitted
whole-engine peaks 566.1/1162.2/564.2/1185.3 MB in those four candidate cells.
These byte forecasts do not purport to include all Stata process overhead.
Refinement remains enabled: segmented calls refine 143–158 columns out of 601
initial RHSs, in both the new observation route and existing match route.
The 7-thread receipt counts seven refinement batches versus two at 28 threads;
that reflects batching, not fewer refined columns. Well-mixed calls need none.
All iterations, refinement work and residuals are retained with each call.

**Decision: retain the implemented shared unfused solver for both eligible
deletion modes.** The observation gains persist on degree-four and mixed-degree
graphs, repeated-match semantics pass, and match performance is preserved.
Keep fused lanes experimental; do not expand into larger runs, inference,
controls, public weighted routing or a paper campaign automatically. Different
hosts across thread cells mean the 7-versus-28 comparison does not identify a
pure thread-scaling effect. All campaign jobs are terminal; no monitor remains.

## Evidence locations

Durable local development evidence, frozen source snapshots, diagnostic failures,
native artifacts, individual call receipts, full diagnostic matrices, RSS traces
and independently rederived summaries live in
`.local/deletion-symmetry-20260912/` at the repository root. The SCC directory
above preserves original remote evidence. `100k-analysis.json` and
`400k-analysis.json` enumerate every
accepted attempt and its command/phase timings, iterations, refinements,
workspace/retained forecasts and physical RSS. `source-compatibility.json`
records exact binary hashes and the later documentation-only differences.
`collected-audits-match.json` confirms the locally rederived receipts exactly
match all three remote acceptances. Collection emitted nonportable directory
permission warnings; every required content hash and result was independently
verified. The 400k acceptance receipt was fetched separately after its remote
audit completed; the earlier collection began before that file existed.
No fused execution or Matlab comparison is included in this follow-up.
