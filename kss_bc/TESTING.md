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

The suites include the source-bound API10--API15 counterexamples: explicit and
negative-zero controls across every backend/nuisance/deletion route, automatic
dispatch, the six-row `K(2,3)` firm-relabeling attack, determinant-four and
anchor-boundary control-basis maps at `probes(2)`, safely eligible `Q` versus
`-Q` anchors, downstream-conditioned withholding, the exact `2^53` frequency
boundary, maximum allowed tolerance, final corrected-row overflow, all-JLA
physical-copy allocation withholding, frequency regrouping, and stale-runtime
rejection. API14 also registers scalar-B0 versus lockstep-B1 equivalence,
per-RHS complete residuals, zero/inactive RHS handling, matrix action batching,
and the forced test-only CMG path on a weak graph. The package runner also
checks that the Separations preparation uses the physical
`persid estabid time` key and retains valid repetitions of clustered analysis
worker/firm/period coordinates.

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

The API-15 end-to-end benchmark runs the public estimator in a fresh process
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

The owner-authorized real-data harness is separate from KB6. It reads an
existing checksum-bound Separations wage artifact without modifying the
Separations project, writes all derived rows under the SCC KSS run directory,
and collects only aggregate evidence. Use `scc/submit_separations.sh` in this
order: `prepare`, then route-specific `b1`, `cmg`, and `matlab`, then
`compare`. Start with a deterministic 5,000-worker CZ24 slice. A full natural
CZ is permitted only when the measured small run projects every requested
estimator route below 90 minutes. Validate aggregate evidence with
`benchmarks/validate_separations.py`; never copy `prepared.*`, `detailed.csv`,
or `retained_matches.dta` off SCC.

SCC evidence must come from a clean source commit and a unique run directory
under `/projectnb/welfgr/kss-bc/runs/`. Use Stata/MP 19 through `qsub -P
welfgr`; accept a run only after qacct, application-log, and output validation
all pass. The KB6 scripts use only synthetic or public inputs. The separately
named `run_separations_*` scripts implement the narrow SCC-only exception
described above. See `benchmarks/README.md`.
