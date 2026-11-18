# Inference

`fevc` 0.6.0 reports standard errors for the four components (worker
variance, firm variance, worker–firm covariance and variance of the sum) by
one method, the repeat-measurement model below. It is the default,
`inference(auto)`: every component is reported with a normal interval or
the `q = 1` interval of KSS, chosen from the data, and `e(V)` is posted when
every component is normal. `inference(highrank)` asks for normal intervals
only and `inference(q1)` for the `q = 1` interval wherever it exists; both
are strict and stop when the standard errors cannot be computed, where the
default returns point estimates with a reason. In the unreleased 0.6.1
source all three modes find the leading directions and report their shares
in one table (see [Display](#display)). `inference(none)` gives point
estimates only. `inferencemodel(repeat_measurement)` names the method and
may be omitted; `inferencedraws()` and `inferenceseed()` set its
pseudo-noise draws and their seed. Block projections of the fixed effects
(`project()`) have their own leave-out covariance; with `project()` the
default reports point estimates for the components, and explicit
`inference(highrank|q1)` with `project()` stops with
`COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED`. The variance models of
0.5.2 and the development model `leaveout_residual` were removed; see
[Removed in 0.6.0](#removed-in-060). The numerical MCSE of the point
estimates is a separate diagnostic, with the conventions in
[CENTERING.md](CENTERING.md).

## Repeat-measurement standard errors

This model, with one interval per component, is the default and the only
variance model of the command: `inference(auto)`, `inference(highrank)` and
`inference(q1)` use it, and `inferencemodel(repeat_measurement)` names it.
It runs one pass on the deletion units (under match deletion the
collapsed mover matches and one unit per physical stayer observation; under
observation deletion every physical observation), with three influence
solves and four solves per pseudo-noise draw (the fit of the draw and its
three kernel images). The variances and the covariance are:

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
as good as that model; estimated effects enter the profile, and centering
is treated as described [below](#centering-routes-and-the-reported-covariance). Observation
deletion needs joint nuisance and, natively, unit frequencies. The
derivation is in the method note of November 13, 2026 and its control-term
addendum of November 14, with the prototype's code and evidence, in
[`fevc/docs/repeat_measurement/`](repeat_measurement/README.md), whose
README lists where the command differs from the note. The
[unified document](repeat_measurement/unified/se_unified_20261116.pdf) of
November 16, 2026 combines the note and the addendum with the
implementation in 0.6.0 and a comparison with the standard errors of KSS
and of the KSS Matlab package. The
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

All three modes compute the covariance above and add, for each of the four
reported targets, the leading direction and the `q = 1` decomposition along
it; `inference(highrank)` reports the normal interval of every component
and uses the directions only for their shares. (In 0.6.0
`inference(highrank)` skipped the search; its standard errors and `e(V)` are
the same either way: on the six routes of `ex2` and `ex4` checked for
0.6.1, every element of `e(component_inference)`, `e(V)` and `e(kss)`
matched the 0.6.0 source bitwise.) The search never stops
`inference(highrank)`: when the run that carries it stops (a failed search
solve, a direction or a decomposition that fails its own checks, a native
error in the attachment), the command says so and runs once more without
the search, which is the high-rank pass of 0.6.0. The normal intervals,
standard errors, `e(V)` and status are then those of 0.6.0, every target is
recorded with status 7 (no direction) in `e(inference_leading)` and
`e(inference_intervals)`, `e(inference_leading_failure)` names the status
of the stopped run, and the display shows no lead share. A direction the
search did not resolve within its cap is recorded as unconverged, as under
`inference(auto)`; `inference(highrank)` then shows no share for it.
`inference(highrank)` stops, as in 0.6.0, only when a normal interval
itself is unavailable: on the Mata routes when a component variance
estimate is not positive (`INFERENCE_VARIANCE_INVALID`); natively such a
component is withheld and the status is `FEVC_REPEAT_MEASUREMENT_SE_PARTIAL`.
`inference(q1)` needs the directions and stays strict about the search.

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
- the interval is the Andrews-Mikusheva interval of KSS (Section 7.1): the
  range of `lambda b^2 + theta_r` over the ellipse of radius `z(kappa)`
  around the estimates, with `kappa` the curvature
  `2 |lambda| V[b] / (V[theta_r] (1 - rho^2))^(1/2)` and `z` its critical
  value, simulated with 100,000 draws.

One more linear solve per target gives the remainder's influence and one per
target and draw its kernel, so the request adds the search, 4 and 4 per draw
to the 3 and 4 per draw of the pass: about 90 solves with the default eight
draws where the pass alone takes 35. This is the cost of every mode; in
0.6.1 `inference(highrank)` runs the search too, about 55 solves more than
the 35 it took in 0.6.0 (the native plan reserves the search at its cap of
40 solves per target).

The reported interval is chosen component by component, by one rule on every
route (`fevc__residual_se_post.ado`). With `inference(auto)` a component
receives the `q = 1` interval when its leading direction carries more than
ten per cent of the estimated sampling variance,
`(4 lambda^2 max(b^2 - V[b], 0) V[b] + 2 lambda^2 V[b]^2) / V-hat`, or when
the high-rank variance estimate is not positive, and the normal interval
otherwise; the variance of the sum, whose nonzero eigenvalues are all equal,
keeps the normal interval. With `inference(q1)` every target receives the
`q = 1` interval when it exists. With `inference(highrank)` every target
with a positive variance estimate receives the normal interval, whatever
its share, and `e(V)` is posted whenever the joint covariance is
admissible. A `q = 1` component has no standard error,
and `e(V)` is posted only when every component has a normal interval.
Neither KSS nor the technical note has a result for a data-dependent choice
between the two intervals, and the case of two or more dominant directions
is not covered. The share is a ratio of two estimates and can exceed one;
the display then says ">100%".

### Display

The command shows one table of the components with the same columns in
every mode: estimate, standard error (blank for a `q = 1` component),
the interval at `level()`, which interval it is (`normal` or `q = 1`,
marked `*` when `inference(highrank)` or `inference(q1)` requested it) and
the lead share, the statistic of the rule above (`variance_share`, column
10 of `e(inference_intervals)`; `.` for the variance of the sum, to which
the rule does not apply, and under `inference(highrank)` for a direction
the search did not find or resolve, with the line "Lead share .: no
converged leading direction (see estat inference)"). The heading states the rule ("q = 1 interval when
the lead share exceeds 10%") or the request. Under it at most a few lines
say what the lead share and the `q = 1` interval are, whether `e(V)` is
posted and why not, and, after `inference(highrank)`, how many components
the default rule would report with the `q = 1` interval. The display shows
no p-values; `test` and `lincom` give them from `e(V)` when it is posted.

`estat inference` shows the rest, component by component: the
normal-form standard error and interval, the `q = 1` interval with its
curvature, critical value and status, the interval reported and why (the
share against the threshold, a variance estimate that is not positive, a
`q = 1` interval that does not exist, or the request), the lead share,
`lambda1` and `lambda2` (columns 5 and 6 of `e(inference_leading)`), KSS's
Lindeberg statistic `max w_i1^2` (`max_weight_sq`), the effective rank, and
the convergence and solves of the search; then the method (the
repeat-measurement sets, the fitted model for movers, the independence
assumption, the `q = 1` interval) and the warnings that apply: a low
effective rank for a component that keeps the normal interval, a leading
direction that rests on few deletion units, an unresolved search, a `q = 1`
interval that is needed but could not be formed.

The numerical MCSE of the point estimates is one line under the
decomposition table in every mode, including `inference(none)`, for
example `Numerical MCSE (200 projections): worker .00043  firm .00020  cov
.00027  sum .00025; at most 8% of a standard error.` (two lines when it
does not fit in 80 columns). The clause divides each MCSE by the standard
error of its component, the normal-form one for a `q = 1` component, and
is left out without standard errors. Above 10% the display names the
components and suggests more probes, `probes(2000)` or ten times the
current number, whichever is larger; the MCSE falls with the square root
of the number of probes. With exact leverages the line says that there is
no numerical MCSE from projections. `estat diagnostics` keeps the method,
status, availability, conditional values and exclusions. The MCSE is
computational precision, not a standard error.

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

The same term shows where the two blocks of firms differ by three log
points and the control loads on the firm effect (`dev6_shapes`,
`dev7_adv`; three designs, 250 replications per case): under the default
joint nuisance the standard error of the worker variance, and in two of
the three designs that of the firm variance, is 0.87 to 0.94 of the
spread, against 0.95 to 1.02 for the same design shapes without controls.
In two of the designs these components take the `q = 1` interval; the
reported intervals of the worker and firm variances cover 0.924 to 0.956.
Under `nuisance(fixedoffset)` the two designs of `dev6_shapes` give 0.92
to 0.99 (`dev7_fixedoffset`, match deletion). A control loaded on the
worker effect, or unrelated to the effects, shows nothing of the kind in
designs with that gap (0.97 to 1.13). Over the ten further design shapes
of `dev6_shapes` (5,000 fits, none failed) the lowest coverage is 0.924
and the mean 0.962; without a bottleneck, `e(V)` is posted in 65% to 92%
of the samples (`dev5_posting`, three designs with 400 firms).

These are development checks, not confirmations.

*Confirmation.* A registered confirmation (`conf3_controls`, manifest
sha256 `fcb4ffa0...93e4`, frozen before any draw) ran the plain default
command on the release-candidate source (`f3938986`) with its qualified
macOS arm64 plugin: the twenty design shapes, the replications and the
tolerances of `conf2`, new design and run seeds, and controls in every
design, in a fixed rotation of four kinds (one control correlated with
both effects; that control with a second one trending with the year; one
unrelated to the effects; one loaded on the worker effect). A control
loaded on the firm effect appears only where the two blocks of firms do
not differ; that assignment rule was fixed from the development checks
above, before any confirmation draw. Verdict PASS: 40,000 fits, none
failed, none returned point estimates; over the 160 cases (design,
deletion mode, component) the lowest coverage of the reported interval is
0.923 and the mean 0.960; the variance of the sum covers 0.940 to 0.954;
without a bottleneck the reported interval is at most 1.3% longer than
the normal one and `e(V)` is posted in 88% to 98% of the samples. The
lowest cases are the worker and firm variances (0.923 and 0.926) in a
design whose blocks differ, with the control loaded on the worker effect,
where their standard errors are 8% and 7% short. Not covered by design: a
control index as large as the effects, error variances that move with a
control inside a cell, and a control loaded on the firm effect where the
blocks differ. The release source differs from `f3938986` by the removal
of the other variance models and two fixes outside the default's
computation; the default's results are bitwise the same (see the
compatibility note in `docs/default_interval/README.md`).

### Centering, routes and the reported covariance

*Centering.* Under `centering(mean)`, the default, the estimate subtracts
the retained frequency-weighted mean of the outcome (net of the fitted
controls under `nuisance(fixedoffset)`). The pseudo-noise is centred in the
same way before it enters the kernel, so the kernel is that of the centred
estimate and no approximation that holds the observed mean fixed is needed;
`e(inference_mean_omitted)` is zero. `centering(none)` uses the uncentered
kernel. Under `centering(corrected)` the standard error is that of the
mean-centred estimate and treats the corrected-centering increment as
fixed, as the numerical MCSE does. `e(inference_centering)` records which.

*Routes.* The native route attaches the pass to generic JLA under
Counter-V1 with the automatic, diagonal or CMG solver route. A mover-only
request without controls runs on the generic engine when standard errors
are attached, and under joint nuisance with controls one further target
pass of two solves per point-estimate probe forms the fixed-offset units.
Native requests need readiness bit 18 (`r(component_residual_api) == 1`),
which the pass introduced, bit 19 (`r(component_repeat_api) == 1`),
`r(component_mixed_api) == 1` and, under `centering(mean)`,
`r(component_centering_api) == 1`; variance-source code 4 and the
diagnostic exports `vckss_rust_engine_leaveout_residual_diagnostic_v1` (of
the shared pass) and `vckss_rust_engine_repeat_measurement_diagnostic_v1`
are additive. The plugins distributed with 0.5.2 predate bits 18 and 19, so
with them the default returns point estimates and says why. The Mata routes
(`backend(mata)`, or `rng(stata)` under `backend(auto)`, with
`algorithm(jla)` or `algorithm(exact)`) carry the model without a plugin;
under `algorithm(jla)` the standard errors are a separate stage that draws
its own projections and costs about as much as the point estimates. The
native attachment has its own memory forecast, a lifetime tally of its
allocations. The pass (the pseudo-noise trace estimate, the control term,
cost and memory accounting) is written up in the
[technical supplement](standard_errors/standard_errors.tex) for the
leave-out residual method that shared it; that method's squared-residual
variances are not used.

*The reported covariance.* The pass estimates the covariance of the worker
variance, the firm variance and the covariance and maps it through
`total = worker + firm + 2 × covariance`, so `e(V)`, when it is posted, is
a coherent four-by-four matrix with the coefficient names of `e(b)`, and
Stata's standard `lincom` command works without a FEVC-specific wrapper.
For example,

```stata
lincom worker_variance + firm_variance + 2*worker_firm_covariance
```

reproduces the posted `total_variance` estimate and standard error. This is
distinct from the KSS Matlab function `lincom_KSS`, whose purpose is
fixed-effect projection inference and whose FEVC counterpart is `project()`.

*The `q = 1` critical value.* For curvature \(\kappa>0\), a critical-value draw is

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
its maintained assumptions. The fitted variance model and the
data-dependent choice of interval require their own conditions; the
reported interval does not inherit that guarantee automatically. It is
not an exact finite-sample quantile of the shortest distance to a particular
parabola, so design-specific conservatism is expected and is not removed by an
empirical critical value.

The high-rank form and the `q = 1` interval follow Kline, Saggio and
Sølvsten (2020); the variance model is FEVC's own. The implementation is
repository-authored MIT source. No MATLAB source, critical-value table, or
binary data are included.

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
`e(V)`: with `project()` the default reports point estimates for the
components, and `inference(highrank|q1)` with `project()` stops with
`COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED`.

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

`inferenceseed()` seeds the standard-error calculation: its pseudo-noise
draws, the `q = 1` critical-value simulations and, on the Mata
`algorithm(jla)` stage, its own projections. The point estimates use
`seed()`. The outer command restores the caller's RNG algorithm, stream, and complete
state on success and every supported failure. Repeating the same accepted
request with the same source, data, options, and inference seed is
deterministic.

A strict request (`inference(highrank)`, `inference(q1)` or `project()`)
is atomic with the point estimate: if the variance fit, covariance, leading
direction, projection, or runtime validation fails, no partial estimation
result is posted. Under the default a failure that can come from the
standard errors is followed by one more run without them, and
`e(inference_auto_reason)` names the status of the stopped run. Typed
failure statuses and the stored-return
contract are listed in [`FAILURES_AND_RETURNS.md`](FAILURES_AND_RETURNS.md).

## Interpretation boundary

The standard errors of the components estimate sampling uncertainty under
the repeat-measurement variance model: sample variances where deletion
units repeat a worker–firm cell and a fitted model elsewhere. They are not
the unrestricted leave-two-out construction of KSS. Projection inference
supports independent match blocks with unrestricted within-match covariance,
including the default mixed mover-match/stayer-observation population. On the
scalable projection route, JLA approximates the leverage and block-deleted
residual used by the covariance estimator; probe dispersion itself is not
reported as an econometric standard error.

The source-bound comparison in
[archived inference/KSS Matlab qualification record](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference)
separates exact projection validation from descriptive component-SE evidence.
The additional KSS Matlab routine `leave_out_COMPLETE.m` returns only three marginal component standard
errors, not the joint covariance or total-target uncertainty. It also retains
materially negative local-fit predictions that FEVC rejects, so its component
standard errors are not treated as a parity gate.

Parallel declared match blocks at one worker--firm coefficient cell use the
original-deletion-unit population contract in `ESTIMATOR_CONTRACT.md`. Exact
Mata projection and current Rust projection inherit that partition, and so
do the standard errors of the components.
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

## Removed in 0.6.0

On the owner's decision of 2026-11-14 ("Only the default";
[DECISIONS.md](DECISIONS.md)) the command offers one way to get standard
errors of the components. Removed from the command:

- `inferencemodel(target_lowess)`, the exact Mata observation-deletion
  model that `inference(highrank|q1)` selected without `inferencemodel()`
  in 0.5.2 (binned local-linear smoothing of leave-out variance proxies,
  with the observed mean held fixed);
- `inferencemodel(structured_common)` and `inferencemodel(structured_leverage)`,
  the experimental structured residual-moment models of 0.5.2 for
  observation deletion, fixed-offset match deletion and pooled movers and
  stayers, on Rust/JLA/Counter-V1 and exact Mata;
- `inferencemodel(leaveout_residual)`, a squared-residual (HC2) method
  added during development after 0.5.2 and never released; the
  repeat-measurement model runs its pass with different variances;
- `inferencesimulations()`, `inferencebins()` and `inferencegramprobes()`,
  which tuned only those models; and
- component inference and `project()` in one call, which 0.5.2 allowed on
  exact Mata.

A request for a removed model stops before estimation with
`INFERENCE_MODEL_REMOVED`, one for a removed option with
`INFERENCE_OPTION_REMOVED` (return code 198 for both), and explicit
`inference(highrank|q1)` with `project()` with
`COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED` (return code 498).
`inference(highrank)` and `inference(q1)` now select the intervals of the
repeat-measurement model; their results equal those with
`inferencemodel(repeat_measurement)`. The Mata and Rust machinery of the
removed models stays in the source, unreachable from the command. Their
derivations, assumptions, confirmations and failures (the fixed-observed-mean
approximation, the structured variance fit, the fixed-offset match
confirmations, the observation-`q1` calibration failure, the Mean component
assessment) are documented at the git tag `v0.5.2`, in
[INFERENCE.md at v0.5.2](https://github.com/johannes-schmieder/fevc/blob/v0.5.2/fevc/docs/INFERENCE.md);
the design records [POOLED_COMPONENT_INFERENCE.md](POOLED_COMPONENT_INFERENCE.md),
[MATRIX_FREE_COMPONENT_INFERENCE.md](MATRIX_FREE_COMPONENT_INFERENCE.md) and
[INDIVIDUAL_INFERENCE_INTERFACE.md](INDIVIDUAL_INFERENCE_INTERFACE.md) and
the dated assessments remain in this directory. The leave-out residual
method is described in the
[technical supplement](standard_errors/standard_errors.tex).
