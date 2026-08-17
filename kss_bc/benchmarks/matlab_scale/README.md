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
12-hour header is a hard ceiling; a future submitter must use the lower-scale
projection plus headroom when a smaller request is supported.

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

No SCC job has been submitted by this package. A scheduled small smoke is
still required to test MATLAB R2025b syntax, pool behavior, MEX compilation,
and receipt production before any scale run.
