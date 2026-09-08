# Residual-moment conditioning: bounded mathematical review

This is a read-only mathematical review prompted by the current Veneto
q0/q1 failures. It is not a repair, new experiment, coverage result or owner
approval to change the fitter. The owner-requested GPT-6 Astra Extra High
subagent independently checked the identities below. No human sign-off is
claimed, and no source files, thresholds or outcomes were changed by that review.

## The observed failure is not yet localized

Both current Veneto requests return rc=498 with
`SINGULAR_INFORMATION [observation_residual_moments]`. The implementation
calls `invert_scaled_spd` under that same phase for two distinct matrices:

1. the pre-probe basis matrix `Z'Z` in `residual_moments.rs`;
2. the estimated moment matrix after Gaussian probing.

The saved log therefore establishes a failed conditioning/identification
check, but not which matrix fails, whether its smallest eigenvalue is
negative, or whether a small positive eigenvalue misses the threshold.
Changing the later probe calculation cannot fix a failure of the first matrix.

## An equivalent population identity

Let `M=I-P` be an orthogonal residual projection and let `Z` be a fixed,
outcome-free variance basis. For independent standard Gaussian numerical
probes `xi`, define `q=Z'((M xi) ∘ (M xi))`. Then

\[
K=Z'(M\circ M)Z=\tfrac12\operatorname{Cov}_\xi(q).
\]

For `D_j=diag(Z[:,j])`, Gaussian quadratic-form covariance gives
`Cov(xi' M D_j M xi, xi' M D_k M xi) = 2 tr(M D_j M D_k) = 2 K[j,k]`.
Gaussianity is a condition on the numerical probes, not on outcome errors
for the underlying residual-second-moment equation.

Thus half the ordinary centered sample covariance of these contractions
is PSD in exact arithmetic and unbiased for `K`. It uses the same 512
projection solves, because `M xi = xi - P xi`. It preserves the population
moment target and the chosen variance basis, while changing the finite-probe
estimator.

The current calculation instead combines
`Z' diag(1-2 hhat) Z` with half the sample covariance of `Z'(P xi)^2`.
With exact projection actions and Gram probes independent of the basis and
JLA estimates, its conditional expectation is

\[
K-2Z'\operatorname{diag}(\widehat h-h)Z.
\]

Approximate leverage and cancellation can therefore make the current estimate
indefinite. The residual-probe representation removes this particular
mechanism. It does not eliminate leverage/target approximation in the basis
or leverage-dependent prediction flooring.

## What PSD would not establish

For any vector `a`,

\[
a'Ka=\|M\operatorname{diag}(Za)M\|_F^2.
\]

Full column rank of `Z` does not imply that this expression is positive for
every nonzero `a`. With residual rank `r`, `rank(K) <= r(r+1)/2`. A centered
512-probe sample covariance also has rank at most 511. With positive-definite
population `K`, ideal Gaussian probes and at least `p+1` probes for `p`
columns, the sample covariance is PD almost surely, but that supplies no
useful lower bound for the existing numerical-conditioning gate.

Let `A=M diag(Za) M`. For the direct half-sample-covariance estimate with `R`
independent probes, the directional variance is

\[
\operatorname{Var}_\xi(a'\widehat K_Ma)
=\frac{12}{R}\operatorname{tr}(A^4)
+\frac{2}{R-1}\{\operatorname{tr}(A^2)\}^2.
\]

At 512 probes, its relative numerical SD in a fixed nonzero direction is
approximately 6.3–16.5 percent, depending on that direction's spectrum.
It need not be more precise than the current decomposition. Unbiasedness
before inversion does not give unbiased coefficients after inversion, and
flooring is another nonlinear operation. None of these identities supplies
a coverage or availability guarantee.

Approximate projection actions also matter. If the residual action were a
fixed matrix `B`, the covariance identity would involve
`Z'((BB') ∘ (BB'))Z`, not automatically the desired `K`. Adaptive iterative
solves need not define one fixed linear matrix. Existing complete-system
residual checks must remain; PSD of the computed covariance alone would not
certify its statistical target.

## Next owner decision

Authorize at most a diagnostic replay of the saved Veneto input to identify
the failing stage, active columns, and scaled eigenvalue/conditioning bounds
for each reached small matrix. Preserve the original failure and gates.
If the run reaches the Gram stage, the equivalent residual-probe calculation
can be evaluated on those same probes. If `Z'Z` fails first, it is not a remedy.

This recommendation does not authorize ridge, a tolerance change, a different
variance model, new outcome draws, or automatic promotion. The paper's
current-method formulas continue to describe the implemented calculation;
the alternative above is a possible diagnostic, not a software feature.
