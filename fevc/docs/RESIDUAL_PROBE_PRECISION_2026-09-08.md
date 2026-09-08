# Direct residual probes: bounded 2,048-probe precision experiment

## Decision

The owner-approved experiment improves numerical stability substantially, but
does not pass every registered calibration screen. **Do not integrate this
candidate yet.** Production source, installed binaries, paper, point-estimation
budget (200 JLA probes), outcomes, eligibility rules and thresholds are unchanged.
This is development evidence, not confirmation or release qualification.

The prospective scope is [V2](residual_probe_precision_v2.json), using the
unchanged screens in [V1](residual_probe_validation_v1.json). The comparison
raises only the direct residual-moment Gram budget from 512 to 2,048. “Current”
below means the archived unified subtractive-Gram implementation, not the paper
or the newly tested direct-probe candidate.

## Veneto: the numerical stability screen now passes

All six fixed inference seeds (8675309–8675314), for both q0 and q1, return
all four intervals. Point estimates, corrections, numerical point MCSEs and
sample signatures are unchanged. The maximum fraction of fitted variances
using the existing floor falls from 6.829% to 0.234%.

| Across-seed statistic | 512 Gram probes | 2,048 Gram probes |
| --- | ---: | ---: |
| Worker SE coefficient of variation | 12.50% | 5.17% |
| Firm SE coefficient of variation | 11.82% | 4.55% |
| Covariance SE coefficient of variation | 17.76% | 6.47% |
| Total SE coefficient of variation | 2.29% | 1.04% |
| q1 firm interval-width coefficient of variation | 16.41% | 6.47% |
| q1 covariance interval-width coefficient of variation | 17.88% | 7.11% |

All SE/width coefficients of variation are below the registered 10% ceiling.
All endpoint ranges divided by median interval width are below 25%; the largest
is 12.18%. These are computational-seed checks on one dataset, not coverage
checks or evidence that the variance model is correct.

The first 12 Stata attempts failed with rc=498 because two remaining Stata
receipt checks still expected 512 Gram probes. Native calculations completed;
the failure was execution-count reconciliation. Those attempts remain archived.
A separate copy changes only these two count checks to 2,048 and uses the same
native plugin. One end-to-end smoke passed before the remaining eleven calls.
The corrected 12-call audit passes, including strict actual-budget checks.

## Saved outcomes: substantial improvement, three remaining screen failures

The eight registered primary cells each reuse 200 saved outcomes: 1,600 new
native calls and 6,400 target attempts. No new outcome draws were generated.
Every selected-reference interval returns, with no shared-fit failure or point,
sample, deletion-unit or point-MCSE regression. The two earlier arms are read
from their hash-verified archived results, not rerun.

| Firm target | Current coverage | Direct 512 | Direct 2,048 | 2,048 empirical SD / RMS SE |
| --- | ---: | ---: | ---: | ---: |
| Match q1, equal independent | 97.0% | 89.5% | 93.5% | 1.026 |
| Observation q0, diffuse with controls | 92.0% | 88.0% | 90.5% | 1.163 |
| Observation q1, dominant common | 93.0% | 91.0% | 91.5% | 1.120 |

The complete independent raw-output audit finds exactly three registered
failures:

1. Match-q1 equal-independent firm coverage falls 3.5 percentage points relative
   to current, exceeding the permitted 3-point drop. Its absolute coverage and
   SD/RMS-SE screens pass. At 200 replications, this is one covered draw short
   of the paired screen; it is not strong evidence by itself of poor absolute
   calibration. The predeclared screen nevertheless remains a failure.
2. Observation-q0 controls-firm SD/RMS-SE is 1.163, above 1.10. Its coverage
   screens now pass. Current already has SD/RMS-SE 1.150 in this cell, so this
   residual problem cannot simply be attributed to the direct representation.
3. Observation-q1 dominant-common firm SD/RMS-SE is 1.120, above 1.10.

An SD/RMS-SE ratio above one means that simulated point estimates vary more
than the reported standard errors suggest. It is distinct from SD divided by
the mean SE; both summaries are retained. Original two-sided flags are also
reported separately, without replacing the registered V1 rules.

