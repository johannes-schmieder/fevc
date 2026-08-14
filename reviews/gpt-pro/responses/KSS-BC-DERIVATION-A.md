---
review_id: KSS-BC-DERIVATION-A
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: dd14661e342dce1cedd8ea713032b63b6d8fc55fc4912ce0a354bd205edeac6f
verdict: valid_with_repairs
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7ef21d-8994-83ea-9289-f660eb620ca9
---

## Verdict

The complete reviewer response is preserved verbatim under Findings.

## Assumptions

Preserved verbatim under Findings.

## Findings

Verdict

Valid with repairs.

The two local mathematical conclusions are substantially correct:

The finite-projection ratio expansion has a coefficient of one, not two, on the mixed raw fourth moment E[UV]. The displayed O(R
−1
) bias and variance formulas are correct under the packet’s definitions.

For a match block, the decomposition

M
g
	​

(h)=I−C
g
	​

−hv
g
	​

v
g
′
	​


is exact, even when controls vary within the match. The first- and second-inverse derivatives, the sign of the B adjustment, the sign of the V adjustment, and the matrix order in the proposed correction are correct through O(R
−1
).

The packet is not yet a complete certification, for three reasons:

Withholding singular or nonpositive finite-probe realizations changes the relevant expectation. The packet supplies no conditioning or vanishing-rejection argument.

The deterministic frequency compression is correct, but the stochastic transition from literal physical-copy Rademacher probes to stored-row probes is not documented. Those probe laws generally have different fourth moments.

The production JLA callers are not supplied. The packet therefore does not establish that every caller uses the FE-only projection, coefficient one, the right frequency mapping, independent or fixed multiplicative factors, and a positive-definiteness gate.

The packet itself characterizes the JLA calculation as a second-order approximation and acknowledges that numerical tests do not establish exact unbiasedness or econometric validity. 

Pasted text +1

Assumptions needed for the valid result

The displayed formulas require more than bare invertibility:

Exact orthogonal projections. P
A
	​

 must be symmetric and idempotent and R
A
	​

=I−P
A
	​

. For an approximate operator 
P
,

E[(
P
q)
i
2
	​

]=(
P
P
′
)
ii
	​

,

which need not equal the intended leverage. Merely setting 
M
=I−
P
 does not repair non-idempotence.

Full-rank deterministic design. After the stated FE normalization, A
′
A must be nonsingular. Controls must be linearly independent modulo the FE span:

S=Z
′
R
A
	​

Z=U
′
U≻0.

These are exactly the conditions under which the orthogonal projection split is valid. 

Pasted text

Correct block structure. Every row in the declared match block must have the same unweighted FE row, and

v
g
	​

=
∑
r∈g
	​

f
r
	​

	​

(
f
r
	​

	​

:r∈g)
	​


must be unit length. 

Pasted text

Leave-block-out estimability. The true block residual maker must be positive definite:

M
g
	​

(h
g
	​

)=I−C
g
	​

−h
g
	​

v
g
	​

v
g
′
	​

≻0.

For a true principal block of an orthogonal residual maker, invertibility and positive definiteness coincide. For the estimated M
g
	​

(
h
g
	​

), however, nonsingularity alone is insufficient: an estimated block can be invertible but indefinite.

A spectral neighborhood, not only pointwise invertibility. Taylor expansion requires M
g
	​

(t)
−1
 to remain bounded for t between h
g
	​

 and 
h
g
	​

. A sufficient condition is

t between h
g
	​

,
h
g
	​

inf
	​

λ
min
	​

M
g
	​

(t)≥η>0.

Without such a margin, inverse derivatives can be arbitrarily large.

Safe scalar residual leverage. For observation deletion with controls,

m
r
full
	​

=m
r
FE
	​

−c
r
	​


must be strictly positive, and its finite-probe estimate must remain above a registered tolerance. 

Pasted text

Specified probe law. Probes must be independent across r, isotropic, independent of fixed design and outcome, and their actual fourth moments must be the moments used in B and V. The packet expressly permits conditional analysis but does not fully specify the frequency-compressed probe law. 

Pasted text

Fixed other factors. The block derivation holds for a fixed residual vector e and fixed C
g
	​

. If the final caller multiplies the adjusted inverse residual by another O
p
	​

(1) quantity estimated from the same probes, a first-order covariance term may arise.

