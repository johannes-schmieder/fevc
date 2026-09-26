# Pending changes

## Frozen subsample boundaries — 2026-09-26

- Keep combined mover–stayer samples and stayer counts inside the requested
  `if`/`in` sample in both Mata and Rust dispatch. Excluded rows, including
  rows with missing values, cannot enter through a missing logical indicator.
- Stabilize JLA stayer ordering in both dispatch paths against incidental Stata sorts so fitting
  selected rows agrees with fitting the same rows as a separate dataset.
- Add paired subsample/compact-data regressions for exact and JLA, controls,
  weights, pooled deletion IDs, graph exclusions, and caller restoration.
  Preserve deletion-unit mover eligibility and missing-input failures within
  the requested match sample. Compiled plugin files are unchanged.

## Original deletion-unit mover eligibility — 2026-09-26

- Count original declared match blocks for match-mode mover eligibility and
  fixed-point support, including parallel blocks at pooled model employers.
  Keep effect IDs separate from deletion IDs and freeze one-block stayer
  eligibility before physical-observation augmentation. Default match and
  observation-mode populations retain their meanings.
- Apply deletion-unit support in Mata and Rust, including final graph
  certification and supported match-component/projection inference. Native
  readiness bit 15 protects users with older plugins: automatic requests may
  use Mata, while strict requests require the updated plugin.
- Add literal-refit, weighted/control, JLA, sample/partition, restoration,
  failure and focused correlated-block Monte Carlo regressions.

## Applied help and example data — 2026-09-22

- Shorten the help, place a runnable example first, explain the leave-out sample
  and main options, and group advanced options by purpose. Refer technical
  discussion to the companion paper and existing implementation guides.
- Add `fevc, simulate_data(ex1)` through `ex5`, with readable installed source,
  true realized variance components, seed control for random examples, explicit
  data replacement, RNG restoration, and data rollback on failure. Keep all
  five clickable examples and their data-restoration behavior.

## Command-wide runtime progress — 2026-09-19

- Label one command-wide wall clock as `Total elapsed`, spanning Stata and
  native work, with a completion line after successful cleanup.
- Show leverage and target probe counts together, marking targets pending
  until they begin. Preserve throttling, quiet modes, and older plugin support.

## Rust runtime reporting — 2026-09-18

- Add default sample, planning, memory, and coarse phase/probe progress messages.
- Add `nolog` and `verbose`; respect public `nodisplay` and `quietly` while
  preserving memory warnings, errors, and all estimator decisions.
- Add an optional synchronous reporting ABI with caller-thread-only Stata
  output, bounded stack storage, and unchanged legacy entrypoints. Older
  compatible binaries continue estimating without live reporting.


## Package-prefixed helper filenames — 2026-09-18

- Rename all 28 shipped `_fevc*.ado` helpers and their entrypoints to
  `fevc__*.ado`, keeping SSC runtime files in the package's `f/` directory.
- Preserve public commands, numerical code, private backend identities and
  native plugin filenames. Update maintained callers, package inventories,
  tests and qualification tooling together; ship no old-name wrappers.
- Before upgrading an older development installation, uninstall its registered
  package, install the current package, and restart Stata. `replace` alone
  leaves obsolete helper files behind. See the installation guide.

## Symmetric stayer population options — 2026-09-12

- Default to `stayers(both)` for observation and match deletion. Observation
  deletion keeps its existing full retained population and physical-row
  corrections; it does not use the match/stayer mixed correction.
- Make explicit `stayers(movers)` exclude original one-firm workers in both
  modes. This intentionally changes explicit observation/movers calls from
  earlier prereleases; omission or `stayers(both)` retains the prior population.
- Preserve native ABI and raw receipts; reconcile public population, option
  presence and pre-graph exclusion counts independently. Correct display and
  `estat sample` so observation/both is not described as mixed deletion.
- Qualify supported point/projection surfaces without changing solver kernels,
  tolerances, inference algorithms, fusion, installed binaries or paper figures.

## Observation/match solver symmetry — 2026-09-12

- Share degree-four full CMG, thread-aware batching and the ordered unfused
  queue beneath eligible observation and match JLA point estimation. Preserve
  the separate statistical probes, deletion units and correction formulas.
- Add generic-route pool, output and construction-peak accounting; preserve
  optional budgets, phase-specific residual gates and cancellation/reuse.
- Test both modes on repeated-match and weighted mixed-degree fixtures across
  eight thread counts. Keep controlled/inference capabilities and fusion
  unchanged. See the [implementation report](docs/DELETION_SYMMETRY_2026-09-12.md)
  for source-bound qualification and bounded comparison evidence.

