# Fixed-offset collapsed-match q=1 development result

## Decision and scope

The first registered grouped match-deletion `q=1` development campaign
**fails** its frozen scientific gates. Its exact execution source is
`c4e9f362ed0c9cbea7ea083e21b0cc40fae80198`; the immutable registration is
[`match_inference_q1_campaign_v1.json`](match_inference_q1_campaign_v1.json),
SHA-256
`c12047cd6c40702f2d86b18c1ee18cdc118014a8286502f655b88e124c046127`.
The machine-readable result is
[`match_inference_q1_campaign_v1_result.json`](match_inference_q1_campaign_v1_result.json).

All 280 SCC tasks and all 22,400 expected target attempts completed and
reconciled. The failure is scientific rather than operational: the equal-mass
structured-common and leverage-only cells missed the registered 0.98 success
rate, and the equal-mass structured-common worker interval covered 0.982 among
successful fits, above its frozen 0.98 upper tolerance. No threshold, fixture,
seed, target, eligibility rule, or source was changed after observing results.
No task was retried, excluded, relabeled, repaired, or resubmitted.

This result does not invalidate the accepted grouped `q=0` development
foundation or the independent local grouped `q=1` algebra and oracle tests.
It does block progression to a `q=1` confirmation, public match-inference
option, or larger experiment. The next slice is a bounded diagnosis of the
recorded equal-mass failures, not confirmation or tuning.

The inferential interpretation remains narrow:

> Match-cluster inference conditional on the full-sample fixed nuisance-control
> offset, allowing unrestricted within-match dependence and using a structured
> model for match-aggregate variances.

Different declared matches are independent, including different matches for
the same worker. The procedure conditions away uncertainty in the estimated
nuisance-control coefficients. It is neither joint-nuisance nor unrestricted-
KSS inference.

## Frozen source and preflight

Campaign tooling was registered at
`0694e4c3fda624f289e30ef9da7842f1363bd844`. Execution source `c4e9f36`
differs from that source in only:

```text
fevc/CHANGELOG.md
fevc/PLAN.md
fevc/docs/MATCH_INFERENCE_Q1_CAMPAIGN_SMOKE_2026-09-04.md
fevc/docs/README.md
```

Those changes record the accepted prerequisite smokes and update active
documentation. They do not change the estimator, grouped covariance, raw
`q=1` recenter, critical radius, ellipse image, RNG, solver, build system,
plugin ABI, input DGPs, target eligibility, or acceptance thresholds. Every
registration-bound campaign file retained its registered SHA-256. The
development manifest nevertheless binds the clean execution source directly;
no result was carried across source identities.

The exact source identity is:

```text
source commit       c4e9f362ed0c9cbea7ea083e21b0cc40fae80198
worktree diff       e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
source manifest     a43ce65ae0a924bd3e6c7b5bdff7097e5430a4d4e9005d1763258e4a6b2a12f1
source archive      d7283fa143d6084aac00b86967a40f7261ca28a52262a6370369baddd3390fae
manifest            9b4bb1d88917f3c9e0d173ca1c38d27e3cb32570800c1c9beef265d543f6323f
macOS preflight bin 2fd54af00e61cb5ca51a452d76abf643070f37ed57d1f2c568a4a1a0f1e2dbb7
SCC Linux binary    57fe2471b10e021b09122190da1fdde6f80a6e9eb003f3a01cb6630ac390b5d8
```

The development preflight ran before manifest creation with 4,096 spectral
trace probes and passed all 56 outcome-free target checks. In the primary
equal-mass design, worker, firm, and total leading/remainder shares were
`0.9152/0.0559`, `0.9515/0.1010`, and `0.9800/0.0738`. The covariance target
was deliberately multi-mode at `0.5000/1.0000` and remained coverage-
ineligible. Deliberately multi-mode remainder shares were `0.6120`--`0.8403`.
The diffuse `q=0` comparator's leading shares were `0.0292`--`0.1700`.

The preflight payload and receipt hashes are:

```text
preflight payload  045a7d3caf845d94dd92ddf47981934a4fa3467671bd1544f451b049fe6c0dfc
preflight receipt  1718f40760eeec68ccaf47e8a3d041b10ab68db5d6aff54db7741d9b949d6d3b
```

