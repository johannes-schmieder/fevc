# Matrix-free component inference

## Status and authority

This document is the active scientific and implementation contract for the
Rust generic-JLA component inference. The observation-deletion attachment is a
supported explicit capability through
`inferencemodel(structured_common|structured_leverage)`. It is never selected
automatically. Omitting `inferencemodel()` preserves the independent dense
exact Mata target-specific implementation for `inference(highrank)` and
`inference(q1)`. No existing request is silently redirected to a different
variance model or reference distribution. FEVC does not reserve or accept an
`unrestricted_kss` parser token: implementing the paper's unrestricted
variance-product construction would require a separate future scientific and
public-interface decision.

The statistical reference is Kline, Saggio, and Sølvsten (2020, henceforth
KSS), especially Sections 5--7 and the supplement. The maintained
`LeaveOutTwoWay` MATLAB repository and the package-owned exact Mata code are
implementation comparators, not substitutes for the paper's statistical
contract. Source identities are recorded in `SOURCE_PROVENANCE.md`.

The three dimensions below are independent and must remain explicit:

1. deletion unit: observation or worker--firm match;
2. error-variance model: the paper's unrestricted KSS construction, the exact
   Mata target-specific smoother, or a named structured FEVC model;
3. reference distribution: `q=0` Gaussian approximation or `q=1` with one
   dominant generalized eigenmode handled explicitly.

In particular, `q=1` does not mean match deletion.

## Estimand and unchanged point estimator

For each symmetric coefficient-space target `Q_t`, the estimand is
`theta_t=beta'Q_t beta`. The primitive targets are worker variance, firm
variance, and worker--firm covariance. The fourth target is always the exact
map

```text
worker + firm + 2 worker--firm covariance.
```

It is never estimated, smoothed, or simulated as a fourth primitive. The
component point estimator remains the existing leave-out estimator. A
variance model changes only covariance estimates, standard errors, and
confidence sets. Rust tests require the generic-JLA point result to be
bitwise unchanged when the private attachment is present.

## Three distinct variance-estimation constructions

### Unrestricted KSS

The strict unrestricted-heteroskedastic construction is the one in KSS
Section 5. It is not the LOWESS procedure in the MATLAB code. For independent
observations it constructs two unbiased predictions of `x_i'beta`, both
independent of `y_i` and supported on disjoint observations, and uses them to
estimate `sigma_i^2`. Products `sigma_i^2 sigma_l^2` require additional pair
leave-outs. When the complete construction is unavailable, the paper's stated
fallback can be conservative only under its additional conditions; such a
result must be labeled as a fallback and report the affected-pair share.

For a worker--firm graph this contract requires more than ordinary leave-one-
out connectedness. Split predictions require the applicable edge-disjoint
paths/full-rank-after-two-deletions condition; the no-fallback product
construction requires the stronger after-three-deletions condition. The
method must report failures of split prediction, bad variance-product pairs,
and whether the exact or conservative construction was used.

Only this construction may be called `unrestricted KSS`. It is not implemented
in the Rust attachment, is not an accepted option name, and is not on the
current implementation path. The derivation is retained here to prevent the
structured extension from inheriting the paper's stronger claim by shorthand.

### Target-specific LOWESS comparator

The maintained and legacy KSS MATLAB code smooths the raw leave-out proxy

```text
s_i = y_i e_{i,-i}
```

on leverage and a target-specific target diagonal. The current exact Mata
implementation follows that target-specific pattern, with package-owned
binning, local-linear, positivity, and failure rules. It refits the variance
model for primitive and polarized targets. Consequently it is neither the
paper's unrestricted construction nor a single common observation-variance
model. It remains valuable as a compatibility comparator and dense oracle,
but it is not the primary scalable joint-covariance contract.

### Structured common FEVC model

The recommended pragmatic scalable mode is a common observation-level model,
identified publicly as `structured_common`. It uses the same
positive fitted variance vector for every component target. It is an FEVC
extension and must never be described as the unrestricted KSS estimator.

The proposed primary specification is precise enough to implement and test:

- response: `s_i=y_i e_{i,-i}` from observation deletion;
- primary design diagnostics: leverage and the three primitive target
  diagonals, all computed without outcomes;
- sensitivity design: leverage alone;
- transform: deterministic mid-ranks in the full design population; exact
  diagnostic ties share a rank, and outcome-free ranks may be shared across
  folds;
- basis: an intercept, every linear term, every square, and every pairwise
  interaction of the transformed diagnostics (15 columns for the primary
  specification and 3 for leverage-only);
- cross-fitting: five deterministic folds addressed by a dedicated
  Counter-V1 variance-fold domain and the canonical semantic observation key;
  only the variance regression is cross-fitted--the worker--firm model is
  never refit by folds;
- fit: training-fold ridge least squares with an unpenalized intercept. The
  penalty is chosen separately inside each outer training set by four-fold
  semantic-fold validation over
  `lambda/mean(diag(Z_c'Z_c)) in {0,1e-8,1e-6,1e-4,1e-2,1,1e2}`; singular
  zero-penalty candidates are discarded and an exact validation tie chooses
  the smaller penalty. The selected penalty, validation loss curve, and
  unregularized and regularized condition receipts are reported;
