# 76,800-row approximate comparison

Completed September 9, 2026. SCC job 7506330: all 15 approximate calls pass
execution/collection checks; `failed=0`, `exit_status=0`. Run:
`output/five_way_scaling/20260909Tlarge76800`.

See `diagnostic/reviewed_report.md` for the appendix table, estimates, runtime,
memory, comparison with N=3,840, validation and limitations. The independent
exact Schur reference removes 84.0%/41.5% of worker/firm plug-in variance.
Every implementation's median worker and firm variance is within 0.8% of
that reference. This does not establish unbiasedness or universal equivalence.

Median time / peak summed RSS MiB: FEVC 2.769 / 383.20; MATLAB 83.233 / 5004.09;
Julia 22.780 / 1003.63; R 12.681 / 5921.28; PyTwoWay 4.518 / 695.72.
Worker-pool setup/JIT is included, generic CSV import excluded. Shared pages
can be counted more than once in summed RSS. The two sizes ran on different
shared hosts, so cross-size ratios are descriptive.

RNG caveats: Julia uses fixed internal thread streams; MATLAB seeds only the
client while drawing projections on parallel workers. Their three estimates
are effectively identical in this run. Do not claim three independently
controlled projection draws or interpret zero range as zero projection error.
No estimator formula, tolerance or RNG policy was modified after results.

The original raw report and plots remain immutable; the reviewed report adds
the MATLAB RNG observation and a separate plot fixes crowded axis labels.
Validation: 42 bundle hashes, 15 calls, 60 component comparisons, raw/normalized
identities, scheduler/application receipts; root pytest 798 passed, four focused
tests passed, CMG assembly and wrapper syntax checks passed. No changes were
made to package code, paper sources, releases or earlier benchmark evidence.
