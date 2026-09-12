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

Rust workspace tests, strict Clippy, formatting, CMG assembly and Python tests
pass. One existing registered distribution diagnostic remains intentionally
ignored, not newly skipped. Fresh Mac/Linux native and integrated package
qualification are pending; this record will be updated with exact source and
binary receipts before final handoff. Logs live in
`.local/bq-integration-20260912/` and are not release artifacts.

During development, an old test expected a forecast excluding queue metadata;
its expected charge was corrected and the full Rust suite then passed. The
first Python source-inventory run encountered saved experimental installation
symlinks; the precise local experiment directories are now ignored, without
deleting evidence, and all 800 tests then passed. The new provenance regression
also passes. The owner's unrelated local `KSS_Veneto_replication/` checkout is
preserved and locally excluded from the source qualification inventory.

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

No fused port, upstream optional kernel, CMG branch update or timing-based
routing is integrated. Windows qualification and public binary distribution
remain separate outstanding work; historical receipts do not qualify this
runtime automatically.
