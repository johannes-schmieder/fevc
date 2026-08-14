---
review_id: KSS-BC-API7-F
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 0923b52200cb0224002763438da5e825174d06b495ea0d446b6c807acfbbd5a8
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f017c-e0e8-83ea-bda0-f26bd73d1419
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

False.

The coefficient-one finite-projection formulas, the +B−V inverse correction, the noncommuting joint-control block algebra, the target contractions, and the trace certificate are mathematically valid in exact arithmetic. However, the stronger production claim—“every accepted API 7 execution satisfies the documented finite fixed-design contract”—is false.

I found five material accepted-path defects:

Observation-level JLA with frequency f
r
	​

>1 implements a different finite-R algorithm from running the public command on literal copies. The packet’s later pooled definition is an unauthorized narrowing of the authoritative frequency semantics.

The ado can silently bind to a stale or foreign Mata runtime merely because it reports API level 7.

Joint-control batched solves use a global matrix residual, allowing one failed right-hand side to be masked by other columns.

Tied connected components are selected through encoded node labels, so harmless ID relabeling can change the retained sample and estimates.

The finite-precision deletion-rank “gap” is not a certified lower bound because its acceptance margin can be smaller than the permitted whitening error.

The governing request explicitly requires a false-acceptance search rather than treating tests as proof. 

Pasted text

Source identity

The raw mounted attachment has SHA-256

37db5461ac8a0943a989a3a19577b498ae526ff9310fda2940f3d1440995a79a

rather than the stated outer-packet hash 0923b52200cb0224002763438da5e825174d06b495ea0d446b6c807acfbbd5a8.

I separately recomputed the hashes of the embedded file bodies. They agree with the packet’s source map, including the specification, ado, Mata runtime, documentation, oracle, and tests. Accordingly, the substantive verdict below is source-bound to that verified embedded source set, but I cannot certify the raw transport envelope as the stated SHA-256. The packet’s claimed source map is at lines 46–64. 

Pasted text

1. Production dependency chain
Stage	Accepted production path	Adversarial result
Public syntax and sample	The ado validates options, integer positive frequency weights, nonnegative target mass, dependent variable, identifiers, and controls; match deletion rejects any incomplete requested row. 

Pasted text

	Generally fail-closed. Validation on rows later excluded can be conservative, but I found no false acceptance here.
Runtime binding	It tests only whether a currently loaded kssbc__api_level() equals 7; otherwise it finds and executes a file named kss_bc.mata. 

Pasted text

	Bypass: no version, source hash, or build identity is verified.
Graph pruning	It constructs the deletion partition, selects a largest component, removes stayers in match mode, prunes insufficient workers and worker articulations, and retains a final component. 

Pasted text +1

	Structurally sufficient for the FE deletion-rank premise, but tied components are resolved by encoded node labels.
Backend dispatch	auto chooses exact when the parameter count is within exact_limit() and otherwise JLA; explicit exact or JLA is respected. 

Pasted text

	No hidden retry or backend substitution found.
Fit	Exact mode factors the weighted full system. JLA eliminates workers, solves the grounded firm Schur system, and augments it with an exact low-dimensional control Schur complement. 

Pasted text +2

	Scalar FE solves have a recomputed full residual. Joint matrix solves do not enforce that gate separately for every right-hand side.
Leverage sketch	Physical signs are compressed to binomial sums. Match contractions are rank-one physical-copy contractions. Observation mode instead pools copy residual squares within each stored row before the constrained ratio. 

Pasted text +1

	Match compression is exact. Observation pooling changes the finite-R estimator when f
r
	​

>1.
Finite inverse correction	Code computes coefficient-one B, V, observation 1/m+B/m
2
−V/m
3
, and the corresponding block inverse expansion. 

Pasted text

	Algebraically correct for the random variable the code selected.
Target sketch	Independent target probes factor the target centering operator; worker, firm, and total use common directions, and covariance is recovered probe by probe. 

Pasted text

	Correct target factorization and accounting.
