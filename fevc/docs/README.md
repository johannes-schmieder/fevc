# Documentation index

This directory separates active contracts from source-bound engineering
evidence. Start with the active package files before reading a historical
report.

## Active package guidance

- [Inference completion report](INFERENCE_COMPLETION_2026-09-08.md),
  [source/evidence result](inference_completion_v1_result.json), and
  [frozen registration](inference_completion_v1.json): the direct residual
  Gram, 2,048 default probes and separate precision option are implemented.
  Local Mac, replay, public-example, scaling and paper checks pass within
  the owner-approved approximate-inference scope. Historical calibration
  failures remain failures; this is not a public release or new confirmation.

The following dated registrations describe their original decisions and sources.

- [`unified_residual_moments_v1.json`](unified_residual_moments_v1.json):
  owner-approved common residual-moment fitter for observation and fixed-offset
  match deletion, bounded saved-draw validation and approximate-inference paper
  update. Implementation and bounded saved-draw assessment pass; historical
  results remain source-bound. See the
  [complete assessment](UNIFIED_RESIDUAL_MOMENTS_DEVELOPMENT_2026-09-07.md).
  Paper examples/scaling are audited; current inference fails on public Veneto.
  The [2026-09-08 diagnosis](VENETO_CONDITIONING_DIAGNOSIS_2026-09-08.md)
  localizes this to an indefinite estimated moment matrix shared by q0/q1,
  not the predictor-basis check. No repair or promotion has been made.
  The subsequent [isolated residual-probe candidate](VENETO_RESIDUAL_PROBE_CANDIDATE_2026-09-08.md)
  returns all four Veneto q0/q1 intervals with unchanged points and gates;
  production remains unchanged and coverage is not established by this replay.
  Its [bounded continuation](RESIDUAL_PROBE_VALIDATION_2026-09-08.md) fails
  predeclared seed-stability and saved-outcome calibration screens at 512 Gram
  probes. All 3,680 paired calls are accounted for; no integration is approved
  by that result. The [registration](residual_probe_validation_v1.json) remains
  unchanged. The subsequent [2,048-probe precision experiment](RESIDUAL_PROBE_PRECISION_2026-09-08.md)
  passes Veneto seed-stability screens and improves saved-outcome calibration,
  but three registered screens still fail across 1,600 new calls. Its
  [V2 registration](residual_probe_precision_v2.json) retains 200 JLA probes
  and unchanged thresholds. No production or paper promotion follows.
  The owner-approved [exact-Gram diagnostic](RESIDUAL_EXACT_GRAM_2026-09-08.md)
  then resolves two of the three flagged screens on 600 saved outcomes;
  observation-with-controls firm SD/RMS-SE remains 1.151. Its
  [registration](residual_exact_gram_v1.json) and
  [result](residual_exact_gram_v1_result.json) are diagnosis only, not promotion.

- [`INDIVIDUAL_POPULATION_COVARIANCE_2026-09-07.md`](INDIVIDUAL_POPULATION_COVARIANCE_2026-09-07.md):
  exact population moments for the actual fixed-200-sketch kernels on the
  same two q1 designs; 800 saved draws, no new outcomes. Reported variances
  are close to population, and true covariance does not cure total overcoverage.
  Original FAIL and confirmation/paper/release boundaries remain unchanged.

- [`INDIVIDUAL_INFERENCE_FOLLOWUP_2026-09-06.md`](INDIVIDUAL_INFERENCE_FOLLOWUP_2026-09-06.md):
  bounded match-q0 iteration repair at 200 JLA probes, unchanged arithmetic
  checks and saved-draw q1 covariance/radius diagnosis. The old development
  FAIL remains; no new outcome draws or confirmation. Its registration and
  exact evidence receipt are linked in the report.

