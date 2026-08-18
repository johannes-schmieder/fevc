# Benchmark and optional SCC evidence harness

`KSS-NUMOPT-2` adds a separate mandatory scale-evidence matrix. The local
driver `numopt2_local.do` runs the immutable Optimization II source and the
candidate at P200 with one cold and three warm repetitions;
`validate_numopt2_local.py` binds both sources and enforces unchanged results,
work, route, identities, and complete residuals.

The SCC synthetic path uses `scc/numopt2_generate.do`,
`scc/run_numopt2_scale.sge`, `scc/submit_numopt2_scale.sh`, and
`scc/validate_numopt2_scale.py`. It generates exact `W/F=40` strong or weak
designs with two to four cells per worker and one or eight stored rows per
cell. P20 scale rungs measure 61 complete RHSs; target P200 wall forecasts
must use measured action coefficients and state that extrapolation. These
tasks supplement rather than reinterpret the older replicated-CZ harness.
`../model_numopt2.py` consumes only validated aggregate receipts, fits the
1/64--1/16 rungs, tests the central 1/8 holdout, models raw `R/C` separately,
selects among locally calibrated 1/2/4/8/16 batches under the structural
memory contract, and emits a hashed measurement table, coefficients, and
target forecasts.
`summarize_numopt2_matlab.py` pairs externally validated KSS and maintained
MATLAB receipts for identical synthetic task shapes. It compares time and
resources, and places both sets of corrected targets and their absolute gaps
in explicitly descriptive columns. No corrected-value equality gate is
allowed because the target-weight, RNG, and tolerance contracts differ.
The summary parses the maintained MATLAB command's own PCG termination line;
timing and process-tree RSS remain reported when MATLAB hits its iteration
cap, but that corrected result is explicitly marked not numerically accepted.
MATLAB tasks are admitted from the measured smallest-rung process-tree RSS,
scaled by the largest dimension ratio with 20 percent headroom under 128 GiB.

`KSS-STREAMLINE-1` makes local correctness and numerical tests the active
development gate. No benchmark ladder or SCC run is required to close that
process milestone. The KSS-PROD-1 material below documents completed
historical qualification and remains useful calibration evidence; its fixed
resources, thresholds, and phase DAG do not govern current command work.

For a new optional scale diagnostic, use `scc/submit_kss_scale.sh` with an
independently chosen fixture, copy count, probes, slots, memory per core, Stata
processor count, and wall time. `replicated_blocks` is the canonical fixture
name; `well_connected` is an input alias. `scc/validate_kss_scale.py`
distinguishes a scientific pass from a collected typed diagnostic and writes
a self-contained receipt that never unlocks another run.

`scc/deploy_scale_bundle.sh` now creates a
`KSS-STREAMLINE-SOURCE-BUNDLE-V1` bundle and marks its run directory
`KSS-STREAMLINE-1`. The active bundle contains the installed command, the
fixture builder, and the independent diagnostic submitter/wrapper/validator.
It deliberately omits the slow historical K1 comparison and the MATLAB scale
harness; those remain in the repository as reusable evidence and standalone
historical tools.

The original KB6 drivers qualify the internal `varcomp_kss` point-estimation
package without restricted project data. KSS-NUMOPT-1 also contains a
separately named, owner-authorized SCC-only Separations wage benchmark. These
are development and validation tools, not runtime dependencies.

## Evidence contract

Every SCC run is bound to one clean Git commit and one directory:

