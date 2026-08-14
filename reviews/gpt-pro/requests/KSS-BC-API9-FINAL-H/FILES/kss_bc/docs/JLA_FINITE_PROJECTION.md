# Improved-JLA finite-projection correction

Let one indexed random projection produce a squared projection statistic `U`
and its paired squared residual statistic `V`. For an uncollapsed coordinate,
`U=(Pq)_i^2` and `V=(Mq)_i^2`. With `R` independent directions,

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

The displayed correction retains the terms of order $R^{-1}$. In the
implementation, $P_i$, $M_i$, and the three raw fourth moments inside
$B_i$ and $V_i$ are replaced by their estimates from the same probes.
Because $B_i$ and $V_i$ already carry the factor $R^{-1}$, those plug-in
errors are $O_p(R^{-3/2})$ when the true residual share stays away from zero.
They therefore do not create another term of the displayed order. This is a
second-order expansion, not an exactly unbiased finite-$R$ identity.

There is no factor two on the mixed raw fourth moment in `B_i`. Symbolic
substitution makes the residual from the displayed bias formula exactly zero.
For a rank-one projection with `P=.2` and `M=.8`, the coefficient-one
prediction for `R(E[bar M]-M)` is `-.07872`; a fixed-seed 600,000-replication
simulation at `R=25` gives `-.078956` with MCSE `.001860`. The maintained
MATLAB coefficient-two expression predicts `-.02208`.

## Literal-frequency probe aggregation

Let stored row $r$ represent $f_r$ identical physical copies and write

\[
S_r=\sum_{a=1}^{f_r}q_{ra},
\qquad q_{ra}\stackrel{\mathrm{iid}}{\sim}\operatorname{Rad}(1/2).
\]

The implementation draws and retains one sign for every physical copy during
the observation-leverage pass. It sums those signs to $S_r$ before each
matrix-free solve, so it never expands the regression design. The resulting
probe stream is draw-for-draw equivalent to applying the same algorithm to
literal expanded rows after canonical ordering.

Equivalently, let $K$ map a weighted stored coordinate to its physical copies
with entries $f_r^{-1/2}$ on copy set $r$. Then

\[
K'K=I,\qquad X_{physical}=KX_{weighted},\qquad
P_{physical}=KP_{weighted}K'.
\]

The compressed physical probe is

\[
\xi_r=(K'q)_r=S_r/\sqrt{f_r},
\qquad E(\xi_r^4)=3-2/f_r.
\]

It is generally not a stored-row Rademacher sign. The runtime stores the
unnormalized $S_r$, which is algebraically equivalent because the weighted
design row is $\sqrt{f_r}x_r$ and
$X_{weighted}'\xi=\sum_r x_rS_r$. Thus its raw fourth moments are those of
the physical-copy law, including their frequency dependence.

With $H=\sum_r f_rx_rx_r'$, the projected value shared by every copy of row
$r$ is

\[
p_r=x_r'H^{-1}\sum_s x_sS_s.
\]

In particular, a stored weighted coordinate has diagonal leverage
$(P_{weighted})_{rr}=f_rh_{copy,r}$, while each physical copy has leverage
$h_{copy,r}=(P_{weighted})_{rr}/f_r$. The observation formulas below operate
on this per-copy quantity and then sum the $f_r$ identical deletion targets.

For literal observation deletion, the nonlinear constrained ratio and its
finite-projection adjustment are formed separately for every copy. The
runtime preserves that identity without storing an expanded design. Across
probes $t=1,\ldots,R$, define

\[
A_{2r}=\sum_t p_{rt}^2,\quad
A_{4r}=\sum_t p_{rt}^4,\quad
C_{1,ra}=\sum_t q_{rat}p_{rt},\quad
C_{3,ra}=\sum_t q_{rat}p_{rt}^3.
\]

The projection sums $A_{2r}$ and $A_{4r}$ are common to the copies of a
stored row. Only $C_{1,ra}$ and $C_{3,ra}$ require per-copy state. They recover
the five raw sums used by the delta calculation exactly:

\[
\begin{aligned}
\sum_t U_{rat} &= A_{2r},\\
\sum_t V_{rat} &= R+A_{2r}-2C_{1,ra},\\
\sum_t U_{rat}^2 &= A_{4r},\\
\sum_t V_{rat}^2 &= R+6A_{2r}+A_{4r}-4C_{1,ra}-4C_{3,ra},\\
\sum_t U_{rat}V_{rat} &= A_{2r}+A_{4r}-2C_{3,ra},
\end{aligned}
\]

where $U_{rat}=p_{rt}^2$ and $V_{rat}=(q_{rat}-p_{rt})^2$. The command forms
the constrained ratio, $B$, $V$, and inverse residual multiplier for every
copy. It averages those final multipliers within a stored row only after all
nonlinear operations. Because fitted outcomes, residuals, and target
projections are identical across copies, multiplying that average by $f_r$
is exactly the expanded-copy correction. Pooling copy residual squares before
the nonlinear ratio is a different finite-$R$ algorithm and is not used.

For a match $g$, set $F_g=\sum_{r\in g}f_r$. Its rank-one FE probe
contractions are exactly

\[
\pi_g=F_g^{-1/2}\sum_{r\in g}f_rp_r,
\qquad
\mu_g=F_g^{-1/2}\sum_{r\in g}S_r-\pi_g.
\]

Thus the squares, fourth powers, and squared cross-product of $\pi_g$ and
$\mu_g$ are the literal-copy moments used by the constrained block
calculation. Exact control-block action is added separately as described in
`BLOCK_CONTROL_DERIVATION.md`.

If explicit stored-row target mass is $t_r$, each physical copy has mass
$t_r/f_r$. With $T=\sum_rt_r$, define

\[
a_r=\sqrt{t_r/(f_rT)}S_r,
\qquad d_r=a_r-(t_r/T)\sum_sa_s.
\]

Then $d_r$ is the sum of the centered physical-copy target directions for
stored row $r$, and

\[
E(dd')=\operatorname{diag}(t/T)-(t/T)(t/T)'.
\]

The default $t_r=f_r$ therefore assigns equal mass to every physical copy.

## Conditioning and acceptance boundary

All probe expectations above condition on the fixed retained design, outcome,
frequency counts, target mass, and deletion partition. They are statements
about the unfiltered probe law. Production output is withheld if a constrained
denominator, residual leverage, or block residual maker fails its registered
positivity or conditioning gate. One failed requested block aborts the entire
command; no block is dropped and no partial target is returned. The
implementation does not claim that the second-order correction is unbiased
conditional on passing those gates, and
the expansion is not uniform as a true residual leverage approaches zero.
The reported numerical MCSE covers the independent target-probe average
conditional on the realized leverage sketch; it excludes leverage-sketch,
solver, and econometric sampling uncertainty.

Executable derivation and simulation tests live under `tests/python/`.
`test_frequency_probes.py` enumerates every Rademacher vector in a finite
expanded design and checks the copywise-observation sufficient statistics,
match contractions, and target aggregation identities direction by direction.