- [`INDIVIDUAL_INFERENCE_DEVELOPMENT_2026-09-06.md`](INDIVIDUAL_INFERENCE_DEVELOPMENT_2026-09-06.md):
  implemented runnable candidate at 200 JLA probes; 19,200-call development
  FAIL, complete independent accounting, and the stop before confirmation or
  paper promotion. Source-bound result, final engineering checks and ordering
  evidence are linked there.

- [`INDIVIDUAL_INFERENCE_INTERFACE.md`](INDIVIDUAL_INFERENCE_INTERFACE.md):
  candidate V3 augmentation/V5 result interface, actual variance-fit reporting, individual versus
  joint availability, numerical ordering and remaining qualification gates.

- [`individual_inference_upgrade_v1.json`](individual_inference_upgrade_v1.json):
  owner-approved runnable observation residual-moment and shared individual-
  interval upgrade, with 200 JLA probes and unchanged routing. Implementation
  is not yet qualified; fresh default-setting validation is required.

- [`MATCH_TARGET_INTERVAL_AUDIT_2026-09-06.md`](MATCH_TARGET_INTERVAL_AUDIT_2026-09-06.md)
  and [`match_target_interval_audit_v1_result.json`](match_target_interval_audit_v1_result.json):
  all 5,189 original match joint-PSD rejections replay unchanged; 4,828/5,000
  rejected q1 calls retain all three headline intervals, with genuine local
  failures preserved. Independent dense checks support a common internal
  individual-interval contract, not a new fitter, coverage claim or public change.
- [`OBSERVATION_RESIDUAL_MOMENTS_Q1_TARGET_AUDIT_2026-09-06.md`](OBSERVATION_RESIDUAL_MOMENTS_Q1_TARGET_AUDIT_2026-09-06.md)
  and [`observation_residual_moments_q1_target_audit_v1_result.json`](observation_residual_moments_q1_target_audit_v1_result.json):
  all 43 joint-PSD-rejected draws have computable primary q1 intervals;
  43 successful comparison draws and independent covariance/interval oracles
  agree. Existing-draw diagnosis only. Next: a separately specified internal
  marginal-only result contract, retaining strict joint covariance checks.
- [`OBSERVATION_RESIDUAL_MOMENTS_CONFIRMATION_2026-09-06.md`](OBSERVATION_RESIDUAL_MOMENTS_CONFIRMATION_2026-09-06.md)
  and [`observation_residual_moments_confirmation_v1_result.json`](observation_residual_moments_confirmation_v1_result.json):
  complete 50,000-call native confirmation; all primary calibration gates
  pass, but heavy-tailed dominant-mode availability is 98.28% versus 99%
  required. All 43 rejected draws remain invalid with exact traces, while
  true variance inputs restore joint admissibility. No public promotion or
  waiver; the separate target-specific q1 audit is indexed above.
- [`OBSERVATION_RESIDUAL_MOMENTS_INTEGRATION_2026-09-06.md`](OBSERVATION_RESIDUAL_MOMENTS_INTEGRATION_2026-09-06.md)
  and [`observation_residual_moments_integration_v1_result.json`](observation_residual_moments_integration_v1_result.json):
  hidden Rust live-solver attachment with explicit outcome-free keys; 799/800
  complete native calls, close exact-oracle agreement, one preserved joint-PSD
  failure and local source/native gates. Development only, no public option
  or waiver of the previous overcoverage failure.
- [`OBSERVATION_RESIDUAL_MOMENTS_FOLLOWUP_2026-09-06.md`](OBSERVATION_RESIDUAL_MOMENTS_FOLLOWUP_2026-09-06.md)
  and [`observation_residual_moments_followup_v1_result.json`](observation_residual_moments_followup_v1_result.json):
  completed JLA-input/size and same-physical-data deletion checks; 400,000
  independently seeded target attempts. JLA-input arm PASS, exact-input arm
  FAIL on one overcoverage gate. Conditional internal evidence only; no public
  observation option or change to the match release candidate.
