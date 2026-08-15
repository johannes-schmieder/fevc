# KSS numerical optimization report — 2026-08-15

## Outcome

B1, true lockstep batched diagonal PCG, replaces the scalar matrix-solve loop
in the public JLA path.  The scalar implementation remains as a test-only B0
reference.  A forced CMG adapter is available only under `kss_bc/tests/`.
Automatic CMG routing is not enabled because the complete promotion gates do
not pass.

The accepted synthetic end-to-end SCC ladder is source-bound to commit
`3ac4abe3ee679832943c3d10dd75bece37880b48` under run
`20260815T142657Z-3ac4abe`. The later read-only Separations harness and its
run-local MATLAB CMG loader are recorded under distinct immutable runs; they
do not change the accepted estimator or solver implementation.

No estimator formula, retained sample, target, deletion rule, probe stream,
seed, tolerance, quotient normalization, grounding convention, worker
reconstruction, or complete residual gate changed.  The full fixed-seed Stata
suite still requires batch 1 and batch 17 results to agree within `1e-14`.

## B1 implementation and correctness

Each B1 PCG iteration performs one matrix Schur action for all active RHSs.
Every RHS retains its own recurrence, curvature check, stopping decision,
iteration count, and freshly recomputed full worker-plus-firm residual.
Inactive columns are kept exactly zero.  Every 100 iterations B1 recomputes
the explicit quotient residual; it restarts only when measured drift is
material at the registered tolerance.  Grounding occurs only after quotient
convergence.

The installed command now reports setup, Schur-action, preconditioner-apply,
PCG, leverage-probe, target-probe, and aggregate command timing.  It also
reports RHS-equivalent action counts, physical matrix-batch counts, and an
RHS diagnostic matrix with stage, batch start, RHS, iterations, complete
relative residual, and convergence indicator.

Local gates use Stata 18 with observed `c(flavor)=IC`, seed `20260815`,
tolerance `1e-8`, 10,000 workers, 1,000 firms, and eight RHSs.  Five fresh
process repetitions produced these medians:

| graph | rows | B0 seconds | B1 seconds | B0/B1 | max iterations B0/B1 | max complete residual B0/B1 | coefficient `mreldif` |
|---|---:|---:|---:|---:|---:|---:|---:|
| easy random mobility | 40,000 | 0.065 | 0.022 | 2.95x | 9 / 9 | `1.2861e-9` / `1.2861e-9` | `2.95e-16` |
| moderate circulant | 30,000 | 0.449 | 0.147 | 3.06x | 97 / 97 | `8.7153e-9` / `8.7153e-9` | `1.69e-15` |
| weak ring | 20,000 | 3.146 | 1.168 | 2.69x | 998 / 998 | `8.8984e-9` / `8.8983e-9` | `2.22e-11` |

The weak case initially exposed an unconditional-residual-restart regression
(9,685 instead of 998 iterations).  The registered drift-triggered repair
restored the B0 iteration count before B1 was accepted.

## Shared CMG resource calibration

CMG core API 2 removes the unrelated fixed eight-column graph-action cap.
All requested columns now share a row-chunked graph traversal bounded by the
registered scratch budget.  `options_resource()` converts a declared memory
envelope into explicit caps: action scratch up to 1 GiB, construction scratch
up to 8 GiB, and dense factors up to 512 MiB, while retaining ample caller
headroom.  On graphs with at least 2,048 vertices and at least 4 GiB declared
memory it selects a 256-vertex terminal threshold; the default remains 128.

Calibration on 10,000 path vertices and eight RHSs reduced CMG PCG from 166
iterations / 1.192 seconds at terminal size 128 to 79 iterations / 0.608
seconds at 256.  Raising scratch from 64 MiB to 256 MiB or 1 GiB did not add a
material gain on that case.  A five-repeat 1,000-vertex barbell check found a
6.25% regression at terminal size 256, so small graphs retain 128.  The
preallocated workspace remains slower than ordinary apply and is not selected.

The mathematical hybrid, Galerkin, quotient, pullback, and SPD contracts did
not change.  The standalone core still passes 26 Python tests plus the full
Mata mathematical, namespace, hub, fault, and solver gates.

## Forced C benchmark

The forced adapter loads the generated `kssbc_cmg__*` namespace only in tests.
It preserves the original KSS Schur action, quotient projection, grounding,
worker reconstruction, per-RHS status, and fresh complete residual check.  A
CMG setup or application failure is returned as failure and never falls back.

At eight RHSs, five fresh-process medians were:

