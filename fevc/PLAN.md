# Direct residual-probe inference and paper completion

## Completed owner scope, 2026-09-08

The [approved completion plan](../finalize_fevc.md) is implemented and checked.
The public structured Rust route uses the direct residual Gram with 2,048
probes by default and a separate `inferencegramprobes()` option; point probes
remain 200. V4 augmentation, legacy compatibility, count/state validation,
local native/Stata installation and independent numerical checks pass.

The 70-call replay accounts for 277 computed and three unavailable target
intervals out of 280; no shared failures occur. All 13 Veneto calls and all
27 scaling calls complete. The updated companion paper and every PDF page
have been checked. This is the agreed approximate-inference stopping point,
with the known calibration failures and specification caveats preserved.

See the [completion report](docs/INFERENCE_COMPLETION_2026-09-08.md) and
[source/evidence result](docs/inference_completion_v1_result.json).
The original [registration](docs/inference_completion_v1.json) remains frozen.
The current checkpoint is source-local macOS, not a public or all-platform
release. New Windows/Linux binaries, full private payload, human review,
published-text citation checks, commits, release and submission remain outside
this completed scope. No further coverage campaign is required by this plan.

## Previous source checkpoints

# Unified residual-moment inference and paper finalization

## Active owner decision, 2026-09-07

Implement [the saved finalization plan](../finalize_fevc.md): use one
residual-moment fitter for observation and fixed-offset match deletion,
retain the 200-probe default and individual/joint reporting safeguards,
complete bounded saved-draw validation and local native/Stata checks, then
update the companion paper and its runnable examples and local scaling.
The prospective scope is registered in
[`unified_residual_moments_v1.json`](docs/unified_residual_moments_v1.json).

This owner decision supersedes the earlier unchanged-match-fitter and
full-confirmation-before-paper milestones for this explicitly approximate,
model-based deliverable. It does not turn any historical FAIL into PASS,
transfer old match coverage qualification to a new fitter, or authorize
release, commits, joint-control match inference or new outcome draws.
The common fitter and V3/V5 native/Stata route are implemented. The current
local checkpoint passes 780 Python tests, 522 Rust workspace tests (one
ignored), the standalone backend tests, strict Clippy and ABI-header
compilation. Independent dense tests cover weighted physical-row versus
collapsed-match residual moments and the fixed-offset limitation.

The public individual-inference and match-interface Stata tests pass with
the newly built plugin, including q0/q1, frequency-copy equivalence, unchanged
match point estimates, batch/order and diagonal/CMG comparisons, unavailable
joint covariance, and malformed receipts. Two boundary omissions were fixed:
Stata now counts the match fitter's 512 Gram solves, and the C bridge checks
match ordering code 3 instead of requiring observation ordering code 2.
Neither fix changes point or interval arithmetic.

Source-local macOS arm64, Rosetta and universal qualification passes as
`LOCAL_CHECKPOINT_DIRTY_TREE`, including isolated installation. The qualifier
updated the three ignored repository-local Mac plugin artifacts. Its initial sandboxed fetch
failed DNS resolution and was retried with approved network access. The
integrated `run_checks.py` passed its source/Python/CMG checks but stopped
when the Stata quick suite loaded the older repository-local plugin. The rerun
with the matching locally qualified binaries passes completely: quick/full
Stata suites, clean installation, benchmark smokes and the final
`FEVC LOCAL QUALIFICATION PASS` marker. The final
help-return cleanup after qualification changes documentation only, not
production/build inputs, native artifacts or runnable Stata examples.
The local receipt, source manifest, tested binaries and verification logs are
preserved under
`.local/diagnostics/unified-residual-moments-20260907/local-engineering-1`.
Earlier focused diagnostic logs are under
`/private/tmp/fevc-unified-smoke.0M5EnY` and
`/private/tmp/fevc-unified-match.fKr4Nt`.

The saved-input audit passes all 192 task receipts, 48 cells and 19,200 calls,
including original hashes, semantic seeds, schemas and target truths. It
generates no new outcomes and is not the new-fitter comparison.