Pointwise, not uniform, order. The O(R
−1
) result is pointwise in a safe fixed block. A claim uniform over many blocks requires a uniform leverage/spectral margin and a bound on the probability that any block fails.

1. Finite-projection ratio expansion

For one probe, let

Z
r
	​

=(U
r
	​

,V
r
	​

)
′
,EZ
r
	​

=(P,M)
′
,P+M=1,

and define

P
=
R
1
	​

r
∑
	​

U
r
	​

,
M
=
R
1
	​

r
∑
	​

V
r
	​

,
M
ˉ
=f(
P
,
M
)=
P
+
M
M
	​

.

These are the packet’s definitions. 

Pasted text

At P+M=1,

∇f=(−M,P),∇
2
f=(
2M
M−P
	​

M−P
−2P
	​

).

Let

a=E[U
2
],b=E[V
2
],c=E[UV].

The one-probe covariance matrix is

Σ=(
a−P
2
c−PM
	​

c−PM
b−M
2
	​

).

The second-order delta expansion gives

E[
M
ˉ
]−M=
2R
1
	​

tr{∇
2
fΣ}+o(R
−1
).

Expanding the trace,

2
1
	​

tr{∇
2
fΣ}=
=
	​

M(a−P
2
)+(M−P)(c−PM)−P(b−M
2
)
Ma−Pb+(M−P)c.
	​


The terms involving only P and M cancel because P+M=1. Therefore

B
R
	​

=
R
1
	​

{ME[U
2
]−PE[V
2
]+(M−P)E[UV]}
	​


is the correct leading bias.

Why the mixed coefficient is one

In the quadratic form there are two symmetric off-diagonal entries:

H
PM
	​

Σ
MP
	​

+H
MP
	​

Σ
PM
	​

=2(M−P)Cov(U,V).

The outer delta-method factor 1/2 cancels that two. The remaining coefficient is therefore one.

A coefficient of two double-counts the symmetric cross term. The symbolic test in the packet correctly implements this calculation, and the exact rank-one enumeration distinguishes −0.07872 from the coefficient-two value −0.02208. 

Pasted text

The first-order variance is

Var(
M
ˉ
)
	​

=
R
1
	​

∇f
′
Σ∇f+o(R
−1
)
=
R
1
	​

{M
2
a+P
2
b−2PMc}+o(R
−1
),
	​


because the terms involving P
2
M
2
 again cancel. Thus the packet’s displayed variance is also correct. 

Pasted text

Scope of the conclusion

This establishes a leading R
−1
 expansion. It does not show that the displayed expression is the exact finite-R bias. The packet correctly disclaims exact finite-probe unbiasedness. 

Pasted text

2. Plug-in first moments and omitted covariance

Let the implementation estimate the required moments using the same probes:

T
=(
P
,
M
,
E[U
2
]
	​

,
E[V
2
]
	​

,
E[UV]
	​

).

Suppose

B
=
R
1
	​

b(
T
),
V
=
R
1
	​

v(
T
).

On a smooth, safely conditioned neighborhood,

T
−T=O
p
	​

(R
−1/2
).

Consequently,

B
−B=O
p
	​

(R
−3/2
),
V
−V=O
p
	​

(R
−3/2
),

with mean and covariance contributions ordinarily of order R
−2
. In particular,

Cov(
B
,
M
ˉ
)=O(R
−2
),Cov(
V
,
M
ˉ
)=O(R
−2
).

Therefore the same-probe plug-in does not omit an O(R
−1
) covariance term. It does omit higher-order terms, including nonlinear plug-in bias, third and fourth cumulant terms, and covariance terms of order R
−2
. That is consistent with a correction advertised only through O(R
−1
).

The distinction should nevertheless be made explicit:

B,V: population conditional leading bias and variance terms;

B
,
V
: random same-probe estimates of those terms.

The tests use the random plug-in quantities but do not state the corresponding order argument. 

Pasted text

Reciprocal correction

If

E(
M
ˉ
−M)=B+o(R
−1
),Var(
M
ˉ
)=V+o(R
−1
),

then

E[
M
ˉ
1
	​

]=
M
1
	​

−
M
2
B
	​

+
M
3
V
	​

+o(R
−1
).

Hence

M
ˉ
1
	​

+
M
ˉ
2
B
	​

−
M
ˉ
3
V
	​