## Thread-aware full-CMG batching and queue — 2026-09-12

- Integrate the tested k=2 automatic batching policy for full CMG, scaling
  leverage/target widths with permitted native threads and probe counts.
- Drain independent scalar RHSs through an ordered workspace queue. Preserve
  output/error order, cancellation, complete residuals and scalar refinement.
- Admit selected fallible workspace pools and checked queue storage before
  probe RNG; preserve omitted/explicit memory policy and frozen ABI layouts.
- Keep fused RHS experimental and outside normal builds. Other solver routes,
  public options, statistical targets and default tolerances are unchanged.
- Qualify exact runtime source on Mac arm64/Rosetta and SCC Linux, plus the
  integrated package and clean installs. Record evidence and limitations in
  the [integration report](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/BATCH_QUEUE_2026-09-12.md).

## Exact degree-four full CMG — 2026-09-11

- Eliminate degree-four workers through their exact weighted Schur clique in
  the full-CMG route; retain generic/inference routing and higher-degree stars.
- Charge a checked conservative edge bound before construction and preserve
  optional-memory, residual, RNG, cancellation and lifecycle contracts.
- Add independent weighted Schur and edge-forecast regressions. Selected
  1.6m/6.4m SCC checks pass; see the [development report](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/DEGREE_FOUR_CMG_2026-09-11.md).
- Leave scheduling, smaller aligned batches and fused RHS experimental rather
  than assuming their earlier gains compose with the reduced graph.

## Control preparation with 256 lanes — 2026-09-10

- Register the owner AKM example in the manual inventory, restore the frozen
  historical README bytes, and regenerate parity wording from its JSON source.
- Increase only Mata's compensated control-product block to 256 lanes; retain
  the error coefficient, estimator, tolerances, selection and native arithmetic.
- Price control preparation before allocation and in the generic solver's
  overlapping resource forecast. Advance the Mata runtime to API 24 and the
  resource runtime to API 12 so older loaded implementations are rejected.
- Preserve real control-preparation UserBreak as return code 1, clear estimates
  and run ordinary caller-state cleanup; do not report a runtime mismatch.
  Preserve the captured ordinary exact-Mata return code as well as JLA's.
- The [follow-up validation](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/CONTROL_LANES_256_2026-09-10.md) records
  passing audits, arithmetic/invariance/lifecycle/memory gates, integrated
  Stata checks, Rosetta coverage and measured 64/256-lane comparisons.

## Certified control basis — 2026-09-10

- Certify the selected anchor and its predecessors while preserving the global
  maximum, semantic order, eligibility rule and all numerical gates.
- Retain Rust Neumaier sums; add compensated Mata canonical cross products and
  explicit product, second-order summation and underflow error bounds.
- Preserve control-Gram inverse failures as public control-basis ambiguity with
  measured cause detail; add specific troubleshooting and independent oracles.
- Advance only Mata runtime identity to API 23,
  `vckss-api23-certified-control-basis`; command and native layouts are unchanged.
- Validate the original 50,000-row AKM example through Mata, Rust and automatic
  routing, with local arm64/Rosetta qualification. The
  [checkpoint report](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/CONTROL_BASIS_REPAIR_2026-09-10.md) records unrelated
  aggregate-audit failures and remaining large-control Mata performance cost.


## Optional memory budgets and measured preparation — 2026-09-10

- Remove the public 4-GiB default. Omission forecasts without memory-based
  execution changes; explicit budgets default to warning and support error/off.
- Refine CMG forecasts from the constructed solver and selected phase batches;
  separate expected allocations from conditional refinement reserve.
- Measure Rust preparation allocations, remove duplicate/wrong-solver CMG
  charges, and remove fixed historical RSS allowances from public Mata forecasts.
- Add versioned native policy/forecast transport and bump changed Mata runtime
  identities. Legacy native entrypoints keep their strict numeric behavior.

## Direct residual Gram candidate — 2026-09-08

- Use half the centered residual-probe moment covariance for the current Gram
  calculation, default 2,048; add `inferencegramprobes()` separately from the
  unchanged 200 point probes. Preserve model, positivity, rank and q safeguards.
- Add V4 augmentation with an explicit count, preserve V1–V3 native semantics
  and the V5 result layout, and reconcile the chosen count throughout Stata.
- Retain historical calibration failures and document approximate-inference
  limitations under `inference_completion_v1.json`. No release is included.

## Unified-fitter candidate — 2026-09-07, not qualified

