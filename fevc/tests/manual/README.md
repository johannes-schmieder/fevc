# Manual referee checks

These files provide accessible diagnostics that can be launched from Stata.
They are not release qualification or a substitute for the source-bound test
and benchmark evidence elsewhere in the repository.

Requirements:

- Stata 18 or newer and an installed `fevc` package;
- Python 3 (only its standard library is used to construct a temporary,
  source-bound benchmark copy of the installed Ado file);
- MATLAB with the `-batch` command-line interface and Parallel Computing
  Toolbox; and
- a checkout of the maintained `LeaveOutTwoWay` MATLAB package. The comparison
  is designed for the maintained `codes/leave_out_KSS.m` interface.

For machine-specific paths, copy `.fevc_manual_local.do.example` to
`.fevc_manual_local.do` in this directory and edit the copy:

```stata
local matlab_root "/path/to/LeaveOutTwoWay"
local matlab_binary "/path/to/matlab"
```

The local file is ignored by Git. Both entry points automatically include it
in their own local-macro scope, so it works when the scripts are launched from
within Stata and no machine path is embedded in tracked code. If MATLAB is on
the operating-system path, omit `matlab_binary`. `KSS_MATLAB_ROOT` remains a
fallback when the local file does not set `matlab_root`.

From the repository root, open either entry point in Stata and run it.
Then run `manual_validation.do` for the compact checks or
`manual_benchmarks.do` for the timed size/core slices. Each prints a terminal
`FEVC_REFEREE_SUITE|...` or `FEVC_MANUAL_BENCHMARK|...` receipt; the benchmark
also writes CSV, DTA, and PNG results to its configured output directory.

The benchmark writes exactly two combined timing figures. The first varies
dataset size at eight cores; the second varies requested cores from 1 through
16 at the medium dataset size. Each has side-by-side panels for the two-way
dense, two-way sparse, and two-way bottleneck designs. Every panel has three
lines: fevc's Mata backend, fevc's Rust backend, and maintained MATLAB. A Mata
point above the local Stata processor cap is recorded as `capped` and omitted.
Rust is not constrained by the Stata license: its native worker pool and the
corresponding MATLAB pool are measured at every requested core count through
16. The CSV/DTA output records Rust's requested and used native-thread receipts.
The shared medium/eight-core cell is run only once.

`manual_benchmarks.do` uses `fevc_manual_build_ado.py` to create a temporary
in-memory benchmark copy of the installed `fevc.ado`. That copy alone accepts
a guarded manual thread contract so the native Rust pool can exceed
`c(processors)`; it reconciles `e(cmg_threads_requested)` and
`e(cmg_threads_used)` at every Rust point. The installed public command and its
defaults are not changed. Set `python_binary` in the local settings file if
`python3` is not on the operating-system path.

The settings blocks at the top of `manual_validation.do` and
`manual_benchmarks.do` hold portable test parameters. Keep machine paths in
`.fevc_manual_local.do`; it may set `matlab_root`, `matlab_binary`, and
`python_binary`.
For automated smoke runs, the benchmark also honors `FEVC_MANUAL_SIZES`,
`FEVC_MANUAL_MEDIUM_SIZE`, `FEVC_MANUAL_FIXED_THREADS`,
`FEVC_MANUAL_THREADS`, `FEVC_MANUAL_REPETITIONS`, `FEVC_MANUAL_OUTPUT`, and
`FEVC_MANUAL_ALGORITHM_SETTINGS`.

`algorithm_settings` defaults to `default`, which leaves all three estimator
arms' algorithm, tolerances, and iteration ceiling at their command defaults.
The shipped sizes all exceed the maintained MATLAB package's 10,000-row threshold,
so the defaults select 200-probe JLA. Set it to `harmonized` to request JLA
explicitly in all three arms and align the maintained implementation's 200
probes, `1e-10` fit tolerance, `1e-6` probe tolerance, and 1,000-iteration
ceiling. The default mode is the intended referee-facing benchmark.

`fevc_matlab.ado` is a diagnostic wrapper. It exports a temporary finite
numeric sample, launches a fresh MATLAB process, and imports the four corrected
targets and timing receipts. It never copies or modifies the external MATLAB
source. If the maintained package's CMG MEX files are not already available,
the companion bridge compiles them into a temporary per-Stata-session cache.
Omitting `algorithm()` on the wrapper invokes the maintained package's own
size-dependent default; the selected algorithm is returned in
`r(selected_algorithm)`.

The validation fixture contains one literal observation per worker--firm match
and no stayers or controls. This deliberately aligns the deletion, target,
weight, nuisance, and population conventions used for the descriptive
exact-algorithm comparison.
The benchmark uses three two-observation worker--firm matches per worker, and
all three arms delete the same match blocks. The bottleneck panel has two
equally sized, internally connected firm communities joined by four bridge
workers. This creates a weak cut while ensuring that deleting any single match
does not disconnect the graph. Mata, Rust, and MATLAB use independent JLA
draws; corrected-result differences are diagnostics, not an equality test.