These high-resolution checks certify the frozen fixture labels. They are not
an automatic `q` selector and do not guarantee that every outcome-specific
covariance calculation succeeds.

## Exact-source SCC execution

The registered development profile has 14 cells, 400 replications per cell,
four targets, and 20 nonoverlapping replications per shard. Its 280 unique
tasks therefore contain exactly 22,400 target attempts. The exact source was
deployed read-only under:

```text
/projectnb/welfgr/vckss/runs/20260904T190202Z-q1-development-c4e9f36
```

The real dependency launcher submitted:

| Stage | SGE job | Tasks | Slots | Limit | Result |
|---|---:|---:|---:|---:|---|
| exact-source build | 7445367 | 1 | 1 | 30 min | process pass |
| development array | 7445368 | 280 | 1 each | 30 min | all processes pass |
| aggregate validator | 7445369 | 1 | 1 | 15 min | process pass; scientific receipt `FAIL` |

The SCC build used Rust/Cargo 1.85.1 and Python 3.13.8. The build receipt hash
is `f42fa656f125a6289dd94a4c5ad0f77c53912f2194c4a4422c28fd59b1d54aa7`.
The scheduler ledger hash is
`11360ef07b9a547b688699676e59735c927d5f63ecbc85b8708a0c32c99a1c35`.

Every array record has a unique task ID 1--280, uses one slot, and reports
`failed=0` and `exit_status=0`. Task wall times were 5--33 seconds; maximum
reported virtual memory was 110.738 MiB across 46 hosts and 15 queues. The
build used 36 seconds and 1,701.888 MiB. Aggregation used 8 seconds and
322.574 MiB. The frozen `qacct` hashes are:

```text
build       aaec6f839a0ecec2dd92f46979ed060794234576bfd99f360b264583cb8a14dd
tasks       a129b8cae0e7f68622848efb96eeb7be8646ee1929311b6fe680053c65f41386
aggregate   26de697ab0110eccf6b41b9847b0ea84736970d0b8c48ca0f41fc098b28850b7
```

Scheduler success means that the complete evidence was produced. It does not
override the aggregate receipt's separate scientific `FAIL` decision.

## Inventory and immutable artifacts

The run contains exactly 280 task payloads, 280 task receipts, and 280 task
logs. All logs have the task PASS marker and none has a traceback, panic, or
error marker. Stable SHA-256 tree hashes over sorted per-file hashes and names
are:

```text
task payloads   8d58de8ad54895b4b8b6193795c36bbf5dfa798db174392f39a2ad4a233a8139
task receipts   d783cd171aa439e3d71a22d0925f5830a1b334ac3ec1f1ee418bc2c704bfd8ad
task logs       98d7c65f895a5d03d3b422f7a42e39b0f2d7aa7b93fc983fe1e746561b4152ea
```

An independent local audit recomputed every task payload hash against both its
task receipt and the aggregate receipt, reconstructed every task's registered
cell/replication/target keys, and compared the task-row multiset with the
aggregate. Both contain 22,400 unique rows and have canonical multiset hash
`a0e4f476678394aed6813326c47d4f79d0d39f04a536a1406ea086567c8356e5`.
There are no missing, extra, or duplicate keys. The 5,600 cell-replication
keys have 5,600 unique semantic seeds, with the same seed shared by their four
targets as registered. All 280 receipts match the exact source, manifest,
Linux binary, preflight, task bounds, output hash, and row count.

The final aggregate artifacts are:

```text
aggregate payload  e97875dc7da1c4faf14f35a8d218b6fe50c2f27c3594bf83883b6e39e987e679
summaries           8caeebb5f05fd01d5363623c09d37715f212e2409dc91b547f4276b2c9f2f514
aggregate receipt  1312ff1e5acc864d1fe31dfd65ee7df3ebe0bf4f94736d7137804ff3de22fd35
```

There were 19,048 successful target attempts and 3,352 typed failures. Weak
and null cells account for 3,200 of those failures and intentionally have no
coverage gate. The remaining 152 target failures are retained in the
machine-readable result.

## Frozen scientific decision

The aggregate receipt reports nine failures:

```text
one_mode_equal_independent/worker: success rate
one_mode_equal_independent/worker: coverage
one_mode_equal_independent/firm: success rate
one_mode_equal_independent/covariance: diagnostic success rate
one_mode_equal_independent/total: success rate
one_mode_leverage_sensitivity/worker: success rate
one_mode_leverage_sensitivity/firm: success rate
one_mode_leverage_sensitivity/covariance: diagnostic success rate
one_mode_leverage_sensitivity/total: success rate
```

The primary equal-mass structured-common cell succeeded in 388 of 400
replications per target (`0.9700`), below the registered `0.98` minimum. All
12 failed replications withheld all four targets atomically with
`JLA_CONSTRAINT_FAILED:component_inference_q1`. Success-conditioned worker,
firm, and total coverage was `0.9820`, `0.9691`, and `0.9742`; their
empirical-to-estimated SE ratios were `0.9534`, `0.9672`, and `0.9883`.
The worker coverage estimate is above the frozen upper bound of `0.98` because
its MCSE-based tolerance is smaller than the absolute `0.03` tolerance.

The equal-mass leverage-only sensitivity cell succeeded in 389 of 400
replications per target (`0.9725`), also below the registered success gate.
Its 11 failed replications likewise withheld every target at the typed `q=1`
phase. Success-conditioned worker, firm, and total coverage was `0.9769`,
`0.9692`, and `0.9640`; SE ratios were `0.9376`, `0.9835`, and `1.0009`.

As a diagnostic only, the numbers of successful covered intervals divided by
all 400 attempts are `0.9525/0.9400/0.9450` for the structured-common worker,
firm, and total targets and `0.9500/0.9425/0.9375` for the leverage-only
targets. Those quantities count every withheld result as not covered. They are
not the registered coverage estimator and cannot rescue either failed
success-rate gate. They instead show why outcome-dependent `q=1` withholding
and the composition of the successful subset must be diagnosed together.

The other four primary correct-model one-mode cells completed all attempts and
passed their frozen gates:

- highly unequal mass with independent physical-row errors;
- highly unequal mass with common match shocks;
- highly unequal mass with serial within-match correlation; and
- different within-match covariance shapes with the same aggregate variance.

Across all 18 eligible correct-model rows, success ranged from `0.9700` to
`1.0000`, success-conditioned coverage from `0.9375` to `0.9820`, and SE ratios
from `0.9376` to `1.0558`. The largest absolute bias was 2.58 Monte Carlo
standard errors, inside the frozen four-MCSE gate. These aggregate ranges do
not make the failed rows pass.

The CMG solver diagnostic succeeded in 392 of 400 replications per target,
exactly meeting its `0.98` diagnostic gate; the eight failures were typed and
atomic. Its descriptive target coverage was `0.9617`--`0.9770`, with SE ratios
`0.9740`--`1.0277`. The diffuse `q=0` comparator completed all 1,600 target
attempts and passed, with coverage `0.9400`--`0.9675` and SE ratios
`0.9158`--`1.0666`.

Mild omission completed every attempt and passed: eligible coverage was
`0.9500`--`0.9625` and SE ratios were `0.9864`--`1.0124`. Severe omission was
visibly invalid as required. Firm and total coverage fell to `0.8499` and
`0.8448`, with SE ratios `1.2746` and `1.3944`. This confirms the limitation
of the structured aggregate-match variance assumption; it does not provide
robustness to an omitted variance driver.

Every weak- and null-signal attempt was withheld. Per target, weak signal
produced 195 typed PSD and 205 typed `q=1` failures; null signal produced 88
PSD and 312 `q=1` failures. No regularization, `q=0` substitution, target
dropping, or success-conditioned concealment occurred.

The deliberately multi-mode cell completed every attempt but remains outside
the coverage claim. Mean worker, firm, covariance, and total remainder shares
were `0.8331`, `0.8268`, `0.6426`, and `0.8382`. The covariance target in the
one-mode fixtures likewise retained mean remainder shares of about
`0.934`--`0.949` and remained coverage-ineligible.

All varying-control fixed-offset attempts succeeded and explicitly marked
nuisance uncertainty as conditioned away. That cell is coverage-ineligible.
Its descriptive repeated-estimation coverage was `0.8750`, `0.8975`, `0.8875`,
and `0.7775` for worker, firm, covariance, and total. These values combine
variation in `gamma_hat` that the conditional method deliberately omits and
are evidence of the limitation, not a conditional coverage result.