- Use the residual-moment fitter for both observation and fixed-offset match
  component inference. Match residuals and FE projection use one weighted
  aggregate per declared match; regression mass is not independent replication.
- Reduce only outcome-free redundant variance predictors, preserving the
  model span and existing conditioning and positivity safeguards.
- Add V3 augmentation entrypoints, retain V1/V2 semantics and V5 result layout,
  and report actual match Gram work instead of inapplicable cross-fit metadata.
- Bounded development comparisons, final native/Stata qualification and the
  companion-paper update remain pending; old coverage passes do not transfer.

## Individual-inference candidate — 2026-09-06, not qualified

- Increase the pre-RNG public match-q0 spectral budget from 128 to 512
  iterations to certify near-tied diffuse modes. Keep match q1 at 128,
  observation at 512, JLA probes at 200 and every residual gate unchanged.
  Saved-draw numerical checks pass; the original scientific FAIL remains.
- Attach the small residual-moment variance fitter to explicit observation
  inference. Retain match cross-fitting, the exact Mata family, point-only
  defaults and 200 JLA probes; introduce no new user option.
- Add V2 augmentation/V5 result transport with separate q0 target and joint
  covariance statuses. Withhold invalid joint matrices without suppressing
  computable individual intervals. q1 never posts a Gaussian `e(V)`.
- Report actual variance-fit, Gram, solver, memory and Counter diagnostics;
  use outcome-free observation ordering and document finite-probe ties.
- Add native/Stata regressions and a source-bound default-setting experiment.
  Fresh validation, platform qualification and release promotion remain pending.

## 0.5.0-rc.1 — 2026-09-05

- Prepare the first fixed-offset match inference release candidate. Match q0
  and eligible one-mode q1 are explicit, approximate structured-model options;
  they omit nuisance-control estimation uncertainty. Point defaults are unchanged.
- Retain the corrected observation-q1 confirmation failure and calibration
  warning. No scientific cutoff is waived and no new coverage claim is made.
- Synchronize public version identifiers. The owner requests a complete native
  installation payload for macOS arm64/x86_64, Linux x86_64 and Windows x86_64.
  Platform qualification and exact-artifact installation must finish before
  that payload is accepted; version assignment alone is not a public release.

## Explicit fixed-offset match inference — 2026-09-05

- Expose the separately confirmed grouped q0 and eligible q1 kernels through
  explicit Rust generic-JLA/Counter-V1, match deletion, fixedoffset nuisance,
  mover-only population, structured model and diagonal/CMG options. Preserve
  point defaults, declared match IDs, target mass and frequency semantics.
- Add an explicit match augmentation entrypoint and additive V1 unit receipt
  without changing the frozen statistical-result ABI or estimator formulas.
  Reconcile and display independent/effective match counts, mass share,
  leverage, maker denominator and omitted nuisance-estimation uncertainty.
- Preserve target-specific q1 withholding and structural fail-closed behavior.
  Add native, public Stata, corruption, copy-equivalence and clean-install gates.
- Record the owner-approved integration scope. The corrected observation-q1
  confirmation remains FAIL with a known calibration shortfall; no new
  simulation, scientific waiver, release or binary distribution is implied.

## Q1 inference repair — 2026-09-04

- Correct maximal curvature to use the leading variance, not its square root,
  and certify q1 covariance using a dimensionless correlation determinant.
- Align Mata recentering with the raw leave-out leading product and check its
  direct remainder identity. Component point estimation is unchanged.
- Add target-specific q1 availability, native result ABI V4, missing Stata AM
  endpoints and status diagnostics, actual critical-draw/solve accounting,
  and explicit rejection of partial exports through legacy result interfaces.
- Diagnose historical covariance-target failures without suppressing valid
  worker, firm, or total intervals. Preserve historical receipts and require
  fresh q1 qualification.
- Describe fixed-offset match inference as approximate uncertainty that omits
  nuisance estimation, not proven conditional inference given estimated controls.

## Internal fixed-offset match q1 foundation — 2026-09-04

- Register the separate grouped `q=1` construction after the accepted q0
  development result. The raw leave-match product recenters one leading mode;
  the structured aggregate-match variance fit is used only for covariance and
  studentization.
- Add independent frequency-expanded physical-block and collapsed-scalar
  dense oracles for the leading eigenmode and score, raw recenter, direct
  rank-one remainder, physical-to-scalar kernel compression, and joint
  leading/remainder covariance under four within-match covariance patterns.
