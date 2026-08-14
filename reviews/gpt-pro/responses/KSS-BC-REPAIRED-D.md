---
review_id: KSS-BC-REPAIRED-D
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 7fc350e0434c2584ad553358f303b0eb09c85e487070321b696dd0bfc4c829a5
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7efb15-ee90-83ea-9f45-405684ff2c16
---

## Verdict

The complete reviewer response is preserved verbatim under Findings.

## Assumptions

Preserved verbatim under Findings.

## Counterexample search

Preserved verbatim under Findings.

## Repairs

Preserved verbatim under Findings.

## Uncertainty

Preserved verbatim under Findings.

## Findings

Verdict

FALSE.

The repaired implementation still contains a critical wrong-estimator defect in the public production path

algorithm(jla) deletion(observation) frequency > 1

The physical-copy Rademacher draws are generated correctly, but production computes the second-order ratio adjustment using the wrong fourth-moment object. It estimates the residual share from the within-stored-row average of copy residual squares, while supplying the delta calculation with the within-row average of copy residual fourth powers. The required object is instead the square of the within-row average of residual squares. The discrepancy survives at order R
−1
, so it violates the packet’s expressly permitted claim of a pointwise second-order finite-probe approximation; it is not merely a failure of exact finite-R unbiasedness. 

Pasted text

There is also an unresolved critical numerical false-acceptance problem in both rank-gating mechanisms. The exact-arithmetic trace theorem is valid and basis invariant, and its caller placement is correct. But the implemented tolerances provide no backward-error margin: at the default settings the allowed whitening residual is ten times the minimum accepted certificate gap. An explicit ill-conditioned change of control basis makes an exactly singular deletion pass an IEEE-754 reproduction of both the JLA certificate and the dense exact block gate, while a well-conditioned basis of the same control space is withheld.

The narrower conclusions are:

Production mode	Conclusion
Dense exact algebra, away from numerical rank boundaries	Symbolically correct
Dense exact rank gate	Numerically unresolved; potential false acceptance
JLA match deletion, correct rank event	Frequency law and block correction are symbolically correct
JLA observation deletion, all frequencies =1	Second-order formula is correct
JLA observation deletion, any frequency >1	False as implemented
Econometric inference or theorem transfer	Not reviewed and not claimed
Assumptions and scope

I used the packet’s fixed-input convention, interpreted frequency weights as literal identical physical copies, allowed arbitrary within-match control variation, and treated conservative withholding as permissible. I reviewed only the finite point estimators and their registered numerical gates, not sampling inference or the broader paper. 

Pasted text

The 14 delimited component bodies match the SHA-256 values in the packet’s source map. The chat-transport wrapper itself is not byte-identical to the asserted aggregate hash, so the findings below are bound to the verified delimited component bodies. 

Pasted text

Production control flow and caller coverage
Public caller

The ado layer correctly validates:

supported deletion, algorithm, nuisance, and stayer modes;

probe and tolerance ranges;

positive integer frequency weights;

nonnegative target mass with positive total;

complete frozen inputs for match deletion. 

Pasted text +1

It then constructs the graph deletion identifier, invokes graph pruning, replaces the estimation sample by the returned component, re-encodes identifiers, selects exact or JLA, and forwards all relevant tolerances and modes to Mata. 

Pasted text

A Mata result is posted only when mata_status=="CONVERGED"; all other returned statuses become WITHHELD. There is no caller retry with fewer probes, a different deletion unit, a looser tolerance, or another algorithm. 

Pasted text

 Success metadata, including the rank-certificate label and KSS_POINT_ESTIMATES_ONLY, is posted only after that check. 

Pasted text

FE graph certificate

The final match graph has:

one selected connected component;

no retained worker with only one fitted firm;

no retained worker articulation vertex. 

Pasted text

This is sufficient for FE match deletion. If an edge were a bridge, its worker endpoint would either have degree one or, if its degree exceeded one, be an articulation vertex. Both cases are removed. A declared match is also required to stay within one worker–firm coordinate. 

Pasted text

For observation deletion, a bridge represented by only one physical copy would similarly have either:

a degree-one worker with physical history one, which is removed; or

a worker of degree greater than one, which is an articulation vertex.

If a degree-one bridge has more than one physical copy, deleting one copy leaves the graph edge present. Thus the final FE graph does not falsely certify a singular one-copy FE deletion. The graph convention can nevertheless withhold valid multiplicity-heavy designs more aggressively than necessary, which is conservative rather than false acceptance.

