# Joint controls and finite-projection block deletion

Status: candidate-complete derivation for adversarial model review.

## Orthogonal projection split

Work with frequency-transformed rows. Let `A` be the identified two-way FE
design and `Z` the jointly estimated controls. Write

\[
P_A=A(A'A)^{-1}A',\qquad R_A=I-P_A,
\qquad U=R_AZ,
\qquad S=U'U.
\]

If `S` is nonsingular, the projection for `[A Z]` decomposes exactly as

\[
P_{[A,Z]}=P_A+US^{-1}U'.
\]

The two terms are orthogonal because \(A'U=0\). The implementation obtains
the first projection through the worker-eliminated mobility system and the
second through the exact low-dimensional Schur complement `S`.

## FE part of a match block

All rows of an actual match have the same unweighted FE row `x`, although
controls may vary. If its stored rows have frequencies \(f_r\), define

\[
F_g=\sum_{r\in g}f_r,
\qquad
v_g=F_g^{-1/2}(\sqrt{f_r}:r\in g).
\]

The FE projection restricted to the match is rank one:

\[
(P_A)_{gg}=h_gv_gv_g'
\]

for a scalar block leverage \(h_g\). The control contribution is retained as
the full matrix

\[
C_g=U_gS^{-1}U_g'.
\]

Thus the full block residual maker is

\[
M_g(h)=I-C_g-hv_gv_g'.
\]

No within-match mean replacement is made for controls. This is why controls
may vary arbitrarily inside a match subject to the rank and block-leverage
gates.

## Delta adjustment for estimated block leverage

The improved JLA supplies \(\widehat h=1-\widehat m\). Let the registered
second-order bias and variance of \(\widehat m\) be `B` and `V`. Therefore

\[
E(\widehat h-h)\simeq-B,
\qquad
\operatorname{Var}(\widehat h)\simeq V.
\]

For a fixed transformed residual vector `e`, define

\[
g(h)=M_g(h)^{-1}e.
\]

Differentiating the inverse gives

\[
g'(h)=M^{-1}v(v'M^{-1}e)
\]

and

\[
\tfrac12g''(h)=
M^{-1}v(v'M^{-1}v)(v'M^{-1}e).
\]

A second-order expansion of \(g(\widehat h)\) has bias

\[
-B g'(h)+V\,\tfrac12g''(h).
\]

The production adjusted deleted residual is consequently

\[
\begin{split}
g_{bc}={}&M(\widehat h)^{-1}e
+B M(\widehat h)^{-1}v
  (v'M(\widehat h)^{-1}e)\\
&-V M(\widehat h)^{-1}v
  (v'M(\widehat h)^{-1}v)
  (v'M(\widehat h)^{-1}e).
\end{split}
\]

The plus sign on the `B` term follows from the fact that `B` is the bias of
the residual share, while the block maker is parameterized by the projection
share. The executable derivative test checks both derivatives by centered
finite differences. A separate exact enumeration and fixed-seed simulation
shows that this adjustment reduces block-inverse bias in a block with varying
controls.

## Observation deletion with controls

For observation `r`, let \(c_r=(US^{-1}U')_{rr}\). The full residual leverage
is

\[
m_r^{full}=m_r^{FE}-c_r.
\]

The control term is computed exactly. Applying the same second-order
expansion to the reciprocal gives

\[
\frac1{\widehat m^{full}}
+\frac{B}{(\widehat m^{full})^2}
-\frac{V}{(\widehat m^{full})^3}.
\]

This expression and the block formula are withheld whenever the estimated
full residual maker is singular, nonpositive, nonfinite, or fails the
registered block tolerance.

## Fixed-offset mode

In `nuisance(fixedoffset)`, `C_g` is absent from the deletion calculation.
Controls affect `y*` through the full-sample nuisance estimate and are then
held fixed. This mode is deliberately distinct from the joint derivation
above.

## Evidence and limits

The candidate derivation is checked by:

- an independent dense FWL projection identity;
- exact general-block KSS calculations with within-match control variation;
- finite-difference checks of both inverse derivatives;
- a fixed-seed finite-projection bias simulation; and
- Stata JLA convergence toward the dense joint-control result.

These are finite algebraic and numerical checks. The second-order adjustment
does not make a finite-probe ratio exactly unbiased, and none of these checks
establishes econometric sampling validity.
