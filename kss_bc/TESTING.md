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

1. `preflight`: normal package installation on Stata 18/19, four/eight-slot
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
The SCC submitter and manual validator explicitly load `python3/3.12.4`; the
cluster's unversioned Python 3.6 is not a supported harness interpreter.

SCC evidence must come from a clean source commit and a unique run directory
under `/projectnb/welfgr/kss-bc/runs/`. Use Stata/MP 19 through `qsub -P
welfgr`; use the registered Stata 18 cells for compatibility. Accept a phase
only after qacct, wrapper, application-log, output, identity, reproducibility,
and RSS validation all pass. See `benchmarks/README.md` for the exact commands
and evidence boundary.