Posting	Any backend status CONVERGED is sliced and posted as the current ado version and KSS_POINT_ESTIMATES_ONLY, with frequency convention labeled “literal physical copies.” 

Pasted text +1

	No postcondition verifies runtime identity, matrix identities, or that the result used the advertised frequency algorithm.
2. Critical finding 1: pooled observation frequencies violate literal-expansion finite-R semantics
2.1 What the formula actually computes

Suppose a stored row represents f identical physical copies. For one probe, all copies have the same projected coordinate p, while their signs are q
a
	​

∈{−1,1}, with S=∑
a
	​

q
a
	​

. Then

f
1
	​

a=1
∑
f
	​

(q
a
	​

−p)
2
=1+p
2
−
f
2pS
	​

.

Thus the production expression

V
pool
	​

=1+p
2
−
f
2pS
	​


is exactly the within-probe average of copy residual squares. Its raw second moment is therefore

V
pool
2
	​

=(1+p
2
−
f
2pS
	​

)
2
,

not f
−1
∑
a
	​

(q
a
	​

−p)
4
. On this narrow point, the derivation and code are correct. The documentation makes precisely this distinction, and the runtime squares the pooled quantity. 

Pasted text +1

2.2 Why it is nevertheless the wrong public finite-R algorithm

On literally expanded data under deletion(observation), each copy is a separate deletion unit. Across R probes, copy a obtains its own residual-square average 
M
a
	​

, its own constrained ratio

M
ˉ
a
	​

=
P
+
M
a
	​

M
a
	​

	​

,

and its own nonlinear B
a
	​

,V
a
	​

, and inverse weight.

The collapsed code instead first averages copy residual squares within every probe, retains no copy identity across probes, and forms one nonlinear ratio for the entire stored row. In general,

f
1
	​

a
∑
	​

P
+
M
a
	​

M
a
	​

	​


=
P
+
f
1
	​

∑
a
	​

M
a
	​

f
1
	​

∑
a
	​

M
a
	​

	​

.

The packet explicitly acknowledges that the two algorithms differ and supplies a finite example with values 0.373684… and 0.382352…. 

Pasted text +1

That admission does not resolve the specification conflict. The authoritative specification says:

frequency weights are literal physical copies;

observation deletion removes one physical copy;

collapsed random quantities must reproduce the explicitly expanded calculation under those semantics; and

implementation choices affecting the estimator or results require owner discussion rather than unilateral narrowing. 

Pasted text +1

It also specifically identifies “frequency weights require narrower supported semantics” as an issue that must be brought back to the owner. 

Pasted text

Line-specific specification verdict

The pooled algorithm is not within the existing specification. It is an internally coherent new finite-probe convention, but it is an unauthorized narrowing because the public command run on literal copies uses the coordinatewise algorithm, while the collapsed call uses the pooled algorithm. Convergence to the same limit as R→∞ does not establish finite-R equivalence.

2.3 Four-row accepted counterexample

Use four stored rows and five physical copies:

Stored row	Worker	Firm	Frequency	y
1	1	1	2	1
2	1	2	1	0
3	2	1	1	0
4	2	2	1	0

Run pure two-way FE, default target mass, deletion(observation), algorithm(jla), and probes(2).

This is a K
2,2
	​

 mobility graph. It has no worker articulation, both workers have at least two physical observations, the stored-row count is four, and the identified coefficient count is three. It therefore passes the structural production gates. The relevant graph checks are implemented at lines 2788–2807. 

Pasted text

Let physical-copy order be (1a,1b,2,3,4), and take the positive-probability first-pass probes

q
(1)
=(−1,−1,−1,−1,−1),q
(2)
=(−1,+1,−1,−1,+1).

For the two copies of stored row 1:

U=(1,9/49),

while the copywise residual-square draws are

V
1a
	​

=(0,16/49),V
1b
	​

=(0,100/49).

The pooled residual-square draws are

V
pool
	​

=(0,58/49).

Consequently,

