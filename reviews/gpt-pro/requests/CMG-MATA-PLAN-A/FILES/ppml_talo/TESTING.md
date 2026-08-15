# Testing and qualification guide

The test suite separates finite algebra, numerical implementation, statistical
simulation evidence, identification certification, integration behavior, and
scale qualification. Passing a finite or Monte Carlo test does not prove an
asymptotic theorem.

## Local commands

From the repository root, run:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do ppml_talo/tests/run_all.do quick

/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do ppml_talo/tests/run_all.do full

.venv/bin/python ppml_talo/tests/oracle/check_st1_algebra.py

.venv/bin/python ppml_talo/tests/oracle/check_weighted_match.py

.venv/bin/python ppml_talo/tests/oracle/check_st11_independent_deletion.py

.venv/bin/python -m pytest \
  ppml_talo/tests/test_validate_scc_evidence.py -q

ppml_talo/tests/test_local_qualify_wrapper.sh
```

The quick suite contains deterministic unit, dense-oracle, exact-face,
failure, and command-integration tests. The full suite adds fixed-design
randomization calibration and a prespecified bias/degree sequence.

## Suite map

| Suite | Main obligation | Seeds/tolerances |
|---|---|---|
| `unit/test_hdfe_operators.do` | grouped design actions, Schur algebra, scaled PCG, batch isolation, multigraph deletion primitives, direct block-kernel action, exact coefficient-basis trace, coherence/nonfinite guards, compensated accumulation, RNG preservation | deterministic; solve tolerances `1e-8` to `1e-15`; exact trace `2e-11`; batches `1`, `3`, `p-1`, `p`, `p+1` |
| `unit/test_joint_schur.do` | complete joint inverse against dense inversion, exact and tree two-FE certificates, residualized nuisance leverage, weighted one-copy sketch, common positive quotient, nuisance-order/tolerance invariance, deliberately degraded preparation, nonfinite output containment, reciprocal-condition diagnostics, and exact/near nuisance collinearity | deterministic; perturbation scales from `1e-1` through `1e-12` with platform-dependent conservative withholding allowed only after typed inverse/rank checks |
| `oracle/test_dense_mata.do` | dense Hessian/correction identities, exact deleted refits, offset handling, conservative-rejection refits | deterministic; registered absolute tolerances through `2e-9` |
| `identification/test_exact_face_oracle.do` | independent positive-component/zero-arc SCC characterization | exhaustive 2,160 connected row-labeled 2-by-2 patterns |
| `identification/test_failures.do` | public typed failures for rank, face, score, deterministic leverage, solver, scope, level-count, and post-fit cleanup gates; every post-fit branch asserts no inherited `e(V)` | seed `2026812` and explicit failure statuses |
| `integration/test_command_small.do` | `e()` contract, `e(b)` without `e(V)`, estimates store/restore, data/order preservation, accounting and option invariance | seed `642019`; randomized streams explicitly set |
| `integration/test_match_command.do` | rich whole-match command, integer frequency collapse versus literal expansion, explicit and default target mass, nonlinear offset, probability gate, exact block gates, randomized and exact trace modes, and unsupported combinations | seed `20260812`; 800 trace probes plus deterministic basis trace with `probes(1)` and batches through `p+1` |
| `integration/test_st11_deletion.do` | observation/cluster feature parity, one-copy and all-copy frequency semantics, singleton equivalence, supplied numeric/string IDs, actual matches sharing bin cells, dense/matrix-free comparison, the four-row weighted-sketch counterexample, joint/fixed-offset distinction, reciprocal-condition returns, and absence of `e(V)` | seed `20260813`; exact traces, 1,200-probe joint calibration, and 8,192-probe weighted regression |
| `integration/test_actual_match_freezer.do` | original-person/original-establishment enforcement, opaque deletion ID construction before exact collapse, physical-row accounting, and same-design/different-match preservation | deterministic 12-row expanded fixture |
| `integration/test_match_shape_preflight.do` | privacy-safe design-shape aggregation and finite-count guards used by the scheduled SCC preflight | deterministic synthetic collapsed fixture |
| `integration/test_separations_match_adapter.do` | legacy bin-cell and v3 actual-match four-target schemas, independent deletion-ID preservation, provenance binding, quotient dimensions, runtime, success diagnostics, and typed withholding | seed `20260812`; 200 trace probes plus exact trace |
| `integration/test_cz20_prepare_driver.do` | scheduled CZ20 driver argument plumbing, exact 100/200-group specification, and rejection of superseded 1,000-group launches against a side-effect-free mock Separations program | deterministic temporary files |
| `integration/test_all_cz_drivers.do` | shared prebuild, per-CZ split, reusable wage map, E--U cell, pair validation, and preserved-output plumbing | deterministic two-CZ temporary fixture |
| `test_make_pilot_configuration.py` | canonical pilot configuration, registered settings, and overwrite refusal | deterministic JSON fixture |
| `test_all_cz_contract.py` | paired task generation, provenance/checksum binding, exact-result aggregation, tamper/overwrite rejection, one-wage-job-per-CZ held-qsub graph, and scheduler exit/resource validation | deterministic two-CZ fixtures and mock qsub/qacct |
| `test_validate_cz20_crossfit.py` | four-target cross-fitting schema, checksum/configuration binding, total accounting, split-sample support accounting, and qsub resource/privacy contract | deterministic four-row fixtures; 10 replications and seed `190424` |
| `test_sge_prepare_wrapper.py` | pilot/all-CZ qsub resource, opt-in, checksum, output, privacy guards, and exact-status failure replay | deterministic source-contract regression |
| `test_validate_probe_ladder.py` | two-seed nested-probe completeness, frozen-design equality, cross-node roundoff, checksums, stability/MCSE gates, typed 800-probe escalation, and probe-ceiling reassessment | deterministic six/eight-result fixtures |
| `integration/test_benchmark_contract.do` | successful stable and Poisson-with-zeros CSV evidence contracts, source binding, solver certificate, accounting identities | outcome seed `20260811`, command seed `991827` |
| `test_local_qualify_wrapper.sh` | clean-tree qualification, process/CSV status propagation, malformed or absent evidence rejection | deterministic mock executable; no estimator run |
| `test_validate_scc_evidence.py` | complete SCC artifact/qacct contract and tamper rejection | deterministic synthetic five-size evidence set |
| `monte_carlo/test_randomized_accuracy.do` | leverage and conditional trace unbiasedness/MCSE calibration | 80 streams, 64 probes, fixed registered seeds |
| `monte_carlo/test_bias_consistency.do` | finite bias improvement and denser-network sequence | seeds derived from `730000 + 1000*degree + replication` |
| `monte_carlo/test_joint_nuisance_bias.do` | correctly specified Poisson bias improvement with a jointly estimated numeric control and year effects, plus denser-network consistency evidence | 20 replications at degrees 10 and 30; outcome seeds derived from `950000 + 1000*degree + replication`; deterministic exact trace |
| `oracle/check_st1_algebra.py` | implementation-independent finite algebra and falsification cases | seed `20260811`, `atol=2e-10`, `rtol=2e-9` |
| `oracle/check_weighted_match.py` | literal frequency expansion, weighted Woodbury shift, two-sided correction mass, stored-coordinate finite difference, rank-revealing local basis, separated-refit failure, and deterministic basis-trace equality | seed `20260812`, `atol=4e-10`, `rtol=4e-9` |
| `oracle/check_st11_independent_deletion.py` | implementation-independent observation/cluster expansion identities, generic multi-coordinate blocks, joint Schur actions, frozen target matrix, fixed-offset counterexample, target-weight scaling, singleton boundary, and nuisance/positive rank failures | seed `20260813`, `atol=5e-10`, `rtol=5e-9` |

## Identification interpretation

The production positive-support gate is deliberately sufficient rather than
necessary. `ROW_DELETION_FACE_CERTIFICATE_FAILURE` means that the package did
not certify every deleted fit. It does not assert that a finite deleted fit is
impossible. The independent SCC oracle checks the exact finite criterion on
small designs and requires zero unsafe production acceptances; conservative
rejections are counted and reported.

The dense block engine uses a separate rich-design certificate. It forms full and positive
information matrices on the fixed quotient, then computes exact whole-match
spectra from coefficient-space kernels. The positive matrix is
`X'diag(f*y)X`, not the fitted-mean information. The rich regression fixture
contains a match with full-fit eigenvalue below .8 but positive eigenvalue one;
the package must withhold it as `MATCH_POSITIVE_FACE_FAILURE`.
The same suite multiplies one identified control by `1e12`. Raw `rank(H)` then
fails, while diagonal equilibration must preserve the plug-in and TALO values,
pass both inverse residuals, and continue to reject a deliberately duplicated
column as genuinely singular.