| graph | CMG status | setup | B1 PCG | C PCG | PCG speedup | setup-inclusive speedup | iterations B1/C | max residual B1/C |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| easy | `HIERARCHY_EDGE_LIMIT` | 6.689 | 0.022 | — | — | — | 9 / — | `1.2861e-9` / — |
| moderate | `CONVERGED` | 0.132 | 0.151 | 0.075 | 2.03x | 0.737x | 97 / 22 | `8.7153e-9` / `6.7175e-9` |
| weak | `CONVERGED` | 0.072 | 1.207 | 0.098 | 12.25x | 7.11x | 998 / 34 | `8.8983e-9` / `8.7093e-9` |

The easy failure remained typed after experimental reduction thresholds down
to 1%; relaxing the threshold eventually reached the registered hierarchy
complexity cap.  It is recorded as a rejection rather than an unchecked
success.

At 32 moderate RHSs, median B1/C PCG was 0.410/0.179 seconds and
setup-inclusive speedup was 1.33x, so measured break-even occurs by the
router-relevant 32-RHS boundary.  At 200 RHSs:

| graph | setup | B1 PCG | C PCG | PCG speedup | setup-inclusive speedup | iterations B1/C | max residual B1/C |
|---|---:|---:|---:|---:|---:|---:|---:|
| moderate | 0.131 | 2.393 | 0.897 | 2.66x | 2.33x | 98 / 23 | `9.2458e-9` / `7.3400e-9` |
| weak | 0.072 | 20.573 | 1.141 | 18.03x | 16.96x | 998 / 35 | `9.3519e-9` / `9.6226e-9` |

The 200-RHS coefficient `mreldif` values are `2.81e-8` (moderate) and
`2.17e-6` (weak), while every original-system residual passes.  This is
expected sensitivity of coefficient coordinates on the weak system and is not
treated as estimator equality evidence.

## End-to-end Stata 19 forced-C SCC evidence

The source-bound SCC estimator ladder used 10,000 workers, 1,000 firms, 200
probes, seed `8675309`, tolerance `1e-10`, four requested slots, and a 56 GiB
CMG envelope. Stata reported version 19 and flavor IC. Each accepted estimate
has 601 complete original-system RHS residual records: one full fit, 200
leverage probes, and 400 target probes.

| graph | B1 command | C command | C/B1 result | B1/C max iterations | B1/C max complete residual | B1/C peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| easy | 23.871 s | 25.867 s | typed `HIERARCHY_STALLED` | 10 / — | `5.51e-11` / — | 119,236 / 97,580 KiB |
| moderate | 57.902 s | 33.574 s | 1.725x | 90 / 29 | `9.98e-11` / `9.83e-11` | 111,148 / 121,992 KiB |
| weak | 205.287 s | 44.439 s | 4.620x | 500 / 64 | `6.13e-12` / `9.93e-11` | 111,960 / 98,348 KiB |

Moderate B1/C estimator-matrix relative difference was `3.23e-12`; weak was
`2.01e-11`. Dimensions, retained sample, probes, seed, tolerance, target,
formulas, and RNG stream were identical. Easy C posted no estimate.

The measured stage timings were:

| graph/route | setup | Schur actions | preconditioner applications | PCG | leverage probes | target probes | command |
|---|---:|---:|---:|---:|---:|---:|---:|
| moderate B1 | 0.157 | 34.986 | 0.678 | 45.955 | 21.854 | 33.663 | 57.902 |
| moderate C | 0.653 | 10.188 | 7.690 | 21.642 | 13.130 | 18.059 | 33.574 |
| weak B1 | 0.103 | 140.705 | 3.675 | 197.635 | 74.656 | 128.606 | 205.287 |
| weak C | 0.394 | 16.238 | 12.897 | 36.372 | 16.968 | 25.760 | 44.439 |

The six SCC jobs were `7186711`, `7186712`, `7186733`, `7186734`,
`7186766`, and `7186767`. Every job has `failed=0`, `exit_status=0`, and four
slots in its collected `qacct`. Their wall times were 26, 28, 59, 35, 206,
and 45 seconds; CPU times were 90.420, 40.467, 227.856, 132.059, 818.486,
and 176.380 seconds. Complete per-RHS iteration and residual records remain
under each route's `numopt_*_rhs.csv` in the immutable SCC run. Easy C has no
RHS file because hierarchy preparation rejected it before a solve.

## Promotion decision

Automatic routing remains disabled. C passes synthetic estimator equality,
complete-residual, speed, memory, and Stata 19 gates on moderate and weak
graphs, but rejects the easy graph and has no accepted real-data estimate.
The public package therefore ships only B1 and retains diagonal as its sole
preconditioner. No failed C setup or later estimator gate can change a KSS
result or turn a former failure into success.

## Read-only Separations CZ24 wage ladder