- Extend only the internal Rust attachment to grouped `q=1`, using the existing
  KSS/Andrews--Mikusheva maximal-curvature critical radius and ellipse-image
  map. Diagonal and CMG routes preserve the point estimate, Counter-V1 batch
  invariance, complete-system residual gates, and typed null-signal and
  delete-match failures.
- Keep the parser, plugin ABI, Stata returns, automatic routing, and public
  capability registry unchanged. Numerical success remains distinct from a
  target having one dominant mode and a diffuse remainder; no concentration
  cutoff or grouped q1 campaign is introduced.
- Record a pre-result registration amendment that corrects only the notation
  distinguishing population plus-trace variance from the realized-influence
  minus-trace covariance estimator.
- Register the first bounded q1 development campaign before any outcome is
  inspected. Its target-specific contract treats worker, firm, and total as
  eligible only in one-mode cells with a diffuse remainder; the deliberately
  multi-mode covariance target remains outside the coverage claim.
- Add a source-bound 14-cell generator, high-resolution outcome-free design
  preflight, production Counter-V1 critical-value execution, atomic Python
  manifest/task/aggregate receipts, adversarial validators, and one-core SCC
  build/task/aggregate launchers. These remain internal development tools and
  do not change the parser, plugin ABI, Stata surface, or point estimator.
- Record the clean-source 56-row local tiny pipeline and the exact-source
  one-core SCC Linux build/task/aggregate smoke. Both reconcile completely;
  this authorizes only the frozen bounded q1 development profile, not a
  scientific coverage claim or public match-inference route.
- Complete the frozen 280-task, 22,400-attempt q1 development profile at exact
  source `c4e9f36` and retain its untuned `FAIL` decision. Equal-mass
  structured-common and leverage-only cells miss the registered 0.98 success
  rate, and structured-common worker coverage among successful fits is
  `0.9820`, just outside the frozen upper tolerance. Unequal-mass q1 cells,
  the diffuse q0 comparator, mild-omission gate, severe-omission limitation,
  multi-mode exclusion, weak/null withholding, numerical identities, and the
  complete scheduler/output inventory otherwise behave as registered. This
  blocks confirmation, a larger experiment, and public q1 match routing until
  a separate bounded diagnosis; it changes no production or public code.

## Internal fixed-offset match q=0 foundation — 2026-09-04

- Add an internal-only Rust generic-JLA `q=0` attachment that conditions on
  the full-sample estimated control offset and treats each declared match as
  one collapsed scalar inferential observation. Positive integer frequency
  weights remain regression mass rather than independent inference copies;
  target mass remains separate.
- Register and test the exact scalar-collapse identities against independent
  original-row block-maker and collapsed-scalar dense oracles, including
  whole-match deletion, point correction, zero-block-diagonal kernels, and
  Gaussian covariance under unrestricted within-match dependence.
- Fit the primary structured aggregate-match variance model from outcome-free
  ranks of match leverage, the three primitive target diagonals, and match
  regression mass, while retaining leverage-only sensitivity results.
- Keep the path outside the parser, plugin ABI, Stata returns, and automatic
  routing. At this q0 checkpoint grouped `q=1`, eligible stayers,
  joint-nuisance uncertainty, and every public support claim remained staged
  pending separate evidence.
- Register the first bounded q0 campaign with semantic Counter-style seeds,
  outcome-free target-specific regime checks, exact source/file bindings,
  atomic per-task outputs and receipts, complete attempt classification, and
  frozen coverage/misspecification gates. Its tiny and one-task SCC profiles
  are execution checks only.
- Generate physical-row errors under independent, common-shock, serial, and
  equal-aggregate-variance covariance shapes while making the correct
  aggregate variance exactly affine in the structured model's normalized
  match-mass midrank. Controls vary within matches before fixed-offset removal;
  that cell is diagnostic and not coverage-eligible.
- Pass the campaign's complete 14-task local tiny pipeline and its clean-source
  one-core SCC Linux build/task/aggregate smoke with exact manifest, binary,
  receipt, scheduler-accounting, and output-inventory reconciliation. These
  are execution checks only.
- Complete the frozen 280-task, 22,400-attempt development profile at exact
  source `c26a7ee`. All registered gates pass: correct-model coverage is
  `0.9325`--`0.9775`, empirical-to-estimated SE ratios are
  `0.9324`--`1.0437`, mild omission passes, severe omission visibly
  invalidates the total interval, and weak/null failures remain typed and
  fully counted. This accepts the internal q0 foundation for a separate q1
  derivation/oracle slice; it does not expose or promote match inference.

## Structured observation-inference promotion — 2026-09-04

