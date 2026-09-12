# Thread-aware batching and ordered scalar queue — 2026-09-12

## Decision and scope

The owner approved integrating batching plus queue and committing/pushing
the source. The baseline is `82be1822758db3ae8d057323a2181fac8944f4eb`.
The accepted mixed-degree candidate is adopted; fused lanes stay experimental.
No public API, ABI layout, estimator definition, tolerance, RNG stream,
upstream CMG pin, paper figure or release artifact is changed.

For permitted threads `T` and probes `P`, full CMG now selects
`min(P, max(32, 8*T))` leverage probes and
`min(P, max(32, 4*T))` target probes per batch. Each target probe has two RHSs.
The internal `k=2` choice was frozen before the accepted end-to-end comparisons;
it is not a public tuning option. Other solver routes retain their prior widths.
At 28 threads and 200 probes this means widths 200/112 and capacity 224 RHSs,
instead of 32/32 and capacity 64. It does not mean 224 simultaneous workers:
the scalar workspace pool remains bounded by permitted threads and RHS count.

Admitted scalar workers draw the next RHS from an ordered queue, removing the
old barriers between waves of RHSs. Each solve keeps its scalar arithmetic,
warm-start refinement and residual checks. Results and failures are returned
in input order. Cancellation stops further work and allows subsequent reuse.

Explicit budgets can reduce automatic widths before allocation and RNG.
Omitted memory does not activate memory-based planning; the existing
warn/error/off policies and full-CMG explicit-batch rejection remain.
Both capacity arithmetic and new queue/pool allocations are checked/fallible.
The integration additionally charges queue result slots and the ordered result
list before allocating them. Widths are frozen before pool allocation and
retained-memory reconciliation; allocation never reselects a numerical policy.

## Accepted experimental evidence reused

These are prior isolated, same-host, complete-command measurements, not a new
performance campaign on the integrated package. Inputs, seeds, probes and
thread allocations were matched; measured order was rotated. Exact source
snapshots and successful/failed attempts are preserved under
`rust/experiments/mixed_degree_20260912/` in the local working artifact area.
They are not shipped as ordinary source. The promoted source was compared to
the hashes in `scc-native-rss-repair/qualifications/batch-queue.json`.

| Mixed-degree 2–8 case | Baseline median | Batching + queue median | Median paired reduction |
| --- | ---: | ---: | ---: |
| 1.6m, well-mixed, 5 pairs | 37.808 s | 32.117 s | 14.5% |
| 1.6m, segmented, 5 pairs | 74.930 s | 43.981 s | 40.5% |
| 6.4m, well-mixed, 3 pairs | 267.677 s | 215.471 s | 18.4% |
| 6.4m, segmented, 3 pairs | 394.487 s | 266.866 s | 32.4% |

The aggregate paired runtime reduction was 29.2% at 1.6m and 25.9% at 6.4m.
All 24 native 1.6m calls and 16 native 6.4m calls passed, including separately
recorded warm-ups. Corrected-target gaps were zero and paired iterations
matched. The 6.4m maximum original-system residual was 4.379939e-6, below the
unchanged 1e-5 gate; no 6.4m refinement was needed. These are selected graph
results, not a universal speed guarantee. Median paired reductions need not
equal the ratio of the two marginal medians.

Wider batches cost memory: accepted 6.4m native peak physical RSS was about
24.684 GiB for batching + queue versus about 14 GiB for the baseline. The
retained CMG scalar workspace pool stayed about 1.51 GiB; the main growth was
batch storage, not additional simultaneous workers. Forecasts are not RSS.

The earlier 400k confirmation passed all 156 attempted calls and reduced
geometric-mean runtime 12.7% across eight graph/thread cells and 15.4% on the
mixed-degree subset. Fused execution was 5.6% slower than batching + queue
on that subset, including a 20.8% regression on well-mixed at 28 threads.
That evidence supports keeping fusion experimental, not enabling it by default.

Separate descriptive Matlab comparisons found median paired speedups of
2.84x/2.64x at 1.6m and 2.05x/1.79x at 6.4m (well-mixed/segmented).
They are not incremental gains over FEVC. The Matlab comparison retains its
documented finite-projection coefficient/RNG differences; it is not a
finite-probe corrected-target equivalence test.

Preserved compact source evidence:

- `CONFIRMATION_400K.md`, `LARGE_1_6M.md`,
  `SCALE_UP_AND_MATLAB_2026-09-12.md` and `scale-completion-metrics.json`.
- `MATLAB_6_4M_RETRY_2026-09-12.md`, SHA-256
  `09339b4eeba6d1527c33d0724df07ec44c0433fc932db2d3cfccc24e5b74596d`.
- `matlab-xl-retry-metrics.json`, SHA-256
  `4c694acee5690dc8df957ba3be981eea697b662fbcc1cb42febea76448ade736`.
- Native jobs 7535470/7535511; Matlab jobs 7535552/7536349.
  The original Matlab wrapper failure 7536310 remains preserved and was not
  counted as successful timing evidence.

## Integration differences and checks

The snapshot's experimental module names are replaced by private production
names. Solver/batch policy is unchanged. Additional integration hardening
charges both queue metadata lists, makes workspace growth fallible, reserves
the result list before numerical work, and checks probe-count conversion.
The macOS qualifier now includes vendored CMG runtime files in its source
manifest; a Python regression protects that provenance boundary.

Tests exercise threads 1, 2, 3, 4, 7, 14, 28 and 64; odd/zero work; mixed
worker degrees 2–8; weighted inputs; ordered failures; slow RHSs; cancellation
and reuse; corrected-target equality; counter/sample identities; overflow;
exact budget boundaries and retained-memory reconciliation. Unit coverage at
64 threads is not a 64-core performance claim. Public Stata tests check actual
small-probe capacity, automatic widths, residuals and caller-state restoration.

