# FE-BUF-1 benchmark

This archive-isolated benchmark compares the measurement commit with a
descendant candidate by running the same tracked Stata driver in fresh Stata
processes. Pass the full measurement commit with `--baseline`; the harness has
no self-referential default revision. Scientific results, solver structure,
and the complete `FE-BUF-PERF-V1` profile are source-bound in the CSV evidence.

The four-run local fixture is deliberately moderate (60,000 rows, P200). It is
the causal gate before same-host SCC pairs. Runtime thresholds are advisory:
safe improvements remain eligible even when their measured gain is small.
The synthetic SCC driver normally records three repetitions. At a scale whose
empirical three-repetition pair cannot fit the scheduler wall, the runner may
set `FE_BUF_REPETITIONS=1`; the analyzer registers F=15,625 as a cold,
single-repetition extrapolation check rather than silently labeling it warm.
When that replacement uses a distinct immutable run ID, pass its collected
directory with `analyze_scc.py --large-root`; smaller tiers remain bound to
`--root`.

The CZ18 holdout uses the frozen 8,201,888-row retained design with SHA-256
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`,
P20, seed 8675309, batch(auto), 56 GiB reserved memory, and four actual Stata
processors. `run_cz18_pair.sge` runs three fresh-process repetitions in each
of AB and BA order on the same host within each scheduler job. The tracked
analyzer requires exact scientific, solver, lifecycle, data, sort, and RNG
receipts while reporting timing and FE-buffer accounting separately.
