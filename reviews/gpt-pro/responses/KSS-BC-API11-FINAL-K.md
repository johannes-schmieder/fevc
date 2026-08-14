---
review_id: KSS-BC-API11-FINAL-K
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: c1d2728068e5703710091fa467289d6c92e6f5535e89875ba59c865296bed675
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f29f6-7028-83ea-bd80-26b0f47dbd42
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

False. A critical objection remains, and KB5 should not close.

The API-11 repair to the full-firm quotient solve is sound: the six-row K(2,3) grounding attack no longer produces encoded-firm dependence, for scalar or batched right-hand sides. The exact estimator algebra, explicit-control rank handling, literal-frequency semantics, block-control adjustment, target accounting, and posted metadata also survive the counterexamples I reconstructed.

The remaining critical defect is the purported canonical control-span basis. The code can return CONVERGED for both Z and an invertibly transformed ZT, yet select different anchor sets and construct materially different numerical bases. Because JLA then performs adaptively stopped FE inverse actions on those different columns, the public plug-in, correction, and corrected outputs can differ under the same conceptual probe stream. That violates the governing requirement that accepted Z and ZT runs agree within the registered tolerance; conservative withholding was permitted, false acceptance was not. 

Pasted text

1. Source authentication

I reconstructed the packet bodies and hashed their exact transported bytes, including their terminal newlines.

32 of 32 manifest-listed bodies match their declared SHA-256 hashes.

The twelve package, documentation, and runtime bodies match the entries at packet lines 47–58.

The seven Python bodies match lines 59–65.

The nine Stata test bodies match lines 66–74.

varcomp_hdfe_specification.md, PROMPT.md, REQUEST.yaml, and SOURCE_MAP.md match lines 75–78.

All 29 path/hash rows in SOURCE_MAP.md agree with the manifest. 

Pasted text +1

The outer ZIP bytes were not supplied, so I could not independently recompute the stated outer ZIP hash c1d272…bed675. The packet expressly classifies that as a transport limitation rather than an estimator finding. 

Pasted text

The supplied package-layout test refers to kss_bc.pkg, stata.toc, and SCC harness files whose bodies are not included in the manifest-bound packet. I therefore did not authenticate those unsupplied files or treat claims about them as source-bound evidence. The omission does not affect the estimator counterexample below. 

Pasted text +1

2. Assumptions and audit boundary

I limited the audit to fixed-design point estimation, deterministic preprocessing, numerical acceptance, and the finite randomized projection algorithm. I made no inference, finite-R unbiasedness, asymptotic, application-validity, licensing, or public-release claim. I treated conservative typed withholding as valid behavior, but not basis-dependent acceptance or silent model alteration, consistent with the packet’s governing assumptions. 

Pasted text

I did not use any earlier review or chat. The environment did not contain Stata/Mata, so native Stata seeds could not be replayed. I independently translated the supplied arithmetic line by line into double-precision numerical code, inspected the production control flow, and used the tests only as locations of registered attacks—not as proof.

3. Production-path reconstruction and caller coverage
3.1 Public preprocessing and dispatch

The ado caller:

validates the deletion, algorithm, nuisance, stayer, frequency, target-weight, and numerical options;

caps tolerance() at 10
−4
;

forms a complete frozen sample;

expands factor variables;

removes a control only when _ms_parse_parts reports Stata omitted metadata;

leaves ordinary zero and collinear numeric columns in the design;

checks the exact API level, version, and semantic Mata build token;

selects and prunes the worker–firm graph;

re-encodes the retained quotient;

dispatches auto according to the complete requested parameter count; and

for observation JLA, applies physical_limit() before physical-copy allocation. 

Pasted text +3

For fixed-seed JLA, the caller orders rows by outcome and per-copy target mass, checks that ties are exchangeable, and refuses to use encoded identifiers or raw control coordinates to resolve a nonexchangeable tie. 

Pasted text

3.2 Graph selection

The graph implementation separately represents worker coordinates, firm coordinates, and deletion IDs. It:

rejects cross-coordinate match IDs;

ranks components by firm count and then physical mass;

withholds tied winners;

