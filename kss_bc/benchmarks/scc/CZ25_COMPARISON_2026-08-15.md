# CZ25 MATLAB, B1, and forced-CMG comparison

This record closes the next-larger-CZ scale step under KSS-NUMOPT-1. It is
finite numerical and software evidence, not a production-routing decision or
an asymptotic claim.

## Source and sample binding

- SCC run: `/projectnb/welfgr/kss-bc/runs/20260816T002101Z-7f07d27`
- source commit: `7f07d27d543b9e947ef557e41f406b77bf9400da`
- source archive SHA-256:
  `6d34080bb0a5d19175555874ef17bede3f156447dbf368488317881536eeab20`
- CZ25 wage-input SHA-256:
  `1327122c7cdca58975a8cbf59862534abffaece6f30401969a49c1479980a382`
- MATLAB core SHA-256:
  `7ab72bcf1f9e1a0091a6a423b1ef5cbd23688f7c64d753cf9adcc6243989a120`
- MATLAB CMG entry/MEX-family/solver-family SHA-256:
  `90c995f6fca12026827a6ad6ad3ddced042c94fa81d8b7335e9abbdf1f841140`,
  `f3c981994865171a77b458b0624ab85b506d276805807fcfd28a42bffdae2347`,
  and `6dfabc3f4adeb89b81bf739ebbfc23767a791349256644066b42ed9176f02b83`.

CZ25 is the next larger common-AKM commuting zone after CZ24 in the existing
all-CZ manifest: 1,236,658 source rows versus 854,267. The full CZ25 parent
has 21,912 workers and 2,450 firms. The first MATLAB pass reports a retained
set with 1,092,670 rows, 5,825 movers, 1,780 firms, and 28,007 matches. The
independent KSS graph audit removes 702,542 non-core rows and no additional
match bridges. The final shared bridge core has 390,128 rows, 5,825 workers,
1,780 firms, and 15,097 matches.

The final DTA and CSV hashes are, respectively,
`9b9c862fb4ff9a06d0b8b9dd0069246689f74c22a01f7d30ceba36e21cb312cd`
and `1f9d863cae601ab2139f8bbf47635cde62c2f3362af4128920332b2a6a4b0803`.
MATLAB, B1, and forced CMG were submitted concurrently at 20:47:18 EDT and
started together at 20:49:25 on `fhs-pub@scc-md2.scc.bu.edu`, with four slots
per job. Every route used 200 probes and seed `8675309`. B1 and CMG also used
tolerance `1e-10`, batch size eight, and `observation_key` probe order. Stata
reported version 19, flavor IC; MATLAB reported R2025b.

## Timing and memory

Times are seconds. KSS stage timers are nested and therefore do not sum to
the command time. The maintained MATLAB code does not expose separate Schur,
preconditioner, PCG, or target-probe timers; its log reports 6.673 seconds for
the leverage section. MATLAB's GNU/SCC wall includes parallel-pool startup.

| route | setup | Schur | preconditioner | PCG | leverage | target | command | GNU wall | SCC wall | peak RSS KiB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| MATLAB | 5.457 | n/a | n/a | n/a | 6.673 | n/a | 6.924 | 155.44 | 156 | 1,737,096 |
| B1 | 1.514 | 493.971 | 1.218 | 521.824 | 240.192 | 386.279 | 642.986 | 644.42 | 645 | 596,672 |
| forced CMG | 4.539 | 5.681 | 19.885 | 36.765 | 69.027 | 80.471 | 166.550 | 168.11 | 169 | 702,704 |

Forced CMG is 3.861 times faster than B1 by command time. MATLAB is 92.86
times faster than B1 and 24.05 times faster than CMG by its internal command
timer. Including MATLAB MEX setup changes those ratios to 51.93 and 13.45.
Using end-to-end SCC wall, MATLAB is 4.13 times faster than B1 and 1.08 times
faster than CMG. The command and SCC-wall comparisons answer different
questions and must remain separate.

## Complete RHS checks

Each KSS route records 601 complete RHS checks: one inverse action, 200
leverage probes, and 400 target probes.

| route/stage | RHS | iteration min/median/max | maximum complete residual |
|---|---:|---:|---:|
| B1 inverse | 1 | 85/85/85 | `1.1303e-11` |
| B1 leverage | 200 | 82/87/90 | `4.6727e-11` |
| B1 target | 400 | 85/91/92 | `9.9488e-11` |
| CMG inverse | 1 | 1/1/1 | `8.9544e-15` |
| CMG leverage | 200 | 1/1/1 | `3.6248e-14` |
| CMG target | 400 | 1/1/1 | `8.4162e-14` |

The separately reported maximum solver/inverse residual is `9.9488e-11` for
B1 and `4.0364e-13` for CMG. Both are below the registered `1e-9` acceptance
ceiling.

## Numerical comparison

| corrected target | B1 | forced CMG | MATLAB |
|---|---:|---:|---:|
| worker | 0.0752791176952 | 0.0752791176924 | 0.0743483392060 |
| firm | 0.0235551143299 | 0.0235551143296 | 0.0238791735238 |
| covariance | 0.0117556803659 | 0.0117556803675 | 0.0113360858843 |
| total | 0.1223455927569 | 0.1223455927570 | 0.1208996844984 |

The B1/CMG matrix relative difference across plug-in, correction, corrected,
and numerical-MCSE outputs is `2.1744e-11`. The two routes therefore agree
inside registered solver tolerance. MATLAB is close but not identical: the
B1-minus-MATLAB differences are 1.24% of B1 for worker, -1.38% for firm,
3.57% for covariance, and 1.18% for total. This is descriptive because the
legacy MATLAB finite projection and language-specific probe stream differ
from API 17; it is not an equality oracle.

The comparison job finds 15,097 retained matches in every route and zero
one-sided matches for every pair. The small current-source exact prerequisite
also passes on 11,549 rows in 2.096 seconds, with maximum residual
`1.4309e-13`.

## SCC accounting and gate result

| job | ID | failed/exit | SCC wall | CPU seconds | max vmem |
|---|---:|---:|---:|---:|---:|
| parent prepare | 7190551 | 0/0 | 10 | 23.286 | 709.000M |
| parent MATLAB | 7190587 | 0/0 | 219 | 231.924 | 43.619G |
| final adapter | 7190608 | 0/0 | 25 | 43.564 | 7.914G |
| exact prerequisite | 7190596 | 0/0 | 2 | 7.864 | 571.656M |
| final MATLAB | 7190626 | 0/0 | 156 | 217.919 | 44.038G |
| final B1 | 7190627 | 0/0 | 645 | 2,512.185 | 952.957M |
| final CMG | 7190628 | 0/0 | 169 | 594.100 | 1,010.992M |
| overlap comparison | 7190670 | 0/0 | 2 | 3.823 | 270.266M |

The privacy-safe aggregate evidence passes
`validate_matlab_subset.py --omit-exact --include-matlab`, including source
hashes, the current-source exact prerequisite, projections, complete RHS
residuals, B1/CMG estimator equality, MATLAB identity, memory, scheduler
accounting, and three-way retained-match equality. Every estimator projection
was at most 1,800 seconds and every wrapper retained its independent
5,400-second timeout.

This evidence supports forced CMG for repeated-RHS moderate/weak systems. It
does not enable automatic routing. B1 remains the production route for easy
or well-conditioned graphs; CMG is still test-only and uninstalled.
