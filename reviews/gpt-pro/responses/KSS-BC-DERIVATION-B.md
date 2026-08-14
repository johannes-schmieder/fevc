---
review_id: KSS-BC-DERIVATION-B
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 54dad46585ac84a775a207f2565d339f3731de02c5c3f1c73fc8ca949be9d0df
verdict: valid_with_repairs
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7ef240-68b0-83ea-b93e-27286d8287d9
---

## Verdict

The complete reviewer response is preserved verbatim under Findings.

## Assumptions

Preserved verbatim under Findings.

## Findings

Verdict: valid with repairs

The two central algebraic conclusions are correct:

The mixed raw fourth moment in the R
−1
 bias coefficient of the ratio-normalized residual leverage has coefficient one, not two.

With exact residualized controls, the general match-block residual maker is

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

,

and the displayed +B, −V adjustment has the correct signs and matrix order through order R
−1
, without requiring controls to be constant within the match.

However, the packet does not yet justify an unconditional finite-R, uniform, or implementation-wide claim. It needs explicit treatment of random rejection events, positive-distance-from-singularity conditions, and the mapping between literal physical-copy probes and stored-row probe aggregates. The feasible use of the same probes to estimate first and fourth moments introduces no omitted R
−1
 covariance term, but it does introduce lower-order terms that the documentation should state. The displayed formulas are leading delta corrections, not exact finite-probe identities. These are critical qualification and implementation-coverage repairs, not a sign or coefficient failure. The packet itself describes the correction as second order and acknowledges that it is not exactly unbiased. 

Pasted text +1

Milestone disposition: KB5 should remain open until the conditioning, frequency-probe mapping, and production-caller checks below are added.

1. Ratio expansion and the mixed coefficient

Let, conditional on the fixed design and outcome,

δ
p
	​

=
P
−P,δ
m
	​

=
M
−M,P+M=1,

and define

μ
20
	​

=E[U
2
],μ
02
	​

=E[V
2
],μ
11
	​

=E[UV].

The ratio is

M
ˉ
=
1+δ
p
	​

+δ
m
	​

M+δ
m
	​

	​

.

Expanding directly to quadratic order gives

M
ˉ
−M=
	​

−Mδ
p
	​

+Pδ
m
	​

+Mδ
p
2
	​

+(M−P)δ
p
	​

δ
m
	​

−Pδ
m
2
	​

+ρ
3
	​

.
	​


For R independent probes,

E[δ
p
2
	​

]
E[δ
m
2
	​

]
E[δ
p
	​

δ
m
	​

]
	​

=
R
μ
20
	​

−P
2
	​

,
=
R
μ
02
	​

−M
2
	​

,
=
R
μ
11
	​

−PM
	​

.
	​


Therefore,

E[
M
ˉ
]−M=
R
1
	​

{Mμ
20
	​

−Pμ
02
	​

+(M−P)μ
11
	​

}+o(R
−1
).

All terms involving only P and M cancel:

−MP
2
−(M−P)PM+PM
2
=0.

Equivalently, in the Hessian calculation, the two off-diagonal terms appear twice inside tr(HΣ), but the delta formula multiplies the trace by 1/2. The resulting coefficient on the covariance—and hence on the raw mixed moment after the mean terms cancel—is one. The formula in kss_bc/docs/JLA_FINITE_PROJECTION.md and the symbolic test are correct. 

Pasted text +1

The leading variance follows from the linear term

L=−Mδ
p
	​

+Pδ
m
	​

:
Var(
M
ˉ
)=
R
1
	​

{M
2
μ
20
	​

+P
2
μ
02
	​

−2PMμ
11
	​

}+o(R
−1
).

The factor two in the variance mixed term is correct; it comes from the ordinary cross-product in Var(−Mδ
p
	​

+Pδ
m
	​

). Thus there is no inconsistency between coefficient one in B and coefficient two in V. 

Pasted text

Smallest coefficient-discriminating design

A three-observation intercept projection already distinguishes the two proposed coefficients. Let

P
A
	​

=
3
1
	​

11
′
,

and inspect the first coordinate under all eight Rademacher vectors. Then

P=
3
1
	​

,M=
3
2
	​

,

and exact enumeration gives

μ
20
	​

=
27
7
	​

,μ
02
	​