- Promote only the explicit Rust generic-JLA/Counter-V1 observation-deletion
  `q=0` and eligible one-mode `q=1` structured capabilities after the clean
  preregistered V5 confirmation passed every frozen scientific, spectral,
  execution, and inventory gate. Point estimation and automatic/default
  inference routing are unchanged.
- Keep `inferencemodel(structured_common|structured_leverage)` as the sole
  explicit structured-model selection. Do not add an unrestricted-KSS alias,
  redirect exact Mata requests, or admit match deletion, eligible stayers,
  nonunit frequency weights, within-match dependence, or general `q>1`.
- Expose supported-capability, selected-reference, population, variance-model,
  and asymptotic-scope metadata. The default display and `estat diagnostics`
  show target-specific leading, remainder, mode-weight, and influence
  concentration together with prominent misspecification warnings.
- State that `q=0` needs diffuse spectral and influence contributions and
  `q=1` removes one leading mode but needs a diffuse remainder. A completed
  calculation is not evidence that these conditions hold, and no post-hoc
  concentration cutoff or automatic `q` choice is introduced.
- Preserve typed atomic withholding for weak/null, nonpositive-covariance,
  unidentified, singular, nonconverged, malformed, or inadmissible requests.
  Severe omitted variance drivers can invalidate inference without changing
  the established component point estimates.
- Carry V5 scientific claims to the promotion source only through the recorded
  compatibility review and exact-source native/Stata qualification. No release,
  tag, binary distribution, or version change is implied.

## Structured component q=1 correction — 2026-09-03

- Keep every component point estimate unchanged, but recenter the explicit
  Rust q=1 leading square with its raw observation leave-out mode variance
  product. The positive cross-fitted structured variance vector now enters
  only joint covariance estimation and studentization.
- Add a fail-closed direct-remainder identity, result ABI V3 diagnostics for
  the raw recenter and critical-draw count, independent critical-value and
  ellipse-image numerical oracles, and a 100,000-draw public q=1 minimum.
- Register the corrected factorized moderate-dimension campaign before new
  coverage evidence. At that historical checkpoint the structured routes
  remained experimental pending the later V4 diagnosis and V5 confirmation;
  they were never unrestricted-heteroskedastic KSS inference.
- Record the clean source-bound V3 smoke and 20,000-attempt development run.
  The correction satisfies its direct-remainder identity, but the oracle t8
  firm cell at dimension 64 still covers 0.972 and fails the frozen gate;
  confirmation and promotion therefore remain withheld.

## Projection inference repair — 2026-09-02

- Replace the biased centered observation proxy with uncentered cross fitting
  and a symmetrized block identity for unrestricted within-match covariance.
  Exact Mata and sparse Rust/JLA now honor the default match-deletion
  mover/stayer partition; explicit observation deletion remains supported.
- Projection slopes are location-normalization invariant. The automatic
  intercept is normalization-dependent under last-firm-zero grounding.
- Replace the repetitive default result display with one additive table that
  jointly reports plug-in values, estimated bias, KSS-corrected values, and
  corrected outcome-variance shares. Preserve component and projection
  inference tables, add conditional sample/routing warnings, and expose the
  complete accounting through `estat decomposition, full`, `estat sample`,
  `estat computation`, and `estat diagnostics`.
- Reorganize and expand the Stata help around the effective estimator,
  deletion, inference, and projection contracts. Add executable component-
  inference and mixed mover/stayer projection examples.
- Prepare the repository for public source development: replace privileged
  runner workflows with read-only hosted source checks, pin workflow actions,
  align ignore and cleanup policy, and move paper-specific coefficient-one
  material to the companion paper repository.

# Changelog

## 0.5.0-alpha.1 — 2026-08-30

- Match the maintained MATLAB package's default match-deletion population:
  retained movers plus eligible attached one-firm stayers in one pooled fit
  and target. Movers retain declared match deletion; stayers use literal
  physical-observation deletion and are explicitly not match-robust.
  `stayers(movers)` is the mover-only opt-out.
- Make the combined result the primary `e(results)`, `e(b)`, `e(kss)`, and
  `e(sample)` contract, while retaining exact mover intermediates under
  `e(mover_*)` and compatibility aliases under `e(stayer_hybrid_*)`.
- Implement the mixed convention in Mata and Rust generic JLA, including
  joint leverage sketches, deterministic rank checks, memory admission,
  zero-stayer reduction, and exact-oracle regression coverage.

- Rename the public Stata command, package, help topic, and repository from
  `vckss` to `fevc` as a hard cut. No compatibility command or wrapper is
  installed.