## Numerical diagnostics

Every successful result reports 400 independent matches. Effective match
count ranged from `342.34` to `400`, and largest match-mass share from `0.0025`
to `0.00477`. Largest match leverage was `0.1190`--`0.2815`; the smallest
delete-match maker denominator was `0.7198`--`0.8816`; maximum influence share
was `0.00856`--`0.71347`. Across successful `q=1` attempts, maximum leading-
mode match weight was `0.02635`--`0.09500` and remainder-influence
concentration was `0.00824`--`0.40056`.

The largest complete-system residual was `9.89e-12` against a `1e-10`
tolerance. Maximum trace and remainder-trace MCSEs were `2.68e-6` and
`1.98e-6`. Maximum whole-match point-correction, `q=1` point-decomposition,
and direct remainder identity errors were `1.73e-18`, `1.67e-16`, and
`4.43e-13`. Maximum leading and second mode residuals were `6.13e-5` and
`2.53e-4`, below the registered `0.02` spectral limit. No successful
covariance required PSD cleanup; successful smallest covariance eigenvalues
were `1.97e-8`--`1.08e-4`.

Variance-floor share ranged from zero to `0.43`, boundary share from `0.0075`
to `0.0375`, and fitted-design reciprocal condition number from `1.44e-4` to
`0.9998`. All successful `q=1` rows used exactly 4,000 Counter-V1 critical
draws; the `q=0` comparator used none. Every successful row reports that
nuisance uncertainty is conditioned away.

These diagnostics show that successful calculations preserve the registered
algebraic and numerical contracts. They do not override the atomic `q=1`
failures or failed coverage gate.

## Validation record

The execution source first rebuilt the registered release example locally and
ran the development preflight and manifest creator. The exact source archive
was then deployed and submitted through the package-owned SCC build/task/
aggregate dependency launcher. The documentation/evidence result passed:

```text
./.venv/bin/python -m pytest -q fevc/tests/python/test_match_inference_q1_campaign.py
    20 passed in 9.07s
./.venv/bin/python -m pytest -q
    562 passed in 47.67s
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
    passed
bash -n rust/stata_backend/scc/deploy_match_inference_q1_campaign.sh \
  rust/stata_backend/scc/run_match_inference_q1_build.sge \
  rust/stata_backend/scc/run_match_inference_q1_task.sge \
  rust/stata_backend/scc/run_match_inference_q1_aggregate.sge \
  rust/stata_backend/scc/submit_match_inference_q1_campaign.sh
    passed
jq empty fevc/docs/match_inference_q1_campaign_v1_result.json
    passed
independent local manifest/receipt/task/aggregate/qacct reconciliation
    passed
git diff --check
    passed
```

Both pytest runs emitted only the known permission-protected temporary-cleanup
warnings from deliberately rejected symlink fixtures; no test failed.

Rust, licensed Stata, and plugin qualification are not repeated for this
result-recording commit. The result changes documentation and immutable
evidence only. The exact campaign source already passed the pinned Rust gates,
and the campaign execution itself built and ran that source on SCC Linux.
Neither the campaign nor this record changes the grouped implementation,
public Stata surface, plugin ABI, build system, RNG, solver, or production
inference path.

## Boundary and next slice

The failed development result is immutable evidence. It cannot be converted
to a pass by changing the frozen success threshold, using all-attempt covered
shares as coverage, dropping failed replications, removing the worker target,
or relabeling equal-mass designs after inspection.

Do not launch confirmation or a larger Monte Carlo. In a separate bounded
diagnostic slice, start from the recorded equal-mass failure replication keys
and determine why their joint `q=1` covariance is withheld. Independently
check the reference distribution and the relationship between outcome-
dependent withholding and worker success-conditioned coverage. Any change to
the estimator, covariance, failure semantics, generator, or scientific claim
requires a new prospective registration and fresh development evidence.

Grouped match inference remains internal. No parser option, public route,
automatic `q` selection, eligible-stayer support, cross-match dependence,
joint-nuisance inference, tag, release, push, publication, or binary
distribution is authorized.