=
9
8
	​

,μ
11
	​

=
27
2
	​

.

The correct coefficient-one result is

RBias(
M
ˉ
)=−
81
8
	​

.

Putting coefficient two on μ
11
	​

 instead gives

−
27
2
	​

=−
81
6
	​

,

which is different. Thus coefficient two is already falsified in dimension three. The packet’s five-dimensional rank-one example and enumeration test reach the same conclusion. 

Pasted text +1

2. Inverse-residual correction

Write

B
R
	​

=E[
M
ˉ
]−M=O(R
−1
),V
R
	​

=Var(
M
ˉ
)=O(R
−1
).

A Taylor expansion of x↦x
−1
 gives

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
R
	​

	​

+
M
3
V
R
	​

	​

+o(R
−1
).

Consequently, the feasible leading correction is

M
ˉ
1
	​

+
M
ˉ
2
B
R
	​

	​

−
M
ˉ
3
V
R
	​

	​

,

or, equivalently, the multiplier

1−
M
ˉ
2
V
R
	​

	​

+
M
ˉ
B
R
	​

	​


applied to an expression already containing 1/
M
ˉ
. The packet’s signs are therefore correct. 

Pasted text

For an observation with exact control leverage c
r
	​

,

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

.

Because c
r
	​

 is fixed conditional on the design, subtracting it changes neither the leading bias nor the leading variance of the FE residual-share estimator. Hence

m
r
full
	​

1
	​

+
(
m
r
full
	​

)
2
B
R
	​

	​

−
(
m
r
full
	​

)
3
V
R
	​

	​


also has the correct signs. No additional covariance with c
r
	​

 is required when the control projection is exact. 

Pasted text

This conclusion applies only when the object multiplied by the finite-projection multiplier contains exactly one inverse-residual-leverage factor. The exact observation-deletion identity has that form. It would not automatically apply to a separately constructed statistic containing another power of the estimated leverage. 

Pasted text

3. Plug-in first and fourth moments

The theoretical B
R
	​

 and V
R
	​

 above use population probe moments. The feasible code instead plugs in constrained first moments and raw sample fourth moments computed from the same R probes. The supplied block simulation does exactly this before applying the block adjustment. 

Pasted text

Let a denote the finite vector containing P,M,μ
20
	​

,μ
02
	​

,μ
11
	​

, and let 
a
−a=O
p
	​

(R
−1/2
). Since the correction has an explicit outer factor R
−1
,

B
R
	​

−B
R
	​

=O
p
	​

(R
−3/2
),
V
R
	​

−V
R
	​

=O
p
	​

(R
−3/2
)

away from singular denominators. Therefore:

Using the same probes for the first and fourth moments does not create an omitted O(R
−1
) covariance term.

Covariances between 
B
R
	​

,
V
R
	​

 and functions of 
h
 are lower order.

Under fixed positive margins and bounded iid probes, their signed expectation contributions are ordinarily O(R
−2
).

The oracle’s implementation is therefore leading-order correct when called with the intended raw moments and coefficient one. 

Pasted text

The terms omitted from the displayed correction include:

B
R
2
	​

2
g
′′
(h)
	​

,
6
g
′′′
(h)
	​

E[(
h
−h)
3
],

the effect of evaluating g
′
 and g
′′
 at 
h
, and same-probe covariances involving the plug-in estimates of B
R
	​

 and V
R
	​

. These are lower than order R
−1
, but they prevent the formula from being an exact finite-R correction.

Thus the documentation should say explicitly:

The formula cancels the leading R
−1
 probe bias under a fixed-design interior expansion. It is not exactly unbiased at finite R.

The packet already gives the second sentence in substance, but it should also state the order and the role of plug-in moments. 

Pasted text

Cross-block correlations caused by reusing the same global probes do not require an additional term for the expectation of a sum of block corrections: expectation remains additive. Such correlations would matter for a projection-noise variance calculation, which the contract does not claim to provide. 

Pasted text +1

4. General block-control algebra
4.1 Exact projection decomposition

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

When A has full column rank and S is nonsingular,

col[A,Z]=col(A)⊕col(U),

so

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
.

This is exact. The two projection matrices are globally orthogonal because A
′
U=0. Importantly, global orthogonality does not imply that their block restrictions commute or are orthogonal:

(P
A
	​

)
gg
	​

C
g
	​


need not be zero. No such condition is used by the packet’s inverse formula. 

Pasted text

4.2 Rank-one FE restriction

For stored rows in block g, let

w
g
	​

=(
f
r
	​

	​

:r∈g),F
g
	​

=w
g
′
	​

w
g
	​

,v
g
	​

=
F
g
	​

	​

w
g
	​

	​

.

All rows have the same unweighted FE row x
′
, so the weighted block design is

A
g
	​

=w
g
	​

x
′
.

Therefore

(P
A
	​

)
gg
	​

=w
g
	​

[x
′
(A
′
A)
−1
x]w
g
′
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

,

where

h
g
	​

=F
g
	​

x
′
(A
′
A)
−1
x.

Thus the normalization in the displayed formula is correct. In particular,

v
g
′
	​

v
g
	​

=1,

which is what makes the associated FE projection and residual means sum to one. Target weights must not replace frequencies in this calculation: frequency determines the least-squares geometry, while target mass determines Q. The contract makes this distinction correctly. 

Pasted text +1

Controls may vary arbitrarily inside the match. Their contribution is the full matrix

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

.

Hence

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


is exactly the block restriction of the full residual maker. There is no missing term such as C
g
	​

hv
g
	​

v
g
′
	​

: projections are added before restricting, and their interaction is handled when the resulting matrix is inverted. 

Pasted text

4.3 Exact inverse perturbation and matrix order

Set

K
g
	​

=I−C
g
	​

,M
g
	​

(h)=K
g
	​

−hvv
′
.

When M
g
	​

(h)≻0, one also has K
g
	​

≻0. Define

s=v
′
K
g
−1
	​

v,d(h)=1−hs.

Then d(h)>0, and the exact rank-one inverse is

M
g
	​

(h)
−1
=K
g
−1
	​

+
1−hs
h
	​

K
g
−1
	​

vv
′
K
g
−1
	​

.

For fixed transformed residual e, define

g(h)=M
g
	​

(h)
−1
e.

Because

∂h
∂M
g
	​

	​

=−vv
′
,

the inverse derivative is

∂h
∂M
g
−1
	​

	​

=M
g
−1
	​

vv
′
M
g
−1
	​

.

It follows that

g
′
(h)=M
g
−1
	​

v(v
′
M
g
−1
	​

e),

and

2
1
	​

g
′′
(h)=M
g
−1
	​

v(v
′
M
g
−1
	​

v)(v
′
M
g
−1
	​

e).

The order of the matrix-vector products in the packet is correct. The derivation does not use C
g
	​

v=0, blockwise orthogonality, or constant controls. 

Pasted text

4.4 Bias signs

Let

δ
h
	​

=
h
−h.

Since 
h
=1−
m
 and B
R
	​

 is the bias of 
m
,

E[δ
h
	​

]=−B
R
	​

+o(R
−1
),E[δ
h
2
	​

]=V
R
	​

+O(R
−2
).

Therefore

E[g(
h
)−g(h)]=−B
R
	​

g
′
(h)+V
R
	​

2
g
′′
(h)
	​

+o(R
−1
).

Adding

+B
R
	​

g
′
(
h
)−V
R
	​

2
g
′′
(
h
)
	​


cancels the leading bias. This gives exactly the packet’s

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
+B
R
	​

M(
h
)
−1
v(v
′
M(
h
)
−1
e)
−V
R
	​

M(
h
)
−1
v(v
′
M(
h
)
−1
v)(v
′
M(
h
)
−1
e).
	​


Thus the B sign is positive and the V sign is negative. 

Pasted text

5. Literal-frequency normalization and physical probes

The dense oracle’s literal expansion is internally coherent: integer frequencies are expanded into repeated physical rows, while explicit target weights remain total stored-row target masses. 

Pasted text +1

To prove equivalence with a collapsed implementation, define E to be the physical-to-stored expansion matrix whose column for stored row r has value f
r
−1/2
	​

 on its f
r
	​

 physical copies. Then

E
′
E=I,

and

X
physical
	​

=E
X
stored
	​

.

Consequently,

P
physical
	​

=EP
stored
	​

E
′
.

For exact duplicate rows, transformed outcomes and residuals also lie in the range of E. The physical block inverse then satisfies

