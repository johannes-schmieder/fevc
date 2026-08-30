# Focused scalable-projection comparison

This harness implements the registered six-cell comparison in
`qualification/inference_matlab/SCALABLE_PROJECTION.md`. It is deliberately
limited to VCkss Rust/JLA/diagonal `project()` and maintained MATLAB
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
under `/projectnb/welfgr/vckss/runs`; only compact source-bound evidence is
committed.

Deploy a clean exact commit with `deploy_scc.sh RUN_ID`. Submit `prepare`,
`gate`, and `array` as three separate `submit_scc.sh RUN_DIR MODE` operations,
checking complete `qacct` and application pass receipts between stages. This
prevents an SGE dependency from releasing larger cells after a failed gate.
