---
review_id: KSS-BC-API12-FINAL-M
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: ceb7fffed0a92bcdf9f9e7887fb0613fdef06a6f40d07025e48c671c25e1f47c
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f3dd6-f980-83ea-814e-c18021a32c25
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

API12 closure audit
1. Verdict

Verdict: false.

KB5 closure decision: do not close. One critical production-contract objection remains.

API12 correctly repairs the final plugin − correction finiteness hole, and the quotient-PCG, literal-frequency, block-control, rank-certificate, target-accounting, and withholding paths are substantially consistent with the stated finite point-estimator contract. However, the public exact and auto-to-exact paths do not supply the control canonicalizer with a control-basis-invariant row order. They instead sort on the raw control columns and encoded identifiers. Because the canonicalizer chooses the first row inside a fuzzy eligible set, an invertible transformation Z↦ZT can change the physical anchor while both runs remain accepted. That directly disproves the claimed accepted-path canonical-basis invariance and can change accepted exact results.

The governing obligation explicitly requires invariance for accepted control reparameterizations and permits withholding only when the anchor cannot be certified. 

Pasted text

2. Packet authentication

I independently parsed the pasted packet, reconstructed each body, and recomputed SHA-256 hashes.

Result: all 33 manifest-listed bodies authenticate exactly. This includes:

Group	Authenticated bodies
Governing/meta	PROMPT.md, REQUEST.yaml, SOURCE_MAP.md, FILES/varcomp_hdfe_specification.md
Package/runtime/docs	CHANGELOG.md, README.md, TESTING.md, the six files under docs/, kss_bc.ado, kss_bc.mata, kss_bc.sthlp
Python evidence	oracle.py and all six supplied Python test files
Stata evidence	All ten supplied Stata .do files

The 32 delimited bodies match their manifest entries byte-for-byte. The leading, undelimited PROMPT.md body at packet lines 1–44 matches its manifest hash after the ordinary single-terminal-LF normalization used by the source body. There are no missing or extra manifest-listed source bodies. The manifest and source map identify the same governing runtime and evidence set. 

Pasted text +1

The outer PACKET.zip hash cannot be independently authenticated from this transport because the packet expressly states that the ZIP bytes are unavailable. This does not impair authentication of the embedded source bodies. 

Pasted text

3. Findings
ID	Severity	Finding	Closure consequence
F1	Critical	Exact-mode preprocessing sorts on raw controls and encoded IDs before using a first-eligible-row anchor. Accepted Z and ZT runs can therefore obtain different physical canonical anchors.	Blocks KB5.
F2	Major proof gap	Even after repairing the ordering, the claimed numerical uncertainty envelope is not a demonstrated forward-error bound: it is dimension-free, uses a maximum column residual as though it were the required operator bound, and omits some score/cutoff arithmetic errors.	Requires a proof or a stronger fail-closed implementation.
F3	Noncritical runtime hardening	physical_limit() protects observation JLA, but match and target passes still temporarily allocate a sign vector of length sum(frequency).	Does not alter accepted estimates, but contradicts the narrow memory claim and can produce an avoidable runtime failure.
F4	No defect	Exact and JLA final subtraction gates correctly withhold NONFINITE_CORRECTED_TARGET before any point-result matrix is posted.	API12 repair is effective.
4. Critical finding F1: the public exact path is not canonically ordered
4.1 Cross-layer defect

The JLA branch deliberately orders rows by outcome and per-copy target mass, checks whether tied rows are exchangeable, and avoids using raw control coordinates or encoded IDs to index the conceptual sign stream. That is the required invariant precondition for its control anchor. 

Pasted text

The exact branch does something materially different:

stata
sort id_worker id_firm deletion_id controlvars depvar frequency target

for match deletion, and

stata
sort id_worker id_firm controlvars depvar frequency target

for observation deletion. Thus both raw control coordinates and encoded identifiers determine the physical row order reaching Mata. 

Pasted text

The exact backend then invokes kssbc__canonical_controls() on that reordered matrix before forming the full design. 

Pasted text

Inside kssbc__canonical_controls():

Every row whose residual score exceeds maximum − margin × max(1,maximum) is eligible.

Only rows near the cutoff, not rows tied or nearly tied with the maximum, are withheld.

The selected anchor is eligible[1], the first eligible row in the current order.

