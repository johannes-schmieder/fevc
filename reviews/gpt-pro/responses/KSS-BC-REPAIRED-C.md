---
review_id: KSS-BC-REPAIRED-C
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: adef11c2e8ba0f5abbcc5bda78a3dbe2ff36b4190f60243c615a5edbacfba896
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7efafd-e114-83ea-b8b1-c20c01629140
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

FALSE. KB5 cannot close.

The packet contains correct repairs for several important pieces: the coefficient-one delta formula, the match-level literal-copy contraction, the exact FE/control block decomposition, the +B,-V inverse adjustment, target-probe independence, and the logical limitation of residual-based solver gates. But two critical production defects remain:

For deletion(observation) with frequency f
r
	​

>1, the JLA path combines a pooled first moment with an individual-copy fourth moment. The resulting B and V are not the coefficient-one delta correction for the ratio the code actually computes, and the discrepancy is order R
−1
, not a smaller plug-in error.

The exact within-cell trace theorem is valid, but the floating-point implementation is not a sufficient rank certificate. Its permitted whitening error can exceed the accepted deletion_rank_gap. A finite, leave-one-worker-connected design exists for which an exactly singular deleted joint design is numerically “certified,” the randomized spectral gate passes with positive probability, and the public JLA path returns a point estimate.

Source identity qualification

The SHA-256 of the attachment bytes available in this chat is

78f9b27a51e63af48c80aa0126895c64930a24d76ef249d52b7206b57b835401,

not the stated wrapper digest adef11c2e8ba0f5abbcc5bda78a3dbe2ff36b4190f60243c615a5edbacfba896. I therefore cannot independently authenticate the outer wrapper under the stated digest. However, after extracting the 13 embedded repository-file bodies, every body exactly matches its individual SHA-256 in the packet’s source map. The review below is source-bound to those authenticated embedded bodies. 

Pasted text

No sampling-inference, asymptotic-uniformity, or broader paper claim is assessed. The review uses the finite fixed-design and conditional-probe scope stated in the packet. 

Pasted text

Component verdicts
Component	Verdict
Coefficient-one ratio derivation	Valid
Plug-in order O
p
	​

(R
−3/2
), when moments are consistently defined and residual share is bounded away from zero	Valid pointwise
Literal frequency aggregation for projections and match contractions	Valid
Exact observation- and match-deletion frequency scaling	Valid
JLA observation deletion with f
r
	​

>1	False
FE/control projection split and general block inverse derivatives	Valid
Exact-arithmetic within-cell trace theorem	Valid but conservative
Floating implementation of the trace “certificate”	False as a sufficient certificate
Solver residual and conditional-unbiasedness descriptions	Valid
Public caller result order, signs, and fail-closed routing	Mechanically consistent, but it trusts the two defective JLA calculations
1. Public caller-to-formula trace

The public caller correctly validates frequency weights as positive integers, treats the default target weight as physical-copy mass, constructs the declared observation or match deletion units, canonicalizes stored-row order, dispatches to the requested backend, and aborts the command on any non-CONVERGED backend status. 

Pasted text +4

The posted matrix order is also consistent:

(worker variance,firm variance,worker-firm covariance,total variance),

with rows plugin, bias_correction, corrected, and numerical_mcse; corrected = plugin - correction. This agrees between the return documentation, ado layer, exact backend, and JLA backend. 

Pasted text +3

Exact path

The exact backend:

constructs the weighted joint design and full inverse;

implements nuisance(fixedoffset) separately from joint nuisance estimation;

forms worker, firm, covariance, and total plug-ins in the documented order;

uses per-physical-copy leverage for observation deletion;

uses 
f
	​

-transformed blocks for match deletion;

solves the exact compressed block residual-maker problem; and

posts plugin - correction.

Those formulas are internally consistent. 

Pasted text +1

JLA path

The JLA backend:

prepares the FE and joint-control systems and applies the deletion-rank check;

fits the full or fixed-offset model;

generates physical-copy Rademacher sums for the leverage pass;

estimates FE projection/residual moments;

adds the exact residualized-control projection;

constructs observation or match deleted residuals;

generates fresh target directions in a second pass; and

computes covariance probe by probe from total, worker, and firm contractions.

The second target pass is disjoint from the leverage pass, and covariance accounting is in the correct order. 

Pasted text +2

No caller-level sign, result-column, covariance-centering, or target-probe-independence mismatch remains. The two failures are inside the observation finite-projection moments and the numerical rank gate.