is the appropriate one-step correction through O(R
−1
). Equivalently, multiplying 1/
M
ˉ
 by

1−
M
ˉ
2
V
	​

+
M
ˉ
B
	​


has the packet’s stated signs. 

Pasted text

The same reasoning validates the observation-with-controls expression

m
full
1
	​

+
(
m
full
)
2
B
	​

−
(
m
full
)
3
V
	​

.

Pasted text

Important caller limitation

The no-first-order-covariance result applies to the plug-in B,V, because they already carry an explicit R
−1
 factor. It does not automatically apply if the caller multiplies the corrected inverse by another stochastic O
p
	​

(1) JLA quantity based on the same probes. If

A
=A+O
p
	​

(R
−1/2
),g(
h
)=g(h)+O
p
	​

(R
−1/2
),

then

Cov{
A
,g(
h
)}=O(R
−1
),

which would have to be included or eliminated using independent probe streams. The supplied block derivation explicitly holds e fixed, but the actual production callers are absent. 

Pasted text

3. General block-control algebra
Exact projection split

Let

P
A
	​

=A(A
′
A)
−1
A
′
,R
A
	​

=I−P
A
	​

,U=R
A
	​

Z,S=U
′
U.

Since A
′
U=0 and

col[A,Z]=col(A)⊕col(U),

if S is nonsingular,

P
[A,Z]
	​

=P
A
	​

+US
−1
U
′
	​

.

This is an exact identity, not an approximation. The packet states it correctly. 

Pasted text

Rank-one FE contribution

For a match block g, the frequency-transformed FE rows are

A
g
	​

=
	​

f
1
	​

	​

x
⋮
f
k
	​

	​

x
	​

	​

=
F
g
	​

	​

v
g
	​

x,F
g
	​

=
r∈g
∑
	​

f
r
	​

.

Therefore

(P
A
	​

)
gg
	​

=A
g
	​

(A
′
A)
−1
A
g
′
	​

=F
g
	​

{x(A
′
A)
−1
x
′
}v
g
	​

v
g
′
	​

.

Defining

h
g
	​

=F
g
	​

x(A
′
A)
−1
x
′
,

one obtains

(P
A
	​

)
gg
	​

=h
g
	​

v
g
	​

v
g
′
	​

	​

.

The normalization in the packet is correct.

Controls contribute the unrestricted matrix

C
g
	​

=U
g
	​

S
−1
U
g
′
	​

,

so

M
g
	​

(h)=I−C
g
	​

−hv
g
	​

v
g
′
	​

	​

.

No within-match constancy of controls and no commutativity between C
g
	​

 and v
g
	​

v
g
′
	​

 are required. 

Pasted text

Inverse derivatives

Write M=M
g
	​

(h). Because

M
′
(h)=−vv
′
,

the inverse derivative identity yields

dh
dM
−1
	​

=−M
−1
M
′
M
−1
=M
−1
vv
′
M
−1
.

For fixed e,

g(h)=M(h)
−1
e

therefore satisfies

g
′
(h)=M
−1
v(v
′
M
−1
e)
	​

.

Differentiating once more,

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
e)
	​

.

This matrix order is correct even when C
g
	​

v is not proportional to v. The packet’s finite-difference test checks the same order and signs. 

Pasted text

Bias correction and signs

Let

δ=
h
−h.

Since 
h
=1−
m
, if

E(
m
−m)=B+o(R
−1
),Var(
m
)=V+o(R
−1
),

then

Eδ=−B+o(R
−1
),Eδ
2
=V+o(R
−1
),

where the omitted B
2
 is O(R
−2
).

Thus

E[g(
h
)]−g(h)=−Bg
′
(h)+
2
V
	​

g
′′
(h)+o(R
−1
).

Subtracting this leading bias gives

g
bc
	​

=g(
h
)+Bg
′
(
h
)−
2
V
	​

g
′′
(
h
),

or explicitly

g
bc
	​

=
	​

M(
h
)
−1
e
+BM(
h
)
−1
v{v
′
M(
h
)
−1
e}
−VM(
h
)
−1
v{v
′
M(
h
)
−1
v}{v
′
M(
h
)
−1
e}.
	​


The packet’s plus sign on B and minus sign on V are correct. 

Pasted text

4. Positivity and estimability

The exact full-sample block residual maker is a principal block of an orthogonal residual projection. It is therefore positive semidefinite. The condition

