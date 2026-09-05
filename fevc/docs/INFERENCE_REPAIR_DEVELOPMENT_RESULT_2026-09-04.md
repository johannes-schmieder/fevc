# Corrected match inference: development result

## Decision

The frozen repaired match campaign passes all registered development gates at
source `31dd37f2954c02d223ad81175dd4ded7b5840b8d`. All 280 tasks and 22,400
target attempts reconcile; all build, task, and aggregate scheduler records
have `failed=0` and `exit_status=0`. No design, cutoff, exclusion, seed, or
estimator was changed after inspecting outcomes. This is development evidence,
not independent confirmation and not public match-inference promotion.

The exact machine-readable result, all 56 target summaries, every failed cell
and replication key, artifact hashes, and accounting summaries are in
[`inference_repair_match_campaign_v1_result.json`](inference_repair_match_campaign_v1_result.json).
The preceding native/local/SCC prerequisite gates are recorded in
[`INFERENCE_REPAIR_CHECKPOINT_2026-09-04.md`](INFERENCE_REPAIR_CHECKPOINT_2026-09-04.md).

## What improved

Across the 18 eligible correct-model q1 target/design combinations, 7,197 of
7,200 intervals are computed. Target-specific availability is 0.9975--1.0000,
coverage among computed intervals is 0.93484--0.97250, and empirical standard
deviation divided by mean estimated standard error is 0.95735--1.05066.
Bias, availability, coverage, and SE-calibration gates all pass. The single
shared fitted-variance failure in the common-shock cell counts against every
target; it is not discarded from availability or all-attempt coverage.

The original equal-mass/common, equal-mass/CMG, and leverage-sensitive cells
still have respectively 12, 8, and 11 unavailable covariance intervals.
Their replication keys exactly match the historical campaign. They no longer
suppress worker, firm, or total intervals: all three primary targets are
computed in all 400 replications in each of those cells. The covariance
target was excluded prospectively from one-mode coverage claims because its
remainder is not diffuse. A nonpositive realized q1 covariance estimate is
reported, not regularized away or replaced by a q0 interval.

The local historical-key replay had reproduced 30 of these 31 failures;
replication 26 succeeded on macOS. The Linux development run now includes
that covariance withholding, but it uses newly fixed folds. This is additional
platform/source-bound evidence, not an isolation of the precise reason for
the earlier local discrepancy.

The four diffuse q0 comparator rows and the mild-omission gates pass. The
severe omitted-driver design exposes the intended limitation: firm coverage
is 0.86480 and total coverage 0.82143 among the 392 successful replications;
their empirical-to-estimated SE ratios are 1.3053 and 1.4148. Eight shared
structured-variance positivity failures are retained in that cell.

Complete attempt accounting is:

| Outcome | Target attempts |
|---|---:|
| Computed intervals | 19,133 |
| Target-specific covariance unavailability | 31 |
| Shared structured-variance failures | 36 |
| Shared covariance PSD failures in weak/null designs | 3,200 |
| Total | 22,400 |

Both weak and null designs withhold every interval. The deliberately
multi-mode design executes, but remains outside a one-mode coverage claim.
Successful execution is not a certificate that q1 applies.

## The practical controls limitation remains

All 400 replications in the estimated-offset control cell compute intervals,
but those intervals under-cover:

| Target | Coverage | Empirical SD / mean estimated SE |
|---|---:|---:|
| Worker | 0.8675 | 1.3171 |
| Firm | 0.8900 | 1.2110 |
| Covariance | 0.8925 | 1.2468 |
| Total | 0.7625 | 1.7228 |

Point-estimate biases are small relative to their Monte Carlo standard errors
(absolute bias/MCSE below 0.59 for every target). For the total target,
empirical SD is 0.007890 versus a mean estimated SE of 0.004580. Thus the
observed gap is primarily uncertainty calibration, not a large centering bias
in this experiment.

It is not yet justified to assign the whole gap to control-estimation
uncertainty. Mean production remainder-concentration diagnostics are also
unfavorable: about 0.316, 0.327, and 0.401 for worker, firm, and total,
respectively. These are stochastic fitted-model diagnostics, not the frozen
high-resolution preflight gates. Controls, fitted aggregate variances, and
remainder geometry have not been isolated in a paired comparison.

The cell was explicitly excluded from formal coverage claims before this
campaign. Its exclusion permits the registered development PASS; it does
not make these intervals well calibrated for users with controls. Keep the
description **fixed-offset approximate inference, ignoring nuisance-control
estimation uncertainty**. Do not describe it as valid conditional inference
given a control estimate obtained from the same outcomes.

## Exact run and numerical audit

