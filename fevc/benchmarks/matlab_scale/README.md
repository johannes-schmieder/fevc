# KSS Matlab descriptive scale benchmark

This package benchmarks the checksum-bound maintained LeaveOutTwoWay MATLAB
implementation at the KSS-SCALE fixture sizes. It does not modify, copy, or
replace the maintained estimator. It is a development comparator and never a
runtime dependency of `fevc`.

The source contract registers scale `1`, `2`, `4`, `8`, or `16` and topology
`well`, `well_connected`, or `ring`. The current preparation, case builder,
and submitter accept only `fixed` retained-sample cases. A future selection
benchmark needs a separate raw-input path; this fixed path rejects selection
before `qsub`. Every estimator call uses JLA match deletion, 200 probes, and a
four-worker pool. Inputs must have four numeric columns in this order:

1. dense positive worker ID;
2. dense positive firm ID;
3. period/order variable;
4. outcome.

The input must already be ordered by worker, period, and firm. The driver
checks that order in bounded one-million-row tiles instead of allocating a
second row-sized sort matrix.

## Fixed retained-sample preparation

`prepare_fixed_sample.do` is the pure-Stata preparation path for the retained
CZ18 base and its deterministic scale fixtures. The source DTA must already be
the independently selected, fixed retained sample and must contain numeric
`worker`, `firm`, `period`, `y_minus_xb`, and unique `observation_key`
variables. Preparation performs no new leave-out sample selection. It
deterministically densifies worker and firm IDs, treats a unique worker--firm
coordinate as one match, and writes the four-column MATLAB input ordered by
worker, period, and firm. Each retained DTA row is one literal physical
observation; this preparation path does not reinterpret frequency weights or
expand a weighted aggregate.

The admitted preparation cases are exactly:

- scale 1 with topology `well` (the unreplicated retained CZ18 base);
- scales 2 and 4 with topology `well_connected`;
- scale 2 with topology `ring` as a weak-connectivity stress.

The replicated cases use `kss_scale_fixtures.do` and remain connected and
match-deletion safe. `well_connected` is the ordinary scale comparator;
`ring` is reported separately and is not an ordinary scaling substitute.
Connector rows have zero outcome and deterministic within-worker periods.

Preparation writes `input.csv`, `prepare.csv`, and
`retained_keys.canonical.txt` under the run-scoped preparation directory. The
key file has no header and contains strictly sorted UTF-8 lines
`<integer-worker>,<integer-firm>\n`. `build_prepare_receipt.py` streams that
file, validates the exact bytes and dense dimensions, and records its SHA-256
in `wrapper.json`. It also records source/input/bundle hashes; input and output
rows, workers, firms, and matches; fixture graph diagnostics; Stata stage
times; wrapper wall, CPU, and peak RSS; requested and actual slots; and
requested and actual Stata processors. Input staging and output promotion are
timed separately from Stata's load, normalization, fixture, and export stages.

`run_prepare_fixed_sample.sge` is one scalar SGE job and one Stata/MP 19
process. It reserves 14 `omp` slots at 4 GiB per slot for memory, I/O, and
shared-node capacity while setting and verifying exactly four Stata
processors. The distinction is explicit in every receipt. The input DTA and
all preparation work are staged under `$TMPDIR`; row-level outputs remain in
the SCC run directory and must not be collected into this repository.
`validate_scc_job.py` must accept the preparation scheduler, application, and
output layers and write `prepare/acceptance.json` before case construction.

## Scientific comparison boundary

The maintained entry point returns corrected worker, firm, and covariance
targets but does not expose its plug-in components. The case therefore binds
an accepted Stata KSS-SCALE reference receipt and its four plug-in components.
A generic aggregate, fixture receipt, manually selected source kind, or
separately supplied retained-key hash is rejected. The only production
producer is `build_reference_receipt.py`. It replays the immutable-bundle KSS
validator and derives the reference kind and estimator semantics from one
accepted KSS run. The reference bytes must use schema
`kss_matlab_scale_reference_v1`, report `PASS`, and bind the fixed label,
source commit, bundle, retained key, dimensions, source and prepared inputs,
JLA, match deletion, no controls, literal physical rows, uniform stored-row
target mass, 200 probes, the case seed, and all four plug-in components. Its
provenance hashes the KSS admission receipt, validation certificate, driver,
validator, source manifest, qsub-verify scheduler request, qacct, job-ID file,
node receipt, reservation, summary, accepted preparation receipts, source
input, prepared input, and retained key. The case and final validators recheck
these fields, the receipt SHA-256, and the accounting identity. They do not
infer a MATLAB plug-in from corrected output.