Rank-gate placement

For joint controls, JLA prepares the full joint system and immediately invokes kssbc__joint_rank_certificate; the seed is not installed until after the certificate succeeds. 

Pasted text +1

The certificate contains separate match and observation calculations. 

Pasted text

 Therefore it cannot be bypassed by:

seed();

probes();

batch();

match versus observation deletion;

algorithm(auto), which simply routes to exact or JLA.

nuisance(fixedoffset) legitimately omits the certificate from the working deletion problem because controls are first fitted in the full model and then removed from the working design while their fitted index is held fixed. This is the separately documented conditional estimator, not joint leave-out estimation. 

Pasted text +1

The placement is therefore correct. The defect is in the numerical strength of the gates, not an obvious caller bypass.

Findings
F1 — Critical: wrong second-order physical-copy moment in observation JLA

Source: kss_bc/kss_bc.mata, observation first pass and reciprocal adjustment. 

Pasted text +1

For stored row r, let its f copies have signs q
a
	​

, let

S=
a=1
∑
f
	​

q
a
	​

,

and let p be their common projected value. Define

V
a
	​

=(q
a
	​

−p)
2
,
V
ˉ
=
f
1
	​

a
∑
	​

V
a
	​

.

Production correctly accumulates

V
ˉ
=1+p
2
−
f
2pS
	​


in m_first. Thus the implemented ratio uses

M
=
R
1
	​

j=1
∑
R
	​

V
ˉ
j
	​

.

The second-order bias and variance of that ratio require the raw second moment

E(
V
ˉ
2
).

But m_second accumulates

f
1
	​

a
∑
	​

V
a
2
	​

=1+6p
2
+p
4
−
f
4p(1+p
2
)S
	​

.

These are different:

f
1
	​

a
∑
	​

V
a
2
	​

−
V
ˉ
2
=
f
1
	​

a
∑
	​

(V
a
	​

−
V
ˉ
)
2
,

which is generally positive when f>1.

The packet’s physical-copy derivation correctly identifies the displayed formula as the average copy fourth power, but then calls it one of the quantities needed for observation deletion without reconciling it with the pooled residual-share ratio used in production. 

Pasted text

 The general delta formulas require the second moment of whatever random coordinate is actually averaged into 
M
. 

Pasted text

p_second=p^4 remains correct because all copies have the same projected value, and

p
2
V
ˉ
=
f
1
	​

a
∑
	​

p
2
V
a
	​


makes mixed_second correct. The defect is specifically m_second.

Exact admissible counterexample

Use the four stored K
2,2
	​

 rows

X=
	​

1
1
0
0
	​

0
0
1
1
	​

1
0
1
0
	​

	​

,W=diag(2,1,1,1),

with no controls and deletion(observation). The columns are worker 1, worker 2, and grounded-firm-1. The first stored row represents two physical copies.

The expanded design has five physical rows and rank three. Deleting any physical copy preserves rank:

deleting either first-row copy leaves the same worker–firm edge;

deleting any other edge leaves a connected three-edge tree.

For either duplicated copy, exhaustive enumeration of all 2
5
 physical Rademacher vectors gives

P=
7
3
	​

,M=
7
4
	​

,

and the moments

Moment	Correct pooled coordinate	Production input
E(U
2
)	993/2401	993/2401
E(U
V
ˉ
)	132/2401	132/2401
residual raw second moment	E(
V
ˉ
2
)=1378/2401	E[f
−1
∑
a
	​

V
a
2
	​

]=1672/2401

Consequently, before division by R,

B
true
	​

=−
16807
30
	​

,B
code
	​

=−
16807
912
	​

,

and

V
true
	​

=
117649
25122
	​

,V
code
	​

=
117649
27768
	​

.

The reciprocal adjustment in production is

M
1
	​

+
M
2
B
	​

−
M
3
V
	​

.

Using the production moments rather than the pooled-coordinate moments leaves the order-R
−1
 error

M
2
B
code
	​

−B
true
	​

	​

−
M
3
V
code
	​

−V
true
	​

	​

=−
32
9
	​

.

Thus the adjusted inverse has residual bias

−
32R
9
	​

+o(R
−1
).

This is exactly the order the repair claims to remove.

To establish that this changes the returned target rather than only an unused intermediate, take stored outcomes

y=(1,2,4,8)
′
.

The weighted residuals are

(3,−6,−6,6)
′
/7.

