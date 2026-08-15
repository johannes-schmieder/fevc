# KSS numerical optimization report — 2026-08-15

## Outcome

B1, true lockstep batched diagonal PCG, replaces the scalar matrix-solve loop
in the public JLA path.  The scalar implementation remains as a test-only B0
reference.  A forced CMG adapter is available only under `kss_bc/tests/`.
Automatic CMG routing is not enabled because the complete promotion gates do
not pass.

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

## Promotion decision

Automatic routing remains disabled.  C shows a valuable weak/moderate
repeated-RHS workload, but the required end-to-end forced-estimator, Stata 19,
and comparative RSS gates are not yet available, and forced setup rejects the
easy expander.  The public package therefore ships only B1 and retains
diagonal as its sole preconditioner.  No failed C setup can change a KSS
result or turn a former failure into success.

The next SCC step is a source-bound short portability/smoke run.  A medium or
large B1 job requires inspection of that accounting first.  No additional
12-hour-or-longer job is justified by the local evidence alone.
