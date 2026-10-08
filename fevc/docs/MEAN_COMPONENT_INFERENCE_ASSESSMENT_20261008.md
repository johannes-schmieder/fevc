# Mean-centered component inference: bounded assessment, October 8, 2026

The completed preregistered assessment retains **22 failed primary availability-screen entries across three exact-Mata cells**, in both Mean and fixed-c0 arms. Exact highrank k12/k20 and q1 k12 do not qualify for reliable inference on these fixtures. All eight native observation/match cells have 100% interval availability and pass the broad descriptive screens. The complete attempt inventory is engineering-valid, but the scientific failures and material numerical sensitivity below preclude a general coverage claim.

The union audit verifies 12 cells × 2,000 outcomes × two paired arms × four targets = 192,000 target rows, 48,000 calls and 960 completed task receipts, with no overlap, missing attempts, exclusions or retries. Exact runs use source `63757839`; native runs use `b9f80ce9`. The four intervening Windows runner/documentation changes have a recorded compatibility review; all statistical runtime and harness files are identical.

The designs, primary exclusions, thresholds and semantic RNG keys were frozen in the [sampling registration](mean_component_inference_validation_v1.json), its [pre-assessment amendment](mean_component_inference_validation_v1_amendment1.json), and the separate [numerical registration](mean_component_inference_numerical_v1.json). Platform qualification and payload adoption are tracked separately in [native provenance](../../native/mean-component-20261008/checkpoint.json); the new Mac/Linux payloads remain unadopted while Windows is blocked.

## Primary results

Ranges below cover the predeclared primary targets: all four for highrank, worker/firm/total for q1. Coverage counts missing intervals as not covered. Conditional-on-success coverage remains available in the machine-readable summaries.

| Route/reference/dimension | Mean interval availability | Fixed-c0 availability | Mean coverage, all attempts | Descriptive screens |
|---|---:|---:|---:|---|
| mata_highrank_k12_primary | 11.40% | 11.40% | 11.05%–11.10% | FAIL: availability |
| mata_highrank_k20_primary | 18.80% | 18.85% | 17.75%–17.95% | FAIL: availability |
| mata_q1_k12_primary | 32.40%–35.45% | 32.60%–35.50% | 31.45%–34.60% | FAIL: availability |
| mata_q1_k20_primary | 98.00%–98.15% | 98.00%–98.15% | 93.45%–94.45% | Pass, bounded scope |
| match_highrank_k12_primary | 100.00% | 100.00% | 93.60%–95.00% | Pass, bounded scope |
| match_highrank_k20_primary | 100.00% | 100.00% | 94.45%–95.35% | Pass, bounded scope |
| match_q1_k12_primary | 100.00% | 100.00% | 96.25%–97.10% | Pass, bounded scope |
| match_q1_k20_primary | 100.00% | 100.00% | 95.90%–97.15% | Pass, bounded scope |
| observation_highrank_k12_primary | 100.00% | 100.00% | 94.90%–95.75% | Pass, bounded scope |
| observation_highrank_k20_primary | 100.00% | 100.00% | 95.05%–95.85% | Pass, bounded scope |
| observation_q1_k12_primary | 100.00% | 100.00% | 95.05%–96.40% | Pass, bounded scope |
| observation_q1_k20_primary | 100.00% | 100.00% | 95.05%–97.15% | Pass, bounded scope |

The exact primary runs have 9,419 atomic failures and 38,090 unavailable target rows: 19,644 negative fitted-variance rows, 18,032 non-PSD covariance rows, and 414 target-local q1 status-5 rows. All 22 failed primary screen entries are availability screens. Exact highrank k12/k20 and q1 k12 do not qualify for reliable inference on these fixtures. Exact q1 k20 meets these broad screens. No cutoff or fixture was changed.

All native primary intervals were available. Mean highrank coverage ranges from 93.60% to 95.85%; q1 primary-target coverage ranges from 95.05% to 97.15%. The predeclared multi-mode q1 covariance diagnostic covers 91.75%–94.00% and remains outside the q1 primary screen. These finite designs and broad screens establish no universal coverage guarantee.

Mean minus fixed-c0 changes were small here. The largest all-attempt coverage loss was 0.15 percentage point for exact Mata and 0.05 point for either native route. Maximum Gaussian dense-oracle RMS Mean-versus-fixed-c0 point difference divided by actual-Mean SD was 0.000541 for observation/Mata and 0.001300 for match; the largest q1 conditional-remainder ratios were 0.000247 and 0.000748. These outcome-free oracle quantities do not include numerical JLA error.

## Availability diagnosis

The true Gaussian covariance in the independent worker/firm/covariance basis is positive definite in all four exact cells. Highrank raw condition numbers are approximately 15,907 and 38,113, reflecting a much smaller covariance-target scale; standardized correlation condition numbers are only 2.48 and 3.24. The q1 standardized condition numbers are 6.74 and 11.46. The primitive covariance gate precedes creation of the redundant total target, so the accounting identity is not the failed check.