The new saved-draw harness is under
`rust/experiments/unified_residual_moments/`. Its 34 adversarial tests pass.
The complete source-bound tiny pipeline passes all 192 native calls, with
no shared fit failures or point regressions, 12 exact split/reversed replays,
single-draw captures and deliberate malformed-CLI failures. Build inputs
include the CMG `.rs.in` template; the historical generators are unchanged.

The full comparison completed locally with eight single-threaded calls and
alternating arm order: **BOUNDED_READINESS_PASS**, independently audited.
All 22,560 calls and 90,240 target attempts are accounted for; there are no
accounting failures, point regressions or registered readiness failures.
The unified fitter has no shared fit failures. Eligible match q0/q1 targets
also pass the original two-sided checks, but severe misspecification,
estimated-offset and weak/null diagnostics retain important limitations.
See [the full checkpoint](docs/UNIFIED_RESIDUAL_MOMENTS_DEVELOPMENT_2026-09-07.md)
and its machine-readable result. Its frozen manifest is
`.local/diagnostics/unified-residual-moments-20260907/comparison-1/manifest.json`;
build and tiny prerequisites are `paired-build-1` and `tiny-1` beside it.
It contains 22,400 match calls and 160 paired observation regression calls,
all reconstructed from saved outcomes. All bound source and executable hashes
were rechecked after completion. Keep this evidence immutable. The independent
auditor reconstructed all 384 target summaries and 96 diagnostic rows, verified
11,280 baseline replays, and passes 17 arithmetic and corruption tests.

Paper work has started: the command section, inference supplement and
limitations now describe the common fitter, corrected individual/joint
reporting and source-specific evidence. The companion `PAPER_PLAN.md` records
the remaining runnable-example/parity, local-scaling, compatibility, figure
and PDF checks. The 59-page manuscript and delivered PDF are refreshed and
visually checked; the complete paper `make check` passes with 105 tests.
The new result is bounded development evidence, not independent confirmation.
No paper completion is claimed yet. The current scaling result is below; dated checkpoints
below describe their original sources.

The public Stata command now passes direct comparison with the three saved
native fixtures (observation q0, match q0, match q1), including interval
diagnostics, actual fitter, Counter counts and absent q1 Gaussian covariance.
Receipts are under `stata-parity-2` beside the comparison; `stata-parity-1`
preserves a harness-only missing-matrix check error, with no estimator change.

The new companion-paper executor reuses archived CSV outcomes and preserves
the historical harness. Its 20 focused tests and seven summary/audit tests
pass. The final tiny pipeline, including a deliberate rc=459 failure and
paired point/sample checks, passes under
`../fevc-paper/replication/results/raw/unified_match_scaling_tiny_20260907_v2`.
The frozen 27-call scaling run is complete in
`../fevc-paper/replication/results/raw/unified_match_scaling_20260907_v1`
and independently audited: all 27 calls pass, with unchanged paired points
and samples. At 245,760 observations the median point/q0/q1 times are
20.541/341.844/160.008 seconds; inference RSS is about 1.05 GiB. All q0
intervals return; q1 withholds the covariance target in the three smallest-size
repetitions. These are runtime/availability results, not coverage evidence.
The paper scaling prose, generated table and Stata color/grayscale figures
are refreshed and visually checked. The paper Makefile/validator include the
new inventory audits, exhibits and logs; three integration tests pass, and
the 27 executor/auditor tests pass again. The 59-page manuscript has 24
main-text pages. The complete visual audit and delivered-PDF validation pass.
A new read-only paper archive audit pins the historical stability manifests,
source commit and raw outputs instead of requiring today's modified checkout
to match the old clean source. Eight corruption/identity tests pass; the
frozen harness and scientific results are unchanged. Final PDF SHA-256:
`97b1b5241db1af3990734f9a22128d0f55251a68c19c24c43083cb4969efece0`.

