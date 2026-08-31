# Mixed-precision full-CMG checkpoint

VCkss source `2f94e361f2e6da25d5d897be78355568b2e8ae1e` adds an explicitly private
mixed-precision variant of the fused independent-PCG experiment. It uses the
exact archived standalone CMG source at
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`. The finest hybrid operator,
independent PCG state and reductions, solution reconstruction, and independent
complete original-system residual certification remain `f64`. Only hierarchy
operators, inverse diagonals, and hierarchy vector traffic use `f32`.

The measured source commit was rebased over the append-only Stata receipt
`ae2ec15` before publication. Its published source-equivalent commit is
`de866c21e6b11cd5e248ec523936c6174730da75`; the only tree differences between
those two commits are `.ci/stata/latest.json` and the new receipt JSON. The
injected fused source is bound independently by SHA-256
`b3f8efe3c54dd7e4bad795a60d386e0ace8f21b011003d7ad6438e528a4138ea`.

The mixed route is preserved but disabled. In the clean source-bound paired
macOS development run it was only 2.01% faster end-to-end and 2.21% faster in
the native solve than the same fused executor in `f64`. This misses the
registered 10% speed gate. Its measured peak footprint was 1.06% higher and
its conservative admitted peak was 2.14% higher because the conversion
buffers and retained finest `f64` operator outweighed hierarchy savings. This
also misses the required material memory reduction.

| Four-thread 8,192-firm / 200-probe run | Fused `f64` | Mixed hierarchy | Mixed / `f64` |
| --- | ---: | ---: | ---: |
| Complete command seconds | 141.953 | 139.103 | 0.980 |
| Native solve seconds | 131.194 | 128.297 | 0.978 |
| Native total seconds | 138.848 | 135.949 | 0.979 |
| Maximum complete residual | `9.86343e-8` | `9.86343e-8` | 1.000 |
| Admitted private peak bytes | 1,622,731,128 | 1,657,421,044 | 1.021 |
| Process peak-footprint bytes | 5,676,652,112 | 5,736,601,168 | 1.011 |
| Maximum resident-set bytes | 6,075,187,200 | 6,131,302,400 | 1.009 |

All four corrected estimates were identical at the precision exported by the
harness. Both routes passed the unchanged `1e-5` probe complete-residual gate,
target identity, `e(sample)`, data, RNG, and sort restoration checks. Mixed
precision used 0.810 times the registered maintained-MATLAB command time of
171.733 seconds, but that remains far above the private promotion gate of
0.500 times MATLAB.

Build command:

```bash
VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
VCKSS_SPIKE_WORK_ROOT=/private/tmp/fevc-mixed-2f94 \
  rust/full_cmg_spike/build_macos.sh
```

The two Stata commands used the common prefix below; the mixed run additionally
set `VCKSS_PRIVATE_CMG_MIXED_V1=1` and wrote under
`/private/tmp/fevc-mixed-2f94-run`, while the `f64` run omitted that variable
and wrote under `/private/tmp/fevc-f64-2f94-run`.

```bash
VCKSS_PRIVATE_CMG_FULL_V1=1 \
VCKSS_PRIVATE_CMG_THREADS=4 \
VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
VCKSS_PRIVATE_CMG_FUSED_V1=1 \
VCKSS_PRIVATE_CMG_FIT_TOLERANCE=1e-10 \
VCKSS_PRIVATE_CMG_PROBE_TOLERANCE=1e-6 \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  fevc/benchmarks/full_cmg_spike/stata_run.do ...
```

This is one paired development run on a host with background activity, not the
registered alternating five-run qualification. The exact identities, paths,
measurements, estimates, and decision are recorded in
[`mixed_precision_2026-08-25.json`](mixed_precision_2026-08-25.json).