Under default physical target mass, the first row’s worker-target kernel diagonal is 24/1225. Because the other three rows have frequency one, their moment calculation is unaffected. The production worker correction is therefore wrong by

−
17150R
81
	​

+o(R
−1
),

with no cross-row cancellation.

The true residual share 4/7 is bounded well away from zero, so JLA acceptance probability tends to one as R grows. Conditioning on an exceptional gate event does not remove this unconditional order-R
−1
 discrepancy.

Required code repair

The observation branch should construct the actual pooled coordinate explicitly:

mata
vbar = 1 :+ projected:^2 :-
    2 :* projected :* rademacher_sum :/ frequency


m_first       = m_first       + vbar
m_second      = m_second      + vbar:^2
mixed_second  = mixed_second  + projected:^2 :* vbar

The current physical generator need not change.

A temporary fail-closed repair would be to withhold JLA observation deletion whenever any retained frequency>1.

F2 — Critical: the tests validate the wrong moment and miss the finite-R defect

The exhaustive frequency test verifies that the collapsed expression equals

f
1
	​

a
∑
	​

(Mq)
a
4
	​

.

That identity is true, but it is not the second moment of the pooled random coordinate used in production. The test never checks

{
f
1
	​

a
∑
	​

(Mq)
a
2
	​

}
2
.

Thus it affirmatively validates the wrong object for this estimator. 

Pasted text

The public Stata frequency tests compare JLA with exact only at R=4000, allowing discrepancies of six reported target-probe MCSEs plus 4×10
−4
. An order-R
−1
 error is small at that probe count and is not covered by the reported target-probe MCSE in any event. 

Pasted text +1

The necessary regression is a finite-R, direction-by-direction comparison between:

the compressed production statistic; and

a literal expanded-copy implementation of the same pooled ratio.

The four-row counterexample above supplies a minimal fixture.

F3 — Exact-arithmetic trace certificate is valid and sufficient

The mathematical certificate in the repaired derivation is correct.

Let G be total weighted within-cell control scatter. In an affected cell, split observations into deleted and retained subsets with masses F
D
	​

,F
R
	​

, weighted means μ
D
	​

,μ
R
	​

, and within-subset scatters S
D
	​

,S
R
	​

. If F
R
	​

>0, weighted ANOVA gives

G
cell
	​

=S
D
	​

+S
R
	​

+
F
D
	​

+F
R
	​

F
D
	​

F
R
	​

	​

(μ
D
	​

−μ
R
	​

)(μ
D
	​

−μ
R
	​

)
′
.

After deletion, that cell contributes S
R
	​

. Therefore

Δ
g
	​

=G−G
−g
	​

=S
D
	​

+
F
D
	​

+F
R
	​

F
D
	​

F
R
	​

	​

(μ
D
	​

−μ
R
	​

)(μ
D
	​

−μ
R
	​

)
′
⪰0.

If the entire cell is removed, Δ
g
	​

 is simply its original scatter and is again positive semidefinite.

Assuming G≻0, define

L
g
	​

=G
−1/2
Δ
g
	​

G
−1/2
⪰0.

Then

G
−g
	​

=G
1/2
(I−L
g
	​

)G
1/2
.

For a positive-semidefinite matrix,

λ
max
	​

(L
g
	​

)≤tr(L
g
	​

).

Hence

tr(L
g
	​

)<1⟹I−L
g
	​

≻0⟹G
−g
	​

≻0.

Finally, if a nonzero control combination became collinear with the retained two-way FE design, its value would be constant within every retained worker–firm cell. Its within-cell scatter would then be zero, contradicting G
−g
	​

≻0. Combined with FE rank after deletion, the trace condition is sufficient for full deleted-design rank. This is the argument stated in the derivation. 

Pasted text

Exact basis invariance

For a nonsingular control transformation Z
∗
=ZT,

G
∗
=T
′
GT,Δ
g
∗
	​

=T
′
Δ
g
	​

T.

Therefore

tr{(G
∗
)
−1
Δ
g
∗
	​

}
	​

=tr{T
−1
G
−1
(T
′
)
−1
T
′
Δ
g
	​

T}
=tr(T
−1
G
−1
Δ
g
	​

T)
=tr(G
−1
Δ
g
	​

).
	​


Thus the mathematical certificate is invariant to every nonsingular control basis change.

F4 — Critical unresolved numerical issue: the production certificate is not fail-closed or basis invariant