## Randomized accuracy interpretation

The randomized tests condition on a fixed fitted design. Reported probe MCSEs
measure numerical randomization only. The registered calibration checks are
pointwise and do not justify a simultaneous leverage confidence bound or an
econometric standard error.

Match mode can instead enumerate the coefficient basis with `traceexact`.
The tests compare that deterministic contraction to the direct dense
observation-space correction, require invariance to seed and batch width at
the registered floating-point tolerance, and require exactly zero reported
trace MCSE. This is a finite numerical identity, not an econometric standard
error or a theorem-applicability result.

The existing all-CZ qualification path first runs one qsub prebuild over the source
panel. Its manifest and each generated configuration are SHA-256 bound before
submission, including an individual checksum for every read-only CZ panel.
The submission script performs no estimation: it submits one
wage job per CZ and two held E--U/TALO jobs for the 100- and 200-bin cells.
Each cell independently publishes its result, frozen design, E--U map,
resource record, and sanitized logs. The aggregate validator succeeds only
after both resolutions are present and available for every registered CZ.
The separate qacct validator requires every registered wage and cell job to
pass the eight-slot, twelve-hour, 56-GiB, scheduler-exit, and process-exit
gates; a missing or extra accounting file is a failure.

ST11 adds a separate actual-match qualification before this workflow may be
reused. On the same frozen CZ20 sample, it must compare cross-fitting, frozen
worker-bin by firm-bin deletion, and deletion of original-person by original-
`estabid` matches with identical PPML specification, target weights, sample,
and pooling rules. The v3 adapter requires the exact deletion-definition label
`actual_person_original_estabid`. The actual-match ID must be constructed
before duplicate collapse and included in the collapse keys. The `.8` full-
block gate, positive-face gate, and numerical tolerances remain unchanged. A
failed run and its typed status remain evidence; it is never repaired by
automatic coarsening, dropping, or threshold relaxation. All SCC estimation
must run through `qsub`, never on the login node.