- [`OBSERVATION_RESIDUAL_MOMENTS_INTERNAL_2026-09-06.md`](OBSERVATION_RESIDUAL_MOMENTS_INTERNAL_2026-09-06.md),
  [`observation_residual_moments_internal_v1.json`](observation_residual_moments_internal_v1.json)
  and [`observation_residual_moments_internal_v1_result.json`](observation_residual_moments_internal_v1_result.json):
  separate core-only projected residual-moment fitter, independent dense and
  full-quotient solver checks, and original-draw replay. Exact leverage input
  only; no changed Stata option, match route or confirmation status.
- [`OBSERVATION_VARIANCE_REMEDY_DEVELOPMENT_2026-09-06.md`](OBSERVATION_VARIANCE_REMEDY_DEVELOPMENT_2026-09-06.md)
  and [`observation_variance_remedy_development_v1_result.json`](observation_variance_remedy_development_v1_result.json):
  owner-requested local residual-moment variance candidate, paired original-draw
  comparisons, all-attempt failure accounting and projected-probe feasibility.
  Promising development only; no production change or confirmation waiver.
- [`OBSERVATION_VARIANCE_FIT_DIAGNOSIS_2026-09-06.md`](OBSERVATION_VARIANCE_FIT_DIAGNOSIS_2026-09-06.md)
  and [`observation_variance_fit_diagnosis_v1_result.json`](observation_variance_fit_diagnosis_v1_result.json):
  original-outcome oracle replay isolates underestimated bridge variances in
  the observation-q1 shortfall; original confirmation remains FAIL.

- [`RC_BINARY_CHECKPOINT_2026-09-05.md`](RC_BINARY_CHECKPOINT_2026-09-05.md):
  source-bound RC1 Mac/Linux, local source/Stata and supply-chain results;
  Windows smoke fails without a detailed diagnostic; the machine is stopped.
  Bounded Windows evidence collection and final all-platform archive gates
  remain open.

- [`RC_BINARY_PAYLOAD.md`](RC_BINARY_PAYLOAD.md): owner-requested complete
  Mac/Linux/Windows RC payload, bounded platform gates, exact-artifact
  installation and the pending Windows collection boundary.

- [`RC_FINALIZATION.md`](RC_FINALIZATION.md): current remaining candidate
  decisions, distribution/installation boundary and documentation-only
  compatibility checks. It does not authorize release or new experiments.
- [`FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md`](FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md)
  and [`fixed_offset_match_interface_v1_result.json`](fixed_offset_match_interface_v1_result.json):
  completed explicit public match q0/q1 interface; 703 Python tests, integrated
  Stata and clean-source arm64/Rosetta native/install gates pass. Includes
  source compatibility and the current owner-facing RC checklist; observation
  q1 remains an unresolved FAIL and release remains a separate owner decision.
- [`fixed_offset_match_interface_v1.json`](fixed_offset_match_interface_v1.json):
  owner-approved explicit fixed-offset match q0/q1 integration and engineering
  gates; the observation confirmation remains FAIL and release is not authorized.
- [`RC_OBSERVATION_RATIO_REVIEW_2026-09-05.md`](RC_OBSERVATION_RATIO_REVIEW_2026-09-05.md)
  and [`rc_observation_ratio_review_v1_result.json`](rc_observation_ratio_review_v1_result.json):
  owner-approved existing-output diagnosis, not new confirmation. The 1.10
  boundary miss is only 0.072 MCSE, but both leverage/common one-mode firm
  rows show an SE shortfall. The original FAIL remains unchanged.
- [`FIXED_OFFSET_RC_CHECKLIST_2026-09-05.md`](FIXED_OFFSET_RC_CHECKLIST_2026-09-05.md):
  bounded release-candidate checkpoint and remaining gates. Match q0 passes;
  corrected observation confirmation has one frozen SE-ratio failure, so
  its historical pause is superseded only by the prospective interface decision above.