M
g
	​

≻0

is equivalent to the deletion retaining the identified rank; equivalently, no nonzero coefficient direction is supported exclusively on the deleted block.

For the estimated maker

M
g
	​

=I−C
g
	​

−
h
vv
′
,

the projection geometry no longer guarantees positive semidefiniteness. Thus:

checking only det
M
g
	​


=0 is insufficient;

the implementation must check λ
min
	​

(
M
g
	​

)>τ;

a stable Taylor argument also needs the true block to be separated from the threshold.

For example, in a scalar block with C=.6 and true h=.3, the true maker is 0.1>0. A realization 
h
=.5 gives −0.1: it is invertible but not a valid residual-maker block. The packet’s documentation says nonpositive blocks are withheld, which is the correct gate, but the governing assumption’s reference merely to “invertible” accepted blocks is too weak. 

Pasted text +1

A convenient exact characterization is obtained by setting A
0
	​

=I−C
g
	​

. When A
0
	​

≻0,

M
g
	​

(h)≻0⟺1−hv
′
A
0
−1
	​

v>0.

This also identifies the singular boundary that the finite-probe estimate must not cross.

5. Literal-frequency normalization
Deterministic match-block compression is correct

Let N=∑
r
	​

f
r
	​

 be physical-copy size, and define the isometry K∈R
N×n
 by

K
(r,j),r
	​

=f
r
−1/2
	​

,K
′
K=I.

If X
w
	​

=W
1/2
X is the stored-row weighted design and X
e
	​

 is the literal expanded design, then

X
e
	​

=KX
w
	​

,P
e
	​

=KP
w
	​

K
′
.

For match g,

u
g
	​

=
F
g
	​

	​

1
F
g
	​

	​

	​

=K
g
	​

v
g
	​

.

Consequently,

u
g
′
	​

P
e
	​

u
g
	​

=v
g
′
	​

P
w
	​

v
g
	​

=h
g
	​

.

Thus the packet’s 
f
	​

 normalization is exactly the compressed counterpart of equal physical-copy weights.

The same is true for controls. If duplicates are literal copies,

Z
e
	​

=KZ
w
	​

,U
e
	​

=KU
w
	​

,S
e
	​

=S
w
	​

.

For residual and outcome vectors lying in the duplicate-constant subspace,

M
e,g
−1
	​

K
g
	​

e
w,g
	​

=K
g
	​

M
w,g
−1
	​

e
w,g
	​

.

So the deterministic match-deletion inverse can be evaluated in stored-row weighted coordinates without physical expansion.

This is consistent with the contract’s use of frequency weights in the estimation design and total stored-row target mass. 

Pasted text

The probe law is not automatically preserved

If q
e
	​

 has iid physical-copy Rademacher entries, its compressed coordinate is

ξ=K
′
q
e
	​

,ξ
r
	​

=
f
r
	​

	​

1
	​

j=1
∑
f
r
	​

	​

q
rj
	​

.

Then

Eξ
r
	​

=0,Eξ
r
2
	​

=1,

but

Eξ
r
4
	​

=3−
f
r
	​

2
	​

	​

.

Except when f
r
	​

=1, ξ
r
	​

 is not Rademacher.

Therefore:

physical-copy probes compressed through K
′
, and

fresh iid Rademacher probes drawn directly on stored weighted rows

have the same isotropic second moment but generally different fourth moments. They target the same first-moment leverage, but they have different finite-R B and V.

This does not invalidate the coefficient-one formula: that formula works for either law if E[U
2
], E[V
2
], and E[UV] are computed under the law actually used. It does invalidate any claim that stored-row Rademachers are literally the same finite-probe experiment as physical-copy Rademachers.

The packet juxtaposes frequency-transformed block algebra with an oracle that physically expands integer frequencies, but it gives no probe-law bridge or frequency-specific JLA test. 

Pasted text +2

Observation deletion needs an additional mapping

The contract says observation deletion treats each physical copy as its own deletion unit. 

Pasted text

For physical copy i belonging to stored row r,

(P
e
	​

)
ii
	​

=
f
r
	​

(P
w
	​

)
rr
	​

	​

.

Hence

m
e,i
	​

=1−
f
r
	​

(P
w
	​

)
rr
	​

	​

,

not 1−(P
w
	​

)
rr
	​

.

