---
review_id: KSS-BC-API9-FINAL-H
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: b034abdeb8ce9545c5043875dd17015cf6d8872bf068da65277726bbbb45a619
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f0bc2-61f0-83ea-8294-991768adae18
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

KB5-FINAL-PRODUCTION-CONTRACT-H must remain open. A critical objection remains before KB5 closure.

API 9 has a representation-dependent “canonical” sort immediately before indexed random-projection generation. It sorts on densely encoded worker and firm identifiers, raw control coordinates, stored-row frequency, and stored-row target mass. The subsequent leverage and target signs are generated sequentially in that sorted order. Consequently, API 9 can accept and post different finite-R KSS estimates for:

a frequency-2 row versus its two literal frequency-one copies;

a pure relabeling of worker or firm identifiers; and

an invertible reparameterization of controls that changes their lexicographic ordering.

The defect is at the production boundary, not merely in a test or explanation: the sort is performed at kss_bc.ado lines 1326–1334, while the physical-copy signs are allocated in that order at kss_bc.mata lines 3437–3475 and 3620–3630. 

Pasted markdown +2

That contradicts the governing literal-expansion requirement, which says collapsed frequency calculations must reproduce explicitly expanded calculations, and the validation requirement that results be invariant to ID relabeling, permissible normalization, row ordering, and harmless sorting. 

Pasted markdown +1

 It also contradicts the implementation documentation’s stronger claim of a common canonical copy stream and draw-for-draw equality with explicit expansion. 

Pasted markdown +2

Source binding

I parsed 25 delimited bodies from the attachment. All 22 source components listed in SOURCE_MAP.md match their recorded SHA-256 values exactly, including:

kss_bc/kss_bc.ado: 2b7d366d4c67892c1d62c54086e79942f994d15799ea8079e776670e07df8cac

kss_bc/kss_bc.mata: 974e23b666bf7d3ecde5d467cd8e575ec6df10fee7597a233c79b5f109f8cadb

all listed documentation and Python/Stata tests.

The expected component hashes are recorded at lines 6492–6515. 

Pasted markdown

The attachment does not contain a MANIFEST.sha256 body or the deterministic ZIP bytes. It contains SOURCE_MAP.md as the component-hash ledger. I therefore verified every supplied component body against that embedded ledger, but I cannot independently recompute the stated outer ZIP hash b034abdeb8ce9545c5043875dd17015cf6d8872bf068da65277726bbbb45a619 from the markdown serialization alone. This is a source-package evidence limitation, separate from the critical estimator defect.

Production estimator reconstruction
1. Caller, sample, and semantics

The ado caller parses the dependent variable, controls, worker and firm IDs, deletion contract, nuisance convention, frequency and target weights, and numerical tuning. It restricts frequency weights to positive integers and treats an explicit targetweight() as total stored-row target mass; absent an explicit target, the stored row receives its frequency as target mass. 

Pasted markdown +1

It binds the caller to Mata API level 9, version 0.1.0-dev, and semantic token kss-bc-api9-coordinatewise-copy-tie-safe, withholding a normally stale same-level runtime. 

Pasted markdown

The graph stage selects a component by firm count and then physical mass, imposes the mover restriction in match mode, iteratively removes worker articulations and insufficient worker histories, and withholds tied largest-component choices. 

Pasted markdown +1

After the graph selection, the caller densely re-encodes the retained identifiers, chooses exact or JLA, and performs the representation-dependent sort that creates the defect:

stata
sort id_worker id_firm [deletion_id] controlvars depvar frequency target

It then dispatches to the exact or JLA Mata backend. 

Pasted markdown

2. Exact fit and correction

The exact backend builds the identified worker–firm–control design, forms the frequency-weighted information matrix, uses an equilibrated inverse, estimates the full model, and under fixedoffset refits the pure two-way model after subtracting the full-sample control index. 

Pasted markdown

The target matrices implement worker variance, firm variance, and the symmetrized worker–firm covariance, with controls receiving zero target action. Total variance is constructed as worker plus firm plus twice covariance. 

Pasted markdown +1

For observation deletion, the exact correction uses the per-physical-copy residual leverage 1−x
r
′
	​

H
−1
x
r
	​

, multiplies identical-copy contributions by frequency, and subtracts one unweighted copy from the information matrix when a direct deleted factorization is needed. For match deletion it uses the full frequency-transformed block maker and block Woodbury residual identity. 

Pasted markdown