The next residualized scores and all later anchors depend on that choice.

The function’s own comment only invokes the ado layer’s invariant order for JLA rows; it does not establish the corresponding precondition for exact mode. 

Pasted text

Consequently, the local whitening residuals and cutoff gates cannot certify a common anchor when the two calls reach the function in different physical orders. The documentation nevertheless claims that accepted Z and ZT calculations supply the same anchor sequence and canonical controls. 

Pasted text +1

4.2 Eight-row accepted-path counterexample

A compact finite witness uses two workers, two firms, and two observations in every worker–firm cell. Let the worker sign w, firm sign f, and within-cell copy sign t each take values in {−1,+1}, giving all eight triples (w,f,t). Define

s=
8
	​

1
	​

,d=0.002,a=
1−d
2
	​

,

and controls

z
1
	​

=s(aw+dtf),z
2
	​

=s(af−dtw).

Use unit frequencies, default target weights, observation deletion, and, in lexicographic row order,

y
r
	​

=sin{1.7(r−1)}+0.1(r−1),r=1,…,8.

These controls have three useful exact properties:

Z
′
Z=I
2
	​

,z
1r
2
	​

+z
2r
2
	​

=
4
1
	​

for every row,

and the d-scaled within-cell components make both controls independent of the worker–firm FE span and leave every physical-observation deletion identified.

Now apply

T=(
1
1
	​

−3
1
	​

),detT=4,

so that

u
1
	​

=z
1
	​

+z
2
	​

,u
2
	​

=−3z
1
	​

+z
2
	​

.

At the default rank_tolerance(1e-10), the anchor margin is 10
−7
. Every first-pivot score is 0.25, while the cutoff is 0.2499999. Thus every row is eligible and every row is roughly 10
−7
, not 10
−12
, from the cutoff. Neither basis is ambiguous under the API12 boundary gate.

Nevertheless:

In the first worker–firm cell, sorting by (z1,z2) places the t=−1 row first.

Sorting by (u1,u2) places the t=+1 row first.

eligible[1] therefore selects different physical first anchors.

The corresponding second anchors also differ.

After restoring physical row order, the two accepted canonical-control matrices differ by approximately 2d=0.004 in their entries.

This is not a tiny uncertainty-bound dispute. The two runs select different exact physical anchor rows because the public exact caller changed their order.

A line-for-line IEEE-double reconstruction of kssbc__inverse(), kssbc__canonical_controls(), the exact target matrices, and the exact observation correction—validated first against the packet’s 24-row exact fixture—accepts both versions. For nuisance(joint) it gives information reciprocal condition numbers near 5.000005×10
−7
 and corrected rows differing by a maximum relative amount of approximately 7.55×10
−10
. The canonical-basis discrepancy itself is algebraic and does not depend on reproducing a particular BLAS accumulation.

Because the default auto rule selects exact whenever the five-parameter design is below exact_limit(500), the same defect reaches algorithm(auto). Canonicalization also precedes the preliminary full joint fit, so both nuisance(joint) and nuisance(fixedoffset) inherit the anchor failure. Auto dispatch is based solely on the declared parameter count. 

Pasted text

4.3 Why the supplied API12 witnesses do not close this hole

The four-row exact-cutoff fixture is handled correctly: both bases place a score at the fuzzy eligibility boundary, and both calls are withheld as AMBIGUOUS_CONTROL_BASIS. 

Pasted text

The 14-row determinant-one witness is also handled conservatively:

the base basis is accepted;

the transformed basis is withheld;

the result is repeated across exact, JLA, auto-to-JLA, both nuisance modes, both batch sizes, probes(2), seed 2, and tolerance(1e-4).

That is valid one-sided withholding, not basis-dependent acceptance. 

Pasted text

But the fixture never exercises the missing case: both bases are far from the cutoff, both pass, and the raw-control sort changes which eligible physical row is first. Passing the registered witness therefore does not imply the required universal statement.

5. Anchor-certificate analysis
5.1 Exact-arithmetic result under a common row order

Conditional on a fixed physical row order, the intended algebra is correct.

Let

Q=ZL,Q
′
WQ=I,

and let Z
T
	​

=ZT for invertible T. Any weighted-orthonormal basis Q
T
	​

 for the same span satisfies