The match-block compression does not by itself prove that an observation-level JLA caller performs this division or emulates the physical-copy probe. That remains unresolved because the relevant production caller is absent.

6. Finite counterexample search
Counterexample A: exact deletion estimable, finite-probe reciprocal singular

Take the two-observation intercept design

A=(1,1)
′
.

Then

P=
2
1
	​

(
1
1
	​

1
1
	​

),M=
2
1
	​

(
1
−1
	​

−1
1
	​

).

For observation i=1,

P
11
	​

=M
11
	​

=
2
1
	​

.

Deleting observation 1 leaves one observation and the intercept remains identified, so the exact deletion is estimable.

With one Rademacher probe:

If q
1
	​

=q
2
	​

, which occurs with probability 1/2, then U=1,V=0, so 
M
ˉ
=0 and the reciprocal correction is undefined.

If q
1
	​

=−q
2
	​

, then U=0,V=1, so 
M
ˉ
=1.

Thus exact leave-out estimability does not imply that the finite-probe reciprocal is defined. If zero-
M
ˉ
 draws are withheld, the accepted-draw distribution is selected; at R=1, every accepted draw has 
M
ˉ
=1, whereas the target inverse leverage is 1/M=2.

For R probes the singular event still has probability 2
−R
. It may be numerically small for a chosen R, but it is not zero and is not covered by an unconditional expectation formula.

This contradicts any interpretation under which the second-order expansion automatically remains valid after the packet’s data-dependent withholding rule. The packet states the formula and the gate but gives no conditional-bias analysis. 

Pasted text +1

Counterexample B: a valid match block can have zero ratio denominator

Take four observations with an intercept-only projection

P=
4
1
	​

1
4
	​

1
4
′
	​


and let the deletion block contain the first two observations. With

v=(1,1)
′
/
2
	​

,

the restricted FE projection is

P
gg
	​

=
4
1
	​

(
1
1
	​

1
1
	​

)=
2
1
	​

vv
′
,

so h=1/2, and

I−P
gg
	​


has eigenvalues 1 and 1/2. The block is exactly leave-out estimable because two observations remain.

For

q=(1,−1,1,−1)
′
,
Pq=0,v
′
(Mq)
g
	​

=v
′
q
g
	​

=0.

Thus both projected and residual scalar coordinates are zero:

U=V=0.

Among iid Rademacher directions, this happens whenever each of the two pairs has opposite signs, with probability 1/4. At R=1, the ratio denominator is therefore exactly zero with positive probability despite a strictly positive true residual share.

For R probes the all-zero event has probability 4
−R
. Again, a gate is necessary, and accepted-only expectations require separate analysis.

Counterexample C: compressed-row leverage is not physical-copy leverage

Take two stored rows with frequencies

f=(2,1)

and an intercept-only design. The weighted design is

A
w
	​

=(
2
	​

,1)
′
.

Its projection is

P
w
	​

=(
2/3
2
	​

/3
	​

2
	​

/3
1/3
	​

).

The literal expanded design has three observations and

P
e
	​

=
3
1
	​

1
3
	​

1
3
′
	​

.

Thus the first stored weighted coordinate has leverage 2/3, while each of its two physical copies has leverage

2
2/3
	​

=
3
1
	​

.

Using the stored-row residual leverage 1/3 for physical-copy observation deletion would be wrong; the physical-copy residual leverage is 2/3.

The physical-copy compressed probe for that row is

ξ
1
	​

=(q
1
	​

+q
2
	​

)/
2
	​

,

whose fourth moment is 2, rather than the fourth moment 1 of a stored-row Rademacher.

This is an exact finite contradiction to an unqualified physical-copy/stored-Rademacher equivalence. The oracle itself avoids the issue by physically expanding frequencies. 

Pasted text

Counterexample conclusion

I found no finite counterexample to either local formula once all of the following are imposed:

exact orthogonal projections;

the actual probe law used in the raw fourth moments;

unit normalization;

fixed e and C
g
	​

;

a positive spectral neighborhood;

no conditioning on a non-negligible gate event;

interpretation only through O(R
−1
).

The counterexamples instead defeat broader unconditional, finite-exact, or physical-probe-equivalence readings.

Findings
F1 — Major: withholding changes the expectation

