# Maintained MATLAB descriptive scale benchmark

This package benchmarks the checksum-bound maintained LeaveOutTwoWay MATLAB
implementation at the KSS-SCALE fixture sizes. It does not modify, copy, or
replace the maintained estimator. It is a development comparator and never a
runtime dependency of `kss_bc`.

The supported case grid is scale `1`, `2`, `4`, `8`, or `16`; topology
`well`, `well_connected`, or `ring`; and sample mode `fixed` or `selection`.
Every estimator call uses JLA match deletion, 200 probes, and a four-worker
pool. Inputs must have four numeric columns in this order:

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
- scales 2 and 4 with topology `ring` as a weak-connectivity stress.

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

## Scientific comparison boundary

The maintained entry point returns corrected worker, firm, and covariance
targets but does not expose its plug-in components. The case binding therefore
names a checksum-bound Stata or fixture reference receipt and its four plug-in
components. The validator checks that reference receipt and the plug-in
accounting identity. It does not infer a MATLAB plug-in from corrected output.

Corrected MATLAB values are descriptive. Every call must satisfy

```text
total = worker + firm + 2 * covariance
```

but no corrected-estimate equality is tested across languages, processes, or
repetitions. This is explicit in every aggregate and comparison receipt as
`IDENTITY_ONLY_NO_EQUALITY_GATE`.

For a `fixed` case, the maintained retained-key hash and all retained
dimensions must equal the reference sample. For a `selection` case, exact
agreement is reported but is not required. A selection mismatch marks the
reference plug-in as incomparable to the MATLAB-selected sample.

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

Each job consumes a `kss_matlab_scale_case_v1` JSON object. It contains:

- `label`, `scale`, `topology`, `sample_mode`, `seed`, `probes`, and
  `warm_repetitions`;
- `source`: repository commit, bundle SHA-256, maintained upstream commit,
  complete runtime-tree SHA-256, core/CMG/hierarchy/solver hashes, and the
  benchmark source-contract hash;
- `input`: SHA-256, rows, workers, firms, matches, dense-ID contract, and
  order contract;
- `reference_sample`: source kind, receipt SHA-256, retained-key SHA-256,
  rows, workers, firms, matches, and the four reference plug-in values.

The retained-key digest is SHA-256 over unique worker--firm keys sorted
numerically by worker and firm and encoded as UTF-8 lines
`<integer-worker>,<integer-firm>\n`. This is the same canonical digest used by
the driver after each maintained call.

The fixed source snapshot and registered selection-profile lines are in
`source_contract.json`. `verify_case.py` recomputes the complete maintained
runtime-tree identity plus its core and CMG-family hashes before MATLAB starts.
`make_case.py` constructs the binding from a supplied four-column CSV and a
validated one-row KSS/fixture aggregate. It hashes both artifacts and the live
maintained source snapshot without loading the row data into Python.

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
process wall time, user and system CPU, and peak resident memory from GNU
`time -v`. Scheduler `qacct` remains a separate acceptance layer when jobs are
eventually submitted.

`run_matlab_scale.sge` is deliberately non-submitting. It stages input and
case bytes under `$TMPDIR`, compiles MEX files there, caps the client and each
worker's numerical threads, and retains no row-level detail artifact. Its
12-hour header is only a hard ceiling. The submitter always overrides it with
an explicit measured application timeout plus a ten-minute wrapper reserve
and records the measurement-basis token in both submission and wrapper
receipts.

## Scalar SCC submission

`submit_matlab_scale.sh` submits exactly one requested stage per invocation.
It has no arrays, `hold_jid`, downstream submission, reducer, estimator
sharding, or automatic retry. Preparation must finish and validate before the
owner separately creates a checksum-bound `case.json` and submits either
MATLAB process. Cold and warm are independent descriptive jobs and may run in
either order or concurrently once their common prepared input and case exist.

Cold and warm MATLAB jobs request four `omp` slots at 14 GiB per slot (56
GiB total), start exactly one four-worker pool, and cap the client and each
worker's numerical-library threads to one. Their resource receipts keep this
allocation distinct from the 14-slot preparation reservation.

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
  CONTRACT_SHA256 MATLAB_ROOT TIMEOUT_SECONDS TIMEOUT_BASIS

submit_matlab_scale.sh warm \
  RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 \
  SOURCE_MANIFEST LABEL INPUT_CSV INPUT_SHA256 CASE_JSON CASE_SHA256 \
  CONTRACT_SHA256 MATLAB_ROOT TIMEOUT_SECONDS TIMEOUT_BASIS
```

`TIMEOUT_SECONDS` is the measured application boundary, not an unexplained
ceiling. `TIMEOUT_BASIS` is a filesystem-safe token naming the lower-scale
measurement or fitted timing rule. The submitter accepts 300--42,600 seconds,
adds exactly 600 seconds for staging and receipt finalization, and keeps the
scheduler request within 12 hours. It verifies immutable source bundle,
source commit, input, case, and source-contract hashes before submission. Each
invocation writes a pre-submission request receipt, then records the scalar
job ID and an accepted-submission receipt after `qsub -terse` succeeds.

The submitter does not create `case.json`. After preparation, use
`make_case.py` with `input.csv`, the dimensions from preparation
`wrapper.json`, the digest in `retained_keys.sha256`, and a separately
checksum-bound KSS or fixture reference aggregate containing the four plug-in
targets. That separate reference is required because fixed-sample preparation
does not run either estimator and cannot manufacture plug-in targets.

## Outputs and failures

One job directory has:

```text
application.txt
identity.json
resources.txt
wrapper.json
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
```

`wrapper.json` is written after success, application failure, timeout, or a
source/input rejection. It records the failure stage, exit status, timeout or
signal, partial aggregate failure, resource evidence, and hashes of every
available artifact. The application log remains intact. Failed runs must use
new job directories; do not overwrite or weaken their inputs.

After separately completed cold and warm jobs, validate with:

```bash
./.venv/bin/python kss_bc/benchmarks/matlab_scale/validate.py \
  --case <case.json> --case-sha256 <sha256> \
  --contract kss_bc/benchmarks/matlab_scale/source_contract.json \
  --cold-job-dir <cold-job-dir> --warm-job-dir <warm-job-dir> \
  --reference-aggregate <checksum-bound-kss-summary.csv> \
  --output <comparison.json>
```

Omitting `--reference-aggregate` leaves the plug-in status
`REFERENCE_BOUND_ONLY`. Supplying it requires its bytes to match the reference
receipt hash in the case binding and checks sample dimensions, all four
plug-in values, and their accounting identity.

No SCC job has been submitted by this package. Static and Python unit tests do
not establish Stata 19 export bytes, scheduler behavior, MATLAB R2025b syntax,
pool behavior, MEX compilation, runtime, or memory. Run one source-bound
scheduled preparation smoke and one cold/warm MATLAB smoke before CZ18. Then
measure CZ18 before deriving 2x/4x timeout requests. Preserve every failed
attempt under a new label and require SGE accounting, application logs, and
validated wrapper outputs before accepting a benchmark.