imposes the mover restriction in match mode;

repeatedly removes insufficient workers and worker articulation vertices; and

reselects the largest component after each pruning step. 

Pasted text +1

I found no encoded-ID tie-break or partial match-block retention in this path.

3.3 Exact paths

The exact backend covers all four combinations of observation/match deletion and joint/fixed-offset nuisance handling.

It canonicalizes any controls, builds all worker indicators, all but one firm indicator, and the controls, checks the equilibrated full information matrix, and performs the preliminary full fit. Fixed-offset then subtracts the fitted nuisance index and constructs a separate pure-FE working system. 

Pasted text

The exact correction then uses:

per-physical-copy leverage and one-copy deleted information for observation deletion; and

the frequency-compressed block projection, residual-maker inverse, and block target contraction for match deletion.

Near a numerical rank boundary it directly factors the deleted information matrix. 

Pasted text

No generalized inverse, ridge, repeated outcome refit, changed deletion unit, or partial target was found.

3.4 JLA paths

The matrix-free path:

prepares the pure two-way FE service;

canonicalizes the requested control span;

forms the low-dimensional joint-control Schur system;

runs the probe-independent full-fit/deletion-rank certificate whenever controls are present;

performs the full joint fit;

optionally constructs the fixed-offset pure-FE working fit;

computes the plug-in components;

runs the literal-copy leverage sketch;

forms copywise or matchwise finite-projection deletion multipliers;

runs the independent target sketch; and

posts all four targets and diagnostics. 

Pasted text +2

The only falsified stage is step 2, the canonical control transformation. It contaminates the subsequent joint JLA fit and inverse actions under both nuisance conventions.

4. Critical finding: the control basis is not canonical on all accepted inputs
4.1 Governing obligation

The contract requires an invertible Z↦ZT change to preserve accepted outputs. It specifically represents the intended construction as whitening the span, selecting anchors from invariant row inner products, and mapping those anchor rows to the identity. If that construction cannot be certified at the numerical margin, the command may withhold. 

Pasted text

The numerical documentation makes the stronger operational claim that the two parameterizations therefore feed the same right-hand sides to adaptively stopped PCG. 

Pasted text

4.2 What is valid in exact arithmetic

Let

G=Z
′
WZ,O=ZL,LL
′
=G
−1
.

Then O
′
WO=I. For Z
∗
	​

=ZT, any exact weighted whitening gives

O
∗
	​

=OR

for an orthogonal R. Consequently, all row inner products and all residual pivot scores are invariant.

If the same ordered anchor rows A are selected, the returned basis is

C=OA
′
(AA
′
)
−1
.

For O
∗
	​

=OR, the anchor matrix becomes A
∗
	​

=AR, and direct substitution gives C
∗
	​

=C. Thus the underlying exact-arithmetic argument is correct conditional on selection of the same anchor sequence.

4.3 The missing numerical argument

The implementation accepts a whitening error as large as

margin=max(10
−10
,1000rank_tolerance),

which is 10
−7
 at the default rank tolerance. It then treats every row whose score is within that same margin of the computed maximum as eligible and chooses the first eligible row. It does not certify that the eligibility decision is stable under the admitted whitening error or under an invertible input reparameterization. 

Pasted text

The critical lines are:

score = rowsum(residualized:^2)
maximum = max(score)
eligible = selectindex(score :>= maximum-margin*max((1,maximum)))
chosen = eligible[1]

The final gate verifies only that the basis maps its own selected rows approximately to the identity. It does not establish that another representation of the same span selects the same rows. 

Pasted text

Near an eligibility boundary, coordinate-dependent rounding in the inverse and Cholesky operations can therefore move one row from just inside to just outside the band. Both calculations can pass every whitening, inverse, and anchor residual gate.

4.4 Finite counterexample

A compact control-span witness is the following eight-row, two-control matrix:

Z =
[ 0.7037651560862372   -0.01213100064390886 ]
[-0.2552775470643382    0.6559466556462534  ]
[-0.1128966060152190    0.28729193595054625 ]
[-0.26212880006292877  -0.15584457130166096 ]
[-0.08287286955710015  -0.1733117487431483  ]
[-0.06938638543245550  -0.0772968683597004  ]
[ 0.08296261314678174  -0.5616086968926074  ]
[-0.5826882952311562   -0.33368629034981906 ]