Q
T
	​

=QR

for an orthogonal matrix R. Therefore:

row inner products and first-pivot row scores are unchanged;

after selecting the same anchor rows, the residualized scores are unchanged;

if A is the selected anchor matrix, then

Q
T
	​

A
T
′
	​

(A
T
	​

A
T
′
	​

)
−1
=QA
′
(AA
′
)
−1
.

So the proposed anchor construction is coordinate-invariant if the same ordered physical rows are supplied and the same anchor decisions are certified.

F1 breaks precisely that antecedent on exact paths.

5.2 The floating-point envelope is not yet a proved certificate

The routine defines

numerical_error=max{whitening_error,
max(rcond,rank_tolerance)
inverse_relres
	​

},

then uses

uncertainty=max{10
−12
,100numerical_error}.

It rejects uncertainty >= margin/4 and screens scores within one uncertainty of the computed cutoff. 

Pasted text

That is a plausible hardening heuristic, but the packet does not establish it as a uniform forward-error bound:

kssbc__max_column_relres() reports the largest Euclidean norm of an individual residual column. If R is the inverse residual for k controls, the operator norm can be as large as

∥R∥
2
	​

≤∥R∥
F
	​

≤
k
	​

j
max
	​

∥R
⋅j
	​

∥
2
	​

.

The code does not include this dimension factor. 

Pasted text

JLA has no public control-count cap analogous to the exact backend’s total exact_limit(). A fixed factor of 100 therefore cannot be treated as a uniform dimension-independent theorem.

The later-pivot update

mata
residualized = orthonormal -
    orthonormal*anchor'*anchor_inverse.inverse*anchor

adds matrix-product and cancellation errors that are not separately measured.

The eligibility comparison involves errors in both the row score and the computed maximum-derived cutoff. The code compares their difference with a single copy of uncertainty; no derivation in the packet shows that this contains both errors.

I did not obtain a second fixed-order, both-accepted counterexample in my adversarial rotation/shear/scaling search. The API12 envelope is quite conservative on the supplied two-control witnesses. Nevertheless, after F1 is repaired, KB5 still needs either a dimension-aware perturbation proof or a stronger directly checked certificate before the documentation can assert the universal accepted-path implication.

6. Accepted public-path reconstruction
6.1 Preprocessing and runtime authentication

The ado layer:

validates the algorithm, deletion, nuisance and stayer conventions;

enforces probes() ≥ 2 and tolerance() ∈ [10^{-15},10^{-4}];

validates positive integer frequency weights and nonnegative finite target weights with positive mass;

defaults stored-row target mass to frequency;

expands factor variables and removes only terms carrying Stata’s explicit r(omit) metadata;

leaves ordinary zero and collinear numeric columns in the requested design;

uses complete cases for observation deletion but requires the entire frozen requested input for match deletion. 

Pasted text

The semantic API-level, version and build-token checks are fail-closed. A loaded same-level but different build is rejected as STALE_MATA_RUNTIME; a newly loaded nonmatching runtime is rejected as INVALID_MATA_RUNTIME. 

Pasted text

No stale-runtime execution path remains in the supplied caller.

6.2 Graph and sample selection

The implementation keeps the design identifiers separate from the deletion partition, rejects a declared match crossing worker–firm coordinates, and ranks connected components by firm count and then physical mass. A tie on both quantities is withheld rather than resolved by encoded IDs. 

Pasted text +1

The graph service then:

selects the initial largest component;

restricts match headlines to movers;

repeatedly removes insufficient histories;

removes worker articulation vertices;

reselects the largest component after each change;

withholds every tied selection;

records retained rows, mass, edge counts, articulation removals and iterations. 

Pasted text

This is conservative, but no silent component, deletion-unit or target-population substitution appears.

6.3 Dense exact backend

After canonicalization, exact mode constructs the all-worker/all-but-one-firm/control design, forms weighted information, equilibrates and factors it, and rejects singular or residual-failing inverses. Under fixed offset it first performs the full joint fit, subtracts the fitted nuisance index and separately factors the pure two-way working system. 

Pasted text

For observation deletion:

x_r'Ax_r is the leverage of one physical copy;

a stored row with frequency f
r
	​

 contributes f
r
	​

 identical physical-copy corrections;

a near-boundary deletion directly factors H−x
r
	​

