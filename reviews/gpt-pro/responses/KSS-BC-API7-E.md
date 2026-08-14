---
review_id: KSS-BC-API7-E
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 54ff42b495ca5d49380eb5f1ad669fa670da4fa1211d1ce642d9b6d141003597
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f0179-b56c-83ea-81c6-374b4b5f5681
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

FALSE — KB5 cannot close under the governing specification.

The coefficient-one finite-projection algebra, the selected pooled-V moment calculation, the joint-control inverse adjustment, the deterministic deletion-rank certificate, target scaling, and four-target accounting are internally correct. The failure is more specific:

The pooled observation-frequency algorithm is a mathematically coherent finite-R estimator, but it does not satisfy the governing specification’s ordinary literal-expansion requirement. It is not the finite-R algorithm obtained by physically expanding a frequency-weighted row, running the observation-level JLA separately for every copy, and averaging the resulting copywise corrections.

The production caller can return different successful point estimates for the same physical dataset depending only on whether identical copies are stored as one frequency-weighted row or as separate rows.

A separate deterministic counterexample exists in connected-component selection: when two components tie on the documented criteria, the code chooses according to an identifier-dependent union-find root. Relabeling IDs can therefore change the retained sample and the returned point estimate despite the stated relabeling-invariance requirement.

The pooled implementation can become valid under an explicit owner-approved narrowing of the finite-projection contract. That approval is not recorded in the supplied owner decisions. The milestone is KB5 and is marked critical. 

Pasted text

Source-bound qualification

The raw uploaded attachment hashes to:

9cfdcf84bd63aa973c1777aa69e7e795c57bd71de62735c5b124ae95d76c1a37

not the stated packet hash 54ff42b495ca5d49380eb5f1ad669fa670da4fa1211d1ce642d9b6d141003597. Common final-newline and LF/CRLF normalizations also did not produce the stated hash.

However, every embedded file, extracted without the separator newline, matches its individual SHA-256 in the packet source map. I therefore bind the substantive review below to those verified embedded files. The declared source map is at packet lines 47–64. 

Pasted text

1. Production control-flow trace
1.1 Public Stata caller

The public command:

defaults to match deletion, automatic exact/JLA selection, joint nuisance estimation, mover targeting, 200 probes, and the registered numerical tolerances;

validates positive integer frequency weights;

treats an explicit targetweight() as total stored-row target mass;

otherwise assigns target mass equal to frequency;

constructs the requested deletion partition separately from the worker and firm coordinates. 

Pasted text +1

The caller then:

builds and prunes the worker–firm graph;

recodes the final worker, firm, and deletion IDs;

chooses exact or JLA from the coefficient dimension;

canonicalizes stored-row order;

dispatches the same ordered arguments—outcome, worker, firm, controls, frequency, target mass, deletion ID, sample, deletion convention, and nuisance convention—to the appropriate Mata entry point. 

Pasted text +1

The JLA and exact wrappers read those arguments in the same order. Their four result rows and 22 diagnostic positions agree with the order assumed by the ado caller. I find no positional argument or diagnostic-index mismatch in this dispatch. 

Pasted text +1

The caller labels the four result columns as worker variance, firm variance, worker–firm covariance, and total variance; labels the rows as plug-in, correction, corrected, and numerical MCSE; posts the corrected row in e(b) and e(kss); and posts success only after Mata returns CONVERGED. 

Pasted text +1

1.2 Exact formulas

The exact backend constructs

H=X
′
WX,
β
	​

=H
−1
X
′
Wy,

with all worker coordinates, one omitted firm coordinate, and joint controls where requested. Its target matrices implement weighted centering, put zero target action on controls, and define total as worker plus firm plus twice covariance. 

Pasted text +1

For observation deletion it computes per-copy leverage h
i
	​

=x
i
′
	​

H
−1
x
i
	​

, multiplies the correction by the number of physical copies, and divides the residual by 1−h
i
	​

