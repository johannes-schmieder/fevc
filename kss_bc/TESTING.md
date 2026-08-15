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

The suites include the source-bound API10--API14 counterexamples: explicit and
negative-zero controls across every backend/nuisance/deletion route, automatic
dispatch, the six-row `K(2,3)` firm-relabeling attack, determinant-four and
anchor-boundary control-basis maps at `probes(2)`, safely eligible `Q` versus
`-Q` anchors, downstream-conditioned withholding, the exact `2^53` frequency
boundary, maximum allowed tolerance, final corrected-row overflow, all-JLA
physical-copy allocation withholding, frequency regrouping, and stale-runtime
rejection. API14 also registers scalar-B0 versus lockstep-B1 equivalence,
per-RHS complete residuals, zero/inactive RHS handling, matrix action batching,
and the forced test-only CMG path on a weak graph.

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

SCC evidence must come from a clean source commit and a unique run directory
under `/projectnb/welfgr/kss-bc/runs/`. Use Stata/MP 19 through `qsub -P
welfgr`; accept a run only after qacct, application-log, and output validation
all pass. SCC scripts use only synthetic or public inputs and never restricted
Separations data. See `benchmarks/README.md`.
