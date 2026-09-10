# Point-estimator contract

## Model, quotient, and target

On the retained sample, write the linear model as

\[
y=X\beta+\varepsilon,
\qquad X=[D\;F\;Z].
\]

`D` and `F` are worker and firm indicators and `Z` contains optional
controls. The implementation keeps all worker coordinates and drops one firm
coordinate in the returned coefficient representation. The matrix-free solve
itself operates on the full firm Laplacian's zero-sum quotient and grounds the
displayed coordinates only after convergence. Every reported quadratic target
is invariant to the omitted firm and to relabeling the identifiers, up to the
registered numerical solve and roundoff tolerances.

Let `W` contain positive integer frequency weights. With
\(\widetilde X=W^{1/2}X\), \(\widetilde y=W^{1/2}y\), and
\(H=\widetilde X'\widetilde X\), the full-sample estimate is

\[
\widehat\beta=H^{-1}\widetilde X'\widetilde y.
\]

The exact nonnegative integer total of the frequency weights may not exceed
\(2^{53}\). The caller accumulates the total by testing the remaining exact
capacity before every addition. A larger total returns
`PHYSICAL_TOTAL_LIMIT` before component ranking. Thus every accepted component
mass, physical observation count, and `e(N_physical)` value is an exactly
represented integer; rounded component ties cannot select an ID-dependent
sample.

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
inverse actions. Near its numerical rank boundary it directly factors the
small deleted information matrix as an additional rank gate; it does not
refit outcomes once per block. The
independent Python oracle physically expands frequencies and compares the
Woodbury residual identity with direct deleted regressions.

## Nuisance modes

Under `nuisance(joint)`, `Z` is part of `X`, and every deleted inverse allows
the nuisance coefficients to move. Under `nuisance(fixedoffset)`, the command
first estimates the full joint model, forms
\(y^*=y-Z\widehat\gamma\), and applies the two-way calculation to `y*` while
holding the fitted nuisance index fixed. The latter is a conditional
point-estimate convention. It is not joint leave-out estimation.
In either convention, `Z` is the requested full control design after removing
only factor-variable terms that Stata explicitly labels omitted. An ordinary
zero or collinear numeric regressor is not silently dropped; it makes the full
model unidentified and the command withholds.
Before inverse actions, the implementation replaces `Z` by an ID-free
canonical basis for the same weighted column span. Controlled exact and JLA
paths first use the same order by outcome and per-copy target mass, without
raw controls or encoded IDs. An explicit complete and unique `probeorder()`
physical-observation key may refine exact ties. The command never infers this
key, and adding it does not reorder non-tied rows. A tied resulting semantic
key is accepted only when its
rows have identical controls and the same model coordinate and match block;
otherwise the affected backend withholds. The canonicalizer admits at most 32
controls, whitens the span, selects anchor rows using invariant row inner
products in that common order, and maps those rows to the identity.

The numerical certificate uses \(\gamma_m=m\epsilon/(1-m\epsilon)\), with
binary64 machine epsilon \(\epsilon\). A maximum-column inverse residual
\(r\) in dimension \(k\) becomes the subordinate-norm bound
\(\rho=\sqrt{k}r\), and the inverse contribution is withheld unless its
denominator is positive:

\[
E_{\mathrm{inv}}=\frac{\rho}{\operatorname{rcond}-\rho}.
\]

The initial envelope adds the measured whitening residual, the dimensioned
Gram term \(k\gamma_{2n}/\operatorname{rcond}\), a Cholesky term
\(k\gamma_{2k+1}/\operatorname{rcond}\), control-product rounding, and
\(E_{\mathrm{inv}}\). Every later pivot adds the anchor-inverse bound,
matrix-product rounding, the measured projector symmetry/idempotence error,
and the selected-anchor residual. Score and cutoff comparisons use four times
the resulting score-error envelope, with a `1e-12` floor. The completed basis
also checks its anchor identity and weighted-span reconstruction.

Finally, the basis envelope is propagated through the reciprocal-conditioning
margin \(r_*\) of the full fit and every relevant deletion certificate as

\[
E_* = \frac{E}{r_*-E}.
\]

An accepted controlled calculation requires each denominator to be positive
and each propagated value to be at most `1e-8`. A pivot near its eligibility
cutoff, more than 32 controls, an unresolved semantic tie, or an insufficient
full/deletion margin causes `AMBIGUOUS_CONTROL_BASIS`. On accepted inputs, an
invertible change `Z -> ZT` therefore generates the same canonical
right-hand sides within the registered control-forward, solve, and roundoff
gates.

## Sample and dependence contract

Match mode first constructs the certified mover graph. The command selects a
largest connected worker--firm
component, then iteratively removes insufficient histories and worker
articulation vertices and retains the largest resulting component. Each
distinct deletion ID is then represented as one edge in a deletion-unit
multigraph. Distinct IDs at the same worker--firm coordinate are parallel
edges, so neither is classified as a bridge merely because their common
coordinate is a cut edge. All deletion-unit bridges found in one pass are
removed simultaneously. Component, mover, insufficient-history,
articulation, and bridge pruning repeat to a fixed point. A final Tarjan
certificate requires zero retained deletion-unit bridges. Observation
deletion keeps the prior complete-case and leave-one-worker graph selector.
If components tie on firm count and physical mass, the command withholds
rather than select by an encoded ID; this keeps sample selection invariant to
identifier relabeling. The global \(2^{53}\) frequency-total gate makes those
physical-mass comparisons exact on every accepted input.
It is stronger than the graph condition needed for some individual
deletion units. The exact and JLA backends still test every requested block
for estimability, including failures introduced by controls.

Match deletion permits unrestricted dependence within a declared match and
treats distinct declared matches as independent. It does not permit arbitrary
dependence across all matches belonging to one worker.

With match deletion, `stayers(both)` is the default so that the target
population matches the KSS Matlab package. Let M denote the final
mover rows. Eligible stayers are workers who were one-firm stayers in the
frozen complete-case sample, are attached to a retained mover firm, and have
at least two literal physical observations. The command fits M and those
stayers jointly and normalizes every target over their pooled target mass.
Mover matches are deleted as declared blocks; each eligible stayer is
corrected by deleting one literal physical observation. The latter component
is explicitly not match-robust. `stayers(movers)` opts out and restores a
mover-only fit, target, correction, and `e(sample)`. If no stayer is eligible,
the default reduces exactly to that mover result. Observation deletion keeps
the retained-observation population and defaults to `stayers(movers)`.

## Scope of the result

These formulas define the point estimates and remain the default command
contract. Component `inference(highrank|q1)` defaults to the observation-only
exact Mata target-specific variance approximation; supported
explicit `inferencemodel(structured_common|structured_leverage)` attaches
matrix-free covariance inference to the unchanged generic-JLA point result.
Its explicit match tuple additionally requires `nuisance(fixedoffset)` and
`stayers(movers)`: control-estimation uncertainty is omitted, and a structured
model for independent aggregate-match variances is imposed. This is approximate
match inference, not joint-control or unrestricted KSS inference. It does not
change the default joint-nuisance, combined-population point estimator.
Fixed-effect `project()` instead inherits the point estimator's effective
deletion and population contract: omitted `deletion()` means declared mover
matches plus eligible-stayer observation units. Its block cross-fit covariance
permits unrestricted covariance within each mover match and independence
across deletion units. Point-only calls still post no `e(V)`, and JLA probe
dispersion remains numerical error rather than a sampling standard error. See
[`INFERENCE.md`](INFERENCE.md) for the derivation and normalization contract.

The plug-in row, correction row, and their final difference must each be
finite. A finite plug-in and finite correction do not by themselves authorize
posting: overflow in `plugin-correction` is withheld as
`NONFINITE_CORRECTED_TARGET`.
