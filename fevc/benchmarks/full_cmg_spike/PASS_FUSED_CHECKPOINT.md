# Scalar pass-fusion checkpoint

Decision: **preserve the private route and keep it disabled**.

## Implemented architecture

Commit `08565be7468c09999c984ae327d2498dcfb13fbb` adds a private
`VCKSS_PRIVATE_CMG_PASS_FUSED_V1=1` experiment to the direct hybrid full-CMG
path. It keeps standalone CMG's official hierarchy and preconditioner and runs
mathematically independent scalar PCGs. For connected graphs it fuses the
solution/residual update with deterministic norm accumulation and delays
solution centering until an explicit residual checkpoint. The official
submitted-RHS certification and VCkss's independent complete original-system
residual check remain in force.

Connectedness and route selection occur before estimator RNG. Disconnected
graphs use the ordinary scalar route before RNG; a later error fails closed.
The experiment is mutually exclusive with the older fused-block and
mixed-precision experiments.

## Source-bound local check

The clean benchmark source was
`c1ae402c47227ac0adda6c4e301382c6c25c98cb`; the exact standalone CMG source
was `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`. The build used Rust 1.85.1 and
produced plugin SHA-256
`c6a1984780a68944a9a9c3360cdcdda95ead58256d64ced422bc746567813efe`.

The exact build command was:

```bash
env VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  VCKSS_SPIKE_WORK_ROOT=/private/tmp/fevc-pass-fused-build-c1ae402 \
  rust/full_cmg_spike/build_macos.sh
```

The exact direct command ran from
`/private/tmp/fevc-pass-fused-source-bound-c1ae402/direct`:

```bash
env VCKSS_PRIVATE_CMG_FULL_V1=1 \
  VCKSS_PRIVATE_CMG_THREADS=4 \
  VCKSS_PRIVATE_CMG_DIAGNOSTICS=1 \
  VCKSS_PRIVATE_CMG_FIT_TOLERANCE=1e-10 \
  VCKSS_PRIVATE_CMG_PROBE_TOLERANCE=1e-6 \
  /usr/bin/time -l -o resources.txt \
  /Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -q -b do \
  /Users/johannes/Git/varcomp_hdfe/fevc/fevc/benchmarks/full_cmg_spike/stata_run.do \
  /private/tmp/fevc-pass-fused-source-bound-c1ae402/package \
  /private/tmp/fevc-tol-ladder-e128/input/input.csv \
  /private/tmp/fevc-pass-fused-source-bound-c1ae402/direct/stata.csv \
  candidate c1ae402c47227ac0adda6c4e301382c6c25c98cb \
  1c4ee2af8014a150f7a0cd4a9effc35a3a9be9860dddb052b1f532ad0fb0d0be \
  19744b8418ffff82461527d7426cdc976dce18bf36597555d19bf847dec5afbe \
  strong_d6 strong 1966080 6 200 2026082501
```

The exact favorable pass-fused repeat used the same command from
`/private/tmp/fevc-pass-fused-source-bound-c1ae402/pass2`, added
`VCKSS_PRIVATE_CMG_PASS_FUSED_V1=1`, and changed the output to
`/private/tmp/fevc-pass-fused-source-bound-c1ae402/pass2/stata.csv`. The full
literal commands are retained in the JSON receipt. The deliberately short sequence was
pass-fused, direct, pass-fused; the first pass-fused observation experienced a
visible transient host slowdown. Because the route was not promising, the
registered five-run median and SCC matrix were not launched.

| Observation | Command s | Native solve s | Native total s | Peak RSS bytes |
|---|---:|---:|---:|---:|
| Pass-fused 1 | 88.763 | 77.660 | 85.313 | 5,241,405,440 |
| Direct | 81.735 | 70.861 | 78.614 | 5,251,760,128 |
| Pass-fused 2 | 80.276 | 69.518 | 77.202 | 5,248,729,088 |

The favorable adjacent comparison is only a 1.8% command improvement and a
1.9% native-solve improvement. An earlier unreported development pair showed
the same order of magnitude (1.1% command, 1.3% solve). Private admitted peak
memory increased from 1,007,900,328 to 1,034,159,112 bytes (2.6%) because the
ordinary planned-fit workspace and pass-fused probe pool coexist.

All runs returned identical corrected targets, maximum complete residual
`6.97125921276e-6` against the unchanged `1e-5` gate, target-identity residual
`6.10622663544e-16`, and full data/RNG/sort-RNG restoration. The injected CMG
unit tests also verified deterministic repeated results and final residual
certification.

The gain is too small and noisy to materially close the registered SCC gap to
MATLAB. The route remains private, off by default, and unqualified; no SCC time
will be spent on it. The compact receipt is `pass_fused_2026-08-25.json`.