x
r
′
	​

, correctly subtracting one physical copy rather than the entire stored row. 

Pasted text

For match deletion:

rows are transformed by 
f
	​

;

the full block projection X
g
	​

H
−1
X
g
′
	​

 is formed;

a near-boundary block directly factors H−X
g
′
	​

X
g
	​

;

the deleted residual is obtained from (I−P
g
	​

)
−1
e
g
	​

;

each target uses X
g
	​

H
−1
QH
−1
X
g
′
	​

. 

Pasted text

Those formulas match the governing exact block KSS identity. The critical defect is not the exact correction algebra; it is the noninvariant row order used before canonicalization and dense accumulation.

6.4 Matrix-free JLA backend

The pure two-way solver reconstructs the omitted full-firm right-hand side, eliminates workers exactly, projects the firm system onto the zero-sum quotient, runs diagonally preconditioned CG without privileging a base firm, grounds the public last-firm representation only after convergence, reconstructs workers, and recomputes every worker and every firm equation. Zero reduced right-hand sides return the exact zero quotient solution. 

Pasted text

Batched solving is a column-by-column call to the same scalar solver, with the maximum iteration count and maximum per-column residual retained. Thus a large neighboring right-hand side cannot mask a failed column. 

Pasted text

Joint controls use the exact low-dimensional FWL Schur complement:

S
Z
	​

=Z
′
WZ−(A
′
WZ)
′
H
A
−1
	​

(A
′
WZ),

with an independently recomputed residualized-control cross-product and a per-column full joint-system residual gate. 

Pasted text

Every controlled JLA path, including fixed offset, then applies the probe-independent within-cell certificate. It uses two-pass centering, nonnegative deletion-scatter losses, direct deleted-scatter eigenvalues and inverse residuals, and subtracts both measured whitening error and the registered rounding threshold from its reported gap. 

Pasted text

No base-firm grounding defect, aggregate-batch residual defect, or fixed-offset rank-bypass remains in the inspected code. As the numerical architecture itself acknowledges, a residual gate does not prove a uniform forward-error bound for arbitrarily ill-conditioned quotient systems; within the stated contract it is an acceptance residual, not an econometric theorem. 

Pasted text

6.5 JLA leverage and deletion adjustment

The first pass uses literal physical Rademacher signs. Observation deletion retains the two copy-specific correlation sums required to reconstruct each copy’s residual second moment, residual fourth moment and mixed fourth moment. 

Pasted text

The production code then:

constrains P and M by their common positive sum;

uses coefficient one on the mixed fourth moment;

computes

V=
R
M
2
m(P
2
)+P
2
m(M
2
)−2PMm(P,M)
	​

,

and

B=
R
Mm(P
2
)−Pm(M
2
)+(M−P)m(P,M)
	​

;

forms the observation reciprocal with +B/m
2
−V/m
3
;

performs those nonlinear operations separately for every physical copy before averaging final multipliers within a stored row. 

Pasted text

For match deletion, the FE contribution is rank one in the frequency direction, the exact residualized-control block is added without replacing controls by a match mean, and the adjusted inverse residual carries the required +B and −V signs. 

Pasted text

These signs agree with differentiating

M(h)
−1
e,M(h)=I−C−hvv
′
,

when B is the bias of the estimated residual share rather than the projection share. 

Pasted text

6.6 Target contractions and accounting

The dense target matrices normalize the complete retained target mass and assign zero target action to controls. Worker and firm blocks are centered marginal shares; the cross block is one half of the centered worker–firm cell share. 

Pasted text

The JLA target direction

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


is the stored-row sum of the centered literal-copy target direction. 

Pasted text

Worker and firm target inverse actions use common directions; total is their sum; covariance is formed probe-by-probe as

2
total−worker−firm
	​

.

Thus plug-in, correction and corrected totals satisfy the accounting identity up to the final floating operations. 

Pasted text

7. Final corrected-target overflow repair

Take the finite rows

P=(8×10
307
,1,2,3),B=(−8×10
307
,0,0,0).

Both rows contain finite doubles, but

P−B=(1.6×10
308
,1,2,3)

overflows in its first component. The packet’s Mata arithmetic fixture confirms that each input row passes hasmissing() and their difference fails it. 

Pasted text

Exact completion path

