# Inference

## Scope

FEVC provides opt-in econometric component inference through two
separately identified families. Point estimation remains the default and
continues to post no `e(V)`. Omitting `inferencemodel()` uses the deterministic
Mata exact target-specific family and requires:

- `deletion(observation)`;
- `algorithm(exact)` or an omitted/automatic algorithm that resolves to
  exact;
- the Mata backend and guarded Stata RNG;
- `stayers(movers)`; and
- unit frequency weights.

The explicit fixed-offset match route below also permits positive integer
frequency mass; observation component inference remains unit-frequency.
The supported explicit Rust generic-JLA attachment
implements matrix-free `q=0` and eligible one-mode `q=1` component inference
with a named structured variance model. The current development candidate
uses one residual-moment fitter for observation and match deletion.
It is selected only by
`inferencemodel(structured_common|structured_leverage)` together with the
explicit Rust/JLA/Counter-V1 observation or fixed-offset match tuple. Omitting
`inferencemodel()` preserves the exact Mata target-specific smoother. The
paper's unrestricted KSS variance-product construction remains unimplemented
and has no reserved FEVC option token; see
[`MATRIX_FREE_COMPONENT_INFERENCE.md`](MATRIX_FREE_COMPONENT_INFERENCE.md).
A separate, explicit scalable Rust/JLA capability is
available for `project()` under observation or match deletion and supports
positive integer frequency weights as literal physical copies. These restrictions
prevent the command from silently changing the dependence model, deletion
unit, or randomized approximation.

The implementation follows the high-rank and rank-one procedures described by
Kline, Saggio, and Sølvsten (2020) and the maintained MATLAB package's
observable behavior. It is repository-authored GPL-3.0-only source. No MATLAB
source, critical-value table, or binary data are included.

## Variance proxy and smoothing

There are two distinct implemented variance-model families. The default exact
Mata family below is target-specific and MATLAB-compatible. It is not the
paper's unrestricted heteroskedastic variance-product construction. The
explicit Rust family fits one common positive variance vector. For either
deletion unit, the new candidate solves the residual-moment equations
`Z'[(I-P) ◦ (I-P)]Z gamma = Z'e²`, using 2,048 Gaussian probes by default for
the small Gram matrix: one half the centered sample covariance of
`Z'[(g-Pg)^2]`, with denominator `R-1`. This direct residual representation
avoids an additional subtractive estimated-leverage term. Estimated JLA leverages and a fit-only positivity floor
mean this implementation is not an exactly unbiased variance estimator.
It uses no ridge, cross-fitting or fallback model. Observation rows use the
full-model projection; match rows use weighted, fixed-offset aggregates and
their FE-only projection. Residuals are aggregated before squaring. The match
common model adds regression mass to the predictor set.
`structured_common` uses normalized midranks of leverage and all
three primitive target diagonals with squares and pairwise interactions;
`structured_leverage` uses leverage and its square as a sensitivity model.
Outcome-free redundant columns are removed without changing the variance-model
span; genuine weak identification still fails the existing rank gates. The
support requirement counts independent units per active term. Full details,
positivity and support failures, and diagnostics are in
[`INDIVIDUAL_INFERENCE_INTERFACE.md`](INDIVIDUAL_INFERENCE_INTERFACE.md) and
[`MATRIX_FREE_COMPONENT_INFERENCE.md`](MATRIX_FREE_COMPONENT_INFERENCE.md).
The structured model is an additional statistical assumption, not an
unqualified heteroskedasticity-robust construction. V5's severe omitted-driver
fixtures produced visibly invalid intervals even though the established
component point estimator was unchanged. Cross-fitting and agreement with the
leverage-only sensitivity fit cannot detect a driver omitted from both models.
The historical V5 confirmation passed the registered correct-model and mild-
misspecification gates on its own source; its scope and limits are recorded in
[`structured_inference_confirmation_v5_result.json`](structured_inference_confirmation_v5_result.json).
It does not qualify the later q1 repair. The corrected observation confirmation
has one failed SE-ratio gate (1.101204 versus 1.10), an unresolved calibration
limitation recorded in `RC_OBSERVATION_CONFIRMATION_2026-09-05.md`.