- positivity: predictions below
  `1e-8 * median(e_i^2/(1-h_i))` in the training fold are raised to that
  strictly positive floor; zero/nonfinite scale is a hard failure;
- support: every outer and inner training fold must contain at least five
  observations per active basis column and have finite rank/condition
  receipts. The maximum
  nearest-training distance in rank space, coordinatewise boundary hits, and
  leverage-only versus primary prediction discrepancies are reported. A
  nonfinite query, empty active basis, singular intercept, failed penalty
  grid, or nonpositive variance scale is a hard failure. The floor count and
  share are credibility diagnostics rather than an undocumented automatic
  cutoff; no silent target-specific fallback is allowed;
- determinism: semantic folds, rank ties, linear algebra order, floor rules,
  and diagnostic aggregation are invariant to stored-row order, solver batch
  width, and scheduling.

The finite-dimensional conditional-mean restriction on `s_i` is a substantive
structured variance assumption. Cross-fitting limits own-response fitting
bias; it does not make the model unrestricted and does not repair global
misspecification. In particular, regression-fold exclusion alone does not make
the prediction for row `i` independent of `y_i`: a training response `s_j`
contains a leave-`j`-out residual that can depend on `y_i`. Eliminating that
indirect dependence would require a different fold-level leave-out object and
is not implied by the phrase cross-fitting. The proposed contract therefore
claims regularization against direct own-response fitting, not KSS-style
sample-split independence. The primary four-diagnostic model is defensible as
a pragmatic specification because it conditions on every design-only quantity
that enters the three primitive leave-out corrections while preserving one
common variance vector. The leverage-only fit is therefore a useful
sensitivity analysis, not the primary specification or a specification test.
Agreement between the two fits cannot detect a variance driver omitted from
both conditioning sets. Coverage and misspecification experiments are
qualification evidence for this structured mode; its public name does not
upgrade it to unrestricted KSS inference. Target-specific LOWESS stays with
the exact Mata comparator. Strict unrestricted KSS remains a documented
distinction, not a promised FEVC mode.

## Observation-deletion matrix identities

Let `H=X'X`, `P=XH^{-1}X'`, `M=I-P`, and
`B_t=XH^{-1}Q_tH^{-1}X'`. Reusing the generic-JLA leverage multiplier and
target diagonal gives

```text
r[t,i] = B[t,i,i] / M[i,i]
C_t = B_t - (diag(r_t) M + M diag(r_t))/2.
```

The diagonal of `C_t` is zero. Its quadratic form is exactly the unchanged
observation-deletion point estimator. Each influence vector `g_t=C_t y` uses
one combined target-specific inverse action through the prepared full
worker--firm--control solver.

For each covariance probe, one common `z ~ N(0,V)` is solved once and all three
primitive scalars are evaluated as

```text
q_t = u' Q_t u - e' diag(r_t) z = z' C_t z.
```

Online joint moments estimate
`Cov(q_t,q_s)=2 tr(V C_t V C_s)`. No observation-by-probe matrix is retained.
The primitive covariance is

```text
Omega[t,s] = 4 g_t' V g_s - Cov(q_t,q_s).
```

The fourth row and column are derived by the exact three-to-four linear map,
so the reported four-by-four covariance is coherently singular.

## Spectral diagnostics and reference distributions

For target `t`, let

```text
A_t = H^{-1/2} Q_t H^{-1/2}
```

with eigenvalues ordered by decreasing absolute value. Operational results
must expose, without imposing a universal statistical cutoff:

- `lambda_1` and `lambda_2`;
- `lambda_1^2 / sum_j lambda_j^2` (leading concentration);
- for `q=1`, `lambda_2^2 / sum_{j>=2} lambda_j^2` (remainder concentration);
- randomized trace-square estimate, its numerical MCSE, probe count, and the
  implied ratio uncertainty;
- generalized-eigen residuals for the first two modes;
- `max_i v_1,i^2` after normalizing the observation-space leading mode to unit
  norm;
- the largest observation share of the linear-influence variance, using the
  exact three-to-four influence map for the sum target.

For `q=0`, KSS strong identification requires the leading concentration to
vanish asymptotically, plus a Lindeberg condition for the linear influence.
The command must report the concentration even when `q=0` was requested; a
request alone is not evidence that the Gaussian approximation is credible.

For `q=1`, one leading mode is removed explicitly:

```text
V_b1_hat_LO = sum_i v_1i^2 y_i e_hat_i,-i
theta_hat = lambda_1 (b_1_hat^2 - V_b1_hat_LO)
            + theta_1_hat + o_p(sd).
```

