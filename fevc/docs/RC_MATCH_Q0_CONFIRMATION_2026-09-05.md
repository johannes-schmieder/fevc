# Fixed-offset match q0 independent confirmation

## Decision

PASS at source `bb580fe69085d1f98c9151cea2de038aec0a8ba6`. All 14 frozen
designs, 70 tasks, and 140,000 target attempts reconcile. All scientific
gates pass without changing the original q0 designs, exclusions, or cutoffs.
The 72 scheduler records have `failed=0` and `exit_status=0`. Independent
local reaggregation matches the scheduled raw output and summary CSV byte
for byte.

The 24 primary correct-model rows have 59,996 computed intervals out of
60,000 attempts: availability 99.96--100%, coverage 93.88--95.92% for nominal
95% intervals, and empirical-SD/mean-SE ratios 0.9709--1.0370. Every bias
gate passes. This completes the previously missing full match-q0 confirmation;
it is not merely the single diffuse comparator in the repaired q1 campaign.

The complete machine-readable record is
[`rc_match_q0_v1_result.json`](rc_match_q0_v1_result.json). It contains all 56
compact target summaries, every failure group and inclusive replication
range, exact hashes, accounting, and prerequisite smoke audit. The complete
unabridged summaries and raw attempts are retained in the verified archive.

## What passed, and what did not become a claim

There are 138,988 computed intervals and 1,012 shared statistical failure
attempts. The latter consist of 256 positivity failures (64 replications:
61 severe-model, one mild-model, one unequal-common-shock, one controls)
and 756 full-covariance PSD failures (189 replications: 144 null, 44 weak,
one multi-mode). Each shared failure counts against all four target attempts.
No attempt was dropped and no unavailable interval was replaced by another q.

The fixed-offset approximation is still a substantive limitation. In the
prospectively excluded estimated-controls diagnostic, total coverage is
65.95% and the SE ratio is 2.058. The present result does not undo the paired
controls diagnosis or qualify estimated-offset uncertainty. It supports the
fixed/known-offset model and a practical explicitly labeled approximation
when the offset is estimated. Having few controls alone is not a guarantee.

Mild-model coverage is 93.56--95.04% and passes its degradation gates.
Severe misspecification is visible, as required: total coverage is 80.44%
and the SE ratio is 1.516. Null/weak and deliberate spectral diagnostics
remain outside the calibration claim. CMG availability is 100%, with
descriptive coverage 94.64--95.40%. q0 still requires diffuse target geometry;
this campaign does not select q automatically or qualify multi-mode cases.

Maximum complete-system residual is `9.9984e-12`, and maximum point-correction
identity error is `1.735e-18`. There is no production-estimator change in this
campaign. The shared validator's generic receipt interpretation mentions q1;
this campaign explicitly requests q0, records zero critical draws and null
q1-specific fields, and makes no new q1 claim.

## Frozen execution and audit

Registration: [`rc_match_q0_v1.json`](rc_match_q0_v1.json), SHA-256
`1b48f5d61648bc34a7b5cf9dc21055072892c1106534ec8b384c66d4c9dfde84`.
The campaign uses 2,500 replications per design, 400 independent matches,
confirmation seed `0xeca18f437d60b295`, pipeline seed `0x73bd6a80912fc4e5`,
and fixed fold seed `0x593f7d82a106ce4b`. Probe settings are 256 estimator,
512 covariance, 128 spectrum, 256 spectral iterations; outcome-free preflight
uses 4,096 spectrum probes. Original correct-model gates remain availability
at least .98, bias within four MCSEs, coverage within `max(.03,3*MCSE)` of
.95, and SE ratio within [.82,1.18].

The clean complete tiny pipeline first reconciled 112 attempts and 56
geometry records. Reversed task order and one-versus-two-replication shards
produced byte-identical raw and summary files. The one-core SCC smoke
7466868/7466869/7466870 passed all eight attempts, all three accounting
records, and independent local reaggregation before confirmation submission.

Confirmation run:
`/projectnb/welfgr/vckss/runs/20260905T081500Z-rcq0-confirmation-bb580fe`.
Build 7466885 took 52 seconds and 1.668 GB; the 70 tasks in 7466886 took
187--366 seconds each, used at most 117.613 MB, and consumed 18,710.583 CPU
seconds. Aggregate 7466887 took 49 seconds and 1.872 GB. All jobs used one
slot, project `welfgr`, and owner `johannes`. Rust is pinned to 1.85.1;
SCC Python is 3.13.8 and local reaggregation Python is 3.13.0.

The initial download encountered macOS directory-permission metadata errors
after copying files. A checksum-only resumed transfer into the existing
directories succeeded. Exact file hashes, unique task keys, schemas, row
counts, failure counts, final log markers, fixed fold fingerprints, and all
scheduler records were then verified. This was not a scientific retry.

Durable ignored archive:
`.local/diagnostics/rc-match-q0-bb580fe-verified.tar.gz`, SHA-256
`c9eb92a58ad131be5a5100d0f9b4d1bff8fe6c3f0ba7593e58a2a14c2684c0a9`.
Its gzip integrity and 288-member inventory were checked. It includes the
immutable source bundle and Linux example binary, full raw tasks and scheduled
aggregate, receipts, logs, accounting, prerequisite evidence, and local audit.
It is not a release archive and is not committed or distributed.

The registered runner command used for independent validation was:

```bash
./.venv/bin/python fevc/tools/run_rc_match_q0.py aggregate /private/tmp/fevc-rcq0-bb580fe/scc-confirmation/input/campaign-manifest.json /private/tmp/fevc-rcq0-bb580fe/scc-confirmation/output/tasks /private/tmp/fevc-rcq0-bb580fe/scc-confirmation/revalidated --build-receipt /private/tmp/fevc-rcq0-bb580fe/scc-confirmation/receipts/build.json
```

## Remaining release-candidate boundary

The separate corrected observation confirmation must pass before public
match integration. Match-q1 scientific evidence remains tied to its repaired
confirmation source and requires a compatibility review; no rerun is justified
by documentation/example-only changes. A new public match interface will
require affected ABI, Stata, diagnostics/help, installation, and exact-source
native qualification. Point defaults remain joint nuisance handling and the
combined mover/stayer population. No push, tag, public release, or binary
distribution is authorized by this confirmation.
