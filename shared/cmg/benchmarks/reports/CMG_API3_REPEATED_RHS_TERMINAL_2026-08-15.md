# CMG API 3 repeated-RHS terminal report — 2026-08-15

## Scope and numerical contract

API 3 changes only the deterministic resource policy used before hierarchy
construction. When there are at least 512 planned RHSs, at least 16 GiB of
declared memory, no more than 1,536 hybrid vertices, and the predicted square
factor fits the registered dense-factor budget, `options_resource()` selects
the complete fine graph as the terminal. The actual component allocations are
checked again before allocation. Low-RHS cases, larger graphs, the hybrid
operator, quotient projection, PCG recurrence, tolerances, and package-level
complete residual checks are unchanged.

This policy was introduced after the 1,285-firm MATLAB-retained Separations
graph reached `HIERARCHY_STALLED` under API 2. The forced-C route posted no
estimate. Diagonal B1 completed the identical 256,472-row, 4,063-worker,
1,285-firm, 10,343-match problem in 415.995 Stata command seconds, with 95
maximum iterations and maximum complete residual `9.99779250872e-11`.
Scheduler job `7189279` used 418 seconds wall, 1,650.928 CPU seconds, four
slots, and 439,940 KiB maximum RSS. Failed-closed CMG job `7189280` used 11
seconds wall and posted status `HIERARCHY_STALLED` with no estimates.

## Failing test and bounded repair

The registered Mata regression asks for the resource profile at 1,285
vertices, 601 RHSs, and a 56 GiB envelope. It failed before the repair because
the profile retained `coarse_max=128`; API 3 requires 1,285 and validates the
result. Companion cases retain 128 at 8 GiB, at 1,537 vertices, and for only
16 RHSs. The accepted upper bound on an explicit benchmark terminal is 1,536.

The factor prediction is `8*n^2` bytes. At the cap it is about 18 MiB, below
the maximum 512 MiB dense-factor budget. This bounded policy never allocates
an observation-square or observation-parameter matrix.

## Local calibration

The calibration used Stata 18, a 1,285-vertex ring, tolerance `1e-10`, maximum
2,000 iterations, and the ordinary selected apply path. Stata reported eight
processors. Times are one-run kernel measurements in seconds.

| RHSs | terminal | setup | CMG PCG | setup + PCG | max iterations | max residual | levels | structural bytes |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 64 | 128 | 0.022 | 0.321 | 0.343 | 54 | `4.9550e-11` | 3 | 320,560 |
| 64 | 1,285 | 0.306 | 0.095 | 0.401 | 1 | `1.9017e-11` | 1 | 195,336 |
| 601 | 128 | 0.022 | 4.215 | 4.237 | 54 | `4.9980e-11` | 3 | 320,560 |
| 601 | 1,285 | 0.307 | 1.494 | 1.801 | 2 | `4.6776e-11` | 1 | 195,336 |

The exact terminal is 16.9% slower setup-inclusive at 64 RHSs and 2.35x
faster at 601 RHSs. The policy threshold of 512 therefore preserves the
measured low-RHS case while admitting the repeated-RHS gain.

## Local gates

The deterministic assembler regenerated all three namespaces and manifest.
`shared/cmg/tools/run_checks.py` passes 26 Python tests plus all Mata math,
namespace, bounded-hub, fault, and solver gates. The complete KSS local gate
also passes with CMG API 3, as does the repository-wide `--scope full` gate.
These executable passes establish finite implementation evidence only.

## Stata 19 SCC small real-data gate

The source-bound run is
`/projectnb/welfgr/kss-bc/runs/20260815T210200Z-561e040`, commit
`561e040e029fe76658af798ee71ede9a9bfc0f4f`. It reads the prior maintained
MATLAB output only to reconstruct its retained match set. The adapter restored
all physical rows and the KSS audits removed no additional rows or matches:
11,549 rows, 216 workers, 69 firms, and 538 matches. The derived DTA SHA-256 is
`5eed59dae28e3311d0af0ae0ef96c4eb1f75d071966c22ec89759cd9160e942c`.

All estimator routes used 200 probes, seed `8675309`, tolerance `1e-10`,
observation-key probe order, Stata 19 IC, four requested slots, and the same
checksum-bound DTA. Exact passed first in 1.673 command seconds with inverse
residual `1.43093535625e-13`. B1 and CMG then passed, followed by the retained
match comparison and the local aggregate validator.

| route | setup | Schur | preconditioner | PCG | leverage | target | correction | total | max iterations | max residual | peak RSS KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| B1 | 0.036 | 5.004 | 0.025 | 5.690 | 2.953 | 5.748 | 8.701 | 9.247 | 28 | `9.9489e-11` | 63,720 |
| CMG | 0.090 | 0.178 | 0.046 | 0.543 | 1.612 | 1.979 | 3.591 | 4.056 | 1 | `3.4877e-13` | 67,020 |

CMG is 2.280x faster at command level. Its one-level terminal records 23,520
structural bytes and 69,192 dense-factor bytes. B1/CMG estimator-matrix
`mreldif` is `1.1555341115543391e-11`; exact/B1 plug-in `mreldif` is
`8.071025808414697e-12`. Match counts are 538/538 with zero one-sided matches.
Jobs `7189360`, `7189368`, `7189431`, `7189433`, and `7189900` all record
`failed=0`, `exit_status=0`. The privacy-safe validation bundle is
`/private/tmp/kss-api3-small-evidence.Qv70Pm`; it contains no prepared DTA,
MATLAB detail, or retained-match file. Automatic routing remains disabled.

## Stata 19 SCC moderate real-data gate

The post-oracle moderate step reconstructed 60,160 rows, 953 workers, 250
firms, and 2,360 matches, again with no extra graph or bridge removals. Its
derived DTA SHA-256 is
`3b8f5b77e897b5728d70d9468b2b449ddd35bbfd3275a03cce10f890daa07b67`.
It reused the small exact result above as a source-bound prerequisite and did
not form a moderate dense exact inverse.

| route | setup | Schur | preconditioner | PCG | leverage | target | correction | total | max iterations | max residual | peak RSS KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| B1 | 0.196 | 33.129 | 0.127 | 36.182 | 20.066 | 31.851 | 51.917 | 54.192 | 41 | `1.2925e-10` | 131,324 |
| CMG | 0.561 | 0.817 | 0.343 | 2.621 | 8.476 | 10.063 | 18.539 | 20.968 | 1 | `4.0074e-13` | 138,572 |

CMG is 2.585x faster at command level. Its one-level terminal records 97,080
structural bytes and 1,013,888 dense-factor bytes. B1/CMG estimator-matrix
`mreldif` is `4.448900124325195e-12`; all 601 RHS diagnostics pass the
complete residual gate. Match counts are 2,360/2,360 with zero one-sided
matches. Jobs `7189967`, `7190006`, `7190007`, and `7190039` all record
`failed=0`, `exit_status=0`. The privacy-safe validation bundle is
`/private/tmp/kss-api3-moderate-evidence.Qrjzqt` and contains no row-level
artifact.