The refreshed public Veneto example is **not successful for inference**:
point-only PASS, both q0/q1 COMMAND_FAILED (rc=498), with
`SINGULAR_INFORMATION [observation_residual_moments]` from the small
information-matrix conditioning check. Its complete three-call audit is under
`../fevc-paper/replication/results/raw/unified_veneto_20260907_v1` and
`../fevc-paper/replication/results/derived/unified_veneto_audit.json`.
This new runnable-example failure does not rewrite the bounded comparison
PASS or justify promoting the old Veneto intervals as current. The manuscript
now labels them historical and reports the current failure. Scaling inventory
and audit are complete as recorded above.

The owner-approved diagnosis-only replay on 2026-09-08 localizes the failure:
the reduced 15-term predictor basis passes, but the probe-estimated moment
matrix has one negative scaled eigenvalue (-0.0142961). q0 and q1 capture
identical matrices and fail before interval-specific calculations. Projection
checks pass. See [the diagnostic report](docs/VENETO_CONDITIONING_DIAGNOSIS_2026-09-08.md).
Production source/binaries and thresholds are unchanged; this is not a repair.
The proposed next owner decision is an isolated same-input test of the reviewed
direct residual-probe representation, with unchanged model and admission
checks. Do not weaken the conditioning gate, add ridge, or promote automatically.

The owner then approved the isolated direct residual-probe test. It is complete:
both Veneto q0/q1 calls return all four intervals, with unchanged point exports
and sample. The candidate Gram is positive definite and passes the original
gate; 72/12,828 predictions use the unchanged floor. Eleven focused Rust tests,
independent population/sample covariance oracles, and the output/interval audit
pass. See [the candidate report](docs/VENETO_RESIDUAL_PROBE_CANDIDATE_2026-09-08.md).
Production code/binaries and the paper remain unchanged. This is not coverage
or promotion evidence. Proposed next: bounded numerical-seed stability and
existing-outcome regression checks before integration; no automatic adoption,
new outcomes, threshold changes or paper claim transfer.

The owner-approved bounded continuation is now **FAIL** at 512 Gram probes.
See [the complete validation](docs/RESIDUAL_PROBE_VALIDATION_2026-09-08.md) and
[its prospective registration](docs/residual_probe_validation_v1.json).
All ten additional Veneto calls return all intervals, but six-seed SE CVs are
12.5%/11.8%/17.8% for worker/firm/covariance and one seed floors 6.8% of
predictions. The complete 3,680-call paired saved-outcome comparison has no
shared fit or point regressions, but fails registered coverage/SE screens.
Match-q1 equal-independent firm coverage falls from 97% to 89.5%; observation
q0 controls-firm coverage falls from 92% to 88%. All attempted outcomes and
target-local failures are accounted for. A separate raw audit resolves the
first reporter's complete-campaign-helper/subset mismatch without changing
outcomes, eligibility or thresholds. Neither production nor paper is updated.
Proposed next owner decision: a bounded 2,048-Gram-probe precision comparison
on the same inputs/seeds, retaining 200 JLA probes, plus exact small-matrix
checks. Do not integrate the 512-probe candidate or tune a seed/gate.

The owner-approved 2,048-Gram-probe precision comparison is complete. See
[the source-bound report](docs/RESIDUAL_PROBE_PRECISION_2026-09-08.md) and
[V2 registration](docs/residual_probe_precision_v2.json). The corrected twelve
Veneto calls pass all six-seed stability screens; component SE CVs fall to
4.5–6.5%, and maximum flooring falls to 0.234%. The initial twelve Stata
count-reconciliation failures are preserved separately, with a bounded
two-literal receipt fix in an isolated copy using the same native plugin.
The 1,600-call saved-outcome replay and independent accounting audit complete,
but three registered calibration screens still fail. Match-q1 firm coverage
improves to 93.5% (absolute screens pass, paired-drop screen narrowly fails);
two observation firm targets retain SD/RMS-SE above 1.10. Exact three-term
small-matrix checks improve for every tested seed, but do not establish exact
behavior on these failing designs. Production and paper remain unchanged.
Proposed next owner decision: exact-Gram diagnostics on those three existing
simulation designs and saved outcomes, not an automatic further probe-budget
increase, model change or promotion.