- [`RC_MATCH_Q0_CONFIRMATION_2026-09-05.md`](RC_MATCH_Q0_CONFIRMATION_2026-09-05.md)
  and [`rc_match_q0_v1_result.json`](rc_match_q0_v1_result.json): independent
  140,000-attempt PASS with all-attempt, source, byte-level, and scheduler audit.
- [`RC_OBSERVATION_CONFIRMATION_2026-09-05.md`](RC_OBSERVATION_CONFIRMATION_2026-09-05.md)
  and [`rc_observation_inference_v1_result.json`](rc_observation_inference_v1_result.json):
  complete 200,000-attempt corrected-source FAIL under unchanged V5 gates;
  the one-mode leverage-only firm SE ratio is 1.101204 versus maximum 1.10.
- [`RC_INFERENCE_SOURCE_COMPATIBILITY_2026-09-05.md`](RC_INFERENCE_SOURCE_COMPATIBILITY_2026-09-05.md):
  recorded reuse of accepted repaired match-q1 science and unchanged native
  source identities; post-confirmation test-only q0-example maintenance.
- [`rc_match_q0_v1.json`](rc_match_q0_v1.json): fresh full match-q0
  confirmation registration for the bounded fixed-offset release candidate;
  unchanged q0 scientific gates, fixed outcome-free folds, independent seeds,
  all-attempt accounting, and compute-node smoke prerequisites. Not a result.
- [`rc_observation_inference_v1.json`](rc_observation_inference_v1.json):
  corrected-source observation confirmation with the original V5 matrix and
  scientific cutoffs, independent pipeline/confirmation seeds, outcome-free
  preflight, curvature identities, and fixed-fold diagnostics. Not a result.

- [`INFERENCE_REPAIR_ERRATUM_2026-09-04.md`](INFERENCE_REPAIR_ERRATUM_2026-09-04.md):
  corrected q1 curvature, target-specific availability, and fixed-offset
  approximation. Earlier q1 coverage receipts do not qualify the repair.
- [`inference_repair_v1.json`](inference_repair_v1.json): owner-approved repair
  and fresh-evidence contract, registered before repair execution.
- [`inference_repair_match_campaign_v1.json`](inference_repair_match_campaign_v1.json):
  frozen corrected-match development/confirmation harness, fixed outcome-free
  folds, independent confirmation seeds, and target-specific accounting.
- [`INFERENCE_REPAIR_CHECKPOINT_2026-09-04.md`](INFERENCE_REPAIR_CHECKPOINT_2026-09-04.md):
  repaired exact-source macOS/Rosetta native qualification, complete local tiny
  pipeline, and audited one-task SCC smoke; not coverage confirmation.
- [`INFERENCE_REPAIR_DEVELOPMENT_RESULT_2026-09-04.md`](INFERENCE_REPAIR_DEVELOPMENT_RESULT_2026-09-04.md)
  and [`inference_repair_match_campaign_v1_result.json`](inference_repair_match_campaign_v1_result.json):
  passing 22,400-attempt repaired match development result, complete failure
  accounting, and the remaining estimated-controls calibration limitation.
- [`INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md`](INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md)
  and [`inference_repair_match_confirmation_v1_result.json`](inference_repair_match_confirmation_v1_result.json):
  passing 140,000-attempt independent confirmation, complete scheduler/hash
  audit and failure inventory, and preserved controls/multi-mode limitations.
- [`fixed_offset_diagnostic_v1.json`](fixed_offset_diagnostic_v1.json):
  prospective known/estimated-offset Gaussian moment diagnosis with two
  controls, growing match samples, and historical/bounded FE loadings;
  no coverage claim or production nuisance correction.
- [`FIXED_OFFSET_DIAGNOSIS_2026-09-04.md`](FIXED_OFFSET_DIAGNOSIS_2026-09-04.md)
  and [`fixed_offset_diagnostic_v1_result.json`](fixed_offset_diagnostic_v1_result.json):
  exact nuisance-induced covariance explains the controls SE shortfall;
  growing samples with two controls do not remove the relative gap in these
  designs. Exact spectrum corrects the interpretation of noisy diagnostics.
