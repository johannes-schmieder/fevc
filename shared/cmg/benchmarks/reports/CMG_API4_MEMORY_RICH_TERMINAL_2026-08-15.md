# CMG API 4 memory-rich terminal report — 2026-08-15

## Trigger and bounded repair

API 3 admitted a directly factored terminal only through 1,536 hybrid
vertices. Source-bound Stata 19 job `7190153` showed that the natural
all-mover MATLAB-retained graph remained outside that cap: it followed the
API 2 hierarchy path and failed closed as `HIERARCHY_STALLED` in 9.282 command
seconds. The job posted no estimate and scheduler accounting records
`failed=0`, `exit_status=0`, four slots, ten seconds wall, and 210,580 KiB
maximum RSS. Diagonal B1 job `7190152` completed the same 256,472-row sample in
525.619 command seconds with maximum complete residual
`9.99779250872e-11`.

The registered graph has 1,285 firms and 4,063 workers. The clean-room hybrid
can add at most one auxiliary vertex per worker, so 5,348 is a conservative
fine-vertex upper envelope. API 4 uses a 6,144 hard cap. It still requires at
least 512 planned RHSs, at least 16 GiB of declared memory, and a predicted
factor no larger than the memory-derived dense-factor budget. At the hard cap,
one persistent factor is 288 MiB. Actual component factors are rechecked
before allocation; construction scratch has a separate registered cap.

## Red tests and diagnostics

Before the implementation change, the resource-profile assertions for 1,537
and 6,144 vertices failed. API 4 now selects those exact terminals under a 56
GiB envelope and 601 RHSs, retains the recursive 256 threshold at 6,145, and
keeps the 8 GiB and low-RHS cases unchanged. `options_valid()` and the explicit
solver benchmark accept no terminal larger than 6,144.

The forced test adapter now records fine hybrid vertex and edge counts in its
route diagnostics. The SCC aggregate harness exports both counts for a
converged or sufficiently constructed failed hierarchy. This is diagnostic
only and does not alter the operator, estimator, routing, sample, probes, or
RNG state.

## Local calibration and gates

The memory-rich terminal was calibrated locally with Stata 18 on a
2,048-vertex ring, 601 deterministic RHSs, tolerance `1e-10`, and maximum
2,000 iterations. Stata reported eight processors. The exact terminal used
1.264 seconds setup and 4.247 seconds CMG PCG; diagonal PCG used 84.828
seconds. CMG required at most two iterations and its maximum fresh residual
was `4.82479861574e-11`. The setup-inclusive CMG/diagonal gain was 15.39x.
The terminal records one level and 311,312 structural bytes.

The API 4 red test passes after deterministic assembly. The complete shared
CMG and KSS local qualification gates pass. Automatic routing remains
disabled; SCC qualification of the actual all-mover graph is required.

## Source-bound Stata 19 small oracle

Run `/projectnb/welfgr/kss-bc/runs/20260815T221100Z-f0dd3ec` binds commit
`f0dd3eca92d18ee507b618853e39d6cf6add3ec9`. The small MATLAB-retained
adapter reconstructed 11,549 rows, 216 workers, 69 firms, and 538 matches with
no additional removals. Exact passed first in 1.659 command seconds with
inverse residual `1.43093535625e-13`.

| route | setup | Schur | preconditioner | PCG | leverage | target | correction | total | iterations | max residual | peak RSS KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| B1 | 0.037 | 5.068 | 0.016 | 5.753 | 2.954 | 5.801 | 8.755 | 9.270 | 28 | `9.9489e-11` | 64,212 |
| CMG | 0.090 | 0.184 | 0.041 | 0.542 | 1.605 | 1.985 | 3.590 | 4.060 | 1 | `3.4877e-13` | 68,404 |

The CMG hybrid has 94 vertices and 255 edges and uses a one-level 69,192-byte
factor. Command speedup is 2.283x. B1/CMG estimator `mreldif` is
`1.1555341115543391e-11`; exact/B1 plug-in `mreldif` is
`8.071025808414697e-12`. All 601 RHSs and all match-overlap checks pass. Jobs
`7190221`, `7190271`, `7190279`, `7190280`, and `7190288` record
`failed=0`, `exit_status=0`. The privacy-safe evidence bundle is
`/private/tmp/kss-api4-small-evidence.XtrYSD` and contains no row-level
artifact.

## Source-bound Stata 19 natural all-mover gate

The same run derives 256,472 rows, 4,063 workers, 1,285 firms, and 10,343
matches from the checksum-bound maintained-MATLAB retained set, with no
additional graph or bridge removal. The prerequisite small exact oracle above
passes before this post-oracle scale step. B1 and forced C use the identical
prepared DTA, 200 probes, seed `8675309`, tolerance `1e-10`, observation-key
probe order, target, formulas, quotient normalization, grounding, and worker
reconstruction.

| route | setup | Schur | preconditioner | PCG | leverage | target | correction | total | max iterations | max residual | peak RSS KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| B1 | 0.944 | 319.914 | 0.897 | 339.073 | 153.654 | 252.831 | 406.485 | 416.750 | 95 | `9.9978e-11` | 443,140 |
| CMG | 2.569 | 3.329 | 8.278 | 18.087 | 41.452 | 49.482 | 90.934 | 101.096 | 1 | `4.0524e-13` | 490,172 |

The command speedup is 4.122x and estimator `mreldif` is `1.2879e-10`. All
601 RHS diagnostics pass complete original-system residual checks. The API 4
profile builds a one-level terminal for the 1,796-vertex, 5,948-edge hybrid;
it records 509,608 structural bytes and a 25,776,200-byte dense factor. This
uses more memory than the API 3 recursive attempt but remains below 479 MiB
process RSS and reduces B1's maximum 95 iterations to one for every RHS.

Jobs `7190290`, `7190300`, and `7190301` record wall times of 11, 419, and
102 seconds and CPU times of 21.194, 1,654.006, and 393.246 seconds. Every job
uses four slots and has `failed=0`, `exit_status=0`. Comparison job `7190325`
passes with no one-sided retained matches and records one second wall, 0.772
CPU seconds, four slots, and `failed=0`, `exit_status=0`. Both estimator
submissions carry a 900-second measured projection and a separate
5,400-second process timeout.

API 4 therefore passes the bounded KSS real-data correctness, complete
residual, speed, memory, and SCC gates. The core remains forced test-only:
easy CMG retains a typed setup rejection, the installed package contains only
B1, and automatic-dispatch/no-regression promotion remains open.
