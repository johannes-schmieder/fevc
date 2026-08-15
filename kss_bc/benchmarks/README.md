# Benchmark and SCC qualification harness

The original KB6 drivers qualify the internal `kss_bc` point-estimation
package without restricted project data. KSS-NUMOPT-1 also contains a
separately named, owner-authorized SCC-only Separations wage benchmark. These
are development and validation tools, not runtime dependencies.

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

No Separations or other restricted data may be staged into a KB6 run. The
KSS-NUMOPT Separations wrappers instead read an existing checksum-bound wage
artifact in place, keep all derived rows and match identifiers under the SCC
run directory, and expose only aggregate evidence. The synthetic network has
ungrouped workers, a ring of firm effects with redundant movers on every
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

`estimator_cmg_benchmark.do` performs the API-15 end-to-end comparison through
the public ado. `validate_numopt.py` requires unchanged dimensions, probes,
seed, tolerance, targets, and every complete RHS residual. SCC jobs use four
slots, may reserve 64 GB, and are admitted only with a measured projection no
larger than 90 minutes. The process timeout is also 90 minutes. Do not submit a
large case.

The real-data sequence uses `separations_wage_prepare.do`,
`separations_wage_estimator.do`, and `separations_kss_reference.m`. It starts
with a 5,000-worker deterministic slice of CZ24 and uses `logrwage-xb`, the
analysis worker/firm units stored in the wage artifact, match deletion, 200
probes, seed `8675309`, and Stata tolerance `1e-10`. The MATLAB result is a
checksum-bound descriptive reference because its legacy finite-projection
formula and random stream differ. `separations_compare_samples.do` compares
match sets on SCC and exports counts only. Never collect the prepared DTA/CSV,
MATLAB detail file, or retained-match DTA files.
The bounded `small` preparation selects a deterministic dense mover core by
worker overlap through high-degree firms, then mover degree and raw worker
key. It does not use a first-ID prefix. The natural `full` route does no such
subsampling. Every downstream route is checksum-bound to the same prepared
slice.

When B1 or CMG correctly withholds, `validate_separations.py --matlab-only`
validates a separately successful descriptive MATLAB reference without
requiring a sample comparison or relabeling the failed Stata routes as
estimates. It still requires preparation and MATLAB `qacct`, peak RSS, the
90-minute projection, all maintained source hashes, and the four-target
identity.

For a benchmark-only diagnosis of the sample mismatch,
`separations_matlab_sample.do` reads the checksum-bound detailed MATLAB output
in place on SCC, extracts only its retained worker--firm keys, and joins those
keys back to every physical row in the parent prepared DTA. It then repeats
the KSS graph filter and an independent iterative match-bridge audit until the
sample is stable. The derived rows remain SCC-only. This path neither changes
the public KSS selector nor creates a MATLAB production dependency.

`submit_separations.sh ... matlab-sample` constructs that audited sample.
The `exact`, `b1`, and `cmg` jobs then run on its single checksum-bound DTA;
`validate_matlab_subset.py` requires exact/B1 plug-in agreement, B1/CMG
estimator agreement, complete residuals, identical samples and tuning,
successful scheduler accounting, stage timings, and peak RSS. All three
routes retain the measured 90-minute admission rule and 5,400-second process
timeout. The exact route is the first fail-closed gate; B1 and CMG must not be
submitted if it fails. After that small exact oracle passes,
`--omit-exact` validates a larger post-oracle scale step without attempting a
dense inverse. It still requires B1/CMG estimator equality, every complete
RHS residual, identical sample and tuning, timing, RSS, and SCC accounting.
It also requires `--oracle-run-dir` and `--oracle-label` and revalidates the
source-bound small exact result; omission cannot be asserted without stored
oracle evidence.
The comparison job checks B1/CMG match equality directly on a MATLAB-derived
label; it checks the additional MATLAB overlap when a detail file belongs to
the same label.

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