kss_bc/docs/BLOCK_CONTROL_DERIVATION.md says singular, nonpositive, nonfinite, or tolerance-failing realizations are withheld. It does not say whether one block is silently omitted or the entire target is withheld, nor does it establish that the failure probability is o(R
−1
). 

Pasted text

If an estimate is interpreted conditional on passing the gate, the unconditional delta bias formula is not the relevant bias formula. The two- and four-observation counterexamples above show that the selection event can have positive finite-R probability even when the exact deletion is estimable.

Repair: specify that any failed requested block aborts the whole target rather than being silently dropped. For a bias statement, either:

include a deterministic fallback on failures and analyze the unconditional estimator; or

prove, under an explicit spectral margin, that the probability of any failure is o(R
−1
).

For G blocks, a per-block result is not enough. A uniform statement needs control of

P(any failure)≤
g=1
∑
G
	​

P(block g fails).
F2 — Major: physical-copy and stored-row probe laws are not equivalent

The 
f
	​

 block direction is correct for deterministic algebra, but iid physical-copy signs compress to standardized sums, not stored-row Rademachers. Their fourth moments differ, so finite-projection B and V differ. The supplied JLA tests do not exercise nontrivial frequencies. 

Pasted text +1

Repair: state explicitly which algorithm is intended:

Physical-copy JLA: generate

ξ
r
	​

=f
r
−1/2
	​

j=1
∑
f
r
	​

	​

q
rj
	​

,

perhaps by drawing a centered binomial count without expanding rows; or

Weighted-row JLA: draw iid Rademachers on stored rows and define the finite correction under that probe law, without claiming literal finite-probe equivalence.

For observation deletion, separately map weighted leverage to per-copy leverage.

F3 — Major: production caller coverage is absent

The supplied scalar helper implements coefficient one, and the tests manually reconstruct the general block adjustment. The dense oracle performs exact inverse actions rather than the production JLA calculation. 

Pasted text +1

Consequently, the packet does not establish that production callers:

use FE-only P
A
	​

,R
A
	​

, rather than the full projection including controls;

avoid double-counting C
g
	​

;

use coefficient one everywhere;

apply the physical-copy frequency map when required;

check positive definiteness rather than only successful linear solution;

keep other same-probe stochastic factors fixed or account for their covariance.

Repair: supply the production source and a complete caller search. Add integration tests that enter through the public command rather than manually reimplementing the formulas in a test.

F4 — Major if iterative projection actions are used: exact projection geometry is required

The proof uses P
A
2
	​

=P
A
	​

=P
A
′
	​

. The supplied algebra and dense tests use exact matrix projections. 

Pasted text +1

If production uses approximate linear solves, the first moments target row norms of the approximate action rather than the intended leverage unless solver error is separately controlled.

Repair: require exact-enough certified projection actions, with an error bound negligible relative to the desired R
−1
 correction, or include deterministic solver error in the theorem and gates.

F5 — Moderate: the plug-in covariance issue is higher order, but the notation obscures this

No O(R
−1
) term is missing merely because first and fourth moments are estimated from the same probes. The omitted same-probe covariance terms are ordinarily O(R
−2
). The derivation should nevertheless distinguish B,V from 
B
,
V
 and state the needed safe-neighborhood condition.

Repair: add a short lemma giving the orders

B
−B=O
p
	​

(R
−3/2
),
V
−V=O
p
	​

(R
−3/2
)

and the resulting o(R
−1
) effect on the corrected inverse.

F6 — Minor: the documented simulation is not the supplied executable test

The document reports a 600,000-replication simulation. The supplied fixed-seed test uses 150,000 experiments and an absolute tolerance of 0.008. 

Pasted text +1

This does not affect the symbolic conclusion, but the stated evidence is not reproducible from the supplied test as written.

Repair: either change the documentation to 150,000 or include the script and registered output for the claimed 600,000-replication run.

Caller coverage
Component	What the packet establishes	What remains unverified
JLA_FINITE_PROJECTION.md	Correct leading coefficient-one formulas	Production use, gates, frequency law
test_jla_formula.py	Symbolic Hessian identity, one rank-one enumeration, one simulation	General designs, frequencies, gate conditioning, callers
BLOCK_CONTROL_DERIVATION.md	Correct FWL split, block form, derivatives and signs	Actual implementation and safe-event semantics
test_block_controls.py	Dense projection identity, derivative signs, one bias-reduction example	Universal order, near-singular blocks, nontrivial frequencies, public caller
oracle.py	Exact physical-copy KSS algebra and coefficient-one scalar helper	Production JLA; the exact oracle does not exercise finite-probe block callers
Monte Carlo tests	Seed-specific falsification evidence	Theorem, uniformity, econometric validity

