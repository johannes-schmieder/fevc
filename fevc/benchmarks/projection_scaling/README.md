# Focused scalable-projection comparison

This harness implements the registered six-cell comparison in
`qualification/inference_matlab/SCALABLE_PROJECTION.md`. It is deliberately
limited to VCkss Rust/JLA/diagonal `project()` and KSS Matlab
JLA-plus-`lincom_KSS` at 6,000, 24,000, and 96,000 rows, with three paired
repetitions per size.

The source archive must be an exact clean commit. `prepare.sge` builds the
Linux plugin with Rust 1.85.1, compiles the maintained CMG MEX boundary, and
generates the three deterministic inputs. `gate.sge` runs dense exact Mata,
Rust, and MATLAB on 6,000 rows and must pass before the measured array is
released. `run_task.sge` contains only the nine registered size/repetition
pairs. Each task uses four slots, launches fresh processes sequentially on one
host, rotates the implementation order, and records both estimator-phase time
and complete process-tree RSS.

The maintained comparator is external commit
`8b957ffeb10b8465a3584fceb0265cccc48379e1`; its source is never copied into
this repository. The harness verifies the committed hashes of
`leave_out_COMPLETE.m` and `lincom_KSS.m` before execution.

After all jobs leave `qstat`, wait for structurally complete `qacct` records,
then run `aggregate.py RUN_DIR SUMMARY_JSON CELLS_CSV`. Raw run artifacts stay
under `/projectnb/welfgr/fevc/runs`; only compact source-bound evidence is
committed.

The aggregator is failure-preserving. It requires structurally complete
accounting for all nine tasks and writes one `PASS`, `FAIL`, or `NOT_RUN` row
for every registered implementation run. A terminal scientific failure still
produces `summary.json` and `cells.csv`, then returns exit status 2. This keeps
accepted smaller cells and typed convergence evidence without treating absent
downstream runs as successful or silently dropping them.

Deploy a clean exact commit with `deploy_scc.sh RUN_ID`. Submit `prepare`,
`gate`, and `array` as three separate `submit_scc.sh RUN_DIR MODE` operations,
checking complete `qacct` and application pass receipts between stages. This
prevents an SGE dependency from releasing larger cells after a failed gate.

## Recorded run

The first registered run used source
`96e7a666c0a2dcc2c89183c656edd72e04b8ec0e`, preparation job `7368458`, gate
job `7368471`, and paired array `7368481`. Preparation and the 6,000-row gate
passed. All six paired repetitions at 6,000 and 24,000 rows passed every
scientific and application gate. Their maximum covariance-diagonal differences
from KSS Matlab were 0.4785% and 0.4403%, and maximum standard-error
differences were 0.2395% and 0.2204%.

At 6,000 rows, median VCkss/MATLAB command times were 32.494/78.743 seconds. At
24,000 rows they were 1,466.706/77.336 seconds: the diagonal route became
18.97 times slower than MATLAB on the measured command. The three 96,000-row
tasks were terminal scientific failures: MATLAB's independently reconstructed
grounded fit failed the strict harness tolerance in both MATLAB-first tasks,
and VCkss diagonal PCG failed at its 20,000-iteration limit in the Rust-first
task. Consequently no 96,000-row paired speed, covariance/SE, or accepted
memory-growth result exists. The run establishes the need for a stronger
preconditioner; it does not justify a paper performance claim.

The compact source-bound record is in
`evidence/scc/96e7a666c0a2dcc2c89183c656edd72e04b8ec0e/`.

The later forced-CMG implementation checkpoint is documented separately in
`../../qualification/inference_matlab/SCALABLE_PROJECTION.md`. Do not reinterpret
this diagonal harness or its immutable receipts as CMG evidence. Any new CMG
comparison must use an exact-source harness variant and preserve the same
sample, probe, statistical, residual, PSD, memory, process, and order-rotation
gates.