M
ˉ
1a
	​

=
37
8
	​

,
M
ˉ
1b
	​

=
79
50
	​

,

whereas the pooled calculation gives

M
ˉ
pool
	​

=
2
1
	​

.

After the production B,V adjustment, the two coordinatewise inverse weights sum to

784
2257
	​

+
4900
1501
	​

=
19600
62429
	​

≈3.185153,

but the pooled per-copy weight is 40/49, giving total stored-row weight

2
49
40
	​

=
49
80
	​

≈1.632653.

All these weights are positive; the estimated variances are nonnegative; no JLA inverse or positivity gate fails.

Now take both target probes to be

q
∗
=(−1,−1,−1,−1,+1).

The common plug-in vector is

(
1225
96
	​

,
1225
96
	​

,
1225
16
	​

,
175
32
	​

).

The literal expanded coordinatewise correction is

52521875
561861
	​

(1,1,1,4)≈(0.010697657,0.010697657,0.010697657,0.042790628),

while the collapsed pooled correction is

420175
2304
	​

(1,1,1,4)≈(0.005483430,0.005483430,0.005483430,0.021933718).

Thus the accepted corrected outputs are respectively approximately

(0.06766969, 0.06766969, 0.00236357, 0.14006652)

and

(0.07288392, 0.07288392, 0.00757779, 0.16092342).

Because the two target probes are identical, the reported conditional target-pass MCSE is zero even though the leverage-stage finite-R algorithms disagree.

The current Stata tests do not test equality of these finite-R laws. They compare the collapsed and expanded paths separately to the exact result at 4,000 probes, where both algorithms approach the same limit. 

Pasted text +1

Required repair

Until the owner explicitly changes the contract, production should do one of the following:

retain copy identities across leverage probes and aggregate the resulting coordinatewise inverse weights, possibly through streamed physical-copy histories;

use exact mode for such calls; or

fail closed for algorithm(jla) deletion(observation) whenever any retained f
r
	​

>1.

A separately named, owner-approved option could expose the pooled exchangeable approximation, but it must state that its finite-R law differs from literal expansion. It must not be labeled simply “literal physical copies.”

This defect does not invalidate:

exact observation deletion;

JLA observation deletion when every f
r
	​

=1; or

JLA match deletion, where the declared block has one common rank-one contraction and the compressed contraction agrees with literal expansion. The match identity is explicitly tested direction by direction. 

Pasted text

3. Critical finding 2: an unverified API-7 Mata runtime can be accepted

The ado checks only

stata
assert(kssbc__api_level() == 7)

and loads a Mata file only if that assertion fails. It never calls the supplied kssbc__version() function, compares a build identifier, or verifies the runtime’s source hash. 

Pasted text

The Mata file separately exports both a version and an API level, but the version is unused by the ado. 

Pasted text

A minimal accepted counterexample is therefore:

An older or alternate Mata runtime is already loaded.

It defines kssbc__api_level() as 7 and exports the expected wrapper signatures.

Its kssbc__stata_jla() returns a four-by-four result and status CONVERGED.

The current ado skips loading its own runtime, posts the returned results, labels them version 0.1.0-dev, and reports KSS_POINT_ESTIMATES_ONLY. 

Pasted text +1

The package-layout test checks only that the source strings contain matching API literals; it does not exercise stale-runtime binding. 

Pasted text

This is a production control-flow false acceptance, independent of the mathematical formulas.

Required repair

Add an immutable build fingerprint, for example:

kssbc__build_id() =
"api7-f-2eba40b54226abfd030f19e82b899c0c7ace2b55def9d84c87938db0f45cc6d5"

The ado should verify all of:

API level;

semantic version;

exact build ID; and

expected wrapper capability.

A mismatch should return a typed RUNTIME_BUILD_MISMATCH. The ado should also verify before posting that:

corrected=plugin−correction

and that the total accounting identity holds in every posted row.

4. Critical finding 3: joint-control batched residuals are not gated per right-hand side