M
g,physical
−1
	​

E
g
	​

e
g
	​

=E
g
	​

M
g,stored
−1
	​

e
g
	​

.

This proves that the displayed v
g
	​

=
f
	​

/
F
	​

 and stored-block inverse have the correct deterministic normalization.

The random probes require an additional step that the supplied packet does not state. If q is a literal physical-copy Rademacher vector, the corresponding stored-coordinate probe is

z=E
′
q,z
r
	​

=
f
r
	​

	​

1
	​

ℓ=1
∑
f
r
	​

	​

q
rℓ
	​

.

The z
r
	​

 are independent, mean zero, and variance one, but they are not Rademacher when f
r
	​

>1. In particular,

E[z
r
4
	​

]=3−
f
r
	​

2
	​

.

For the physical-copy probe law, all stored-row projected and residual scalar moments must be computed using these normalized sums. Using the unnormalized sum would already give the wrong first moments. Using one fresh stored-row Rademacher sign preserves isotropy and therefore preserves the first moments, but it defines a different probe law and generally changes the fourth moments and the R
−1
 correction.

The displayed JLA tests do not cover this distinction: the coefficient tests and general-block simulation use ordinary unit-frequency Rademacher coordinates, while the dense oracle separately expands frequencies. 

Pasted text +1

Minimal frequency counterexample

Take a physical intercept design with three observations, collapsed into two stored rows with frequencies

(f
1
	​

,f
2
	​

)=(2,1).

For the first stored row,

P=
3
2
	​

,M=
3
1
	​

.

With the literal physical aggregate

z
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

,z
2
	​

=q
3
	​

,

the raw moments are

μ
20
	​

=
27
28
	​

,μ
02
	​

=
9
2
	​

,μ
11
	​

=
27
2
	​

,

and

RB
R
	​

=
81
14
	​

.

Using one Rademacher sign per stored row instead gives

μ
20
	​

=
81
68
	​

,μ
02
	​

=
81
17
	​

,μ
11
	​

=
81
2
	​

,

and

RB
R
	​

=
243
32
	​

.

The first moments agree, but the finite-projection coefficient does not. Therefore the implementation must either:

use z
r
	​

=f
r
−1/2
	​

∑
ℓ
	​

q
rℓ
	​

 to claim literal physical-copy equivalence; or

declare a stored-row isotropic probe law and compute B
R
	​

,V
R
	​

 for that law, without claiming equality to the literal physical-copy finite-R distribution.

This is an unresolved implementation/documentation issue, not an error in the displayed v
g
	​

 normalization.

6. Required rank, positivity, and conditioning conditions

The following conditions are needed for the stated identities and expansions.

Object	Required condition
FE projection	A has full column rank under the chosen firm normalization, so A
′
A≻0.
Joint controls	S=Z
′
R
A
	​

Z≻0, equivalently no control remains collinear after residualizing against the FE span. With no controls, C
g
	​

 is interpreted as zero.
Full regression	[A,Z] has full column rank and its information matrix is positive definite.
Frequencies	f
r
	​

 are positive integers for the literal-copy argument, F
g
	​

=∑
r∈g
	​

f
r
	​

>0, and exact copies repeat the same outcome, controls, IDs, and deletion assignment.
Block structure	Every row in g has the same FE row x
′
; controls need not be constant.
True deletion estimability	M
g
	​

=I−X
g
	​

H
−1
X
g
′
	​

≻0. This is equivalent to the design remaining full column rank after deleting g.
Rank-one inverse	K
g
	​

=I−C
g
	​

≻0 and 1−hv
′
K
g
−1
	​

v>0. These follow from M
g
	​

(h)≻0.
Observation inverse	m
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

>0.
Probe expansion	R≥1; probes are iid across directions and independent of the fixed design and outcome; the relevant raw moments exist. Rademacher probes satisfy the moment condition.
Feasible ratio	
P
+
M
>0, the inverse numerator is positive after any exact-control subtraction, and the estimated block maker is spectrally positive definite.
Uniform R
−1
 statement	There must be a fixed margin such as m
r
full
	​

≥κ>0 and λ
min
	​

(M
g
	​

)≥κ>0. Mere nonsingularity is insufficient for a uniform expansion.
Conditioning	Expectations are over probes conditional on fixed X,y,e,C
g
	​

