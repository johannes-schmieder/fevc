# Repaired match q1 independent confirmation — 2026-09-04

## Decision

**PASS at exact source `4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`.**
The registered independent confirmation completed all 700 tasks and 140,000
target attempts. Every frozen scientific gate passed without changes to the
estimator, fixtures, target exclusions, seeds, or acceptance thresholds.
All 702 scheduler records reconcile with clean exits. Local reaggregation
matches the scheduled aggregate and summary CSV byte for byte.

The 18 primary correct-model q1 rows (six designs times worker, firm, total)
have 44,995 computed intervals out of 45,000 attempts. Availability is
99.96--100%, coverage among computed intervals is **94.24--96.76%**, and
empirical-SD/mean-SE ratios are **0.9675--1.0297**. Every registered bias gate
passes. This is fresh confirmation for the corrected q1 calculation, not a
reinterpretation of the failed historical campaign.

The confirmation does **not** qualify inference with estimated controls:
that prospectively excluded diagnostic again has total coverage only
77.551% among 2,499 computed intervals, versus nominal 95%, and SE ratio
1.6728. The independent exact covariance diagnosis explains why simply
having two controls is insufficient; see
[`FIXED_OFFSET_DIAGNOSIS_2026-09-04.md`](FIXED_OFFSET_DIAGNOSIS_2026-09-04.md).
No second-stage correction or public match route was added.

The complete 56 summary rows, every failure group and replication inventory,
frozen cutoffs, runtime/source identities, numerical maxima, and audited
scheduler records are in
[`inference_repair_match_confirmation_v1_result.json`](inference_repair_match_confirmation_v1_result.json).

## Frozen campaign

The campaign follows
[`inference_repair_match_campaign_v1.json`](inference_repair_match_campaign_v1.json),
registered before the repaired development run. Its SHA-256 is
`9d32eabd8f07152f0b6bb64552ee33c2dd31ba6b355e329e3bd92bc4e1f469d8`.

- Fourteen original designs, `k=20`, 400 independent declared matches,
  2,500 replications per design, four target attempts per replication.
- Confirmation master seed `0xb73490d26a18ce55`, separate from development
  `0x4d617463685131a1`; fixed outcome-free fold master
  `0x862ad51c3f07b994`. Semantic cell/dimension/replication keys are unchanged.
- Settings: 256 estimator, 512 covariance, 128 spectrum probes, 256 spectral
  iterations, and 4,000 production critical draws. Scientific intervals use
  the previously qualified independent deterministic quadrature reference;
  production Counter critical values remain a separately validated boundary.
- Fresh outcome-free preflight at 4,096 spectrum probes was completed on
  clean source before freezing and deploying the confirmation manifest.
- Each array task owns 50 replications and unique atomic result/receipt
  files. Aggregation is a scheduler dependency, not a monitoring action.
- Primary q1 targets exclude the multi-mode covariance target, controls,
  weak/null, and deliberately multi-mode designs. Excluded attempts remain
  in the complete output and failure accounting.

The gates remain 0.98 correct-model availability, coverage within
`max(0.03, 3*coverage_MCSE)` of 0.95, SE ratio in `[0.82, 1.18]`, and bias
within four MCSEs. The original mild/severe-model and CMG availability gates
are unchanged. No result inspection was used to weaken them.

## Complete success and failure accounting

| Attempt status | Count | Interpretation |
| --- | ---: | --- |
| Computed interval | 119,564 | Includes all eligible and diagnostic targets |
| Target-specific unavailable | 208 | Other computed targets are retained |
| Shared backend/statistical failure | 20,228 | Counts against all affected target attempts |
| Total | 140,000 | Complete frozen inventory |

The 208 target-specific withholdings consist of 202 nonpositive covariance-
target remainder variances and six uncertified modes. Nonpositive remainder
counts are 63 in equal-mass diagonal, 71 in equal-mass CMG, and 68 in the
leverage-model cell. All of these covariance targets were excluded before
the run. The six mode withholdings are explicitly enumerated in the JSON:
one each in equal-mass total, serial firm, controls total, CMG covariance,
and severe-model covariance and total. No alternative q or covariance
regularization was silently substituted.