Exact mode first checks the finite plug-in and correction rows, forms corrected = plugin-correction, and returns NONFINITE_CORRECTED_TARGET before setting out.status="CONVERGED" or assigning the output result rows. 

Pasted text

JLA completion path

JLA performs the same separate final subtraction gate before setting the successful status or copying any point result into the output structure. 

Pasted text

Public posting path

The ado caller checks mata_status first. Any status other than CONVERGED invokes _kss_bc_post_failure, which clears e() and posts only e(status)="WITHHELD" and the typed withholding status. ereturn post and all point-result matrices are reached only after the successful-status branch. 

Pasted text +1

Conclusion: the API12 final-subtraction repair is valid on both exact and JLA paths. No missing, infinite or partial corrected target is posted.

8. Required attack replay and audit matrix

I extracted the authenticated Python sources and ran the 29 supplied dense-oracle, literal-frequency, JLA-formula, block-control and Monte Carlo algebra tests. All 29 passed. I nevertheless based the conclusions below on production control flow and independent derivation rather than on the test assertions.

Attack or obligation	Independent conclusion
Four-row cutoff fixture	Both bases are correctly withheld at the cutoff. This is valid conservative failure.
Native 14-row determinant-one witness	Base basis proceeds and transformed basis is withheld before algorithm-, seed-, batch- or nuisance-specific work. This is valid conservative withholding, not accepted-path disagreement.
New eight-row both-accepted witness	Fails: exact and auto-to-exact can choose different physical anchors because raw controls determine exact row order.
Six-row K(2,3) relabeling	Full-firm quotient construction, post-convergence grounding and full residual reconstruction remove the previous omitted-base defect. The scalar and batched paths share the same solver. 

Pasted text


Determinant-four JLA map	On the registered JLA fixture, conceptual-copy order is unchanged and the same canonical controls reach adaptive solves. Both nuisance conventions are covered. Exact mode was not covered by that registered transformation and is where F1 occurs. 

Pasted text


Positive zero / negative zero	Both are ordinary numeric zero columns; neither carries omitted-factor metadata. The raw Gram or full information rank gate withholds them.
Zero plus valid control	The zero coordinate remains in the requested design, making the requested control matrix singular. Exact returns SINGULAR_INFORMATION; JLA returns SINGULAR_NUISANCE_BLOCK. No silent column deletion occurs. 

Pasted text


Six-cycle FE-collinear control	The preliminary full joint fit is required under both nuisance conventions. Exact full information or JLA Schur/rank gates withhold; fixed offset does not bypass identification. 

Pasted text


Frequency-two and literal copies	Exact observation deletion removes one copy; JLA generates physical signs, retains copywise nonlinear statistics and agrees algebraically with explicit expansion. The minimal six-copy stream exercises probes(2). 

Pasted text


Match-frequency contraction	The stored sufficient statistics are exactly the literal-copy block projection and residual contractions. No pooled-copy nonlinear ratio remains.
Automatic dispatch	Dispatch is solely by the complete parameter count and registered exact_limit(). It neither retries with another estimator nor relaxes tuning. Auto-to-JLA inherits the repaired JLA paths; auto-to-exact inherits F1.
Public tolerance	The ado layer rejects every tolerance()>1e-4; the former near-unit tolerance attack has no public path. 

Pasted text


Stale runtime	API level, version and semantic build token are all checked before graph/backend execution.
Final subtraction overflow	Correctly withheld before any point result is posted, as established above.
Physical-copy limit	Observation JLA is checked before allocating retained copy-specific state. The gate is correctly typed as PHYSICAL_COPY_LIMIT. 

Pasted text

9. Parameter metadata, return contract and caller coverage

The successful caller posts:

e(b) and e(kss) from the corrected row;

separate plug-in, correction and numerical-MCSE matrices;

the four-row e(results) matrix;

retained stored and physical counts;

full and correction parameter counts;

deletion units and target mass;

conditioning, inverse and solver diagnostics;

graph, timing, algorithm, deletion, nuisance, target-population and weight-convention metadata;

no e(V). 

Pasted text

full_parameters and correction_parameters are populated consistently:

joint exact/JLA: both include controls;

fixed-offset exact/JLA: the full count includes controls and the correction count is the pure two-way working dimension;

parameters aliases the correction dimension.

