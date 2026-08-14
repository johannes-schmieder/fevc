# Point-estimator contract

## Model, quotient, and target

On the retained sample, write the linear model as

\[
y=X\beta+\varepsilon,
\qquad X=[D\;F\;Z].
\]

`D` and `F` are worker and firm indicators and `Z` contains optional
controls. The implementation keeps all worker coordinates and drops one firm
coordinate. This is only a computational quotient. Every reported quadratic
target is invariant to the omitted firm and to relabeling the identifiers.

Let `W` contain positive integer frequency weights. With
\(\widetilde X=W^{1/2}X\), \(\widetilde y=W^{1/2}y\), and
\(H=\widetilde X'\widetilde X\), the full-sample estimate is

\[
\widehat\beta=H^{-1}\widetilde X'\widetilde y.
\]

For target masses \(a_r\), normalized to \(s_r=a_r/\sum_t a_t\), let
\(C_a=\operatorname{diag}(s)-ss'\). The worker, firm, and covariance target
matrices are

\[
Q_\alpha=D'C_aD,\qquad
Q_\psi=F'C_aF,\qquad
Q_{\alpha\psi}=\tfrac12(D'C_aF+F'C_aD).
\]

Controls have zero rows and columns in these matrices. The total target is
\(Q_T=Q_\alpha+Q_\psi+2Q_{\alpha\psi}\). The plug-in value for target `q`
is \(\widehat\beta'Q_q\widehat\beta\).

An explicit `targetweight()` is the complete mass of a stored row. Without
that option, a stored row receives mass equal to its frequency. Target mass
never enters the least-squares information matrix.

## General deletion block

For deletion unit \(g\), define

\[
P_g=\widetilde X_gH^{-1}\widetilde X_g',\qquad
M_g=I-P_g,
\]

and let \(\widehat e_g\) be the corresponding full-sample transformed
residual. If \(M_g\) is nonsingular, the residual at the deleted rows from
the regression fitted without `g` is

\[
\widehat e_{g,-g}=M_g^{-1}\widehat e_g.
\]

For a symmetric target matrix \(Q\), set

\[
B_{Q,g}=\widetilde X_gH^{-1}QH^{-1}\widetilde X_g'.
\]

The exact point correction implemented here is

\[
\widehat b_Q=\sum_g
\widetilde y_g'B_{Q,g}\widehat e_{g,-g},
\qquad
\widehat\theta_Q^{KSS}=
\widehat\beta'Q\widehat\beta-\widehat b_Q.
\]

Observation deletion treats each physical copy as its own unit. Match
deletion removes every physical copy carrying the declared `deletionid()`.
Different deletion IDs remain different units even when they share the same
worker--firm coefficient coordinate.

The dense backend forms `H`, factors it once, and evaluates each block with
inverse actions. It never refits the regression once per block. The
independent Python oracle physically expands frequencies and compares the
Woodbury residual identity with direct deleted regressions.

## Nuisance modes

Under `nuisance(joint)`, `Z` is part of `X`, and every deleted inverse allows
the nuisance coefficients to move. Under `nuisance(fixedoffset)`, the command
first estimates the full joint model, forms
\(y^*=y-Z\widehat\gamma\), and applies the two-way calculation to `y*` while
holding the fitted nuisance index fixed. The latter is a conditional
point-estimate convention. It is not joint leave-out estimation.

## Sample and dependence contract

Match mode uses a mover-only fit and mover-only target for all four headline
quantities. The command first selects a largest connected worker--firm
component, then iteratively removes worker articulation vertices and retains
the largest resulting component. This reproduces the conservative
leave-one-worker-connected convention used by the maintained MATLAB routine.
It is stronger than the graph condition needed for some individual
deletion units. The exact and JLA backends still test every requested block
for estimability, including failures introduced by controls.

Match deletion permits unrestricted dependence within a declared match and
treats distinct declared matches as independent. It does not permit arbitrary
dependence across all matches belonging to one worker.

`stayers(both)` is withheld. The package does not label an observation-level
fallback for stayers as a match-robust worker variance.

## Scope of the result

These formulas define point estimates. The command does not implement KSS
econometric inference, does not post `e(V)`, and does not turn projection
variation into a sampling standard error. Executable agreement with the
dense oracle is finite numerical evidence, not a proof that the identifying
assumptions hold in an application.