The numerical documentation promises that every batched PCG right-hand side retains its own convergence and full-residual gate. 

Pasted text

The pure FE matrix wrapper does satisfy that promise: it loops over columns, invokes the scalar solver, and takes the maximum of the columnwise residuals. 

Pasted text

The joint-control solver does something different. After solving all columns, it computes one Frobenius norm:

1+∥B∥
F
	​

∥R∥
F
	​

	​


for the entire residual and entire right-hand-side matrix. 

Pasted text

That is not equivalent to requiring

j
max
	​

1+∥b
j
	​

∥
2
	​

∥r
j
	​

∥
2
	​

	​

≤τ.

For example, suppose the registered threshold is τ, one column has

b
1
	​

=0,∥r
1
	​

∥=2τ,

so it fails its required columnwise gate, while another column has zero residual and ∥b
2
	​

∥=L≥1. The matrix check gives

1+L
2τ
	​

≤τ

and accepts.

This check is used for target batches containing up to 2batch() columns. 

Pasted text

 The low-dimensional control preparation likewise checks the whole Schur discrepancy through a single matrix norm rather than a column-scaled maximum. 

Pasted text

I did not execute a Stata floating-point fixture that realizes the masking state, but the acceptance inequality itself demonstrably permits a right-hand side that violates the documented gate. It therefore cannot serve as the claimed fail-closed certificate.

Required repair

For every joint solve, compute and gate

ρ
j
	​

=
1+∥b
j
	​

∥
2
	​

∥H
x
j
	​

−b
j
	​

∥
2
	​

	​


separately, then report max
j
	​

ρ
j
	​

. Apply the analogous column-equilibrated check to control-Schur preparation. Add a test with one zero or small right-hand side beside a much larger column.

5. Critical finding 4: tied component selection is not invariant to ID relabeling

The component routine ranks components by firm count and physical mass, but exact ties are broken by the numerical union-find root:

node < best_root

Pasted text

Those node numbers are derived from dense encodings of the user’s worker and firm values. 

Pasted text

 Thus a one-to-one relabeling can change which tied component is selected.

A minimal construction uses two disconnected four-edge K
2,2
	​

 components. Give both two firms and physical mass four, but use outcome vector (1,0,0,0) in the first and (2,0,0,0) in the second. Both components separately pass exact observation-deletion gates. With the first component assigned the lower dense worker labels, it is retained. Swap the worker and firm labels between components, leaving the graph, observations, and partition otherwise unchanged, and the second component is retained. Because the estimator is quadratic in y, the returned quantities differ by a factor of four.

The validation specification requires invariance to identifier relabeling, row ordering, and harmless sorting. 

Pasted text

 The existing relabeling test uses one connected component and therefore cannot expose this tie path. 

Pasted text

Required repair

An exact tie on all declared component-ranking criteria should fail closed with a typed status such as AMBIGUOUS_LARGEST_COMPONENT, unless the owner specifies an additional label-invariant selection rule. Outcome- or control-dependent tie-breaking should not be introduced silently because that would create a new data-dependent sample-selection rule.

6. Critical finding 5: the finite-precision rank gap is not a certified lower bound

The exact mathematical trace argument is valid, but the implementation’s numerical error budget does not justify calling the posted quantity a lower bound.

After whitening, the code accepts

∥
G
−I∥
F
	​

≤max(10
−9
,1000rank_tolerance)

but later requires only

1−
tr
(L
g
	​

)>max(10
−10
,100rank_tolerance).

Pasted text +1

At the permitted rank_tolerance(1e-14), the whitening discrepancy may be 10
−9
, while a reported gap of only slightly more than 10
−10
 is accepted.

The acceptance inequalities themselves permit the following one-control numerical state:

G
=1−5×10
−10
,

which passes the 10
−9
 whitening check. Suppose the deletion removes all remaining within-cell scatter, so

Δ
=
G
,
G
−g
	​

=0.

The computed loss is 1−5×10
−10
, giving