3. Matrix-free JLA fit and rank gates

The matrix-free backend eliminates the worker block, solves the grounded firm Schur system, reconstructs worker coefficients, and recomputes the full-system residual. Batched solves are gated column by column. 

Pasted markdown +1

Joint controls are handled by an exact low-dimensional FWL Schur complement. The full joint solve is recomputed against the original joint operator. 

Pasted markdown +1

For JLA joint-control calculations, the deterministic rank certificate:

centers controls separately within worker–firm cells;

whitens the stable within-cell scatter;

computes nonnegative deletion scatter losses;

directly eigendecomposes and inverts every deleted whitened scatter; and

subtracts measured whitening error and a rounding margin from the reported gap.

That certificate is invoked before any point estimate is accepted. 

Pasted markdown +1

4. JLA deletion adjustment

For observation deletion, API 9 does correctly retain the two copy-specific correlations needed to reconstruct each physical copy’s residual second moment, fourth moment, and mixed fourth moment. It forms the constrained ratio and nonlinear reciprocal adjustment copy by copy, averaging only final inverse multipliers within the stored row. 

Pasted markdown +1

For match deletion, the FE block is represented by the frequency direction v
g
	​

, while the residualized-control projection remains a general, potentially noncommuting matrix. The production signs of the bias and variance adjustments agree with the inverse derivative calculation: plus for the residual-share bias term and minus for the variance term. 

Pasted markdown +1

5. Target contraction and posting

The target probe direction uses

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

,

so the association between each sequential physical sign and its per-copy target mass matters. 

Pasted markdown

The target pass generates a fresh sequential physical-copy stream, solves the worker and firm target systems, forms worker, firm, and total corrections, and defines covariance probe by probe from the accounting identity. 

Pasted markdown

The ado caller then posts corrected values in e(b) and e(kss), the four-row results matrix, conditional numerical MCSE for JLA, explicit no-inference labels, and KSS_POINT_ESTIMATES_ONLY. 

Pasted markdown

Critical counterexample 1: frequency 2 and duplicate-row splitting

This is a six-physical-copy observation-deletion example. Six is minimal for this ordering mechanism under an accepted two-worker/two-firm cycle: four worker–firm edges are needed, and one cell must contain three copies so that a frequency-two stored row can be split while crossing another target-mass class in the production sort.

Use no controls and probes(2). Let target denote total stored-row target mass.

Collapsed representation C, after the production sort
worker	firm	y	frequency	target
1	1	1	1	4
1	1	1	2	2
1	2	2	1	1
2	1	3	1	1
2	2	5	1	1

The physical target masses in cell (1,1) are 4,1,1.

Literal split representation E, after the production sort
worker	firm	y	frequency	target
1	1	1	1	1
1	1	1	1	1
1	1	1	1	4
1	2	2	1	1
2	1	3	1	1
2	2	5	1	1

These are the same six physical observations, with the same estimation information, outcomes, deletion units, and per-copy target masses. Exact KSS therefore agrees. The sort differs because C orders the frequency-one, target-four row before the frequency-two row, whereas E orders the two target-one rows before the target-four row.

Take the following admissible physical Rademacher streams, with columns denoting the two probes:

q
L
	​

=
	​

1
1
1
1
−1
−1
	​

−1
1
1
−1
−1
−1
	​

	​

,q
T
	​

=
	​

−1
1
−1
1
−1
1
	​

1
−1
−1
−1
−1
1
	​

	​

.

Direct evaluation of the complete production formulas gives the same plug-in vector in both representations:

(0.995555555556,0.338765432099,0.207407407407,1.749135802469).

The deterministic exact correction is also identical:

(−0.100740740741,−0.066172839506,0.124514991182,0.082116402116).

But the accepted JLA corrections are:

b
C
	​

=(−0.007843084896,−0.298151507392,0.095508300564,−0.114977991159),
b
E
	​

=(−0.039215424478,−0.321767468373,0.164968882792,−0.031045127267).

Hence the posted corrected totals are:

θ
T,C
JLA
	​

=1.864113793629,
θ
T,E
JLA
	​

=1.780180929736.

The difference, 0.083932863893, is not floating-point accumulation noise.

Both executions pass the relevant production gates:

information eigenvalues: approximately 0.6168,2.2835,7.0996;

every true physical-copy deletion retains full rank;

minimum constrained P+M denominator: 0.76;

minimum estimated residual share: 0.0512820513;

minimum adjusted inverse multiplier: 0.6544444444;