, or an explicit extension/conditioning rule is given on withheld probe realizations.

The packet states several of these assumptions, and the dense oracle checks full rank and the smallest residual-maker eigenvalue. 

Pasted text +1

For numerical gates, “positive” for a block must mean

λ
min
	​

(M
g
	​

(
h
))>tolerance,

not elementwise positivity or a nonzero determinant. The documentation presently says singular, nonpositive, nonfinite, or tolerance failure, but should specify the spectral criterion. 

Pasted text

7. Exceptional events and finite counterexamples
7.1 An accepted observation can have 
m
=0

Consider a two-observation intercept regression:

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

),M=I−P.

For the first observation, the true residual leverage is M
11
	​

=1/2, and deleting either observation leaves an estimable intercept.

For one Rademacher probe:

equal signs give (U,V)=(1,0);

opposite signs give (U,V)=(0,1).

Thus 
M
ˉ
=0 with probability 1/2, even though the true deleted regression is estimable.

For R=2, the zero-numerator event has probability 1/4. Conditional on not withholding, the same-probe plug-in correction in the packet gives corrected inverse leverage equal to 1 in every accepted case, whereas the truth is 2. This is a finite counterexample to any interpretation that the correction is exactly unbiased—or necessarily bias-reducing—conditional on passing the gate. It does not contradict the repaired R
−1
 asymptotic statement. The packet correctly acknowledges non-exact unbiasedness but does not spell out the selection consequence of withholding. 

Pasted text +1

7.2 A valid aggregate block can have 
P
+
M
=0

Take a four-observation intercept design and a declared two-row block

g={1,2},v=(1,1)
′
/
2
	​

.

Then

(P
A
	​

)
gg
	​

=
4
1
	​

1
2
	​

1
2
′
	​

=
2
1
	​

vv
′
,

so h=1/2, and

M
g
	​

=I−
2
1
	​

vv
′

has eigenvalues 1 and 1/2. The deletion block is fully estimable.

For any Rademacher vector satisfying

q
1
	​

=−q
2
	​

,q
3
	​

=−q
4
	​

,

both directional scalars vanish:

v
′
(P
A
	​

q)
g
	​

=0,v
′
(R
A
	​

q)
g
	​

=0.

There are four such vectors among the sixteen possible directions. Hence, for one probe,

Pr(U+V=0)=
4
1
	​

,

and with R probes the ratio denominator is zero with probability 4
−R
.

Thus exact block estimability does not imply that every finite-probe ratio realization is defined. A pre-ratio nonfinite gate is necessary, and any probe-expectation statement must say how withheld draws are treated. Distinct declared deletion IDs may share an FE coordinate under the point-estimator contract, so this construction is allowed by the stated block algebra. 

Pasted text +1

7.3 Near singularity destroys uniformity

For a scalar block,

M(h)=1−c−h=ϵ>0.

Then

g
′
(h)=
ϵ
2
e
	​

,
2
1
	​

g
′′
(h)=
ϵ
3
e
	​

.

Full rank permits arbitrarily small ϵ. Therefore no remainder bound can be uniform over all accepted designs unless a positive leverage/eigenvalue margin is imposed. A realized tolerance gate is not itself a theorem about uniform bias: it also creates data-dependent selection when the estimated maker crosses the gate. 

Pasted text +1

8. Exact identities, approximations, diagnostics, and econometric claims
Exact finite algebra

The following are exact under the rank conditions:

the FWL projection split P
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

the rank-one FE block restriction h
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

the full block maker I−C
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

the first and second inverse derivatives;

the Woodbury deleted-residual identity;

the dense KSS point-correction accounting identity. 

Pasted text +1

R
−1
-order approximations

The following are not exact finite-R identities:

B
R
	​

 as the complete bias of the ratio;

V
R
	​

 as its complete variance;

the inverse multiplier;

the block +B,−V adjustment;

replacing population moments by same-probe plug-in moments.

They are correct through the leading R
−1
 term under interior fixed-design conditions. 

Pasted text +1

Conditional numerical diagnostics

The symbolic identity, exact Rademacher enumeration, finite differences, dense FWL comparison, and fixed-seed simulations are useful falsification evidence. They do not establish uniformity, handle all rejection events, or prove production caller conformity. The packet itself correctly states that its simulations do not establish econometric validity. 