The remainder kernel and its diagonal are formed by subtracting
`lambda_1 v_1 v_1'`. The same common variance vector yields the joint
covariance of `(b_1_hat, theta_1_hat)` but does not replace the raw leave-out
product `V_b1_hat_LO` used to recenter the leading square. This distinction is
structural: using the positive modeled value `sum_i v_1i^2 sigma_hat_i^2` as
the realized recenter changes the remainder center while the remainder
influence and trace covariance still describe the rank-one-subtracted
leave-out kernel. FEVC verifies the exact remainder identity and fails closed
on a material discrepancy. The remainder receives the Gaussian
approximation only when its reported concentration is diffuse. Confidence
sets are the Andrews--Mikusheva image of the joint Gaussian covariance ellipse,
using a separate Counter-V1 critical-value domain and the same deterministic
ellipse-image calculation as the exact Mata implementation. The largest
observation share of the remainder's linear-influence variance is reported as
well. Numerical mode finding fails closed on nonconvergence, a material
generalized-eigen residual, an absent nonzero mode, or invalid joint
covariance. No hard-coded concentration threshold chooses between `q=0` and
`q=1`.

The q=1 critical radius must not be interpreted as an exact finite-sample
quantile of the shortest Mahalanobis distance from a Gaussian draw to the
specific transformed parabola. Following KSS and Andrews--Mikusheva, it is the
quantile of the distance to a circle determined by an upper bound on maximal
curvature. KSS Lemma 7 therefore establishes asymptotic coverage of at least
the nominal level, uniformly over the leading-score nuisance, rather than an
exact-size statement. Some conservatism for a particular design is expected;
replacing this radius by a design-calibrated empirical quantile would define a
different method and is not part of FEVC.

## Grouped match-deletion algebra

A match is not a cosmetic set of observation deletions. Partition rows into
matches `g`. With blocks `M_gg` and `B_t,gg`, whole-match deletion gives

```text
theta_hat_t = y' B_t y
              - sum_g y_g' B_t,gg M_gg^{-1} e_g.
```

The symmetric zero-block-diagonal kernel is

```text
C_t,gg = 0
C_t,gh = B_t,gh
         - 1/2 [B_t,gg M_gg^{-1} M_gh
                + (B_t,hh M_hh^{-1} M_hg)'],  g != h.
```

Equivalently, with block-diagonal
`R_t=blockdiag(B_t,gg M_gg^{-1})`,

```text
C_t = B_t - (R_t M + M R_t')/2.
```

This identity determines the grouped influence action and shows why retaining
only row diagonals is insufficient. A scalable implementation must retain or
stream the target blocks `B_t,gg`, apply `M_gg^{-1}` with the existing prepared
maker blocks, and verify the quadratic identity against the unchanged point
estimate.

### Fixed-offset scalar specialization

The selected first grouped route is narrower and admits an exact scalar
specialization. It requires `nuisance(fixedoffset)`: FEVC first estimates the
full joint model, fixes `gamma_hat`, and forms
`y_i_star = y_i - z_i' gamma_hat`. All inference below conditions on that
realized offset. It does not include sampling uncertainty from estimating
`gamma_hat`.

For declared match `g`, let `F_g=sum_(i in g) f_i`. Because every row in the
match has the same FE row `x_g`, define

```text
v_g[i] = sqrt(f_i / F_g)
y_g_c  = v_g' (sqrt(f_g) .* y_g_star)
       = sum_(i in g) f_i y_i_star / sqrt(F_g)
x_g_c  = sqrt(F_g) x_g.
```

Then the physical-row match blocks that enter component estimation have the
rank-one form

```text
P_gg   = h_g v_g v_g'
B_t,gg = b_tg v_g v_g'
h_g    = x_g_c H^{-1} x_g_c'
b_tg   = x_g_c H^{-1} Q_t H^{-1} x_g_c'.
```

Consequently `M_gg^{-1}` matters only through `v_g`:

```text
v_g' M_gg^{-1} e_g_c = e_g_c / (1-h_g),
e_g_c = sum_(i in g) f_i e_i / sqrt(F_g),
```

with the existing finite-JLA maker correction substituted for the scalar
reciprocal in approximate production runs. Within-match residual contrasts
orthogonal to `v_g` are annihilated by every `B_t,gg`. Thus the original block
point correction is exactly

```text
sum_g b_tg y_g_c ehat_g,-g,c.
```

The same reduction preserves the full FE normal equations, all component
plug-in targets, whole-match deletion, and the zero-block-diagonal kernel.
After collapse, the kernel has one row per declared match and

```text
C_t = B_t - {D(r_t) M + M D(r_t)} / 2,
r_tg = b_tg m_g,
m_g  = the scalar maker multiplier used by the point correction.
```

This is an algebraic reduction of genuine whole-match deletion, not a request
to delete physical observations independently. Distinct declared deletion IDs
at one worker--firm coordinate remain distinct scalar match clusters. A
deletion ID spanning coordinates still fails with `CROSS_COORDINATE_MATCH`.

Target mass does not merge with regression mass. `F_g` enters the collapsed FE
row, while stored-row target mass continues to define `Q_t`. A frequency
weight therefore changes regression mass but never creates `F_g` independent
inferential observations.

If matches are independent clusters with covariance blocks `Gamma_g`, then

```text
Omega[t,s]
  = 4 sum_g (C_t mu)_g' Gamma_g (C_s mu)_g
    + Cov(epsilon' C_t epsilon, epsilon' C_s epsilon).
```

