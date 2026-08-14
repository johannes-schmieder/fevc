---
review_id: KSS-BC-API9-FINAL-G
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: b3abcac6b302f3271849af9c8bd55f3907901c7f5333a1ea7daec8f4a6fcfe0e
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f0bc0-8b68-83ea-89b6-9dbfd37ac779
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

False. A critical objection remains, and KB5 should not be closed.

The API 9 literal-copy formulas, joint-control block algebra, graph tie handling, four-target accounting, and most fail-closed routes are sound within the packet’s stated finite-design scope. However, there is an accepted production path under JLA + controls + nuisance(fixedoffset) that can treat an exactly unidentified full joint model as identified and return arbitrary variance components.

Source-integrity result

I independently extracted and hashed the exact bodies between each supplied BEGIN/END delimiter.

22 of 22 source bodies match the SHA-256 values listed in SOURCE_MAP.md. 

Pasted markdown

The attachment contains no MANIFEST.sha256 body and no ZIP file. Therefore I cannot authenticate the claimed deterministic ZIP digest
b3abcac6b302f3271849af9c8bd55f3907901c7f5333a1ea7daec8f4a6fcfe0e.

The attached markdown itself has SHA-256
4a08228c93173cb572876baf879c589077e4b7c265f03f1566e027708521d656, but that is not a check of the absent ZIP.

PROMPT.md, REQUEST.yaml, and SOURCE_MAP.md are not themselves covered by the supplied source-map hashes.

Thus, component-body consistency is verified internally, but the external ZIP→MANIFEST→component authentication chain is not verifiable from this attachment.

Critical finding C1: fixed-offset JLA can accept a singular full nuisance model
Contract being violated

Under nuisance(fixedoffset), the command is required to estimate an identified full joint model first, obtain 
γ
	​

, form y
∗
=y−Z
γ
	​

, and then hold that fitted nuisance index fixed. 

Pasted markdown

 The control derivation expressly requires the residualized-control Schur complement to be positive definite; a failed nuisance block is not supposed to be repaired with a generalized inverse, ridge, or numerical perturbation. 

Pasted markdown

The exact backend enforces this by factoring the complete full-model information matrix before constructing the fixed offset. 

Pasted markdown

 The JLA backend does not provide an equivalent rank certificate in fixed-offset mode.

Why the JLA preparation can manufacture rank

The FE solver stops when

∥r
k
	​

∥≤tolerance(1+∥b
reduced
	​

∥),

and its full-system gate likewise divides by 1+∥b∥, with an acceptance threshold of max(10
−11
,10tolerance). These are mixed absolute/relative gates, not scale-invariant relative gates. 

Pasted markdown

Control preparation then:

uses this approximate FE solve to form 
U
=Z−A
K
;

computes

S
=Z
′
WZ−(A
′
WZ)
′
K
;

passes 
S
 to a scale-equilibrated inverse; and

compares it with 
U
′
W
U
 using the same loose residual threshold. 

Pasted markdown

For a scalar positive 
S
, however small, the inverse routine rescales it to the 1×1 matrix [1], reports reciprocal condition number one, and accepts it. 

Pasted markdown

If Z is exactly in the FE span, the true Schur complement is zero. But an unfinished FE solve leaves an error E=
K
−K, and

U
′
W
U
=E
′
H
A
	​

E>0.

Under PCG Galerkin orthogonality, the separately computed 
S
 agrees with this same positive solve-error quantity. Consequently, the recomputation check does not distinguish a true control residual from numerical FE-solve error.

The deterministic control-rank certificate would reject such a control, but it is called only when nuisance == "joint". It is skipped in fixed-offset mode. 

Pasted markdown

Six-row finite counterexample

Take the six-cycle with singleton matches and unit frequencies:

(1,1),(1,2),(2,2),(2,3),(3,3),(3,1),

where each pair is (worker,firm). This sample:

has one unique connected component;

consists entirely of movers;

has no worker articulation vertex; and

remains connected after deleting any singleton match.

With firm 3 grounded, the firm Schur system is

S
F
	​

=(
1
−1/2
	​

−1/2
1
	​

),

and its registered diagonal preconditioner is the identity.

Counterexample at the default tolerance

Let

z=y=λ(1,2,2,0,0,1)
′
,λ=10
−10
.

This control is exactly a firm-effect combination:

z=λ1{j=1}+2λ1{j=2},

so [A,z] is mathematically rank deficient.

The reduced control right-hand side is

b
red
	​

=λ(0,3/2)
′
.

After the first PCG step, the residual is

r
1
	​

=λ(3/4,0)
′
,∥r
1
	​