- [`fixed_offset_paired_v1.json`](fixed_offset_paired_v1.json): prospective
  four-arm paired controls experiment at the original 400-match design,
  separating offset estimation from variance fitting; 1,000 replications,
  independent physical-row checks, and complete failure accounting. No
  second-stage correction or public qualification.
- [`FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md`](FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md)
  and [`fixed_offset_paired_v1_result.json`](fixed_offset_paired_v1_result.json):
  complete 16,000-attempt paired diagnosis. Known-offset calibration passes;
  estimated-offset total coverage remains about 76% even with true original
  match variances. Independent fits, fixed folds, and raw reaggregation pass.

- [`../README.md`](../README.md): command overview, backend routing,
  installation, and current development boundary.
- [`../PLAN.md`](../PLAN.md): authoritative current milestone and next steps.
- [`../TESTING.md`](../TESTING.md): local, Stata, plugin, SCC, and evidence
  taxonomy.
- [`../CHANGELOG.md`](../CHANGELOG.md): user- and developer-visible changes.
- [`RELEASE_HARDENING_2026-08-31.md`](RELEASE_HARDENING_2026-08-31.md):
  changed-surface review, evidence carry-forward, and final RC gate record.
- [`STRUCTURED_OBSERVATION_INFERENCE_PROMOTION_2026-09-04.md`](STRUCTURED_OBSERVATION_INFERENCE_PROMOTION_2026-09-04.md):
  historical V5-to-promotion compatibility review and exact-source native
  qualification. It does not qualify the subsequent q1 repair; use the current
  corrected-observation FAIL and separately passing match records above.
- [`../AGENTS.md`](../AGENTS.md): mandatory agent constraints.
- [`DECISIONS.md`](DECISIONS.md): durable package, backend, routing, evidence,
  and release decisions.
- [`development_acceptance_v1.json`](development_acceptance_v1.json): active
  performance-first corrected-result equivalence and MATLAB-competitiveness
  policy for candidate promotion and differential development tests.
- [`match_inference_q0_development_v1.json`](match_inference_q0_development_v1.json):
  preregistered local-development contract for the internal fixed-offset,
  collapsed-match `q=0` foundation. It freezes the scalar sufficient
  statistics, match-level structured-variance diagnostics, oracle identities,
  local gates, and limitations before any campaign output is inspected.
- [`match_inference_q1_development_v1.json`](match_inference_q1_development_v1.json):
  prospective local-development contract for the separate internal grouped
  `q=1` slice. It freezes the raw leave-match leading recenter, physical-block
  and collapsed-scalar oracle identities, direct rank-one remainder,
  covariance, diagnostics, failure gates, and exclusions before implementation
  or q1 campaign output.
- [`match_inference_q1_development_v1_amendment1.json`](match_inference_q1_development_v1_amendment1.json):
  pre-result notation correction distinguishing the population remainder
  variance from its realized-influence minus-trace estimator. It changes no
  method, threshold, fixture, or scope.
- [`MATCH_INFERENCE_Q1_LOCAL_CHECKPOINT_2026-09-04.md`](MATCH_INFERENCE_Q1_LOCAL_CHECKPOINT_2026-09-04.md):
  exact-source record of the independent physical-block and collapsed-scalar
  q1 oracles, internal diagonal/CMG attachment, local source and licensed-Stata
  gates, and the boundary before any q1 campaign.
- [`match_inference_q1_campaign_v1.json`](match_inference_q1_campaign_v1.json):
  registered generator, target-specific one-mode eligibility, high-resolution
  outcome-free regime checks, production Counter-V1 critical-value path,
  semantic seeds, task inventory, atomic receipts, scientific gates, and
  exclusions for the first bounded fixed-offset collapsed-match q1 campaign.