. For match deletion it uses the frequency-transformed block, forms I−X
g
	​

H
−1
X
g
′
	​

, solves the block residual identity, and contracts the target-specific block matrix. Total correction is again component one plus component two plus twice component three. 

Pasted text

The final sign is correct:

θ
KSS
=
θ
plugin
−
b
.

The code sets corrected = plugin - correction; there is no sign reversal. 

Pasted text

1.3 JLA formulas

The first JLA pass:

draws physical-copy Rademacher sums;

solves the pure two-way FE projection;

accumulates projection and residual-square means and raw second moments;

constrains projection and residual shares to sum to one;

applies the coefficient-one bias formula and corresponding variance formula;

adds the exact residualized-control contribution separately. 

Pasted text

Observation deletion then uses

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

,

while match deletion uses the full control block plus the estimated FE rank-one term and applies the corresponding first- and second-derivative adjustment. The signs in production agree with the derivation. 

Pasted text

The second pass uses independent target directions, common worker/firm directions, and defines covariance probe by probe from total minus worker minus firm. Observation corrections carry the physical-copy multiplier; match corrections use the two correct transformed block contractions. 

Pasted text

2. Pooled-frequency specification decision
2.1 Correct pooled variables

Let stored row i represent f
i
	​

 identical physical copies. For probe ℓ, write

S
iℓ
	​

=
a=1
∑
f
i
	​

	​

q
iaℓ
	​

,q
iaℓ
	​

∈{−1,1}.

Every copy has the same projected coordinate

p
iℓ
	​

=x
i
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
sℓ
	​

.

For a pooled-before-ratio algorithm, the appropriate per-probe variables are

U
iℓ
	​

=p
iℓ
2
	​

,

and

V
iℓ
	​

	​

=
f
i
	​

1
	​

a=1
∑
f
i
	​

	​

(q
iaℓ
	​

−p
iℓ
	​

)
2
=1+p
iℓ
2
	​

−2p
iℓ
	​

f
i
	​

S
iℓ
	​

	​

.
	​


This is exactly the candidate documentation’s construction. 

Pasted text

Because the nonlinear ratio is formed from the sample means of U and this pooled V, its raw second moments must be

E[U
2
],E[V
2
],E[UV].

Therefore:

E[V
2
]=E
	​

{
f
1
	​

a
∑
	​

(q
a
	​

−p)
2
}
2
	​


is correct for the selected pooled algorithm.

It is not

E[
f
1
	​

a
∑
	​

(q
a
	​

−p)
4
].

Substituting the average copywise fourth moment into a delta expansion whose random variable is the pooled residual square would mix two different random variables. The production line

mata
m_second = m_second + observation_residual_square:^2

is therefore correct for the pooled specification. 

Pasted text

2.2 Frequency-two fixture

For the supplied frequency-(2,1,2) fixture, the first frequency-two row has

E[U]=0.45,E[V]=0.55,

and the relevant raw moments are

E[U
2
]=0.44025,E[V
2
]=0.54925,E[UV]=0.03975.

The average copywise fourth moment is instead

E[
2
r
1
4
	​

+r
2
4
	​

	​

]=0.63925.

The packet’s exhaustive enumeration independently records the key difference 0.54925 versus 0.63925. 

Pasted text

With the coefficient-one formula, the pooled second raw moment gives

B=.55(.44025)−.45(.54925)+(.55−.45)(.03975)=−.00105.

Using the copywise fourth moment would give −.04155. The supplied formula test records exactly these two values and its simulation favors the pooled calculation for the pooled ratio. 

Pasted text

Decision on the moment question: for the selected pooled random variable, the second raw moment must be E[V
2
], the square of the pooled copy-residual second moment. The production code is right on this narrow mathematical question.

2.3 What is equal and what is not

The collapsed implementation does reproduce, draw for draw as a function of S
i
	​

:

the common projected value of every copy;

U=p
2
;

the pooled residual square V=f
−1
∑
a
	​

(q
a
	​

−p)
2
;