The runnable candidate retains **200 JLA probes by default**. The separate
`inferencegramprobes(#)` option sets Gram precision for this explicit structured
Rust tuple: integers 512 through 2,147,483,647, default 2,048, subject to memory
and count admission. Unsupported explicit use is rejected before RNG. Counts
below 2,048 are a lower-precision speed tradeoff, not recommended for reported
inference. `e(inference_gram_method)` is `direct_residual_covariance`.
Changing this count leaves point, covariance, spectral and critical-value RNG
addresses unchanged. A few preselected inference seeds provide a numerical
sensitivity check; do not select a seed or q to obtain preferred intervals.
Larger Gram budgets can reduce numerical error, but cannot ensure coverage.

The [completion scope](inference_completion_v1.json) freezes this approximate
candidate without another coverage campaign. The archived 1,600-call direct
2,048-probe assessment improves numerical stability but retains three calibration
screen failures. Exact-Gram diagnosis resolves two, with the observation-controls
firm case still outside its historical screen. Severe variance-model
misspecification and estimated fixed offsets remain substantive limitations.
Old confirmation passes and failures remain source-specific; neither those
passes nor the new engineering replays establish universal KSS coverage.

### Individual intervals and joint covariance

Native result V5 checks each target separately. A materially indefinite joint
matrix no longer discards otherwise computable individual intervals. It is
withheld, not projected onto the PSD cone. `e(q0_status)` and, for q1,
`e(q1_status)` distinguish computed from unavailable intervals;
`e(inference_joint_status)` reports joint admissibility. For q0, `e(V)` is
posted only if the joint matrix and all four target checks pass. For q1 no
Gaussian `e(V)` is posted; use the reported q1 intervals. Shared fit, input and
solver failures remain atomic. Numerical availability alone is not evidence
of coverage, a diffuse q0 spectrum, or a one-mode q1 remainder.

### Explicit fixed-offset match inference

The separate public match tuple requires all of:

```text
backend(rust) rng(counter_v1) algorithm(jla) engine(generic)
deletion(match) nuisance(fixedoffset) stayers(movers)
preconditioner(diagonal|cmg)
inference(highrank|q1)
inferencemodel(structured_common|structured_leverage)
```

It estimates the full joint model once, forms `y_star` with `gamma_hat`,
and holds that offset fixed in an FE-only whole-match calculation. Regression
mass and the weighted offset-outcome mean reduce each physical match block
to one exact scalar sufficient row. One Gaussian inference draw is generated
per declared match, irrespective of frequency mass. Different `deletionid()`
values remain separate units even at the same worker--firm coordinate.
Target mass retains its existing stored-row meaning.

The working model permits unrestricted covariance inside an original match
and assumes independence across declared matches, including different matches
of the same worker. The primary structured aggregate-match variance model
adds normalized match-mass midrank to the leverage and primitive-target
features (21 candidate terms); the leverage-only sensitivity retains three
candidate terms. Design-equivalent matches stay in the same outcome-free
variance-regression fold. Severe omitted aggregate-variance drivers can
invalidate the reported uncertainty.

**Required interpretation: Fixed-offset approximate match inference, ignoring
nuisance-control estimation uncertainty.** Same-sample estimation of controls
can induce cross-match dependence; few controls do not guarantee negligible
uncertainty or conditional validity. There is no delta-method, influence,
cross-fitted FE-model, joint-nuisance or second-stage correction. See the
[controls diagnosis](FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md).