Rust workspace tests pass: 554 successful tests and one existing registered
distribution diagnostic intentionally ignored, not newly skipped. Strict
Clippy, formatting, CMG assembly, all 805 Python tests and the additional
28-test CMG component gate pass. The complete Mac native qualification passes
at clean commit `0ec6f3fe1ff1a99208a3dd4dc2e9f9a60509703e`: arm64, Rosetta,
signed thin/universal binaries, public routes, C/ABI contracts and isolated
clean installs. It finished in 358.104 seconds. The standard plugin-build
receipt at `.ci/stata/results/0ec6f3fe1ff1a99208a3dd4dc2e9f9a60509703e.json`
is independently validated for that exact SHA.

The qualifier staged its tested Mac binaries in the local package; their
hashes were independently rechecked. Earlier degree-four binaries remain
intact and hash-matched to their original receipt under the saved
`performance_20260911/qualification/artifacts/candidates/` evidence.
Mac source manifest: 181 files, SHA-256
`4e415807cfea92c9e1493c5d1fba7f42de572a269653c325e4ab5b373c96b06a`.
Mac qualification receipt SHA-256:
`d015214092e6ea0884e2411c0c7dab738d28066be98faf6c2caf20a96b1182f1`.

Linux native/full/install job **7536791** passes all three layers: scheduler
`failed=0`, `exit_status=0`; application PASS; and validated source/binary/output
hashes. It ran on `scc-gd4` in `econ`, selected without queue/host restrictions,
using four slots at 4G/core and a two-hour cap. Runtime was 1,106 seconds;
scheduler peak virtual memory was 9.398G, not a physical-RSS measurement.
The qualifier's full Stata suite and isolated clean install pass under Stata
MP 19. All 21 collected evidence files match their remote hashes. macOS
rejected copied SCC directory metadata; file contents transferred correctly,
and the remaining log was collected as an individual file. No job retry,
resource increase or scientific repair was needed on SCC.

The local integrated gate also passes: source/identity/history/license audits,
805 Python tests, CMG component checks, Stata quick/full suites, clean install,
benchmark and bridge smokes. Its supervised process exited zero without
timeout after **1,017.982 seconds**. Final independent validation rechecked
all 181 native source files, the three installed Mac binaries, Linux candidate,
exact-SHA CI receipt, scheduler record and application markers. The machine
record is [batch_queue_v1_result.json](batch_queue_v1_result.json).

Linux receipt SHA-256:
`d7b36222414799f68efa2ea2aa6f795ad82db506b2a0d9a6a1fe8cf415cc38d3`.
Linux candidate SHA-256:
`e4498c6534c053272836a2201543475d1c775e58f206dbddaaeb1357ac96ec42`.
Local independent validation SHA-256:
`2dd5761c9835794b2c6585d0ed5862339a4dfdd72cd5fd54c9e85ee30150a404`.
Logs and sanitized native artifacts live in `.local/bq-integration-20260912/`;
the SCC source and evidence remain under
`/projectnb/welfgr/vckss/runs/20260912T172500Z-bq-integration-0ec6f3f`.
No qualification jobs or continuation monitor remain. These are development
qualifications, not release artifacts or a new performance campaign.

The exact native-tested commit is `0ec6f3f`, not the later documentation
checkpoint. Post-qualification changes are this report/result, PLAN/changelog/
index, the standard CI receipt, and `fevc/tools/license_audit.py` plus its
three Python regressions. The latter were included in the passing integrated
gate and do not enter native builds. Native/public estimator sources, build
inputs, ABI, binaries, benchmark inputs and acceptance rules remain unchanged;
native and accepted performance evidence are reused only within those bounds.

During development, an old test expected a forecast excluding queue metadata;
its expected charge was corrected and the full Rust suite then passed. The
first Python source-inventory run encountered saved experimental installation
symlinks; the precise local experiment directories are now ignored, without
deleting evidence, and all 800 tests then passed. The new provenance regression
also passes. The owner's unrelated local `KSS_Veneto_replication/` checkout is
preserved and temporarily excluded from the source qualification inventory;
that local-only exclusion is removed again at handoff.

The first native qualification at source `e4186dc` stopped at the public
full-CMG receipt check: the integration had omitted the accepted snapshot's
Stata wrapper change and still required exactly 64 RHS slots. The estimate
itself completed; trace evidence identifies the stale capacity assertion.
The narrow repair restores the snapshot's exact cross-check against selected
leverage/target widths, with a Python regression. No solver or tolerance
changes were made. The failed transcript and unsubmitted SCC source bundle
are retained; fresh native qualification must use the repaired source.
The focused retest passed the original estimate and capacity assertions, then
identified a test-setup mistake in the added 65-probe case: it omitted the
qualified full-CMG routing tuple. That test now specifies the same explicit
tuple as the original case, while still omitting the memory budget. This is
a test correction, not a widening of automatic solver selection.

The first integrated package run stopped before Stata at the license audit:
that audit recursively inspected ignored experimental dependency snapshots.
It now uses the same tracked-plus-new source inventory as the other repository
audits; Git-less source distributions still inspect every Cargo manifest.
Three regressions cover new/tracked manifests, source distributions and a
fail-closed Git error. No license condition or tracked-source coverage was
removed, and the ignored experimental files were not modified. The corrected
audit and full 805-test Python suite pass. This is test tooling, not a new
native source or performance candidate.

No fused port, upstream optional kernel, CMG branch update or timing-based
routing is integrated. Windows qualification and public binary distribution
remain separate outstanding work; historical receipts do not qualify this
runtime automatically.