The block test manually computes the correction rather than invoking a supplied production function, while the exact oracle forms dense inverses and exact block makers. 

Pasted text +1

Exact identities, approximations, diagnostics, and econometric claims

Exact finite identities

P
[A,Z]
	​

=P
A
	​

+US
−1
U
′
;

P
A,gg
	​

=h
g
	​

v
g
	​

v
g
′
	​

;

M
g
	​

=I−C
g
	​

−h
g
	​

v
g
	​

v
g
′
	​

;

g
′
(h) and g
′′
(h);

the Woodbury deleted-residual identity e
g,−g
	​

=M
g
−1
	​

e
g
	​

;

the literal-frequency deterministic compression.

Second-order approximations

B
R
	​

 and V
R
	​

 for the normalized finite-projection ratio;

the reciprocal correction;

the general block inverse correction;

replacement of population B,V by same-probe plug-ins.

Conditional numerical diagnostics

symbolic simplification;

exact enumeration of one-probe directions;

fixed-seed simulations;

finite-difference derivative checks;

dense-oracle agreement.

These are useful falsification checks but do not prove the uniform or gated claims. The packet itself recognizes that limitation. 

Pasted text

Econometric claims not certified here

unbiasedness under application-specific dependence assumptions;

independence of declared deletion blocks;

sampling inference;

theorem transfer from numerical agreement;

empirical validity of match definitions.

The estimator contract explicitly limits the result to point estimates and disclaims inference. 

Pasted text

Required repairs and falsification tests

State the theorem at the right order. Use “conditional, pointwise O(R
−1
) expansion” rather than “finite correction” without qualification.

Separate population and plug-in notation. Write B
R
	​

,V
R
	​

 for the conditional leading terms and 
B
R
	​

,
V
R
	​

 for same-probe estimates.

Define the safe event.

A
R
	​

={
P
+
M
>ϵ,λ
min
	​

M
g
	​

(
h
)>τ,all quantities finite}.

State exactly what output occurs on A
R
c
	​

.

Add a margin condition if any bias claim is retained after gating:

λ
min
	​

M
g
	​

(h
g
	​

)≥τ+η

uniformly over requested blocks, together with a failure-probability argument.

Document the frequency isometry and probe law. Include X
e
	​

=KX
w
	​

, P
e
	​

=KP
w
	​

K
′
, u
g
	​

=K
g
	​

v
g
	​

, and

ξ
r
	​

=f
r
−1/2
	​

j
∑
	​

q
rj
	​

.

Special-case physical-copy observation deletion. Test and document

p
copy
	​

=p
w,rr
	​

/f
r
	​

.

Verify FE-only probing. The JLA residual share used to form 
h
=1−
m
 must be based on P
A
	​

,R
A
	​

, not the full projection with controls, because C
g
	​

 is then added exactly.

Audit all same-probe factors. Either show every other factor multiplying g
bc
	​

 is deterministic conditional on the fixed input, use independent probes, or add the joint covariance correction.

Add exact adversarial tests:

the two-observation reciprocal singularity;

the four-observation match-block zero denominator;

the f=(2,1) physical/stored leverage example;

a near-singular block with nonzero C
g
	​

;

a noncommuting C
g
	​

 and vv
′
;

same-sample versus population B,V across increasing R, checking an R
−2
 residual;

a public-caller integration test with nontrivial frequencies and varying controls.

Add a static coefficient audit. Search all production callers for any legacy mixed coefficient two and reject it in CI.

Uncertainty

The algebraic verdict is strong: the coefficient-one ratio expansion and the general block-control derivative correction are correct through O(R
−1
) under the strengthened conditions above.

The implementation verdict remains incomplete because the packet does not contain the production JLA code, its probe generator, its frequency mappings, or its public callers. I therefore cannot determine from the supplied material whether the frequency issue is only a documentation omission or an actual implementation error, nor whether any same-probe multiplicative covariance is present in the final production correction.

## Counterexample search

Preserved verbatim under Findings.

## Repairs

Preserved verbatim under Findings.

## Uncertainty

Preserved verbatim under Findings.

