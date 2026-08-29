# Maintained-MATLAB inference comparison

This qualification compares VCkss 0.5.0-alpha.1 with the maintained
`LeaveOutTwoWay` MATLAB implementation on one identical, deterministic
1,002-observation mover sample. It is a focused inference oracle, not a
performance benchmark. The MATLAB source is called from its external checkout
and is not copied into this GPL-3.0-only repository.

## Source and execution identity

- VCkss source: `0ddba8a56a1e1aafa37a649ca6b5d3b477c51b5c`.
- Maintained MATLAB checkout: `8b957ffeb10b8465a3584fceb0265cccc48379e1`.
- `leave_out_COMPLETE.m`: SHA-256
  `54b30ebdc51b4c94873e2e3f205bbf865179220e3ad0df0e382922db7c84fc58`.
- `lincom_KSS.m`: SHA-256
  `71fb47ce35d26c91dbf97926031359ed0eb31d9916b07c167c577867620ee5a9`.
- MATLAB: R2024b Update 3 on BU SCC job `7365939`, four slots.
- Accepted immutable run:
  `/projectnb/welfgr/vckss/runs/20260829T213821Z-inference-matlab-0ddba8a-r2`.
- Accounting: `failed=0`, `exit_status=0`, 170 seconds wall time, and an
  application pass marker.

Job `7365880` is retained as a rejected harness attempt. The estimators reached
the MATLAB `lincom_KSS` output, but the result writer called the nonexistent
MATLAB function `fflush` and exited 1. The corrected writer closes and reopens
the CSV between seeds; no estimator code changed.

## What is directly comparable

MATLAB's `lincom_KSS` is fixed-effect projection inference. Its VCkss
counterpart is `project()`, not Stata's standard postestimation `lincom`.
The two implementations use the same observation-level KSS variance proxy for
this projection. The complete three-by-three projection covariance agrees to
`7.93e-14` in maximum absolute difference (`6.21e-13` relative).

| Term | VCkss coefficient | MATLAB coefficient | VCkss KSS SE | MATLAB KSS SE | SE relative difference |
|---|---:|---:|---:|---:|---:|
| `z1` | -0.142097411268 | -0.142097411282 | 0.021076740417 | 0.021076747813 | 3.51e-7 |
| `z2` | -0.138262856405 | -0.138262856401 | 0.043094575442 | 0.043094579972 | 1.05e-7 |

The small `lincom_KSS` differences come from the maintained function's PCG
solve; the dense same-formula covariance oracle agrees with VCkss at roughly
machine precision.

The corrected component point estimates are also close, but they are a
descriptive comparison because the maintained wrapper uses iterative solves.

| Component | VCkss | MATLAB | Relative difference |
|---|---:|---:|---:|
| Worker variance | 1.030790164 | 1.031842340 | 0.1020% |
| Firm variance | 0.529295236 | 0.529841318 | 0.1031% |
| Worker--firm covariance | -0.044504630 | -0.044564243 | 0.1338% |

## Component covariance boundary

The maintained `leave_out_COMPLETE` interface returns only three marginal
standard errors. It does not return off-diagonal component covariances or a
standard error for the additive total. Therefore only the diagonal entries of
VCkss's component covariance have a direct MATLAB quantity to display.

| Component | VCkss mean SE | MATLAB mean SE | Mean seedwise VCkss/MATLAB SE ratio | VCkss mean variance | MATLAB mean variance |
|---|---:|---:|---:|---:|---:|
| Worker variance | 0.0350649 | 0.0431159 | 0.8133 | 0.00122956 | 0.00185901 |
| Firm variance | 0.0290720 | 0.0232777 | 1.2490 | 0.000845196 | 0.000541882 |
| Worker--firm covariance | 0.0253078 | 0.0271887 | 0.9309 | 0.000640509 | 0.000739263 |

VCkss's five-seed mean covariance is

|  | Worker | Firm | Covariance | Total |
|---|---:|---:|---:|---:|
| Worker | 1.229559e-3 | 3.914263e-4 | -7.196653e-4 | 1.816551e-4 |
| Firm | 3.914263e-4 | 8.451957e-4 | -5.712316e-4 | 9.415887e-5 |
| Covariance | -7.196653e-4 | -5.712316e-4 | 6.405092e-4 | -9.878400e-6 |
| Total | 1.816551e-4 | 9.415887e-5 | -9.878400e-6 | 2.560571e-4 |

This matrix is symmetric and satisfies the registered linear map
`total = worker + firm + 2*covariance`. MATLAB provides no off-diagonal oracle
for it; VCkss validates those entries by dense scalar-target polarization and
the additive-map identity in its focused Stata tests.

The marginal-SE differences are expected and must not be described as MATLAB
parity. VCkss used an accepted 16-bin, fail-closed local-linear fit. The
maintained MATLAB call used its default 1,000-grid smoother and reported
negative fitted variance shares of 18.36%, 19.56%, and 17.56% for the firm,
covariance, and worker targets in every seed, then continued. VCkss rejects an
analogous materially negative fit. The two programs also use different random
number generators, so equal seed labels do not couple their Monte Carlo draws.

## Standard Stata `lincom`

Accepted component inference posts a coherent four-target `e(b)` and `e(V)`,
so Stata's standard `lincom` works directly. The focused regression test
`tests/stata/test_inference_lincom.do` verifies that

```stata
lincom worker_variance + firm_variance + 2*worker_firm_covariance
```

reproduces the posted `total_variance` estimate and standard error to below
`2e-12`. Projection rows remain in `e(projection_*)` and are intentionally not
addressed by standard `lincom`.

## Reproduction

Run `vckss_compare.do` locally from the repository to create `input.csv` and
`vckss_results.csv`, deploy those files plus `matlab_compare_scc.m` to a new
SCC run directory, and submit `run_matlab_compare.sh` with the immutable run
directory and external MATLAB checkout as arguments. Then run:

```text
python3 analyze_comparison.py vckss_results.csv matlab_results.csv \
  comparison.json component_se_summary.csv component_covariance_summary.csv
```

The `evidence/` directory contains the literal input, raw outputs, compact
comparison products, source hashes, accepted accounting, and the rejected
harness-attempt accounting. From this directory,
`shasum -a 256 -c MANIFEST.sha256` verifies the checked-in bundle without
rerunning either estimator.
