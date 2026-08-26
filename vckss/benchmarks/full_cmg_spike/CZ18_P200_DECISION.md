# CZ18 P200 direct full-CMG decision

The checksum-bound fixed-CZ18 P200 run passes every VCkss scientific,
residual, state, wrapper, process, and scheduler gate, but the private route is
not competitive with the registered warm maintained-MATLAB estimator command.
It therefore does not enter hardening, public exposure, or the benchmark-PDF
stage.

## Bound run

- candidate source: `506d2538177218ba3fc897cb1cc946f904761be9`
- baseline: `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- standalone CMG: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- SCC job: `7314918` on `scc-fe2.scc.bu.edu`
- qacct: `failed=0`, `exit_status=0`, 737 scheduler seconds
- run: `/projectnb/welfgr/vckss/runs/20260826T030000Z-alpha-cz18-p200-matlabfix-506d253`
- pinned validator SHA-256: `bd021c908e6d5764c9f69b0248b97cbbec21f4bfeff4b7a33383b865a032b2b7`

Submission command:

```bash
env VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  vckss/benchmarks/full_cmg_spike/submit_scc_cz18_smoke.sh \
  20260826T030000Z-alpha-cz18-p200-matlabfix-506d253 200
```

## Timing and memory

| Route | Registered command (s) | Native solve (s) | Native total (s) | Process peak RSS (KiB) |
|---|---:|---:|---:|---:|
| A, baseline | 235.251 | 168.588 | 212.999 | 2,735,480 |
| C, direct full-CMG | 89.698 | 21.182 | 66.883 | 2,734,064 |
| maintained MATLAB | 55.383 | — | — | 4,335,512 root; 10,446,344 tree |

C is 2.62 times as fast as A but takes 1.620 times the registered MATLAB
estimator command. It misses both competitiveness and the 2x target.

The MATLAB receipt separately reports 7.485 seconds of input validation,
84.024 seconds of pool startup, 6.850 seconds of MEX setup, 55.383 seconds in
the estimator command, and 7.999 seconds of pool teardown. Their 161.742-second
sum makes C look 1.80 times faster under a cold-component total, but that is a
descriptive sensitivity only. The frozen decision metric is not changed after
observing the result.

## Science

The candidate completes with 21 maximum iterations and maximum complete
original-system residual `1.19218677221e-6 < 1e-5`. Its target-identity
residual is zero. All four common-probe corrected-target differences pass; the
largest difference-to-limit ratio is `7.431e-6`. Data, RNG, and sort RNG state
are restored. The MATLAB corrected targets remain descriptive because target
weights, RNG draws, and solver tolerances are not common.

## New bottleneck and next lane

Only 21.182 seconds, 23.6% of the candidate command, is repeated solve time.
Native preparation consumes 45.701 seconds: canonicalization 17.186, plan
construction 13.006, compression 8.395, graph selection 6.648, and ingest
0.465. Another 22.815 seconds lies outside the native phase total. Thus 68.516
seconds is preparation and command-boundary work.

The next private experiment targets the deterministic preparation sorts.
Current cancellable generic heap/merge sorts repeatedly order millions of
primitive rows. A private explicit-consent lane will measure standard Rust
stable/unstable sorting while leaving the ordinary interruptible path and all
public contracts unchanged. It is worth hardening only if the complete command
improves materially; full-CMG kernel micro-optimization remains stopped.

Machine-readable evidence is in `cz18_p200_decision_2026-08-25.json`.