Corrected MATLAB values are descriptive. Every call must satisfy

```text
total = worker + firm + 2 * covariance
```

but no corrected-estimate equality is tested across languages, processes, or
repetitions. This is explicit in every aggregate and comparison receipt as
`IDENTITY_ONLY_NO_EQUALITY_GATE`.

The maintained retained-key hash and all retained dimensions must equal both
the preparation and reference receipts. There is no current selection
submission route.

## Cold and warm processes

`matlab_scale_cold.m` launches exactly one maintained estimator call in a
fresh MATLAB process. There is no warmup call. For an independently selected
sample, that one call is line-profiled against the registered source ranges so
selection self-time is available; its call time therefore includes profiler
overhead.

`matlab_scale_warm.m` launches one warmup followed by at least three
unprofiled measured calls. When sample selection is requested, only the
warmup is profiled. The unprofiled measured-call distribution is the core
warm benchmark.

Both processes restore the captured client and worker RNG state before each
call. The maintained `parfor` scheduler does not provide execution-path-
independent probe assignment, so target hashes and scaled target drift remain
diagnostics. Retained keys and dimensions are hard deterministic gates.

## Bound case metadata

Each job consumes a `kss_matlab_scale_case_v2` JSON object. It contains:

- `label`, `scale`, `topology`, `sample_mode`, `seed`, `probes`, and
  `warm_repetitions`;
- `source`: repository commit, bundle SHA-256, maintained upstream commit,
  complete runtime-tree SHA-256, core/CMG/hierarchy/solver hashes, and the
  benchmark source-contract hash;
- `input`: SHA-256, rows, workers, firms, matches, dense-ID contract, and
  order contract;
- `preparation`: wrapper and SCC-acceptance SHA-256 values plus validated
  label, scale, topology, source commit, executing bundle, source input,
  prepared input, retained key, source dimensions, and output dimensions;
- `reference_sample`: derived KSS source kind, receipt SHA-256, retained-key
  SHA-256, matching input binding, dimensions, estimator options, complete
  accepted-run provenance, and the four reference plug-in values.

The retained-key digest is SHA-256 over unique worker--firm keys sorted
numerically by worker and firm and encoded as UTF-8 lines
`<integer-worker>,<integer-firm>\n`. This is the same canonical digest used by
the driver after each maintained call.

The fixed source snapshot is in `source_contract.json`. Before submission,
`verify_submission.py` rejects nonfixed cases and checks their exact input,
preparation receipts, source commit, and executing bundle. Both it and
`make_case.py` replay the preparation acceptance from the actual request,
submission, qsub-verify scheduler request, job-ID, qacct, wrapper, and output
bytes named by that acceptance; hash strings or a shallow `PASS` layer are
insufficient. The replay reruns the preparation producer's CSV, canonical-key,
fixture-arithmetic, processor, Stata/MP, marker, input-hash, and GNU-time
validation rather than trusting a rehashed wrapper. On the compute node,
`verify_case.py` repeats those checks against the original accepted evidence
and recomputes the complete maintained runtime-tree identity plus its core and
CMG-family hashes before MATLAB starts. `make_case.py` constructs the binding
from the prepared CSV, replayed preparation evidence, and converter-produced
KSS reference receipt without loading row data into Python.

## Timing and resource evidence

The MATLAB aggregate separates:

- first-statement executable startup from the wrapper process timestamp;
- import;
- tiled input/order validation;
- selection self-time or fixed-sample validation;
- retained-output validation;
- pool startup;
- run-local MEX compilation/setup;
- warmup and measured estimator calls;
- aggregate serialization;
- pool teardown.

The execution wrapper adds node-local input staging, module setup, complete
process wall time, user and system CPU, and GNU `time -v` peak RSS.
`monitor_process_tree.py` also samples and sums RSS across the MATLAB launcher,
client, pool workers, and current descendants. MATLAB R2025b uses the
documented `matlabProcessID` API on the client and inside `spmd`, paired with
`spmdIndex`, to write an atomic identity artifact naming the client and four
ordered worker PIDs. The monitor accepts only when all five named PIDs appear
together as descendants in a summed-RSS sample and binds the identity bytes.
Individual procfs entries that disappear with `ENOENT`, `ESRCH`, or another
per-process `OSError` during a scan are skipped; the named-PID observation and
positive-sample gates still determine whether the complete monitor passes.
The application aggregate independently reports the same PIDs and exactly four
pool workers. The wrapper, SCC acceptance gate, and final validator enforce
these identities. Summed process-tree peak RSS, GNU RSS, and `qacct maxvmem`
remain separate fields and must fit the 56-GiB reservation.

