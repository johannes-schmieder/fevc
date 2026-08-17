# Testing and qualification

Use the repository interpreter for Python:

```bash
./.venv/bin/python -m pytest kss_bc/tests/python -q
```

Run the Stata/MP 18 suites from the repository root:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do kss_bc/tests/stata/run_all.do quick

/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do kss_bc/tests/stata/run_all.do full
```

Run the registered local gate, including an isolated `net install` smoke test,
with:

```bash
./.venv/bin/python kss_bc/tools/run_checks.py
```

Generated `.log` files are transient and ignored. A successful suite prints a
distinct `KSS_BC TEST SUITE PASS` marker. The registered runner requires that
application marker because some Stata launchers return process status zero
even after a do-file error.

The suites include the source-bound API10--API18 counterexamples: explicit and
negative-zero controls across every backend/nuisance/deletion route, automatic
dispatch, the six-row `K(2,3)` firm-relabeling attack, determinant-four and
anchor-boundary control-basis maps at `probes(2)`, safely eligible `Q` versus
`-Q` anchors, downstream-conditioned withholding, the exact `2^53` frequency
boundary, maximum allowed tolerance, final corrected-row overflow, all-JLA
physical-copy allocation withholding, frequency regrouping, and stale-runtime
rejection. API14 also registers scalar-B0 versus lockstep-B1 equivalence,
per-RHS complete residuals, zero/inactive RHS handling, matrix action batching,
and the supported CMG path on a weak graph. The package runner also
checks that the Separations preparation uses the physical
`persid estabid time` key and retains valid repetitions of clustered analysis
worker/firm/period coordinates.

API17 compares both dimension choices of the residual-maker helper with a
directly inverted dense block, forces the reduced exact path with a
literal-copy match wider than the identified coefficient dimension, and
requires equality with the equivalent frequency-weight representation.

API18 adds deterministic fixed-point match-bridge pruning, installed CMG API5,
automatic exact/B1/CMG routing before probe generation, hard batch and solver
memory gates, batched probe algebra, an exact-terminal fast path with the full
original-system residual check, and typed pre-RNG fallback. The quick suite
checks routing, memory rejection, batch invariance, and multilevel setup. The
full suite adds B1/CMG estimator equality and the namespace-loader regression.
The clean-install smoke executes both exact estimation and the installed CMG
backend through the public command.

API19 adds the experimental single-process scale path. The quick suite runs
independent Python and Mata dense oracles for the scalar no-control match
formula; coefficient-cell, deletion-unit, and exact target-stratum
compression; fixed-domain `mt64s` candidates and caller-state restoration;
overlap-aware resource admission; native disk-backed dataset lifecycle;
richer route diagnostics; well-connected and ring fixtures; and the complete
compressed command. The command test includes repeated rows, multiple deletion
IDs at one coefficient cell, exact target strata, literal frequencies, P40 and
P200, batch partition invariance, ID relabeling and row permutation, restored
caller data and `e(sample)`, typed controls/observation fallback, forced
fast-path rejection, and a generic pre-allocation physical-copy failure.

Every compressed fit, leverage, and target RHS must appear in
`e(solver_rhs_diagnostics)` and pass the complete original worker-plus-firm
normal-equation gate `max(1e-11,10*tolerance())`. The tested normalization is
the Euclidean residual divided by the original RHS norm, or the absolute
residual for a zero RHS. Solves use the full-firm zero-sum quotient and check
the displayed last-firm ground's original equation. Graph residuals alone do
not pass. The target tests require
`total = worker + firm + 2*covariance` for plugin, correction, and corrected
rows and require corrected = plugin - correction within the registered
regrouping tolerance.
Stage-4 RHS identifiers must cover the global logical range `1..P` exactly,
and stage-5 identifiers must cover `1..2P` exactly; identifiers never restart
at a batch boundary. Compressed evidence also reports `e(rng_seconds)` as a
nested part of leverage and target work. `e(correction_seconds)` is likewise
a nested attribution and is not added to complete-command wall time.

Run the API19-focused Python subset with:

```bash
./.venv/bin/python -m pytest \
  kss_bc/tests/python/test_scale_compression.py \
  kss_bc/tests/python/test_scale_match_formula.py \
  kss_bc/tests/python/test_scale_resource.py \
  kss_bc/tests/python/test_scale_rng.py \
  kss_bc/tests/python/test_scale_route_diagnostics.py \
  kss_bc/tests/python/test_scale_fixtures.py \
  kss_bc/tests/python/test_scale_scc_validator.py -q