The run is
`/projectnb/welfgr/vckss/runs/20260905T001430Z-repair-development-31dd37f`.
Source and inputs were deployed read-only. Build 7461677, array 7461678
(tasks 1--280), and aggregate 7461679 used the registered one-slot,
4-GiB-per-slot dependency launcher with Python 3.13.8 and Rust 1.85.1.
No queue or CPU restriction was added. The frozen profile uses k=20,
400 independent declared matches, 400 outcome replications per design,
256 estimator probes, 512 covariance probes, 128 spectrum probes,
256 spectrum iterations, and 4,000 production critical draws.
Scientific interval comparisons use the independently checked deterministic
quadrature. The outcome-free preflight uses 4,096 spectrum probes.

Development outcome seeds reuse the original campaign's domain; outcome-free
folds are now fixed across replications. This is not independent confirmation.
The separately registered confirmation domain has not been consumed here.

Build wall time was 36 seconds, task wall times 6--33 seconds, and aggregate
wall time 13 seconds. The array consumed 3,369.996 CPU seconds in total.
Build maximum virtual memory was 2.565 GiB, task maximum 179.379 MiB, and
aggregate maximum 403.617 MiB. Each task retained its own output and receipt.
Accounting has exactly 280 distinct task IDs, all one-slot, project `welfgr`,
successful records. Application success markers, full inventory, row schema,
semantic seeds, fixed-fold fingerprints, all hashes, and source/binary/receipt
bindings pass. Local reaggregation reproduces the scheduled aggregate and
summaries byte-for-byte.

Largest observed returned numerical diagnostics are:

- complete-system residual: `9.88844e-12`;
- direct q1 remainder identity error: `4.42896e-13`;
- q1 point decomposition error: `1.66533e-16`;
- point correction identity error: `1.73472e-18`.

Key SHA-256 values are:

| Artifact | SHA-256 |
|---|---|
| Task manifest | `a0c78f798ce20d7c151176b5e7f94efd12e3ab619cc78a5a1ae745dcde2cc227` |
| Source bundle | `ca83f4b426103060c08ee110aaf75ea9fbbe1f756ac9022601733136dadce8db` |
| Linux example | `8746ce0f3995af8fa3f95866c2f4e43ba3c49c17e0a69d1f073a27f8a2da7114` |
| Aggregate | `f3c40823d11b209514741cfb1c64128422c8f3985bddbd9f987b263ec101aee7` |
| Summaries | `243db205b0d250cb40b382991a5662de7eae6ab89cf32c8b0393229eb23ebb77` |
| Aggregate receipt | `d978d7c96525dd723aa9a209990ebc72bfbec605a57bfdbfaeed12be38754392` |

Collected raw synthetic data, logs, receipts, and complete accounting are under
`/private/tmp/fevc-repair-31dd37f-development/`. Downloading SCC permission
bits initially produced macOS directory-attribute errors; resuming the
non-destructive content transfer succeeded and all content hashes passed.
There was no scientific rerun, changed source, or changed scheduler job.

## Evidence-only compatibility and remaining work

The documentation/evidence snapshot accompanying this report changes only
the active `fevc/PLAN.md` and documentation index; the two new repair
checkpoint/result reports; this campaign's new result JSON; and the new
exact-source macOS receipt packet and common CI receipt. It does not change
production, Ado/Mata posting, native build inputs, dependencies, scientific
registration, task manifest, inputs, RNG, acceptance meanings, or binaries.

The native source-content identity remains
`d5f9d246537c6b983507cd3dd1bf89bae2c11d629e9b616d63bef4a49cb33e5e`;
every entry in its saved source manifest and all ten frozen campaign-file
hashes revalidate against the current tree. Native candidate hashes remain
those in the checkpoint. The scientific result remains bound to source
`31dd37f...`; evidence additions do not turn it into a new experiment.
The evidence-only source gates pass 585 Python tests, generated-CMG checks,
CI-receipt validation, saved-packet hashes, and `git diff --check`.
No new Rust/native or scale run is justified by these evidence-only changes.

Next steps, in order:

1. Run the already registered independent match-q1 confirmation (14 designs,
   2,500 replications each), with its frozen new seeds and unchanged gates.
2. Register and perform a bounded controls diagnosis before making a practical
   controls-calibration claim: compare known and estimated offsets on the
   same design, distinguish fitted-variance error from remainder concentration,
   and examine larger numbers of independent matches with the nuisance
   dimension fixed. This proposal does not add the excluded second-stage
   nuisance correction, and must be frozen before its new results are seen.
3. Obtain the separate full match-q0 and fresh observation-q1 confirmations.
   The comparator in this campaign cannot substitute for them.
4. Only after the required confirmation gates, integrate the qualified match
   tuple into public capability checks, native transport, Stata returns and
   display, help, and focused tests. Keep fixedoffset explicit, preserve point
   defaults, do not silently drop stayers or switch q, and state the supported
   population and diagnostic limitations.

No confirmation, public match route, second-stage correction, push, tag,
release, or binary distribution occurred in this slice.