The owner-approved exact-Gram diagnostic is complete: [report](docs/RESIDUAL_EXACT_GRAM_2026-09-08.md),
[registration](docs/residual_exact_gram_v1.json) and
[result](docs/residual_exact_gram_v1_result.json). Independent SVD and normal-
equation oracles agree, all 600 saved-outcome calls/2,400 intervals are accounted
for, and points/point MCSEs remain identical. Exact-Gram substitution resolves
the match-q1 paired-drop and dominant-observation SE screens; match firm
coverage is 97.5%, dominant firm SD/RMS-SE is 1.054. The controls-firm ratio
remains 1.151 and fails the unchanged 1.10 screen. This selected-subset diagnosis
does not qualify a candidate. Public code, plugins and paper remain unchanged;
780 source Python tests and the assembly check pass. Retain the common fitter
as the working architecture; exact matrices now benchmark any future scalable
Gram work. Treat the surviving controls-case calibration limitation separately,
without automatically raising the probe budget, changing models or promoting.

## Previous owner decision, 2026-09-06

The owner approved the runnable-software upgrade and subsequent paper update,
with **200 JLA probes**, unchanged routing, residual-moment observation fitting,
the unchanged match fitter, and individual intervals separated from joint
covariance admission. Implementation and validation are in progress; nothing
in this decision converts historical failures into passes or qualifies the new
default. The prospective contract is
[`individual_inference_upgrade_v1.json`](docs/individual_inference_upgrade_v1.json).

Implement core reporting and observation integration, versioned native/Stata
transport, focused development checks, then a separately frozen confirmation
at 200 probes. A material development failure blocks confirmation; a fresh
confirmation failure blocks promotion without automatic retuning. Update the
paper only after the relevant software gates. Commit, publication and final
distribution remain separate decisions. The historical checkpoints below
retain their source-bound meaning; their internal-only next-step restrictions
are superseded by this explicit owner decision.

## Current runnable-upgrade checkpoint, 2026-09-06

The residual-moment observation/V5 individual-inference integration is
implemented at the unchanged 200-probe default. Its registered development
run is **FAIL**, with all 19,200 calls and 76,800 target attempts independently
audited. Confirmation and paper/release promotion are stopped. Match-q0
worker/firm intervals fail spectral certification in four diffuse cells;
three primary q1 coverage rows are conservative and fail unchanged gates.
Individual reporting retains 5,388 computable intervals from joint-rejected
calls, without turning availability into a coverage claim. See
[the complete checkpoint](docs/INDIVIDUAL_INFERENCE_DEVELOPMENT_2026-09-06.md)
and its machine-readable result. No default, gate or target exclusion was
changed after seeing the result. The next owner decision is the bounded q0
spectral problem and saved-draw diagnosis of conservative q1 cases. The owner
subsequently approved that bounded follow-up; its completed checkpoint follows.
The final source passes 732 Python tests, the 16 separate harness tests, Rust
and native ABI checks, dirty-tree Apple Silicon/Rosetta/universal qualification,
isolated installation and the integrated Stata quick/full suites. See the
[engineering receipt](docs/individual_inference_engineering_v1_result.json).
These engineering passes do not override the scientific FAIL. Changes remain
uncommitted; full-platform qualification and paper updates are not performed.

## Current bounded follow-up, 2026-09-06

The [registered follow-up](docs/individual_inference_followup_v1.json) resolves
all 32 selected q0 spectral failures in 60 matched q0 replays by giving public
match q0 512 iterations, with the same 200 JLA probes and 0.002 certificate.
Points, covariance inputs and Counter draws are unchanged; runtime is about
1.9 times the baseline in the small fixtures. Match q1 remains at 128.
Stata validates and reports the actual budget, including corruption tests.

