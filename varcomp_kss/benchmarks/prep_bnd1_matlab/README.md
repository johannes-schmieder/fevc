# PREP-BND-1 maintained-MATLAB comparison

This is an isolated validation harness, not part of the installed estimator.
It compares one source-bound `varcomp_kss` candidate with the checksum-bound
maintained `LeaveOutTwoWay` MATLAB implementation without copying that
implementation into Git or an SCC bundle.

## Claims and boundaries

The scaled matrix uses the same literal stored rows, worker and firm IDs,
match coordinates, outcome bytes, dimensions, requested probes, and seed
label in both applications. Targets are uniform stored-row targets: there are
no frequency or custom target weights. Each scalar SCC job runs one Stata and
one MATLAB process sequentially on the same host. Source order is reversed for
every seed. Stata must expose exactly four processors; MATLAB must expose four
distinct pool workers with single-threaded numerical libraries.

Maintained MATLAB remains a descriptive JLA comparator. Its legacy
finite-projection expression, parallel random schedule, and solver contract
differ from `varcomp_kss`. Therefore corrected estimates have no equality
gate. A MATLAB result contributes to numerical-similarity summaries only when
the maintained command reports that its own PCG converged. Nonconverged calls
remain valid timing evidence and are labeled numerical rejections.

The matrix is deliberately small-first:

- F64, F256, and F1024 at P20;
- F256 and F1024 at P200;
- seeds `104729`, `8675309`, and `20260819`; and
- both `stata_matlab` and `matlab_stata` process orders.

That is 30 independently scheduled, same-host pairs. Each job separately
records input generation, Stata import and command phases, MATLAB import,
validation, pool startup, run-local MEX compilation, command, teardown,
process wall/RSS, process-tree RSS, and scheduler accounting. These cells are
the extrapolation gate before any larger synthetic MATLAB job. CZ18 uses the
existing fixed-data comparator rather than pretending a synthetic rung is the
same workload.

The hard numerical track is different: the existing clean-room dense MATLAB
oracle in `../oracle/varcomp_kss_dense_oracle.m` and Stata exact driver in
`../oracle/stata_oracle.do` compare plug-in, correction, and corrected targets
at `2e-10`/`2e-9` tolerances. `validate_dense_oracle.py` replays that hard gate.
No maintained or licensed MATLAB source participates in the dense oracle.

## Local gates

Run Python with the repository interpreter:

```bash
./.venv/bin/python -m pytest -q \
  varcomp_kss/benchmarks/prep_bnd1_matlab/tests
./.venv/bin/python -m ruff check \
  varcomp_kss/benchmarks/prep_bnd1_matlab
```

On a host with a working local MATLAB license, the clean-room exact comparison
can run in a disposable directory:

```bash
candidate=$(git rev-parse HEAD)
oracle_dir=$(mktemp -d /private/tmp/prep-bnd1-dense-oracle.XXXXXX)
/Applications/MATLAB_R2024b.app/bin/matlab -batch \
  "addpath('varcomp_kss/benchmarks/oracle'); varcomp_kss_dense_oracle('$oracle_dir','$candidate')"
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q do \
  varcomp_kss/benchmarks/oracle/stata_oracle.do "$oracle_dir" "$candidate"
./.venv/bin/python \
  varcomp_kss/benchmarks/prep_bnd1_matlab/validate_dense_oracle.py \
  --matlab "$oracle_dir/matlab_oracle.csv" \
  --stata "$oracle_dir/stata_oracle.csv" --source-commit "$candidate" \
  --output "$oracle_dir/validation.json"
```

Local MATLAB is not an acceptance prerequisite. On the current development
host it is unavailable or hangs, so the canonical exact-oracle execution is
the source-bound SCC path below using MATLAB R2025b. The SCC path contains only
the repository's independent dense oracle and Stata exact driver; it has no
maintained-comparator root variable and never reads licensed comparator source.

## Source-bound SCC workflow