- [`MATCH_INFERENCE_Q1_CAMPAIGN_SMOKE_2026-09-04.md`](MATCH_INFERENCE_Q1_CAMPAIGN_SMOKE_2026-09-04.md):
  exact-source record of the complete 14-task local tiny pipeline and the
  one-core SCC Linux build/task/aggregate prerequisite smoke for the registered
  q1 development campaign.
- [`match_inference_q1_campaign_v1_result.json`](match_inference_q1_campaign_v1_result.json):
  immutable machine-readable result of the source-bound 14-cell, 22,400-
  attempt grouped match q1 development campaign. The complete run fails its
  frozen equal-mass success-rate and worker-coverage gates; it is not
  confirmation or public support evidence.
- [`MATCH_INFERENCE_Q1_DEVELOPMENT_RESULT_2026-09-04.md`](MATCH_INFERENCE_Q1_DEVELOPMENT_RESULT_2026-09-04.md):
  reviewed scientific, numerical, exact-inventory, artifact-hash, and SCC
  accounting record for the untuned failed result, plus the handoff to a
  bounded diagnostic slice.
- [`MATCH_INFERENCE_Q0_LOCAL_CHECKPOINT_2026-09-04.md`](MATCH_INFERENCE_Q0_LOCAL_CHECKPOINT_2026-09-04.md):
  exact-source record of the independent dense oracles, internal matrix-free
  q=0 attachment, local gates, macOS arm64/Rosetta licensed-Stata
  qualification, and the boundary before the first bounded campaign.
- [`match_inference_q0_campaign_v1.json`](match_inference_q0_campaign_v1.json):
  registered generator, outcome-free regime checks, semantic seeds, task
  inventory, atomic receipts, scientific gates, and explicit exclusions for
  the first bounded fixed-offset collapsed-match q=0 campaign.
- [`match_inference_q0_campaign_v1_amendment1.json`](match_inference_q0_campaign_v1_amendment1.json):
  pre-result correction of the one-mode outcome-free fixture and tiny-profile
  numerical resolution after the first development preflight stopped before
  manifest creation; all development outcomes, seeds, gates, and inventories
  remain unchanged.
- [`MATCH_INFERENCE_Q0_CAMPAIGN_SMOKE_2026-09-04.md`](MATCH_INFERENCE_Q0_CAMPAIGN_SMOKE_2026-09-04.md):
  exact-source record of the complete local tiny pipeline and one-core SCC
  Linux build/task/aggregate smoke, including scheduler accounting, artifact
  hashes, exact inventory, and the boundary before the bounded development
  profile.
- [`match_inference_q0_campaign_v1_result.json`](match_inference_q0_campaign_v1_result.json):
  immutable machine-readable result of the source-bound 14-cell, 22,400-
  attempt grouped match q=0 development campaign. All registered gates pass;
  this is development evidence, not confirmation or public promotion.
- [`MATCH_INFERENCE_Q0_DEVELOPMENT_RESULT_2026-09-04.md`](MATCH_INFERENCE_Q0_DEVELOPMENT_RESULT_2026-09-04.md):
  reviewed scientific, diagnostic, exact-inventory, artifact-hash, and SCC
  accounting record for that development result, plus the fail-closed handoff
  to a separately derived internal grouped q1 slice.
- [`../../rust/README.md`](../../rust/README.md) and
  [`../../rust/TEST_PLAN.md`](../../rust/TEST_PLAN.md): current native backend
  architecture and qualification gates.

## Scientific and numerical contracts

- [`ESTIMATOR_CONTRACT.md`](ESTIMATOR_CONTRACT.md): estimator, population,
  deletion, weighting, nuisance, and target definitions.
- [`NUMERICAL_ARCHITECTURE.md`](NUMERICAL_ARCHITECTURE.md): model systems,
  quotient conventions, solvers, residual certification, and scale engines.