U
2
, V
2
, and UV;

the match-level rank-one contraction;

the collapsed target direction.

It does not reproduce at finite R:

the individual copy residual squares (q
a
	​

−p)
2
;

the separate sample mean 
M
a
	​

 retained for each expanded copy;

each copy’s constrained nonlinear ratio;

each copy’s finite-bias and finite-variance adjustment;

the average of the resulting copywise correction weights.

In general,

P
+
M
M
	​


=
f
1
	​

a=1
∑
f
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

,
M
=
f
1
	​

a
∑
	​

M
a
	​

.

The packet expressly admits this distinction: it says the implementation reproduces the expanded calculation only after pooling copies within their original stored row and does not reproduce the separate-copy nonlinear algorithm. 

Pasted text

The supplied finite four-probe fixture obtains

0.3823529412

for the pooled ratio and

0.3736842105

for the coordinatewise expanded-data ratio. 

Pasted text

Both approaches converge to the same leverage as R→∞, but equality of limits is not equality of the finite randomized point algorithm.

2.4 Does pooling satisfy the governing specification?

No.

The governing specification says:

frequency weights represent literal physical copies;

observation deletion removes one physical copy;

the semantics must agree with literal data expansion;

collapsed JLA quantities must reproduce the distribution and fourth moments of the explicitly expanded calculation;

validation must include agreement between integer-frequency results and literal expansion. 

Pasted text +1

The same specification states that an implementation choice affecting results must be returned to the owner rather than adopted unilaterally. 

Pasted text

The recorded owner decisions authorize the coefficient-one change, but they do not authorize pooled-before-ratio observation-frequency semantics. 

Pasted text

The pooled algorithm satisfies an amended reference of the following form:

Expand every stored row into copies, preserve an immutable original-stored-row grouping, pool the copy residual squares within that group on every probe, and only then form the nonlinear ratio.

That is not the ordinary result of literally expanding the data and running the same observation-level command. It also makes the finite-R result depend on an arbitrary storage partition: one row with frequency two is treated differently from two otherwise identical rows with frequency one.

This is a critical documented narrowing, not an error in the pooled delta algebra.

3. Minimal production point-estimate counterexample

The following finite example uses the actual production formulas and passes the pure-FE graph and numerical gates.

Use observation deletion, no controls, default equal physical-copy target mass, and two probes. The stored data are:

y	worker	firm	frequency
1	1	1	2
2	1	2	1
4	2	1	1
8	2	2	1

The physical data have five observations: two identical copies of the first row. The worker–firm graph is K
2,2
	​

, so every worker has two firms and no worker is an articulation vertex.

In canonical physical-copy order, take the two leverage probe directions

q
(1)
=(−1,−1,−1,−1,−1),
q
(2)
=(−1,1,−1,−1,1),

and use the target direction

q
T
=(−1,−1,1,1,1)

on both target probes. Each such sequence has positive probability under the registered probe law.

The weighted fit is the same under both storage representations. Its plug-in vector is

(4.4081633, 1.2538776, 0.3918367, 6.4457143).

For the two copies in the frequency-two cell:

the current collapsed pooled algorithm gives one common adjusted inverse weight

0.8163265;

literal expansion into two frequency-one rows gives separate adjusted inverse weights

2.8788265and0.3063265.

All are finite and positive, and the estimated residual shares are safely above the registered boundary.

The resulting corrections and corrected points are:

Representation	Correction vector	Corrected vector
One stored row with f=2, pooled production algorithm	(−0.0908320, 0.0772757, 1.7583531, 3.5031499)	(4.4989953, 1.1766018, −1.3665163, 2.9425644)
Two literal frequency-one rows, coordinatewise production algorithm	(−0.0630228, 0.1050850, 1.7861623, 3.6143867)	(4.4711860, 1.1487926, −1.3943255, 2.8313276)

Thus the successful corrected total is either

2.9425644

or

2.8313276