∥=7.5×10
−11
.

It therefore satisfies the default stopping rule. The approximate residualized-control Schur complement is

S
=0.75λ
2
=7.5×10
−21
>0.

The scalar equilibration reports rcond = 1, and the Schur recomputation agrees because both calculations measure the same projection error.

Now set y=z. In the subsequent joint solve, the control numerator is the same manufactured 
S
, so the code obtains 
γ
	​

=1. It then forms y
∗
=y−z
γ
	​

=0, refits the pure FE model, and obtains zero plug-in and correction components. The fixed-offset branch and its second FE solve are at kss_bc.mata lines 3404–3428. 

Pasted markdown

The true full model has no unique γ. For example:

γ=1 gives zero FE effects;

γ=0 gives firm effects (λ,2λ,0), whose default-target firm variance is 2λ
2
/3.

Therefore even this default-tolerance case returns a point decomposition that is not identified by the supplied model.

Ordinary-scale version

This is not merely a negligible-outcome scaling curiosity. The public syntax permits any tolerance() in [10
−15
,1). 

Pasted markdown

Set tolerance(1e-4), δ=10
−5
, and define the exactly collinear firm-level control

q
1
	​

=2+
3
2δ
	​

,q
2
	​

=2−
3
2δ
	​

,q
3
	​

=0,y=z=q
j(r)
	​

.

Then

S
F
	​

(q
1
	​

,q
2
	​

)
′
=(1+δ,1−δ)
′
.

One PCG iteration leaves residual norm approximately

2
2
	​

δ=2.83×10
−5
,

below the registered stopping threshold 2.41×10
−4
. The manufactured scalar Schur complement is approximately

5.33×10
−10
,

well above rounding noise and again accepted with reported reciprocal condition number one. The full joint solve again returns 
γ
	​

=1, followed by essentially zero FE targets.

Yet the equally valid decomposition γ=0 has firm variance

Var(q
j
	​

)=
9
8
	​

+
27
8δ
2
	​

≃0.8888888889.

Thus an accepted production path can move the reported firm and total variance by order one solely through an unidentified nuisance normalization.

For ordinary leverage draws, the pure-FE six-cycle has h
i
	​

=5/6 and m
i
	​

=1/6, strictly inside all block gates. Hence there is an open, positive-probability set of finite probe realizations on which the command reaches CONVERGED; its acceptance probability approaches one as probes() grows. Once the target pass is completed, the backend posts CONVERGED, its fabricated control_schur_rcond, and the point estimates. 

Pasted markdown

 The ado layer then treats any backend CONVERGED result as successful and posts KSS_POINT_ESTIMATES_ONLY. 

Pasted markdown

Why the supplied tests do not catch C1

The singular-control JLA test uses a constant control. Its reduced FE right-hand side is exactly zero, so FE residualization is exact and the Schur complement is correctly rejected. 

Pasted markdown

The deletion-rank counterexamples use the default nuisance(joint), where the deterministic within-cell rank certificate catches the problem. 

Pasted markdown

 The fixed-offset fixture uses nonsingular controls and checks numerical agreement, but it does not include a nonconstant FE-span control whose base solve terminates early. 

Pasted markdown

Audit of the remaining production formulas
Exact backend

I found no accepted exact-backend path outside the stated finite point-estimator contract.

The exact implementation constructs the weighted identified design, factors the complete information matrix, obtains the full or fixed-offset coefficients, and forms the three primitive target quadratics plus total accounting. 

Pasted markdown

For physical-observation deletion it uses the per-copy leverage h
r
	​

=x
r
′
	​

H
−1
x
r
	​

 and multiplies the copy correction by f
r
	​

:

b
Q
	​

=
r
∑
	​

f
r
	​

y
r
	​

e
r
	​

1−h
r
	​

x
r
′
	​

H
−1
QH
−1
x
r
	​

	​

.

It subtracts only one x
r
	​

x
r
′
	​

 in the direct deleted-information check, correctly corresponding to deletion of one physical copy. 

Pasted markdown

For match deletion it uses

P
g
	​

=X
g
	​

H
−1
X
g
′
	​

,e
g,−g
	​

=(I−P
g
	​

)
−1
e
g
	​

,

and contracts the general block without replacing within-match controls by a mean row. 

Pasted markdown

 This agrees with the governing block estimator. 

Pasted markdown

The equilibrated full inverse, forward-error proxy, and direct deleted-information factorization are conservative numerical gates. I did not find a second finite false-acceptance construction for this backend in the supplied source.

API 9 coordinatewise literal-copy JLA

The API 9 sufficient statistics are algebraically correct.