For the fixed-offset scalar specialization, all dependence inside `g` enters
only through

```text
u_g_c  = sum_(i in g) f_i u_i / sqrt(F_g),
tau_g2 = f_g' Gamma_g f_g / F_g.
```

The grouped covariance formula therefore becomes the ordinary scalar formula
on `G` independent match aggregates, with `diag(tau_g2)` and one Gaussian
probe draw per match. This permits unrestricted covariance within a declared
match without estimating the full `Gamma_g`; it does not permit dependence
across declared matches, including different matches of the same worker.

The first primary match-variance regression uses the raw proxy
`s_g=y_g_c ehat_g,-g,c`. Its outcome-free covariates are normalized midranks of
match leverage, the three primitive `b_tg` diagonals, and `F_g`, expanded to an
intercept, five linear terms, five squares, and ten pairwise interactions (21
terms before inactive columns are dropped). The sensitivity model retains
only intercept, leverage, and leverage squared. No duration/support covariate
is included in version 1: `F_g` is the declared support/mass summary, and an
additional feature would require a new registration. The positive fitted
`tau_g2` vector enters covariance only. Cross-fitting regularizes this small
regression; it does not supply an independent nuisance estimate or strengthen
the sampling claim.

For grouped `q=1`, let `a_1` solve

```text
Q_t a_1 = lambda_1 H a_1,       a_1' H a_1 = 1,
v_1g = x_g_c a_1.
```

Thus `sum_g v_1g^2=1`, `b_1_hat=v_1' y_c`, and the nonzero grouped
spectrum is the same spectrum as `H^(-1/2) Q_t H^(-1/2)`. The leading square
is recentered with the raw leave-match product

```text
V_b1_hat_LO = sum_g v_1g^2 y_g_c ehat_g,-g,c,
```

not with `sum_g v_1g^2 tau_hat_g2`. The positive structured aggregate-match
variance fit enters only the covariance and studentization.

Write `B_t^R=B_t-lambda_1 v_1 v_1'` and
`r_tg^R=(b_tg-lambda_1 v_1g^2)m_g`. The direct remainder kernel is

```text
C_t^R = B_t^R - {D(r_t^R) M + M D(r_t^R)}/2,
y_c' C_t^R y_c
  = theta_hat_t - lambda_1 (b_1_hat^2 - V_b1_hat_LO).
```

This equality is a hard numerical gate, not merely a rearrangement used to
compute the reported remainder. In the weighted original-row block, define
`w_gi=sqrt(f_i/F_g) v_1g` and use the square-root-weighted outcomes.
Equivalently, each literal physical copy has mode weight
`v_1g/sqrt(F_g)`. The leading score is unchanged and the raw recenter is
`sum_g (w_g' y_tilde_g)(w_g' ehat_-g_tilde_g)`. Compression maps the physical
rank-one-subtracted block kernel exactly to `C_t^R`. This is why the grouped
recenter contains one cross-product of match contractions rather than a sum
of observation-by-observation products.

With `V=diag(tau_g2)` and `g_t^R=C_t^R mu_c`, the joint leading/remainder
covariance uses

```text
Var(b_1_hat)       = sum_g v_1g^2 tau_g2,
Cov(b_1_hat,R_t)   = 2 sum_g v_1g tau_g2 g_tg^R,
Var(R_t)           = 4 sum_g (g_tg^R)^2 tau_g2
                     - 2 tr(V C_t^R V C_t^R).
```

The existing KSS/Andrews--Mikusheva maximal-curvature radius and ellipse-image
map then apply without modification. Numerical success does not establish the
one-mode regime: the remainder spectrum and influence must be diffuse for the
target at hand. No concentration cutoff or automatic `q` selection is part of
this contract. The prospective internal local gates are frozen in
[`match_inference_q1_development_v1.json`](match_inference_q1_development_v1.json).

Every deleted match requires nonsingular `M_gg`; graphically, deleting the
whole match must leave the identifying worker--firm graph connected. The
fixed-offset scalar route reports the number of independent matches, effective
match count, largest `F_g` share, largest match leverage, smallest maker
denominator, target-specific maximum match influence share, leading spectral
share, q=1 remainder share when applicable, maximum leading-mode match weight,
structured-model support/boundary/floor diagnostics and common-versus-
leverage sensitivity, solver residuals, trace MCSE, covariance PSD diagnostics,
and an explicit fixed-offset-conditioning flag.

## Explicit support matrix