All 800 saved q1 calls in the two flagged cells pass independent interval and
critical-radius arithmetic checks. Observation worker/total overcoverage is
more sensitive to the covariance estimate than to the conservative radius;
match total covariance calibration is close and the conservative radius
contributes more. Opposite-half empirical covariance is a simulation-only
diagnostic, not a replacement estimator or proof of a persistent fit bias.
See [the diagnosis](docs/INDIVIDUAL_INFERENCE_FOLLOWUP_2026-09-06.md).
The original development remains FAIL. Stop before new draws, confirmation or
paper promotion. The proposed next bounded scientific decision is an exact
population-covariance comparison on these same designs, without a new fitter.

## Current population-covariance diagnosis, 2026-09-07

The owner approved the bounded population comparison. All 800 saved point
vectors and their q1 centers reconstruct with the actual fixed 200-sketch kernels;
four final native captures reproduce the original outputs. No new outcomes
or production changes were made. See
[the population diagnosis](docs/INDIVIDUAL_POPULATION_COVARIANCE_2026-09-07.md).

Mean reported variances are close to population values. Observation total's
remainder ratio is 0.996, while the variance in its 400 saved draws is only
0.797 of population. Its coverage remains 97.75% with population covariance;
match-total coverage similarly remains 98.25%. Small cross-covariance
discrepancies remain, but a wholesale variance-fitter rewrite is not indicated.
The next proposed owner decision is a predeclared higher-replication validation,
retaining the current estimator and probes. No new draws are authorized by
this checkpoint. The original development FAIL, stop before confirmation,
paper/release boundary and separate acceptance-contract decisions remain.

## Previous RC binary checkpoint

RC preparation source `4d5470f870f350121da7a1c8bf1a625e66e04a4c`
sets `0.5.0-rc.1`, adds a complete five-binary payload constructor and a
guarded Windows build/install smoke, and aligns the Windows C/Rust CRT.
It passes 726 Python tests, integrated Stata quick/full/install checks,
exact-source macOS arm64/Rosetta/universal qualification, SCC Linux job
`7468587` (including public match q0/q1 and isolated installation), and a
fresh dependency/SBOM gate. Windows smoke failed with `STATA_DRIVER_FAILED`;
the machine is stopped and cleanup is complete. Its receipt does not identify
the compiler/Stata failure stage. Sanitized diagnostic and tested-binary
collection needs the separately requested bounded runner extension before
another Windows attempt.
See [RC binary checkpoint](docs/RC_BINARY_CHECKPOINT_2026-09-05.md).
No complete all-platform archive or public release is claimed yet.
The Windows receipt must retain its original CRLF bytes: its recorded SHA-256
binds the controller output, so the evidence path has an explicit Git text
normalization exception. The first evidence commit's normalization is repaired
without changing the original receipt contents or rewriting history.

The explicit fixed-offset, mover-only match q0/q1 interface is implemented.
Source `53f22a109effee87467b4ef0602b21d0b8ec1ca9` passes public Stata,
clean-source macOS arm64/Rosetta native qualification and isolated installation.
The statistical core and joint-nuisance/combined-population point defaults
are unchanged. The source-bound implementation, scientific compatibility and
artifact identities are in
[`FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md`](docs/FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md)
and its JSON result. The preceding documentation checkpoint is `0adc143`.

The completed documentation cleanup removes obsolete internal-only and
confirmation-pending claims without changing execution or old evidence.
It passes 714 Python tests, CMG checks, integrated Stata quick/full suites,
and clean installation with the updated help. Eleven documentation regressions
protect the supported scope, warnings, examples and package inventory.
[`RC_FINALIZATION.md`](docs/RC_FINALIZATION.md) records the remaining
candidate decisions and verification. On 2026-09-05 the owner authorized
`0.5.0-rc.1` preparation with the complete Mac, Linux and Windows binary
payload, private Linux/Windows tests and exact-artifact checks. Push, tag and
public distribution remain separate owner decisions.