for the same physical observations, solely because their identical copies were stored differently.

The source of the difference is precisely the observation pooling at Mata lines 3218–3226, followed by the common per-stored-row inverse weight at lines 3277–3289 and the frequency-multiplied target contraction at lines 3375–3382. 

Pasted text +2

This is a finite production counterexample to the unqualified returned label "literal physical copies". That label is posted at the public caller without stating the pooled finite-R exception. 

Pasted text

4. Coefficient-one ratio expansion

Let

P
=P+δ
P
	​

,
M
=M+δ
M
	​

,P+M=1,

and

f(p,m)=
p+m
m
	​

.

At P+M=1,

∇f=(−M,P),

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

The second-order bias is

2
1
	​

tr{∇
2
fCov(
P
,
M
)}.

The two symmetric off-diagonal covariance terms contribute a factor two inside the trace, but the Taylor factor 1/2 cancels it. Hence the mixed raw moment has coefficient one, not two:

B=
R
1
	​

[ME(U
2
)−PE(V
2
)+(M−P)E(UV)].

Similarly,

V=
R
1
	​

[M
2
E(U
2
)+P
2
E(V
2
)−2PME(UV)].

The packet’s symbolic derivation and simulation support this formula, and the owner decision expressly selects coefficient one. 

Pasted text +1

Production implements exactly

mata
finite_bias =
    (m_constrained:*p_second
     - p_constrained:*m_second
     + (m_constrained-p_constrained):*mixed_second) :/ probes

with no extra factor two. 

Pasted text

Because B and V already contain R
−1
, replacing population moments by same-probe estimates changes them by O
p
	​

(R
−3/2
), provided the true residual share remains bounded away from zero. This is a second-order plug-in expansion, not an exactly unbiased finite-R identity. The documentation states that limitation correctly. 

Pasted text

Failed finite-probe gates do not trigger a retry, a reduced probe count, or a dropped block. Mata returns a typed failure and the ado caller posts WITHHELD. Consequently, the code also correctly refrains from claiming conditional unbiasedness after selection on the gate event. 

Pasted text +1

5. Joint controls and deletion-rank certificate
5.1 Projection decomposition

For frequency-transformed FE design A and controls Z, let

U=(I−P
A
	​

)Z,S=U
′
U.

If the identified FE quotient has full rank and S≻0,

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

This is the standard orthogonal FWL projection split. The implementation:

solves the FE inverse actions for A
′
Z;

forms residualized controls;

constructs the low-dimensional Schur complement;

checks it against U
′
WU;

inverts it with spectrum and residual gates. 

Pasted text +1

5.2 General match block

Within an actual match, the FE contribution is rank one,

h
g
	​

v
g
	​

v
g
′
	​

,v
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

,

but the control contribution is the full matrix

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

The resulting residual maker is

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

Production retains the full noncommuting control matrix; it does not replace varying within-match controls by a mean row. 

Pasted text +1

For g(h)=M
g
	​

(h)
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

Since the finite bias B is the bias of the residual share m=1−h, the bias of 
h
 is −B. Bias-correcting g(
h
) therefore requires

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

Production has exactly those signs. 

Pasted text +1

5.3 Deterministic rank certificate

Let G be weighted control scatter after removing a separate mean in every retained worker–firm cell, and let deleting g remove scatter Δ
g
	​

⪰0. If

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
,

then

G
−g
	​

≻0⟸λ
max
	​

(L
g
	​

)<1.

Because L
g
	​

⪰0,

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

Thus the implemented condition

g
max
	​

tr(L
g
	​

)<1

is sufficient. If a nonzero control combination became collinear with the FE span after deletion, it would be constant within each retained worker–firm cell and would have zero deleted within-cell scatter, contradicting G
−g
	​

≻0. 

Pasted text

The code independently recomputes the whitened scatter, requires it to be numerically close to the identity, calculates the deletion-specific trace loss, and withholds unless the gap is positive by the registered tolerance. 

Pasted text

