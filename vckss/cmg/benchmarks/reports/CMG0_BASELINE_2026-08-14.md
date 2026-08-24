# CMG0 baseline — 14 August 2026

## Scope and source

- Startup commit: `744ca8ed7271b791a721d1d05011d864d439801a`
- Worktree: `main`, ahead of `origin/main` by four commits at startup
- Existing untracked GPT Pro CMG packets and `varcomp_naming.md` were preserved
- Handover verification: PASS
- Environment doctor: PASS
- Handover test scope: 12 passed

This report freezes pre-CMG evidence. It does not rerun or relabel historical
package qualification, and it does not claim that current in-progress package
milestones are complete.

## PPML diagonal-PCG baseline

The retained Stata/MP 18 stable-mobile ladder uses 100 leverage plus 100
correction probes, batch eight, tolerance `1e-8`, and maximum 5,000 iterations.

| Rows | Stata seconds | Peak RSS (GB) | Maximum iterations |
|---:|---:|---:|---:|
| 10,000 | 0.618 | 0.075776 | 11 |
| 100,000 | 5.207 | 0.243384 | 15 |
| 1,000,000 | 54.283 | 2.057814 | 17 |
| 5,000,000 | 292.465 | 10.794041 | 18 |
| 10,000,000 | 629.950 | 19.656180 | 19 |

The retained one-million weak ring with 2+2 probes took 109.450 Stata seconds
and safely withheld as `LEVERAGE_SOLVE_STAGNATED` at the registered
5,000-iteration gate. This is the primary CMG rescue target. The stable ladder
is the primary easy-graph no-regression target.

The retained Stata/MP 19 SCC ladder used the four-core site license. Its
ten-million result took 4,114.027 Stata seconds, measured 14.108975 GB RSS, and
used 19 iterations. Cross-platform runtime is compared only within host and
Stata version.

Authoritative historical reports:

- `ppml_talo/benchmarks/reports/LOCAL_STATA18_2026-08-12.md`
- `ppml_talo/benchmarks/reports/SCC_STATA19_2026-08-12.md`

## KSS baseline boundary

KSS KB5 and KB6 remain in progress. The retained synthetic ladder is smoke
5,000 workers/250 firms/40 probes, medium 50,000/2,500/100, and large
250,000/10,000/200. Its existing matrix-RHS service calls scalar PCG once per
column; therefore B1 must isolate the effect of true lockstep batching before
CMG performance is credited.

Authoritative harness: `kss_bc/benchmarks/README.md`.

## New CMG0 oracle evidence

Command:

```text
./.venv/bin/python -m pytest shared/cmg/tests/test_oracle.py -q
```

Result: `14 passed in 0.40s`.

Covered identities include deterministic duplicate collapse, degree-one
zero contribution, degree-two/three cliques, degree-four auxiliary stars,
hub allocation bounds, exhaustive small incidence cases, 500 rational random
cases, exact Galerkin contraction, quotient-SPD V-cycle materialization, KSS
and PPML pullbacks, and the nonsymmetric PPML extraction counterexample.

## Open CMG0 gates

- canonical Mata source and deterministic assembler;
- native Stata/Mata oracle comparison;
- kernel feasibility measurements;
- ownership handoff before package integration;
- adversarial reviews after design and code candidate completion;
- local/SCC qualification and licensing decision.