## Owner-requested observation follow-up, 2026-09-06

The owner reopened bounded local diagnosis of the observation-q1 shortfall.
The original-draw replay isolates shrinkage of influential error variances;
a residual-moment variance candidate restores calibration in those cases and
has a projected-probe, small-matrix implementation path. This is exploratory
development, not qualification or a change to the accepted match interface.
See [residual-moment development](docs/OBSERVATION_VARIANCE_REMEDY_DEVELOPMENT_2026-09-06.md).
The separate internal Rust fitter now passes dense/solver checks and the
original-draw replay; see the
[implementation checkpoint](docs/OBSERVATION_RESIDUAL_MOMENTS_INTERNAL_2026-09-06.md).
It is not attached to Stata and its core API still assumes exact leverages.
The owner-requested JLA-input/size check, same-design deletion comparison and
independent confirmation are now complete; see the
[follow-up checkpoint](docs/OBSERVATION_RESIDUAL_MOMENTS_FOLLOWUP_2026-09-06.md).
All 400,000 fresh target attempts were audited. The predeclared JLA-input
arm passes the scientific gates; the exact-input arm fails one overcoverage
gate (joint-control firm coverage 96.60%, ceiling 96.50%). This remains a
mixed internal result, not public qualification. Scaling reaches 32,899 rows
with about 1.6--1.7 seconds of variance-fit preparation, excluding the JLA
prepass and complete inference command. Bounded internal integration is now
complete: 799/800 native calls succeed, and the one joint-PSD failure also
appears with an exact trace and the same fitted variances. Native firm
coverage is 94--97% in the 100-draw-per-design slice and matches its paired
exact calculation. See the
[integration checkpoint](docs/OBSERVATION_RESIDUAL_MOMENTS_INTEGRATION_2026-09-06.md).
Outcome-free geometry, dense moments, memory/Counter accounting, local Stata
and dirty-worktree macOS arm64/Rosetta checks pass. The separately registered
[end-to-end confirmation](docs/OBSERVATION_RESIDUAL_MOMENTS_CONFIRMATION_2026-09-06.md)
is complete: 50,000 native calls and 400,000 native/paired-reference target
rows were audited. All 39 correctly specified primary rows pass bias,
coverage and SE-calibration gates, but the run is FAIL: the dominant t8
case has 98.28% availability versus 99% required (43 rejected calls).
All 43 rejections persist with exact traces and an exact point kernel;
all become admissible with true variance inputs. All retain positive
worker/firm/total marginal variances, but that is not yet a q1 validity
check. The separate
[target-specific audit](docs/OBSERVATION_RESIDUAL_MOMENTS_Q1_TARGET_AUDIT_2026-09-06.md)
now establishes that all 43 rejected draws have computable worker, firm and
total q1 intervals with comfortably positive leading/remainder covariance
pairs. All 43 successful comparison draws reproduce the original output;
independent kernels, traces, critical values and interval endpoints agree.
The original full-call failures remain. The next bounded step is an explicit
internal marginal-only result contract that separates individual interval
availability from joint covariance reporting, not a changed fitter, PSD
clipping or gate waiver. Preserve strict joint-request behavior and withhold
invalid joint matrices; no fabricated `e(V)`, public promotion or null/multi-mode
claim. Prospectively register broader existing-draw development checks before
any new confirmation decision.
The original FAIL, null/multi-mode limitations, and release boundaries remain.

The owner-requested corresponding
[match target audit](docs/MATCH_TARGET_INTERVAL_AUDIT_2026-09-06.md) is complete.
All 5,189 original joint-PSD rejections and 16 successful comparisons replay;
20,820 target attempts and 333 dense checks reconcile. Among 5,000 rejected
match-q1 calls, worker/firm/total intervals are individually computable in
4,918/4,909/4,994 draws, and all three survive together in 4,828. Genuine
target-local failures remain. Match q0 retains positive scalar variances in
120/126/189/182 of its 189 rejected draws (worker/firm/covariance/total).
In all 128 dense weak/null q1 draws, the joint matrix still fails with exact
traces and true variances. Do not replace the match variance fitter on this
evidence. The next bounded implementation is a common **internal**
individual-interval contract for observation and match, with separate joint
availability and strict target checks. Preserve invalid-joint withholding,
old confirmations, weak/null/multi-mode exclusions and fixed-offset limitations;
register broader development before a new confirmation/public decision.
This audit changes no production code or release-candidate reporting.