Use the invertible map

T = [8000   1       ]
    [   0   0.000125]

with detT=1. At the default rank_tolerance(1e-10):

the Z calculation selects anchor rows 1 and 2;

the ZT calculation selects anchor rows 2 and 8;

the reported canonical-basis residuals are 2.22×10
−16
 and 5.53×10
−8
, both below the 10
−7
 gate.

At the first pivot, rows 1 and 2 have computed scores

(0.49543255609770803, 0.49543264108869006)

under Z. Row 1 is inside the eligibility band by 1.50×10
−8
. Under ZT, the scores become

(0.49543254539326590, 0.49543266155903387),

placing row 1 outside the same band by 1.62×10
−8
. This is not a one-ulp knife edge.

Public-path embedding

Embed those eight rows in a complete K(2,5) worker–firm graph as follows:

Stored rows	Worker–firm cell	Frequency	Controls
1–2	(1,1)	1 each	Z
1
	​

,Z
2
	​


3–4	(1,2)	1 each	Z
3
	​

,Z
4
	​


5–6	(2,1)	1 each	Z
5
	​

,Z
6
	​


7–8	(2,2)	1 each	Z
7
	​

,Z
8
	​


9	(1,3)	8	zero
10	(1,4)	8	zero
11	(1,5)	11	zero
12	(2,3)	14	zero
13	(2,4)	2	zero
14	(2,5)	3	zero

Set

y
r
	​

=10(r−1)+sin(r−1),

use default target mass a
r
	​

=f
r
	​

, observation deletion, probes(2), and tolerance(1e-4). Outcomes are distinct and every physical copy has the same unit target mass, so the JLA order is unambiguous. The complete K(2,5) graph survives the graph and physical-history gates.

In my independent line-for-line execution, both parameterizations passed:

Diagnostic	Z	ZT
Canonical-basis residual	2.22×10
−16
	5.53×10
−8

deletion_rank_gap	0.1464878908	0.1464878908
Maximum recorded solver/preparation residual	4.66×10
−5
	1.06×10
−5

Public solver gate at tolerance(1e-4)	10
−3
	10
−3

The same admissible two-probe physical-copy sign stream produced CONVERGED on both sides.

For nuisance(joint), the ZT−Z differences were:

plugin:
[-9.6055945e-05, -1.0700284e-03,  8.7103121e-05, -9.9187807e-04]


corrected:
[-1.5461541e-04, -1.0257207e-03,  1.0316229e-04, -9.7401152e-04]

The largest corrected-output discrepancy is 1.026×10
−3
.

For nuisance(fixedoffset), the corrected-output difference was:

[ 5.8787843e-05, -1.7321420e-05, -1.4460472e-05, 1.2545478e-05]

The registered determinant-four test requires relative agreement below 2×10
−10
, including at probes(2) and the maximum allowed tolerance. 

Pasted text

A dense exact calculation on the same two spans agreed within 4×10
−12
. Thus the discrepancy is not a changed estimand; it is generated by the different canonical columns and adaptively stopped inverse actions.

The JLA code constructs and accepts the canonical basis before the joint Schur preparation and computes the public plug-in row before drawing any probes. Therefore the plug-in discrepancy itself is seed-independent once the run reaches this stage. 

Pasted text

 The plug-in, correction, corrected, and MCSE rows are all publicly posted in e(results). 

Pasted text

Finding K-1 — Critical estimator defect

The whitening-plus-anchor routine does not map all accepted Z and ZT inputs to the same numerical control basis. Accepted controlled JLA output can consequently depend on the user’s invertible control coordinates.

This is a false-acceptance defect, not conservative withholding or merely an omitted proof.

5. Firm quotient and K(2,3) grounding attack

The quotient repair passes.