deletion_rank_gap=5×10
−10
>10
−10
,

so the code returns CONVERGED, even though the deleted control scatter is singular.

The issue is that when the actual whitened full scatter is T

=I, the generalized loss is governed by (D,T), not merely by tr(D). A valid bound is, for example,

λ
max
	​

(T
−1/2
DT
−1/2
)≤
λ
min
	​

(T)
tr(D)
	​

.

The posted 1−tr(D) is therefore not a certified lower bound unless whitening error is incorporated. The documentation currently describes it as a probe-independent lower bound. 

Pasted text

Required repair

Let

ϵ=∥T−I∥
2
	​

≤∥T−I∥
F
	​

.

Require at least

1−ϵ
tr
(D
g
	​

)+η
g
	​

	​

<1−δ

for every deletion, where η
g
	​

 covers accumulation error and δ is the registered safety margin. Equivalently, require the reported gap to dominate the measured whitening and accumulation errors by a fixed factor. Report this discounted gap, not 1−max
g
	​

tr
(D
g
	​

).

7. Independent derivation of B, V, and the inverse corrections
7.1 Coefficient-one finite-projection formulas

Let

P
=R
−1
r=1
∑
R
	​

U
r
	​

,
M
=R
−1
r=1
∑
R
	​

V
r
	​

,

with E[U]=P, E[V]=M, and P+M=1. Define

f(p,m)=
p+m
m
	​

.

At p+m=1,

∇f=(−M,P)

and

∇
2
f=(
2M
M−P
	​

M−P
−2P
	​

).

By the standard second-order multivariate Taylor expansion,

E[f(
P
,
M
)]−M=
2R
1
	​

tr[∇
2
fVar(U,V)]+O(R
−2
).

Writing

a
2
	​

=E[U
2
],b
2
	​

=E[V
2
],c=E[UV],

and using P+M=1, the products of first moments cancel, yielding

B=
R
1
	​

{Ma
2
	​

−Pb
2
	​

+(M−P)c}.
	​


There is one—not two—coefficient on the mixed raw moment.

The first-order delta variance is

V=
R
1
	​

{M
2
a
2
	​

+P
2
b
2
	​

−2PMc}.
	​


These agree with the documentation and production expressions. 

Pasted text +1

7.2 Observation inverse

Let 
m
=m+δ, with

E[δ]≃B,E[δ
2
]≃V.

Expanding 1/(m+δ),

E[
m
1
	​

]≃
m
1
	​

−
m
2
B
	​

+
m
3
V
	​

.

Thus the second-order corrected inverse is

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

.
	​


With joint controls, subtracting the exact control leverage c gives

m
full
=
m
FE
−c; the random error, B, and V are unchanged. The code’s signs are correct. 

Pasted text +1

7.3 Noncommuting match-control block

By Frisch–Waugh–Lovell,

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
,U=(I−P
A
	​

)Z,S=U
′
U.

For a match block, the FE restriction is

(P
A
	​

)
gg
	​

=hvv
′
,

while the exact control restriction is

C=U
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

M(h)=I−C−hvv
′
.

No commutation between C and vv
′
 is needed. For

g(h)=M(h)
−1
e,

the inverse derivative identity gives

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

Because 
h
=1−
m
,

E(
h
−h)≃−B,Var(
h
)≃V.

Therefore

g
bc
	​

=g(
h
)+Bg
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
	​


This is exactly the production +B, −V block formula, and it remains valid when the control block and FE rank-one block do not commute. 

Pasted text +2

8. Joint-control deletion-rank certificate
8.1 Exact-arithmetic proof

Let G be the weighted control scatter after separately centering controls within every retained worker–firm cell. For deletion g, let

G
−g
	​

=G−Δ
g
	​

.

Deleting observations and recomputing the affected cell mean reduces the within-cell scatter, so

Δ
g
	​

⪰0.

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
⪰0.

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

Therefore

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

