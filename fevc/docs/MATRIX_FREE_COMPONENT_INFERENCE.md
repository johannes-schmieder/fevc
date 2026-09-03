# Matrix-free component inference

## Status and authority

This document is the active scientific and implementation contract for the
new Rust generic-JLA component-inference work. The attachment is exposed as an
explicit experimental capability through
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
qualification evidence for this experimental mode; its public name does not
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
theta_hat = lambda_1 (b_1_hat^2 - Var_hat(b_1_hat))
            + theta_1_hat + o_p(sd).
```

The remainder kernel and its diagonal are formed by subtracting
`lambda_1 v_1 v_1'`. The same common variance vector yields the joint
covariance of `(b_1_hat, theta_1_hat)`. The remainder receives the Gaussian
approximation only when its reported concentration is diffuse. Confidence
sets are the Andrews--Mikusheva image of the joint Gaussian covariance ellipse,
using a separate Counter-V1 critical-value domain and the same deterministic
ellipse-image calculation as the exact Mata implementation. The largest
observation share of the remainder's linear-influence variance is reported as
well. Numerical mode finding fails closed on nonconvergence, a material
generalized-eigen residual, an absent nonzero mode, or invalid joint
covariance. No hard-coded concentration threshold chooses between `q=0` and
`q=1`.

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

If matches are independent clusters with covariance blocks `Gamma_g`, then

```text
Omega[t,s]
  = 4 sum_g (C_t mu)_g' Gamma_g (C_s mu)_g
    + Cov(epsilon' C_t epsilon, epsilon' C_s epsilon).
```

For Gaussian block probes the second term is the corresponding block trace.
Estimating general `Gamma_g` and cross-products `Gamma_g x Gamma_h` is a new
grouped variance-product problem. A diagonal common variance model supports
only independent observations within matches; it does not justify arbitrary
within-match dependence. Strict grouped inference must therefore be exposed
separately and only after its block covariance/product construction is proved
and tested.

Every deleted match requires nonsingular `M_gg`; graphically, deleting the
whole match must leave the identifying worker--firm graph connected. The
largest block leverage eigenvalue, minimum maker-block eigenvalue, maximum
match influence share, bad product-pair share, and grouped spectral
concentration are mandatory diagnostics.

## Explicit support matrix

| Cell | Point estimator and assumptions | Variance and reference law | Current state |
|---|---|---|---|
| Observation x `q=0` | Existing observation leave-out estimator; independent observations; every observation leave-out identified; unit frequency and mover-only in the Rust MVP | Explicit `structured_common` or `structured_leverage` positive common `V`; Gaussian approximation with reported leading concentration and influence concentration | Exposed experimentally on the qualified Rust generic-JLA tuple; oracle infrastructure remains internal |
| Observation x `q=1` | Same point estimator and deletion assumptions as observation `q=0` | Same separately selected structured variance mode; one estimated leading generalized eigenmode treated explicitly and a Gaussian remainder; report leading/remainder concentration and maximum mode weight | Exposed experimentally on the same Rust tuple; exact Mata remains the target-specific comparator when `inferencemodel()` is omitted |
| Match x `q=0` | Existing whole-match point estimator; delete-match connectedness and nonsingular maker blocks; independent match clusters for general grouped inference | Requires block `Gamma_g` model and grouped variance products; a diagonal structured model is valid only with independent observations within match; Gaussian grouped limit with match influence and spectrum diagnostics | Point estimation implemented; grouped covariance kernel above is registered but Rust inference is not yet implemented |
| Match x `q=1` | Same grouped point and connectivity conditions | Requires both validated grouped covariance machinery and a dominant grouped generalized mode with a diffuse grouped remainder | Staged; fail closed until match `q=0` is qualified |

Across all cells, low-dimensional controls enter through the existing full
model operator. Nonunit frequency weights, eligible stayers, mixed deletion,
within-match dependence under a diagonal variance model, automatic routing,
and simultaneous projection inference are unsupported in the first Rust
implementation and fail before inference draws.

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

**Match deletion x `q=0`.** The estimand remains `beta'Q_t beta`, but the point
estimator deletes a complete worker--firm match and uses `M_gg^{-1}`. It is
unbiased when cross-match errors have zero covariance; general within-match
dependence is allowed only with a declared block covariance/product estimator.
Each deleted match must leave the identifying graph connected and each maker
block must be nonsingular. Computation requires target blocks, grouped maker
solves, grouped influence actions, and block Gaussian probes. Required
diagnostics include the largest block-leverage eigenvalue, smallest maker-block
eigenvalue, grouped leading spectral share, maximum match influence share,
bad variance-product-pair share, numerical residuals, MCSE, and PSD. The first
production slice may instead declare independent rows within match and a
diagonal structured variance; it must say so explicitly and reject claims of
match-robust inference.

**Match deletion x `q=1`.** The grouped estimand, point correction, connectivity,
and block covariance requirements remain those of match `q=0`; `q=1` would
only change the reference approximation by removing one grouped generalized
mode. This cell is staged until grouped `q=0` passes its block covariance and
variance-product tests. Until then it is rejected explicitly rather than
falling back to observation deletion or `q=0`.

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
complete-system residual receipt. In the experimental public `q=1` mode it subtracts the
leading rank-one target from the diagonal, influence, and covariance-probe
quadratic form, and returns the joint leading/remainder covariance, curvature,
simulated critical value, and Andrews--Mikusheva confidence interval.
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
dense leave-out kernels independently of the generic-JLA attachment, uses the
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
predeclared `max(0.015, 3 MCSE)` coverage tolerance. The modes therefore remain
experimental. This transient run is not source-bound release evidence.

The result ABI now returns the maximum observation share of each full linear
influence variance together with the existing spectral, support,
positivity-floor, fold-condition, and probe-MCSE diagnostics. The default
display exposes the leading share, trace MCSE, maximum mode weight, full or
remainder influence concentration, selected variance model, floor share, and
training-boundary share. These diagnostics permit an application-level audit;
they do not repair misspecification or turn a failed qualification cell into a
supported claim.

The next coherent implementation order is:

1. make the fitted-variance qualification efficient enough to run larger
   graph dimensions, then determine whether the two failed `q=1` cells converge
   to nominal coverage without changing the registered statistical gates;
2. retain target blocks and implement the grouped match `q=0` kernel under a
   narrowly declared covariance model;
3. promote the explicit experimental Stata results beyond experimental status
   only after the selected variance mode passes misspecification, coverage,
   lifecycle, and failure tests.

No SCC campaign, million-row benchmark, automatic routing, or default
substitution is part of these scientific slices.