The exact theorem does not transfer to the implemented floating-point inequalities.

At the default rank_tolerance=1e-10 and block_tolerance=1e-10, the caller forwards those tolerances unchanged. 

Pasted text

The common inverse routine permits an inverse residual as large as

max(10
−10
,100rank_tolerance)=10
−8
.

Pasted text

The control certificate then permits

∥
W
′
G
W
−I∥
F
	​

≤max(10
−9
,1000rank_tolerance)=10
−7
,

but accepts a deletion-rank gap as small as

max(10
−10
,100rank_tolerance)=10
−8
.

Pasted text +1

Thus the permitted whitening discrepancy is ten times the minimum accepted gap. The code does not subtract a bound for:

whitening error;

inverse backward error;

cell-scatter accumulation error;

subtraction error in cell_scatter_trace-new_trace.

A true generalized trace loss of exactly one can therefore be numerically recorded below one by more than the required acceptance gap.

Concrete singular-deletion construction

Use a K
2,2
	​

 FE graph and two controls. Let

a
1
	​

=(16266.677758123767,−44348.82872964797),
a
2
	​

=(−1183.041058527761,3228.3081384403536).

Construct 12 unit-frequency rows:

deletion block g
1
	​

, cell (w
1
	​

,f
1
	​

): controls ±a
1
	​

/
2
	​

;

blocks g
2
	​

,g
3
	​

,g
4
	​

, all in cell (w
1
	​

,f
2
	​

): each has controls ±a
2
	​

/
6
	​

;

blocks g
5
	​

,g
6
	​

, cells (w
2
	​

,f
1
	​

) and (w
2
	​

,f
2
	​

): two zero-control rows each.

The controls have zero mean in every cell. Hence their residualized Schur matrix is

G=a
1
	​

a
1
′
	​

+a
2
	​

a
2
′
	​

.

Deleting g
1
	​

 leaves

G
−g
1
	​

	​

=a
2
	​

a
2
′
	​

,

which has rank one. The full five-column FE-plus-control design has rank five; the deleted design has rank four. Moreover,

tr(G
−1
a
1
	​

a
1
′
	​

)=1

exactly, so the correct trace certificate must withhold.

An IEEE-754 reproduction of the source operations gives:

Registered quantity	Computed value	Production threshold
scaled G reciprocal condition	1.06627×10
−9
	must exceed 10
−10

inverse residual	7.54×10
−10
	at most 10
−8

whitening residual	9.90481×10
−8
	at most 10
−7

computed certificate gap	9.77275×10
−8
	must exceed 10
−8

Thus the source inequalities accept an exactly singular deletion.

After an ordinary nonsingular whitening transformation of the same two control columns, the reproduced certificate gap is 3.33×10
−16
, so the same control space is withheld. The exact generalized trace is invariant, but production acceptance is not.

The current failure contract says that a positive reported deletion_rank_gap certifies retained control rank. That statement is too strong under these numerical gates. 

Pasted text

JLA spectral gate does not repair the false certificate

In this construction, the critical control direction inside g
1
	​

 is the within-block contrast, while the rank-one FE direction is the common vector. They are orthogonal. The exact FE block leverage is 0.7. The reproduced control-block eigenvalue is

0.9999999883387.

Therefore, for ordinary leverage sketches near 0.7, the full estimated block’s largest eigenvalue remains 0.9999999883387, giving maker gap 1.1661×10
−8
>10
−10
. Its reproduced scaled reciprocal condition and inverse residual also pass. The JLA block checks therefore do not recover the structural singularity. Those checks are exactly the ones applied after the deterministic certificate. 

Pasted text

The dense exact gate has the same numerical hole

The dense backend computes the full inverse, forms the block projection, and accepts when the computed 1−λ
max
	​

 exceeds block_tolerance; it does not directly factor the deleted information matrix. 

Pasted text +1

For the same construction, reproducing the dense source operations gives:

λ
max
	​

(P
g
1
	​

g
1
	​

	​

)=0.99999998833885,

so the computed maker gap is again 1.1661×10
−8
, despite the exact deleted design being singular. The full inverse and maker-inverse residual gates pass.

I could not execute Stata’s specific invsym() and eigensolver in this environment, so this part is properly classified as an unresolved critical numerical false-acceptance counterexample, rather than a confirmed Stata run. The source supplies neither a backward-error proof excluding it nor a Mata regression for an ill-conditioned basis.

F5 — Physical-copy probe generation, scaling, and target centering are otherwise correct