```text
/projectnb/welfgr/varcomp-kss/runs/<run-id>/
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
The adapter requests four slots at 16 GiB per slot because Stata 19 may launch
a Java import helper with a separate 2 GiB virtual-memory reservation; the
larger request is an admission safeguard and does not change the sample.
It writes the same four numeric columns to a separately checksum-bound CSV so
that a fresh MATLAB reference and the `b1` and `cmg` jobs can run concurrently
on one audited input. The Stata routes consume the DTA and MATLAB consumes the
CSV; both artifacts are derived in one deterministic adapter invocation.
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
With `--include-matlab`, the validator also checks the maintained MATLAB and
CMG source hashes, seed, probes, four-target identity, projection, timing, RSS,
SCC accounting, input dimensions, and exact retained-match overlap with B1.
The MATLAB values remain descriptive because its legacy finite projection and
language-specific probe stream differ from API 17. The comparison job checks
B1/CMG match equality directly on a MATLAB-derived label and checks MATLAB
overlap when a detail file belongs to the same label.

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
bash varcomp_kss/benchmarks/scc/submit_one.sh \
  /projectnb/welfgr/varcomp-kss/runs/<run-id> "$PWD" <commit> portability

bash varcomp_kss/benchmarks/scc/submit_one.sh \
  /projectnb/welfgr/varcomp-kss/runs/<run-id> "$PWD" <commit> oracle

bash varcomp_kss/benchmarks/scc/submit_one.sh \
  /projectnb/welfgr/varcomp-kss/runs/<run-id> "$PWD" <commit> \
  smoke 5000 250 40 20260814
```

After a job finishes, collect its accounting record with:

```bash
bash varcomp_kss/benchmarks/scc/collect_qacct.sh \
  /projectnb/welfgr/varcomp-kss/runs/<run-id> smoke
```

Run the validator on either the remote directory or a byte-for-byte local
copy:

```bash
./.venv/bin/python varcomp_kss/benchmarks/validate_scc.py \
  --run-dir <run-directory> --expected-commit <commit> \
  --jobs portability oracle smoke
```

The production-size target remains a qualification question.  Passing the
synthetic ladder establishes portability and bounded numerical behavior on
the tested public design; it does not establish application assumptions,
econometric inference, or performance on restricted data.

## Historical KSS-PROD-1 SCC qualification DAG

KSS-PROD-1 adds a separate content-addressed SCC harness. It does not replace
or reinterpret any completed KB6 or KSS-NUMOPT evidence above. The deployment
builder reads the explicit tracked-file allowlist in
`prod_bundle_allowlist.txt`; it never archives the repository root, follows a
symlink, or performs a deleting synchronization. The deterministic archive
SHA-256 names one immutable bundle under
`/projectnb/welfgr/varcomp-kss/bundles/<sha256>/`. Every job in a run verifies that
same archive checksum and every allowlisted source-file checksum before doing
work. `run.metadata.json`, `bundle.sha256`, the submission ledger, per-job IDs,
logs, outputs, resource reports, and qacct records remain run-scoped.

`prod_experiments.tsv` is the frozen DAG. It includes:

- source-tree routed JLA smokes and separate normal `net install` public
  `algorithm(auto)` and forced-JLA/CMG smokes in Stata 18 and 19, plus
  exact four-slot license probes;
- a 65,536-vertex CMG hierarchy/workspace stress cell at four processors;
- a public automatic-selector smoke;
- pure-Stata preparation of CZ24, CZ25, and CZ18 from checksum-bound raw wage
  DTAs, followed by exact CZ24/CZ25 retained-match comparisons with the
  maintained MATLAB detail artifacts;
- cold and warm automatic CZ24/CZ25 replications plus fixed-seed forced B1
  and CMG equality runs;
- an automatic-CMG CZ18 preflight followed by concurrent forced-CMG and public
  automatic-route calibrations at explicit widths 8 and 16 and at automatic
  width. Every P20/P40 cold/warm cell has three independent, parallel SCC
  replicas, so setup and marginal per-probe time are summarized without
  serializing otherwise independent experiments;
- a calibration selector; and
- a separately authorized CZ18 full-200 run, three identical 20-probe
  larger-stress calibrations released in parallel after it, and a separately
  admitted larger-stress full-200 run. The full CZ18 job saves its
  production-retained rows in the SCC run only.
  The stress input duplicates that retained graph and connects the copies by a
  deletion-safe four-edge cycle, so every connector firm belongs to the
  accepted component and the case reaches at least twice the retained, firm,
  and hybrid dimensions.