```

Local P40/P200 fixture timings exercise the algorithm but are not scale
qualification. Canonical compression must agree with the general/dense oracle
and demonstrate forecasted four-times-CZ18 memory feasibility. P20 by itself
is not performance evidence because setup may dominate it.

Bounded solver benchmarks run from the repository root:

```bash
stata-mp -q do kss_bc/benchmarks/lockstep_solver_benchmark.do \
  weak 10000 1000 8 20260815 <output-directory>
stata-mp -q do kss_bc/benchmarks/cmg_kss_benchmark.do \
  moderate 10000 1000 200 20260815 56 <output-directory>
```

The CMG driver records typed hierarchy rejection rather than converting it to
success. The durable local evidence and per-RHS rows are under
`benchmarks/reports/`.

The end-to-end benchmark runs the public estimator in a fresh process
for each route and validates estimator equality, complete per-RHS residuals,
stage timing, and (on SCC) peak RSS:

```bash
stata-mp -q do kss_bc/benchmarks/estimator_cmg_benchmark.do \
  local b1 moderate 1200 300 40 8675309 4 60 local_e2e \
  <output>/numopt/moderate/b1 0000000000000000000000000000000000000000
./.venv/bin/python kss_bc/benchmarks/validate_numopt.py \
  --run-dir <output> --expected-commit <commit> --scenarios moderate