The production generator draws

S
r
	​

=2Binomial(f
r
	​

,1/2)−f
r
	​

,

independently by stored row, which is the literal sum of f
r
	​

 independent Rademacher signs. 

Pasted text

The FE projection RHS uses S
r
	​

, not a stored-row Rademacher sign and not 
f
	​

S
r
	​

. This gives

X
physical
′
	​

q=
r
∑
	​

x
r
	​

S
r
	​


and produces the shared per-copy projected value. The observation branch therefore uses per-copy rather than weighted-row leverage. 

Pasted text

For match deletion, production computes

π
g
	​

=F
g
−1/2
	​

r∈g
∑
	​

f
r
	​

p
r
	​


and

μ
g
	​

=F
g
−1/2
	​

r∈g
∑
	​

S
r
	​

−π
g
	​

,

which are the correct literal-copy contractions. 

Pasted text

The target direction

d
r
	​

=
f
r
	​

T
t
r
	​

	​

	​

S
r
	​

−
T
t
r
	​

	​

s
∑
	​

f
s
	​

T
t
s
	​

	​

	​

S
s
	​


is also implemented correctly and has covariance

diag(t/T)−(t/T)(t/T)
′
.

Pasted text +1

The target directions are generated after the leverage pass, without reseeding, so they consume a disjoint part of the logical random stream. The target MCSE is correspondingly conditional on the realized first-pass sketch. 

Pasted text +1

There is no general physical-copy-law defect in match mode or target centering. F1 is the narrower mismatch between the correctly generated copy law and the moment used for the pooled observation ratio.

F6 — The mixed-moment coefficient is correctly one

Production computes

B=
R
1
	​

[ME(U
2
)−PE(V
2
)+(M−P)E(UV)],

with coefficient one on the mixed raw fourth moment. 

Pasted text

That agrees with the Hessian calculation in the derivation and the symbolic test. 

Pasted text +1

 The static package test also explicitly excludes the earlier coefficient-two expression. 

Pasted text

This repair is correct. It does not cure F1 because the mixed coefficient and the identity of the residual raw moment are separate questions.

F7 — Rank-one FE plus exact-control block derivatives, signs, and order are correct

For

M(h)=I−C−hvv
′
,g(h)=M(h)
−1
e,

we have

dh
dM
	​

=−vv
′
,

and therefore

dh
dM
−1
	​

=−M
−1
dh
dM
	​

M
−1
=M
−1
vv
′
M
−1
.

Hence

g
′
(h)=M
−1
v(v
′
M
−1
e)

and

2
1
	​

g
′′
(h)=M
−1
v(v
′
M
−1
v)(v
′
M
−1
e).

If B is the bias of 
m
, then 
h
=1−
m
 has bias −B. Its variance is V. The bias of the naïve block inverse is therefore

−Bg
′
(h)+V
2
1
	​

g
′′
(h),

so the adjustment must be

+Bg
′
(
h
)−V
2
1
	​

g
′′
(
h
).

That is exactly the production order and sign:

M^{-1}e
+ B M^{-1}v (v'M^{-1}e)
- V M^{-1}v (v'M^{-1}v)(v'M^{-1}e).

Pasted text +1

Controls are not replaced by within-match means. Production forms

C
g
	​

=(
W
	​

U
g
	​

)S
−1
(
W
	​

U
g
	​

)
′

from every stored control row in the block, symmetrizes the full projection, and checks its spectrum and inverse. 

Pasted text

 No commutation between C
g
	​

 and vv
′
 is used.

The observation reciprocal signs

m
1
	​

+
m
2
B
	​

−
m
3
V
	​


are likewise correct once B,V are computed from the correct pooled random coordinate. 

Pasted text

Adversarial tiny-design results
Design	Exact backend	JLA backend	Assessment
FE-valid cycle: K
2,2
	​

, one row per edge, no controls, match deletion	Every edge deletion leaves a connected tree; accepted	Correctly eligible; may be conservatively withheld by an unlucky finite sketch	Correct
FE-invalid bridge: two small cycles connected by a single bridge incident to a degree->1 worker	Articulation pruning removes the unsafe worker/component before backend	Same	Correct withholding
Block-only control: K
2,2
	​

, two rows per cell, z=1 only in cell (1,1), zero elsewhere; delete that cell	Full design rank four, deleted rank three; block eigenvalue one, withheld	Within-cell scatter G=0; UNVERIFIED_DELETION_RANK	Correct withholding; packet test covers an equivalent case 

Pasted text


Nonconstant control inside match: two rows per K
2,2
	​

 cell with z=(2,1),(0,−1),(−1,−2),(−2,−2)	Every deleted design has rank four; accepted	G=3/2, maximum trace loss 1/3, certificate gap 2/3; accepted on spectral event	Correct. In the first block C
g
	​

 and vv
′
 do not commute; production’s full multiplication order is valid
High leverage: K
2,2
	​

 match masses (1,10
6
,10
6
,10
6
)	Three block leverages are approximately 0.999999000003; true maker gap 10
−6
, accepted	Finite sketches may conservatively withhold; accepted sketches use the correct pointwise block formula	Correct but nonuniform near the boundary, as documented 

Pasted text


Frequency >1: K
2,2
	​

, observation frequencies (2,1,1,1)	Correct literal-copy correction	Returns the wrong order-R
−1
 adjustment	False — F1
Ill-conditioned two-control basis: 12-row construction in F4	Source-operation reproduction falsely accepts exact singular deletion	Certificate and spectral-gate reproduction falsely accept	Unresolved critical numerical defect
Repairs
Required before this milestone can pass
1. Repair or disable observation JLA with frequency weights

Replace m_second by the square of the pooled residual-square coordinate, as shown in F1. Until that is implemented and tested, production should return a typed withholding status whenever:

algorithm(jla)
deletion(observation)
max(frequency) > 1

Expanding to physical rows, so every frequency equals one, also avoids this particular defect but is not a production-scale repair.

2. Add a finite-R literal-copy oracle

The oracle must reproduce the complete estimator, not merely its marginal copy moments. For the four-row fixture:

enumerate all 32 physical directions;

compare E(
V
ˉ
2
), not E(
V
2
);

compare the resulting B, V, and reciprocal correction;

test at a moderate finite R, where the R
−1
 term is visible.

The current large-R convergence tests should remain, but they are not substitutes for this check.

3. Give the JLA rank certificate a backward-error budget

A minimally defensible acceptance test must compare the claimed gap with a bound that includes:

the norm of checked_within-I;

error in the inverse/whitening factor;

scatter-sum accumulation error;

loss-subtraction error.

Merely changing the factor 100 to another undocumented constant is not a proof.

Because the control dimension is deliberately low, the safer repair is to form each small deleted scatter matrix G
−g
	​

, or its whitened equivalent I−L
g
	​

, and directly apply a rank-revealing Cholesky/eigendecomposition with a backward-error margin. The trace bound can remain an inexpensive preliminary screen, but it should not be the final rank certificate near the boundary.

4. Stabilize the control basis internally

The rank decision should be made after an internal orthonormalization or rank-revealing factorization of the full control Schur block. The test suite must verify identical accept/withhold status under severe but nonsingular transformations of the control columns.

The F4 construction and its whitened version should be a mandatory paired regression.

5. Strengthen dense exact deletion-rank verification

For exact mode, do not certify deleted rank solely from

I−X
g
	​

H
−1
X
g
′
	​


when the full inverse is ill-conditioned. Since exact mode already has a bounded parameter dimension, use a direct factorization or rigorously checked downdate of

H
−g
	​

=H−X
g
′
	​

X
g
	​


for the rank gate. The existing Woodbury calculation can still be used after rank has been certified.

6. Correct the documentation and returned metadata

Until the numerical certificate has an error proof:

a positive e(deletion_rank_gap) should not be described as certifying rank;

“exact block inverse” should be understood as a dense deterministic computation, not exact arithmetic;

the documentation should distinguish E(
V
ˉ
2
) from E(
V
2
) in the frequency section.

Uncertainty

The observation-frequency defect is algebraic and definitive; it does not depend on Stata, a particular eigensolver, simulation noise, or conditional gate interpretation.

The numerical singular-deletion construction was evaluated by reproducing the displayed source operations in IEEE double precision with LAPACK-backed linear algebra. Stata/Mata was not available for a direct public-entry run, and invsym() may round differently. That prevents me from claiming the supplied decimal fixture is a confirmed Stata false acceptance. It does not resolve the underlying defect: the source’s accepted error bounds are larger than its accepted rank gap, no backward-error argument is supplied, and the same control space produces different rank outcomes under numerically equivalent bases.

No inference claim, KSS independence assumption, application validity, or theorem outside this finite production calculation is certified.