Each phase is a separate submission boundary. Calibration requires a complete
`preflight.pass` evidence certificate. Production requires a complete
`calibration.pass` certificate, the exact checksum of the accepted selector,
and the literal `--authorize-production KSS-PROD-1` argument. The production
phase runs full CZ18 and then releases all three stress calibrations together;
their common dependency does not serialize them. The final stress phase
requires the independently revalidated `production.pass` and the same owner
authorization. The selector admits only a
public automatic-route configuration whose conservative projection is no
more than 42,600 seconds. This is a process-time ceiling that leaves the
wrapper's 600-second margin inside SCC's 12-hour boundary; it is not the
submitted timeout. Each candidate has 12 measurements: three independent
replicas of every cold/warm P20/P40 cell. Cell medians produce a typical-time
projection used only to rank admissible public-auto candidates. Admission and
the production timeout use this all-row upper envelope:

```text
beta = max(0, correction_seconds/probes over all 12 rows,
           (max(cold40)-min(cold20))/20,
           (max(warm40)-min(warm20))/20)
alpha = max(0, command_seconds - probes*beta over all 12 rows)
cold_overhead = max(0, qacct_wall-command_seconds over all cold rows)
headroom = max(120, cold_overhead+120)
ceil(max(300, 1.25*alpha + 1.5*200*beta + headroom))
```

The correction timer must equal leverage plus target time, allowing only a
named `2e-7` relative diagnostics tolerance for separately serialized Stata
CSV fields. This construction
does not treat an inverted noisy P20/P40 pair as zero marginal cost and keeps
extra headroom for the full job's retained-DTA save. All 72 calibration jobs
depend only on the accepted CZ18 preflight and are otherwise independent; the
selector is the sole barrier after they finish. Every replica binds hostname,
queue, architecture, CPU model, logical CPU count, CPU time, wall time, memory,
and the complete scientific configuration. Heterogeneous nodes are retained
in the summaries. An inverted timing is labeled unexplained variability unless
recorded host/load/accounting evidence identifies a cause; it is never
discarded as presumed contention. When the automatic candidates have identical
12-row node-class multisets, their median summaries rank typical performance;
otherwise the conservative upper-envelope timeout and projection rank them.
Forced-CMG candidates are corroborating backend evidence and their scientific,
RNG, graph, and complete-RHS certificates must equal the matching automatic-CMG
cells. Production selection is restricted to the installed public
`preconditioner(auto)` route when it selected CMG, and the full run receives
that candidate's conservative timeout and selected width explicitly.

Each calibration process is capped at 5,400 seconds, with a 6,000-second SCC
hard limit. The initial fully parallel grid showed that the former 3,600-second
cap censored 17 warm cells with accounting-confirmed exit status 124. The
larger cap remains bounded and changes no estimator, probe, seed, tolerance,
route, or residual semantics; it permits all repetitions to enter the robust
summary and conservative timeout envelope.

CZ24/CZ25 fixed-sample jobs retain diagonal equality and performance evidence.
CZ18 is not gated on an unreasonable large B1 run: its preflight,
calibrations, full run, and larger stress must select multilevel CMG and reach
a terminal of at most 6,144 vertices. The stress jobs choose their batch
automatically for the doubled graph; the deterministic forecast normally
reduces width 16 to width 8 there. The selector bounds each P20 stress
calibration by twice the conservative automatic batch-8 CZ18 envelope, with a
1,800-second floor and 42,600-second ceiling. After all three independent
stress calibrations finish, the full-stress admission uses

```text
alpha = max_r max(0, qacct_wall_r - correction_seconds_r)
beta = max_r correction_seconds_r/20
projected = ceil(max(300, 1.25*alpha + 1.5*200*beta + 120))
timeout = max(1,800, projected)
```

