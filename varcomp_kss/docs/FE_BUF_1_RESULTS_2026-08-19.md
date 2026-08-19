# FE-BUF-1 implementation and results

Date: 2026-08-19

Status: implemented and regression-qualified; retained as a safe cumulative
optimization, not a universal speedup or MATLAB-parity claim

Measurement baseline: `7f5a38b49cccd5bf0396f62fedd542881d68bad1`

Runtime candidate: `1cb441f20be0d747483cd8f81746af4187192d0f`

## Outcome

FE-BUF-1 removes repeated large destination allocations from the compressed
fixed-effect Schur operator. The archive-isolated local P200 fixture improves
complete-command time by `2.30%` in baseline-first order and `1.42%` in
candidate-first order. The directly affected Schur work improves by `8.61%`
and `7.65%`, while PCG improves by `4.31%` and `2.94%`. Exact scientific,
structural, solver-work, residual, route, sample, caller-state, and RNG
comparisons pass in both orders.

The same-host synthetic SCC ladder shows that the optimization remains useful
at larger coefficient dimensions. From F256 through F15625, median complete-
command reductions range from `3.29%` to `6.85%`. The separately labeled,
cold, single-repetition F15625 endpoint reduces complete-command time by
`5.70%`, Schur time by `7.29%`, and PCG time by `6.28%`. It uses one timed
repetition per role because an empirical F8192 extrapolation showed that three
repetitions per role could not fit the original two-hour scheduler envelope.

The fixed 8,201,888-row CZ18 P20 holdout is deliberately less favorable to
this optimization. Across three repetitions in each AB/BA order, the median
complete-command change is `0.0%` overall and `-0.35%` after excluding the
first repetition in each process. Median Schur time improves `1.36%`, while
PCG changes `+0.68%` and total numerical work changes `+0.28%`. These are
neutral, noise-sized changes, not evidence of a real-data command speedup.
The candidate nevertheless uses the new path in every CZ18 call, has zero
fallback batches, and avoids about 2.58 GB of modeled cumulative cell-buffer
allocation volume per call with a bounded 112.6 MB workspace.

The code is retained. It has no scientific regression, improves the targeted
kernel and multiple large synthetic command paths, is neutral on CZ18, and
reduces allocator pressure even where wall-clock savings are too small to
resolve. The package remains entirely Stata/Mata at runtime.

## Implementation

The measurement commit introduced `FE-BUF-PERF-V1` without changing the
operator. It records workspace builds, buffered and legacy Schur batches and
columns, packed fallbacks, maximum width, workspace bytes, and modeled
cumulative cell-materialization bytes avoided. These counters do not
participate in route, batch, convergence, scientific acceptance, or RNG
decisions.

The candidate adds a solve-local fixed-effect workspace with preallocated
cell and worker destination matrices. While all right-hand sides remain
active, the Schur action writes into these buffers in place. If asynchronous
convergence creates a packed active subset, execution falls back to the
existing packed operator rather than changing logical RHS order or stale-
column behavior. Every solve still receives complete original-system residual
certification.

The resource model charges `8 * width * (cells + workers + firms)` bytes
inside the existing phase-scratch envelope. Core API 21, compressed-scale API
5, and resource API 10 use new semantic build identifiers; the scale-engine
and lifecycle APIs retain their numerical levels with new FE-BUF-1 build
identities. No public option, estimator status, result shape, tolerance,
random-atom domain, probe count, or failure contract changed.

## Local causal comparison

The local harness archives the baseline and candidate commits separately,
uses the same tracked driver, and launches every role in a fresh Stata/MP
process. The fixture has 60,000 stored rows, 30,000 coefficient cells, 10,000
workers, 1,000 firms, 200 probes, batch 16, and four processors. Each role has
one cold and three warm runs; the table reports warm medians.

| Stage | Baseline first | Candidate first |
|---|---:|---:|
| Complete command | -2.303% | -1.422% |
| Lifecycle work | -2.583% | -1.602% |
| Schur action | -8.605% | -7.650% |
| PCG | -4.309% | -2.937% |
| Leverage | -2.459% | -1.726% |
| Target | -2.699% | -1.602% |
| RNG | +0.143% | +0.197% |