If a nonzero control combination became collinear with the retained FE design after deletion, it would be constant within each retained worker–firm cell and would consequently have zero G
−g
	​

 scatter, contradicting G
−g
	​

≻0. Combined with retained FE rank, the condition is sufficient. This is the packet’s stated proof. 

Pasted text

8.2 Weighted observation and match updates

The implementation correctly distinguishes the deletion semantics:

for observation deletion, it removes weight one from a stored row’s cell even when that stored row has f
r
	​

>1;

for match deletion, it removes every frequency unit in all stored rows carrying the declared deletion ID and recomputes the remaining cell mean.

The weighted sums, weighted squares, remaining frequency, and recentered scatter are updated accordingly. 

Pasted text +1

8.3 FE graph premise

For a simple worker–firm graph, deletion of a sole edge loses FE rank exactly when that edge is a bridge.

After production pruning:

in match mode, every retained worker has more than one fitted firm;

in observation mode, a worker supported by only one physical copy is removed; and

no retained worker is an articulation vertex. 

Pasted text

A bridge whose worker endpoint has degree greater than one makes that worker an articulation vertex. If its worker endpoint has degree one, match mode removes it as a stayer; observation mode either removes a one-copy worker or, when the edge has multiple physical copies, deleting one copy leaves the edge present. Thus the graph premise is sufficient for the pure FE block.

Rank-certificate verdict

Exact real-arithmetic theorem: valid and conservative.

Weighted observation update: valid.

Weighted match update: valid.

Combination with the FE graph premise: valid.

Finite-precision claim that the posted raw gap is a certified lower bound: false without the repair in Finding 5.

9. Other required attacks
Frequency and exact deletion scaling

The weighted fit uses

H=
r
∑
	​

f
r
	​

x
r
	​

x
r
′
	​

,

which is the literal-copy information matrix. Exact observation deletion uses per-copy leverage x
r
′
	​

H
−1
x
r
	​

, multiplies the correction by f
r
	​

, and therefore sums the identical physical-copy corrections correctly. 

Pasted text +1

Exact match deletion uses 
f
r
	​

	​

-transformed block rows and is an exact isometric compression of the block-constant physical-copy subspace. 

Pasted text +1

Verdict: valid outside the observation-JLA finite-R pooling defect.

Target scaling

For explicit stored-row target mass t
r
	​

, assigning each physical copy mass t
r
	​

/f
r
	​

 produces the compressed target direction

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

.

Its covariance is exactly

diag(t/T)−(t/T)(t/T)
′
.

Default t
r
	​

=f
r
	​

 gives equal physical-copy target mass. The production helper implements this formula. 

Pasted text +1

Verdict: valid.

Deletion counts

Exact and JLA report

∑
r
	​

f
r
	​

 observation deletion units; and

the number of declared IDs in match mode.

That is the correct physical count. 

Pasted text +1

The caveat is semantic: observation JLA reports physical deletion units while forcing all copies of a stored row to share one finite-R leverage ratio and inverse adjustment.

Mover-only headlines

Match mode removes stayers before fitting and targeting, and the posted target population is “movers.” stayers(both) is withheld rather than silently mixing a match correction with an observation fallback. 

Pasted text +2

Verdict: valid but deliberately conservative.

Conditional MCSE

The reported MCSE is the standard error of the second target-probe mean conditional on the realized leverage sketch. It excludes leverage-sketch, solver, and econometric sampling uncertainty. That limitation is accurately stated. 

Pasted text +1

It is therefore possible, as in the frequency counterexample, to obtain zero target-pass MCSE while having substantial leverage-stage approximation error or a different leverage algorithm. This does not make the stated conditional MCSE formula wrong, but it makes prominent labeling essential.

Accounting identities

Worker, firm, and total target contractions use common directions, and covariance is defined probe by probe as

2
total−worker−firm
	​

.

Thus plugin, correction, and corrected results satisfy

T=W+F+2C

up to floating arithmetic. 

Pasted text +1

Verdict: valid.

Exceptional probe events