The full stress job is submitted only after this robust upper envelope is at
most 42,600 seconds. Its scheduler request is the measured timeout plus the
wrapper's 600-second termination margin. Median wall and correction times are
reported descriptively; they do not replace either maximum in admission.
Host, queue, architecture, CPU class, logical CPUs, CPU time, wall time, and
RSS are recorded for every repetition. Timing inversions remain unexplained
variability unless those records identify a cause; concurrent `qsub`
submission is never treated as evidence of contention.

SCC capability run `20260816T035454Z-c3cb6a3` measured the cluster limit before
the production DAG was frozen: Stata 18 rejected an eight-slot module load and
Stata 19, while MP-enabled, reported four actual processors for an eight-slot
request (jobs `7191050` and `7191052`). Both four-slot checks passed. The SCC
production DAG therefore binds every numerical cell to four processors. Local
Stata/MP8 tests provide the eight-processor behavior measurement; the harness
does not disguise the SCC license cap as numerical coverage.
The same capability run's four-slot hierarchy job `7191053` completed 65,536
vertices and 133,014 edges in nine levels, reaching a 67-vertex terminal with
664.016 MiB qacct `maxvmem`; its reusable workspace was 1.172 times slower
than ordinary batched application.

`local_processor_scaling.do` supplies the corresponding local MP4/MP8 gate.
It generates one deterministic moderate design, runs the public estimator at
batch widths 32 and 64 (and optionally 128), and requires identical terminal
RNG state, estimates, selected route, and complete residual acceptance across
all cells. Run the driver under the platform resource-accounting command so
the suite has one process-level peak-RSS certificate; Stata does not expose a
portable per-cell peak RSS scalar.

The first successful real CZ18 preflight at source `b487f07` took 1,490
command seconds. Subsequent source-bound preflights use a 2,100-second command
limit: `ceil(1.25*1490+120)` rounded upward. This is a measured safety bound,
not a numerical acceptance threshold; the scheduler adds a separate
ten-minute termination margin.

Every estimator declares at most 56 GiB and the validator rejects either
declared or observed memory above the run policy. The data manifest names raw
wage DTAs, SHA-256 values, preparation mode, the Separations commit, and the
two benchmark-only MATLAB detail artifacts. The first preflight submission
copies it to `input/data_manifest.tsv`, records its checksum, and makes it
read-only. Every job and every later phase verifies those exact bytes. The
prepared DTA and its checksum are produced once per dataset inside the run;
no MATLAB-prepared sample is an estimator input.

CZ18's registered batch-8 scratch forecast is 7,447,296,000 bytes. Under the
production rule that probe scratch may use at most 35% of 56 GiB, the batch
budget is 21,045,339,750 bytes. The deterministic linear forecasts are:

| Batch width | Scratch forecast (bytes) | Calibration treatment |
|---:|---:|---|
| 8 | 7,447,296,000 | measured |
| 16 | 14,894,592,000 | measured |
| 32 | 29,789,184,000 | infeasible; not submitted |
| 64 | 59,578,368,000 | infeasible; not submitted |
| 128 | 119,156,736,000 | infeasible; not submitted |

The selection certificate records these forecasts and the 35% budget. Widths
32, 64, and 128 are deterministic infeasibility evidence, not expected-OOM
jobs and not calibration failures.

`hold_jid` controls same-phase scheduling only; accepted earlier phases are
bound by their immutable phase certificates rather than retired scheduler job
IDs. Each consumer additionally requires the
producer's source/bundle/data-manifest-bound wrapper marker and retries the
complete `qacct` query until it proves `failed=0` and `exit_status=0`. The
calibration selector independently rechecks every calibration CSV, full RHS
certificate, wrapper, qacct record, input identity, route, batch, processors,
seed, tolerance, target identity, and cold/warm result before emitting a
content digest over the accepted evidence.

The phase validator writes a checksum manifest over the complete accepted
phase evidence, including restricted per-RHS and application evidence without
copying those files. The phase certificate binds that manifest. Privacy-safe
collection rechecks every remote checksum before transferring aggregates.