## Scale ladder

Local smoke tests use 10,000 and 100,000 rows. The SCC profile runs 10,000,
100,000, 1 million, 5 million, and 10 million rows with 100 leverage and 100
correction probes. Its `stable` outcome is exactly in the fitted worker--firm
model, isolating TALO engine scaling. The separate `stochastic_positive`
profile also stresses external PPML convergence. Every run writes a one-row
CSV, including failures, with dimensions, settings, return/status fields, wall
time, residuals, iterations, leverage diagnostic, all four result vectors,
source commit, Stata version, platform, and an engineering memory estimate.
The `poisson_zeros` profile supplies a correctly specified skewed count outcome
with zeros. Scheduler maximum RSS must be added to the qualification record;
the estimate is not a substitute.

The provenance-bound local Stata 18 Apple-Silicon ladder has passed through
ten million rows with 100+100 probes. The ten-million run completed in 631
operating-system seconds with 19.66 decimal GB measured peak RSS. That evidence
closes the local performance subgate but not the SCC gate, which requires
Stata 19/Linux and scheduler-measured RSS. The complete record is
[the local qualification report](benchmarks/reports/LOCAL_STATA18_2026-08-12.md).

Use `benchmarks/local_qualify.sh` for locally measured evidence. The wrapper
refuses dirty source and output overwrites, derives the source commit, and
stores the platform resource log beside the Stata log. Direct calls to
`benchmark_akm.do` remain useful for development but rely on the caller to
provide the correct commit.

Validate a retained local evidence set with:

```bash
ppml_talo/benchmarks/validate_local_evidence.py \
  --expected-commit FULL_40_CHARACTER_COMMIT \
  --stable RESULT_10K.csv RESOURCE_10K.txt \
  --stable RESULT_100K.csv RESOURCE_100K.txt \
  --stable RESULT_1M.csv RESOURCE_1M.txt \
  --stable RESULT_5M.csv RESOURCE_5M.txt \
  --stable RESULT_10M.csv RESOURCE_10M.txt \
  --repeat RESULT_1M.csv RESULT_1M_REPEAT.csv \
  --poisson RESULT_POISSON.csv RESOURCE_POISSON.txt \
  --positive RESULT_POSITIVE.csv RESOURCE_POSITIVE.txt \
  --weak-ring RESULT_WEAK.csv RESOURCE_WEAK.txt
```

