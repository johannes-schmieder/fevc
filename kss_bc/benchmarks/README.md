# Benchmark and SCC qualification harness

These drivers qualify the internal `kss_bc` point-estimation package without
using restricted project data.  They are development and validation tools,
not runtime dependencies.

## Evidence contract

Every SCC run is bound to one clean Git commit and one directory:

```text
/projectnb/welfgr/kss-bc/runs/<run-id>/
```

The run directory contains `source_commit.txt`, an archive of that exact
commit under `source/`, scheduler receipts under `submissions/`, application
outputs, and `qacct` records.  A job is accepted only when all three layers
pass:

1. `qacct` reports `failed 0` and `exit_status 0`;
2. the Stata/MATLAB application log and explicit `.pass` marker are present;
3. `validate_scc.py` accepts the structured outputs and their source commit.

No Separations or other restricted data may be staged.  The synthetic network
has ungrouped workers, a ring of firm effects with redundant movers on every
edge, two actual matches and four stored observations per worker, varying
within-match controls, literal integer frequencies, and explicit target mass.

## Jobs

- `portability`: full Stata suite plus isolated `net install` under Stata/MP
  19.
- `oracle`: an independently written dense MATLAB calculation and the Stata
  exact backend on the same registered fixture.  The MATLAB file contains no
  copied code from `LeaveOutTwoWay`.
- `smoke`, `medium`, and `large`: matrix-free JLA scale steps.  Each records
  graph, fit, setup, Schur-action, preconditioner-application, PCG, leverage,
  target, correction, and total time; per-RHS solver diagnostics; dimensions;
  target estimates; and GNU `time` peak RSS.

`lockstep_solver_benchmark.do` compares the frozen scalar B0 service with B1.
`cmg_kss_benchmark.do` compares the forced test-only CMG path with B1 and
records a typed hierarchy failure as a benchmark result. Neither driver
changes the installed route. The 2026-08-15 evidence is in `reports/`.

Suggested initial ladder:

| Scenario | Workers | Firms | Stored rows | Probes |
|---|---:|---:|---:|---:|
| smoke | 5,000 | 250 | 20,000 | 40 |
| medium | 50,000 | 2,500 | 200,000 | 100 |
| large | 250,000 | 10,000 | 1,000,000 | 200 |

Submit one step at a time with `scc/submit_one.sh`.  Validate and inspect
`qacct` resource use before requesting the next step; the scripts never
automatically cascade into a larger job.  They request four slots because the
available Stata/MP license is four-core, and they use `qsub -P welfgr`.

Example from the staged source directory on an SCC login node:

```bash
bash kss_bc/benchmarks/scc/submit_one.sh \
  /projectnb/welfgr/kss-bc/runs/<run-id> "$PWD" <commit> portability

bash kss_bc/benchmarks/scc/submit_one.sh \
  /projectnb/welfgr/kss-bc/runs/<run-id> "$PWD" <commit> oracle

bash kss_bc/benchmarks/scc/submit_one.sh \
  /projectnb/welfgr/kss-bc/runs/<run-id> "$PWD" <commit> \
  smoke 5000 250 40 20260814
```

After a job finishes, collect its accounting record with:

```bash
bash kss_bc/benchmarks/scc/collect_qacct.sh \
  /projectnb/welfgr/kss-bc/runs/<run-id> smoke
```

Run the validator on either the remote directory or a byte-for-byte local
copy:

```bash
./.venv/bin/python kss_bc/benchmarks/validate_scc.py \
  --run-dir <run-directory> --expected-commit <commit> \
  --jobs portability oracle smoke
```

The production-size target remains a qualification question.  Passing the
synthetic ladder establishes portability and bounded numerical behavior on
the tested public design; it does not establish application assumptions,
econometric inference, or performance on restricted data.