| Cell | Point estimator and assumptions | Variance and reference law | Current state |
|---|---|---|---|
| Observation x `q=0` | Existing observation leave-out estimator; independent observations; every observation leave-out identified; unit frequency and mover-only in the Rust MVP | Explicit `structured_common` or `structured_leverage` positive common `V`; Gaussian approximation requires diffuse leading and influence contributions, which remain reported diagnostics | Supported only on the explicit Rust generic-JLA/Counter-V1 tuple; oracle infrastructure remains internal |
| Observation x `q=1` | Same point estimator and deletion assumptions as observation `q=0` | Same separately selected structured variance mode; one estimated leading generalized eigenmode treated explicitly; the remainder kernel and influence must be diffuse and remain target-specific diagnostics | Supported for the eligible one-mode regime on the same explicit tuple; a successful multi-mode calculation is outside the coverage claim, and exact Mata remains isolated when `inferencemodel()` is omitted |
| Match x `q=0` | Existing whole-match point estimator; `nuisance(fixedoffset)`; delete-match connectedness and positive scalar maker denominator; inference conditional on the full-sample `gamma_hat` | Independent declared matches, unrestricted within-match dependence absorbed by `tau_g2`, and an explicit structured model for aggregate-match variances; Gaussian grouped limit requires diffuse target and influence contributions | Internal development only; scalar-collapse identities and local gates pass and the first bounded q0 campaign is registered, with no public routing |
| Match x `q=1` | Same grouped point, fixed-offset conditioning, and connectivity conditions | Same aggregate-match variance model; the raw leave-match product recenters one dominant grouped mode and the grouped remainder must be diffuse | Internal local foundation registered; no public routing or coverage evidence |

Across observation cells, low-dimensional controls remain in the joint model
operator. In the fixed-offset match cell, controls enter only through the
full-sample `gamma_hat` used to construct `y_star`; the inference operator is
FE-only and conditions on that offset. Eligible stayers, mixed deletion,
automatic routing, simultaneous projection inference, cross-match dependence,
and nuisance-estimation uncertainty remain unsupported and fail before
inference draws.

### Cell contracts

**Observation deletion x `q=0`.** The estimand and point estimate are the
unchanged component quadratic form and observation leave-out correction. The
identifying graph must remain connected after deleting every observation and
every `M_ii` must pass the registered numerical gap. Rows are independent,
unit-frequency mover observations; controls are included in the full operator.
The covariance uses a separately selected oracle, unrestricted-KSS, or
structured-common variance mode, never an implicit substitution. The reference
law is Gaussian only under a diffuse transformed target and linear-influence
Lindeberg condition. Matrix-free combined influence solves and shared Gaussian
pseudo-outcome solves provide the covariance. Maximum leverage, leading
spectral share and MCSE, mode residuals, maximum linear-influence share, solver
residuals, trace MCSE, and covariance PSD receipts determine credibility.
Unsupported population, weights, dependence, solver, or variance modes fail
before inference RNG.

**Observation deletion x `q=1`.** The estimand and point estimator are exactly
the same as in observation `q=0`; `q` changes only the reference approximation.
The same observation leave-out connectivity, stochastic assumptions, and
explicit variance-mode choice apply. One largest-absolute generalized mode is
estimated matrix-free, subtracted from the zero-diagonal kernel, and treated as
a recentered squared Gaussian score. The remainder receives a Gaussian
approximation and the final interval is the image of its joint covariance
ellipse. Leading and remainder spectral shares, maximum leading-mode weight,
maximum remainder-influence share, both mode residuals, joint-covariance PSD,
curvature, critical simulation count, and trace MCSE are mandatory. A tied
opposite-sign mode can leave a concentrated remainder; requesting `q=1` does
not suppress that diagnostic.

**Match deletion x `q=0`.** The estimand remains `beta'Q_t beta`, and the point
estimator remains the existing whole-match correction. Under
`nuisance(fixedoffset)`, its physical-row block formula reduces exactly to one
collapsed scalar per match, conditional on `gamma_hat`. Arbitrary covariance
inside a match is absorbed by the scalar `tau_g2`; different declared matches
must be independent. Matrix-free combined influence solves and Gaussian
aggregate-match probes use the collapsed sufficient statistics without
treating `F_g` as replication. The model for `tau_g2` is structured and can be
invalid under omitted aggregate-variance drivers. Each match must remain in
one coordinate and its deletion must retain identification. This route is
suggestive conditional sampling uncertainty, not joint-nuisance or
unrestricted-KSS inference.

The first campaign is frozen in `match_inference_q0_campaign_v1.json` together
with its pre-result `match_inference_q0_campaign_v1_amendment1.json`. The
amendment repairs only the outcome-free one-mode fixture and the tiny
profile's numerical resolution; no development outcome, threshold, seed, or
inventory changed. Its
correct-model aggregate variance is exactly affine in the normalized
match-mass midrank included by the primary structured model. Separate cells
vary physical-row covariance while holding the scalar aggregate variance
fixed, vary match and target masses independently, exercise the diagonal and
CMG solvers, and diagnose diffuse, one-mode, multi-mode, weak, null, mild-
omission, severe-omission, and varying-control regimes. Outcome-free
target-specific spectral checks must pass before a task manifest can be
created. The control cell is not coverage-eligible because its repeated
full-sample nuisance estimation is outside the conditional coverage claim.
Tiny and one-task SCC profiles validate only the source-bound execution and
inventory path. The registered 400-replication-per-cell profile subsequently
completed all 22,400 target attempts at exact source `c26a7ee`. All frozen
correct-model, mild-misspecification, severe-limitation, numerical, and
inventory gates passed. Correct-model coverage was `0.9325`--`0.9775` and
empirical-to-estimated SE ratios were `0.9324`--`1.0437`; severe omission
reduced total coverage to `0.8096` with an SE ratio of `1.4519`. Weak/null
draws retained typed PSD withholding, and the varying-control cell remained
coverage-ineligible. The exact development result is
`match_inference_q0_campaign_v1_result.json`. It is not a confirmation
campaign or public support evidence.