The bundle builder rejects untracked files and closes over every installed
runtime module, this harness, the MATLAB source contract and verifier, the
process-tree monitor, and corresponding CMG source. Therefore the candidate
must first be a committed full SHA; an uncommitted candidate is an intentional
prerequisite failure.

Build immutable local inputs:

```bash
candidate=$(git rev-parse HEAD)
bundle_out=$(mktemp -d /private/tmp/prep-bnd1-matlab-bundle.XXXXXX)
bundle=$(./.venv/bin/python \
  varcomp_kss/benchmarks/prep_bnd1_matlab/build_bundle.py \
  --root "$PWD" --source-commit "$candidate" --output-dir "$bundle_out")
task_out=$(mktemp -d /private/tmp/prep-bnd1-matlab-tasks.XXXXXX)
mkdir "$task_out/tasks"
./.venv/bin/python \
  varcomp_kss/benchmarks/prep_bnd1_matlab/build_manifest.py \
  --output "$task_out/tasks.tsv" --task-dir "$task_out/tasks" \
  --source-commit "$candidate" --bundle "$bundle"
```

Deploy incrementally without deletion to:

```text
/projectnb/welfgr/varcomp-kss/bundles/<bundle>.tar.gz
/projectnb/welfgr/varcomp-kss/bundles/<bundle>.files.sha256
/projectnb/welfgr/varcomp-kss/bundles/<bundle>/source/
/projectnb/welfgr/varcomp-kss/prep-bnd1-matlab/runs/<run-id>/input/tasks/
```

Extract the archive once into the exact `source/` path and verify its external
manifest there. Copy `tasks.tsv` and the 30 task files into the run input
directory. Create `logs/`, `qacct/`, and an empty job ledger before submission.
The authorized comparator remains at:

```text
/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay
```

For each task, submit the non-submitting `run_pair.sge` wrapper with the same
four slots, 14 GiB per slot, and 02:10 hard wall recorded in the task. The
caller—not this repository wrapper—invokes `qsub -terse`, captures its numeric
job ID, and records the exact task path and SHA-256 in the ledger. Supply only
these environment bindings:

```text
PBM_ROOT=/projectnb/welfgr/varcomp-kss/prep-bnd1-matlab
PBM_RUN_ID=<unique-run-id>
PBM_SOURCE_DIR=/projectnb/welfgr/varcomp-kss/bundles/<bundle>/source
PBM_SOURCE_COMMIT=<candidate-full-sha>
PBM_BUNDLE_SHA256=<bundle>
PBM_BUNDLE_MANIFEST=/projectnb/welfgr/varcomp-kss/bundles/<bundle>.files.sha256
PBM_TASK_FILE=<run>/input/tasks/<experiment>.tsv
PBM_TASK_SHA256=<sha256-of-that-task>
PBM_MATLAB_ROOT=/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay
```

Use `-P welfgr -pe omp 4 -l mem_per_core=14G -l h_rt=02:10:00`, a
run-specific stdout path, and the bundled `run_pair.sge`. Submit independent
tasks in parallel. Do not use an array unless the array task-to-manifest
mapping and per-task qacct collection are added and tested separately.

After every job leaves `qstat`, save its exact `qacct -j <job-id>` output and
run:

```bash
./.venv/bin/python \
  varcomp_kss/benchmarks/prep_bnd1_matlab/validate_scc_job.py \
  --job-dir <run>/<experiment> --qacct <run>/qacct/<experiment>.txt \
  --expected-source-commit "$candidate" --expected-bundle "$bundle" \
  --output <run>/<experiment>/validation.json
```

Acceptance requires scheduler `failed=0`, `exit_status=0`, source/task/input
hashes, all terminal markers, four processors/workers, the five-process
MATLAB RSS proof, exact Stata sample/state/residual gates, MATLAB target
identity, and the maintained PCG classification. Once all 30 validation files
exist, summarize them with:

```bash
./.venv/bin/python varcomp_kss/benchmarks/prep_bnd1_matlab/analyze.py \
  --evidence-root <run> --manifest <run>/input/tasks.tsv \
  --output-dir <empty-summary-directory>
```

