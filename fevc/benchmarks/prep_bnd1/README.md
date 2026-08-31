# PREP-BND-1 benchmark

This harness compares the separately committed PREP-BND measurement baseline
and candidate without making performance thresholds scientific gates.  It
runs the exact package archives in fresh Stata processes, while the benchmark
driver is a third, separately commit- and SHA-bound artifact.  No baseline or
candidate commit is embedded in a script.

The causal exposure gate requires:

- `retained_id_group_calls` changes from 2 to 0;
- `semantic_group_calls` changes from 1 to 0 and `stata_sort_calls` changes
  from 2 to 1 for the cumulative PREP-SEM candidate (the MAP-only candidate
  retains the legacy 1/2 pair);
- all other preparation counts stay exact;
- the returned retained map has two columns and `e(N_retained)` rows;
- compression still imports `e(N_retained)` rows; and
- solver structure, `e(sample)`, data/sort state, and RNG restoration compare
  exactly; and
- scientific results compare at the registered absolute/relative tolerance
  of `2e-12`, which admits only the observed floating-point reduction-order
  roundoff and is far tighter than the estimator's acceptance tolerance.

Both the established `PREP-RHS-PERF-V1` matrix and the exclusive eleven-field
`PREP-BND-PERF-V1` matrix are exported as long-form CSV.  The
`PREP-BND-COUNTS-V1` operation counts are exported beside them.

## Local gate

Commit the harness first so the tool commit is clean and source-bound.  Then
run both source orders; `run_local.py` creates four fresh Stata processes and
uses two warm rows after one cold row in each process:

```bash
./.venv/bin/python fevc/benchmarks/prep_bnd1/run_local.py \
  --baseline 72179fb --candidate a83f902 \
  --output-dir /private/tmp/prep-bnd1-local
```

The local fixture is F256/P40 (15,360 stored rows).  It is deliberately small
enough to diagnose receipt or contract failures before any SCC submission.

## SCC inputs and jobs

Create immutable archives and hash-bound driver/wrapper artifacts:

```bash
./.venv/bin/python fevc/benchmarks/prep_bnd1/prepare_scc.py \
  --baseline 72179fb --candidate a83f902 \
  --output-dir /private/tmp/prep-bnd1-scc-input
```

Deploy incrementally, without deletion, under
`/projectnb/welfgr/fevc/prep-bnd1/`.  Use a new UTC run ID.  Submit AB
and BA scalar jobs for F256, F1024, F4096, and F8192 at P256 with
`run_pair.sge`; submit two more scalar jobs for the fixed CZ18/P20 input with
`run_cz18_pair.sge`.  The wrappers require the two commits, bundle hashes,
driver hash, order, and run ID as environment variables.  They reject an
existing result directory.

Synthetic jobs request four slots, 4 GiB/core, three hours, and set Stata to
four actual processors.  CZ18 jobs reserve fourteen slots and 4 GiB/core
(56 GiB total) for I/O/memory fairness while still setting Stata to four
actual processors.  Neither wrapper selects a queue or host.  The CZ18 input
must hash to
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`;
the row-level data are never copied into this repository or bundle.

Collect each scalar job's full `qacct -j JOB_ID` output as
`qacct/JOB_ID.txt`.  Create `jobs.tsv` with this exact header and one row per
case/order:

```text
job_id	case	order	result_relpath	scheduler_log	expected_slots	repetitions
```

The required matrix is ten rows: five cases (`F256-P256`, `F1024-P256`,
`F4096-P256`, `F8192-P256`, `CZ18-P20`) times `ab` and `ba`.  Run the
fail-closed collector only after all jobs have left `qstat` and accounting is
available:

```bash
./.venv/bin/python fevc/benchmarks/prep_bnd1/analyze_scc.py \
  --run-root COLLECTED_RUN --jobs-tsv COLLECTED_RUN/jobs.tsv \
  --qacct-dir COLLECTED_RUN/qacct \
  --baseline BASELINE_SHA --candidate CANDIDATE_SHA \
  --baseline-bundle-sha256 BASELINE_TAR_SHA \
  --candidate-bundle-sha256 CANDIDATE_TAR_SHA \
  --driver-sha256 DRIVER_SHA --output COLLECTED_RUN/summary.json
```

Acceptance requires `qacct failed=0`, `exit_status=0`, exact slot accounting,
the wrapper and application markers, node/qacct host agreement, source and
input hashes, every expected artifact, the causal exposure transition, exact
structural/sample/RNG comparisons, and the registered scientific tolerance.
Timing changes remain advisory and are reported separately by stage and case.