Negative fitted-variance and non-PSD labels identify the observed gate, not its complete cause. The exact runtime fits primitive and polarized targets separately by binned local-linear smoothing, then constructs cross-covariances by polarization and subtracts simulated trace terms. These operations are plausible sources of finite-sample instability; the recorded labels alone do not isolate the offending fit or establish causality. No new production draws were run for this diagnosis.

## Diagnostic and numerical profiles

The ten predeclared stress cells used 100 paired outcomes each: 2,000 calls and 8,000 target rows, all engineering-valid. They retain 2,264 unavailable target rows. Exact homoskedastic, weak and null-q1 cells produced intervals in 14%, 28% and 0% of calls, respectively, in both arms. Native concentrated-mass highrank covariance was unavailable in all draws, while its other targets were available. Across these four stress cells, all 1,600 paired target attempts agree exactly on availability and failure/status code. This is shared behavior on these draws, not a general causal claim about centering. The frozen diagnostic aggregate label `SCREENS_PASS` only means that no primary screens apply; its scientific interpretation is descriptive.

The separate fixed-outcome profile has four outcomes × three numerical seeds × four budgets × two arms: 96 calls and 384 target rows, with all outcomes held byte-identical across numerical settings and no unavailable intervals. Relative to baseline point/Gram/covariance budgets 200/2048/1000, one budget at a time changed to 800/8192/4000. Point-budget changes reached 1.09 dense-oracle SD for the match highrank covariance point and 1.15 SD for its endpoint. q1 firm endpoints changed by up to 0.78 SD for match and 0.33 SD for observation. Gram-budget endpoint changes reached 0.166 SD and covariance-budget changes 0.015 SD. These fixed outcomes show material numerical sensitivity independently of the mean approximation; no sampling coverage claim follows.

## Interpretation and identities

- Bias and empirical SD use posted finite points only. Atomic failures can make this selected population highly unrepresentative. SE ratios use available highrank SEs and posted-point SD; q1 covariance summaries additionally condition on interval success. Attempts, posted-point counts, interval counts and SE availability are retained separately.
- Exact Gaussian dense-oracle SD describes an unconditional dense point estimator; it excludes finite-probe JLA variation. The t8 diagnostic Gaussian covariance reference omits the fourth-cumulant term `1.5 sum_i sigma_i^4 K_t,ii K_s,ii`. Estimated-offset moments are a known-offset reference, not an unconditional law after estimating gamma.
- The geometry records actual spectral shares. A q0 fixture name does not establish asymptotic diffuseness; the maximum registered highrank leading share is about 0.214. q1 primary leading shares are 0.914–0.959 with remainder shares no larger than 0.136.
- Primary, stress and numerical profiles were frozen separately before their outcomes. The older permissive pilot aggregate remains explicitly invalidated as scientific evidence and is preserved for timing/diagnosis.

The native sampling payload SHA-256 is `a2d599ce727f74d579c0cd315288413ac3980cb795258759537e560b00e89e5d`. It exactly matches the completed clean-source Mac qualification receipt at b9, whose SHA-256 is `211d2ea66ac522cded428ae8b4fc9989d8a0fade9cc80e2f88477d5195da4599`. Sampling occurred on arm64; Rosetta runtime qualification is a separate receipt claim.

The compact public evidence preserves original source and input identities; raw local runtime logs and row-level simulation outputs are outside this summary. Evidence records:
- [primary-union-audit.json](../../native/mean-component-20261008/evidence/scientific/primary-union-audit.json) — SHA-256 `44616a155ebf1f7a5e769242a7e52228c730296ecbe165eee023612cc7c179c4`.
- [primary-compact-summary.json](../../native/mean-component-20261008/evidence/scientific/primary-compact-summary.json) — SHA-256 `e85335d2314d73abfba974e557b3d2847a1420b426f37777d29605175884cb72`.
- [assessment-mata-63757839/assessment.json](../../native/mean-component-20261008/evidence/scientific/assessment-mata-63757839/assessment.json) — SHA-256 `322e9b5c23178906b2708487ecc9423c37f2eab953541588c1e01f68cf485525`.
- [assessment-native-b9f80ce9/assessment.json](../../native/mean-component-20261008/evidence/scientific/assessment-native-b9f80ce9/assessment.json) — SHA-256 `f025b481b681164417f50b3f0622402de0c93a1a95d4e6652f3508b11b88f32c`.
- [diagnostic-b9f80ce9/assessment.json](../../native/mean-component-20261008/evidence/scientific/diagnostic-b9f80ce9/assessment.json) — SHA-256 `6e0b806d3b0418ab4a335bbe670b30e97201ca48ea7575d8b237e97ca59dd50b`.
- [numerical-b9f80ce9/assessment.json](../../native/mean-component-20261008/evidence/scientific/numerical-b9f80ce9/assessment.json) — SHA-256 `438873c45f5d2221187455a1f828d2d34c91661bffd1024163cd8bfc5b2aafb9`.