Estimable observations and blocks can have zero estimated residual mass or zero constrained denominator with positive finite-probe probability. The tests demonstrate these events. 

Pasted text

The command withholds rather than retries with new probes or reduces the block. The documentation also correctly declines to claim unbiasedness conditional on acceptance. 

Pasted text

Verdict: conservative withholding, not a defect.

10. Findings by disposition
Critical defects

Unauthorized pooled observation-frequency estimator
kss_bc/docs/JLA_FINITE_PROJECTION.md:656–668; kss_bc/kss_bc.mata:3195–3203; conflict with varcomp_hdfe_specification.md:5628–5647.
Withhold JLA observation calls with f
r
	​

>1, implement literal-copy coordinatewise histories, or obtain owner approval for a separately labeled pooled algorithm.

Unverified API-7 runtime binding
kss_bc/kss_bc.ado:1104–1111, with current-version posting at 1319–1348.
Require version and exact build fingerprint.

Global rather than per-RHS joint residual gate
kss_bc/kss_bc.mata:2321–2329, contrary to NUMERICAL_ARCHITECTURE.md:798–801.
Gate the maximum scaled residual over columns.

ID-dependent tied-component selection
kss_bc/kss_bc.mata:2499–2513.
Fail closed on unresolved component ties or specify an owner-approved label-invariant rule.

Deletion-rank numerical gap omits whitening error
kss_bc/kss_bc.mata:2916–2923, 2977–2985.
Discount the gap by measured whitening, inverse, and accumulation error.

Conservative withholdings

The compatibility graph rule is stronger than deletion-specific connectedness and can discard valid designs. This is documented and permitted. 

Pasted text

The exact-arithmetic trace bound may reject valid joint-control designs when the trace exceeds one even though the maximum generalized eigenvalue is below one. This is explicitly acknowledged. 

Pasted text

Requiring stored n>p can reject a frequency-compressed observation design whose literal expansion has enough physical residual degrees of freedom. The exact and JLA checks use stored rows. 

Pasted text +1

Exact size and block-size limits fail explicitly rather than silently switching algorithms. 

Pasted text

One failed block aborts all requested output rather than returning a partial target. 

Pasted text

These are conservative rejections, not false acceptances.

Noncritical documentation and test repairs

Add a finite-R, same-physical-probe comparison between collapsed observation frequency and the public command on expanded rows. The existing test proves the two ratios differ but then treats the pooled algorithm as the selected contract. 

Pasted text

Add a dynamic test that preloads an incompatible API-7 runtime and verifies a typed build mismatch.

Add a target/control batch with one small or zero RHS and one large RHS to test columnwise residual gates.

Exhaustively test the weighted deletion-rank certificate for observation deletion from f>1 and for partial and full within-cell match blocks.

Add tied-component relabeling tests.

Display fixedoffset as “conditional on the full-sample nuisance estimate,” not merely as the option name. The contract expressly distinguishes it from joint leave-out estimation, while the display currently provides only the terse nuisance label. 

Pasted text +1

State the MCSE exclusions directly next to the displayed numerical row.

Final line-specific verdict

FALSE — KB5-PRODUCTION-SAFETY and KB5-FREQUENCY-SEMANTICS are not satisfied.

KB5-JLA-AND-BLOCK-ALGEBRA is substantially valid at the formula level, including:

coefficient-one B;

V;

observation +B,−V inverse correction;

noncommuting control-block +B,−V correction;

match frequency contraction;

target contraction and accounting; and

the exact-arithmetic trace-rank theorem.

The package should not return the unrestricted production-success label until the five critical defects above are repaired. As an interim fail-closed envelope:

exact mode may be retained after runtime-identity and tied-component repairs;

JLA match mode and JLA observation mode with all frequencies equal to one may be retained after the per-RHS and numerical-rank-margin repairs;

JLA observation mode with any frequency greater than one should be withheld unless literal coordinatewise expansion semantics are implemented or the owner explicitly authorizes and relabels the pooled finite-R estimator.
