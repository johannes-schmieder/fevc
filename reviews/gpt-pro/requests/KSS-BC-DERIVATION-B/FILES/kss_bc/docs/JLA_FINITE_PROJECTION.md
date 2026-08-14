# Improved-JLA finite-projection correction

Let one indexed random projection produce squared projection and residual
coordinates `U=(Pq)_i^2` and `V=(Mq)_i^2`. With `R` independent directions,

\[
\widehat P=R^{-1}\sum_r U_r,\qquad
\widehat M=R^{-1}\sum_r V_r,\qquad
\bar M=\frac{\widehat M}{\widehat P+\widehat M}.
\]

Write `P+M=1`, and let `m(P^2)=E[U^2]`, `m(M^2)=E[V^2]`, and
`m(P,M)=E[UV]`. For `f(p,m)=m/(p+m)`, evaluated at `p+m=1`,

\[
\nabla f=(-M,P),\qquad
\nabla^2f=
\begin{pmatrix}2M&M-P\\M-P&-2P\end{pmatrix}.
\]

The second-order delta bias is therefore

\[
B_i=\frac1R\{M_i m(P_i^2)-P_i m(M_i^2)
 +(M_i-P_i)m(P_i,M_i)\}.
\]

The corresponding delta variance is

\[
V_i=\frac1R\{M_i^2m(P_i^2)+P_i^2m(M_i^2)
 -2P_iM_i m(P_i,M_i)\}.
\]

The finite-projection multiplier applied to the leave-out residual variance
estimate is

\[
1-V_i/\bar M_i^2+B_i/\bar M_i.
\]

There is no factor two on the mixed raw fourth moment in `B_i`. Symbolic
substitution makes the residual from the displayed bias formula exactly zero.
For a rank-one projection with `P=.2` and `M=.8`, the coefficient-one
prediction for `R(E[bar M]-M)` is `-.07872`; a fixed-seed 600,000-replication
simulation at `R=25` gives `-.078956` with MCSE `.001860`. The maintained
MATLAB coefficient-two expression predicts `-.02208`.

Executable derivation and simulation tests live under `tests/python/`.