```

`scc/submit_numopt.sh` submits only one route/scenario at a time. It rejects a
projection above 5,400 seconds; `run_numopt.sge` independently stops the
estimator at 5,400 seconds, uses four slots, reserves 64 GB, and gives forced
CMG a maximum 56 GiB envelope. Easy CMG is accepted only as a typed hierarchy
rejection. Moderate and weak require estimator equality and complete residuals.

The owner-authorized KSS-PROD-1 real-data harness is separate from KB6. It
builds one content-addressed lean bundle from
`benchmarks/prod_bundle_allowlist.txt`, freezes one checksum-bound raw-data
manifest in the run directory, and never takes a MATLAB-retained sample as
estimator input. Submit and validate the three phases in order:

1. `preflight`: normal package installation on Stata 18/19, exact four-slot
   license checks, hierarchy stress, pure-Stata CZ24/CZ25/CZ18 preparation,
   CZ24/CZ25 B1/CMG gates, exact retained-match comparison with maintained
   MATLAB, and CZ18 graph preflight;
2. `calibration`: concurrent cold/warm route, batch, and processor cells,
   followed by a source/input/qacct-bound selector; and
3. `production`: the selected full 200-probe CZ18 run and the two-copy stress
   graph derived from its retained sample. The stress uses an auto-sized batch;
   a bound 20-probe run must project inside the capped timeout before its full
   200-probe job starts.

Use `benchmarks/scc/deploy_prod_bundle.sh`,
`benchmarks/scc/submit_prod_dag.sh`, and
`benchmarks/validate_prod_scc.py`. Production submission requires the explicit
`--authorize-production KSS-PROD-1` argument. The validator requires every RHS
to pass the registered `max(1e-11,10*tolerance())` complete residual gate,
binds batch/seed/probe/resource metadata, and writes phase certificates. Use
`collect_prod_summary.sh` for privacy-safe aggregate collection; do not copy
prepared data, retained identifiers, or retained DTA files off SCC.

The KSS-SCALE-1 SCC harness is not distributed estimation. Each experiment is
one SGE job containing one Stata process. Scheduler slots reserve CPU, memory,
I/O, and shared-node capacity; they do not set Stata's numerical processor
count. The provisional large-job request is `-pe omp 14` with
`mem_per_core=4G`, reserving 56 GiB, while the wrapper separately sets and the
driver verifies `c(processors)==4`. The receipt and validator keep requested
slots, actual `NSLOTS`, requested/actual Stata processors, GNU-time RSS, and
`qacct maxvmem` separate. Later accounting may justify a narrower reservation.
Do not describe the 14-slot reservation as a 14-processor estimator.

The wrapper stages the checksum-bound input to node-local `$TMPDIR`, provides
that directory to Stata's native disk-backed preservation, and copies only
compact validated outputs back. It refuses inadequate temporary capacity.
Reconcile the selection, compression-transition, numerical, and restoration
peaks to verify that `preserve` actually releases row-resident memory. If it
does not, benchmark the native `tempfile` save/clear/use alternative. If safe
restoration still cannot fit, stop for owner authorization; never weaken
normal caller-data or `e(sample)` semantics implicitly.
One job may not write coefficient shards, invoke another estimator job, or
use a numerical reducer. The registered scale submitter is
`benchmarks/scc/submit_kss_scale.sh`, the wrapper is
`benchmarks/scc/run_kss_scale.sge`, the driver is
`benchmarks/scc/kss_scale_driver.do`, and aggregate validation uses
`benchmarks/scc/validate_kss_scale.py`. Resource-API-4 P200 jobs 7203808 and
7203861 pass every CZ24/CZ25 scheduler, application, scientific, residual,
identity, lifecycle, and resource-reconciliation gate on two host classes.
CZ18 P40 job 7203882 also passes its scientific gates and confirms the
compressed raw-data lifecycle, but scale progression remains blocked: its
5,739,220,992-byte process peak is 146,872,938 bytes above the registered
transition envelope. Resource API 5 retains the 96-MiB fixed runtime family
and charges another 32 bytes per retained row to sorting/compression allocator
high water, compared with the measured 17.91-byte-per-row omission. API 5
raw-input jobs 7204143 (P40) and 7206467 (P200) reconcile that repair.
Fixed-retained P200 job 7206667 passes all scientific and restoration gates
but is preserved as failed resource evidence: its 3,775,438,848-byte
numerical RSS peak exceeds the 3,181,596,634.6-byte forecast. Resource API 6
and solver receipt API 23 add the structural no-reuse bound
`max(live nonsolver, transition + phase scratch + solve-ahead) + routed
solver`; the exact 7206667 regression is 5,328,065,418.6 bytes before the
separate 30-percent margin. Clean API 6 CZ24/CZ25 and fixed CZ18 P40/P200
evidence must pass before 2x.

API 6 CZ24 P200 job 7207871 completed with `failed=0`, `exit_status=0`, and
601 accepted original-equation RHS certificates, but it is not accepted SCC
evidence. The deployed validator reconstructed the old live numerical sum and
failed closed on the API 6 no-reuse receipt with `resource phase overlap
arithmetic failed`. The validator regression now charges
`max(live nonsolver, transition + phase scratch + solve-ahead) + routed solver`
for compressed work and retains the old live sum for generic work. Qualifying
evidence must come from a fresh source-bound bundle and run; do not revalidate
job 7207871 with newer code.

The repaired API 6 source then passed the full predecessor chain: CZ24 P200
job 7209064, CZ25 P200 job 7209095, fixed CZ18 P40 job 7209195, and fixed
CZ18 P200 job 7209358. The fixed P200 job accepted 601 complete original-
equation residual certificates with maximum relative residual
`9.99040353264e-11`; its 3,869,261,824-byte measured process peak reconciled
against the 5,328,065,589.75-byte registered no-reuse upper bound. Job
7209118 is the preserved typed pre-RNG wall-admission failure for the earlier
3,600-second request.

Well-connected 2x P40 job 7209896 is separate failed fixture evidence. It
stopped before compression and RNG with `INVALID_FIXTURE_INPUT` because an
unweighted source has no public frequency variable and the connector builder
was passed that empty option instead of the internal literal-one diagnostic
frequency vector. `tests/stata/test_scale_fixtures.do` now exercises this
exact unweighted path and verifies 2-copy rows, physical mass, workers, firms,
coefficient cells, deletion units, connectedness, and zero bridge units. Do
not apply the repaired validator or fixture retrospectively to job 7209896.
Its preserved SHA-256 values are
`5519feca5dd0850bf61e673ec57c8552d45f31874fa9c045e161b4e088888a4d`
for qacct,
`0eb1ac38bcdabb9555a5b529d95609da2a17c138bca28abf1785418982839b95`
for the application log,
`e696bcf489628ec63f318665f204caddd55d57721b632711a2de394247c767c3`
for `wrapper.fail`,
`b743f75157ec657bf16510ae96e62cea75e49d11887049330e4ecd5e3290d8d1`
for the node receipt,
`30369f2e0890d9ad5d20c96463161a44caac35d7b4587b6d019ed7696b9f1ca5`
for the reservation receipt, and
`5881fcebd40783b3bc02d915f2b39643d85bd796386282b5b94a0f929d68b5ff`
for process resources. Qacct records `failed=0`, `exit_status=1`, 97 seconds
wall, 174.776 CPU seconds, 2,377,184 KiB `ru_maxrss`, and 2.654 GiB
`maxvmem`.

After the unweighted-connector repair, source `cdc74f4` passed new CZ24 P200
job 7209971, CZ25 P200 job 7209987, fixed CZ18 P40 job 7210003, and fixed
CZ18 P200 job 7210098. The last job completed in 1,392 seconds, certified 601
right-hand sides at maximum residual `9.99040353264e-11`, and reconciled a
3,767,971,840-byte process peak against a 5,328,065,600.25-byte registered
bound. Its admission receipt unlocked the repaired 2x fixture.

Well-connected 2x P40 job 7210297 passed fixture construction but returned
typed pre-RNG `NO_REALISTIC_SOLVER_ROUTE`; it is failed evidence. It measured
16,403,780 rows, 623,464 cells/deletion units, 235,060 workers, 21,206 firms,
57,154 hybrid vertices, 339,183 hybrid edges, nine hierarchy levels, and a
226-vertex terminal. Its qacct SHA-256 is
`a4b120285d7de26b02f2822cfd391958385a925fa5d3ee16eed61cb02505b7a5`;
qacct records 1,229 seconds wall, 1,786.549 CPU seconds, 6,624,555,008 bytes
`ru_maxrss`, and 6.400 GiB `maxvmem`. Reduced-probe P20 route-profile job
7210431 reproduced the same rejection. Its 61 planned right-hand sides retain
the same 64-iteration pilot cap, so it is not a cap comparison and is not
performance or scale evidence. Its qacct SHA-256 is
`6de9bbb2f51da5c119f4ad35b5cd8ce3cc3d4528d4a33656d272571c1a589d4b`.
P10 job 7210689 used the distinct 128-iteration cap and still returned the
same typed pre-RNG rejection. Its qacct records 644 seconds wall, 964.457 CPU
seconds, 6,627,987,456 bytes `ru_maxrss`, and textual `maxvmem=6.562G`; the
qacct SHA-256 is
`02a7c823df8f3ef9badd6ba8ebef0b23b55319920051deeb377d4b049469dec4`.
The application, summary, wrapper-failure, and Stata-failure SHA-256 hashes
are respectively
`3c34c21ed3eb7e25ada5a0f83b6b27de8bf957ba02087fc9aae276604bbf098c`,
`fc298381ae23df53fe0740a63e0ba0945fde2f3f684b4f0fe4b55fd9b11f10b7`,
`38dcdbeded931e385217632a8b63e5fe66b2b41fdc1a52ece8ab8f45aa634569`,
and `137e586a5c278155b912a43a011e6f5682930f89f74e4c72d37d705559e1e00f`.
This rules out only a pilot cap of 64 or less as the sole cause. Because the
old source did not serialize per-pilot rows, it does not identify or justify
changing the status, residual, iteration, or work gate.

Every SCC driver exit now preserves the already posted route and eight-pilot
matrices before returning a typed failure. `route_diagnostics.csv`,
`route_pilot_diagnostics.csv`, and `summary.csv` retain status, residual,
iterations, exact action counts, projected work, failure codes, human-readable
reasons, and route forecast data. `test_scale_scc_driver.do` executes the
real driver locally and verifies these receipts; the Python contract test
requires serialization to precede the failure exit.

SCC evidence must come from a clean source commit and a unique run directory
under `/projectnb/welfgr/kss-bc/runs/`. Submit through `qsub -P welfgr`.
Source-bound K1 job 7201105 passed the Stata 19 golden-vector, atom-invariance,
complete-state-restoration, call-shape, chunking, timing, wrapper, and qacct
gates. Stata 18 and 19 therefore share the registered API 19 contract
`KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19`; other runtimes fail closed. Accept a
phase only after qacct, wrapper, application-log, output,
identity, reproducibility, and RSS validation all pass. See
`benchmarks/README.md` for the exact commands and evidence boundary.

Scale progression starts only after local gates, then uses CZ24/CZ25,
reduced-probe CZ18 calibration, full CZ18 P200, connected 2x cases, and the
mandatory well-connected 4x P200 case. The well-connected fixture measures
dimensional scaling without a worsening bottleneck; the ring is a separate
weak-connectivity and deletion-safety stress and cannot be the sole normal-
scale extrapolator. Record condition proxies, hierarchy levels/terminal,
iterations, actions, and all stage timings for both. Admit 8x or 16x only from
an upper forecast, not a point estimate, with 25--30 percent memory and 50
percent wall headroom inside 56 GiB and 12 hours. Reconcile every phase
forecast against process RSS and `qacct maxvmem` before advancing.

Performance acceptance uses complete-command measurements including loading,
sample selection, compression, restoration, and output validation. Complex
mechanisms such as solve-ahead, recycled PCG, or block PCG require a repeatable
gain of at least 10 percent after their dense algebra. A simple low-risk
change may survive with a repeatable gain of at least 5 percent or a measured
scale-enabling reduction in peak memory or required passes. Report cold wall,
warm command wall, CPU time, repetitions, and run-to-run spread. Iteration
count without wall-time improvement is not acceptance evidence. Stop the
optimization search after the mandatory 4x P200 qualification when no
remaining candidate projects a 5 percent end-to-end gain or enables an
otherwise inadmissible scale within the hard limits.

Maintained MATLAB LeaveOutTwoWay runs remain descriptive benchmarks and
sample-selection comparators. Separate startup, import, sample selection,
pool/MEX setup, core estimator, serialization, teardown, cold wall, warm call,
CPU, and RSS. Its legacy finite-projection formula and language-specific probe
schedule preclude corrected-estimate equality. Every MATLAB or scale number is
a hypothesis until a source-bound measured run reports its source data, fitted
scaling rule, uncertainty, iteration/I/O assumptions, and actual result.
Use the same fixed retained sample for core-computation timing and a separate
independent retained-match comparison for sample selection. Attempt connected
2x and 4x inputs when the lower-scale evidence admits them. If MATLAB fails,
preserve the unweakened input, exact failure, dimensions, elapsed time, and
memory instead of rewriting the maintained estimator or shrinking the case.
