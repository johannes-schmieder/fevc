# CZ18 P200 fast-preparation decision

The exact-source private fast-preparation route passes the checksum-bound CZ18
P200 A/C/maintained-MATLAB comparison. It is 1.090 times as fast as the
registered warm MATLAB estimator command and 4.93 times as fast as the VCkss
baseline. This is the first fixed-hard-case result that beats MATLAB, but it
does not approach the private 2x promotion gate and therefore remains an
experimental route.

## Bound run

- candidate source: `f11a50e19449be72de22a136d79cdf3bfe588560`
- baseline: `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- standalone CMG: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- SCC job: `7315501` on `scc-ec4.scc.bu.edu`
- qacct: `failed=0`, `exit_status=0`, 747 scheduler seconds
- run: `/projectnb/welfgr/vckss/runs/20260826T031500Z-alpha-cz18-p200-fastprep-f11a50e`
- task SHA-256: `14bb8c39ec69222af19eb93868081376fc6bf0df1bbb1d62d0d3ff8db61ef842`
- pinned validator SHA-256: `26119fbbfb4cc16b488f8a7b78e88e1c4ff6445bc5f1487dca97f7f8485c238f`

Submission command:

```bash
env VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  vckss/benchmarks/full_cmg_spike/submit_scc_cz18_smoke.sh \
  20260826T031500Z-alpha-cz18-p200-fastprep-f11a50e 200 1
```

## Timing and memory

| Route | Registered command (s) | Native solve (s) | Native total (s) | Process peak RSS (KiB) |
|---|---:|---:|---:|---:|
| A, baseline | 252.570 | 182.703 | 229.103 | 2,736,864 |
| C-fast, direct full-CMG | 51.256 | 12.894 | 28.506 | 2,729,964 |
| maintained MATLAB | 55.864 | — | — | 4,303,192 root; 10,486,516 tree |

C-fast takes 0.918 times the MATLAB command, a 1.090x speed advantage. The 2x
gate would require at most 27.932 seconds, so 23.324 seconds—45.5% of the
candidate command—still must be removed.

The private sort switch cuts native preparation dramatically:

| Candidate native phase | Seconds |
|---|---:|
| Ingest | 0.406 |
| Canonicalization | 1.486 |
| Graph | 7.295 |
| Compression | 3.195 |
| Plan | 3.230 |
| Repeated solve | 12.894 |
| Native total | 28.506 |

Only 15.612 seconds now lie in native preparation. The command still spends
22.750 seconds outside the native phase receipt. That boundary is the new
dominant bottleneck. Even eliminating it completely would leave 28.506 seconds,
0.574 seconds above the 2x threshold, so a winning architecture must both
remove nearly all duplicated Stata preprocessing/boundary work and preserve a
small native improvement.

The shortest next lane is a private raw-input dispatch. The current public
wrapper groups and sorts 8.2 million worker/firm/match rows in Stata before the
native backend canonicalizes the same identities again. The experiment should
pass raw worker and firm identifiers to the plugin, construct implicit match
deletion units and stayer counts natively, and return the same reconciled
preparation facts. It must remain private until bounded UserBreak polling and
every public preparation, deletion, sample, residual, state, and lifecycle
contract are restored. Further full-CMG kernel tuning is not justified.

## Science and decision

The candidate uses 21 maximum iterations and has maximum complete
original-system residual `1.19218677221e-6 < 1e-5`. Target identity is zero.
All four common-probe corrected-target gates pass; the largest difference-to-
limit ratio is `7.431e-6`. Data, caller RNG, sort RNG, sample count, wrapper,
process-tree, qacct, and the pinned validator pass.

The maintained MATLAB receipt separately reports 7.495 seconds of input
validation, 100.679 seconds of pool startup, 7.296 seconds of MEX setup, 55.864
seconds in the estimator command, and 7.543 seconds of pool teardown. Those
cold components are descriptive only; the frozen decision metric remains the
warm estimator command.

Machine-readable evidence is in
`cz18_p200_fast_preparation_2026-08-25.json`. This single run justifies the next
architectural spike, not hardening, public exposure, the benchmark PDF, or an
alpha-release claim.
