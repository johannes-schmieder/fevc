# Private fast-preparation checkpoint

VCkss source `016d1134d4afb440e7333d6fc5798206085a6866` adds an
explicitly private fast-preparation timing lane. With
`VCKSS_PRIVATE_CMG_FAST_PREP_V1=1`, and only when the private full-CMG route is
also selected, primitive preparation sorts use the standard Rust stable and
unstable sort implementations. The ordinary public build and the ordinary
private full-CMG control retain the existing bounded interruptible merge/heap
sorts.

This lane is a timing spike, not a qualification candidate. It polls UserBreak
immediately before and after each sort, but not inside the standard sort. A
production implementation must restore bounded cancellation within the sort
before it can be promoted.

## Source-bound macOS result

The clean arm64 plugin was built from the source above, Rust 1.85.1, and exact
standalone CMG commit `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`.
The source-bound plugin SHA-256 is
`59d2a858d383d34136e6bd486fed2fc7f84d748dd8fdc4d88dc916626077877d`.

On the fixed 8,192-firm / 200-probe synthetic case, one adjacent fast/control
pair produced:

| Four-thread route | Fast preparation | Ordinary control | Fast / control |
|---|---:|---:|---:|
| Complete command seconds | 77.540 | 82.252 | 0.943 |
| Native total seconds | 74.188 | 79.132 | 0.938 |
| Native solve seconds | 69.897 | 71.145 | 0.982 |
| Process elapsed seconds | 83.810 | 88.570 | 0.946 |
| Maximum complete residual | `6.971e-6` | `6.971e-6` | identical |
| Maximum iterations | 17 | 17 | identical |
| Maximum RSS bytes | 5,292,326,912 | 5,289,885,696 | 1.0005 |

The complete command improved by 4.712 seconds, or 5.7%. The deterministic
preparation phases explain 3.696 seconds of the native gain: canonicalization
saved 1.280 seconds, compression 0.675 seconds, planning 1.423 seconds, and
graph construction 0.318 seconds. The remaining 1.248-second solve difference
is treated as run noise rather than a sort benefit.

Two earlier adjacent development pairs independently produced 76.288 and
76.448 seconds for the fast lane versus 81.586 and 81.275 seconds for the
ordinary control. Their two-run medians imply a 6.2% command improvement. The
clean source-bound pair is the decision evidence; the dirty development pairs
are only a stability check.

All four corrected targets are bit-identical across the paired routes. Target
identity, complete original-system residuals, sample count, data restoration,
caller RNG restoration, and sort-RNG restoration pass. The fast route does not
alter solver tolerances, iterations, Counter-V1 probes, the estimator, or the
full-CMG hierarchy.

## Commands

Build:

```bash
VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
VCKSS_SPIKE_WORK_ROOT=/private/tmp/vckss-fast-prep-build-016d113 \
  rust/full_cmg_spike/build_macos.sh
```

Candidate run, with the ordinary control obtained by omitting only the
fast-preparation variable:

```bash
VCKSS_PRIVATE_CMG_FULL_V1=1 \
VCKSS_PRIVATE_CMG_THREADS=4 \
VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
VCKSS_PRIVATE_CMG_FAST_PREP_V1=1 \
VCKSS_PRIVATE_CMG_FIT_TOLERANCE=1e-10 \
VCKSS_PRIVATE_CMG_PROBE_TOLERANCE=1e-6 \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  vckss/benchmarks/full_cmg_spike/stata_run.do ...
```

The evidence paths and exact measurements are recorded in
`fast_preparation_checkpoint_2026-08-25.json`. This single clean pair is
sufficient to justify an SCC architectural measurement, not a performance or
release claim.
