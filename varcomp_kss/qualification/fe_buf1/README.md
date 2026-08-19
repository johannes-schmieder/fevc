# FE-BUF-1 qualification evidence

This directory binds measurement baseline
`7f5a38b49cccd5bf0396f62fedd542881d68bad1` to runtime candidate
`1cb441f20be0d747483cd8f81746af4187192d0f`.

The deterministic source bundles used on SCC have SHA-256 hashes:

- baseline: `61f15ae1fc380b8db429f9c342cb6274c16724ec91c659590be48f426e306321`;
- candidate: `ec25639b6a42ea22d7b114ded94a7fe4cca56350b659eeea8b4147c16d1f342b`.

`local_ab/` and `local_ba/` are the archive-isolated same-machine
baseline-first and candidate-first comparisons. Each contains the raw CSV,
complete Stata log, and a summary that binds the archived commit/tree and
artifact hashes. Both report exact scientific comparison.

`synthetic/` contains the accepted raw CSVs, logs, pair markers, and qacct
receipts for F256, F1024, F4096, and F8192 in both source orders. The raw SCC
source is:

`/projectnb/welfgr/varcomp-kss/fe-buf1/runs/20260819T115747Z`

`synthetic_large/` contains the separately labeled F15625 cold,
single-repetition endpoint. Accepted AB job 7229208 comes from:

`/projectnb/welfgr/varcomp-kss/fe-buf1/runs/20260819T125441Z-f15625-single-fe-buf1`

Accepted replacement BA job 7230511 comes from:

`/projectnb/welfgr/varcomp-kss/fe-buf1/runs/20260819T150000Z-f15625-single-fe-buf1-ba-r2`

Both accepted jobs pass qacct and exact pair validation. The original
three-repetition F15625 tasks 7228370 and 7228371 were canceled before a
complete role receipt when the empirical F8192 wall projection exceeded the
two-hour pair envelope. The first one-repeat BA job 7229209 was killed at its
originally submitted two-hour limit after completing only the candidate half;
its rejected qacct receipt is retained, but none of its timing is accepted.

`cz18/` is the definitive fixed-retained-data rerun at 8,201,888 rows and P20.
Its raw SCC source is:

`/projectnb/welfgr/varcomp-kss/fe-buf1/runs/20260819T125913Z-cz18-p20-fe-buf1-r3`

The input SHA-256 is
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`.
Jobs 7229227 and 7229228 pass qacct and all twelve matched role calls. The
summary requires exact science, solver work, lifecycle, sample, data, sort,
and RNG records. It reports a neutral complete-command comparison while
retaining the raw per-order/per-round timing records.

The CZ18 qacct record also preserves two rejected wrapper revisions. Jobs
7229180--7229181 did not yet admit the registered compressed success status;
jobs 7229192--7229193 asserted the wrong registered result-matrix row count.
All four returned wrapper status 1 and none contributes accepted timing.

`synthetic_summary.json` is the replayed PASS summary across all ten accepted
synthetic pairs. The interpretation, validation gates, and next optimization
recommendation are in `../../docs/FE_BUF_1_RESULTS_2026-08-19.md`.