For stored row r, copy a, and probe t, let q
rat
	​

∈{−1,1}, S
rt
	​

=∑
a
	​

q
rat
	​

, and let p
rt
	​

 be the FE projection shared by every copy. Define

A
2r
	​

=
t
∑
	​

p
rt
2
	​

,A
4r
	​

=
t
∑
	​

p
rt
4
	​

,C
1,ra
	​

=
t
∑
	​

q
rat
	​

p
rt
	​

,C
3,ra
	​

=
t
∑
	​

q
rat
	​

p
rt
3
	​

.

Then for U
rat
	​

=p
rt
2
	​

 and V
rat
	​

=(q
rat
	​

−p
rt
	​

)
2
,

t
∑
	​

U
rat
	​

t
∑
	​

V
rat
	​

t
∑
	​

U
rat
2
	​

t
∑
	​

V
rat
2
	​

t
∑
	​

U
rat
	​

V
rat
	​

	​

=A
2r
	​

,
=R+A
2r
	​

−2C
1,ra
	​

,
=A
4r
	​

,
=R+6A
2r
	​

+A
4r
	​

−4C
1,ra
	​

−4C
3,ra
	​

,
=A
2r
	​

+A
4r
	​

−2C
3,ra
	​

.
	​


These are literal identities, not approximations. The packet derives them at JLA_FINITE_PROJECTION.md lines 731–763, and production stores exactly the required physical signs and C
1
	​

,C
3
	​

 correlations. 

Pasted markdown +1

The constrained shares are

p
ˉ
	​

=
P
+
M
P
	​

,
m
ˉ
=
P
+
M
M
	​

,

and the coefficient-one delta terms are

B=
R
m
ˉ
E(U
2
)
	​

−
p
ˉ
	​

E(V
2
)
	​

+(
m
ˉ
−
p
ˉ
	​

)
E(UV)
	​

	​

,
V=
R
m
ˉ
2
E(U
2
)
	​

+
p
ˉ
	​

2
E(V
2
)
	​

−2
p
ˉ
	​

m
ˉ
E(UV)
	​

	​

.

The coefficient on the mixed moment is correctly one, and the code implements these formulas directly. 

Pasted markdown +1

For observation deletion with exact control leverage c
ra
	​

, production forms

m
ra
full
	​

=
m
ˉ
ra
	​

−c
ra
	​

,w
ra
	​

=
m
ra
full
	​

1
	​

+
(
m
ra
full
	​

)
2
B
ra
	​

	​

−
(
m
ra
full
	​

)
3
V
ra
	​

	​

,

and only then averages w
ra
	​

 over copies within the stored row. It does not pool copy residual squares before the nonlinear operation. 

Pasted markdown

Therefore, conditional on the same physical-copy sign assignment, the collapsed calculation and the literal expanded calculation are algebraically identical at finite R. Under the packet’s unconditional random-probe law, they implement the same random estimator in distribution.

Match contraction and general controls

For a match g with F
g
	​

=∑
r∈g
	​

f
r
	​

,

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

,π
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

The production aggregation reproduces these literal-copy contractions. 

Pasted markdown +1

With U=R
A
	​

Z and S=U
′
WU, the full match projection is

P
g
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

+U
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

(h)=I−U
g
	​

S
−1
U
g
′
	​

−hv
g
	​

v
g
′
	​

.

For g(h)=M
g
	​

(h)
−1
e
g
	​

,

g
′
(h)=M
−1
v(v
′
M
−1
e),
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

Because the residual-share bias is B, the projection-share bias is −B, giving

g
bc
	​

=M(
h
)
−1
e+BM
−1
v(v
′
M
−1
e)−VM
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

The signs and contractions in production match this derivation. 

Pasted markdown +1

For nuisance(joint), the within-cell scatter certificate, whitening-error subtraction, trace bound, direct deleted-scatter eigendecomposition, and inverse-residual check form a conservative sufficient rank gate. I found no counterexample to that certificate. 

Pasted markdown

 The critical problem is specifically that this independent certificate is not applied to the full nuisance fit used by nuisance(fixedoffset).

Target scaling and four-target accounting

If stored row r has total target mass t
r
	​

, each copy has mass t
r
	​

/f
r
	​

. The compressed target direction

a
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

,d
r
	​

=a
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

a
s
	​


satisfies

E(dd
′
)=diag(t/T)−(t/T)(t/T)
′
.

Thus explicit target mass and default equal-physical-copy mass are handled correctly. 

Pasted markdown

Production uses a common d for worker and firm target solves, defines the total projection as their sum, and computes covariance probe by probe as