Shared failures comprise 20,000 weak/null target attempts (every interval
withheld through the registered full-covariance PSD check) and 228 aggregate-
variance positivity failures: 55 severe-model replications, one mild-model
replication, and one unequal-independent replication, each affecting four
targets. There were no scheduler, schema, missing-output, mixed-source, or
point-invariance failures.

The diffuse q0 comparator passes all four gates, with availability 100% and
coverage 94.44--94.80%. This is only the single comparator design, not full
match q0 confirmation. CMG worker/firm/total availability is 100%; its
coverage is descriptive under the original registration. Mild-model target
coverage is 94.48--95.44%. The severe model visibly miscalibrates as intended:
firm coverage 87.44%, total 84.00%, and total SE ratio 1.4064. The campaign
therefore confirms its specified limitation as well as its supported cases.

Maximum observed residual/identity errors:

| Diagnostic | Maximum |
| --- | ---: |
| Complete solve relative residual | `9.998e-12` |
| Direct q1 remainder identity | `4.077e-13` |
| q1 point decomposition identity | `1.666e-16` |
| Point correction identity | `1.735e-18` |

## Scheduler, inventory, and immutable evidence

Private SCC run:
`/projectnb/welfgr/vckss/runs/20260905T005500Z-repair-confirmation-4a68ea2`.
Collected/revalidated locally under
`/private/tmp/fevc-repair-4a68ea2-confirmation`.

| Stage | Job | Records | Slots each | Wall time | Peak memory |
| --- | ---: | ---: | ---: | --- | --- |
| Build | 7462270 | 1 | 1 | 38 seconds | 2.539 GB |
| Tasks | 7462271 | 700 | 1 | 11--81 seconds each | 111.324 MB maximum |
| Aggregate | 7462273 | 1 | 1 | 35 seconds | 1.985 GB |

Every accounting row records owner `johannes`, project `welfgr`, `failed=0`,
and `exit_status=0`. The tasks consumed 21,663.293 CPU seconds in total.
The complete host and queue inventory is retained in the JSON audit. Queue
placement was left to the scheduler; no fixed architecture or host was
requested. Rust and Cargo are pinned to 1.85.1; SCC Python is 3.13.8. The
private run contains no restricted data or licensed Stata execution.

Both collections succeeded without retries or metadata errors; destination
directories were created locally before transfer. All task file hashes,
receipts, source/manifest/binary identities, exact unique task keys, target
counts, final logs, and scheduler records were verified. The local ordinary
runner independently reaggregated the 140,000 rows; both output files match
the scheduled files byte for byte. Raw accounting and the local audit were
archived under the private run's `qacct/` and `receipts/local-audit.json`,
then their remote hashes were rechecked.

| Artifact | SHA-256 |
| --- | --- |
| Immutable source bundle | `b2ce0f60f1c656ae5a41358a8ed273b601096856ba322f733d22d9139f01cca7` |
| Frozen task manifest | `ac98250fb9e3687294f5d7d53c92bab0d58aa5fc4e215fb6a4f6bd5a6ce222d9` |
| SCC example binary | `89d069b3b02e3712b3d201652da71c7ad10ad6e549f9a6b6ebb58d573a10e6e0` |
| Aggregate JSONL | `047842c5b24134b10de1704c8f195bb891648081c8bea84958557f9cb687cb29` |
| Summary CSV | `cac921245dbcb2637f4d105a129488a4f86bf979c7e290be7b6c56306afae58f` |
| Scheduled receipt | `d49e25c5e07197deeb6dcb10543dd430bc0dd8756575416619bd27cfb0c0fada` |
| Local audit | `4e3b6ab635c87a7613c6f5d391f157ca03663a37d4521827582f419983e0c1d5` |

## Compatibility and proportional gates

The native/tiny/SCC-smoke qualification source is
`31dd37f2954c02d223ad81175dd4ded7b5840b8d`. The confirmation source is
`4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`. Their changed paths are exactly:

```
.ci/stata/results/31dd37f2954c02d223ad81175dd4ded7b5840b8d.json
fevc/PLAN.md
fevc/docs/INFERENCE_REPAIR_CHECKPOINT_2026-09-04.md
fevc/docs/INFERENCE_REPAIR_DEVELOPMENT_RESULT_2026-09-04.md
fevc/docs/README.md
fevc/docs/inference_repair_match_campaign_v1_result.json
rust/qualification/evidence/INFERENCE-REPAIR-MACOS/31dd37f2954c02d223ad81175dd4ded7b5840b8d/SANITIZED_EVIDENCE.txt
rust/qualification/evidence/INFERENCE-REPAIR-MACOS/31dd37f2954c02d223ad81175dd4ded7b5840b8d/SHA256SUMS
rust/qualification/evidence/INFERENCE-REPAIR-MACOS/31dd37f2954c02d223ad81175dd4ded7b5840b8d/plugin-qualification.txt
rust/qualification/evidence/INFERENCE-REPAIR-MACOS/31dd37f2954c02d223ad81175dd4ded7b5840b8d/source-manifest.sha256
rust/qualification/evidence/INFERENCE-REPAIR-MACOS/README.md
```

These are evidence and documentation only. No production, build, wrapper,
ABI, binary source, input generator, acceptance policy, or registered harness
file changed. All entries in the exact native source manifest were rehashed
successfully; its own SHA-256 is
`d5f9d246537c6b983507cd3dd1bf89bae2c11d629e9b616d63bef4a49cb33e5e`.
The registration's ten frozen harness files also validate unchanged.
Accordingly the passed exact-source macOS/Rosetta native profile, complete
tiny/adversarial pipeline, and representative real-entrypoint SCC smoke are
carried forward from the recorded checkpoint. A documentation-only SHA did
not trigger another platform matrix or duplicate scientific smoke.

The new source `506c170e7621ccc0b20f510d634c706f13cfcc7d` adds only the
independent Python controls diagnostic, its tests and registration, plus
PLAN/index updates. It changes none of the confirmation or production
surfaces. Final source tests on that diagnostic pass: 598 pytest tests and
the CMG assembler check. Both gates were repeated after evidence recording:
598 tests passed in 51.70 seconds and assembly remained current. The
evidence-recording working changes add only
this report, the controls report, their machine-readable result records,
and PLAN/index updates. The scientific claim is always tied to the original
frozen confirmation source; these ancillary Python/docs changes do not claim
to requalify a different native artifact. No expensive new native, Windows,
or broader SCC campaign was needed for this unaffected surface.

Key commands were the clean confirmation `run-preflight` and
`create-manifest` modes of `fevc/tools/run_inference_repair_campaign.py`,
the unchanged SCC deploy/submit scripts, and:

```bash
./.venv/bin/python fevc/tools/run_inference_repair_campaign.py aggregate /private/tmp/fevc-repair-4a68ea2-confirmation/collected/campaign-manifest.json /private/tmp/fevc-repair-4a68ea2-confirmation/collected/output/tasks /private/tmp/fevc-repair-4a68ea2-confirmation/revalidated --build-receipt /private/tmp/fevc-repair-4a68ea2-confirmation/collected/receipts/build.json
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

## Next steps and qualification boundary

1. Register the paired known/estimated-offset by known/fitted-variance
   controls experiment, informed by the exact diagnosis. Retain the
   no-second-stage-correction choice. Do not call the estimated-offset
   approximation calibrated on the strength of the no-control result.
2. Complete full match q0 independent confirmation and fresh observation q1
   confirmation on the corrected source. The single q0 comparator and old
   observation q1 receipts do not replace these.
3. Only then incorporate explicit match q0/q1 options, diagnostics, target-
   specific availability, documentation, and affected interface/native tests
   into the project. Preserve fixed-offset and mover-population boundaries;
   do not silently drop stayers, switch q, or choose a variance model.

A more stable deflated-trace spectrum diagnostic is a separate possible
follow-up, not a post-result adjustment to this accepted confirmation.
Public tagging, release archives, pushes, and binary distribution remain
separate owner decisions. Nothing was pushed or released in this work.
