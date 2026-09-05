# Corrected observation-inference confirmation

## Decision: one frozen gate fails

The independent confirmation at source
`73fa75805c8cef6d4d1a6ad843da5894ccedc956` is **FAIL**, not a pass with an
exception. All 20 original V5 design/dimension cells, 100 tasks, and 200,000
target attempts are complete. The sole failed scientific gate is:

| Cell and target | Empirical SD | Mean reported SE | SD / mean SE | Required maximum |
| --- | ---: | ---: | ---: | ---: |
| One-mode `structured_leverage`, k=16, firm variance | 0.279145 | 0.253490 | 1.101204 | 1.100000 |

The ratio exceeds the bound by 0.001204. Empirical sampling dispersion is
about 10.12% larger than the average SE; the registered allowance was 10%.
This is a narrow cutoff miss, not evidence of a large failure of the point
estimator or the curvature calculation. Conversely, its small size does not
make the preregistered gate pass. Monte Carlo variation is a possible
explanation, not an established diagnosis and not grounds for a favorable
rerun.

That row has 2,500 computed intervals out of 2,500 attempts, coverage 93.52%
(MCSE 0.492 percentage points), and bias 0.002968 (MCSE 0.005583). Its
availability, bias and coverage gates pass. The q1 interval is not simply
the point estimate plus or minus 1.96 times this SE, so the interval-coverage
and SE-calibration checks need not have identical decisions.

All other frozen scientific gates pass. This includes all 24 primary q0
rows, the other primary q1 rows, the prospectively nonprimary covariance
availability rule, and the mild/severe-model rules. None is used to override
the overall FAIL decision. The complete record is
[`rc_observation_inference_v1_result.json`](rc_observation_inference_v1_result.json).

## Complete result and failure accounting

The primary q0 rows compute 60,000/60,000 intervals, cover 94.32--95.92%,
and have SE ratios 0.9683--1.0131. The 15 primary q1 rows compute
37,500/37,500 intervals, cover 93.52--95.60%, and have SE ratios
0.9748--1.1012. Across the complete matrix there are 192,234 successful
target attempts, 7,175 `q0_variance_failed` attempts and 591 `q1_failed`
attempts. All are classified and enumerated by cell, dimension, target,
status, and inclusive replication ranges in the JSON. No attempt is removed
from the availability denominator. There are no variance-fit, process,
missing-file, schema, mixed-source, or inventory failures.

The typed withholdings occur in null/weak and prospectively nonprimary
covariance rows; no primary correct-model worker/firm/total q1 result is
withheld. The original V5 harness is target-local and checks each target's
scalar covariance, not the public attachment's additional full joint-PSD
boundary. Its target availability must not be advertised as a measurement
of whole-command public availability.

Mild-model eligible coverage is 92.24--96.08% and passes the original bounds.
Severe omitted-driver designs visibly invalidate inference: total coverage
is 82.48% for q0 and 77.68% for q1. The deliberate multi-mode covariance
target remains outside the q1 coverage claim even when it computes. The
maximum direct remainder identity error is `1.475e-13`. Successful q1 rows
also pass the corrected curvature identity and fixed actual-fold checks.
No algebraic or numerical failure is indicated by the single failed SE gate.

## Frozen execution and independent audit

The registration is
[`rc_observation_inference_v1.json`](rc_observation_inference_v1.json), SHA-256
`ccb584b63a261e8be7964018d61f586cb17fe01dc8911de2218440f75daa34d7`.
The matrix, 2,500 replications per cell and scientific thresholds are exactly
the original V5 contract. Confirmation seed is `0xc247f9083ae16d5b`;
the separate pipeline seed is `0x9816ac5d72b0f3e4`; outcome-free folds retain
seed 8675309. No cutoff, exclusion, model, or outcome was tuned after inspection.

The clean local tiny path first passed 80 outcome-free geometry records and
all 160 target attempts. Reversed order and changed shard size gave
byte-identical raw and summary output. The one-task SCC smoke
7466902/7466903/7466904 passed its 16 target attempts, all three scheduler
records, and independent local reaggregation before confirmation submission.

Confirmation run:
`/projectnb/welfgr/vckss/runs/20260905T083655Z-rcobs-confirmation-73fa758`.
Build 7466916 took 31 seconds and 1.664 GB. Array 7466917 has 100 records,
27--108 seconds per task, at most 107.812 MB, and 5,602.562 CPU seconds total.
All 101 build/task records have `failed=0` and `exit_status=0`. Aggregate
7466918 took 39 seconds and 1.232 GB. It has `failed=0` but deliberately
returns `exit_status=1` because a scientific gate failed. This is correct
fail-closed operation, not a reason to restart the job. All stages used one
slot, owner `johannes`, project `welfgr`; Rust is 1.85.1 and SCC Python 3.13.8.

Independent local reaggregation under Python 3.13.0 reproduces the FAIL
decision and both raw and summary bytes exactly. Every task hash, source,
manifest, binary, target key, attempted replication, status, final task/build
log marker, fold fingerprint and scheduler record reconciles. The complete
aggregate's deliberate nonzero exit is audited separately from scheduler
failures. The confirmation download succeeded without a retry; the smoke's
earlier macOS directory metadata error was resolved by a checksum-only resume.

Durable ignored archive:
`.local/diagnostics/rc-observation-73fa758-verified.tar.gz`, SHA-256
`1ba403bfb42d58380a5e58a1655a1254d3326ba7808e8b287c8e8bc986650bb9`.
Its gzip integrity and 379-member inventory were checked. It contains the
exact source bundle and Linux example binary, all raw task and aggregate
data, summaries, manifests, receipts, accounting, logs, local audit and
reaggregation receipt, and prerequisite evidence. It is not a release archive.

Independent validation used:

```bash
./.venv/bin/python fevc/tools/run_rc_observation_inference.py aggregate /private/tmp/fevc-rcobs-73fa758/scc-confirmation/input/campaign-manifest.json /private/tmp/fevc-rcobs-73fa758/scc-confirmation/output/tasks /private/tmp/fevc-rcobs-73fa758/scc-confirmation/revalidated --build-receipt /private/tmp/fevc-rcobs-73fa758/scc-confirmation/receipts/build.json
```

The command returned 1 as required. No new confirmation or diagnostic
simulation was submitted after this result.

## Release-candidate consequence

The bounded RC sequence pauses before public match integration because its
fresh observation prerequisite did not pass. The separately passed match-q0
confirmation and accepted repaired match-q1 confirmation remain intact.
There is no reason here to reopen joint-control inference or change the
fixed-offset decision.

The recommended next decision is a short review using these existing outputs
to quantify uncertainty in the failed ratio and compare the leverage-only
row with the primary structured-common model. That review is not yet executed
and cannot retroactively change this FAIL. Any reduced release scope or new
scientific attempt requires an explicit prospective owner decision. Until
then, no public match ABI/ado/help change, exact-artifact RC qualification,
tag, push, release, or binary distribution is claimed.