**Match deletion x `q=1`.** The grouped estimand, fixed-offset conditioning,
point correction, connectivity, and aggregate-match variance requirements
remain those of match `q=0`; `q=1` changes only the reference approximation by
removing one grouped generalized mode. Grouped `q=0` now passes its registered
development gates. The separate q1 local contract is registered in
`match_inference_q1_development_v1.json`; it requires the raw leave-match
recenter, independent physical-block and collapsed-scalar oracles, and a
direct remainder identity before any campaign. No q1 implementation or
campaign is inherited from the q0 result, and no request may fall back to
observation deletion or `q=0`.

## Rust implementation and evidence

The internal oracle layer and public structured-variance attachment support generic JLA,
observation deletion, mover-only samples, independent rows, unit frequency,
joint nuisance treatment, low-dimensional controls, and explicit diagonal-PCG
or generic-CMG routes. It preserves atomic generation, cancellation,
Counter-V1 addressing, exactly-once prepared-session release, memory admission
and reconciliation, and complete-original-system residual receipts.

The public Stata surface requires the separately named inference model and
the following supported effective tuple. Backend, RNG, algorithm, deletion,
and preconditioner must be explicit; mover-only and joint nuisance may use
their observation-deletion defaults:

```text
backend(rust) rng(counter_v1) algorithm(jla)
deletion(observation) stayers(movers)
preconditioner(diagonal|cmg)
inference(highrank|q1)
inferencemodel(structured_common|structured_leverage)
```

The nuisance mode must be `joint`, frequency weights are rejected, and
`project()` cannot share the generation. `structured_common` is the primary
15-term model; `structured_leverage` selects the three-term sensitivity model
as the covariance input. Both fits and their discrepancy diagnostics are
returned in either case.

The implementation also estimates the first two generalized target modes with fixed-count
two-vector power iteration on the matrix-free squared target operator, rotates
the converged subspace with a Rayleigh--Ritz step, and estimates each target's
trace square with streamed Gaussian probes. Every inverse action has a
complete-system residual receipt. In the supported explicit public `q=1` mode it subtracts the
   leading rank-one target from the diagonal, influence, and covariance-probe
   quadratic form, recenters the leading square with the raw leave-out mode
   variance product, and returns the joint leading/remainder covariance,
   curvature, simulated critical value, remainder-identity diagnostic, and
   Andrews--Mikusheva confidence interval.
Concentration statistics do not trigger an automatic `q` choice.

With `R` covariance probes, `S` spectrum probes, and `I` fixed block-power
iterations, the current `q=0` attachment requests
`3 + R + 5S + 16I + 18` solver columns: three influence columns; one common
fit per covariance probe; one base plus four target actions per trace probe;
four target actions per iteration for each of four targets; two common starts;
and four final Rayleigh--Ritz/residual columns per target. The `q=1` route adds
four remainder-influence columns but reuses the same `R` pseudo-outcome solves
for all q=0 and q=1 moments. Probe outcomes and moments are streamed in admitted
batches; retained state is linear in observations, parameters, targets, and
solver receipts, never observations times probes.

Focused tests use independent explicit dense matrices for `H^{-1}`, `P`, every
`B_t` and `C_t`, zero diagonals, point-kernel values, combined influence
vectors, and the one-solve Gaussian scalar identity. They also verify the
`q=1` rank-one subtraction and leading/remainder covariance, distinguish the
grouped match kernel from singleton deletion, and cover diffuse and dominant
spectra without a hard-coded routing threshold. A deterministic
50,000-probe experiment checks the complete joint trace covariance against
`2 tr(V C_t V C_s)` within reported numerical MCSE. Integration tests cover
bitwise preservation of point estimates, probe/batch invariance, the exact
three-to-four map, diagonal and CMG routes, complete-system receipts,
cancellation, typed rejection, and the exact admitted-memory boundary.
A separate 10,000-replication conditional Gaussian experiment with known
heteroskedastic variances and a block target whose leading spectral share is
`1/80` produced 0.9466 coverage for a nominal 0.95 oracle-variance `q=0`
interval. This validates the bounded oracle layer only.

The deterministic fitted-variance harness is
`rust/crates/vckss-core/examples/structured_inference_qualification.rs`; its
fail-closed validator is
`fevc/tools/validate_structured_inference_qualification.py`. It builds the
leave-out kernels independently of the generic-JLA attachment, uses the
production structured variance regression, and covers unit-weight diffuse and
graph-bottleneck designs, controls, Gaussian and standardized t8 errors,
correct common/leverage variance models, mild functional and omitted-driver
misspecification, severe omitted drivers, and weak-signal diagnostics. The
registered confirmation profile uses 2,500 replications per cell at dimensions
12 and 16. Its spectral thresholds verify the constructed test cases; they are
not public routing cutoffs.

