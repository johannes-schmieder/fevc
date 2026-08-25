# VCKSS alpha benchmark protocol

`VCKSS-ALPHA-BENCH-V1` is the maintained M6 benchmark and report workflow.
It compares the public Rust and Mata backends with the same estimator request,
apart from the required backend/RNG selector pair. Rust uses Counter-V1 and
Mata uses the Stata RNG; cross-backend point estimates are therefore compared
with numerical-MCSE diagnostics rather than bitwise equality.

The protocol has five non-negotiable properties:

1. Stata timer 80 measures the complete command. Timers 91--98 are forbidden
   because the Mata implementation owns them internally.
2. Every backend/case runs in a fresh Stata process. Run 1 is cold and runs
   2--4 are the warm sample used for medians.
3. Each row records source SHA, fixture-spec SHA-256, route, dimensions,
   residual and accounting gates, `e(sample)`, data/sort/RNG restoration,
   result cells, MCSEs, native phase timings, and process peak RSS.
4. A timing row is accepted only after its scientific and state gates pass.
   Failed, unsupported, nonconverged, or resource-limited rows remain labeled
   results and are never silently removed or rerouted.
5. Restricted CZ18 rows remain on SCC. Only aggregate CSV/JSON evidence,
   scheduler accounting, hashes, and the report may leave the run directory.

The synthetic generator uses six stored rows per worker and three firm links
at offsets 0, 1, and 31. Integer frequencies, target weights, deletion IDs,
and observation order are deterministic. `cases.tsv` is part of the fixture
identity. The headline has 8,192 firms and 200 probes. The CZ18 headline is
bound to retained-design SHA-256
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`.

Local usage from a clean source commit:

```bash
python3 vckss/benchmarks/alpha/run_local.py \
  --output /private/tmp/vckss-alpha-benchmark
python3 vckss/benchmarks/alpha/analyze.py \
  --run-root /private/tmp/vckss-alpha-benchmark \
  --output /private/tmp/vckss-alpha-report
```

The analyzer writes `summary.json`, compact CSV, LaTeX macros, and PDF figures.
Compile `report/report.tex` with `BenchmarkDataRoot` and `BenchmarkFigureRoot`
pointing at that output. The SCC wrapper and validator use the same row schema
and analyzer. Scheduler jobs must be accepted by both `qacct` (`failed=0`,
`exit_status=0`) and the application receipt before their rows enter a report.

Acceptance is the plan contract: Rust median complete-command time must be at
least twice as fast as Mata for both 200-probe headlines, and no supported JLA
cell may be more than 10 percent slower. MATLAB is descriptive and only valid
for compatible no-control match-JLA cells; it is not an alpha gate.