- [`BLOCK_CONTROL_DERIVATION.md`](BLOCK_CONTROL_DERIVATION.md): controlled block
  deletion derivation and rank conditions.
- [`JLA_FINITE_PROJECTION.md`](JLA_FINITE_PROJECTION.md): improved-JLA
  finite-projection correction.
- [`INFERENCE.md`](INFERENCE.md): opt-in exact-observation and explicit
  structured Rust/JLA observation or fixed-offset match high-rank and eligible
  one-mode q=1 component inference, exact observation/match fixed-effect
  projections, and the explicit sparse Rust/JLA block-projection route. Its focused architecture and scaling protocol are
  in the [archived scalable-projection record](../../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference).
- [`MATRIX_FREE_COMPONENT_INFERENCE.md`](MATRIX_FREE_COMPONENT_INFERENCE.md):
  internal oracle foundation and supported explicit Rust structured-model
  `q=0`/`q=1` component inference, exact
  observation and grouped-match identities, spectral diagnostics, explicit
  support matrix, and the distinction between unrestricted KSS,
  target-specific LOWESS, and structured common variance constructions. The
  strict unrestricted construction is documented but is not an accepted FEVC
  option.
- [`structured_inference_qualification_v2.json`](structured_inference_qualification_v2.json):
  registered moderate-dimension factorized-oracle diagnosis and the unchanged
  gates required before the structured observation `q=0`/`q=1` modes can be
  promoted together.
- [`structured_inference_diagnostic_v2_result.json`](structured_inference_diagnostic_v2_result.json):
  source-bound development result for that diagnosis. The oracle t8 cell at
  dimension 64 failed coverage, blocking confirmation and promotion at that
  historical checkpoint.
- [`structured_inference_qualification_v3.json`](structured_inference_qualification_v3.json):
  registered correction separating the raw q=1 leave-out recenter from the
  positive structured covariance model, plus the fixed moderate-dimension
  development and confirmation rules. Registration alone is not promotion
  evidence.
- [`structured_inference_diagnostic_v3_result.json`](structured_inference_diagnostic_v3_result.json):
  source-bound result for the corrected campaign. All tasks completed and the
  q=1 remainder identity held, but oracle t8 firm coverage at dimension 64
  remained 0.972, blocking confirmation and promotion at that checkpoint.
- [`structured_inference_qualification_v4.json`](structured_inference_qualification_v4.json):
  preregistered narrow development diagnosis separating the exact q=1
  reference law, analytic fixed-population covariance, random studentization,
  covariance-component hybrids, required-radius calibration, and q=0
  comparator in the remaining dimension-64 firm-target cell.
- [`structured_inference_qualification_v4_amendment1.json`](structured_inference_qualification_v4_amendment1.json):
  pre-result correction for the V4 harness's mistaken zero-signal assertion;
  freezes the exact nonzero-signal population covariance and paired vertex and
  nonvertex Gaussian reference diagnostics while preserving the original V4
  registration and coverage rule.
- [`structured_inference_qualification_v4_amendment2.json`](structured_inference_qualification_v4_amendment2.json):
  pre-result validator correction aligning Python with the frozen historical
  Rust label-hash atom. It changes no semantic RNG domain or draw and preserves
  the V4 scientific design and gates.
- [`structured_inference_diagnostic_v4_result.json`](structured_inference_diagnostic_v4_result.json):
  source-bound result for the 30,000-replication V4 calibration/evaluation
  diagnosis. Production and fixed-covariance q=1 agree, Gaussian and
  standardized-t8 results agree, all registered gates pass, and the remaining
  modest overcoverage is the expected conservatism of the KSS
  curvature-bound critical rather than a covariance or implementation defect.
  This development result authorizes neither confirmation nor promotion.
