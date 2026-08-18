# CMG API 7 mathematical contract

## Status

- Component: package-owned CMG API 7
- Runtime target: Mata 18/19
- Statistical effect: none; this is a numerical preconditioner contract
- Numerical basis: unchanged source-informed API-6-qualified core
- Review status: internally challenged; no human-independent status

## Fine operator and exact hybrid realization

For unique worker--firm weights `a_wf>0`, set `d_w=sum_f a_wf`. The exact firm
Schur complement is

\[
S=\sum_w S_w,
\qquad
S_w=\operatorname{diag}(a_w)-a_wa_w'/d_w.
\]

For every firm vector `x`,

\[
x'S_wx=\frac{1}{2d_w}\sum_{f,g}a_{wf}a_{wg}(x_f-x_g)^2.
\]

Thus `S_w` is a positive-semidefinite clique Laplacian with edge conductance
`(a_wf/d_w)*a_wg`. Degree-one workers contribute zero. Degrees two and three
are materialized as one and three clique edges. Degree-four-or-higher workers
remain auxiliary star centers with spoke weights `a_wf`. The auxiliary block
is diagonal with entry `d_w`, and eliminating it gives exactly `S_w`.

Consequences that production tests must check independently:

- hybrid edges are no more numerous than unique cells;
- hybrid vertices are at most `F+C/4`;
- constants/components are exactly the Laplacian nullspace in exact
  arithmetic;
- no high-mobility worker produces quadratic storage.

## Galerkin hierarchy

Every fine/coarse level is an ordinary positive weighted graph. Binary
aggregate prolongation `P` and restriction `P'` define

\[
K_c=P'KP.
\]

Endpoint contraction and exact duplicate summation implement this identity.
The hierarchy may not merge components, discard positive cross-aggregate
edges, add a ridge, or use a non-Galerkin coarse operator.

API 6 uses the official CMG forest-profile architecture on hybrid and sparse
quotient levels. Each vertex nominates its maximum-weight incident edge under
a canonical total tie-break. Mutual pairs are rooted canonically, forest depth
is bounded by deterministic cuts, branches with more than two forest vertices
on both sides are detached, and a branch is repaired when its retained-tree
incident conductance is less than one eighth of its graph degree. Pointer
jumping then produces dense component-contained aggregate labels. The Mata
port uses bulk sorting, panels, indexed sums, and bounded pointer jumping; it
uses no compiled helper.

For a dense ordinary quotient with no hybrid auxiliary and `E/V>8`, API 6
selects API 5's screened forest. This avoids an algorithmically redundant
profile-and-repair pass in the degree-two and degree-three KSS regimes. If the
selected primary proposal reduces component surplus `V-C` by less than 20%, a
bounded fallback forms a deterministic maximal matching ordered by normalized
heavy-edge score

\[
\eta_{uv}=\frac{w_{uv}}{\sqrt{d_ud_v}},
\]

with raw weight and canonical endpoint keys as strict tie breakers. Unmatched
vertices are packed by canonical key only within their certified component,
with aggregate size at most eight. Aggregate connectivity is not required for
the Galerkin identity: component containment and binary `P` are sufficient.
The fallback changes only `P`; it changes no conductance and introduces no
regularization. The same exact contraction constructs `K_c`.

Reduction is measured as

\[
\rho=1-\frac{V_c-C}{V-C},
\]

with singleton-only graphs assigned `rho=1`. Every nonterminal committed level
requires `rho>=0.20`, the existing cumulative edge/vertex complexity bounds,
and unchanged component count. The hierarchy may attempt at most 96 levels.
These finite guards are construction contracts, not convergence-rate claims.

## Symmetric smoother and V-cycle

Let `C` contain component indicators, `D=diag(K)`, and

\[
R_0=D^{-1}-D^{-1}C(C'D^{-1}C)^{-1}C'D^{-1},
\qquad R=(2/3)R_0.
\]

`R_0` is symmetric positive semidefinite, annihilates `C`, and is positive
definite on `range(C)^perp`. It also satisfies `R_0 D R_0=R_0`. For a graph
Laplacian, `K <= 2D`, so

\[
RKR\preceq 2(2/3)R,
\qquad
2R-RKR\succeq 2(1-2/3)R.
\]

With a quotient-SPD child map `B_c`, the one-child pre/post cycle is

\[
B=2R-RKR+(I-RK)PB_cP'(I-KR).
\]

The second term is a symmetric congruence and the first is quotient-PD.
Therefore `B` is symmetric and positive definite on the quotient. This
induction authorizes ordinary PCG only while all of the following remain true:

- zero initial state for every application;
- fixed one pre- and one post-sweep;
- identical/adjoint pre and post operations;
- exactly one fixed recursive child application;
- exact `P'` restriction and `P` prolongation;
- fixed symmetric coarse solve; and
- no RHS-dependent hierarchy, sweep count, stopping rule, or warm start.

This induction is independent of how a valid component-contained binary
aggregation was selected. The API 6 profile, dense-quotient specialization,
and fallback therefore preserve the same
fixed linear symmetric quotient-SPD apply. It does not authorize additional
recursive calls, low-degree elimination, or a non-Galerkin lift.

The published KMT multi-call recursion is outside this v1 contract.

## Coarse solve

For each component, delete one stable ground with insertion matrix `E`. Let
`Pi` be the Euclidean component-compatibility projector. The coarse map is

\[
B_c=\Pi E(E'KE)^{-1}E'\Pi.
\]

The grounded inverse is applied through a stored diagonally equilibrated
Cholesky factor. Projections occur on both sides. Factor failure is a typed
setup failure, not a reason to regularize.

## Package pullback

Let `Q` inject firm coordinates into the hybrid graph.

KSS uses the component quotient congruence

\[
B_{KSS}=\Pi_FQ'B_KQ\Pi_F.
\]

## Numerical acceptance boundary

The CMG core supplies a preconditioner result, not a convergence certificate.
Each package retains its original exact Schur action, recurrence gates,
worker reconstruction, and fresh complete normal-equation residual. A small
same-precision residual remains an operational backward-error check, not a
forward-error theorem.
