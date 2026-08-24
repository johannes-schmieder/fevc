# PREP-RHS-1 implementation and results

Date: 2026-08-19

Status: implemented, retained, and regression-qualified as a moderate
one-shot improvement; not a MATLAB-parity claim

Baseline: `73ea94a9bfb69a2839ef66f9d9b79c219016abbe`

Runtime candidate: `95d3950c4cfea669c2a244ad98f6fed03ae8ca19`

## Outcome

PREP-RHS-1 produces a repeatable `2.7–3.1%` complete-command improvement on
the archive-isolated local P200 fixture. The directly affected correction
stage improves by about `45%`, leverage by `3.4–3.6%`, target by about `2%`,
and the total lifecycle work phase by `2.9–3.3%`. The gains are smaller than
the milestone's aspirational 1.25x complete-command target, but they are safe,
repeat across source order, and accumulate in the intended large-data path.
They are therefore retained.

No estimator option, probe count, tolerance, random-atom order, deletion or
target definition, route decision, residual certificate, typed failure,
caller-state contract, or public command changed. The package remains pure
Stata/Mata at runtime.

## Implemented changes

### Measurement

- Added diagnostic-only `PREP-RHS-PERF-V1` returns: `e(prep_profile)`,
  `e(rhs_profile)`, and `e(work_counters)`.
- Separated semantic ordering from graph pruning and compression preparation;
  nested correction and RHS timers are labeled rather than double-counted.
- Added fixed-shape plan, scatter, batch, logical-action, and physical-action
  counters. No timer or observed performance value participates in routing,
  admission, scientific acceptance, or RNG decisions.

### Exact-output plan reuse and preparation

- Retained deletion-unit-to-cell and target-stratum-to-cell order/panel plans
  inside the compressed design and reused them for all batch reductions and
  aggregate-integrity checks.
- Removed the duplicate generated KSS scatter artifact and kept a single
  planned destination-buffer reducer.
- Imported graph worker/firm/frequency/deletion columns in one Mata transfer
  and imported the seven compressed numerical columns in one transfer.
- Kept Stata grouping and canonical sort behavior as the semantic oracle.
- Added the retained-plan bytes to the direct resource model rather than
  hiding the extra residency.

### RHS and dataflow

- Added deterministic active-column packing for zero or converged RHSs while
  preserving the original full-width path when every column is active.
- Restored every result to its original logical column and retained complete
  original worker-plus-firm residual certification per RHS.
- Added interleaved zero-column/counting-backend tests for active width,
  stale-column rejection, exact solutions, and logical order.
- Vectorized all deletion-unit adjustments while preserving the scalar
  routine as a test oracle and retaining exact first-failure ordering.
- Removed two audited dead fitted-value gathers that were immediately
  overwritten by certified predictions.

Runtime identifiers were bumped to core API 20, graph API 20, scale API 4,
scale-engine API 3, and resource API 9. The public development version remains
`0.3.0-dev`.

## Local causal comparison

The local harness archives baseline and candidate commits separately, uses an
identical driver, and launches each role in a fresh Stata/MP process. The
fixture has 60,000 stored rows, 30,000 coefficient cells, 10,000 workers,
1,000 firms, 200 probes, batch 16, and four processors. Each role has one cold
run and three warm runs; the table reports warm medians.

| Stage | Baseline-first | Candidate-first |
|---|---:|---:|
| Complete command | -2.697% | -3.110% |
| Lifecycle work | -2.937% | -3.344% |
| Leverage | -3.635% | -3.409% |
| Target | -2.002% | -2.062% |
| Correction | -45.789% | -45.213% |
| Schur action | -0.795% | -1.274% |
| PCG | +0.420% | -0.222% |
| Preconditioner apply | +2.045% | -1.660% |
| Selection | +0.344% | +0.348% |
| Compression preparation | -0.369% | -0.373% |

The sign reversal in the small PCG/preconditioner changes is consistent with
timing noise. The correction, leverage, target, work, and complete-command
gains are stable under source-order reversal. Scientific and structural CSV
fields, route, solver work counts, maximum residual, every result cell, and
the estimator identities match exactly in both runs.

Retained plans add bounded memory: the baseline-first median modeled peak is
311,982,834 bytes versus 313,507,599 bytes (`+0.489%`), while measured work
memory changes from 75,399,272 to 75,456,481 bytes (`+0.076%`). The resource
model includes this cost. It is not an unreported cache.

## SCC scale and topology validation

The SCC sequence began with F64 smoke jobs, then F256/F1024 pilots, F1024
P200 batch-width cells, F4096 topology/row-density cells, and only then F8192
holdouts. Every task used one fresh Stata process and the source-bound
`KSS-NUMOPT-2` generator, wrapper, and validator.

The final evidence contains 67 validated baseline jobs and 68 validated
candidate jobs: 67 paired experiment IDs across 23 scenarios plus one
exploratory candidate smoke. All 135 jobs have clean qacct, wrapper and
application markers, expected dimensions, accepted routes, result identities,
and complete RHS residuals. Across the 67 pairs:

- all registered structural fields match exactly;
- the largest scientific relative difference is
  `1.0206906706043713e-14`, below the `2e-13` cross-node roundoff gate;
- no result, resource, scheduler, timeout, or typed estimator failure occurs;
  and
- all raw receipt/log/diagnostic hashes are recorded in
  `qualification/prep_rhs1/scc_summary.json`.

Only 9 of the 67 pairs landed on the same host, and even those jobs did not
share a controlled load. The SCC timing comparison is therefore explicitly
`HOST_CONFOUNDED`. The following candidate medians describe scale and resource
behavior; they are not baseline/candidate speed estimates.

| Candidate scenario | Command | Work | PCG | Modeled peak |
|---|---:|---:|---:|---:|
| F1024, d3, P200, batch 8 | 113 s | 103.672 s | 46.660 s | 1.365 GiB |
| F1024, d3, P200, batch 16 | 123 s | 113.021 s | 50.076 s | 1.447 GiB |
| F1024, d3, P200, batch 32 | 120 s | 109.844 s | 45.906 s | 1.613 GiB |
| F1024, d3, P200, batch 64 | 127 s | 117.638 s | 49.920 s | 1.944 GiB |
| F4096, d3, strong, rows/cell 1 | 112 s | 82.983 s | 51.892 s | 2.212 GiB |
| F4096, d3, strong, rows/cell 8 | 182 s | 57.312 s | 29.731 s | 3.157 GiB |
| F4096, d7, strong, rows/cell 1 | 319 s | 241.703 s | 156.515 s | 3.460 GiB |
| F8192, d3, strong, rows/cell 1 | 226 s | 165.142 s | 100.534 s | 3.222 GiB |
| F8192, d3, strong, rows/cell 8 | 459 s | 138.677 s | 69.583 s | 5.024 GiB |
| F8192, d7, strong, rows/cell 1 | 313 s | 252.966 s | 177.955 s | 5.720 GiB |

The largest stored-row case has 7,864,320 rows, 327,680 workers, 8,192 firms,
and 983,040 coefficient cells. The degree-seven F8192 case represents
18,350,080 physical observations and 2,293,760 cells. The largest candidate
modeled peak is 5.720 GiB; the largest qacct maximum VM is 5.797 GiB. The
slowest candidate wall time is 755 seconds. All remain below the deliberately
conservative 16-GiB and 30-minute holdout envelope.

The P200 batch cells show the expected memory increase with width, but their
uncontrolled hosts do not justify changing the deterministic batch policy.
No live timing is used to select batch or route.

## Regression gates

- Full integrated local gate passed after the runtime change: 357 Python
  tests, generated-CMG drift, CMG Mata, KSS quick/full, clean install,
  namespace/stale-runtime, resource, benchmark, separations, and MATLAB bridge.
- After adding the benchmark/evidence tooling, the full Python suite passes
  363 tests.
- Both archive-isolated local comparisons pass exact science and residual
  checks.
- All 135 SCC jobs pass the source-bound validator.
- Ruff, the 1,093-file frozen legacy-name audit, and `git diff --check` pass.

Pytest emits a known sandbox cleanup warning for an intentional remote-symlink
rejection fixture; all tests pass and the warning does not touch package data.

## Interpretation and next work

The retained scatter plans and vector adjustment are the successful part of
this round. Bulk imports are architecturally useful for very large data but
are not a decisive cost on the 60,000-row local fixture. Active packing is
fully tested, while the ordinary benchmark has equal logical and physical
work and therefore does not claim a packing speedup.

This candidate does **not** claim the milestone's aspirational 35% preparation
reduction or 1.25x complete-command gain. Those targets remain useful guides,
not reasons to discard the verified 3% cumulative improvement.

A general caller-owned FE solve workspace was not promoted in this candidate.
Mata's matrix-return boundaries require a separately measured destination-
buffer design to avoid merely moving allocations, and the existing CMG
workspace remains disabled because prior evidence shows regressions. There is
no invisible cross-command cache and no public prepare/run/drop lifecycle.

The large degree-seven results do show that PCG/preconditioner work remains a
leading cost at the topology boundary. The next independently killable work
should be:

1. a command-local FE destination-buffer/workspace prototype with explicit
   allocation counters and unchanged full residual certification;
2. a host-controlled SCC timing design if causal large-case speed estimates
   are needed;
3. a new measured-width flat CMG arena only for degree-seven cells, never the
   previously regressive workspace; and
4. an F15625 confirmation only after extrapolating from a host-controlled
   F8192 run, with at least 32 GiB and a one-hour wall envelope.

The implementation, evidence, and this report make no numerical-equivalence
claim with MATLAB. They reduce one part of the measured gap while preserving
the Stata/Mata estimator contract.