- Keep the version at `0.5.0-alpha.1` and preserve the estimator, returned
  results, numerical gates, and routing behavior.
- Normalize every distributed Mata/runtime, private Ado-helper, and native
  plugin artifact filename to `fevc`; private Stata programs now use
  `_fevc_*`. Retain the established internal `vckss__*`, `__vckss_*`,
  `VCKSS_*`, CMG, Rust crate, C ABI, build, runner, and SCC identities.
- Install the result table as the autoloadable `_fevc_display.ado` helper so
  exact and Rust postprocessors remain display-safe after an in-session
  development-package replacement.
- Extract the planned-route projection receipt check into its existing private
  namespace so Stata can skip the no-projection branch without misparsing
  nested braces; numerical checks and posted results are unchanged.
- Preserve predecessor reports, receipts, reviews, migration records, and the
  earlier changelog under their original identities. Remove the duplicate
  `vckss/` working tree after pinning its complete Git tree and adding a
  browsable archive index; retain the unique CMG source-review manifest in the
  active vendor provenance directory.
- Add a deterministic, non-publishing portable source-archive builder driven
  by `fevc.pkg`, with exact file, metadata, receipt, and reproducibility tests.
- Harden clean-install coverage for help lookup and missing-plugin preflight
  fallback, and protect source-bound evidence logs from workspace cleanup.
- Reconcile the active plan with the completed rename qualification and close
  the repaired scale-bundle and Stata 19 harness issues without changing any
  estimator, routing, fallback, inference, RNG, or numerical semantics.

## 0.5.0-alpha.1 — in development

- Add opt-in Mata exact inference for unit-weight observation deletion.
  `inference(highrank)` posts a polarized joint covariance for the four
  established component targets; point-only calls remain unchanged and post no
  `e(V)`.
- Add `inference(q1)` rank-one weak-identification diagnostics and
  Anderson--Rubin-style intervals. Critical values and interval mapping are
  independently implemented from the published KSS formulas; no MATLAB source
  or table is distributed.
- Add worker- or firm-effect projections on an automatic constant and numeric
  covariates, with frequency- or target-mass weighting and separate KSS and
  naive covariance returns under `e(projection_*)`.
- Add an explicit scalable `project()` route for observation deletion and
  positive integer frequency weights through qualified Rust generic JLA with
  either diagonal PCG or forced CMG. Frequency weights are literal physical-
  copy counts. Both routes reuse the retained sparse solver, solve one
  coefficient-space loading per projection column, stream KSS and naive
  covariance accumulation, and reconcile complete-system residual,
  conditioning, PSD, memory, and result-schema receipts. Forced CMG reuses one
  admitted hierarchy across the full and fixed-effect solves and fails closed;
  automatic projection routing remains withheld.
- Add fail-closed capability routing, smoothing/covariance/eigen gates,
  deterministic inference seeds, caller-RNG restoration, an independent dense
  projection oracle, and focused tests for default compatibility, q=1 critical
  values, target identities, typed failures, and clean packaging.
- Keep component inference on Mata exact, and keep match-cluster projection,
  projection stayer hybrids, and non-generic Rust projection routes outside
  the initial scalable projection capability.

- Import standalone CMG `761a0f0` as the immutable pre-routing comparison
  checkpoint while preserving VCkss cancellation, memory admission, warm
  starts, and the contiguous-RHS bridge.
- Integrate the `d9fef06` connected vector-only routing candidate, source-bound
  at descendant `92a12f2` after its arithmetic-order-preserving strict-Clippy
  repair and identifiable-Laplacian benchmark correction, for non-regression
  qualification against the `761a0f0` checkpoint.
- Parallelize independent Schur-RHS assembly for multi-column full-CMG batches
  on the solver-owned thread pool while preserving each column's arithmetic
  order, the one-column/one-thread path, cooperative worker cancellation, and
  `CMG_FULL_V2`. Extend the checked pre-RNG batch-vector forecast for every
  concurrently live worker-scaled temporary.
- Fail candidate qualification, pilot, production, retry, and aggregation
  closed unless a hash-bound preparation receipt proves that Stata/MP licenses
  the four Stata processors needed by the benchmark; this distinguishes the
  16 scheduler slots and Rust/MATLAB target from the capped Stata/Mata
  application entitlement before launching large SCC arrays.
