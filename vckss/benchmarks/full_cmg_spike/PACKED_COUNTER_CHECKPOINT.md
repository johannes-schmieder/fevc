# Packed Counter-V1 checkpoint

VCkss source `9dbd03e06759a638eae9abea3386d752c577b5cc` computes all four
Philox lanes from each frozen Counter-V1 block once. The logical address,
word, sign, probe ordering, batch invariance, and output layout are unchanged;
only redundant block recomputation is removed. An exhaustive unit test over
aligned and unaligned probe spans compares the packed output to the original
scalar-word oracle.

The clean source-bound 4-thread, 8,192-firm, 200-probe macOS run passed in
104.329 seconds. This is 8.546 seconds (7.6%) faster than the direct `1e-6`
checkpoint and 39.2% faster than the registered maintained-MATLAB comparator.
VCkss is now 1.65 times as fast as MATLAB on this development run, but remains
18.463 seconds above the private 2x promotion gate.

| Source-bound headline run | Direct `1e-6` | Packed Counter-V1 | Packed / direct |
| --- | ---: | ---: | ---: |
| Complete command seconds | 112.875 | 104.329 | 0.924 |
| Native solve seconds | 102.269 | 93.564 | 0.915 |
| Counter generation seconds | 12.447 | 3.801 | 0.305 |
| Standalone CMG solve seconds | 57.168 | 57.837 | 1.012 |
| Extraction/recovery/residual seconds | 16.752 | 17.128 | 1.022 |
| Maximum complete residual | `6.97e-6` | `6.97e-6` | identical |
| Process peak-footprint bytes | 5,230,433,512 | 5,227,599,152 | 0.999 |

All four corrected targets are bit-identical to the direct checkpoint. Target
identity, accounting, `e(sample)`, data, caller RNG, and sort restoration pass.
The result therefore changes neither the estimator nor the relaxed statistical
acceptance policy.

The new engine diagnostic divides the remaining native time into registered
phases. Counter generation is no longer the dominant nonsolver phase. The
largest next opportunities are the 17.128 seconds spent extracting firms,
recovering worker effects, and certifying complete residuals, followed by
roughly 10.8 seconds of target construction, leverage accumulation, and target
contraction. Standalone CMG remains the largest single phase at 57.837 seconds.

Build command:

```bash
VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
VCKSS_SPIKE_WORK_ROOT=/private/tmp/vckss-packed-counter-build \
  rust/full_cmg_spike/build_macos.sh
```

Benchmark command:

```bash
VCKSS_PRIVATE_CMG_FULL_V1=1 \
VCKSS_PRIVATE_CMG_THREADS=4 \
VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
  /usr/bin/time -l -o /private/tmp/vckss-packed-counter-run/resources.txt \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  vckss/benchmarks/full_cmg_spike/stata_run.do ...
```

The exact build and command identities, phase timings, gates, hashes, and
temporary evidence paths are recorded in
[`packed_counter_2026-08-25.json`](packed_counter_2026-08-25.json). This is one
development run, not the alternating five-run qualification.