Coverage counts every attempted outcome. Auxiliary Gaussian/high-rank SEs
are not always available for nonprimary q1 covariance targets even when the
selected q1 interval exists; positive-variance SE denominators are explicitly
reported rather than silently conditioning interval coverage on SE availability.
Eligibility exclusions remain exactly as registered.

## Exact checks and harness validation

Two Rust tests pass: the independent same-probe sample-covariance identity and
a new exact small-matrix comparison. The latter uses 96-row fixtures, with and
without controls, each with three variance terms. For all twelve fixture/seed
pairs, scaled Frobenius error relative to the independently constructed exact
Gram decreases at 2,048 probes. Errors span 0.0885–0.2183 at 512 and
0.0388–0.0962 at 2,048. Projection checks pass at tolerance 1e-9.
This supports the precision mechanism but is **not** an exact-Gram test on the
actual failing simulation designs or the 15-term Veneto basis.

Before the main replay, five reporting regressions and the full eight-call
tiny pipeline pass; three malformed CLI calls fail as intended. Corrupt,
missing, duplicate, wrong-seed and wrong-budget results are rejected. The
main reporting code and source are frozen before execution. An initial separate
audit incorrectly tried to average negative auxiliary q0 variances for q1
covariance; its failed log is retained. The auditor was corrected to use the
explicit positive-variance SE denominator while retaining every interval
attempt. A sixth focused regression checks this distinction. The final raw
audit passes and agrees with the main report's scientific SCREEN_FAIL.

Commands, versions, seeds and full outputs are preserved locally. Key commands
(with RUN denoting the directory below) were:

```text
cargo test --release --locked --offline --manifest-path RUN/source/rust/Cargo.toml -p vckss-core --lib direct_residual -- --nocapture
./.venv/bin/python .local/diagnostics/residual-probe-2048-20260908/audit_seeds.py
```

Builds used pinned Rust 1.85.1, isolated offline release targets, and local
Stata/MP. No broad platform qualification or public release gate was run.

## Evidence identities

Local evidence root: `.local/diagnostics/residual-probe-2048-20260908`.
The native manifest, build receipt and staged Stata manifests enumerate source,
adapter, executable, plugin, input and harness hashes; execution receipts bind
individual calls and their exact commands. SHA-256 identities:

| Artifact relative to evidence root | SHA-256 |
| --- | --- |
| `source_manifest.json` | `4d2ac43fe5f07167d19905e292c4b1bd0ad6d4901a6f3725d37942e4490be1d6` |
| `candidate-bin/receipt.json` | `53b927f044b58a8e63f9773c683691b6cc231332a58e47f4ab4cdb5a77d7ade6` |
| `main/manifest.json` | `2bb747249555e5a63bf5b773a36f253bb35c1ef08d58abe16a14810fbc73ab95` |
| `main/result.json` | `9413c0b627f8207ef0fd692358ac220ea4352beb826008420f5e27f31e298e77` |
| `independent_audit.json` | `93d2f8b86b8ac7668080fedeaac91660e6713ed55338bdf42ca16b623c245563` |
| `seed_audit.json` | `d5cc361b2ce6e37b0624725e40014010c3287238e6fb631777af5bbf081af34c` |
| `stata-count-fix/smoke_manifest.json` | `255088ef0dfcf4633371c2783d8d64b16a43a506a216e488721630100e8f2d46` |
| `stata-count-fix/rest_manifest.json` | `a5c51f200e22e2ccee0210d83070a69351a73c09346622b3e47871e3b14c6b01` |

## Recommended next decision

The experiment identifies probe precision as a material contributor to the
earlier instability. It does not establish that more probes resolve all
calibration problems. Before another budget increase or fitter change, compare
the approximate Gram with an exact Gram on the three flagged simulation
designs, then replay their existing outcomes with only that Gram substituted.
This would separate remaining Gram approximation error from downstream
variance fitting or interval approximation. It requires a newly bounded
diagnostic specification; it was not performed or automatically authorized
by this experiment. Keep the paper and production implementation unchanged
pending that decision.
