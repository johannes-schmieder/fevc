# Benchmark harnesses

These are developer tools for reproducible correctness and timing comparisons.
They are not installed by `net install` and are not needed to use `fevc`.

## Entry points

- `synthetic_benchmark.do`: small local command and output-validation smoke.
- `fevc_matlab_2026/`: complete-command comparisons with the KSS Matlab package.
- `five_way_scaling/`: the owner's multi-implementation scaling work.
- `projection_scaling/` and `projection_akm_scaling/`: projection workloads.
- `oracle/`: independent small-design reference calculations.
- `scc/`: private cluster launchers, collectors, and validators.

Other subdirectories retain source and fixtures used by focused regression
tests. Their original protocols describe those specific experiments; they do
not authorize a new run or establish a performance claim for current code.

## Running a benchmark

Choose the smallest input that exercises the changed behavior. Follow the
[test guide](../TESTING.md) and the registered
[acceptance policy](../docs/development_acceptance_v1.json). Compare complete
command time only after validating the estimator, sample, targets, numerical
results, process exits, and output inventory.

New MCSE exports must save `e(mcse)` (all-probe by default), `e(mcse_mode)`,
`e(mcse_method)`, `e(mcse_status)`, `e(mcse_available)` and
`e(mcse_cov_raw)` together. Missing or withheld diagnostics remain missing;
they cannot be replaced with conditional SEs or zeros. Current small command,
paired-CMG, Separations and alpha writers include these fields. Their CSV raw
covariance columns are `mcse_cov_raw_ij`, with one-based indices in the four
main-target order. Historical protocols and archived receipts retain their
original definitions; review their MCSE assumptions before any new campaign.

Write new results under ignored `.local/` or `output/`, with exact source,
input, seed, command, and environment identities. Keep restricted data and
licensed comparator source outside this repository. Do not commit raw logs,
scheduler output, or generated reports.

Superseded reports and qualification output are retained outside the active
checkout; see [historical material](../../docs/ARCHIVE.md).