## Accepted scientific scope

- Match q0: independent full confirmation PASS at
  `bb580fe69085d1f98c9151cea2de038aec0a8ba6`; 140,000 target attempts.
  See [match-q0 result](docs/RC_MATCH_Q0_CONFIRMATION_2026-09-05.md).
- Eligible one-mode match q1: repaired independent confirmation PASS at
  `4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`; 140,000 target attempts.
  See [match-q1 result](docs/INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md).
- Corrected observation confirmation: FAIL at
  `73fa75805c8cef6d4d1a6ad843da5894ccedc956`; one firm q1 SE ratio of
  1.101204 exceeds 1.10. All 200,000 attempts were audited. See the
  [confirmation](docs/RC_OBSERVATION_CONFIRMATION_2026-09-05.md) and
  [existing-output diagnosis](docs/RC_OBSERVATION_RATIO_REVIEW_2026-09-05.md).
  The small cutoff excess does not erase the broader calibration limitation.
- The [prospective owner decision](docs/fixed_offset_match_interface_v1.json)
  permits match integration while preserving the observation FAIL; it does
  not approve public release or waive a scientific gate.
- The fixed-offset approximation omits nuisance-control estimation uncertainty.
  Independent matches, a named structured aggregate-variance model and the
  target's q-specific concentration assumptions remain necessary. Few
  controls do not guarantee negligible omitted uncertainty. The
  [paired controls diagnosis](docs/FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md)
  and [deterministic diagnosis](docs/FIXED_OFFSET_DIAGNOSIS_2026-09-04.md)
  remain limitations, not a second-stage correction.

## Remaining bounded work

1. Prepare and qualify the full native payload for macOS arm64/x86_64,
   Linux x86_64 and Windows x86_64; retain the observation-q1 warning and
   current scientific scope. Start with bounded platform smoke gates.
2. Bind the final selected artifact to its source, inventory, notices and
   isolated installation checks. The approved Windows runner currently returns
   only a receipt; bounded tested-binary collection needs separate runner approval.
   The human package-boundary/provenance review is already complete; final
   exact-artifact approval remains distinct.
3. Obtain explicit authorization before any push, tag, publication or native
   binary distribution.

Do not reopen joint-control match inference, combined mover/stayer component
inference, a second-stage correction, automatic q selection, q>1, or a new
Monte Carlo/performance campaign during this milestone. The new public match
boundary is locally qualified on Mac arm64/Rosetta only; old Linux or scaling
evidence does not automatically qualify it.

## Verification and evidence policy

Candidate promotion follows
[`development_acceptance_v1.json`](docs/development_acceptance_v1.json).
Choose gates by affected behavior and record compatible evidence reuse.
Documentation-only changes do not require rebuilding native binaries or
rerunning scientific campaigns. Help/catalog prose can change the portable
package hash while leaving native build inputs and runnable examples intact.

Minimum source gates:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

When local Stata/MP is available, use
`./.venv/bin/python fevc/tools/run_checks.py`. Details and impact-selected
native gates are in [TESTING.md](TESTING.md). Public workflows remain hosted,
source-only and read-only; licensed Stata remains local/private.

## Historical evidence

The former chronological PLAN entries are preserved in Git at `0adc143`.
Their development instructions are superseded, not current work. Original
registrations, failed and passing campaigns, source manifests, reviews and
receipts remain immutable under [docs](docs/README.md) and
`rust/qualification/evidence/`. Durable scientific and engineering contracts
live in the active documentation, not duplicated here.