2. Literal physical-copy aggregation

Let stored row r represent f
r
	​

 physical copies, with independent Rademacher variables q
ra
	​

, and define

S
r
	​

=
a=1
∑
f
r
	​

	​

q
ra
	​

.

Then

E[S
r
	​

]=0,E[S
r
2
	​

]=f
r
	​

,

and, because only index patterns in which every sign appears an even number of times survive,

E[S
r
4
	​

]=f
r
	​

+6(
2
f
r
	​

	​

)=3f
r
2
	​

−2f
r
	​

.

Consequently, for ξ
r
	​

=S
r
	​

/
f
r
	​

	​

,

E[ξ
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

The documentation states this law, and production generates S
r
	​

 directly as 2Binomial(f
r
	​

,1/2)−f
r
	​

, independently across stored rows. That is distributionally identical to summing literal physical-copy signs. 

Pasted text +1

With

H=
s
∑
	​

f
s
	​

x
s
	​

x
s
′
	​

,

the projection shared by every copy of stored row r is

p
r
	​

=x
r
′
	​

H
−1
s
∑
	​

x
s
	​

S
s
	​

.

The weighted stored-row diagonal is f
r
	​

 times the physical-copy leverage, so the code correctly treats p
r
2
	​

 as the per-copy projection moment rather than as the weighted-row diagonal. 

Pasted text

For one probe, define V
ra
	​

=(q
ra
	​

−p
r
	​

)
2
. The directionwise physical-copy averages are

V
ˉ
r
	​

=
f
r
	​

1
	​

a
∑
	​

V
ra
	​

=1+p
r
2
	​

−
f
r
	​

2p
r
	​

S
r
	​

	​

,
f
r
	​

1
	​

a
∑
	​

V
ra
2
	​

=1+6p
r
2
	​

+p
r
4
	​

−
f
r
	​

4p
r
	​

(1+p
r
2
	​

)S
r
	​

	​

,

and

f
r
	​

1
	​

a
∑
	​

p
r
2
	​

V
ra
	​

=p
r
2
	​

V
ˉ
r
	​

.

Those identities are correct and are implemented direction by direction. 

Pasted text +1

For a match g, with F
g
	​

=∑
r∈g
	​

f
r
	​

, the FE block is rank one in the direction v
g
	​

=(
f
r
	​

	​

)/
F
g
	​

	​

, and the literal-copy contractions are

π
g
	​

=
F
g
	​

	​

∑
r∈g
	​

f
r
	​

p
r
	​

	​

,μ
g
	​

=
F
g
	​

	​

∑
r∈g
	​

S
r
	​

	​

−π
g
	​

.

Production uses exactly these contractions. Thus match-mode frequency aggregation is correct, including its second and fourth moments. 

Pasted text +1

The target-direction aggregation is also correct. If stored target mass is t
r
	​

, each physical copy has mass t
r
	​

/f
r
	​

; the code’s centered direction has covariance

diag(t/T)−(t/T)(t/T)
′
.

The default t
r
	​

=f
r
	​

 therefore gives equal physical-copy target mass. 

Pasted text +1

3. Coefficient-one ratio expansion

For one probe let

U=(Pq)
i
2
	​

,V=(Mq)
i
2
	​

,

with P=E[U], M=E[V], and P+M=1. For

f(p,m)=
p+m
m
	​

,

evaluated at p+m=1,

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

The second-order bias of f(
P
,
M
) is therefore

B=
R
1
	​

{ME[U
2
]−PE[V
2
]+(M−P)E[UV]},

and its delta variance is

V
Δ
	​

=
R
1
	​

{M
2
E[U
2
]+P
2
E[V
2
]−2PME[UV]}.

The mixed raw fourth moment has coefficient one in the bias expression. The documentation and production code agree on these formulas. 

Pasted text +1

When the true residual share is bounded away from zero, all moments are finite under the fixed Rademacher law, and estimated first and fourth moments have O
p
	​

(R
−1/2
) error. Since B and V
Δ
	​

 already contain 1/R, substituting these moments changes them by O
p
	​

(R
−3/2
). That pointwise order statement is correct provided the supplied second moments belong to the same random variables whose sample means form the ratio. 

Pasted text

Failed finite-probe draws have positive probability. Production withholds output when a denominator, residual share, or block maker fails its gate, and the documentation correctly disclaims exact or second-order unbiasedness conditional on acceptance. The supplied tests explicitly exhibit zero-residual and zero-denominator probe events. 

Pasted text +1

Critical finding C1: observation-frequency moments are internally incompatible

For a stored row with f>1, production forms

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

,
V
ˉ
j
	​

=
f
1
	​

a=1
∑
f
	​

V
aj
	​

.

Therefore the raw second moment required by the displayed delta formula is

E[
V
ˉ
j
2
	​

].

But m_second accumulates

E[
f
1
	​

a
∑
	​

V
aj
2
	​

].

The code uses the former object in m_first and the latter object in m_second, then inserts them into one ratio expansion. 

Pasted text +1

Direction by direction,

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
=4p
2
{1−(
f
S
	​

)
2
}.

This is generally positive for f>1. It is not sampling noise and is not an O
p
	​

(R
−3/2
) plug-in effect. It changes both B and V
Δ
	​

 at their displayed order R
−1
.

Exact finite counterexample

Take the complete K
2,2
	​

 worker-firm graph, no controls, with stored rows

(w,f,f
r
	​

)=(1,1,2), (1,2,1), (2,1,1), (2,2,1).

Every physical observation deletion leaves the FE design identified. For either copy of the repeated first row,

P=
7
3
	​

,M=
7
4
	​

.

Exhaustive enumeration of the 2
5
 physical Rademacher vectors gives

E[U
2
]=
2401
993
	​

,E[
V
ˉ
2
]=
2401
1378
	​

,

whereas production uses

E[
2
V
1
2
	​

+V
2
2
	​

	​

]=
2401
1672
	​

,E[U
V
ˉ
]=
2401
132
	​

.

Hence the correct pooled-ratio terms are

B
pool
	​

=−
16807R
30
	​

,V
pool
	​

=
117649R
25122
	​

,

but production computes

B
code
	​

=−
16807R
912
	​

,V
code
	​

=
117649R
27768
	​

.

The error in the reciprocal adjustment,

M
2
B
	​

−
M
3
V
Δ
	​

	​

,

is exactly

−
32R
9
	​

.

That is an order-R
−1
 formula error.

An even more direct R=2 realization uses the same physical direction twice,

q=(−1,+1,−1,−1,−1),

with the two copies of the repeated row listed first. For that row,

p=−
7
1
	​

,U=
49
1
	​

,
V
ˉ
=
49
50
	​

.

Because both probes are identical, the correctly matched pooled moments give B=V
Δ
	​

=0 and inverse weight

50
51
	​

=1.02.

Production instead uses

2
V
1
2
	​

+V
2
2
	​

	​

=
2401
2696
	​


rather than

V
ˉ
2
=
2401
2500
	​

,

and obtains

B=−
2499
2
	​

,V
Δ
	​

=
127449
2
	​

,inverse weight=
3062500
3121149
	​

≃1.019150694.

This also is not the literal expanded-copy coordinatewise average, which is

2
1
	​

(
36
37
	​

+
64
65
	​

)=
1152
1177
	​

≃1.021701389.

Thus the production statistic is neither:

the coefficient-one correction for its pooled ratio, nor

the finite-R statistic obtained by literally expanding and correcting each physical copy.

The denominator, residual-share, and inverse-positivity gates all pass on this realization, so the mismatch can enter a returned point estimate. The relevant gates only check positivity and finiteness; they cannot detect the inconsistent moments. 

Pasted text

Why the supplied tests do not detect C1

The exhaustive frequency test verifies the directionwise identities for 
V
ˉ
, the average of V
a
2
	​

, and the average mixed product separately. It never checks whether the second quantity is the square of the random variable used in the ratio. 

Pasted text

The public Stata frequency tests compare exact compressed and expanded calculations, but their JLA checks use 4,000 probes and compare each random correction with the exact result using an MCSE tolerance. They do not require finite-R compressed/expanded equality and do not test the pooled ratio’s required second moment. 

Pasted text

The static production test only verifies that the mixed term appears with textual coefficient one. It cannot detect that m_second belongs to the wrong random variable. 

Pasted text

Required repair for C1

The implementation and documentation must choose one estimator.

For the practical pooled stored-row estimator, production should form, for each probe,

v
bar
	​

=1+p
2
−2pS/f,

and accumulate

m_first+=v
bar
	​

,m_second+=v
bar
2
	​

,mixed_second+=p
2
v
bar
	​

.

The documentation must then describe this as a pooled or Rao–Blackwellized physical-copy estimator. It should not claim finite-R identity with correcting every explicitly expanded copy.

Alternatively, preserving literal expanded-copy finite-R semantics requires retaining enough per-copy probe information to compute and average the nonlinear copy-specific inverse corrections. The aggregate S
r
	​

 alone is insufficient for that nonlinear operation.

An exact R=2, f=2, direction-by-direction regression test based on the counterexample above is required.

4. General block-control correction

Let A be an identified FE design, Z the controls, U=R
A
	​

Z, and S=U
′
U. Then

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

This is an exact orthogonal projection decomposition when S is nonsingular. 

Pasted text

Within a match g, every FE row is the same before frequency transformation. With

v
g
	​

=
F
g
	​

	​

(
f
r
	​

	​

:r∈g)
	​

,

the FE projection block is

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

,

whereas the control projection is the unrestricted low-dimensional block

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

Thus

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

.

This correctly allows controls to vary arbitrarily within a match. 

Pasted text

Production implements the same order:

h
vv
′
+(
f
	​

U
g
	​

)S
−1
(
f
	​

U
g
	​

)
′
.

The observation control leverage omits f
r
	​

, correctly, because it is the leverage of one physical copy; the match block includes 
f
	​

, also correctly. 

Pasted text +1

Let 
m
 have registered bias B and variance V. Since 
h
=1−
m
,

E[
h
−h]≃−B,Var(
h
)≃V.

For

g(h)=M(h)
−1
e,
g
′
(h)=M
−1
v(v
′
M
−1
e),

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

The naive inverse has approximate bias

−Bg
′
(h)+V
2
1
	​

g
′′
(h),

so the bias-reduced value must add +Bg
′
 and subtract Vg
′′
/2. The documentation and Mata code use exactly that sign and matrix order. 

Pasted text +1

For observation deletion, with exact control leverage c
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

,

and the reciprocal expansion

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


is also implemented with the correct signs. 

Pasted text +1

The evaluated block is symmetrized, its largest projection eigenvalue must remain below one, and its inverse must pass a residual gate. The derivative expansion is expressly not claimed to be uniform near a singular block. Those algebraic and logical statements are correct. 

Pasted text +1

Finding: no remaining sign, coefficient, frequency scaling, or noncommuting matrix-order defect was found in the general block-control correction itself.

5. Within-cell trace certificate
5.1 Exact-arithmetic theorem: valid

Let C be the saturated worker-firm-cell dummy design and let

G=Z
′
R
C
	​

Z

be weighted within-cell control scatter. After deleting g, let

G
−g
	​

=G−Δ
g
	​

.

Deleting observations and recomputing the affected cell mean yields Δ
g
	​

⪰0, by the weighted within/between scatter decomposition.

If G≻0, define

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
.

Then L
g
	​

⪰0 and

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

Therefore,

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

Now suppose the deleted FE design A
−g
	​

 has full identified rank but the deleted joint design does not. Then for some γ

=0,

Z
−g
	​

γ=A
−g
	​

α.

Every FE fitted value is constant within a worker-firm cell, so

R
C,−g
	​

Z
−g
	​

γ=0

and hence

γ
′
G
−g
	​

γ=0,

contradicting G
−g
	​

≻0. Thus the exact trace condition combined with the FE graph certificate is indeed sufficient for joint deleted-design rank. This is the packet’s intended argument. 

Pasted text

5.2 Smallest conservatism example

The condition is not necessary. A smallest simple bipartite example is K
2,3
	​

, with one observation per cell and one cell-level control

z
1⋅
	​

=(0,1,2),z
2⋅
	​

=(0,0,0).

The control is constant within every cell, so G=0 and the trace certificate cannot start.

Nevertheless, after deleting any one edge, the remaining graph has five observations and five joint parameters. The remaining unique four-cycle uses the two undeleted firm columns. Their worker differences are two distinct elements of (0,1,2), so the control is not additive in worker and firm effects on the retained graph. Every deleted joint design therefore has full rank.

This is conservatism, not false acceptance. K
2,2
	​

 cannot provide the analogous one-control example because after one edge deletion it has only three cell rows for four joint parameters. Thus K
2,3
	​

 is minimal among simple two-way graphs.

5.3 Critical finding C2: the floating implementation is not a sufficient certificate

Production does not stably accumulate centered within-cell scatter. It computes

Z
′
WZ−
c
∑
	​

∑
i∈c
	​

w
i
	​

(∑
i∈c
	​

w
i
	​

z
i
	​

)(∑
i∈c
	​

w
i
	​

z
i
	​

)
′
	​

,

which can subtract very large nearly equal cross-products. It repeats the same cancellation-prone construction after whitening. 

Pasted text +1

More decisively, at the default rank_tolerance(1e-10):

the whitening check permits

∥
W
′
G
W
−I∥≤max(10
−9
,1000rank_tolerance)=10
−7
;

but the deletion certificate accepts any reported gap exceeding

max(10
−10
,100rank_tolerance)=10
−8
.

The accepted gap may therefore be strictly smaller than the implementation’s own admitted normalization error. No rounding-error bound is propagated from the whitening check into the trace loss. 

Pasted text +1

That invalidates the documentation’s description of the posted positive gap as a deterministic sufficient certificate. 

Pasted text +2

Finite false-acceptance counterexample

Use K
2,3
	​

, all frequencies one, deletion(match), one joint control z, and the following match blocks:

Worker-firm cell	Match	Control values in that cell
(1,1)	11	0,0,5120
(1,2)	12	25165824,25165824
(1,3)	13	25165824
(2,1)	21	25165824
(2,2)	22	25165824
(2,3)	23	25165824

Take y=1 on every row.

The full design is identified because match 11 has within-cell control variation. Deleting match 11 leaves z=25165824 on every retained row, so the control becomes exactly collinear with the FE intercept. The deleted joint design is therefore singular, although the retained FE graph remains connected.

The exact within-cell scatter is

G=
3
2
	​

(5120)
2
=
3
52428800
	​

=17476266.
6
.

Evaluating the production cross-product subtraction in IEEE-754 double precision gives a stored value of 17476267. After the production whitening and cell-loss calculations:

tr
(L
11
	​

)=0.9999999809265143,

so

deletion_rank_gap=1.90734857×10
−8
.

The whitening residual is approximately 2.98×10
−8
 under sequential accumulation and can round to zero under BLAS accumulation. Either is below the permitted 10
−7
. The apparent gap exceeds the 10
−8
 acceptance threshold, so kssbc__joint_rank_certificate() returns CONVERGED even though G
−11
	​

=0 exactly.

The canonical public-row sorting does not eliminate this construction; it sorts by worker, firm, deletion ID, controls, outcome, frequency, and target before entering Mata. 

Pasted text

The randomized spectral gate also does not save it. With probes(2), take the same legal FE Rademacher direction for both probes, in the displayed row order:

q=(−1,−1,−1,−1,+1,+1,+1,−1,−1).

This event has positive probability 2
−18
. For match 11, production estimates

h
=0.4566037736.

After adding the exact full-sample control projection, the estimated block projection has largest eigenvalue

0.6087776925<1.

All other blocks also pass. The true full projection block for match 11 has eigenvalue exactly one because deleting that match destroys joint rank. Production nevertheless forms and inverts the estimated residual maker. The relevant JLA gate trusts the estimated eigenvalue once the defective trace check has passed. 

Pasted text

With y=1, the fitted residual is zero in exact arithmetic, so the correction and all target draws are finite and zero. The backend reaches CONVERGED, and the ado layer posts KSS_POINT_ESTIMATES_ONLY, even though the exact KSS deletion is undefined. By contrast, the dense exact backend’s block eigenvalue check would return NONESTIMABLE_DELETION. 

Pasted text +2

This is a finite false point return, not merely an overly conservative withholding.

Why the rank tests do not detect C2

The Python rank test uses a well-scaled fixture and confirms the exact trace implication and invariance under an ordinary control reparameterization. It does not stress cancellation from large cell-constant FE components or compare accepted floating certificates against exact deleted ranks. 

Pasted text

The block-only-control tests correctly reject an obvious G=0 singular deletion. They do not cover a small positive floating gap produced by cancellation. 

Pasted text +1

Required repair for C2

At least the following changes are necessary:

Accumulate scatter from centered rows. Compute cell means and then accumulate

c
∑
	​

i∈c
∑
	​

w
i
	​

(z
i
	​

−
z
ˉ
c
	​

)(z
i
	​

−
z
ˉ
c
	​

)
′

using pairwise or compensated summation. Do not obtain within scatter by subtracting global Z
′
WZ and cell-mean cross-products.

Propagate numerical error into the certificate. If the whitening check produces an error bound ε, and loss accumulation has bound η
g
	​

, acceptance requires something of the form

tr
(L
g
	​

)+ε+η
g
	​

<1.

A reported gap smaller than the whitening residual cannot be called certified.

Prefer direct low-dimensional deleted-scatter checks. Since the number of controls is low, construct each affected G
−g
	​

 from stable cell sufficient statistics and require a Cholesky or minimum-eigenvalue gate on G
−g
	​

 itself. This is less conservative than the trace bound and avoids reliance on an estimated gap smaller than the arithmetic error.

Add the explicit counterexample above to both the Mata/public-command tests and an independent high-precision oracle. The test must require JLA to withhold whenever the exact deleted joint design is singular.

Until this is repaired, e(deletion_rank_gap) and e(deletion_rank_certificate) = "within-cell trace bound" overstate what the floating calculation establishes. The ado layer explicitly applies that label to accepted JLA joint-control calculations. 

Pasted text

6. Solver residual gates and logical status

The FE solver correctly:

forms the worker-eliminated firm Schur system;

handles a zero reduced right-hand side;

reconstructs worker coefficients;

recomputes the residual under the original full FE operator; and

rejects a solution whose full residual exceeds the registered tolerance.

The batched solver applies that check separately to every right-hand side. 

Pasted text

The joint-control solve likewise reconstructs the full coefficient vector and recomputes the residual under the full joint operator. 

Pasted text

The numerical architecture explicitly states that these are residual certificates, not forward-error or uniform-theorem bounds. That logical status is correct. 

Pasted text

The target probes are fresh relative to the leverage probes. The reported MCSE is conditional on the realized leverage sketch and excludes leverage-sketch, solver, and econometric uncertainty. No unconditional or acceptance-conditional unbiasedness claim is smuggled in. 

Pasted text +1

The only logical overstatement in this part of the package is calling the floating trace result a deterministic rank certificate.

Critical repairs required before KB5 can close
C1. Repair observation-frequency finite-projection moments

The random variable whose sample mean forms the residual-share ratio must be the same random variable whose second moment enters B and V. Either:

change m_second to the square of the per-probe pooled physical-copy residual moment; or

implement genuinely copy-specific nonlinear corrections.

The finite-R “literal copy” claim must be rewritten to match the chosen estimator.

C2. Replace or rigorously bound the floating rank certificate

Stable centering and a propagated error bound are mandatory. The current 10^{-7} whitening allowance combined with a 10^{-8} gap threshold can falsely certify an exactly singular deletion.

C3. Add adversarial public-entry tests

Required regression tests include:

the K
2,2
	​

, f=(2,1,1,1), R=2 observation-moment counterexample;

the scaled K
2,3
	​

 false-rank-certificate counterexample;

exact-versus-JLA agreement on the decision to withhold, not merely on high-probe point values;

invariance stress tests after adding very large FE-span components to controls.

C4. Correct the public documentation and status metadata until C2 is fixed

A positive computed deletion_rank_gap currently is not a sufficient numerical certificate. The public return documentation and ado metadata must not call it one unless the implementation has a valid error enclosure.

C5. Reconcile packet provenance

The stated outer SHA-256 does not match the attachment bytes made available for review. The individual embedded files are internally authenticated by their source-map hashes, but the wrapper digest should be regenerated or its canonical byte convention specified.

Optional hardening and exposition repairs

State explicitly that the O
p
	​

(R
−3/2
) plug-in claim concerns the unfiltered algebraic probe law, with a residual share bounded away from zero; production output is a selected subset because failures are withheld.

Document the minimal K
2,3
	​

 example showing that the exact trace test is conservative. This would prevent users from interpreting UNVERIFIED_DELETION_RANK as proof of singularity.

Include match-block inverse residuals in the posted maximum numerical diagnostic. The block inverses are gated, but their residuals are not visibly aggregated into the JLA solver_max_residual field.

Clarify that algorithm(exact) means a deterministic dense floating-point implementation of the exact finite algebra, not exact arithmetic.

Supply the actual public help file, package manifest, and test-execution logs in a future source-bound packet. The packet includes tests that refer to those artifacts, but not the artifacts themselves, so their content and successful execution cannot be independently audited here.

Final finding

The repaired packet resolves the prior mixed-coefficient, block-control sign, match-frequency, target-probe, and solver-status issues. It does not yet implement a coherent coefficient-one finite-projection correction for frequency-compressed observation deletion, and it does not possess a valid floating-point sufficient rank certificate. Each defect admits a finite production counterexample. The required verdict is therefore false.
