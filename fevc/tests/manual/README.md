# Manual referee checks

These files provide accessible diagnostics that can be launched from Stata.
They are not release qualification or a substitute for the source-bound test
and benchmark evidence elsewhere in the repository.

Requirements:

- Stata 18 or newer and an installed `fevc` package;
- MATLAB with the `-batch` command-line interface and Parallel Computing
  Toolbox; and
- a checkout of the maintained `LeaveOutTwoWay` MATLAB package. The comparison
  is designed for the maintained `codes/leave_out_KSS.m` interface.

From the repository root, open either entry point in Stata, edit its settings
block, and run it. If MATLAB is already on the operating-system path, the only
required external setting is the maintained package checkout:

```stata
local matlab_root "/path/to/LeaveOutTwoWay"
```

Then run `manual_validation.do` for the compact checks or
`manual_benchmarks.do` for the timed size/thread grid. Each prints a terminal
`FEVC_REFEREE_SUITE|...` or `FEVC_MANUAL_BENCHMARK|...` receipt; the benchmark
also writes CSV, DTA, and PNG results to its configured output directory.

Edit only the settings block at the top of `manual_validation.do` or
`manual_benchmarks.do`. In particular, set `manual_directory`, `matlab_root`,
and, if MATLAB is not on the operating-system path, `matlab_binary`.
For automated smoke runs, the benchmark also honors `FEVC_MANUAL_SIZES`,
`FEVC_MANUAL_THREADS`, `FEVC_MANUAL_REPETITIONS`, and `FEVC_MANUAL_OUTPUT`.

`fevc_matlab.ado` is a diagnostic wrapper. It exports a temporary finite
numeric sample, launches a fresh MATLAB process, and imports the four corrected
targets and timing receipts. It never copies or modifies the external MATLAB
source. If the maintained package's CMG MEX files are not already available,
the companion bridge compiles them into a temporary per-Stata-session cache.

The validation fixture contains one literal observation per worker--firm match
and no stayers or controls. This deliberately aligns the deletion, target,
weight, nuisance, and population conventions used for the descriptive
exact-algorithm comparison.
The benchmark uses the same aligned convention but JLA draws are independent
across implementations; its corrected-result differences are diagnostics, not
an equality test.