- [`structured_inference_confirmation_v5.json`](structured_inference_confirmation_v5.json):
  preregistered clean source-bound confirmation of the complete structured
  observation-deletion `q=0`/`q=1` matrix. It preserves the V1 coverage and
  misspecification gates, uses the corrected raw `q=1` recenter and
  deterministic qualification critical, and treats deliberately multi-mode
  covariance-target rows as diagnosed nonprimary cases rather than coverage
  claims. Registration alone authorizes no promotion or binary qualification.
- [`structured_inference_confirmation_v5_result.json`](structured_inference_confirmation_v5_result.json):
  immutable result of that confirmation. All registered scientific, spectral,
  inventory, and numerical gates pass across 200,000 target-replication rows.
  This qualifies its historical source, not the later q1 repair. It does
  not qualify release binaries, authorize automatic routing, or turn the
  structured FEVC variance model into unrestricted-heteroskedastic KSS.
- [`FAILURES_AND_RETURNS.md`](FAILURES_AND_RETURNS.md): typed failures and
  returned results/diagnostics.
- [`RUST_MATA_PARITY.md`](RUST_MATA_PARITY.md): generated current backend and
  platform parity ledger for the alpha release gates.
- [`../benchmarks/fevc_matlab_2026/STATUS.md`](../benchmarks/fevc_matlab_2026/STATUS.md):
  accepted 2026 comparative-scaling campaign checkpoint.

## Ownership and provenance

- [`SOURCE_PROVENANCE.md`](SOURCE_PROVENANCE.md): package source ledger.
- [Archived MATLAB package comparison](../../docs/history/VCKSS_ARCHIVE.md#development-result-reports):
  publication-era versus maintained MATLAB package comparison and the exact
  current VCkss benchmark pin.
- [`../../CODE_LICENSE.md`](../../CODE_LICENSE.md): repository licensing and
  release boundary.
- [`../cmg/README.md`](../cmg/README.md),
  [`../cmg/STATUS.md`](../cmg/STATUS.md), and
  [`../cmg/docs/SOURCE_PROVENANCE.md`](../cmg/docs/SOURCE_PROVENANCE.md):
  internal CMG component.

## Retained engineering results

These reports are source-bound evidence. Do not edit them to describe newer
source; add a new report or update the active plan instead.

- [`PREP_RHS_1`, `FE_BUF_1`, and `PREP_BND_1` archived results](../../docs/history/VCKSS_ARCHIVE.md#development-result-reports)

Additional exact-SHA evidence lives under `../../qualification/`, and
independent reviews live under `../../reviews/`.

The private full-CMG architectural spike and its rejected-promotion decision
are documented in
[`../benchmarks/full_cmg_spike/DECISION_REPORT.md`](../benchmarks/full_cmg_spike/DECISION_REPORT.md).
It is source-bound performance evidence, not a qualified public backend. Its
historical numerical gate remains recorded; active development interprets the
candidate under `development_acceptance_v1.json`.

The renewed route's registered hard-case decisions are the accepted
[`fixed-CZ18 P200 checkpoint`](../benchmarks/full_cmg_spike/CZ18_P200_MATRIX_CHECKPOINT.md)
and the non-promoted
[`synthetic P200 decision`](../benchmarks/full_cmg_spike/SYNTHETIC_P200_MATRIX_DECISION.md).
CZ18 clears the 2x target, while synthetic is 1.4803x MATLAB and modestly above
MATLAB peak RSS. The latter identifies official full-CMG repeated solves as
the next performance boundary and explicitly defers hardening and the
benchmark PDF.

## Historical records

Repository-level `../../docs/history/` and `../../docs/migration/` contain
immutable predecessor and migration records. Rust dated checkpoints live under
`../../rust/progress/`. They are not current development instructions.

## Automation boundary

Public GitHub workflows run source-only checks on hosted runners with read-only
repository permissions. Licensed Stata qualification remains an explicitly
local or private operation described in [`../TESTING.md`](../TESTING.md).
Historical exact-SHA receipts under `.ci/stata/results/` remain immutable
evidence, not active runner state.
