# Current decisions

## Inference

- Version `0.5.0-alpha.1` adds inference only by explicit request. Point-only
  behavior and the absence of `e(V)` on default calls are unchanged.
- Component `inference(highrank|q1)` defaults to Mata exact with observation
  deletion, movers, and unit frequency weights. Supported explicit
  `inferencemodel(structured_common|structured_leverage)` instead selects the
  Rust generic-JLA/Counter-V1 observation-deletion attachment for unit-
  frequency movers and joint nuisance handling with low-dimensional controls.
  The owner-approved fixed-offset match attachment is also explicit:
  `backend(rust) algorithm(jla) engine(generic) rng(counter_v1)` with
  `deletion(match) nuisance(fixedoffset) stayers(movers)`, a named structured
  model and `preconditioner(diagonal|cmg)`. It allows integer frequency mass
  and stored-row target weights. Point-estimation defaults are unchanged.
  Fixed-effect `project()` is a
  distinct surface: exact Mata and explicit Rust generic JLA support the
  requested observation or default match partition, positive integer
  frequency weights, and the default mover/eligible-stayer population.
- Scalable Rust component inference is a separately identified supported
  explicit method, never an automatic substitute for `inference(highrank)`. Its
  matrix-free core uses one common positive variance vector for the full joint
  covariance and leaves component point estimates unchanged. Strict
  unrestricted KSS variance products and the common structured FEVC variance
  regression are separate constructions. FEVC does not currently reserve or
  promise a strict unrestricted-KSS public mode. The structured model conditions primarily on
  leverage plus all three primitive target diagonals, with leverage-only as a
  sensitivity analysis; neither it nor the target-specific MATLAB/Mata LOWESS
  comparator may be called the unrestricted KSS estimator. Historical V5
  passed on its own source, but does not qualify the corrected q1 source.
  The corrected observation confirmation FAILS one unchanged SE-ratio gate
  (1.101204 versus 1.10), an unresolved RC limitation. The separately
  registered full match q0 and repaired match q1 confirmations PASS their
  primary gates. Severe omitted variance drivers visibly
  invalidated intervals without changing component point estimates. That
  limitation is part of the supported contract, not a robustness claim.
- The structured Rust `q=1` leading square is recentered by the raw leave-out
  product `sum_i v_i^2 y_i e_(i,-i)`. The common positive fitted variance
  vector is used only for leading/remainder covariance and studentization.
  A direct rank-one-remainder identity is a hard numerical gate. The public
  route uses at least 100,000 Counter-V1 critical draws. The registered V3
  factorized development campaign completed all 20,000 attempts, but the
  oracle t8 firm cell at dimension 64 again covered 0.972 and failed its fixed
  gate despite a `3.3e-13` remainder-identity error. The subsequent registered
  V4 diagnosis used 20,000 calibration and 10,000 independent evaluation
  replications at dimension 64. Production and analytic fixed-population q=1
  coverage were 0.9584--0.9591 under standardized-t8 and Gaussian errors; no
  one-component covariance substitution changed the result materially. Exact
  joint-Gaussian reference draws covered 0.9581--0.9593, while held-out
  calibration of the particular parabola's shortest required radius covered
  0.9483--0.9510. This is the intended conservatism of the KSS
  maximal-curvature, at-least-nominal construction, not a covariance,
  studentization, heavy-tail, decomposition, or ellipse-image defect. No
  production correction or empirical critical value was justified by that
  historical diagnosis. The subsequent curvature/recenter repair and fresh
  confirmations, not historical V5, govern the current source. `q=1` is
  supported only for target-specific one-mode regimes with a diffuse
  remainder; the deliberately multi-mode covariance target remains outside
  the coverage claim even when computation succeeds. No automatic
  concentration cutoff or `q` selection is authorized.