2
total−worker−firm
	​

.

Consequently both plug-in and corrected values satisfy

T=W+F+2C

at machine precision. 

Pasted markdown

Graph, runtime, and caller audit
Graph selection

The component selector ranks components only by firm count and physical mass. A tie sets the ambiguity flag rather than selecting by a root or encoded identifier. 

Pasted markdown

 The graph caller checks that flag at the initial, mover, iterative-pruning, and final stages and withholds each time. 

Pasted markdown

The public relabeling tests cover the tied-component case. 

Pasted markdown

 I found no accepted ID-dependent component winner.

Semantic runtime identity

The ado caller checks API level, version, and the exact semantic build token both before and after loading Mata. A same-level different-build runtime is withheld. 

Pasted markdown

Failure propagation

Non-CONVERGED backend statuses are posted as WITHHELD; the caller does not retry with another component, algorithm, deletion unit, probe count, or tolerance. 

Pasted markdown

 I found no hidden partial-target or stale-runtime route to a posted result.

Noncritical repairs and qualifications
R1: the universal “draw-for-draw” expansion claim is too strong

The coordinatewise finite-R estimator is correct in law, but the public fixed-seed collapsed and expanded executions are not universally guaranteed to use the same physical-copy sign assignment.

Canonical sorting includes the stored-row frequency and total stored-row target. 

Pasted markdown

 Literal expansion changes those keys to frequency one and per-copy target t
r
	​

/f
r
	​

. For example, two otherwise identical stored rows with

(f,t)=(1,10),(2,2)

are ordered by frequency in collapsed data, whereas their expanded copies have target masses 10,1,1 and are ordered differently by target. Sequential random signs can therefore be assigned to different target masses.

This does not change the unconditional finite-R distribution because the signs are exchangeable, but it falsifies the unconditional wording that every public execution is “draw-for-draw equivalent” after canonical ordering. The documentation makes that stronger claim. 

Pasted markdown

Repair either the wording to “equal in distribution and algebraically equal under a common physical-copy sign assignment,” or use a representation-invariant counter-based physical-copy stream.

R2: fixed-seed JLA results are not generally invariant to arbitrary ID relabeling

The graph decision is ID invariant, but a nonmonotone relabeling changes dense ID codes, the canonical stored-row order, which firm is grounded, and the assignment of sequential probe signs. The resulting fixed-seed JLA realization can change even though its law and target do not.

The supplied successful JLA test checks invariance to input sorting, not to arbitrary successful-sample ID relabeling. 

Pasted markdown

 This is a reproducibility/documentation repair under the prompt’s unconditional random-probe scope, not a second critical estimator defect.

Evidence limitation

Several Python tests import an oracle.py module that is not included among the source-mapped bodies, and the package-layout test references additional absent package and benchmark files. Therefore their claimed execution cannot be independently reproduced from this source-bound attachment alone. This does not alter the code counterexample above, which follows directly from the supplied production flow.

Required repair before KB5 closure

At minimum:

Add a deterministic, scale-invariant full-sample nuisance-rank certificate for every JLA calculation with controls, including nuisance(fixedoffset). The certificate must be applied before accepting full_joint or using 
γ
	​

.

Do not treat 
U
′
W
U
 from an approximate FE solve as proof that Z
′
R
A
	​

Z is positive definite. It contains the positive solve-error term E
′
H
A
	​

E.

Either:

construct a certified lower bound on the exact control Schur complement after subtracting a bound for FE-solve error; or

conservatively withhold when an independent sufficient certificate fails and require algorithm(exact).

Make rank-preparation solves scale-relative for nonzero right-hand sides. Tightening the PCG tolerance alone is useful hardening but is not a complete rank proof on an ill-conditioned mobility system.

Add public regression fixtures for:

the six-cycle nonconstant FE-span control;

both deletion(match) and deletion(observation);

nuisance(fixedoffset);

control rescaling;

default and explicitly looser allowed solver tolerances; and

exact/JLA agreement on withholding status.

Change the semantic build token after the repair so the existing stale-runtime gate cannot pair the repaired ado with this API 9 Mata body.

Supply the actual deterministic ZIP and MANIFEST.sha256 if source authentication against the stated ZIP digest is a KB5 requirement.

KB5 closure decision

Do not close KB5.

A critical finite point-estimator objection remains: the current JLA fixed-offset control path can post KSS_POINT_ESTIMATES_ONLY for an exactly rank-deficient full nuisance model and thereby report an arbitrary worker/firm variance decomposition. The separate absence of the referenced ZIP and MANIFEST.sha256 also prevents completion of the requested external source-authentication check.