I found no finite false-acceptance counterexample in this rank certificate. It is conservative, not invalid. The supplied block-only-control fixture also shows that deletion-induced rank loss is caught by both the exact block gate and the randomized path’s deterministic certificate. 

Pasted text +1

6. Target scaling and four-target accounting

An explicit stored-row target mass t
i
	​

 gives each physical copy mass t
i
	​

/f
i
	​

. With T=∑
i
	​

t
i
	​

, the collapsed target direction is

a
i
	​

=
f
i
	​

T
t
i
	​

	​

	​

S
i
	​

,d
i
	​

=a
i
	​

−
T
t
i
	​

	​

s
∑
	​

a
s
	​

.

Then

E(dd
′
)=diag(t/T)−(t/T)(t/T)
′
.

Production implements this formula directly. 

Pasted text +1

The public caller also implements the intended mass convention:

default stored-row target mass equals frequency;

explicit targetweight() is not multiplied by frequency. 

Pasted text

Worker and firm target solves use the same direction. Total projection is their sum, and covariance is defined probe by probe as

2
total−worker−firm
	​

.

Therefore both correction and corrected estimates satisfy

T=W+F+2C

up to floating-point accumulation. 

Pasted text

I find no target normalization, target-mass, covariance coefficient, or target matrix-order error.

7. Separate critical production counterexample: component ties

The documented component rule ranks components first by firm count and then by physical mass. 

Pasted text

The implementation adds an undocumented final tie-break:

mata
component_mass[node] == best_mass &
(best_root == 0 | node < best_root)

so an exact tie is resolved by the smaller union-find root. That root depends on encoded worker and firm labels. 

Pasted text

This conflicts with the governing validation requirement that results be invariant to ID relabeling. 

Pasted text

A minimal deterministic counterexample is two disconnected K
2,2
	​

 components:

each has two firms;

each has four physical observations;

each passes the mover and articulation checks once selected;

component A has outcomes (0,1,2,4);

component B has outcomes twice as large, (0,2,4,8).

Under exact singleton-match deletion, the independently calculated corrected total is 2 in component A and 8 in component B. Because firm count and physical mass tie, the code selects whichever component has the lower encoded root. A bijective relabeling that swaps the components’ numeric ID ordering swaps the retained component. The command then returns a successful point estimate of either 2 or 8, with no component-tie status.

This is a genuine production false acceptance relative to the relabeling-invariance contract. It is independent of JLA or frequency pooling.

8. Integration evidence: what it establishes and what it does not

The Python frequency tests correctly establish:

equality of the collapsed pooled residual square with the average expanded copy residual square on every probe;

equality of V
2
 with the square of that pooled quantity;

inequality with the average copywise fourth power;

exact match and target aggregation identities. 

Pasted text +1

They also explicitly establish that the pooled and coordinatewise finite-R ratios differ. 

Pasted text

The Stata frequency test does not test finite-R literal-expansion equivalence. It:

compares the collapsed weighted JLA result to exact at 4,000 probes;

separately expands the data and compares that JLA result to exact at 4,000 probes.

Because both finite algorithms converge to the same exact quantity, both comparisons can pass while the finite algorithms remain different. There is no assertion comparing the weighted and expanded JLA outputs directly under a coupled finite probe realization or comparing their finite-R laws. 

Pasted text +1

Thus the integration evidence validates asymptotic convergence toward exact frequency semantics, not the governing finite literal-expansion requirement.

9. Additional noncritical repairs
9.1 Exact joint-control diagnostic is not posted as documented

The return contract says e(control_schur_rcond) reports the exact residualized-control Schur reciprocal condition number when joint controls were prepared. 

Pasted text

The JLA backend populates it, but the exact backend always assigns missing:

mata
out.control_schur_rcond = .

even for exact joint-control calculations. 

Pasted text +1

This is a result-posting/documentation mismatch, not a point-estimate error. Either compute the exact control Schur condition number or narrow the documented availability.