maximum estimated leverage: 0.9487179487<1;

all quantities finite and no controls requiring a rank certificate.

The backend therefore reaches CONVERGED, after which the caller posts the differing point estimates. The positivity and acceptance logic is at lines 3531–3569, and successful posting follows at lines 3705–3735 and 1378–1483. 

Pasted markdown +2

Classification: critical false acceptance under the stated finite-R, fixed-stream literal-frequency contract.

The local copywise moment formulas are correct. The failure is that the caller does not attach the same canonical signs to the same semantic physical copies after an equivalent frequency split.

Critical counterexample 2: identifier relabeling

The same root defect is visible in the smallest accepted worker–firm cycle: four physical observations, one in each cell of K
2,2
	​

, all with frequency one.

Use outcomes and targets in production order:

(y,t)=(1,1),(2.5,2),(3.2,3),(5.7,4).

Swapping the two external worker labels does not change the model, target, graph, exact projection, or exact KSS estimate. But egen group() followed by the sort at lines 1301–1334 moves the former second worker’s rows to the beginning of the indexed RNG stream. 

Pasted markdown

For the admissible R=2 streams

q
L
	​

=
	​

−1
1
−1
1
	​

1
1
−1
1
	​

	​

,q
T
	​

=
	​

−1
1
−1
−1
	​

−1
−1
1
−1
	​

	​

,

both labelings have the identical plug-in vector

(1.5309,0.96,−0.108,2.2749),

but the JLA correction changes from

(0.12516797,0.14304911,−0.63358166,−0.99894624)

to

(−0.09137993,−0.10443421,0.44812268,0.70043121).

Both runs pass: the minimum estimated residual share is 1/14, the minimum adjusted inverse is 1.75, and maximum estimated leverage is 13/14.

Classification: critical under the governing requirement of ID-relabeling invariance. If the intended contract were only invariance in distribution rather than fixed-seed realized invariance, the specification and documentation would need an explicit owner-authorized narrowing. The current validation contract says “results are invariant to ID relabeling,” without that qualification. 

Pasted markdown

Control reparameterization

The sort also includes the raw expanded control columns before outcome, frequency, and target. An invertible transformation can therefore change the physical sign allocation even though the full projection, KSS target, and exact correction are invariant.

For example, take the K
2,2
	​

 graph with two observations in every cell, scalar control values −1,+1 in each cell, and apply the invertible transformation z
⋆
=−z. Sorting on the transformed control reverses the two observations in every cell.

The deterministic joint-control rank certificate accepts this design: the total within-cell scatter is 8; deleting one observation removes scatter 2, so the exact whitened loss is 1/4 and every deleted scatter retains eigenvalue 3/4, well above the registered margin. The certificate’s formulas and direct deleted factorizations are at lines 3155–3264. 

Pasted markdown

A direct R=2 evaluation with asymmetric outcomes and targets produced the same plug-in vector under z and −z,

(5.405625,2.030625,0,7.43625),

but accepted joint-JLA corrections

(−0.12276594,−0.20416838,−0.08307383,−0.49308198)

and

(0.82078037,0.46121443,0.64365816,2.56931113).

The minimum full residual share was 0.15278, maximum estimated leverage 0.84722, and all inverse and rank gates passed.

Classification: critical fixed-stream reparameterization failure caused by the caller sort. The block algebra and rank certificate themselves are not the source of this defect.

Findings by requested counterexample class
Requested class	Finding	Classification
Frequency 2 and duplicate splitting	Accepted six-copy counterexample above; identical physical data produce different finite-R posted results.	Critical
R=2	Zero denominator or zero residual-leverage events are correctly withheld, but R=2 also permits the accepted counterexamples above.	Critical through ordering defect; otherwise conservative withholding
Graph relabeling	Four-copy accepted counterexample above.	Critical under current contract
Control reparameterization	Raw control columns determine RNG ordering; z↦−z accepted counterexample.	Critical under current contract
Singular deletions	No algebraic false acceptance found. Exact mode uses spectral and, near boundary, direct deleted-information gates; JLA joint controls use a deterministic certificate before randomized acceptance.	Valid/conservative
Noncommuting controls	The block maker retains the full control projection, and inverse derivatives require no commutation. Production signs and dimensions agree.	Valid
Extreme control scaling	Stable cell centering, whitening-error subtraction, direct deleted-scatter factorization, and inverse residual gates address the supplied failure classes. No independent finite false acceptance found here.	Valid subject to documented numerical scope
Tied components	Equal firm count and physical mass produce AMBIGUOUS_LARGEST_COMPONENT; no encoded-ID winner is silently selected.	Conservative withholding
Batch changes	Directions are generated sequentially per logical probe and per-column solves are separately gated; the supplied test checks batch 1 versus 17.	Valid
Stale Mata state	API, version, and semantic build token are all checked, and the supplied stale-token fixture is withheld.	Valid for ordinary stale-build states
Coefficient-one formula	Symbolic delta calculation and production expression have coefficient one, not two, on the mixed raw fourth moment.	Valid
Result labels	Successful output is explicitly point estimates only, no e(V), with conditional numerical MCSE separately labeled.	Valid