The RNG changes are noise-sized and lie outside the optimized path. Exact
results, result-matrix dimensions, route and solver-work counts, complete
residuals, sample signatures, and caller RNG state match. Both orderings use
the same Stata executable with SHA-256
`c09d0a0d7717f862535b8c50982d6ab67586c5d91815ce0160823119546afefb`.

## Same-host synthetic scale ladder

Each AB or BA job runs baseline and candidate sequentially on one SCC host.
F256 through F8192 use one cold and two warm timed repetitions per role.
The table reports the median of the AB and BA warm-percentage changes. The two
orders are separate jobs and can still share node load, so the order-specific
receipts remain authoritative.

| Firms | Stored rows | Command | Work | Schur | PCG | RNG |
|---:|---:|---:|---:|---:|---:|---:|
| 256 | 15,360 | -6.85% | -7.15% | -19.56% | -12.22% | -0.56% |
| 1,024 | 61,440 | -3.29% | -3.56% | -8.20% | -5.29% | +0.50% |
| 4,096 | 245,760 | -4.54% | -4.67% | -8.34% | -5.64% | -0.30% |
| 8,192 | 491,520 | -5.65% | -5.73% | -9.12% | -6.54% | -0.00% |
| 15,625 | 937,500 | -5.70% | -5.78% | -7.29% | -6.28% | +0.80% |

Every completed pair has exact scientific, structural, result, route, solver-
work, and residual fields. F8192 buffers 191,299 cumulative Schur columns,
uses an 85,983,232-byte peak workspace, and reports 376,109,137,920 modeled
bytes of repeated cell materialization avoided. The latter is cumulative
allocation volume, not resident memory. A small legacy-column count remains
when asynchronous convergence requires packed active columns; this is the
deliberate correctness fallback.

F15625 is a cold, single-repetition extrapolation check, not a warm precision
estimate. In AB order, job 7229208 on `scc-gd4` takes 6,131 seconds and reduces
complete-command time by `7.66%`. In BA order, replacement job 7230511 on
`scc-ei3` takes 6,064 seconds and reduces it by `3.74%`. Both finish with
`failed=0`, `exit_status=0`, exact comparison records, and pair markers. The
candidate builds 33 workspaces, buffers 12,260 Schur batches and 287,515
cumulative columns, uses the correctness fallback for 101 batches and 1,415
columns after asynchronous convergence, and has a 164,000,000-byte peak
workspace. Its modeled avoided cell-materialization volume is 1.078 TB; as
above, this is cumulative allocation volume rather than resident memory.

The first three-repetition F15625 tasks, 7228370 and 7228371, were canceled
before a complete role receipt after the F8192 wall projection showed that the
requested work could not fit the two-hour pair envelope. The first one-repeat
BA task, 7229209, ran on the slower `scc-pi2` host. Its candidate half finished,
but the scheduler killed the baseline half at the originally submitted 7,200-
second limit (`failed=100`, `exit_status=137`). No timing from that incomplete
pair is accepted. The replacement was submitted from the outset with a four-
hour limit and supplies the accepted BA receipt.

## CZ18 holdout