Independent q0 and repaired eligible q1 confirmations pass in their declared
fixed-offset regimes:
[match q0](RC_MATCH_Q0_CONFIRMATION_2026-09-05.md) and
[match q1](INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md).
Their estimated-controls cases are calibration limitations, not coverage
claims. Historical development failures remain unchanged. The
[owner-approved interface decision](fixed_offset_match_interface_v1.json)
permits this integration while retaining the corrected observation-q1
confirmation's failed SE-ratio gate as an unresolved RC limitation; it does
not turn that failure into a pass.

The grouped q1 leading square uses the raw leave-match product
`sum_g v_1g^2 y_g_c ehat_g,-g,c`; positive modeled variances are used only
for covariance and studentization. Independent physical-block and collapsed
oracles protect this identity. The interval uses the existing corrected
KSS/Andrews--Mikusheva ellipse-image calculation. A computed target is covered
by the intended q1 claim only if it has one dominant mode and a diffuse
remainder; covariance targets can be multi-mode. No automatic q selection or
universal concentration cutoff is applied.

The public V1 unit receipt reports independent/effective match counts,
largest regression-mass share, match leverage, minimum maker denominator and
omitted nuisance uncertainty. Existing spectrum, influence, variance-fit,
PSD, solve and target-local q1 status diagnostics remain mandatory. Stata
reconciles the receipt with the prepared match count before posting.
Unsupported tuples and shared structural/numerical failures remain atomic;
an unavailable target's AM endpoints are missing, never replaced with q0
intervals. See [returns](FAILURES_AND_RETURNS.md) for the full schema.

