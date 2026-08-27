# VCkss 0.4.0-alpha.1 full-CMG benchmark decision

## Decision

The normal-build scalar direct hybrid-Laplacian route is accepted as
`CMG_FULL_V2` for the qualified no-control match-JLA cell. Runtime source
`4b6874ededa1244bca389e4ee82148b42b9693a1` is faster than matched maintained
MATLAB on both registered hard cases, is faster than the source-bound private
winner, uses less observed process-tree memory than MATLAB, and passes the
registered statistical, complete-residual, state, and lifecycle gates.

This is a private `0.4.0-alpha.1` candidate. The report makes no Windows,
2x-MATLAB, tag, public-distribution, or public-release claim.

## Architecture

VCkss places each transformed firm-Schur RHS on the firm vertices of its
prepared hybrid worker--firm Laplacian, places zeros on auxiliary worker
vertices, and solves the full graph with the vendored deterministic CMG
hierarchy. One graph, hierarchy, Rayon pool, parallel plan, and admitted
workspace pool are reused across fit, leverage, and target RHSs. Firm
solutions are extracted and deterministically recentered, worker effects are
recovered, and every RHS is independently certified against the complete
original worker-plus-firm system.

The implementation keeps independent scalar PCGs, fixed column order, and
deterministic per-column reductions. Fused, mixed-precision, and pass-fused
experiments are preserved outside normal workspaces and remain disabled. A
failing complete residual triggers only the pre-registered same-route 10x
inner-tolerance ladder for the failing columns. There is no post-RNG estimator
or backend fallback.

## Source identities

| Component | Commit or identity |
| --- | --- |
| Alpha runtime | `4b6874ededa1244bca389e4ee82148b42b9693a1` |
| macOS and supply-chain receipt tip | `4dafec6734af4b8d3c25785f268f19f69f780684` |
| SCC Linux qualifier source | `992eba0947ca155c534f532750fc202e41ecf978` |
| Vendored CMG upstream | `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10` |
| Vendor and Rust 1.85.1 | `ba83e5378c6efd42f934ef1a05a1f59d79beb3b6` |
| `CMG_FULL_V2` route and receipt | `93cd9e271c59fbe799fcbe82dd2d492ed0e54656` |
| Memory admission and refinement | `6ab23cbd55b4e395a46aa8d5e6e72c53a32a9ac8` |
| Cancellation and lifecycle | `6022b7bbde844e7c82ce76d265315fcc0ed09cfe` |
| Retired experiment isolation | `0fa31259088320bddc03b2e9aaf8b6ffb43d9ebf` |
| Stale-runtime fail-closed gate | `2a83d8343a07e03a15df753d58437476748fbd95` |
| Residual certificate refinement | `dd39f049c643760727d808173c042568eedbe9b2` |
| Qualified automatic routing | `61dba320d39bd7d2ec50b29e612a598f279209b7` |
| Linux qualifier `libm` linkage | `a1053f9e62138b1c143ada35c768e29beb3b5b7b` |
| Linux qualifier direct pinned `cargo-fmt` | `fddc50f842c584ab194514df00467539639b8ea7` |
| Linux qualifier direct pinned `cargo-clippy` | `9aba2ed02feda203b8a77aff5398822653d4db7f` |
| Linux qualifier literal `cargo-clippy clippy` repair | `992eba0947ca155c534f532750fc202e41ecf978` |

## End-to-end results

Each case uses 200 projections, four application workers, one cold run, and
five position-balanced warm runs. Times and process-tree RSS values below are
warm medians.

| Case | VCkss | MATLAB | VCkss / MATLAB | Speed advantage | Private winner | Prod. / private | VCkss RSS | MATLAB RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| macOS 8,192-firm | 74.774 s | 104.489 s | 0.716 | 1.397x | 81.145 s | 0.921 | 5.121 GB | 12.161 GB |
| SCC fixed CZ18 | 19.097 s | 33.058 s | 0.578 | 1.731x | 33.942 s | 0.563 | 2.788 GB | 11.417 GB |

Both selected promotion gates pass. Neither case reaches the aspirational
`VCkss / MATLAB <= 0.5` target.

## Solver and numerical receipts

| Measure | macOS | SCC CZ18 |
| --- | ---: | ---: |
| RHS count | 601 | 601 |
| PCG iterations | 4,817 | 7,001 |
| Operator applications | 5,418 | 8,886 |
| CMG applications | 4,817 | 7,001 |
| Refinement attempts / columns | 0 / 0 | 35 / 1,284 |
| Maximum reduced residual | `7.862e-6` | `1.541e-3` attempt maximum |
| Maximum complete residual | `6.971e-6` | `9.975e-6` |
| Probe complete-residual limit | `1e-5` | `1e-5` |
| Pre-RNG forecast | 32.828 GB | 9.788 GB |
| Admitted CMG peak | 19.500 GB | 7.133 GB |
| Actual retained native storage | 1.184 GB | 0.095 GB |

The SCC attempt-level reduced maximum includes initially failing columns; all
selected columns pass after deterministic same-route refinement. Against the
common-Counter-V1 private winner, macOS corrected-target differences are zero
and SCC differences range from `9.75e-10` to `3.67e-9`, comfortably within the
registered MCSE-aware limits. MATLAB target differences are descriptive
because its RNG, solver contract, and target-weight implementation are not
pathwise comparable.

## Current bottleneck

On macOS the 59.826-second repeated-solve phase is about 80% of the complete
command and remains the dominant route to the 2x goal. On SCC, repeated solves
take only 4.131 seconds; the 9.298-second front-end remainder and the native
preparation/recovery path are now equally important. The next optimization
wave should therefore profile full-CMG applications on the synthetic macOS
case and preparation/data movement on CZ18, without returning to the retired
simplified hierarchy.

## Reproduction

macOS:

```bash
/opt/anaconda3/bin/python3 vckss/benchmarks/full_cmg_production/run_local.py \
  --output-dir /private/tmp/vckss-full-cmg-alpha-4b6874e \
  --input-csv /private/tmp/vckss-local-matrix-787327f/run/input/input.csv \
  --matlab-root /Users/johannes/Git/varcomp_hdfe/Monte_Carlo/LeaveOutTwoWay
```

SCC:

```bash
vckss/benchmarks/full_cmg_production/submit_scc_cz18.sh \
  20260827T023000Z-alpha-cz18-4b6874e
```

The per-platform reports and compact receipts are
[`MACOS_HEADLINE_4B6874E.md`](MACOS_HEADLINE_4B6874E.md),
[`SCC_CZ18_4B6874E.md`](SCC_CZ18_4B6874E.md), and the adjacent `evidence/`
tree. The CMG-style PDF and its complete LaTeX source are maintained in
[`report/`](report/).