Collect task files, validation records, summaries, logs, qacct, resource
receipts, and source identities. Do not collect synthetic input CSVs (they are
deleted with `$TMPDIR`), restricted CZ18 rows, retained-key files, MATLAB
detail files, compiled MEX objects, or licensed MATLAB source.

## SCC clean-room dense-oracle gate

Run this first, before the two-job F64/P20 maintained-MATLAB smoke. The same
content-addressed source bundle contains both
`benchmarks/oracle/varcomp_kss_dense_oracle.m` and
`benchmarks/oracle/stata_oracle.do`. `run_dense_oracle.sge` is deliberately
non-submitting and uses MATLAB R2025b and Stata 19 from the same extraction.

After deploying and verifying the bundle as above, run these commands on SCC:

```bash
candidate=<candidate-full-sha>
bundle=<bundle-sha256>
run_id=<unique-run-id>
pbm_root=/projectnb/welfgr/varcomp-kss/prep-bnd1-matlab
source_dir=/projectnb/welfgr/varcomp-kss/bundles/$bundle/source
bundle_manifest=/projectnb/welfgr/varcomp-kss/bundles/$bundle.files.sha256
run_root=$pbm_root/runs/$run_id

test -d "$source_dir"
test "$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")" = "$candidate"
test "$(sha256sum "/projectnb/welfgr/varcomp-kss/bundles/$bundle.tar.gz" | \
  awk '{print $1}')" = "$bundle"
mkdir -p "$run_root/logs" "$run_root/qacct"

job_id=$(qsub -terse -P welfgr -pe omp 4 \
  -l mem_per_core=4G -l h_rt=00:20:00 \
  -o "$run_root/logs/dense_oracle.txt" \
  -v "PBM_ROOT=$pbm_root,PBM_RUN_ID=$run_id,PBM_SOURCE_DIR=$source_dir,PBM_SOURCE_COMMIT=$candidate,PBM_BUNDLE_SHA256=$bundle,PBM_BUNDLE_MANIFEST=$bundle_manifest" \
  "$source_dir/varcomp_kss/benchmarks/prep_bnd1_matlab/run_dense_oracle.sge")
printf '%s\n' "$job_id" > "$run_root/dense_oracle.job_id"
```

After that exact job has left `qstat`, collect accounting and run the
post-job validator on SCC:

```bash
qacct -j "$job_id" > "$run_root/qacct/dense_oracle.txt"
module purge
module load python3/3.12.4
python3 \
  "$source_dir/varcomp_kss/benchmarks/prep_bnd1_matlab/validate_dense_oracle_scc.py" \
  --job-dir "$run_root/dense_oracle" \
  --qacct "$run_root/qacct/dense_oracle.txt" \
  --expected-source-commit "$candidate" --expected-bundle "$bundle" \
  --output "$run_root/dense_oracle/scc_validation.json"
```

Acceptance requires the hard `2e-10`/`2e-9` exact parity gate, three
application markers, a complete application hash manifest, the archive and
internal/external source-manifest bindings, scheduler `failed=0` and
`exit_status=0`, four granted slots, and matching job/host receipts. Preserve
failed directories and accounting unchanged; repair the harness in a new
commit, then build a new bundle and use a new run ID.

## CZ18 follow-through

After the small matrix and its timing/RSS extrapolation pass, run the fixed
CZ18 P20 comparison with the existing source-bound files under
`../scc/`: `cmg_cz18_matlab_prepare.do`, `cmg_cz18_matlab_run.m`,
`run_cmg_cz18_matlab.sge`, and `validate_cmg_cz18_matlab.py`. The input is the
SCC-only retained DTA with SHA-256
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`.
Run at least three fresh MATLAB processes and interleave them with the
source-order-reversed Stata baseline/candidate CZ18 jobs. Report native input
preparation, MATLAB pool/MEX, command, process and qacct boundaries separately.
P200 is a separate admission decision after P20 memory and wall receipts; it
is not implied by the small P200 synthetic cells.