For the retained exact design, let \(H=X'X\),
\(\widehat\beta=H^{-1}X'y\), \(P_{ii}=x_i'H^{-1}x_i\), and
\(\widehat e_{i,-i}=\widehat e_i/(1-P_{ii})\).

For a quadratic target \(Q\), define
\(B_{ii}=x_i'H^{-1}QH^{-1}x_i\). The high-rank calculation smooths the
maintained-procedure proxy
\(\widehat\sigma_{Q,i}^2=y_i\widehat e_{i,-i}\) over
\((P_{ii},B_{ii})\), separately for movers and
stayers. The implementation rank-bins each dimension, aggregates populated
joint cells, and applies a weighted local-linear tricube fit. `inferencebins()`
sets the maximum requested joint-cell resolution; the default is 1,000.

Materially negative fitted variances withhold the request. Tiny negative
roundoff values are set to zero and counted in
`e(inference_diagnostics)[1,"variance_floor_count"]`.

## High-rank component covariance

For each of the worker variance, firm variance, and worker--firm covariance
targets, FEVC evaluates the KSS high-rank variance approximation

\[
\widehat V_Q = 4\sum_i W_{Q,i}^2\widetilde\sigma_{Q,i}^2
              - \widehat{\operatorname{Var}}(\widehat\theta_Q^*).
\]

The first term uses the target-specific influence weights. The second is the
variance of the same corrected quadratic form on Gaussian draws with fitted
observation variances. `inferencesimulations()` controls this numerical
approximation and defaults to 1,000. This simulation error is computational;
it is not a second sampling uncertainty estimate.

The three-by-three primitive covariance is assembled by polarization. Each
pair applies the same scalar procedure to the summed target, using the same
standardized simulation draws. A fixed linear map then adds the fourth target,

\[
\theta_T=\theta_\alpha+\theta_\psi+2\theta_{\alpha\psi},
\]

so `e(V)` is a coherent four-by-four covariance matrix and the corresponding
identity holds exactly up to roundoff. A materially indefinite covariance is
withheld. Only a negative eigenvalue within the registered numerical tolerance
may be set to zero, with the adjustment recorded as `psd_cleanup`.

Request this calculation with

```stata
fevc wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) inference(highrank)
```

`e(component_inference)` contains the four estimates, standard errors, and
ordinary Wald interval endpoints.

For the structured Rust `q=0` mode, strong identification requires the leading
generalized spectral contribution to vanish along the intended asymptotic
sequence and the linear-influence contribution to satisfy the corresponding
diffuseness/Lindeberg condition. FEVC reports both diagnostics but imposes no
universal cutoff. Selecting `inference(highrank)` or obtaining a numerical
result does not establish those target-specific conditions.

Because accepted component inference posts matching coefficient names in
`e(b)` and a coherent covariance in `e(V)`, Stata's standard `lincom` command
works without a FEVC-specific wrapper. For example,

```stata
lincom worker_variance + firm_variance + 2*worker_firm_covariance
```

reproduces the posted `total_variance` estimate and standard error. This is
distinct from the maintained MATLAB function `lincom_KSS`, whose purpose is
fixed-effect projection inference and whose FEVC counterpart is `project()`.

## Rank-one weak-identification intervals

`inference(q1)` retains the high-rank covariance and additionally isolates the
largest absolute eigenvalue of
\(H^{-1/2}QH^{-1/2}\) for each target. It reports the dominant eigenvalue,
its squared spectral share, the maximum squared observation-mode weight, the
rank-one covariance terms, the \(F\) diagnostic, curvature, and the simulated
critical value.

For observation-space leading mode \(v_1\), FEVC forms

\[
\widehat b_1=v_1'y,
\qquad
\widehat V_{b_1}^{\mathrm{LO}}
=\sum_i v_{1i}^2 y_i\widehat e_{i,-i},
\]

and removes
\(\lambda_1(\widehat b_1^2-\widehat V_{b_1}^{\mathrm{LO}})\)
from the unchanged leave-out component point estimate. This raw leave-out
product is not positivity-smoothed. In the structured Rust modes, the common
positive fitted variance vector separately estimates the joint covariance of
the leading score and Gaussian remainder. The implementation verifies that
the resulting remainder equals the direct rank-one-subtracted leave-out
quadratic form within the complete-system residual tolerance; a material
identity failure withholds inference.

For curvature \(\kappa>0\), the q=1 critical-value draw is

\[
\sqrt{\chi_1^2+(\widetilde\chi_1+1/\kappa)^2}-1/\kappa,
\]

with independent square roots of chi-squared-one variables. The
\(\kappa\rightarrow0\) limit is \(\chi_1\), while the large-curvature limit is
\(\chi_2\). FEVC simulates at least 100,000 critical-value draws and maps the
resulting two-dimensional confidence ellipsoid through the rank-one quadratic
by a global angular grid followed by bounded refinement. It does not ship or
interpolate the maintained MATLAB package's unlicensed critical-value table.
This critical value is based on the maximal-curvature circle and gives the KSS
uniform asymptotic guarantee of coverage at least at the nominal level. It is
not an exact finite-sample quantile of the shortest distance to a particular
parabola, so design-specific conservatism is expected and is not removed by an
empirical critical value.

```stata
fevc wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) inference(q1) level(95)
```

The Anderson--Rubin-style endpoints and all diagnostics are stored in
`e(q1_inference)`. The command withholds singular or indefinite rank-one
covariance estimates rather than substituting a Wald interval.

The confirmed `q=1` regime is target-specific: one leading mode is removed and
the remaining kernel and linear-influence contributions must be diffuse. A
target with several concentrated modes remains outside the coverage claim even
if the calculation succeeds; V5's deliberately multi-mode covariance target
is the canonical example. FEVC therefore reports leading and remainder shares,
maximum mode weight, and remainder-influence concentration without applying a
post-hoc cutoff or automatically changing `q`. The KSS/Andrews--Mikusheva
interval has an asymptotic at-least-nominal uniform coverage guarantee and may
be modestly conservative for a particular design.

## Fixed-effect projections

`project()` estimates a weighted projection of one fixed-effect dimension on
an automatic constant and supplied numeric variables. `projecteffect()`
selects worker or firm effects. `projectweight(frequency)` uses observation
mass and is the default; `projectweight(target)` uses `targetweight()` mass.

Let \(G\beta\) be the selected observation-level fixed effect,
\(Z=[\mathbf 1,Z_0]\), and \(\Omega\) the selected projection weights. The
reported coefficients are

\[
\widehat\gamma=(Z'\Omega Z)^{-1}Z'\Omega G\widehat\beta.
\]

Write \(L=G'\Omega Z(Z'\Omega Z)^{-1}\),
\(A=H^{-1}L\), and partition the physical observations into independent
deletion blocks. For block \(g\), the fit excluding the complete block gives

\[
\widehat e_{g,-g}=y_g-X_g\widehat\beta_{-g}.
\]

Because \(\widehat\beta_{-g}\) is a function only of the other independent
blocks, it is independent of \(\varepsilon_g\). Under
\(y_g=X_g\beta+\varepsilon_g\), conditional mean zero, and an unbiased
deleted fit,

\[
E[y_g\widehat e_{g,-g}'\mid X]=\Sigma_g.
\]

The raw cross product need not be symmetric in a realized sample, so FEVC
uses the auditable symmetric block estimate

\[
\widehat\Sigma_g=\tfrac12\{y_g\widehat e_{g,-g}'
                         +\widehat e_{g,-g}y_g'\}.
\]

Consequently,

\[
\operatorname{Var}(\widehat\gamma\mid X)
=L'H^{-1}\left\{\sum_g X_g'\Sigma_gX_g\right\}H^{-1}L,
\qquad
\widehat V_\gamma
=\sum_g (X_gA)'\widehat\Sigma_g(X_gA).
\]

This formula permits unrestricted covariance within a declared match and
independence across matches. It does not replace \(\Sigma_g\) by a diagonal
matrix. For observation deletion it reduces to the uncentered identity
\(y_i\widehat e_{i,-i}\). Subtracting a sample mean from \(y_i\) is not
valid under unrestricted heteroskedasticity and is not part of the FEVC
estimator. A separately isolated test diagnostic reproduces the centered
expression in the newer maintained MATLAB pipeline when comparator attribution
requires it. The original Econometrica `lincom_KSS` instead used this same
uncentered, symmetrized block identity.

For stored row \(i\) representing positive integer frequency \(f_i\), define
the score row \(s_i=x_i'A\). A mover-match block is accumulated without
physical expansion as

\[
a_g=\sum_{i\in g}f_i s_i y_i,\qquad
b_g=\sum_{i\in g}f_i s_i\widehat e_{i,-g},\qquad
\widehat V_{\gamma,g}=\tfrac12(a_gb_g'+b_ga_g').
\]

Each eligible stayer remains a literal observation-deletion population, so a
stored stayer row contributes \(f_i y_i\widehat e_{i,-i}s_is_i'\). This is
the same mover-match/eligible-stayer partition, retained sample, pooled target,
regression mass, target mass, and nuisance convention as the point estimator.
Every declared match deletion ID must stay within one worker--firm coordinate;
cross-coordinate IDs fail with `CROSS_COORDINATE_MATCH`.

A residual-squared plug-in covariance is returned separately as a descriptive
naive comparison. Projection inference does not populate the component
`e(V)` unless `inference(highrank|q1)` is also requested.

```stata
fevc wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) project(education experience)    ///
    projecteffect(firm) projectweight(frequency)
```

The projection coefficients, corrected block KSS covariance, naive covariance,
and formatted
coefficient table are stored in `e(projection_b)`, `e(projection_V)`,
`e(projection_V_naive)`, and `e(projection_results)`.
They are intentionally separate from component `e(b)` and `e(V)`, so Stata's
standard `lincom` does not operate on projection rows directly.

The two-way coefficient representation grounds the last retained firm after
solving on the full-firm zero-sum quotient. Under the observationally
equivalent shift \(\alpha_i\mapsto\alpha_i+c\),
\(\psi_j\mapsto\psi_j-c\), projection slopes are invariant. The automatic
intercept rises by \(c\) for worker projections and falls by \(c\) for firm
projections. FEVC therefore returns the intercept under its explicit
last-retained-firm-zero display normalization and labels it
normalization-dependent; it is not an invariant structural parameter.

### Scalable sparse route

The exact Mata implementation remains the default projection oracle. It
materializes the retained observation-by-parameter fixed-effect design and a
full dense inverse, and is therefore governed by `exact_limit()`. The scalable
route is opt-in and requires the complete tuple

```stata
fevc wage controls, worker(worker_id) firm(firm_id)              ///
    deletion(observation) project(education experience)           ///
    projecteffect(firm) backend(rust) rng(counter_v1)              ///
    algorithm(jla) engine(generic) preconditioner(cmg)
```

The capability remains intentionally explicit: positive integer frequency
weights, observation or match deletion, the Rust backend, Counter-V1, generic
JLA with explicit `preconditioner(diagonal)` or forced
`preconditioner(cmg)`, and either frequency or target projection mass.
Match deletion supports both the mover-only population and the default pooled
mover/eligible-stayer population.
Automatic solver routing is not admitted. Other `project()` calls retain exact
behavior or fail their explicit strict request; they are never silently
reinterpreted as the sparse route.

Forced projection CMG uses the planned generic model preconditioner. One
source-informed hierarchy is prepared before estimator RNG and shared by the
full W+F+Q solver and the fixed-effect-only solver. Controls are handled by the
certified residualized-control block, and every returned action is checked in
the complete weighted original system. This is not `CMG_FULL_V2`: that direct
hybrid implementation remains confined to its specialized no-control,
match-deletion point-estimation cell. Explicit projection CMG never falls back
to diagonal after selection.

Each stored row with frequency weight `f_i` contributes `f_i` times to the
fit, leverage variance proxy, projection Gram, score covariance, and physical-
frequency projection mean. This is algebraically equivalent to expanding that
row into `f_i` identical observations. Explicit target mass remains stored-row
mass and is not multiplied by frequency.

The native runtime reuses the prepared generic-JLA solver and its retained
canonical observation map. It:

1. takes the completed JLA block-deleted residual approximation and applies
   the same symmetrized uncentered block formula as exact Mata;
2. reuses the full-model fixed-effect solve already required by JLA;
3. constructs only the small projection Gram and coefficient-space loading
   vectors, then performs one solver inverse action per automatic-constant or
   supplied projection column; and
4. streams observation scores into observation or match-block covariance
   accumulators without retaining an `n`-by-parameter design or an
   `n`-by-`q` score matrix.

Preparation retains `O(pq + q^2)` projection state in addition to the sparse
solver state, where `p` is the identified fixed-effect dimension and `q` is
the number of projection columns including the automatic constant. Covariance
accumulation uses `O(q^2)` output storage. The synchronous augmentation
boundary temporarily holds both the C caller copy and Rust's validated copy of
the supplied `n`-by-(`q-1`) project variables. Both copies and the dense
`q`-square preparation work are admitted before coefficient-space preparation;
neither observation-level copy is retained.

The route fails closed unless all of the following reconcile across Rust, the
C boundary, and Stata:

- projection-Gram reciprocal condition and solve residuals;
- every projection loading's reduced and complete original-system residual,
  iteration count, and convergence status;
- finite symmetric KSS and naive covariance matrices, with only registered
  roundoff-scale PSD cleanup;
- JLA variance-proxy range and complete-system tolerance;
- exact RHS counts, result bytes, prepared resident bytes, augmentation peak,
  solve peak, and whole-command memory admission; and
- the public projection column names and existing `e(projection_*)` shapes.

The existing four public result matrices are unchanged. Additive diagnostic
returns are `e(projection_diagnostics)`,
`e(projection_augmentation_receipt)`, and
`e(projection_solver_diagnostics)`.

### Qualification boundary

The focused public-route test compares small sparse results with the dense
Mata oracle for both firm/frequency and worker/target projections, including
nonunit compressed weights against literal expansion. The
committed 1,002-observation maintained-MATLAB fixture is exercised by
the archived maintained-MATLAB projection fixture: coefficient
solves must agree with exact FEVC, the complete covariance must lie within a
registered deterministic JLA tolerance of the exact oracle, and the reported
`z1`/`z2` standard errors must remain within the registered Monte Carlo band
around maintained `lincom_KSS`.

The weighted 1,002-row maintained-MATLAB oracle agrees with compressed exact
FEVC to `1.78e-15` maximum absolute error under literal expansion, within
`4.39e-8` absolute for the independent dense same-formula calculation, and
within `1.17e-6` relative for official `lincom_KSS` standard errors. Rust with
4,000 Counter-V1 probes is within `0.2994%` of exact for the full covariance.

The
[focused scaling comparison](../../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference)
runs separate, source-bound FEVC and MATLAB processes so wall time and peak
RSS cover MATLAB's JLA-plus-`lincom_KSS` path rather than only `lincom_KSS`.
Source `96e7a66` passes all coefficient, covariance-diagonal, SE, residual, PSD,
and memory gates at 6,000 and 24,000 rows, but explicit diagonal PCG is not a
qualified large-data route: it becomes 18.97 times slower than MATLAB on the
24,000-row command and fails to converge at 96,000 rows. The strict harness
also rejects MATLAB's independently reconstructed 96,000-row grounded fit, so
no paired 96,000-row speed, covariance, or accepted RSS-growth result exists.

The forced-CMG composition was then exercised locally on the same deterministic
6,000-, 24,000-, and 96,000-row designs before public admission. It completed
all three cases with maximum projection complete residuals below `9e-11`, zero
PSD cleanup, worst projection-solve iteration counts of 17, 38, and 79, and
reported projection memory forecasts of 2.8, 10.1, and 39.6 MB. On the
6,000-row common Counter-V1 fixture, forced CMG and diagonal PCG differed by at
most `2.28e-11` across projection coefficients and covariance entries. These
local observations establish convergence and formula-path invariance, not a
same-host MATLAB speed comparison or new cross-platform performance claim.

## RNG and failure behavior

`inferenceseed()` affects only the numerical high-rank and q=1 simulations.
The outer command restores the caller's RNG algorithm, stream, and complete
state on success and every supported failure. Repeating the same accepted
request with the same source, data, options, and inference seed is
deterministic.

Requested inference is atomic with the point estimate: if smoothing,
covariance, eigen, projection, or runtime validation fails, no partial
estimation result is posted. Typed failure statuses and the stored-return
contract are listed in [`FAILURES_AND_RETURNS.md`](FAILURES_AND_RETURNS.md).

## Interpretation boundary

Component procedures estimate sampling uncertainty under their explicitly
selected target-specific or structured variance model; neither implemented
component family is the paper's unrestricted variance-product construction.
Projection inference additionally
supports independent match blocks with unrestricted within-match covariance,
including the default mixed mover-match/stayer-observation population. On the
scalable projection route, JLA approximates the leverage and block-deleted
residual used by the covariance estimator; probe dispersion itself is not
reported as an econometric standard error. The binned local-linear
calculation for component inference is the maintained-MATLAB-compatible
high-rank approximation; it is not the separately derived fully unbiased
leave-three-out variance estimator.

The source-bound comparison in
[archived inference/MATLAB qualification record](../../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference)
separates exact projection validation from descriptive component-SE evidence.
The maintained MATLAB interface returns only three marginal component standard
errors, not the joint covariance or total-target uncertainty. It also retains
materially negative local-fit predictions that FEVC rejects, so its component
standard errors are not treated as a parity gate.
