# Direct probe-tolerance checkpoint

VCkss source `f615918794e01fb3e96ceec83c119981b8981834` removes the private
100-fold over-solve from randomized full-CMG probe solves. The effective and
inner probe tolerance are both `1e-6`; the independent complete original-system
residual gate remains `1e-5`. The deterministic fit keeps its two-order private
margin because it has no Monte Carlo envelope and accounts for one RHS.

The clean source-bound macOS run passed in 112.875 seconds. That is 20.5%
faster than the paired tight fused-`f64` checkpoint and 34.3% faster than the
registered maintained-MATLAB comparator. VCkss is now 1.52 times as fast as
MATLAB on this single development run, but it still misses the private 2×
promotion gate.

| Four-thread 8,192-firm / 200-probe run | Tight fused `f64` | Direct `1e-6` probe | Direct / tight |
| --- | ---: | ---: | ---: |
| Complete command seconds | 141.953 | 112.875 | 0.795 |
| Native solve seconds | 131.194 | 102.269 | 0.780 |
| Native total seconds | 138.848 | 109.801 | 0.791 |
| Maximum randomized iterations | 12 | 8 | 0.667 |
| Maximum complete residual | `9.86e-8` | `6.97e-6` | — |
| Complete-residual gate | `1e-5` | `1e-5` | unchanged |
| Process peak-footprint bytes | 5,676,652,112 | 5,230,433,512 | 0.921 |

The four corrected-target differences from the tight route are between
`2.58e-12` and `2.00e-11`. The largest is only `9.04e-5` of its reported MCSE
and `9.04e-4` of the active common-probe acceptance envelope. Target identity,
`e(sample)`, data, RNG, and sort restoration all pass.

The retained native diagnostic stream accounts for all 601 RHSs:

| Native batch phase | Seconds |
| --- | ---: |
| RHS construction | 1.707 |
| Standalone CMG solves | 57.168 |
| Firm extraction, worker recovery, and complete residuals | 16.752 |
| Other work inside the native solve phase | 26.641 |
| Total native solve phase | 102.269 |

This makes the next optimization target concrete. Krylov work is still the
largest named phase, but 43.4 seconds now lie outside the standalone solves:
16.8 seconds in per-column extraction/recovery/certification and 26.6 seconds
in JLA generation, accumulation, layout/ABI, and other uninstrumented native
work. A contiguous estimator-level batch path must remove those costs before
more hierarchy tuning is justified.

Build command:

```bash
VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
VCKSS_SPIKE_WORK_ROOT=/private/tmp/fevc-inner-f615 \
  rust/full_cmg_spike/build_macos.sh
```

Benchmark command:

```bash
VCKSS_PRIVATE_CMG_FULL_V1=1 \
VCKSS_PRIVATE_CMG_THREADS=4 \
VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
VCKSS_PRIVATE_CMG_FIT_TOLERANCE=1e-10 \
VCKSS_PRIVATE_CMG_PROBE_TOLERANCE=1e-6 \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  fevc/benchmarks/full_cmg_spike/stata_run.do ...
```

The exact build identity, full command, timings, result differences, gates,
and temporary evidence paths are recorded in
[`probe_tolerance_direct_2026-08-25.json`](probe_tolerance_direct_2026-08-25.json).
This is one source-bound development run, not the alternating five-run
qualification.