The audit covered these public route classes:

Backend	Deletion	Nuisance	Controls	Result
Exact	Observation	Joint	none	No critical defect found
Exact	Observation	Joint	present	F1 applies
Exact	Observation	Fixed offset	present	F1 applies
Exact	Match	Joint	none	No critical defect found
Exact	Match	Joint/fixed offset	present	F1 applies
JLA	Observation	Joint/fixed offset	none/present	No additional accepted-path defect established
JLA	Match	Joint/fixed offset	none/present	No additional accepted-path defect established
Auto	Dispatches to JLA	all supported	all supported	Inherits JLA result
Auto	Dispatches to exact	controls present	both nuisance modes	Inherits F1
10. Required repairs before KB5 closure
Repair 1 — mandatory: use one invariant canonical row order on every backend

Before any call to kssbc__canonical_controls() or any basis-sensitive dense reduction:

Construct a physical-copy order from control-basis- and ID-invariant semantics.

Apply it to exact as well as JLA.

Do not subsequently reorder dense cross-products by raw controls.

For tied invariant keys, either:

prove that the rows are fully exchangeable for every relevant dense accumulation, or

withhold under a typed ambiguity status.

The existing JLA outcome/per-copy-target order and exchangeability screen is the natural starting point. Exact mode does not require a random sign order, but it still requires a common physical order if the same row-anchor canonicalization and deterministic floating reductions are used.

Repair 2 — mandatory: register the both-accepted eight-row witness

Add native Stata fixtures for:

algorithm(exact);

default auto-to-exact;

nuisance(joint) and nuisance(fixedoffset);

observation and, after adapting deletion IDs, match deletion;

the determinant-four transform above;

simultaneous worker, firm and deletion-ID relabeling.

The test must require either:

mreldif(e(results)
Z
	​

,e(results)
ZT
	​

)

below the registered comparison tolerance, or the same typed conservative withholding on at least one basis. A success/success result with different physical anchors is unauthorized.

Repair 3 — mandatory for the documented certificate: make the error bound dimension-aware

At minimum:

replace the maximum-column inverse residual proxy by a bound in a compatible matrix norm, including the required control-dimension factor;

include the denominator term required by the inverse perturbation inequality;

account for rounding in the residualized-score product and subtraction;

include uncertainty in both the row score and the maximum-derived cutoff;

impose a certified control-count range if the bound is only proved up to a finite dimension.

Absent that work, the documentation should not call the current 100 × proxy rule a sufficient forward-error certificate.

Repair 4 — recommended runtime hardening

kssbc__physical_rademacher_sum() constructs a temporary vector of length sum(frequency) on every call. Match JLA and all target passes use this helper, although physical_limit() is checked only for observation JLA. 

Pasted text

Either:

stream the conceptual sign sequence in bounded chunks while preserving regrouping invariance, or

apply a typed allocation limit to every JLA path that materializes such a vector.

This is not the critical estimator objection, but it should be reconciled with the stated memory boundary.

11. Uncertainty and scope

A native Stata 18/19 runtime was not present in the execution environment. I therefore did not treat the packet’s .do assertions as independently executed native evidence. I instead:

authenticated all bodies;

executed the 29 available source-bound Python algebra/oracle tests;

reconstructed the canonical and exact production routines independently in double precision;

verified that the reconstruction reproduces the supplied exact fixture;

derived F1 directly from the public sort keys and the first-eligible-row selection rule.

The critical conclusion does not depend on reproducing a particular native random stream or native JLA output: the public exact caller supplies different physical row orders, and the accepted canonical bases in the constructed witness are algebraically different.

This audit makes no claim about econometric inference, exact finite-R unbiasedness, sampling assumptions, application-specific dependence, public licensing, package unification or unrestricted-data operation. Those limitations are expressly outside the governing closure question. 

Pasted text

Final closure decision

KB5-FINAL-PRODUCTION-CONTRACT-M is not satisfied.

Critical objection remaining: the exact and auto-to-exact controlled paths can accept two invertibly equivalent control inputs while selecting different physical canonical anchors. The API12 cutoff and whitening repairs do not detect this because exact preprocessing changes row order before the certificate is applied.

Required status: false; KB5 remains open pending Repair 1, the native eight-row regression fixture, and a defensible fixed-order error certificate.

