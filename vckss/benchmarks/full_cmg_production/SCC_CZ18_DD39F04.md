# Production full-CMG SCC CZ18 promotion gate

Normal-build VCkss source `dd39f049c643760727d808173c042568eedbe9b2`
passes the registered fixed-CZ18/200-probe SCC comparison in job `7328200`.
The run used Stata/MP 19, MATLAB R2024b Update 3 with four pool workers, four
VCkss application threads, Rust 1.85.1, vendored CMG
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`, one cold repetition, and five
position-balanced warm repetitions on `scc-yr4`.

| Warm-median measure | VCkss | MATLAB | VCkss / MATLAB |
| --- | ---: | ---: | ---: |
| Complete estimator command | 31.365 s | 55.148 s | 0.569 |
| Process-tree peak RSS | 2.796 GB | 11.740 GB | 0.238 |

The selected production gate passes: VCkss is 1.76 times as fast as matched
MATLAB, and its median is 0.924 times the frozen private winner's 33.942-second
median. The longer-run 2x MATLAB objective is close but does not pass on this
matrix.

## VCkss timing and solver receipt

| Warm-median production phase | Seconds |
| --- | ---: |
| Native ingest | 0.427 |
| Canonicalization | 2.267 |
| Graph | 0.679 |
| Compression | 2.090 |
| Plan | 1.662 |
| Native solve total | 17.165 |
| Full-CMG hierarchy/plan | 0.038 |
| RHS construction | 1.752 |
| Repeated solve | 6.672 |
| Extraction, recovery, and certification | 3.378 |

All 601 RHSs completed with warm-median totals of 7,001 PCG iterations, 8,886
operator applications, and 7,001 CMG applications. The frozen same-route
schedule performed 35 refinement passes covering 1,284 initially failing
columns. The attempt-max reduced residual was `1.541e-3`; deterministic
refinement brought every selected column inside its reduced-system gate and
the maximum complete original-system residual to `9.975e-6` against the
default probe limit of `1e-5`. No estimator or backend fallback occurred.

The conservative pre-RNG forecast was 9.788 GB, the admitted full-CMG peak
was 7.133 GB, actual retained CMG storage was 0.095 GB, and observed VCkss
process RSS had a 2.796 GB warm median. MATLAB's complete process tree had an
11.740 GB warm median. Data, RNG, sort RNG, `e(sample)`, accounting, wrapper,
and application checks passed in every round; SGE accounting records
`failed=0` and `exit_status=0`.

## Statistical comparison

The maintained MATLAB comparator uses a different RNG, target-weight
convention, and solver contract, so its corrected targets are descriptive
rather than a common-probe numerical gate. Against the frozen common-Counter-
V1 private winner, the four absolute corrected-target differences range from
`9.75e-10` to `3.67e-9`, while the registered MCSE-aware limits range from
`8.78e-7` to `3.23e-6`; all pass.

The pinned validator reports
`production_over_matlab=0.568741` and
`production_over_private=0.924076`. Compact task, platform, MATLAB, wrapper,
scheduler, and validation receipts are preserved under
[`evidence/scc/dd39f049c643760727d808173c042568eedbe9b2`](evidence/scc/dd39f049c643760727d808173c042568eedbe9b2).