Local, non-submitting gates are:

```bash
./.venv/bin/python varcomp_kss/benchmarks/build_prod_bundle.py \
  --root "$PWD" \
  --allowlist varcomp_kss/benchmarks/prod_bundle_allowlist.txt \
  --source-commit "$(git rev-parse HEAD)" \
  --check-only
./.venv/bin/python varcomp_kss/benchmarks/validate_prod_scc.py --static
bash -n varcomp_kss/benchmarks/scc/*.sh varcomp_kss/benchmarks/scc/*.sge
```

From a clean committed local checkout, stage the three-row manifest on SCC,
deploy one lean immutable bundle, and submit only preflight:

```bash
cp varcomp_kss/benchmarks/prod_data_manifest.example.tsv \
  /private/tmp/kss-prod-data-manifest.tsv
# Fill the three registered SCC paths and hashes, then:
rsync -av /private/tmp/kss-prod-data-manifest.tsv \
  scc:/projectnb/welfgr/varcomp-kss/manifests/<run-id>.tsv
bash varcomp_kss/benchmarks/scc/deploy_prod_bundle.sh "$PWD" <run-id>

ssh scc bash /projectnb/welfgr/varcomp-kss/bundles/<bundle-sha>/source/varcomp_kss/benchmarks/scc/submit_prod_dag.sh \
  /projectnb/welfgr/varcomp-kss/runs/<run-id> \
  /projectnb/welfgr/varcomp-kss/bundles/<bundle-sha> <bundle-sha> \
  /projectnb/welfgr/varcomp-kss/manifests/<run-id>.tsv preflight
```

The submitter explicitly loads SCC's `python3/3.12.4` module before running
the bound validator; it does not use the login node's Python 3.6 default.

After all phase jobs leave `qstat`, collect qacct for the phase and run the
validator shown below with `--write-pass`. Submit `calibration` with the same
command only after `preflight.pass` exists. Submit production only after
`calibration.pass` exists and append
`--authorize-production KSS-PROD-1`. After `production.pass` exists, submit
the measured `stress` phase with the same authorization. Never collapse the
four phase boundaries into one submission.
Before advancing a phase, the submitter reruns the bound prior-phase validator,
checks the certificate's evidence-manifest digest, and verifies every recorded
evidence checksum from the run root.

After authorized remote execution, collect one accounting record per
experiment with `scc/collect_prod_qacct.sh`; it retries accounting lag and
writes a structurally complete record atomically. Then run, on SCC, for the
completed phase:

```bash
module load python3/3.12.4
python3 <bundle>/source/varcomp_kss/benchmarks/validate_prod_scc.py \
  --run-dir /projectnb/welfgr/varcomp-kss/runs/<run-id> \
  --bundle-sha <bundle-sha256> --source-commit <commit> \
  --data-manifest /projectnb/welfgr/varcomp-kss/runs/<run-id>/input/data_manifest.tsv \
  --phase preflight --write-pass
```

Use `--phase calibration --write-pass` before production submission,
`--phase production --write-pass` before the measured full-stress admission,
and `--phase stress --write-pass` at qualification. The validator binds plan
fields, Stata version/flavor, requested and actual processors, probes, seed,
tolerance, batch, route, raw/prepared hashes, all RHS records, target
identities, cross-batch and cross-route equality, qacct, stage timings, memory
forecasts, and peak RSS. With `tolerance(1e-10)`, the registered complete
residual gate remains `max(1e-11,10*tolerance)=1e-9`; the phase certificate
also reports whether every residual was at or below the requested tolerance.

Privacy-safe collection consists only of aggregate CSVs, phase certificates,
their evidence-checksum manifests, the aggregate stress-projection
certificate, complete qacct records, resource summaries, and identity receipts.
`scc/collect_prod_summary.sh` enforces an explicit aggregate-file allowlist and
creates a checksum inventory without `--delete`. Never collect
`input/data_manifest.tsv`, raw wage data, prepared DTAs, MATLAB detail,
`retained_matches.csv`, full RHS CSVs, application logs, or
`retained_sample.dta` to this repository. Cold end-to-end time is preparation
wall time plus the cold estimator-process wall time; warm estimator time begins
after the shared prepared DTA is available.