The wrapper returns a nonzero shell status for a nonzero benchmark CSV return
code even when the Stata executable itself exits zero. Failure CSVs and their
typed statuses are retained and validated.

The final benchmark argument selects the network. `mobile` is the default
scale profile and uses deterministic heterogeneous worker mobility.
`weak_ring` retains a near-cycle graph as an adversarial conditioning test.
Never substitute the mobile result for the weak-ring result: they test
different numerical obligations, and both failures and passes must be kept.

Create a Git archive from the intended clean commit, verify its SHA-256 before
and after staging, and extract it into a new immutable SCC source directory.
Submit the portability suite and scale array from the login node; never run
Stata there. The archive hash and full source commit are mandatory job inputs:

```bash
qsub -terse -o /absolute/results -e /absolute/results \
  -v PPMLTALO_ROOT=/absolute/extracted-source,\
PPMLTALO_RESULTS=/absolute/results,\
PPMLTALO_SOURCE_COMMIT=FULL_40_CHARACTER_COMMIT,\
PPMLTALO_ARCHIVE_SHA256=FULL_64_CHARACTER_SHA256 \
  ppml_talo/benchmarks/sge_portability_suite.sh

qsub -terse -o /absolute/results -e /absolute/results \
  -v PPMLTALO_ROOT=/absolute/extracted-source,\
PPMLTALO_RESULTS=/absolute/results,\
PPMLTALO_SOURCE_COMMIT=FULL_40_CHARACTER_COMMIT,\
PPMLTALO_ARCHIVE_SHA256=FULL_64_CHARACTER_SHA256 \
  ppml_talo/benchmarks/sge_scale_ladder.sh
```

The second submission is a five-task array. Once jobs leave `qstat`, export a
separate `qacct` record for the portability job and every scale task. Copy the
evidence to the review environment, validate it with
`benchmarks/validate_scc_evidence.py`, then checksum the raw evidence before
writing a versioned qualification report. A failed or timed-out task remains
part of the evidence set. The validator requires the pre-registered active
Stata processor count and exact dependency versions; these are arguments, not
values inferred from the result being validated.
For every scale task, pass the CSV, GNU-time resource log, Stata log, metadata,
and qacct record to one `--stable` argument in that order.

SCC's `qsub` wrapper does not accept a command-line array-task override.
`sge_scale_single.sh` is the scheduled non-array wrapper for an independent
repeat of one registered scale task. Set `PPMLTALO_TASK_ID=3` for the 1m
repeat; it exports that task only inside the compute job and then executes the
exact candidate `sge_scale_ladder.sh`. Validate the base 1m CSV plus all five
repeat artifacts with `--repeat`.

The scheduled scripts pass the granted slot count to Stata explicitly and
record both the request and active processor count. SCC job `7148595` showed
that the site Stata/MP 19 license permits four processors. The registered SCC
scale envelope therefore uses four slots and 16 GB per slot. The original
eight-core target and failed request remain evidence; four-core results must
not be relabeled as eight-core evidence.

## Portability and release gates

The local baseline is Stata/MP 18 on Apple Silicon. Stata/MP 19 and SCC Linux
passed their registered portability and synthetic scale gates on 12 August
2026 under the four-core SCC license. The evidence is recorded in
[the SCC qualification report](benchmarks/reports/SCC_STATA19_2026-08-12.md).
The ST11 independent-deletion candidate reran quick, full, and clean-install
portability on 13 August 2026; its passing job and the preceding retained test
failure are recorded in
[the ST11 portability report](benchmarks/reports/SCC_ST11_PORTABILITY_2026-08-13.md).
Realized-network and release gates remain separate. Before a release candidate:

1. run quick and full suites on both supported Stata versions and preserve the
   clean-install log;
2. run a clean temporary `net install` smoke test;
3. validate every stored Pro record;
4. build the companion twice and inspect every rendered PDF page;
5. run the 10k/100k local benchmark and the SCC scale ladder;
6. record source commit, dependency versions, seeds, tolerances, hardware,
   wall time, maximum RSS, and all non-success statuses.

Any failed or censored scale run remains evidence. Do not loosen a statistical
or solver threshold solely to turn it green.