9.2 Stored-row count causes unnecessary frequency-weight withholding

Both backends reject whenever

n
stored
	​

≤p,

before relying on the actual weighted rank and deletion gates. 

Pasted text +1

This can reject a valid literal-copy observation-deletion design. For example, take a K
2,2
	​

 design with one independent interaction control and frequency two on every stored row. Then n
stored
	​

=p=4, the physical sample has eight observations, the full design has rank four, and deletion of any one physical copy leaves all four unique design rows and retains rank. The exact backend nevertheless stops at the stored-row count check.

This is false withholding, not false acceptance. The check should be removed in favor of the existing information-rank and deletion-rank gates or restated as an intentional support restriction.

9.3 API-level guard does not bind the loaded Mata runtime to this build

The ado caller checks only

stata
assert(kssbc__api_level() == 7)

and skips loading the supplied Mata file whenever any already-loaded implementation reports API level 7. It does not compare kssbc__version() or a build identifier, although the Mata runtime exposes a version function. The ado then posts its own hard-coded current version. 

Pasted text +2

The static package test verifies only that the source strings contain the same API number; it does not test a stale same-level runtime. 

Pasted text

For a source-bound production claim, the caller should use a versioned entry point or check an exact semantic build token and reliably reload or fail when it differs.

10. Required repairs before KB5 closure
Critical repair 1: resolve observation-frequency semantics

The owner must select and record one of two contracts.

A. Literal coordinatewise expansion. Each physical copy retains its own residual-square sample mean, constrained ratio, finite-bias term, variance term, and inverse weight. The current S
i
	​

-only compression is insufficient because it loses copy identities across probes. A richer representation or logical expansion is required.

B. Pooled finite-projection narrowing. Keep the current implementation, but amend:

varcomp_hdfe_specification.md;

DECISIONS.md;

the public help;

the estimator contract;

e(frequency_convention).

The amended contract must state that finite-R observation JLA pools copies according to the original stored-row partition before applying the nonlinear ratio, is not invariant to splitting or merging identical stored rows, and is not the same as coordinatewise expanded-data JLA. Calling it simply “literal physical copies” is insufficient.

Whichever option is selected, add a finite-probe regression test that directly compares:

one row with f=2;

two identical rows with f=1;

using coupled physical directions. The test must either require equality or explicitly require and document the selected difference.

Critical repair 2: resolve tied connected components

When multiple components tie on both documented criteria, the command should:

withhold with a typed AMBIGUOUS_COMPONENT_TIE; or

require an explicit user component choice; or

adopt an owner-approved, genuinely label-invariant rule.

The current union-find-root tie-break must not silently determine the estimand.

Required documentation repair

Document that finite-probe gate failures may withhold an exactly estimable design and that accepted output has no conditional-unbiasedness claim. The internal derivation already says this correctly; the public help was not included in the packet, so its disclosure could not be verified. The packet’s own finite examples show positive-probability zero-residual or zero-denominator probe events despite exact estimability. 

Pasted text

Final line-specific disposition
Obligation	Disposition
Coefficient-one finite-projection expansion	Pass
Pooled U,V derivation	Pass for the selected pooled random variable
Choice of E[V
2
] versus average copywise fourth moment	E[V
2
] is required for pooled-before-ratio
Equality to coordinatewise expanded-data JLA at finite R	False
Compliance with current literal-expansion specification	Fail; critical narrowing lacks recorded owner approval
Exact frequency semantics	Pass, subject to the unnecessary stored-row count withholding
Match-frequency JLA contraction	Pass
Joint-control projection and inverse adjustment	Pass
Deterministic deletion-rank certificate	Pass; conservative but no false acceptance found
Target mass and probe scaling	Pass
Four-target accounting identity	Pass
Caller argument and matrix ordering	Pass
Exact control_schur_rcond posting	Fail; noncritical metadata mismatch
Component selection under exact ties	Fail; critical ID-dependent point-estimate counterexample
KB5 closure	Denied