The benchmark reads the checksum-bound SCC wage artifact in place and never
edits the Separations checkout. Restricted prepared rows, detailed MATLAB
output, and retained-match files remain only under `/projectnb/welfgr/`.
Commit `8ceace53136a7e49d5628c103565e88d038da046` prepared a genuine 500-worker
dense core with 27,963 rows and 504 firms. It also prepared the larger
all-eligible-mover sample with 292,817 rows, 4,653 workers, and 1,858 firms.
The latter admitted all 4,653 eligible movers, so a nominal 5,000-worker cap
was already the natural full mover target relevant for match-mode headlines.

Both sizes failed the same unchanged estimator gate. On the 500-worker core,
B1 job `7187985` rejected `NONESTIMABLE_DELETION` after 4.416 seconds and
80,688 KiB peak RSS. Forced C job `7187986` successfully built a one-level
hierarchy in 0.083 seconds, then reached the same estimator-level rejection
after 3.107 seconds and 84,804 KiB peak RSS. On the all-mover sample, B1 job
`7187819` reached the rejection after 177.444 seconds and 318,412 KiB peak
RSS; forced C job `7187820` rejected hierarchy construction as
`HIERARCHY_STALLED` after 15.992 seconds and 210,592 KiB peak RSS. No route
posted an estimate or an unchecked residual success. The shorter CMG times
are pre-failure timings and are not estimator speedups.

The maintained MATLAB reference initially exposed two source-environment
issues without changing the read-only project: the CMG subtree was not on the
recursive path, then its SCC MEX files were absent. The harness now binds
`leave_out_KSS.m`, `cmg_sdd.m`, and a canonical SHA-256 over nine hierarchy C
sources, compiles the nine binaries below the KSS run directory, records MEX
setup time, and verifies the run-local `graphprofile` resolution. Each failed
attempt remains preserved. The final bounded loader retry is recorded in the
milestone status when its SCC accounting becomes available.

No further real-data estimator scale-up is authorized. The genuine small
sample did not pass the correctness gate, and the already-measured all-mover
sample failed as well. A natural full input would add stayers that the fixed
match-mode mover target removes; it cannot create a qualifying comparison and
was not submitted.

## SCC portability and smoke

The Stata 19 portability and B1 smoke stages pass scheduler, application, and
structured-output validation. Both invoked `stata-mp`; Stata reported version
`19` and flavor `IC`. Their SCC accounting was:

| job | SCC ID | wall | qacct CPU | peak RSS | failed / exit |
|---|---:|---:|---:|---:|---:|
| portability | 7185628 | 14 s | 45.520 s | 79,956 KB | 0 / 0 |
| smoke | 7185639 | 14 s | 48.268 s | 85,568 KB | 0 / 0 |
| medium | 7185654 | 1,580 s | 6,278.181 s | 432,284 KB | 0 / 0 |

The 5,000-worker, 250-firm, 40-probe smoke command used 0.017 seconds for data
preparation and 12.268 seconds for `kss_bc`, or 12.285 seconds total. Within
the command it recorded 0.816 seconds for graph work, 1.198 for the full fit,
0.100 for setup, 7.746 for Schur actions, 0.054 for preconditioner
applications, 8.745 for PCG, 3.804 for leverage probes, 5.819 for target
probes, and 9.623 for the combined correction phase. Nested phases are not
additive.

All 123 recorded RHSs converged. Stage counts were 2 setup RHSs, 1 full-fit
RHS, 40 leverage RHSs, and 80 target RHSs. Each stage had maximum 125 PCG
iterations; the overall maximum freshly recomputed complete residual was
`2.43329288259e-11`. Logical/physical accounting recorded 15,498 Schur
actions in 1,512 matrix batches and 15,375 diagonal applications in 1,500
matrix batches.

The 50,000-worker, 2,500-firm, 100-probe medium command completed in
1,578.789 seconds, versus 3,868.368 seconds for scalar B0 on the same design:
a 2.45x end-to-end speedup. Peak RSS was essentially unchanged at 432,284 KB
versus 430,956 KB. Medium reported 3.654 seconds graph, 40.501 full fit, 0.675
setup, 1,430.920 Schur actions, 5.881 preconditioner applications, 1,544.700
PCG, 602.393 leverage probes, 929.558 target probes, 1,531.951 correction, and
1,578.838 total including data preparation. All 303 RHSs converged; the
maximum was 1,250 iterations and maximum freshly recomputed complete residual
was `4.92602995941e-9`.

The medium design has 2,500 firms and 1,250 maximum iterations. The proposed
large design has 10,000 firms, five times the rows, and twice the probes. A
conservative work projection using the observed linear iteration growth is
about 40 medium workloads, or 17.5 hours. That is not confidence of a runtime
well below 12 hours. No B1 large or other 12-hour-or-longer job was submitted.

The next SCC action is to keep monitoring and eventually collect unchanged B0
job `7185180`. A B1 large job remains unjustified without another algorithmic
improvement or a smaller intermediate calibration.
