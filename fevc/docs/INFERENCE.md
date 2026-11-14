# Inference

Component-inference routes in this guide accept Mean (the exact/JLA default)
and explicit None. Mean treats the observed retained working-outcome mean as
a fixed constant in highrank and q1 inference, omitting its estimation
uncertainty. This is an approximation, not conditional validity given the
observed mean. Existing combined exact-Mata inference/projection requests
accept both modes; native combined component/projection requests remain
unsupported. Corrected with either request is rejected before estimator
RNG; all other tuple restrictions remain.
The centered point estimator's numerical MCSE is a separate diagnostic,
with the fixed-mean/increment conventions in [CENTERING.md](CENTERING.md).

## Leave-out residual standard errors

`inferencemodel(leaveout_residual)`, the default before the
repeat-measurement model of the next section, uses a different
construction from the methods described below: squared residuals carry the
leading term of the KSS variance formula, and simulated noise passed through
the estimator supplies the quadratic term and a calibration. They assume
similar error variances among units that share a worker or a firm and are not
robust to arbitrary heteroskedasticity. The method, its assumptions,
diagnostics, evidence and supported routes are in the
[technical supplement](standard_errors/standard_errors.tex). The rest of this
document concerns the earlier exact and structured methods.

## Repeat-measurement standard errors

This model, with one interval per component, is the default:
`inference(auto)` without `inferencemodel()` selects it, and
`inferencemodel(repeat_measurement)` requests it by name with
`inference(auto)`, `inference(highrank)` or `inference(q1)`. It runs the
same pass as the leave-out residual method on the deletion units (under match deletion the
collapsed mover matches and one unit per physical stayer observation; under
observation deletion every physical observation), with three influence
solves and four solves per pseudo-noise draw, and differs in the variances
and in how the covariance is assembled:

- deletion units that share a worker-firm cell have identical regressors and
  independent errors under the independence the point estimator already
  assumes, so a cell with two or more units is a repeat-measurement set and
  its units receive the sample variance of their outcomes. Under match
  deletion the sets are the eligible stayers; under observation deletion
  every match of a mover with two or more observations is a set as well;