The FE solver lifts the omitted firm equation from the worker and displayed-firm right-hand sides, forms the full-firm reduced RHS, projects it to zero sum, and keeps all PCG iterates and preconditioned residuals in that quotient. The displayed last-firm grounding is imposed only after convergence. It then reconstructs worker coefficients and checks all worker and all firm normal equations. 

Pasted text

For a firm permutation matrix Π,

S
F
	​

↦ΠS
F
	​

Π
′
,r↦Πr,diag(S
F
	​

)↦Πdiag(S
F
	​

).

Mean projection, diagonal preconditioning, Schur action, dot products, and the PCG recurrence all commute with Π. Starting from zero therefore produces permuted iterates with the same scalar recurrence and stopping iteration. Grounding after the solve changes only the coefficient representative; the accompanying worker reconstruction preserves fitted values.

The batched function does not implement a coordinate-coupled block solver. It invokes the scalar quotient solve separately for each column and takes the maximum iteration count and residual. 

Pasted text

Independent replay

I used the registered six-row outcome/design and checked:

all six firm permutations, rather than only the three displayed base-firm cases;

both worker permutations;

the complete two-probe JLA calculation; and

six simultaneous arbitrary RHS columns.

Results:

maximum corrected-output difference across the 12 worker/firm relabelings:

4.97×10
−14
;

maximum fitted-value difference across all firm permutations in the batched RHS test:

1.78×10
−15
.

The packet’s registered test covers three relabelings that make each physical firm the displayed base and also changes the batch size. 

Pasted text

Finding K-2 — Passed

The full-firm zero-sum quotient PCG and post-solve grounding remove encoded-firm dependence for scalar and batched solves. No critical objection remains on the quotient repair.

6. Determinant-four registered control map

The registered transformation is

(c
1
	​

,c
2
	​

)↦(c
1
	​

−3c
2
	​

, c
1
	​

+c
2
	​

),

whose determinant is four. On the supplied 24-row fixture, my reconstruction selected the same two anchors for the original and transformed controls. The maximum elementwise difference between the two canonical bases was 6.42×10
−18
.

Consequently, the displayed determinant-four attack passes at:

probes(2);

tolerance(1e-4);

nuisance(joint); and

nuisance(fixedoffset).

The test is valid but not exhaustive. Its pivot scores are sufficiently separated that it never exercises the unprotected eligibility boundary identified above. 

Pasted text

7. Explicit-control rank and preprocessing attacks
7.1 Actual preprocessing removal

The caller expands and materializes every factor term, parses each term’s metadata, and adds it to the design unless Stata marks it omitted. There is no data-value test that removes a zero column or a collinearity screen that changes the requested design. 

Pasted text

The factor-variable fixture compares i.time with the explicit nonbase dummies and confirms that the actual omitted factor base is the only removed coordinate. 

Pasted text

7.2 Zero and redundant-control cases

The following attacks survive source reconstruction:

Attack	Exact path	JLA path
explicit +0 numeric control	SINGULAR_INFORMATION	SINGULAR_NUISANCE_BLOCK
explicit −0 numeric control	singular	singular
valid column plus zero column	singular	singular
frequency-two zero column	singular	singular
invertibly exposed zero coordinate in a redundant basis	singular	singular
joint nuisance	withheld	withheld
fixed-offset nuisance	withheld in preliminary full fit	withheld in preliminary full fit
observation deletion	withheld	withheld
match deletion	withheld	withheld
auto selecting exact	withheld by exact rank gate	—
auto selecting JLA	—	withheld by JLA rank gate

The registered matrix covers all backend, nuisance, deletion, and dispatch combinations. 

Pasted text

Because auto dispatch counts all retained control columns before selecting a backend, a zero column is not hidden by the dispatcher. 

Pasted text

7.3 Controls in or near the FE span

The six-cycle control exactly in the firm-FE span is rejected under exact and JLA fixed-offset paths, including observation and match deletion and the maximum allowed PCG tolerance. The full joint fit is checked before the offset is constructed. 

Pasted text

The block-only control, large cell-constant design, and ill-conditioned two-control deleted-rank examples are also withheld by the exact deleted-information gate or the JLA deterministic scatter certificate. 

Pasted text

Finding K-3 — Passed