## Maintained MATLAB phase profiling

`separations_matlab_phase_profile.m` is a source-bound descriptive profiler
for the unmodified maintained MATLAB workflow. It runs one cold call, one
profiled warm call, and one unprofiled warm call in the same R2025b process.
It restores captured client and parallel-worker RNG states before each call,
but the upstream `parfor` schedule is not fixed and R2025b profiling can change
the assignment of draws to workers. The evidence therefore requires exact
retained worker-firm keys and row counts. It recomputes the historical `1e-5`
scaled target-drift flag and records full-detail and target hashes as
diagnostics rather than claiming bitwise legacy replay. Target drift is not a
hard gate because the maintained interface exposes no target-specific Monte
Carlo standard errors from which to register a simultaneous replay bound.
Direct timers separately record MEX setup, input import, pool setup, all three
maintained calls, result serialization, and pool teardown. MATLAB's line
profiler attributes self-time within registered line ranges for selection,
residual collapse, leverage, variance estimation, reporting, and maintained
serialization. Those line aggregates are diagnostic attribution, not an
independent wall-clock decomposition of child work or parallel wait time.

The SCC wrapper accepts only maintained commit
`8b957ffeb10b8465a3584fceb0265cccc48379e1`, the core hash registered with
632 newline bytes and 633 physical source lines, and the registered digest
over all 184 files in the maintained `codes/` and
`CMG/` runtime snapshot, exact CMG-family hashes, an input CSV already under a
KSS run, and the source identity of the lean bundle containing the profiler.
The upstream commit identifies the source snapshot; the SCC copy does not
contain Git metadata, so the complete runtime-tree digest is the executable
identity gate. Its null-delimited path ordering is canonicalized under the C
locale on login and compute nodes. The job uses four slots, a measured complete-process
projection, and a one-hour ceiling. The only persistent outputs are aggregate CSV/JSON, an
identity-bound pass marker, the application log, GNU-time resource report,
submission receipt, and qacct. Temporary detailed results are hashed, reduced
to retained-key hashes and row counts, and deleted on the compute node; no
row-level MATLAB output is collected.

Submit and validate one checksum-bound input as follows:

```bash
bash <bundle>/source/varcomp_kss/benchmarks/scc/submit_matlab_phase_profile.sh \
  <run-dir> <bundle>/source <source-commit> <bundle-sha> <label> \
  <prepared.csv> <input-sha> <maintained-LeaveOutTwoWay-root> \
  8b957ffeb10b8465a3584fceb0265cccc48379e1 200 8675309 \
  <projected-complete-process-seconds> <projection-basis> \
  <core-sha> <cmg-entry-sha> <cmg-mex-family-sha> <cmg-solver-family-sha>

bash <bundle>/source/varcomp_kss/benchmarks/scc/collect_matlab_phase_profile.sh \
  <run-dir> <label>

python3 <bundle>/source/varcomp_kss/benchmarks/validate_matlab_phase_profile.py \
  --run-dir <run-dir> --label <label> \
  --expected-source-commit <source-commit> \
  --expected-bundle-sha256 <bundle-sha> --expected-input-sha256 <input-sha> \
  --expected-upstream-commit 8b957ffeb10b8465a3584fceb0265cccc48379e1 \
  --expected-core-sha256 <core-sha> --expected-cmg-sha256 <cmg-entry-sha> \
  --expected-cmg-mex-sha256 <cmg-mex-family-sha> \
  --expected-cmg-solver-sha256 <cmg-solver-family-sha> \
  --expected-profiler-sha256 <profiler-sha> \
  --expected-projected-seconds <seconds> \
  --expected-projection-basis <projection-basis>
```
