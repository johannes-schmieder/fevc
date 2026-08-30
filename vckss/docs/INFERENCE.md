# Exact-observation inference

## Scope

VCkss 0.5.0-alpha.1 adds opt-in econometric inference to the deterministic
Mata exact route. Point estimation remains the default and continues to post
no `e(V)`. The initial inference surface requires:

- `deletion(observation)`;
- `algorithm(exact)` or an omitted/automatic algorithm that resolves to
  exact;
- the Mata backend and guarded Stata RNG;
- `stayers(movers)`; and
- unit frequency weights.

Match-cluster component inference, literal-copy frequency-weight inference,
randomized JLA component inference, and Rust component inference remain
withheld. A separate, explicit scalable Rust/JLA capability is available only
for `project()` under unit-frequency observation deletion. These restrictions
prevent the command from silently changing the dependence model, deletion
unit, or randomized approximation.

The implementation follows the high-rank and rank-one procedures described by
Kline, Saggio, and Sølvsten (2020) and the maintained MATLAB package's
observable behavior. It is repository-authored GPL-3.0-only source. No MATLAB
source, critical-value table, or binary data are included.

## Variance proxy and smoothing

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
targets, VCkss evaluates the KSS high-rank variance approximation

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
vckss wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) inference(highrank)
```

`e(component_inference)` contains the four estimates, standard errors, and
ordinary Wald interval endpoints.

Because accepted component inference posts matching coefficient names in
`e(b)` and a coherent covariance in `e(V)`, Stata's standard `lincom` command
works without a VCkss-specific wrapper. For example,

```stata
lincom worker_variance + firm_variance + 2*worker_firm_covariance
```

reproduces the posted `total_variance` estimate and standard error. This is
distinct from the maintained MATLAB function `lincom_KSS`, whose purpose is
fixed-effect projection inference and whose VCkss counterpart is `project()`.

## Rank-one weak-identification intervals

`inference(q1)` retains the high-rank covariance and additionally isolates the
largest absolute eigenvalue of
\(H^{-1/2}QH^{-1/2}\) for each target. It reports the dominant eigenvalue,
its squared spectral share, the maximum squared observation-mode weight, the
rank-one covariance terms, the \(F\) diagnostic, curvature, and the simulated
critical value.

For curvature \(\kappa>0\), the q=1 critical-value draw is

\[
\sqrt{\chi_1^2+(\widetilde\chi_1+1/\kappa)^2}-1/\kappa,
\]

with independent square roots of chi-squared-one variables. The
\(\kappa\rightarrow0\) limit is \(\chi_1\), while the large-curvature limit is
\(\chi_2\). VCkss simulates at least 100,000 critical-value draws and maps the
resulting two-dimensional confidence ellipsoid through the rank-one quadratic
by a global angular grid followed by bounded refinement. It does not ship or
interpolate the maintained MATLAB package's unlicensed critical-value table.

```stata
vckss wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) inference(q1) level(95)
```

The Anderson--Rubin-style endpoints and all diagnostics are stored in
`e(q1_inference)`. The command withholds singular or indefinite rank-one
covariance estimates rather than substituting a Wald interval.

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

For this linear-projection calculation the KSS observation-level variance
proxy follows the maintained projection procedure and is
\((y_i-\bar y)\widehat e_{i,-i}\). Writing
\(L=G'\Omega Z(Z'\Omega Z)^{-1}\) and
\(S=XH^{-1}L\), the KSS covariance is

\[
\widehat V_\gamma=S'\operatorname{diag}
\{(y_i-\bar y)\widehat e_{i,-i}\}S.
\]

A residual-squared plug-in covariance is returned separately as a descriptive
naive comparison. Projection inference does not populate the component
`e(V)` unless `inference(highrank|q1)` is also requested.

```stata
vckss wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) project(education experience)    ///
    projecteffect(firm) projectweight(frequency)
```

The projection coefficients, KSS covariance, naive covariance, and formatted
coefficient table are stored in `e(projection_b)`, `e(projection_V)`,
`e(projection_V_naive)`, and `e(projection_results)`.
They are intentionally separate from component `e(b)` and `e(V)`, so Stata's
standard `lincom` does not operate on projection rows directly.

### Scalable sparse route

The exact Mata implementation remains the default projection oracle. It
materializes the retained observation-by-parameter fixed-effect design and a
full dense inverse, and is therefore governed by `exact_limit()`. The scalable
route is opt-in and requires the complete tuple

```stata
vckss wage controls, worker(worker_id) firm(firm_id)              ///
    deletion(observation) project(education experience)           ///
    projecteffect(firm) backend(rust) rng(counter_v1)              ///
    algorithm(jla) engine(generic) preconditioner(diagonal)
```

The first capability is intentionally narrow: mover-only inference, unit
frequency weights, observation deletion, the Rust backend, Counter-V1, generic
JLA with explicit diagonal PCG, and either frequency or target projection
mass. CMG and automatic solver routing are not part of this first projection
qualification. Other `project()` calls retain exact behavior or fail their
explicit strict request; they are never silently reinterpreted as the sparse
route.

The native runtime reuses the prepared generic-JLA solver and its retained
canonical observation map. It:

1. takes the observation variance proxy from the completed JLA inference
   calculation, namely
   `(y_i - mean(y)) * deleted_adjusted_i`;
2. reuses the full-model fixed-effect solve already required by JLA;
3. constructs only the small projection Gram and coefficient-space loading
   vectors, then performs one solver inverse action per automatic-constant or
   supplied projection column; and
4. streams observation scores into the KSS and residual-squared covariance
   accumulators without retaining an `n`-by-parameter design or an `n`-by-`q`
   score matrix.

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
Mata oracle for both firm/frequency and worker/target projections. The
committed 1,002-observation maintained-MATLAB fixture is exercised by
`qualification/inference_matlab/vckss_scalable_projection.do`: coefficient
solves must agree with exact VCkss, the complete covariance must lie within a
registered deterministic JLA tolerance of the exact oracle, and the reported
`z1`/`z2` standard errors must remain within the registered Monte Carlo band
around maintained `lincom_KSS`.

This adds no large-data performance claim. The
[focused scaling design](../qualification/inference_matlab/SCALABLE_PROJECTION.md)
is to be run as separate, source-bound VCkss and MATLAB processes so wall time
and peak RSS cover MATLAB's JLA-plus-`lincom_KSS` path rather than only
`lincom_KSS`.

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

The procedures estimate heteroskedastic sampling uncertainty under the KSS
observation-deletion assumptions. They do not provide match-cluster robust
inference. On the scalable projection route, JLA approximates the leverage and
leave-out residual used by the KSS variance proxy; probe dispersion itself is
not reported as an econometric standard error. The binned local-linear
calculation for component inference is the maintained-MATLAB-compatible
high-rank approximation; it is not the separately derived fully unbiased
leave-three-out variance estimator.

The source-bound comparison in
[`qualification/inference_matlab/`](../qualification/inference_matlab/)
separates exact projection validation from descriptive component-SE evidence.
The maintained MATLAB interface returns only three marginal component standard
errors, not the joint covariance or total-target uncertainty. It also retains
materially negative local-fit predictions that VCkss rejects, so its component
standard errors are not treated as a parity gate.
