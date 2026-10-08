# Mean component inference: bounded assessment

This new assessment compares Mean component inference, treating the observed
mean as fixed inside inference, with a genuinely fixed population mean `c0`.
The actual mean is recomputed for every outcome draw. Both arms use the same
outcomes and fixed numerical seed. It is a bounded implementation assessment,
not a universal coverage qualification; prior campaigns retain their status.

The preregistered scope and descriptive screens are in
[`mean_component_inference_validation_v1.json`](../../docs/mean_component_inference_validation_v1.json).
The separate [pre-assessment engineering amendment](../../docs/mean_component_inference_validation_v1_amendment1.json)
records source/receipt validation corrections without altering fixtures or
scientific cutoffs. Native point errors are checked against the dense oracle
with the registered six-MCSE bound, and interpreter, identity and solver
failures block the assessment rather than becoming ordinary unavailable fits.
The primary variance lies in the intended exact-design rank span; native
regressors use estimated ranks, so numerical basis error can remain.
The primary grid has exact Mata, native observation and native fixed-offset
match routes; highrank/q1; and worker/firm dimensions 12/20. The observation
fixtures have 432/1201 independent rows; match fixtures have 144/400 independent
units and 288/800 stored rows with positive integer frequencies. All four
targets are retained. q1 covariance is diagnostic because its remainder has
another concentrated mode. The other q1 targets have measured leading shares
0.914--0.959 and remainder shares no greater than 0.136. Highrank shares are
reported at both finite sizes; no universal cutoff is imposed.

`oracle.py` independently constructs the retained regression and target
geometry, whole-unit leave-out kernels, actual-Mean kernels and exact Gaussian
expectations/cross-covariances. It calculates fixed-c0 and actual-Mean q1
remainder moments and score/remainder covariances. Dense observation matrices
are confined to this small test oracle. The production runner invokes the
public Stata command, including its actual variance fitter and q1 intervals.

Every run freezes the runtime package, explicitly selected native plugins,
harness and registration, and binds them to a manifest. Raw failures are kept
and all attempted target rows count. Tasks write atomically after validating
source, schema, count, seed, sample and point-oracle identities. Aggregation
requires the complete inventory and returns nonzero for descriptive screen
failures. Such a failure is a finding, not permission to adjust the screen.

Run harness tests with the repository interpreter:

```bash
./.venv/bin/python -m pytest -q fevc/benchmarks/mean_component_inference_20261008/test_harness.py
```

An exact Mata smoke can be frozen before a native build is available:

```bash
./.venv/bin/python fevc/benchmarks/mean_component_inference_20261008/run.py freeze \
  /private/tmp/mean-component-smoke --profile smoke \
  --cell mata_highrank_k12_primary --cell mata_q1_k12_primary
./.venv/bin/python /private/tmp/mean-component-smoke/input/harness/run.py run-task \
  /private/tmp/mean-component-smoke mata_highrank_k12_primary_00001 \
  --stata /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp
```

Run each manifest task once, then call `aggregate ROOT`. Add `--plugin-dir`
with the newly built plugin directory to freeze native cells. Use the frozen
runner for later execution. The `pilot` domain is separate from `assessment`;
measure its runtime before approving the 2,000-replication assessment. Do not
overwrite a task to erase a failure. For deliberate-failure testing, use a new
smoke root and `run-task --deliberate-failure`.
`run-all ROOT --stata PATH --workers 2` executes and aggregates the complete
frozen inventory using separate processes and unique output directories.

Gaussian moment references apply to the Gaussian fixed-offset DGP. The heavy
tail diagnostic and the estimated-control-offset diagnostic are reported
separately; Gaussian known-offset moments are references, not their exact
sampling laws. Severe misspecification, null/weak signal, multimode targets
and concentrated mean weights are intentionally visible limitations.

The separate [fixed-outcome numerical registration](../../docs/mean_component_inference_numerical_v1.json)
defines `--profile numerical`: four dimension12 observation/match highrank/q1
outcomes, three fixed numerical seeds and four one-budget-at-a-time settings.
Its 48 paired tasks make 96 calls. Budget and seed task labels cannot change
the outcome RNG key; aggregation requires identical input hashes for all
settings of an outcome. It reports point/SE/AM/covariance deviations and
unavailable results, without computing sampling coverage. Three numerical
seeds are a sensitivity check, not a tail calibration experiment.
