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

This identity requires an identified full-rank representation of the FE
quotient and a positive-definite control Schur complement $S$. The runtime
checks the latter with an equilibrated inverse and a recomputed residual. It
does not use a generalized inverse or ridge for a failed nuisance block.

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

The displayed matrices use the frequency-compressed coordinates. Literal
copies of one stored row have identical outcomes, FE rows, and control rows.
On the block-constant subspace containing the transformed outcome and
residual, summing the copies is the isometry that maps a stored coordinate to
$\sqrt{f_r}$ times that coordinate. The orthogonal within-copy contrast
subspace has identity residual-maker action and zero contraction with these
vectors. Hence $C_g$, $v_g$, and the block solve above are exactly the
physical-copy calculation; they are not weighted-row deletion in place of
physical-copy deletion.

## Probe-independent joint-control rank certificate

Finite JLA leverage can underestimate a singular block with positive
probability. The randomized block gate therefore cannot by itself certify
that jointly estimated controls remain identified after every deletion.

Let `cell` denote the fitted worker--firm coordinate and let $G$ be the
weighted control scatter after removing a separate mean in every cell. For a
deletion unit $g$, recompute the affected cell mean after deletion and write

\[
G_{-g}=G-\Delta_g.
\]

The scatter loss $\Delta_g$ is positive semidefinite. When $G$ is positive
definite, whiten it and define

\[
L_g=G^{-1/2}\Delta_gG^{-1/2}.
\]

Then

\[
\lambda_{\max}(L_g)\leq\operatorname{tr}(L_g).
\]

The production JLA path requires

\[
\max_g\operatorname{tr}(L_g)<1
\]

by more than the registered numerical tolerance. In floating-point arithmetic
let

\[
\epsilon_W=\lVert W'GW-I\rVert_F,
\qquad
\epsilon_{round}=\max\{10^{-10},1000\,{\tt rank\_tolerance}\},
\]

where `W` is the computed Cholesky whitener. The runtime computes each scatter
with an explicit two-pass within-cell centering before whitening; it does not
subtract a large cell mean square from a large raw second moment. It computes
each scatter loss as the sum of the deleted within-scatter and the nonnegative
between-deleted-and-retained mean term, rather than by subtracting two nearly
equal total scatters. It reports the conservative lower bound

\[
\texttt{deletion_rank_gap}
=\min\left\{
1-\max_g\operatorname{tr}(W'\Delta_gW),
\min_g\lambda_{\min}\{W'(G-\Delta_g)W\}
\right\}
-\epsilon_W-\epsilon_{round}>0.
\]

For every deletion the low-dimensional matrix
`W'(G-Delta_g)W` is also formed directly, symmetrized, eigendecomposed, and
passed through the registered equilibrated inverse residual gate. The trace
test remains a conservative sufficient screen; the direct factorization is a
separate near-boundary guard and cannot be bypassed by a positive trace gap.

The inequality

\[
\lambda_{\min}\{W'(G-\Delta_g)W\}
\geq 1-\epsilon_W-\operatorname{tr}(W'\Delta_gW)
\]

shows why the measured whitening residual must be subtracted rather than
checked against a separate, potentially smaller gap. This condition implies
every $G_{-g}$ is positive definite at the registered numerical margin. If a nonzero
control combination became collinear with the retained two-way FE design
after deleting $g$, that combination would be constant within every retained
worker--firm cell and would have zero $G_{-g}$ scatter, a contradiction.
Together with the graph certificate for the FE block, the bound is therefore
a deterministic sufficient certificate for full deleted-design rank. It is
conservative: a valid design can fail the trace bound. Such a design is
withheld with `UNVERIFIED_DELETION_RANK` and requires `algorithm(exact)` or a
revised control parameterization.

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

## Rank, boundary, and conditioning scope

For every accepted block, both the true and evaluated residual-maker matrices
must be nonsingular. The implementation requires the evaluated symmetric
matrix to have minimum eigenvalue above `block_tolerance()` and requires its
equilibrated inverse to pass a residual check. The derivative expansion is
valid in a neighborhood of the true $h$ only while those inverses exist.
Its remainder is not uniform as the smallest residual-maker eigenvalue tends
to zero. The JLA constrained denominator must also be positive and finite;
observation-mode residual leverage must be strictly positive. A failure at
any of these boundaries returns a typed withholding status.

The bias and variance terms are computed under the unconditional probe law
conditional on the fixed inputs. Passing the numerical gates selects probe
realizations, so the implementation makes no claim of exact or second-order
unbiasedness conditional on acceptance. This limitation is separate from the
KSS independence assumptions and from econometric sampling inference.

## Fixed-offset mode

In `nuisance(fixedoffset)`, `C_g` is absent from the deletion calculation.
Controls affect `y*` through the full-sample nuisance estimate and are then
held fixed. The full joint fit must nevertheless identify that nuisance
index. The JLA backend therefore applies the same probe-independent
within-cell certificate before forming the offset. It conservatively also
requires the registered deleted scatters to retain rank, although fixed-offset
deletions do not re-estimate the controls. This is stronger than necessary but
prevents approximate FE-solve error from being interpreted as genuine
control variation. This mode remains deliberately distinct from the joint
deletion derivation above.

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