`run_matlab_scale.sge` is deliberately non-submitting. It stages input and
case bytes under `$TMPDIR`, compiles MEX files there, caps the client and each
worker's numerical threads, and retains no row-level detail artifact. Its
12-hour header is only a hard ceiling. The submitter always overrides it with
an explicit measured application timeout plus a ten-minute wrapper reserve
and records the measurement-basis token in both submission and wrapper
receipts.

## Scalar SCC submission

`submit_matlab_scale.sh` submits exactly one requested scalar stage per invocation.
It has no arrays, `hold_jid`, downstream submission, reducer, estimator
sharding, or automatic retry. Preparation must finish and validate before the
owner separately creates a checksum-bound `case.json` and submits either
MATLAB process. Cold and warm are independent descriptive jobs and may run in
either order or concurrently once their common prepared input and case exist.

Cold and warm MATLAB jobs request four `omp` slots at 14 GiB per slot (56
GiB total), start exactly one four-worker pool, and cap the client and each
worker's numerical-library threads to one. Their resource receipts keep this
allocation distinct from the preparation policy of 14 slots at 4 GiB per slot
with exactly four Stata processors. SCC validation hard-gates these stage
shapes; a consistently rewritten 8-by-7-GiB request is invalid even though it
also totals 56 GiB.
Both execution wrappers reject a numeric `SGE_TASK_ID`, and the submitter
accepts only an all-digit scalar result from `qsub -terse`. Accounting later
requires the same job ID in the job-ID file, submission receipt, wrapper, and
`qacct`, with `taskid undefined`.

Before the actual submission, the submitter runs `qsub -verify` with the same
project, PE, slots, memory, hard wall, output, environment, and script
arguments. It preserves the raw BU SCC output as
`submissions/matlab_scale_<label>_<stage>.scheduler_request.txt`; the
submission receipt hashes it. Acceptance parses that artifact to bind the
hard resource list, PE name and range, project, output and script paths, and
KSS environment paths. The separate qacct gate binds the granted project,
PE, slots, scalar task ID, job ID, host, exit status, wall, CPU, and maxvmem.
It does not require a qacct `category` field, which BU accounting may omit.

The preparation form is:

```bash
submit_matlab_scale.sh prepare \
  RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 \
  SOURCE_MANIFEST LABEL RETAINED_DTA RETAINED_DTA_SHA256 \
  SCALE TOPOLOGY TIMEOUT_SECONDS TIMEOUT_BASIS
```

The MATLAB form is:

```bash
submit_matlab_scale.sh cold \
  RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 \
  SOURCE_MANIFEST LABEL INPUT_CSV INPUT_SHA256 CASE_JSON CASE_SHA256 \
  CONTRACT_SHA256 MATLAB_ROOT PREPARATION_RECEIPT \
  PREPARATION_ACCEPTANCE TIMEOUT_SECONDS TIMEOUT_BASIS

submit_matlab_scale.sh warm \
  RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 \
  SOURCE_MANIFEST LABEL INPUT_CSV INPUT_SHA256 CASE_JSON CASE_SHA256 \
  CONTRACT_SHA256 MATLAB_ROOT PREPARATION_RECEIPT \
  PREPARATION_ACCEPTANCE TIMEOUT_SECONDS TIMEOUT_BASIS
```

`TIMEOUT_SECONDS` is the measured application boundary, not an unexplained
ceiling. `TIMEOUT_BASIS` is a filesystem-safe token naming the lower-scale
measurement or fitted timing rule. The submitter accepts 300--42,600 seconds,
adds exactly 600 seconds for staging and receipt finalization, and keeps the
scheduler request within 12 hours. It verifies immutable source bundle,
source commit, input, case, preparation, and source-contract hashes before
submission. Each invocation writes a pre-submission request receipt, captures
the scheduler request through `qsub -verify`, then records the scalar job ID
and an accepted-submission receipt after `qsub -terse` succeeds.

The submitter does not create `case.json`. First save the completed job's
accounting bytes at the exact `qacct_path` in its request receipt and run:

```bash
./.venv/bin/python fevc/benchmarks/matlab_scale/validate_scc_job.py \
  --stage prepare --request <prepare.request.tsv> \
  --submission <prepare.tsv> \
  --scheduler-request <prepare.scheduler_request.txt> \
  --job-id-file <prepare.job_id> \
  --qacct <prepare-qacct.txt> --job-dir <prepare-dir> \
  --output <prepare-dir>/acceptance.json
```

Next convert one fully accepted, compressed, 200-probe KSS-SCALE run whose
recorded option tuple is unweighted, uniform-target, match deletion with no
separate deletion variable:

```bash
./.venv/bin/python fevc/benchmarks/matlab_scale/build_reference_receipt.py \
  --kss-run-dir <kss-run-dir> \
  --kss-source-dir <immutable-kss-bundle>/source \
  --experiment-id <accepted-kss-experiment-id> \
  --kss-certificate <experiment-dir>/certificate.json \
  --kss-admission-receipt <experiment-dir>/admission_receipt.tsv \
  --source-input-dta <exact-kss-source-input.dta> \
  --preparation-receipt <prepare-dir>/wrapper.json \
  --preparation-acceptance <prepare-dir>/acceptance.json \
  --output <prepare-dir>/kss_reference.json
```

The converter derives rather than accepts the reference kind. It verifies the
actual admission, certificate, qacct, scheduler, source-bundle, input,
retained-key, and option bytes and fails closed if any binding differs. Then
use `make_case.py` with `input.csv`, preparation `wrapper.json`, preparation
`acceptance.json`, and this converter-produced reference receipt. The case
builder derives dimensions and retained keys from those accepted bytes; it
accepts no manual dimension, key hash, or reference-kind override.

After a cold or warm job, save its exact `qacct` record and run
`validate_scc_job.py` with `--stage cold` or `--stage warm`, adding the case,
case SHA-256, and source-contract arguments. This hard-gates job ID, nonarray
state, requested and granted project/PE/slots, requested `h_rt` and
`mem_per_core`, wall, CPU, `maxvmem`, hostname, wrapper resources,
process-tree RSS, application signals, and output hashes.

## Outputs and failures

One job directory has:

```text
application.txt
identity.json
process_identity.json
resources.txt
process_tree_rss.json
wrapper.json
acceptance.json          # after qacct validation
output/aggregate.json
output/calls.csv
output/wrapper.pass       # success only
```

The preparation directory has:

```text
application.txt
resources.txt
input.csv                         # row-level; SCC only
input.csv.sha256
retained_keys.canonical.txt       # match keys; SCC only
retained_keys.sha256
prepare.csv
prepare.stata.pass
wrapper.json
wrapper.pass                      # success only
acceptance.json                   # after qacct validation
```

`wrapper.json` is written after success, application failure, timeout, or a
source/input rejection. It records the failure stage, exit status, timeout or
signal, partial aggregate failure, resource evidence, and hashes of every
available artifact. The application log remains intact. Failed runs must use
new job directories; do not overwrite or weaken their inputs.

After separately completed cold and warm jobs, validate with:

```bash
./.venv/bin/python fevc/benchmarks/matlab_scale/validate.py \
  --case <case.json> --case-sha256 <sha256> \
  --contract fevc/benchmarks/matlab_scale/source_contract.json \
  --cold-job-dir <cold-job-dir> --warm-job-dir <warm-job-dir> \
  --cold-acceptance <cold-job-dir>/acceptance.json \
  --warm-acceptance <warm-job-dir>/acceptance.json \
  --reference-aggregate <self-bound-reference-receipt> \
  --output <comparison.json>
```

The reference and both SCC acceptance receipts are mandatory. The validator
replays each acceptance against its canonical request, submission, job-ID,
qacct, wrapper, and output source bytes. It also rechecks the reference's
retained key, input binding, KSS provenance, estimator options, probe count,
seed, dimensions, four plug-in values, and accounting identity.

No SCC job has been submitted by this package. Static and Python unit tests do
not establish Stata 19 export bytes, scheduler behavior, MATLAB R2025b syntax,
pool behavior, MEX compilation, runtime, or memory. Run one source-bound
scheduled preparation smoke and one cold/warm MATLAB smoke before CZ18. Then
measure CZ18 before deriving 2x/4x timeout requests. Preserve every failed
attempt under a new label and require SGE accounting, application logs, and
validated wrapper outputs before accepting a benchmark.