- the remaining units (a collapsed mover match; under observation deletion a
  mover's match of one observation) receive a scale, the least-squares fit
  of leave-out variance products on the normalized midranks of leverage and
  unit mass and their squares (floored at five per cent of the mean
  product), times a quadratic in the worker and firm effects of the fit,
  standardised over physical observations and limited to three standard
  deviations, with the six coefficients fitted to the same products; each
  step keeps one coefficient per hundred fitted units. The model is fitted
  on the units it is used for when there are at least a hundred of them,
  and otherwise on all units of movers;
- the covariance is `4 sum_a v_a (K_t z)_a (K_s z)_a - 2 tr(K_t V K_s V)`
  plus `4 sum_g c_g^t c_g^s q_g^2 (T_g-2)(T_g-1)/(T_g+1)` for the pairs of
  units of set `g`, where `q_g` is the sample variance, `T_g` the number of
  units and `c_g^t = B_gg^t / (1 - P_gg)` the weight of target `t` between
  two of them. A stayer's leverage is `1/T_g` and its firm and covariance
  diagonals vanish, so stayers contribute to the worker variance alone; a
  mover's set contributes to every entry. Nothing is truncated or
  projected: a nonpositive variance is reported as a withheld target.

With true variances for the units outside sets and no controls the estimate
is exactly unbiased when the errors of every set are normal with one
variance per set, whatever those variances are. With the fitted model it is
as good as that model; estimated effects enter the profile, and the
observed mean is handled as in the leave-out residual pass. Observation
deletion needs joint nuisance and, natively, unit frequencies. The
derivation is in a technical note maintained with the companion paper; the
simulation harness is `fevc/tools/default_interval/` and its evidence
`fevc/docs/default_interval/`.

#### Controls

With controls the standard errors are those of the fixed-offset estimate:
the estimate of the model without controls for the outcome net of the
fitted control index, with the estimation of the control coefficients
included to first order (`e(inference_nuisance_omitted)` is zero; without
controls nothing is estimated and it is one). Under observation deletion
with jointly estimated controls the rows of one cell differ in their
regressors, which is why the units are those of the model without
controls; the native route forms them in an offset pass of two solves per
point-estimate probe beside the joint point estimate.

Write `X` for the controls collapsed to deletion units as the outcome is,
`Xt` for their residual on the worker and firm effects, `L` for the inverse
cross-product of the residualised controls over physical observations, and
`w_a` for the within-unit product of unit `a`: the sum over its
observations of the controls' deviation from the unit's mean times the
residual (zero for a unit of one observation, hence zero under observation
deletion). The error of the adjusted outcome of unit `a` is
`eps_a - x_a' L (Xt' eps + sum_b w_b)`. With `S` the covariance matrix of
these errors, `C` the kernel of the estimate and `m` its mean influence,
the variance of the estimate under normal errors is exactly
`4 m' S m + 2 tr(C S C S)`. The pass estimates it without a further linear
solve:

- every influence `g = K_t z` becomes `g* = g - Xt L X' g`, with the load
  `c = -2 L X' g` on the within-unit products, and the linear part is
  `4 sum_a v_a g*_a^2 + 4 sum_a r_a g*_a (w_a' c) + sum_a (w_a' c)^2`,
  where `r_a` is the leave-out residual of unit `a`;
- the pseudo-noise is transformed as the data are,
  `zeta - X L (Xt' zeta + sum_a w_a xi_a)` with one further standard normal
  `xi_a` per unit and draw, and the trace is taken in the bilinear form of
  that covariance: `sum_a v_a u*_a^2 + (L X' u)' (sum_a w_a w_a') (L X' u)`
  for a kernel image `u` with `u* = u - Xt L X' u`;
- the pieces of the `q = 1` interval (the variance of the leading score, the
  covariance with the remainder and the remainder's variance) use the same
  projection and loads.

Without controls every formula is the one above and the native results are
bitwise unchanged. What the term does not cover:

1. It is the variance of the fixed-offset estimate. Under the default
   `nuisance(joint)` the point estimate re-estimates the control
   coefficients in every leave-out fit; the two estimates differ by a term
   that involves the control index and is not part of the standard error.
2. A repeat-measurement set receives one sample variance. With controls the
   weights of its units differ, so the set needs one error variance, or
   variances that are unrelated to the squared weights inside the set. An
   error variance that moves with a control inside a worker-firm cell
   violates this.
3. The covariance between a unit's error and its own within-unit product
   enters the linear part but not the covariance of the pseudo-noise. It is
   zero when the errors of a unit have one variance and are uncorrelated;
   otherwise what is left out is of lower order unless one direction
   dominates the component, and then the `q = 1` interval treats that
   direction on its own.
4. Exactness needs normal errors and known variances; the estimate uses
   residuals, sample variances and the fitted model in their place.
5. When the data identify a control coefficient poorly, the noise of the
   estimated sensitivity to it can dominate and the variance estimate of a
   component can come out non-positive. The component is then reported
   without a normal interval (withheld, or with its `q = 1` interval).

### Leading directions and the interval of every component

`inference(highrank)` with this model computes the covariance above and
reports normal intervals. `inference(auto)` and `inference(q1)` with this
model run the same pass and add, for each of the four reported targets, the
leading direction and the `q = 1` decomposition along it:

- the direction is the eigenvector `v` of `A_t v = lambda S v` of largest
  absolute eigenvalue, with `S` the normal matrix of the unit design and
  `w = X v` its unit-space weights of unit length. The native route and the
  matrix-free Mata route find it by a thick-restart Lanczos iteration in the
  `S` inner product (basis six, three Ritz vectors kept, one linear solve per
  step, stopped at a relative residual of `1e-3` or after 40 solves), each
  search started from the previous target's direction; the exact Mata route
  uses a dense eigendecomposition. The start does not depend on the outcome;
- with `b = w'y`, the estimate splits into
  `lambda (b^2 - V-hat_lo[b]) + theta_r`, where `V-hat_lo[b]` is the
  leave-out estimate `sum_a w_a^2 z_a e_{a,-a}` and `theta_r` the remainder,
  a leave-out form with the target's kernel net of the direction (its
  diagonal ratio becomes `(B_aa - lambda w_a^2)/(1 - P_aa)`, and the
  rank-one part is subtracted). The decomposition is an identity, and the
  pass checks it against the remainder computed directly;
- the covariance of `(b, theta_r)` has the entries
  `max(V-hat_lo[b], sum_a w_a^2 v_a)`, `2 sum_a w_a v_a m_a^r` and the
  high-rank form of the remainder,
  `4 sum_a v_a (m_a^r)^2 - 2 tr(K_r V K_r V)` plus the pair term of the sets
  with the remainder's weights, where `v_a` are the unit variances above,
  `m^r` is the remainder's realized influence and the trace uses the same
  pseudo-noise draws. The leave-out variance of `b` is unbiased but rests on
  the few units that carry the direction; taking the larger value keeps it
  positive and, for moderate correlation, can only widen the interval;
- the interval is the Andrews-Mikusheva interval of KSS (Section 6): the
  range of `lambda b^2 + theta_r` over the ellipse of radius `z(kappa)`
  around the estimates, with `kappa` the curvature
  `2 |lambda| V[b] / (V[theta_r] (1 - rho^2))^(1/2)` and `z` its critical
  value, simulated with 100,000 draws.

One more linear solve per target gives the remainder's influence and one per
target and draw its kernel, so the request adds the search, 4 and 4 per draw
to the 3 and 4 per draw of the pass: about 90 solves with the default eight
draws where the pass alone takes 35.

The reported interval is chosen component by component, by one rule on every
route (`fevc__residual_se_post.ado`). With `inference(auto)` a component
receives the `q = 1` interval when its leading direction carries more than
ten per cent of the estimated sampling variance,
`(4 lambda^2 max(b^2 - V[b], 0) V[b] + 2 lambda^2 V[b]^2) / V-hat`, or when
the high-rank variance estimate is not positive, and the normal interval
otherwise; the variance of the sum, whose nonzero eigenvalues are all equal,
keeps the normal interval. With `inference(q1)` every target receives the
`q = 1` interval when it exists. A `q = 1` component has no standard error,
and `e(V)` is posted only when every component has a normal interval.
Neither KSS nor the technical note has a result for a data-dependent choice
between the two intervals, and the case of two or more dominant directions
is not covered. The share is a ratio of two estimates and can exceed one;
the display then says "over 100%".

#### Development of the rule

The statistic and the threshold were chosen on simulations through the
command itself (Stata with a plugin built from the source, package
defaults: 200 probes, eight draws): 16 two-block designs of about 48,000
person-years, from no bottleneck to 30 bridging workers, with error
variances that follow or violate the fitted model and normal or skewed
errors, under match and observation deletion, 1,000 replications each
(32,000 fits, none failed). Their realised regimes, computed without
outcomes, span a true curvature of 0.007 to 8.6 and a true share of the
leading direction of 0.001 to 1. Over the 96 cases (design, deletion mode,
component):

| interval reported | lowest coverage | cases below 0.935 | `q = 1` used without a bottleneck | `e(V)` posted without a bottleneck |
|---|---|---|---|---|
| always normal | 0.770 | 27 | 0% | 100% |
| always `q = 1` | 0.940 | 0 | 100% | 0% |
| share above 5% | 0.940 | 0 | 9.8% | 76% |
| share above 10% (the rule) | 0.937 | 0 | 4.3% | 89% |
| share above 20% | 0.935 | 0 | 1.7% | 95% |
| share above 50% | 0.935 | 0 | 0.3% | 99% |
| curvature above 0.3 | 0.919 | 4 | 6.6% | 83% |
| squared part of the share above 5% | 0.919 | 10 | 5.6% | 86% |

The nominal level is 0.95 and the Monte Carlo standard error of one
coverage is 0.007. Rules that look only at the curvature, or only at the
part of the share that is quadratic in the noise, miss designs in which the
leading direction is mostly signal: the estimate is then close to normal,
but its variance rests on the few units that carry the direction, and the
normal interval covers 0.92 to 0.93. On earlier stored draws of the
prototype (27 designs) the lowest coverage falls from 0.939 at 5% and 0.937
at 10% to 0.933 at 20% and 0.924 at 50%. Ten per cent keeps the protection
of the lower threshold and halves the `q = 1` intervals reported where
there is no bottleneck. The variance of the sum, always reported with the
normal interval, covers 0.941 to 0.969. On three panels of the Veneto data
(2.3 to 5.2 million person-years, both deletion modes) every component
receives the normal interval under this rule (shares 0.03% to 8.1%), and
the default costs 24% to 30% more time than the point estimates alone
(the plain command of the final source, 16 threads).
These are development results: the rule was chosen on them.

#### Confirmation

The rule was then fixed in the source and put to a registered confirmation
of the plain default command, with no inference option: 20 two-block
panels of about 48,000 person-years with design seeds not used before (four
without a bottleneck, the others with 1,000 to 30 bridging workers; error
variances that follow or violate the fitted model; normal and skewed
errors), match and observation deletion, 1,000 replications each. Designs,
seeds, replications and tolerances were written into a manifest before any
outcome of these designs was drawn: in every design and mode at least
99% of the replications with an interval for all four components; coverage
of the reported interval at least 0.92 in every design, mode and component
(a replication without an interval counts as not covered) and at least
0.94 on average; without a bottleneck, a reported interval at most
1.10 times as long as the normal one and `e(V)` posted in at least
50% of the samples; a clean inventory of the outputs.

The first run failed its registered verdict, on the inventory. In one of
its 40,000 fits the standard-error pass stopped (see the limit below) and
the default returned point estimates with the reason, as documented; the
harness had no rule for such a fit and rejected its output as corrupt. That
failure stands. For information only, with that fit counted as a
replication without an interval, the run's other criteria read: at least
99.4% of fits with intervals, lowest coverage 0.937 of 160 cases, mean
0.960, length ratio at most 1.059, `e(V)` posted in at least 60% of the
no-bottleneck samples of a design.

The second run was registered with the package, the designs, the command,
the replications and the tolerances unchanged, new random seeds, and a
harness that counts such a fit as a replication without an interval.
It met all of its tolerances.

Of its 40,000 fits none failed and none fell back to point estimates; at
most 6 of the 1,000 replications of a case had no interval. Coverage of the
reported interval over the 160 cases: lowest 0.930 (the variance of the
sum under match deletion in a design with 1,000 bridging workers), mean
0.959; the variance of the sum, always normal, 0.930 to 0.967. Without a
bottleneck the reported
interval is at most 1.059 times as long as the normal one, the `q = 1`
interval is used for 6.2% of the components and `e(V)` is posted in 89% of
the samples. The same replications judged with the normal interval alone
have a lowest coverage of 0.789. Lowest coverage by kind of design, for the
worker variance, the firm variance and the covariance:

| design | always normal | always `q = 1` | reported | reported, first run |
|---|---|---|---|---|
| no bottleneck | 0.938 | 0.940 | 0.940 | 0.944 |
| weak bottleneck | 0.931 | 0.939 | 0.935 | 0.937 |
| middle | 0.538 | 0.955 | 0.946 | 0.945 |
| strong bottleneck | 0.469 | 0.964 | 0.964 | 0.961 |
| direction mostly signal | 0.931 | 0.940 | 0.940 | 0.946 |

This is simulation evidence for designs of this kind, with one bottleneck
at most, independent errors and no control variables; it is not a coverage
guarantee, and it does not cover two or more dominant directions. The
harness is `fevc/tools/default_interval/`; the manifests (runs `conf1` and
`conf2`, sha256 `6aee3af8e5e6...` and `08427f94de4b...`), cell tables and
verdicts are in `fevc/docs/default_interval/`; the raw task outputs are
kept outside the repository.

#### A limit: outcomes with very little noise

The fitted model is scaled by the mean leave-out variance product of the
units it is fitted on. A single product has a standard deviation
proportional to the size of the outcome's mean part, so when that part
dwarfs the errors the mean of a few thousand products can come out
negative. The pass then stops with `JLA_CONSTRAINT_FAILED` and the default
returns point estimates with that reason. In the simulations above this
happened in one of 112,000 fits, under observation deletion in a design
whose two blocks of firms differ by three log points (the variance of the
outcome's mean part is 130 times the error variance; the model is fitted on
2,836 matches of one observation). With variance ratios like those of wage
data it is not expected.

#### Simulations with controls

All runs pass the plain default command (joint nuisance) through the
harness; records are in `fevc/docs/default_interval/`.

*The term.* Before it, a development check (`dev2_controls`) with one
control made adverse (0.4 times the worker effect plus 0.8 times the firm
effect plus noise, coefficient 0.3, correlation with the sum of the effects
0.55) found the standard error of the variance of the sum 9% to 16% too
small in every one of eight cases (four designs from no bottleneck to 30
bridging workers, both deletion modes, 500 replications each). The same
designs with the same outcome draws after the term (`dev3_controls`):

| component | standard error over spread, before | after | coverage, before | after |
|---|---|---|---|---|
| worker variance | 0.90 to 1.06 | 0.95 to 1.08 | 0.924 to 0.986 | 0.936 to 0.988 |
| firm variance | 0.86 to 1.05 | 0.87 to 1.06 | 0.944 to 0.986 | 0.950 to 0.986 |
| covariance | 0.95 to 1.29 | 0.96 to 1.29 | 0.934 to 0.992 | 0.934 to 0.992 |
| variance of the sum | 0.84 to 0.91 | 0.97 to 1.04 | 0.904 to 0.920 | 0.938 to 0.956 |

(A component that takes the `q = 1` interval has no standard error in
use; its row is informative through the coverage.)

*Other controls.* The same development run varies the control on designs
without a bottleneck and with 30 or 100 bridging workers, 500 replications
per design and deletion mode (13,000 fits in all, none failed, none
without an interval; lowest coverage 0.918, mean 0.962):

| control | realised | coverage of the reported interval |
|---|---|---|
| unrelated to the effects | correlation 0.00 | 0.952 to 0.998 |
| two controls (the adverse one and a trend in the year) | index 0.16 beside effects of 0.34 (standard deviations) | 0.936 to 0.992 |
| loaded on the worker effect (`dev4_worker`) | correlation 0.51, index 0.14 | 0.932 to 0.984 |
| much variation within matches | within-match share of its variance 0.66 | 0.950 to 0.988 |
| little variation within matches | share 0.06, correlation 0.93 | 0.946 to 0.992 |
| error variance moving with the control inside a cell, mild | variance factor 0.47 to 1.68 (10th to 90th percentile) | 0.944 to 0.972 |
| the same, strong | factor 0.17 to 2.16 | 0.930 to 0.970 |
| coefficient of one: an index as large as the effects | index 0.30 beside effects of 0.35 | 0.918 to 0.988 |

*The limit that shows.* With the control index as large as the effects and
no bottleneck, the default under match deletion reports a standard error
of the covariance that is 12% too small (coverage 0.918; the other three
components 0.932 to 0.934), and under observation deletion 0.934 to 0.956.
The same design under `nuisance(fixedoffset)` (`dev4_fixedoffset`, match
deletion, new draws) gives standard errors of 0.99 to 1.07 times the
spread and coverage 0.948 to 0.962: the shortfall belongs to the joint
estimate's additional term (limit 1 above), not to the standard error of
the fixed-offset estimate. With a large control index, `nuisance(fixedoffset)`
is the request whose standard error is the one computed.
(`nuisance(fixedoffset)` with observation deletion is not covered and
returns point estimates with a reason: 1,000 fits of that run, by design.)

These are development checks, not confirmations.

Native requests need readiness bit 19 (`r(component_repeat_api) == 1`) in
addition to the bits of the leave-out residual pass; variance-source code 4
and the diagnostic export
`vckss_rust_engine_repeat_measurement_diagnostic_v1` are additive. The Mata
routes (`backend(mata)`, exact or `algorithm(jla)`) carry the model without
a plugin.

## Scope

FEVC provides opt-in econometric component inference through two
separately identified families. Point estimation remains the default and
continues to post no `e(V)`. Omitting `inferencemodel()` uses the deterministic
Mata exact target-specific family and requires:

- `deletion(observation)`;
- `algorithm(exact)` or an omitted/automatic algorithm that resolves to
  exact;
- the Mata backend and guarded Stata RNG;
- either observation population: `stayers(both)` by default or explicit
  `stayers(movers)`; and
- unit frequency weights.

The explicit fixed-offset match route below also permits positive integer
frequency mass; observation component inference remains unit-frequency.
The symmetric population options do not change the observation inference
formula or its default retained sample. Earlier prerelease observation calls
called that population `movers` while retaining eligible one-firm workers;
explicit `stayers(movers)` now filters those workers before graph selection.
The new explicit structured match routes also support pooled mover–stayer
inference, as described below; that capability does not change observation semantics.
The supported explicit Rust generic-JLA attachment
implements matrix-free `q=0` and eligible one-mode `q=1` component inference
with a named structured variance model. The current development candidate
uses one residual-moment fitter for observation and match deletion.
It is selected only by
`inferencemodel(structured_common|structured_leverage)` together with the
explicit Rust/JLA/Counter-V1 observation or fixed-offset match tuple, or
explicit Mata/exact fixed-offset match tuple. Omitting
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
Kline, Saggio, and Sølvsten (2020) and the KSS Matlab package's
observable behavior. It is repository-authored MIT source. No MATLAB
source, critical-value table, or binary data are included.

## Fixed-observed-mean approximation

For Mean, let `c = sum_i f_i u_i / sum_i f_i` over the retained working
outcome and write `z = u - c`. Joint nuisance handling uses the retained
outcome; fixed-offset handling first subtracts the full fitted nuisance
index. Neither target nor projection weights redefine `c`. The fit,
residuals, target matrices and eigendirections are unchanged.

The realized component calculation uses `z` in every influence vector and
raw leave-out variance proxy, including primitive/polarized exact-Mata
smoothing and q1 leading/remainder terms. Native residual-moment variance
fitting uses unchanged residuals. For fixed-offset match inference the
collapsed outcome is `sqrt(F_g) (ubar_g-c)`. This is not subtraction of the
unweighted mean of the square-root-mass-transformed outcomes.

For an exact observation kernel `C`, the fixed-c calculation uses `z'Cz`
and influence `Cz`, retaining the quadratic-noise trace for `C`. Gaussian
inference error draws remain uncentered: neither `c` nor each draw's own mean
is subtracted. Centering those error draws would describe a different kernel.
q1 retains its raw recenter and exact remainder-identity gate.

The approximation is credible only when the effect of estimating `c` is
negligible on the target's sampling scale, including the q1 remainder scale.
It does not remove estimated-mean bias or guarantee conservative intervals.
This condition is separate from highrank diffuseness, one-mode q1
identification, the variance model and independence assumptions. Fixed-offset
match inference additionally omits nuisance-control estimation uncertainty.
The [centering guide](CENTERING.md#component-inference-with-a-fixed-observed-mean)
details these distinctions.

`e(inference_centering)` records `fixed observed mean` or `uncentered`, and
`e(inference_mean_omitted)` is respectively `1` or `0`. Both belong only to
active component requests. The default output and diagnostic replay state
Mean's omission; numerical MCSE remains a separate diagnostic. Native Mean
component inference requires `r(component_centering_api) == 1` in addition
to point-centering support. Native Mean projection separately requires
`r(projection_centering_api) == 1`. Existing combined exact-Mata requests
support Mean; native combined component/projection requests remain unsupported
for both Mean and None.

The [completed 12-cell assessment](MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
retains **22 failed exact-Mata availability-screen entries**: highrank k12/k20
and q1 k12 fail in both Mean and fixed-c0 arms. The eight native primary cells
have 100% interval availability and pass the broad descriptive screens.
Fixed-outcome native checks nevertheless show material numerical sensitivity,
including point-budget endpoint changes up to 1.15 dense-oracle SD. These
results separate mean-approximation error from other inference limitations;
they establish no general coverage guarantee.

The [native payload guide](RC_BINARY_PAYLOAD.md) records the current
distributed artifacts and their exact platform qualification. Pooled match
inference additionally requires `r(component_mixed_api) == 1`; an ado update
alone does not add that capability to an older plugin. Check the loaded
plugin with `fevc_rust probe`.
Historical None confirmations, calibration failures and platform receipts retain their
original scope; engineering qualification does not imply general coverage.

## Variance proxy and smoothing

There are two distinct implemented variance-model families. The default exact
Mata family below is target-specific and KSS Matlab-compatible. It is not the
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
[`structured_inference_confirmation_v5_result.json`](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/structured_inference_confirmation_v5_result.json).
It does not qualify the later q1 repair. The corrected observation confirmation
has one failed SE-ratio gate (1.101204 versus 1.10), an unresolved calibration
limitation recorded in `RC_OBSERVATION_CONFIRMATION_2026-09-05.md`.

The runnable candidate retains **200 JLA probes by default**. The separate
`inferencegramprobes(#)` option sets Gram precision for this explicit structured
Rust tuple: integers 512 through 2,147,483,647, default 2,048, subject to count
limits and the selected [memory policy](MEMORY.md). An omitted memory budget
never rejects a forecast; an explicit budget warns by default, and only
`memorycheck(error)` makes it a forecast gate. Actual allocation failures
remain errors. Inference memory bounds remain conservative and are not
qualified by the point-estimation full-CMG accuracy measurements. Unsupported explicit use is rejected before RNG. Counts
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
deletion(match) nuisance(fixedoffset) stayers(both|movers)
preconditioner(diagonal|cmg)
inference(highrank|q1)
inferencemodel(structured_common|structured_leverage)
```

Default/explicit `stayers(both)` includes eligible original stayers as
independent physical observations. The additional exact route replaces
Rust/JLA/Counter-V1 and the preconditioner with
`backend(mata) algorithm(exact)`. Both require an explicit structured model.
See [pooled inference](POOLED_COMPONENT_INFERENCE.md); historical mover-only
confirmation evidence below retains its original scope.

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
[controls diagnosis](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/FIXED_OFFSET_PAIRED_RESULT_2026-09-05.md).

Independent q0 and repaired eligible q1 confirmations pass in their declared
fixed-offset regimes:
[match q0](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/RC_MATCH_Q0_CONFIRMATION_2026-09-05.md) and
[match q1](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/INFERENCE_REPAIR_MATCH_CONFIRMATION_2026-09-04.md).
Their estimated-controls cases are calibration limitations, not coverage
claims. Historical development failures remain unchanged. The
[owner-approved interface decision](fixed_offset_match_interface_v1.json)
permits this integration while retaining the corrected observation-q1
confirmation's failed SE-ratio gate as an unresolved RC limitation; it does
not turn that failure into a pass.

The grouped q1 leading square uses the raw leave-match product
`sum_g v_1g^2 z_g_c ehat_g,-g,c`, where `z_g_c` is the centered
collapsed working outcome for Mean and the original one for None; positive
modeled variances are used only
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
\(\widehat\sigma_{Q,i}^2=z_i\widehat e_{i,-i}\) over
\((P_{ii},B_{ii})\), separately for movers and
stayers. The implementation rank-bins each dimension, aggregates populated
joint cells, and applies a weighted local-linear tricube fit. `inferencebins()`
sets the maximum requested joint-cell resolution; the default is 1,000.
Here `z_i=u_i-c` for Mean and `z_i=u_i` for None; the original fit and
leave-out residuals are preserved.

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

The first term uses the target-specific influence weights evaluated at the
selected centered or uncentered working outcome. The second is the variance
of the original corrected quadratic kernel on uncentered Gaussian error
draws with fitted observation variances. `inferencesimulations()` controls this numerical
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
distinct from the KSS Matlab function `lincom_KSS`, whose purpose is
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
\widehat b_1=v_1'z,
\qquad
\widehat V_{b_1}^{\mathrm{LO}}
=\sum_i v_{1i}^2 z_i\widehat e_{i,-i},
\]

with `z=u-c` for Mean and `z=u` for None, and removes
\(\lambda_1(\widehat b_1^2-\widehat V_{b_1}^{\mathrm{LO}})\)
from the unchanged leave-out component point estimate. This raw leave-out
product is not positivity-smoothed. In the structured Rust modes, the common
positive fitted variance vector separately estimates the joint covariance of
the leading score and Gaussian remainder. The native implementation verifies
that the resulting remainder agrees with the direct rank-one-subtracted leave-out
quadratic form after accounting for the independently computed signed
normal-equation residual contribution of both accepted native solves. The
arithmetic threshold and both solve gates are unchanged; an unexplained
identity error withholds inference. The public remainder-identity diagnostic
retains the raw discrepancy before this numerical certificate. This shared
None/Mean repair does not change the point, covariance or interval calculation
and is separate from the statistical mean approximation. See the
[numerical certificate](NUMERICAL_ARCHITECTURE.md#native-q1-remainder-certificate).

For curvature \(\kappa>0\), the q=1 critical-value draw is

\[
\sqrt{\chi_1^2+(\widetilde\chi_1+1/\kappa)^2}-1/\kappa,
\]

with independent square roots of chi-squared-one variables. The
\(\kappa\rightarrow0\) limit is \(\chi_1\), while the large-curvature limit is
\(\chi_2\). FEVC simulates at least 100,000 critical-value draws and maps the
resulting two-dimensional confidence ellipsoid through the rank-one quadratic
by a global angular grid followed by bounded refinement. It does not ship or
interpolate the KSS Matlab package's unlicensed critical-value table.
This critical value is based on the maximal-curvature circle and gives the KSS
uniform asymptotic guarantee of coverage at least at the nominal level under
its maintained assumptions. FEVC's fitted-variance and fixed-observed-mean
approximations require their own validity conditions; selecting Mean does
not inherit that guarantee automatically. It is
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
reference has an asymptotic at-least-nominal uniform coverage guarantee under
its assumptions and may be modestly conservative for a particular design.
This is not a new coverage guarantee for FEVC's observed-mean approximation.

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

The raw cross product need not be symmetric in a realized sample. With
`centering(none)`, FEVC uses the auditable symmetric block estimate

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
matrix. For observation deletion, None reduces to
\(y_i\widehat e_{i,-i}\). Mean, the default, substitutes the centered working
outcome \(z_i=u_i-\bar u\) for the outcome factor in this symmetric proxy,
where \(\bar u=\sum_i f_i u_i/\sum_i f_i\). The working outcome is raw `y`
under joint nuisance handling and `y-Z gammahat` under fixedoffset. The fit,
residuals and projection coefficients are unchanged. Neither target nor
projection weights change this centering mean.

Estimating the mean from the same sample generally removes the exact
unbiasedness identity under heteroskedasticity; no estimated-mean correction
or universal finite-sample coverage guarantee is claimed for Mean projection.
None retains the ordinary cross-fit identity under the assumptions above.
The KSS Matlab main mover-match pipeline instead subtracts a mean after its
square-root match-size transformation. The difference, the separate KSS stayer
proxy and FEVC's translation-invariant frequency convention are documented in
[CENTERING.md](CENTERING.md#projection-and-the-mean-convention).

For stored row \(i\) representing positive integer frequency \(f_i\), define
the score row \(s_i=x_i'A\). A mover-match block is accumulated without
physical expansion under None as

\[
a_g=\sum_{i\in g}f_i s_i y_i,\qquad
b_g=\sum_{i\in g}f_i s_i\widehat e_{i,-g},\qquad
\widehat V_{\gamma,g}=\tfrac12(a_gb_g'+b_ga_g').
\]

Each eligible stayer remains a literal observation-deletion population, so a
stored stayer row contributes \(f_i y_i\widehat e_{i,-i}s_is_i'\). This is
the same mover-match/eligible-stayer partition, retained sample, pooled target,
regression mass, target mass, and nuisance convention as the point estimator.
Under Mean, replace the outcome factor by `z_i` in both the mover and stayer
contractions above, retaining their original residuals. Corrected projection
is unsupported. Every declared match deletion ID must stay within one
worker--firm coordinate;
cross-coordinate IDs fail with `CROSS_COORDINATE_MATCH`.

A residual-squared plug-in covariance is returned separately as a descriptive
naive comparison. Projection inference does not populate the component
`e(V)` unless `inference(highrank|q1)` is also requested.

```stata
fevc wage controls, worker(worker_id) firm(firm_id)       ///
    deletion(observation) backend(mata)                      ///
    project(education experience)                            ///
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
    deletion(observation)                                       ///
    project(education experience)                                ///
    projecteffect(firm) backend(rust) rng(counter_v1)              ///
    algorithm(jla) engine(generic) preconditioner(cmg)
```

The example uses Mean by default and requires a matching native build exposing
`r(projection_centering_api) == 1`. Rebuilt local Mac development plugins pass
the Mean-projection runtime checks; the previously adopted point-centering
payloads lack this capability and their receipts do not cover Mean projection.
Explicit `centering(none)` retains the earlier uncentered native route.
See [native availability](CENTERING.md#restrictions-availability-and-failure)
for development and platform limits.

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
   the same symmetrized None or Mean block formula as exact Mata;
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
  solve peak, and whole-command memory-policy reconciliation; strict forecast
  admission applies only to an explicit budget with `memorycheck(error)`; and
- the public projection column names and existing `e(projection_*)` shapes.

The existing four public result matrices are unchanged. Additive diagnostic
returns are `e(projection_diagnostics)`,
`e(projection_augmentation_receipt)`, and
`e(projection_solver_diagnostics)`.

### Qualification boundary

The focused public-route test compares small sparse results with the dense
Mata oracle for both firm/frequency and worker/target projections, including
nonunit compressed weights against literal expansion. The
committed 1,002-observation KSS Matlab fixture is exercised by
the archived KSS Matlab projection fixture: coefficient
solves must agree with exact FEVC, the complete covariance must lie within a
registered deterministic JLA tolerance of the exact oracle, and the reported
`z1`/`z2` standard errors must remain within the registered Monte Carlo band
around maintained `lincom_KSS`.

The weighted 1,002-row KSS Matlab oracle agrees with compressed exact
FEVC to `1.78e-15` maximum absolute error under literal expansion, within
`4.39e-8` absolute for the independent dense same-formula calculation, and
within `1.17e-6` relative for official `lincom_KSS` standard errors. Rust with
4,000 Counter-V1 probes is within `0.2994%` of exact for the full covariance.

The
[focused scaling comparison](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference)
runs separate, source-bound FEVC and KSS Matlab processes so wall time and peak
RSS cover KSS Matlab's JLA-plus-`lincom_KSS` path rather than only `lincom_KSS`.
Source `96e7a66` passes all coefficient, covariance-diagonal, SE, residual, PSD,
and memory gates at 6,000 and 24,000 rows, but explicit diagonal PCG is not a
qualified large-data route: it becomes 18.97 times slower than KSS Matlab on the
24,000-row command and fails to converge at 96,000 rows. The strict harness
also rejects KSS Matlab's independently reconstructed 96,000-row grounded fit, so
no paired 96,000-row speed, covariance, or accepted RSS-growth result exists.

The forced-CMG composition was then exercised locally on the same deterministic
6,000-, 24,000-, and 96,000-row designs before public admission. It completed
all three cases with maximum projection complete residuals below `9e-11`, zero
PSD cleanup, worst projection-solve iteration counts of 17, 38, and 79, and
reported projection memory forecasts of 2.8, 10.1, and 39.6 MB. On the
6,000-row common Counter-V1 fixture, forced CMG and diagonal PCG differed by at
most `2.28e-11` across projection coefficients and covariance entries. These
local observations establish convergence and formula-path invariance, not a
same-host KSS Matlab speed comparison or new cross-platform performance claim.

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
calculation for component inference is the KSS Matlab-compatible
high-rank approximation; it is not the separately derived fully unbiased
leave-three-out variance estimator.

The source-bound comparison in
[archived inference/KSS Matlab qualification record](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference)
separates exact projection validation from descriptive component-SE evidence.
The additional KSS Matlab routine `leave_out_COMPLETE.m` returns only three marginal component standard
errors, not the joint covariance or total-target uncertainty. It also retains
materially negative local-fit predictions that FEVC rejects, so its component
standard errors are not treated as a parity gate.

Parallel declared match blocks at one worker--firm coefficient cell use the
original-deletion-unit population contract in `ESTIMATOR_CONTRACT.md`. Exact
Mata projection and current Rust projection inherit that partition. Native
match-component inference counts distinct deletion units, including for workers
at one model firm, while retaining its fixed-offset, mover-only contract.
Old native runtimes without deletion-unit readiness return
`RUST_PARALLEL_DELETION_UNSUPPORTED` for strict requests before preparation/RNG.
Engineering parity tests do not extend historical inference-coverage claims.

## Numerical precision of the main estimates

`mcse(all)` is the default and may accompany any otherwise supported inference
or projection request. The reported numerical MCSE concerns the four main
point estimates only. Sampling covariance, intervals, projection coefficients
and their extra simulations receive no numerical MCSE. Their capability and
scientific gates are unchanged. Use `mcse(off)` to skip the main all-probe
diagnostic; numerical covariance is never substituted for `e(V)`.