Pasted text +1

Econometric claims

The deterministic deleted-residual and projection identities require no error-independence assumption. Interpreting the KSS-corrected quadratic as unbiased for an econometric target additionally requires the appropriate conditional mean restriction and block-diagonal error covariance across the declared deletion units. Match deletion permits arbitrary covariance within a declared match but not across different matches of the same worker, as the contract states. The finite-probe correction does not relax that dependence condition and provides no sampling standard error. 

Pasted text

9. Required repairs
Repair 1 — State the exact order and conditioning

Replace any unqualified “finite-projection correction” language with:

Conditional on the fixed design and outcome, and on a positive residual-leverage/eigenvalue margin, the adjustment cancels the leading R
−1
 probe bias. It is not exactly unbiased at finite R.

Define whether expectations are unconditional after assigning a deterministic value on failed draws, conditional on passing the gate, or merely asymptotic on a high-probability admissible event. The present withholding language does not by itself justify a conditional bias statement. 

Pasted text

Repair 2 — Document same-probe plug-in terms

State that B
R
	​

,V
R
	​

 in the derivation are population probe-moment coefficients, while production uses same-probe plug-ins. State that the induced covariance and higher-order terms are lower than R
−1
, rather than silently treating the feasible quantities as fixed. For a claimed R
−2
 correction, use an explicit higher-order expansion, independent probe batches, or appropriate cross-probe/U-statistic constructions. 

Pasted text

Repair 3 — Add the physical/store probe mapping

Add the expansion matrix E and the identity

z
r
	​

=f
r
−1/2
	​

ℓ=1
∑
f
r
	​

	​

q
rℓ
	​

.

Specify whether production uses literal-copy aggregates or one stored-row isotropic sign. Both can estimate first moments, but their fourth moments and finite corrections are different. Add nonuniform-frequency tests; none of the supplied JLA tests establishes this transition. 

Pasted text +1

Repair 4 — Make all gates spectral and auditable

For each block, record at least:

P
+
M
,
m
full
,λ
min
	​

(M
g
	​

(
h
)),

the tolerance, and the reason for withholding. Distinguish true exact estimability from random JLA admissibility. 

Pasted text +1

Repair 5 — Eliminate or quarantine every coefficient-two caller

The Python oracle defaults to coefficient one and expressly identifies coefficient two as a legacy discrepancy. But the documentation says a maintained MATLAB expression still has coefficient two. Any production or validation caller retaining that expression is wrong and must either be changed or explicitly labeled as an expected-failure legacy comparison. The supplied packet does not establish caller-wide coverage. 

Pasted text +1

Repair 6 — Add targeted falsification tests

At minimum, add:

exact enumeration of the three-observation intercept example, requiring −8/81;

the two-observation, R=2 gate example, verifying that no conditional-unbiasedness claim is made;

the four-observation aggregate-block zero-denominator example;

the f=(2,1) physical-versus-stored probe-law comparison;

expanded-versus-collapsed equivalence with nonuniform frequencies and within-match control variation;

near-singular blocks on both sides of the registered spectral tolerance;

a bias-versus-R experiment verifying that residual probe bias scales as R
−2
 only under a fixed interior margin.

10. Uncertainty and implementation coverage

The supplied Python symbolic tests, dense oracle, and simulations support the displayed algebra, but the packet does not include the production Mata/MATLAB callers that construct block probe aggregates, choose FE versus full residual shares, apply control subtraction, or enforce gates. I therefore cannot certify that every production path uses:

coefficient one;

raw rather than inadvertently normalized fourth moments;

f
	​

/
F
	​

 block directions;

the correct physical-copy aggregate probe law;

the full residual denominator after subtracting exact control leverage;

spectral rather than elementwise block gates.

This is an implementation-coverage uncertainty, not uncertainty about the re-derived coefficient or derivative signs. The local proof obligations are algebraically sound after the stated repairs, but they do not certify the entire JLA estimator, the maintained MATLAB routine, or any econometric theorem.

## Counterexample search

Preserved verbatim under Findings.

## Repairs

Preserved verbatim under Findings.

## Uncertainty

Preserved verbatim under Findings.