The real-data gate uses the frozen retained input with SHA-256
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`:
8,201,888 rows, P20, seed 8675309, automatic batch 16, compressed CMG, 56 GiB
reserved memory, and four actual Stata processors. Jobs 7229227 and 7229228
run three fresh-process repetitions in AB and BA order. Both finish with
`failed=0`, `exit_status=0`, all twelve role receipts, and both pair markers.

| CZ18 median change | All repetitions | Warm repetitions |
|---|---:|---:|
| Complete command | +0.000% | -0.351% |
| Numerical work | +0.284% | +0.284% |
| Schur action | -1.042% | -1.360% |
| PCG | +0.676% | +0.676% |
| Selection | -0.220% | -0.317% |
| Leverage | +1.415% | +1.328% |
| Target | -0.714% | -0.714% |
| RNG | +0.312% | +0.312% |

All six matched role pairs preserve exact result cells, structural fields,
solver diagnostics, sample, lifecycle, data order and metadata, and caller
RNG/sort state. Each candidate call builds five workspaces, buffers 84 Schur
batches and 1,036 cumulative columns, and uses no fallback.

Two earlier CZ18 wrapper revisions are rejected but preserved. Jobs
7229180--7229181 stopped because the harness did not yet admit the registered
compressed success status. Jobs 7229192--7229193 stopped because the harness
expected the wrong registered `e(results)` row count. In all four cases the
estimator itself completed, but no timing is accepted from a wrapper that did
not pass its full contract.

## Why the gain differs across designs

FE-BUF-1 attacks repeated destination allocation inside Schur actions. The
local P200 fixture and the synthetic P256 ladder exercise hundreds of RHSs and
many cumulative Schur columns, so the targeted operation is material. CZ18
P20 has only 61 logical RHSs; its candidate records just 1,036 cumulative
buffered columns, and Schur time is roughly six seconds out of a 283--303
second command. Eliminating allocation from a two-percent stage cannot
reliably move the complete command when requested-sample and selection work
alone takes about 175--188 seconds.

This is why the synthetic large-dimension result and the CZ18 result are not
contradictory. Dataset row count is not the relevant exposure by itself. The
benefit scales with repeated-RHS width, iteration count, coefficient-cell
size, and the fraction of the command spent in Schur actions. The counter
receipts, rather than row count alone, determine where the optimization pays.

## Regression gates

- The full Python suite passes 363 tests.
- Deterministic generated-CMG drift and independent CMG Python/Mata gates
  pass.
- KSS quick and full Stata suites, clean `net install`, namespace/stale-
  runtime, benchmark, separation, and MATLAB-bridge gates pass, ending with
  `VARCOMP_KSS LOCAL QUALIFICATION PASS`.
- Archive-isolated local AB/BA comparisons pass exact science.
- Accepted SCC pairs require clean qacct, terminal wrapper and Stata markers,
  exact source/driver hashes, pair markers, and registered scientific and
  state comparisons.
- Ruff, the 1,093-file frozen legacy-name audit, and `git diff --check` pass.

The known pytest cleanup warning comes from the intentional remote-symlink
rejection fixture and does not touch package or qualification data.

## Best next performance target

The next highest-value idea is command-boundary preparation consolidation,
not another Schur allocation micro-optimization. On CZ18, selection consumes
about 62% of the complete command, while Schur consumes about 2%. The active
Ado path still performs multiple requested-sample scans, worker/firm grouping
passes, graph pruning/redensification, semantic grouping and sorting, and the
compressed-state handoff before numerical work.

A separately measured `PREP-BND-1` candidate should build one command-local
preparation context that returns the final active mask, dense worker/firm and
deletion maps, semantic order, graph vectors, and compressed numeric columns
without repeated full-column imports or redundant sorts. Stata should remain
the semantic oracle for arbitrary string IDs, factor variables, and canonical
sort behavior until exact cross-type fixtures prove a narrower Mata path.
There must be no invisible cross-command cache or public lifecycle change.

Implementation should proceed from small to large:

1. split selection into exclusive scan, grouping, graph-prune, redensify,
   semantic-order, and compressed-handoff timers and counters;
2. add a numeric-only command-local context behind exact equality tests;
3. run local no-control match fixtures and adversarial ID/string/order tests;
4. run matched F256/F1024 AB/BA timing before any larger job;
5. promote to F4096/F8192 only after extrapolated resource and wall checks;
6. rerun the fixed CZ18 P20 holdout in both source orders; and
7. retain any safe cumulative gain even if it misses the advisory target,
   while rejecting any sample, semantics, result, route, residual, failure,
   memory, data-state, or RNG regression.

The aspirational target remains a 35% reduction in mark-through-compression
and a 1.25x CZ18 complete-command improvement, but it is a guide rather than a
minimum retention rule. The FE-BUF-1 evidence reinforces that many safe,
independently qualified improvements will be needed to approach MATLAB's
timing.