Ordinary zero or collinear numeric controls are not silently discarded. Only actual factor terms carrying omitted metadata are removed. Full-fit and deleted-rank attacks otherwise fail closed as required.

8. Finite estimator algebra
8.1 Literal physical-copy semantics

For exact observation deletion, the implementation uses the per-copy leverage x
r
′
	​

H
−1
x
r
	​

, subtracts one unweighted x
r
	​

x
r
′
	​

 contribution in a direct deleted-information check, and multiplies the identical copy correction by f
r
	​

. Match deletion instead uses the 
f
r
	​

	​

-transformed block, which is the correct compression of deleting all physical copies in the declared match. 

Pasted text

For observation JLA, it draws one sign per physical copy, stores the two copy-specific correlations C
1
	​

 and C
3
	​

, reconstructs the five required raw moments copy by copy, forms each nonlinear inverse multiplier separately, and averages only the final multipliers back to a stored row. 

Pasted text +1

Match and target directions aggregate the same literal signs with the required frequency and target-mass scaling. 

Pasted text +1

The frequency-two, partial split, literal expansion, minimal R=2, and “stored rows equal parameters but physical copies supply residual degrees of freedom” fixtures are consistent with the implementation. 

Pasted text

Finding K-4 — Passed

No pooled-frequency substitution or weighted-row observation deletion remains. Exact and JLA paths implement literal physical copies.

8.2 Coefficient-one fourth moment

The delta calculation gives

B=
R
1
	​

[Mm(P
2
)−Pm(M
2
)+(M−P)m(P,M)],

with coefficient one—not two—on the mixed raw fourth moment. 

Pasted text

The production code implements precisely that term, after constraining P and M to sum to one. 

Pasted text

Finding K-5 — Passed

The coefficient-one fourth-moment formula is algebraically correct.

8.3 Constrained ratios and +B/−V signs

The code forms

P
ˉ
=
P
/(
P
+
M
),
M
ˉ
=
M
/(
P
+
M
),

requires a positive denominator, and calculates the registered finite-projection B and V.

For observation deletion, the inverse multiplier is

m
full
	​

1
	​

+
m
full
2
	​

B
	​

−
m
full
3
	​

V
	​

.

For match deletion, differentiating the block inverse gives the same plus B, minus V pattern. 

Pasted text

The production observation and match formulas have those signs. 

Pasted text

Finding K-6 — Passed

The constrained P/M ratios and the +B, −V adjustments are correct.

8.4 Block controls

The exact FWL decomposition is

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
,

and within a match the FE part is h
g
	​

v
g
	​

v
g
′
	​

 while the residualized-control part remains the full matrix C
g
	​

. The implementation does not replace varying within-match controls by a mean row. 

Pasted text

Production JLA constructs

P
g
	​

=
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

+U
g
	​

S
−1
U
g
′
	​


and inverts the complete block residual maker before applying the delta adjustment. 

Pasted text

Finding K-7 — Passed

The noncommuting block-control projection is retained correctly.

8.5 Targets, normalization, and accounting

The exact target matrices use normalized retained target mass, give controls zero target action, and correctly represent worker variance, firm variance, and covariance in the grounded coefficient coordinates. 

Pasted text

The JLA target direction has covariance

diag(s)−ss
′
,

with an explicit stored-row target mass divided among its physical copies. 

Pasted text

The target pass uses common worker, firm, and total directions and defines covariance probe by probe as

(total−worker−firm)/2.

Thus both plug-in and correction totals satisfy the accounting identity, and corrected totals inherit it. 

Pasted text

Finding K-8 — Passed

Target normalization and total accounting are correct.

9. Posting, metadata, failures, and resource boundaries

The ado layer names the four target columns, posts plugin, bias_correction, corrected, and numerical_mcse, stores the corrected row in e(b) and e(kss), and posts no e(V). It separately records full and correction parameter counts, so fixed-offset excludes already fitted controls only from the correction-system count. 

Pasted text

The documented failure catalog covers the rank, inverse, block, JLA, graph, stale-runtime, physical-copy, and nonfinite failure states. The code does not repair them by ridge, changed deletion unit, reduced probes, or relaxed tolerance. 