- Version the comparative input receipt at V6 and use a degree-three shallow
  hub-tree leaf-panel graph for the weak connected-vector cell. Five panels
  share each leaf and select spokes in a diameter-four tree with 1,601 hubs;
  the largest cell has 655,360 workers, 132,673 firms, and 788,032 canonical
  edges. It clears CMG's frozen 131,072-vertex and 350,000-edge vector floors
  while contracting completely in one level with zero plan bytes. Earlier
  cycle, chord, block, offset-ring, hub-ring, and six-hub repairs either failed
  the unchanged residual gate, built the same plan in both sources, or could
  not reach the vector floor. The final mapping passed a full local one-core
  200-probe estimator screen at maximum complete residual `6.5084e-6`. The
  registered graph/row/core/repetition matrix and scientific gates remain
  unchanged.

## 0.4.0-alpha.1 — in development

- Make corrected statistical-result equivalence and end-to-end MATLAB
  competitiveness the primary development gates. Register a scale- and
  numerical-MCSE-aware comparison policy; retain bitwise/ULP equality, equal
  iterations, and legacy fixed roundoff thresholds as nonblocking diagnostics
  while keeping estimator, residual, finite-output, accounting, memory,
  failure, and caller-state safety hard.
- Replace the alpha benchmark's exact-repeatability and Mata-speed promotion
  gates with V2 corrected-result equivalence and MATLAB-primary performance
  status. Until a source-bound maintained-MATLAB timing receipt is integrated,
  the analyzer reports `INCOMPLETE` rather than making an alpha claim.
- Begin the private performance-first alpha milestone for macOS arm64/Rosetta and
  SCC Linux x86-64; Windows and public release remain deferred.
- Register Rust-preferred automatic backend and MATLAB-like JLA/200-probe
  defaults as the alpha target, with Mata fallback limited to missing-runtime
  or unsupported-request preflight.
- Qualify the Rust-preferred default and effective-option planned JLA routes on
  macOS arm64, universal, and Rosetta at source `daca3e2`; omitted `rng()`
  resolves to Counter-V1 on Rust and Stata RNG on a preflight Mata fallback.
- Qualify semantic `probeorder()` tie breaking for public compressed and
  generic Rust JLA at source `3bc6a89`, including a versioned preparation ABI,
  exact memory accounting, receipt reconciliation, permutation/batch
  invariance, and clean installation.
- Qualify exact `stayers(both)` through a versioned native augmentation
  lifecycle at source `c199bf0`, including dense/Mata differential oracles,
  zero-RNG reconciliation, typed failures, and arm64/Rosetta clean installs.
- Qualify the Linux x86-64 candidate on BU SCC under Stata MP 19 at source
  `86e0711`, with successful Rust/C/ABI gates, full public suite, isolated
  clean installation, candidate/source hashes, and SGE job accounting.
- Add source-bound bounded Miri, C-shim ASan/UBSan, malformed-ABI libFuzzer,
  RustSec audit, license-inventory, and deterministic CycloneDX 1.5 SBOM
  evidence. Windows and final human public-release review remain deferred.
- Add a generated Rust/Mata parity ledger and a protected dry-run/apply cleanup
  tool. The initial cleanup removed 42,120 ignored files and 3.98 GB without
  touching tracked source or source-bound evidence.
- Stop byte-locking the mutable latest-CI pointer while retaining immutable
  per-SHA receipt protection.
- Preserve the private direct full-CMG performance wave and its registered
  hard-case decisions. The fixed-CZ18 alternating matrix reaches 2.0667x
  maintained MATLAB, while the synthetic alternating matrix reaches 1.4803x
  and modestly exceeds MATLAB peak RSS. Both pass the statistical, residual,
  and state gates; at that checkpoint the route remained private because the
  synthetic performance and memory gates failed. Official full-CMG repeated
  solves were the registered next bottleneck; the subsequent scalar
  production wave supersedes that decision without rewriting its evidence.
- Vendor and pin standalone CMG `dbefbc5`, adopt Rust 1.85.1, and promote the
  scalar direct hybrid solver as `CMG_FULL_V2` with checked whole-command
  pre-RNG memory admission, actual-retained reconciliation, deterministic
  same-route residual refinement, cooperative UserBreak cancellation, and
  exactly-once lifecycle cleanup.
- Qualify the registered no-control match-JLA cell through explicit Rust and
  automatic backend selection on macOS and Linux. Runtime source `4b6874e` is
  1.397x matched MATLAB on the macOS headline and 1.731x MATLAB on SCC's fixed
  CZ18 case while remaining faster than the private winner in both matrices.
  Unsupported cells retain their prior routes; errors
  after full-CMG selection never fall back. Windows and public release remain
  deferred.
- Preserve exact-source macOS, SCC, supply-chain, timing, memory, residual,
  and statistical receipts and publish the source-bound CMG-style benchmark
  report. No 2x-MATLAB, Windows, tag, or public-release claim is made.