The 2026-09-03 dirty-checkpoint confirmation run did not authorize promotion.
Every registered `q=0` correct-model row passed, as did Gaussian
`structured_common` `q=1`, and mild misspecification stayed within its bounded
degradation gate. Severe omitted-driver cases visibly failed, including
coverage of 0.8116 and 0.7824 for the diffuse and bottleneck total targets,
which demonstrates rather than hides the additional assumption. But the
correctly specified `q=1` firm target covered only 0.9348 under the
leverage-only heteroskedastic DGP and 0.9336 under t8 errors. Both miss the
predeclared `max(0.015, 3 MCSE)` coverage tolerance. At that checkpoint the
modes therefore remained experimental. This transient run is not source-bound
release evidence and does not describe the later V5-supported state.

The registered V2 follow-up in
`structured_inference_qualification_v2.json` uses the exact identity

```text
C_t = -diag(r_t) + U_t K_t U_t'
```

to replace retained observation-by-observation kernels with sparse design
rows, diagonal vectors, and coefficient-space factors. Kernel actions and
`2 tr(V C_t V C_s)` are therefore exact for the qualification design while
storage is `O(N + p^2)`. A tiny independent dense test checks the full and
q=1-remainder identities and interval endpoints at `1e-9` scale-relative
tolerance. The campaign runs the two failed firm-target cells under both the
oracle and cross-fitted structured variance at dimensions 16, 24, 32, 48, and
64. Semantic seeds bind draws to cell, dimension, and replication, so array
sharding and scheduling cannot change a result. Immutable manifests, disjoint
task receipts, full-inventory aggregation, and scheduler build/array/aggregate
dependencies prevent partial or duplicate output from appearing complete.

The source-bound V2 development campaign at commit `37f9798` completed all
50 tasks and 20,000 requested rows. All scheduler stages and array tasks had
zero failed/exit status. Every gate passed except the oracle-variance t8 firm
cell at dimension 64: coverage was 0.972 with MCSE 0.0052, versus the fixed
`max(0.015, 3 MCSE)` tolerance. Its maximum mode share fell from 0.0310 at
dimension 32 to 0.0207 at 48 and 0.0156 at 64; remainder-influence
concentration fell from 0.00219 to 0.00188 and 0.00120. The fitted path at 64
covered 0.964. Under the registered classification, an improving diffuse
remainder plus an oracle failure at dimension 48 or 64 is a
`q1_reference_or_remainder_problem`. This blocks confirmation and promotion;
it is not evidence against only the structured variance smoother. Exact run,
hash, accounting, and neighboring-dimension statistics are recorded in
`structured_inference_diagnostic_v2_result.json`.

The V3 audit found a center/covariance mismatch rather than a new variance
model requirement. The former Rust `q=1` path replaced the realized leading
mode leave-out product with the positive structured estimate of
`Var(b_1_hat)`, while its remainder influence and trace covariance continued
to describe the directly rank-one-subtracted leave-out kernel. The corrected
path uses `sum_i v_1i^2 y_i e_hat_i,-i` for the leading recenter, leaves the
structured variance vector solely in the covariance calculation, and requires
the two remainder constructions to agree numerically. Result schema V3 adds
the raw recenter, remainder-identity error, and actual critical-draw count.
The Stata attachment requests at least 100,000 Counter-V1 critical draws.

The correction and its no-post-result-change campaign are registered in
`structured_inference_qualification_v3.json`. The campaign retains the V2
factorized moderate-dimension oracle and the two named firm-target designs,
adds the old center only as a non-gating diagnostic, and uses deterministic
Gauss--Legendre inversion of the q=1 reference CDF so per-replication interval
qualification is not contaminated by critical-value simulation noise.
Independent tests compare that inversion to high-resolution Simpson
integration, production Counter-V1 quantiles to direct numerical integration,
and the ellipse image to a one-million-angle brute-force oracle. Registration
does not itself authorize confirmation or promotion.

The clean source-bound V3 smoke passed. The subsequent development campaign
at commit `7b92cf1` completed all 50 tasks and 20,000 requested rows with zero
scheduler or process failures. It passed every registered gate except
`dominant_common_t8/64/oracle: coverage`: coverage remained 0.972 with MCSE
0.0052. The corrected direct-remainder identity error was `3.3e-13`; the
empirical-to-estimated leading and remainder variance ratios were 0.982 and
0.989; the standardized leading/remainder covariance discrepancy was 0.088.
The maximum leading-mode share was 0.0156 and remainder-influence
concentration was 0.00120. Thus the failure did not disappear as the spectrum
became more diffuse, and it also occurred under oracle variances. Under the
frozen V3 classification this is a
`q1_recenter_reference_or_remainder_problem`, not a structured-variance-model
failure. The raw-recenter correction is algebraically necessary, but it is not
sufficient to authorize promotion. Exact run identities, hashes, accounting,
and neighboring-dimension results are recorded in
`structured_inference_diagnostic_v3_result.json`.