Pasted text

Observation JLA checks physical_limit() before constructing literal-copy state. 

Pasted text

Finding K-9 — Passed

Posted matrices, parameter metadata, failure labels, and the observation physical-copy allocation gate match the stated contract.

10. Classification of issues
Classification	Result
Critical estimator defect	Accepted canonical-control construction can depend on the input basis and cause basis-dependent controlled JLA outputs.
Critical quotient defect	None found; the full-firm quotient repair passes.
Conservative withholding	The within-cell trace/direct-factor certificate can reject valid designs, and fixed-offset uses a stronger deletion-rank requirement than strictly necessary. Both behaviors are explicitly authorized. 

Pasted text +1


Optional numerical hardening	After combining control and FE inverse actions, kssbc__joint_solve checks the grounded joint equations, whereas the pure FE solver explicitly reconstructs every firm equation. The omitted firm equation is algebraically dependent and I found no counterexample, but explicitly reconstructing it after the joint combination would align the implementation more literally with the documentation. 

Pasted text +1


Documentation issue	TESTING.md calls the registered suite “API10 counterexamples” even though the supplied runtime and tests are API 11. 

Pasted text


Packet-evidence limitation	Outer ZIP, package manifest/TOC, SCC scripts, and native Stata execution were unavailable.
Outside scope	Inference, exact finite-R unbiasedness, public release, unified-package completion, and application assumptions.
11. Required repair before KB5 closure

The fuzzy anchor rule cannot remain an accepting rule without a stability certificate.

Certify every pivot decision. At each pivot, bound the numerical perturbation of every invariant row score using the measured whitening/inverse error. Accept an anchor only when its selection and its membership in or exclusion from the eligibility set are separated from every boundary by more than that bound.

Fail closed on an unresolved anchor. A near-tie should return a typed status such as UNSTABLE_CONTROL_ANCHOR or the existing conservative nuisance-basis failure. Selecting the first row in a tolerance band is not sufficient.

Verify the invariant object, not only the chosen anchor residual. The present check C[A,:]≈I proves that the returned basis is valid for the selected rows; it does not prove that the selected rows are representation invariant. The new certificate must address anchor identity or prove that all admissible anchor choices produce the same numerical columns.

Add the 14-row boundary witness. Register the witness above for:

nuisance(joint) and nuisance(fixedoffset);

explicit JLA and auto forced to JLA with exact_limit(2);

probes(2);

tolerance(1e-4);

multiple batch sizes; and

both Z and ZT.

Test the canonical matrix directly. Final-output comparison on one well-separated determinant-four fixture is insufficient. Tests should assert equality of the canonical columns themselves, or typed withholding, across a family of transformations and score gaps approaching every pivot boundary.

Replay the new witness natively in Mata. Record an actual Stata seed for which both public executions accept, then preserve the fixed-seed output comparison as a closure regression.

Tightening only the PCG tolerance does not repair the defect: the two executions are being given different control columns and therefore different adaptively solved right-hand sides.

12. Uncertainty

The exact decimal witness was executed in an independent IEEE-double implementation of the supplied inverse, whitening, anchor, quotient, Schur, rank-certificate, JLA, and target formulas. I could not replay Mata’s native PRNG or BLAS implementation. The critical conclusion does not rest solely on a particular random draw:

the missing anchor-stability condition is visible directly in the production code;

both canonicalizations return accepted residuals;

the anchor decisions lie approximately 1.5×10
−8
 on opposite sides of the eligibility boundary, rather than at a one-ulp tie;

the public plug-in fit is computed before any random probes; and

exact dense results confirm that the underlying span and estimand are unchanged.

A different Mata numerical library could move the exact decimal boundary slightly, but the implementation has no gate preventing a nearby accepted input from reproducing the same crossing. Native replay is therefore required as regression evidence after repair, not as a reason to downgrade the structural finding to unresolved.

KB5 closure decision

Do not close KB5. The quotient and explicit-rank repairs are satisfactory, but the universal accepted-path control-basis invariance contract remains false until anchor selection is numerically certified or conservatively withheld.