- Deletion unit, variance model, and reference distribution are independent
  dimensions. `q=1` never denotes match deletion. On 2026-09-05 the owner
  authorized public fixed-offset match q0/q1 integration using the separately
  passing confirmations, while retaining the failed corrected observation
  result. This prospective scope decision does not waive a scientific gate
  or approve release; see `fixed_offset_match_interface_v1.json`.
  Match inference collapses fixed-offset outcomes to declared matches and
  permits unrestricted within-match dependence, but assumes independent
  matches and a specified structured aggregate-variance model. The label is
  **Fixed-offset approximate match inference, ignoring nuisance-control
  estimation uncertainty.** Same-sample control estimation can violate the
  independence approximation even with few controls. No joint-controls or
  second-stage correction is included. Match mass/concentration, leverage,
  maker denominator and omitted-uncertainty diagnostics must be returned.
- `inference(highrank)` posts a polarized joint covariance for the three
  primitive targets and maps it to the four established targets.
  `inference(q1)` additionally posts rank-one weak-identification diagnostics
  and Anderson--Rubin-style intervals from repository-authored simulation and
  ellipsoid mapping.
- `project()` is a separate fixed-effect linear-projection surface. Its KSS
  covariance and naive residual-squared comparison are stored under
  `e(projection_*)`; projection alone never populates component `e(V)`.
- Projection covariance uses uncentered cross fitting. For observation units,
  `E[y_i ehat_{i,-i}|X]=sigma_i^2`. For declared match blocks it uses the
  symmetrized identity
  `.5*(y_g ehat_{g,-g}' + ehat_{g,-g} y_g')`, allowing unrestricted
  within-match covariance. The formerly centered proxy is not an estimator.
- The explicit Rust/JLA sparse projection route accepts positive integer
  frequency weights as literal physical-copy counts. Frequency projection
  mass and the KSS/naive covariance are physical-copy weighted; explicit
  target mass remains stored-row mass.
- The sparse projection route accepts explicit `preconditioner(diagonal)` and
  forced `preconditioner(cmg)`. Forced CMG reuses the planned generic-JLA
  hierarchy, supports controls and weighted samples, and fails closed after
  selection. It is not the specialized compressed `CMG_FULL_V2` route.
  Automatic projection routing remains withheld so no existing projection
  request silently changes solver.
- The maintained MATLAB package is a behavioral reference only. Its source and
  critical-value table are not licensed for copying and are not included.
  The implementation follows the published formulas and ships as
  GPL-3.0-only repository-authored source.
- Materially negative smoothed variances or indefinite covariance estimates
  withhold the complete request. Only tiny registered roundoff values may be
  set to zero, and all such cleanups are receipted.

## Comparative-scaling right-censoring

- On 29 August 2026, the owner accepted the largest `strong_d2` one-core
  MATLAB cell as right-censored rather than authorizing another long rerun.
  Maintained MATLAB reached the registered 10,800-second estimator limit in
  all three position-balanced repetitions in both immutable production
  generations. The affected historical task IDs are 61, 62, and 63
  (`scale_strong_d2_n1966080_c1_r1` through `r3`).
- The accepted study therefore contains 297 complete same-host tasks and 891
  successful estimator calls. The three MATLAB lower bounds remain in a
  separate source-bound censor ledger. Partial Mata and Rust results from
  those sequential tasks do not enter the accepted call ledger.
- The entire graph--size--core cell is excluded from route rankings, paired
  ratios, and one-to-many-core scaling. This is a declared missing-cell rule,
  not an imputation or a relaxed numerical/application gate.
- Future comparative-scaling generations must not submit those three exact
  experiments. The executable record is
  [`future_exclusions.json`](../benchmarks/comparative_scaling/future_exclusions.json).

## Development priority and equivalence

- The primary product objective is a fast Stata alternative to maintained
  MATLAB KSS that returns the same statistical result on compatible problems.
  Corrected-result equivalence is first, end-to-end MATLAB competitiveness is
  second, and pathwise/backend-internal parity is secondary.
