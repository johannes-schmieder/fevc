# CMG6 local workspace and solver evidence

## Scope and evidence class

This report records two Stata/MP 18 experiments: selection between the ordinary
recursive V-cycle and a reusable-workspace prototype, and a standalone batched
PCG comparison on synthetic graph Laplacians. These are local feasibility and
falsification results. They are not package benchmarks, estimator-equivalence
tests, peak-RSS measurements, Stata 19 portability evidence, or a production
promotion decision.

- Platform: Apple arm64, Darwin 25.5.0, 192 GiB physical memory
- Runtime: Stata/MP 18, 8 reported processors
- Source commit at start: `744ca8ed7271b791a721d1d05011d864d439801a`
- Canonical template SHA-256:
  `b8bb0900d5a36c4a3a97014a86c117cf6a0550b9946fc091296d164bfa41b99b`
- Generated test artifact SHA-256:
  `85b601a77e61d3eb126e58aa0e428989769836e151fb0d61c68cc1064dd820b6`
- Workspace driver SHA-256:
  `beb3fc149e17bfd93c97e06861089302f75d0d689a7e9875091e707a248d6aa7`
- Solver driver SHA-256:
  `9dffab8af28259878454cffa5e301ce70573293429b583da72610602df081e02`
- Workspace raw-data SHA-256:
  `b6e70e72e776f4780b08f857ca0f521b43f14dd021d814e82b0747c423ef6c7c`
- Solver raw-data SHA-256:
  `0686bc6bf093a9ec977b6e1ef926f7af8a5b8681e568ced9143d5d02eb94fa39`

## Workspace selection

Both paths were warmed before timing. Each row averages repeated applications
within one process to an identical deterministic eight-column sine RHS. The
workspace path was checked against the ordinary path to maximum absolute error
`<=2e-12` before timing.

Raw rows are stored in `CMG6_WORKSPACE_2026-08-15.csv`.

| Vertices | Repetitions | Init (s) | Ordinary apply (s) | Workspace apply (s) | Workspace/ordinary | Workspace bytes | Forecast peak bytes |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 10,000 | 20 | 0.001 | 0.0056 | 0.0091 | 1.625 | 2,557,440 | 72,221,920 |
| 100,000 | 5 | 0.003 | 0.0520 | 0.0854 | 1.642 | 25,593,408 | 118,291,480 |

The preallocated implementation is correct on the registered dirty-reuse and
batch-partition tests, but it is 63--64% slower on these path fixtures. The
Stata 18 selection is therefore the ordinary `@CMG_NS@__apply()` recursion.
The bounded workspace API remains test-only so Stata 19 or package-specific
calibration can revisit the decision without silently changing the chosen
path.

The byte forecasts are explicit core accounting plus the fixed graph-action
scratch allowance. They exclude allocator overhead and are not RSS.

## Standalone solver harness

The driver runs independent-column PCG recurrences in batched matrix
operations, comparing degree-diagonal and CMG preconditioning. It scales each
RHS, keeps inactive columns zero, checks curvature and preconditioner failures,
replaces residuals every 100 iterations, and recomputes the final residual
with the original graph action. Timings are one fresh run per row and exclude
data preparation. Tolerance is `1e-8`.

Raw rows are stored in `CMG6_SOLVER_2026-08-15.csv`.

| Graph | Vertices | RHS | Cap | Setup (s) | Diagonal (s) | CMG (s) | Diagonal max/median iter. | CMG max/median iter. | Diagonal status | CMG status | Max CMG rel. residual |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---:|
| path | 100 | 4 | 1,000 | 0.002 | 0.024 | 0.001 | 99 / 74.5 | 1 / 1 | CONVERGED | CONVERGED | 1.78e-12 |
| path | 1,000 | 8 | 5,000 | 0.017 | 2.183 | 0.050 | 999 / 999 | 42 / 38 | CONVERGED | CONVERGED | 4.98e-9 |
| ring | 1,000 | 8 | 5,000 | 0.026 | 1.097 | 0.054 | 500 / 500 | 46 / 38 | CONVERGED | CONVERGED | 4.21e-9 |
| barbell | 1,000 | 8 | 5,000 | 0.023 | 1.060 | 0.046 | 321 / 289 | 29 / 23 | CONVERGED | CONVERGED | 4.22e-9 |
| path | 10,000 | 8 | 500 | 0.176 | 10.775 | 1.194 | 500 / 500 | 166 / 137 | MAXITER | CONVERGED | 8.34e-9 |

The 10,000-vertex path is a concrete local weak-system rescue: diagonal PCG
remained capped while CMG passed a fresh residual check within 166 iterations.
The smaller cases show large iteration reductions. These synthetic results do
not establish package speed gains because the harness uses an ordinary graph
Laplacian, omits the estimators' complete worker--firm reconstruction and
target work, and does not compare against the package-owned B1 batching path.

## Correctness gates run with this source

`./.venv/bin/python shared/cmg/tools/run_checks.py` passed on 2026-08-15:

- deterministic generated-artifact and reverse-substitution checks;
- 26 Python tests, including the independent rational oracle and six generic
  Python--Mata fixtures;
- the Mata algebra/runtime suite, including materialized and randomized
  symmetry, positive curvature, linearity, scale, component, relabeling,
  batch-partition, workspace-reuse, a forced multi-chunk 20,000-edge hub,
  failure, and RNG-state gates; and
- separate compilation of both package namespaces.

## Decision and remaining gates

CMG6 is a local candidate. The symmetric V-cycle and solver harness have no
known local correctness failure, and the ordinary apply path is selected for
Stata 18. No package route is enabled. CMG4, CMG7, and CMG8 require ownership
handoff and unchanged estimator comparisons; CMG10 requires Stata 19/SCC,
repeated scale benchmarks, and peak RSS; CMG11 requires the registered package
promotion gates and external review.