The preregistered V4 layer diagnosis in
`structured_inference_qualification_v4.json` and its two pre-result amendments
held the dimension-64 design and true variance vector fixed. It used 20,000
calibration and 10,000 independent evaluation replications, common random
numbers across paired covariance variants, Gaussian and standardized-t8
outcomes, direct joint-Gaussian reference draws at both the actual nuisance and
parabola vertex, analytic population covariance, one-component covariance
hybrids, held-out required-radius calibration, and q=0 comparators. Before SCC
execution, the real generator-to-receipt path and deliberate duplicate,
missing, mixed-seed, malformed, unpaired, named-failure, dirty-source, and
partial-inventory failures all passed locally. A two-task SCC smoke then
completed before the 60-task registered campaign.

All 120,000 expected raw rows and 60 task receipts were present, and all SGE
stages reported `failed=0` and `exit_status=0`. Production q=1 coverage was
0.9591 (Gaussian) and 0.9584 (standardized t8), each with MCSE about 0.0020.
Analytic fixed-population covariance changed neither result; substituting the
leading variance, remainder variance, or cross covariance separately also had
no material effect. Direct joint-Gaussian reference coverage was 0.9581 at the
vertex and 0.9593 at the actual nuisance value. By contrast, the independently
calibrated shortest required radius covered 0.9483--0.9510 on held-out draws.
This gap is the expected curvature-bound conservatism described above, not an
ellipse-image or reference-CDF implementation error. The q=0 comparators
covered 0.9488--0.9532. Gaussian and t8 standardized leading and remainder
moments were close, covariance error was negligible relative to V3, and the
remainder identity error stayed below `1.7e-12`.

The registered V4 classification is `v4_primary_cell_passes`. Scientifically,
the remaining behavior is modest q=1 curvature-bound conservatism plus Monte
Carlo fluctuation in the earlier 1,000-replication V3 cell--not random
studentization, a covariance component, t8 finite-sample non-Gaussianity, or a
decomposition error. No correction, empirical critical value, automatic q
selection, or same-cell development rerun is justified. The immutable result
is `structured_inference_diagnostic_v4_result.json`; it is development evidence
only and does not authorize confirmation or promotion.

The result ABI now returns the maximum observation share of each full linear
influence variance together with the existing spectral, support,
positivity-floor, fold-condition, and probe-MCSE diagnostics. The default
display exposes the leading share, trace MCSE, maximum mode weight, full or
remainder influence concentration, selected variance model, floor share, and
training-boundary share. These diagnostics permit an application-level audit;
they do not repair misspecification or turn a failed qualification cell into a
supported claim.

The clean source-bound V5 confirmation registered in
`structured_inference_confirmation_v5.json` has passed. Before confirmation,
the complete local tiny path and a real seven-task SCC smoke passed. The
confirmation used 2,500 replications for each of 20 cell-dimension pairs,
producing all 200 task receipts, 200,000 target-replication rows, and 80
summaries with no scheduler, schema, hash, or inventory failure. Every frozen
scientific and spectral gate passed. Correct-model primary `q=0` coverage was
0.9376--0.9572 across 24 rows; correct-model primary `q=1` coverage was
0.9372--0.9544 across 15 rows. The standardized-t8 `q=1` firm row covered
0.9476 with MCSE 0.00446. Diffuse leading shares declined from dimension 12 to
16, the dominant worker and firm targets had leading share 0.841 and remainder
share 0.073, and the deliberately multi-mode covariance target retained a
remainder share of 0.648 after one removed mode. It therefore remains outside
the `q=1` coverage claim even though its atomic execution rate exceeded the
registered 0.95 minimum.

The evidence also bounds the statistical claim. The common and leverage-only
structured models passed their correct-model fixtures and mild departures met
the frozen degradation rules. Severe omitted-driver misspecification produced
total-target coverage of 0.8248 under `q=0` and 0.7692 under `q=1`; null and
weak-signal cells produced frequent typed nonpositive-covariance and `q=1`
failures. These are model and identification diagnostics, not observations to
condition away. The immutable result, all compact cell-target summaries,
hashes, and SCC accounting are in
`structured_inference_confirmation_v5_result.json`.

The next coherent implementation order after the separate promotion and
exact-source native qualification is:

1. do not alter the q=1 recenter, covariance/studentization, curvature radius,
   or ellipse image: V4 and V5 identify no justified correction;
2. preserve the supported explicit observation-deletion boundary, its
   variance-model and spectral warnings, target-specific `q=1` limitation,
   fail-closed support matrix, no automatic routing, and no default
   substitution;
3. run the registered fixed-offset collapsed-match `q=0` tiny and SCC smokes,
   then its bounded development campaign, before any public route;
4. begin grouped `q=1` only after q=0 passes, preserving raw leave-match
   recentering and the existing Andrews--Mikusheva ellipse-image machinery.

No million-row benchmark, automatic routing, or default substitution is part
of these scientific slices. The bounded SCC campaign is only a deterministic
execution vehicle for the registered moderate-dimension experiment.