- The machine-readable policy is
  [`development_acceptance_v1.json`](development_acceptance_v1.json). For a
  corrected target pair `a,b`, let `s=max(1,abs(a),abs(b))`. Deterministic
  comparisons accept `abs(a-b)<=1e-8*s`; common-draw randomized comparisons
  additionally allow ten percent of the larger reported numerical MCSE.
  Independent-draw comparisons accept the greater of the scale floor and `6`
  times combined numerical MCSE. Comparators without MCSE use a registered
  repeated-seed distribution.
- The primary result comparison is the four corrected targets. Plug-in and
  correction rows, bitwise/ULP equality, iteration counts, counters, and
  reduction order remain valuable diagnostics but do not block a faster
  candidate whose corrected estimates pass the registered rule.
- Legacy source-bound receipts keep their original thresholds and statuses.
  Active development may reinterpret a legacy roundoff miss under the current
  policy without rewriting historical evidence.
- Test and qualification selection is impact-based. A new commit SHA alone
  does not require a full rerun. Large SCC arrays, broad platform matrices, and
  full native profiles require a material affected-surface reason, a benchmark
  or release question, risk that focused checks cannot bound, or an explicit
  owner request.
- Exact-source receipts remain immutable, but unaffected claims may be carried
  forward through a recorded compatibility review that identifies both source
  states, the changed paths, the relevant unchanged production/build/binary/
  input/acceptance identities, focused checks, reused claims, and limitations.
- Hard correctness still includes the same estimand/sample/target semantics,
  finite results, identification and accounting, complete original-system
  residuals, no hidden regularization or post-RNG estimator fallback, direct
  memory admission, typed failure/UserBreak, and caller-state/lifecycle
  restoration.
- A candidate is speed-competitive when its registered median complete-command
  time is no slower than maintained MATLAB on the compatible comparison. The
  development target remains approximately one-half MATLAB time. Kernel-only
  or solver-only wins are diagnostic rather than sufficient.

## Package identity

- `fevc` is the only public command, help topic, package manifest, and
  installed package identity.
- No predecessor-command compatibility wrapper is installed.
- Distributed Mata/runtime filenames use `fevc*`; private Stata helpers use
  `_fevc_*`. Internal Mata symbols, plugin scalars, protocol/build IDs,
  environment variables, and Rust crate names retain their established
  `vckss__*`, `__vckss_*`, `VCKSS_*`, and `vckss-*` identities.

## Backend consent and defaults

- The alpha contract makes omitted `backend()` and `backend(auto)` prefer Rust
  only after a complete effective-request capability preflight.
- Missing runtime or a structurally unsupported tuple may fall back to Mata
  before native preparation and estimator RNG. Stale/corrupt receipts and all
  later failures fail closed.
- `backend(rust)` is strict and `backend(mata)` is explicit Mata.
- Omitted `rng()` and `rng(auto)` select Counter-V1 for Rust or Stata RNG for
  Mata. `rng(counter_v1)` is strict Rust consent; `rng(stata)` selects Mata and
  conflicts with strict Rust.
- Omitted `algorithm()` selects MATLAB-like JLA with 200 probes. Explicit
  `algorithm(auto)` retains exact-small/JLA-large structural planning.

## Automatic planning

- Algorithm, engine, solver route, fallback, batch width, memory, and wall
  planning resolve from the materialized retained structure before estimator
  RNG.
- The V7 execution plan is frozen. A later memory, rank, setup, convergence, or
  numerical failure cannot trigger a different estimator or engine.
- Explicit CMG fails closed. Automatic CMG-to-diagonal fallback is allowed only
  before RNG and must be recorded.
- Exact plans use no iterative route, probe batch, or estimator counter range.

## Scientific and numerical acceptance

- Point estimation remains the default; only an explicit accepted
  exact-observation component-inference request posts `e(V)`. Probe dispersion
  has no econometric interpretation.
- Omitted `deletion()` on a `project()` request therefore remains match
  deletion; projection inference never silently changes that default to
  observation deletion. Explicit observation deletion remains supported.
- Match deletion defaults to the maintained-MATLAB combined population:
  declared match blocks for movers and physical-observation deletion for
  eligible attached stayers. `stayers(movers)` is the mover-only opt-out.