The coefficient-one formula is supported by the displayed Hessian calculation and production expression. 

Pasted markdown +1

The R=2 exceptional-event tests correctly show that an estimable design may have zero estimated residual leverage or a zero constrained denominator; production withholds those cases rather than silently repairing them. 

Pasted markdown

The noncommuting-control derivative test and production block calculation agree on the plus-B, minus-V signs. 

Pasted markdown +1

Tied-component withholding is implemented both in the component selector and at every subsequent component-selection stage. 

Pasted markdown +1

The stale-runtime guard and its fixture are consistent. 

Pasted markdown +1

Test-evidence limitation

The packet’s self-contained test_frequency_probes.py exhaustive identities pass, and those identities correctly establish the local copywise formulas. However, several tests presented as independent-oracle evidence import kss_bc.tests.python.oracle, whose body is not included in the packet:

test_block_controls.py imports it at lines 4023–4024;

test_dense_oracle.py imports it at lines 4326–4330;

test_jla_formula.py imports it at line 4717.

Pasted markdown +2

The existing public frequency-expansion test uses a fixture in which controls and outcomes distinguish the stored rows before the frequency target sort keys, so expansion happens to preserve the physical stream. It does not cover two otherwise identical rows with different per-copy target masses, which is the smallest failing class. 

Pasted markdown

The end-to-end invariance test changes raw row order and batch size but does not relabel accepted graph identifiers or transform the control basis. 

Pasted markdown

Classification: evidence-package and test-coverage defect. It reinforces, but is not needed for, the false verdict.

Required repairs
1. Replace representation-dependent RNG canonicalization

The physical probe stream must not be indexed by:

stored frequency;

total stored-row target mass;

raw encoded worker, firm, or deletion labels; or

raw control coordinates.

At minimum, frequency-compressed rows must be canonicalized by per-copy semantic attributes, including target/frequency, and equivalent rows must be combined before physical signs are allocated. A frequency-f equivalence class and f explicit frequency-one copies must consume exactly the same physical sign subsequence.

For the broader ID and control-basis contract, the implementation needs either:

a counter-based RNG keyed to a genuinely invariant physical-observation/class key and probe number; or

an owner-approved narrowing stating that fixed-seed realized JLA output is invariant only to raw row ordering and batch size, while ID relabeling and control reparameterization are invariant only in distribution.

The first alternative preserves the present governing contract. The second changes it and cannot be treated as a documentation-only correction.

2. Add three production-level regression tests

The suite must include:

the six-copy frequency-split fixture above, using explicit target mass and probes(2), requiring exact fixed-stream equality between compressed and expanded calls;

an accepted, nontied K
2,2
	​

 worker/firm relabeling test, not merely a tied-component withholding test; and

an end-to-end joint-control test comparing Z with ZA for an invertible A, including a transformation that changes lexicographic row order.

Each test must compare the entire e(results) matrix, not only asymptotic proximity to exact KSS.

3. Complete the source-bound evidence packet

A closure packet should include:

the actual MANIFEST.sha256;

the deterministic ZIP or another independently verifiable archive binding;

the imported oracle.py;

any package and benchmark files invoked by the included static tests.

KB5 closure decision

Do not close KB5.

The blocking line-specific objection is:

kss_bc.ado lines 1326–1334 defines a “canonical” ordering using representation-dependent IDs, controls, frequency, and target; kss_bc.mata lines 3437–3475 and 3620–3630 then assigns sequential physical-copy signs in that order. This permits accepted finite-R outputs to change under literal frequency expansion, identifier relabeling, and control reparameterization, contrary to the governing invariance contract and the documented draw-for-draw claim. 

Pasted markdown +3

Critical objection remaining before closure: YES.