- Inventory and remove 26,310 obsolete regenerable build, cache, CI-scratch,
  and local-plugin files totaling 4.187 GB while preserving tracked reports,
  failures, benchmark receipts, and qualification evidence.

## 0.4.0-dev — 2026-08-24

- Make a hard-cut public rename from `varcomp_kss` to `vckss`: install only
  `vckss.ado`, `vckss.sthlp`, and `vckss.pkg`, and post
  `e(cmd) == "vckss"` without a predecessor wrapper.
- Rename active developer/runtime entrypoints, plugin artifacts, CI markers,
  environment variables, package metadata, repository URLs, and build IDs to
  the `vckss` identity while retaining established private `_vckss_*`,
  `vckss__*`, and `VCKSS_*` interfaces.
- Retain archived reports, receipts, reviews, progress snapshots, manifests,
  and completed qualification evidence byte-for-byte. The unchanged v1 frozen
  inventory and source-bound v2 relocation inventory enforce that boundary.
- Record CMG ownership API 8 and generator API 5 for the renamed generated
  package target without changing its numerical implementation.
- Repair exact-V7 batch-applicability and poster plan-reason reconciliation
  without weakening any request, plan, residual, accounting, memory, or
  pre-RNG counter check.
- Admit explicit `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)`
  requests through the frozen native V3/V4/V7 plan. When that plan selects
  exact, reconcile and post the exact family directly with zero estimator RNG,
  exact direct-memory admission, and exactly-once lifecycle cleanup.

## Unreleased — 2026-08-24

- Add the optional strict Rust backend with compositional request-capability,
  preparation, solve, result, release, and typed-error boundaries.
- Add deterministic exact, compressed-JLA, and generic-JLA result families,
  Counter-V1 randomized execution, complete original-system residual
  certification, and V7 pre-RNG execution-plan receipts.
- Add structural `engine(auto)`, route, CMG/diagonal fallback, batch, memory,
  wall-advisory, and counter-accounting receipts. Automatic choices are frozen
  before estimator RNG and never reroute after a later numerical or resource
  failure.
- Package the planned exact-V7 reconciler and poster and bind exact-limit,
  selection-reason, selected-engine, plan-memory, and zero pre-RNG counter
  fields in their tests.
- Expand source-local macOS plugin, clean-install, arm64/universal, exact,
  generic, compressed, routing, shared-atom, and differential test coverage.
- Preserve explicit `backend(mata)` and `rng(stata)` while making omitted and
  automatic backend requests Rust-preferred after capability preflight.
- Consolidate active documentation around one package README, one current plan,
  one testing guide, and an indexed contract/evidence directory. Remove
  redundant single-use trusted-patch staging after the exact-V7 helpers were
  already present in the package manifest.

## 0.3.0-dev — 2026-08-20

- Rework the help file in the `cellgraph` SMCL style and add three
  deterministic, self-contained AKM examples that execute from the Stata help
  browser through the installed `varcomp_kss_run` helper while preserving the
  caller's data.
- Add an applied default display and `e(decomposition)`, separating raw
  covariance from the additive `2 x covariance` sorting contribution and
  reporting plug-in and corrected shares of target-weighted outcome variance
  and worker--firm totals.
- Use fixed-width display tables with explicit column headers and readable
  component labels. Each executable example reports the population worker and
  firm variances, covariance, and total implied by its DGP before estimation.
- Add retained target- and frequency-weighted outcome variances, residual
  variance, and the descriptive full-model explained variance and share. The
  full-model fit includes controls and remains distinct from the KSS-corrected
  worker--firm target.
- Standardize recognized failure output with a plain-language reason,
  actionable remedy, technical status, troubleshooting link, and
  `e(withholding_detail)`, `e(withholding_reason)`, and
  `e(withholding_suggestion)` metadata.
- Extend source, clean-install, package-layout, output-identity, failure, and
  executable-help tests without changing the estimator or established result
  matrices.
- Rename the public Stata command, package, help topic, and shipped files to
  `varcomp_kss`; do not install a predecessor-command alias.
- Rename active private Ado/Mata namespaces and runtime build identifiers to
  the `vckss` family while preserving estimator and numerical contracts.
- Internalize CMG under `varcomp_kss/cmg`, expose only the package and test
  generator targets, and remove the unused non-KSS pullback.
- Record CMG API 7 and generator API 4 as ownership/interface boundaries over
  the numerically qualified API 6 implementation.

The byte-exact predecessor changelog is preserved under `docs/history/`.