- Frequency weights are literal positive integer copies; explicit target
  weights are stored-row target mass.
- Coefficient cells, deletion units, and exact target-scale strata remain
  separate.
- Every accepted RHS passes the complete original-system residual gate, target
  accounting identities, rank/estimability gates, direct-memory admission, and
  finite-output checks.
- Omitted `tolerance()` uses `1e-10` for fit/deterministic solves and `1e-6`
  for randomized probes. An explicit value overrides both; effective phase
  tolerances and residual gates are receipted.
- No hidden regularization or silent change of sample, target, deletion,
  nuisance, algorithm, explicit tolerance, probe count, or route is permitted.

## Results and failure behavior

- Request, selection, execution-plan, memory, residual, counter, and
  caller-state receipts are part of the development contract.
- A result family is posted only by its matching poster: exact, compressed JLA,
  or generic JLA.
- Unsupported, inconsistent, unavailable, singular, unconverged, or
  resource-inadmissible states fail with a typed status. They do not fall back
  to a scientifically different command.

## Fixed-offset collapsed-match inference

- The explicit match-deletion component-inference family uses
  `nuisance(fixedoffset)` and holds fixed the full-sample estimated control
  offset. It does not estimate or add uncertainty from `gamma_hat`; joint-
  nuisance match inference is a separate future method.
- Within a declared match, the FE design row is constant. Conditional on the
  offset, regression mass `F_g`, the weighted offset-outcome mean, cell
  identity, and separately aggregated target mass are exact scalar sufficient
  statistics for the FE fit, component plug-ins, whole-match deletion,
  adjusted residual contraction, point correction, and component kernel.
- The inferential observations are declared matches, not physical rows or
  frequency-weight copies. Aggregate match errors may have unrestricted
  within-match covariance; declared matches are assumed independent.
- The primary match-variance model uses normalized midranks of match leverage,
  the three primitive collapsed target diagonals, and total match regression
  mass, with intercept, linear, square, and pairwise terms. The sensitivity
  fit uses only intercept, leverage, and leverage squared. No additional
  duration/support feature is admitted in the first model version.
- Cross-fitting the structured match-variance regression is a regularization
  device. It is not an independent-sample construction and does not turn the
  method into unrestricted-KSS inference.
- The grouped route is public only through the explicit match tuple listed
  above. Independent match q0 and repaired eligible q1 confirmations and
  macOS arm64/Rosetta native/install qualification pass; see
  `FIXED_OFFSET_MATCH_INTERFACE_2026-09-05.md`. It is never selected
  automatically, and the observation-only native entrypoint is unchanged.
- The first registered grouped q1 development campaign is a preserved
  scientific failure. Its exact execution and inventory complete, but the
  equal-mass structured-common and leverage-only cells miss the 0.98 success
  gate and the structured-common worker target modestly exceeds its frozen
  coverage ceiling among successful fits. Thresholds, targets, and failed
  attempts are not changed or conditioned away. That failure blocked the
  original development source. The separately registered repair and later
  passing confirmations supersede that development restriction, not its FAIL
  result. No new campaign is part of RC finalization.

## CMG ownership

- CMG is a package component, not a shared library or independent release.
- Its deterministic generator produces only the shipped `vckss_cmg` runtime
  and the checked-in `cmgtest` target.
- CMG API 8 and generator API 5 are ownership/interface successors over the
  numerically qualified API 6 core.

## Evidence and release

- Historical reports, receipts, reviews, logs, manifests, and benchmark outputs
  are immutable source-bound evidence.
- Quick CI does not qualify the native plugin. Native claims require the
  source-local plugin profile and an exact-SHA receipt.
- Advisory timing or headroom misses do not withhold a scientifically and
  directly memory-safe command.
- GPL-3.0-only governs covered code. The human package-boundary,
  corresponding-source, notice, provenance, and data-exclusion review was
  completed on 2026-08-29. Public tagging and release remain separate owner
  decisions, with a fresh exact-artifact check required before conveyance.
