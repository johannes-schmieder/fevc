---
review_id: KSS-BC-API12-FINAL-N
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 83250f8a41183754f742a93cbc9798714c6c420a95c012e4b8d866e818e53ad1
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f3df5-6eec-83ea-925f-4b5cb08bb5bd
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

False. KB5 closure is blocked. Critical objections remain.

API12 correctly repairs the unchecked final subtraction, and the supplied implementation contains sound repairs for the firm quotient solve, literal-copy JLA formulas, explicit-zero controls, stale-runtime protection, and several rank gates. It nevertheless fails the governing universal contract for every accepted path:

Accepted exact and auto-to-exact executions do not have a control-reparameterization-invariant canonical basis. The exact caller sorts by raw control coordinates before choosing the first eligible anchor. A determinant-one transformation can therefore change the physical anchor by order alone, with neither representation near the API12 cutoff boundary.

Largest-component tie withholding is not reliable for all admitted integer frequency weights. Component physical mass is accumulated as ordinary double precision and compared by exact floating-point equality. Equal exact masses above 2
53
 can be ordered differently after harmless worker relabeling, causing different components to be retained.

Match JLA and all target-probe passes allocate a vector of length equal to total physical frequency without the observation-only physical_limit() gate. This leaves a source-level unbounded-allocation path rather than a typed conservative withholding.

The first two are direct contract violations; the third is a critical production-path failure. The API12 numerical error envelope also remains insufficient as a proof of universal JLA control-basis invariance.

1. Packet authentication

I extracted and SHA-256 checked all 33 manifest-listed bodies:

nine package and mathematical documentation bodies;

kss_bc.ado, kss_bc.mata, and kss_bc.sthlp;

the oracle and six Python test bodies;

ten Stata test bodies; and

varcomp_hdfe_specification.md, the unwrapped PROMPT.md, REQUEST.yaml, and SOURCE_MAP.md.

Every extracted body exactly reproduced its manifest digest. 

Pasted text

The stated outer hash cannot be independently authenticated because the transport expressly says that the ZIP bytes are unavailable; only its asserted hash is present. 

Pasted text(20260814-161020)

2. Critical finding 1 — the API12 canonical-anchor claim is false on accepted exact paths
2.1 What is true with a fixed invariant row order

Suppose exact whitening produces Q, while an invertible reparameterization produces 
Q
	​

=QO for an orthogonal O. If both executions select the same physical anchor rows, then every residualized row-score vector is invariant: after a selected anchor matrix A, the transformed residualized basis is the original residualized basis times O, so its row squared norms agree.

The final canonical basis is also invariant:

C
=QO(AO)
′
[(AO)(AO)
′
]
−1
=QA
′
(AA
′
)
−1
=C.

Thus the exact-arithmetic construction is valid conditional on a common, basis-invariant physical row order and anchor sequence.

2.2 The exact public path violates that condition

The JLA branch first orders rows by outcome and per-copy target mass and withholds nonexchangeable primary-key ties. The exact branch instead sorts primarily by encoded worker and firm identifiers and then by the raw user control coordinates. For match deletion it also puts the encoded deletion ID before those raw controls. algorithm(auto) inherits the exact ordering whenever it dispatches to exact. FILES/kss_bc/kss_bc.ado:1658–1717. 

Pasted text(20260814-161020)

The exact backend then calls kssbc__canonical_controls() on that already sorted matrix. FILES/kss_bc/kss_bc.mata:2409–2421. 

Pasted text(20260814-161020)

The anchor routine does not choose the unique maximum. It defines every row above maximum-margin*max(1,maximum) as eligible and selects eligible[1]. Its ambiguity gate protects only scores near the eligibility cutoff, not multiple interior eligible rows whose physical order changes. It subsequently maps the selected rows to the identity. FILES/kss_bc/kss_bc.mata:2171–2236. 

Pasted text(20260814-161020)

That contradicts the stated contract that the basis is ID-free and that accepted Z and ZT produce the same numerical right-hand sides. FILES/kss_bc/docs/ESTIMATOR_CONTRACT.md:700–709. 

Pasted text(20260814-161020)

2.3 Finite accepted counterexample

Put the following three two-control rows in one worker–firm cell:

q
1
	​

=(
2/3
	​

,0),q
2
	​

=(−1/
6
	​

,1/
2
	​

),q
3
	​

=(−1/
6
	​

,−1/
2
	​

).

They satisfy

Q
′
Q=I
2
	​

,∥q
1
	​

∥
2
=∥q
2
	​

∥
2
=∥q
3
	​

∥
2
=2/3,q
1
	​

+q
2
	​

+q
3
	​

=0.

Add two zero-control observations in each of the other three cells of a K(2,2) worker–firm graph. The resulting public observation-deletion design has nine rows and five parameters. It is connected, its full design has rank five, and every one-observation deletion retains rank five. Give the rows distinct finite outcomes and unit frequencies.

Now compare Z=Q with

U=ZT=−Q,T=−I
2
	​

,det(T)=1.

Both representations have the same Gram matrix, conditioning and residual diagnostics. At pivot one, all three nonzero rows have score 2/3, whereas the cutoff is 2/3−10
−7
. They are therefore safely inside the eligibility set, about 10
−7
 from the cutoff rather than within the 10
−12
 uncertainty floor. After the first pivot, the two remaining candidates each have residualized score 1/2, again safely inside the eligibility set.

The exact raw-coordinate sort gives:

for Q: physical order q
3
	​

,q
2
	​

,q
1
	​

, hence anchors q
3
	​

,q
2
	​

;

for −Q: physical order −q
1
	​

,−q
2
	​

,−q
3
	​

, hence physical anchors q
1
	​

,q
2
	​

.

Aligned to the same physical observations, the two accepted canonical matrices are

C
Q
	​

=
	​

−1
0
1
	​

−1
1
0
	​

	​

,C
−Q
	​

=
	​

1
0
−1
	​

0
1
−1
	​

	​

.

Their maximum elementwise difference is 2. Neither execution is near an API12 cutoff or conditioning gate. This is basis-dependent acceptance in exact arithmetic, not conservative withholding and not a floating-point edge case. Both nuisance conventions invoke the same preliminary canonicalization, and default auto selects exact because the parameter count is only five.

The registered four-row and 14-row fixtures do not cover this case. They exercise a score at the fuzzy cutoff and a representation whose numerical envelope becomes too large. The tests correctly require both four-row representations to withhold, and require the 14-row original representation to converge while the transformed representation withholds across exact, JLA and auto, both nuisance modes, and two batch sizes. Those are legitimate conservative failures, but they do not test multiple safely eligible rows after a coordinate-dependent exact sort. FILES/kss_bc/tests/stata/test_control_anchor.do:6139–6235. 

Pasted text(20260814-161020)

Classification: critical accepted-path contract defect.

3. Critical finding 2 — exact component ties can be resolved differently after ID relabeling

The documented graph rule ranks components first by firm count and then by physical mass, and says an exact tie on both quantities must return AMBIGUOUS_LARGEST_COMPONENT rather than select through encoded identifiers. FILES/kss_bc/docs/NUMERICAL_ARCHITECTURE.md:1086–1101. 

Pasted text(20260814-161020)

The implementation, however:

accepts every finite, positive, integer-valued double as a frequency and imposes no 2
53
 total-mass bound; FILES/kss_bc/kss_bc.ado:1425–1450; 

Pasted text(20260814-161020)

sequentially accumulates component_mass[root] += frequency[row] in ordinary double precision; and

declares ambiguity only when the two resulting floating values compare exactly equal. FILES/kss_bc/kss_bc.mata:3252–3283. 

Pasted text(20260814-161020)

The graph caller then treats any nonambiguous result as authoritative. FILES/kss_bc/kss_bc.mata:3509–3525. 

Pasted text(20260814-161020)

Finite relabeling counterexample

Let L=2
52
. Construct two disconnected K(2,2) components, each with two firms and one stored match row on each edge.

Use these row-major frequency sequences:

A=(L−20,L−20,L−19,L−19),
B=(L−19,L−19,L−20,L−20).

Their exact integer masses are equal:

∑A=∑B=4L−78=18,014,398,509,481,906.

Every individual weight is below 2
53
 and is exactly representable as a double. Sequential binary64 addition nevertheless gives

flsum(A)=18,014,398,509,481,904,
flsum(B)=18,014,398,509,481,906.

The code therefore selects component B and sets no ambiguity flag. Simultaneously swapping the two worker labels within each component reverses each row-major sequence:

A⟼B,B⟼A,

so the selected component flips to A, although exact firm counts and exact physical masses are unchanged.

This is not caused by poor model conditioning. Each retained component is a balanced K(2,2); its scaled three-parameter information matrix has eigenvalues bounded well away from zero, and deleting any one match edge leaves a full-rank tree. With small finite outcomes the dense exact fit, block gates, correction and final subtraction remain finite. Choosing different interaction amplitudes in the two components gives different posted plug-in and correction rows. The source therefore permits harmless ID relabeling to change the retained sample and target instead of returning the required typed ambiguity.

The same unrestricted summation also makes exact N_physical and observation-deletion-unit metadata incapable of representing every literal integer count once totals exceed 2
53
.

Classification: critical false sample selection and estimand change.

4. Critical production defect — match JLA has an unbounded physical-copy allocation path

The ado-level PHYSICAL_COPY_LIMIT check is applied only when the selected algorithm is JLA and deletion is observation. FILES/kss_bc/kss_bc.ado:1651–1656. 

Pasted text(20260814-161020)

But kssbc__physical_rademacher_sum() constructs a vector with sum(frequency) entries before collapsing it to stored-row sums. FILES/kss_bc/kss_bc.mata:3092–3115. 

Pasted text(20260814-161020)

That helper is called:

once per match-leverage probe on match JLA; FILES/kss_bc/kss_bc.mata:4042–4058; 

Pasted text(20260814-161020)

once per target probe for both deletion conventions. FILES/kss_bc/kss_bc.mata:4202–4213. 

Pasted text(20260814-161020)

Thus a small stored match dataset with very large finite frequencies can attempt an arbitrarily large allocation, potentially ending in an untyped Mata/runtime failure rather than conservative withholding. The behavior also contradicts the architecture statement that physical-copy storage is required only for literal observation JLA and has a typed pre-allocation boundary. FILES/kss_bc/docs/NUMERICAL_ARCHITECTURE.md:1270–1278. 

Pasted text(20260814-161020)

The registered test checks only observation JLA, so it does not falsify this match/target-pass route. FILES/kss_bc/tests/stata/test_failures.do:6485–6491. 

Pasted text(20260814-161020)

Classification: critical production-path and typed-failure defect. It does not change the algebra on successfully completed small calls, but it prevents the claimed production contract from holding over admitted finite inputs.

5. Additional unresolved objection — the numerical envelope is not a proved forward-error certificate

Even after repairing exact row ordering, the supplied API12 envelope is not sufficient by itself to prove the universal JLA assertion.

kssbc__max_column_relres() reports

ρ
c
	​

=
j
max
	​

∥Re
j
	​

∥
2
	​

,

not a subordinate matrix norm. FILES/kss_bc/kss_bc.mata:1928–1945. 

Pasted text(20260814-161020)

kssbc__inverse() uses that number as its inverse residual, and the anchor code divides it by reciprocal conditioning, takes a fixed 100-fold multiple, and applies the margin/4 and cutoff gates. FILES/kss_bc/kss_bc.mata:2054–2097, 2153–2217. 

Pasted text(20260814-161020) +1

The relevant standard norm fact is

ρ
c
	​

≤∥R∥
2
	​

≤∥R∥
F
	​

≤
k
	​

ρ
c
	​

.

Consequently, relres/rcond is not itself an inverse forward-error bound. A fixed factor of 100 is not dimension-independent, and JLA has no explicit control-count bound that would make it so. More importantly, the measured quantities do not bound:

error in forming Z
′
WZ;

the distance between the two computed column spaces obtained from Z and ZT;

Cholesky and controls*whitener formation error;

row-score and maximum/cutoff accumulation error; or

the accumulated error in successive residualized projectors.

A small whitening residual proves that each computed basis is nearly orthonormal in its own computed span. It does not prove that two separately computed spans are sufficiently close. The final anchor residual proves that the chosen rows map nearly to the identity; it does not prove that the same physical rows were or should have been chosen.

I did not identify a second native JLA accepted-pair witness beyond the exact-order counterexample, but the packet does not supply the numerical theorem needed to convert these residual proxies into the claimed universal accepted-path invariance. This remains a genuine proof obligation, not merely a documentation omission.

6. Accepted-path reconstruction
Preprocessing and dispatch

The public caller validates tuning, including the 10
−4
 maximum PCG tolerance; validates positive integer frequencies and target masses; drops only factor-variable terms carrying Stata omitted metadata; freezes complete match inputs; and checks the API level, version and semantic build token. It then performs graph selection and selects exact versus JLA from the complete parameter count and exact_limit(). FILES/kss_bc/kss_bc.ado:1384–1417, 1420–1519, 1546–1569, 1639–1656. 

Pasted text +1

The explicit-zero and negative-zero repairs are present: ordinary numeric columns remain in the design and reach the rank gates. The registered tests cover exact, JLA, both nuisance conventions, both deletion conventions and both auto dispatches. FILES/kss_bc/tests/stata/test_failures.do:6419–6483. 

Pasted text(20260814-161020)

Exact backend

After the defective exact row ordering and canonicalization, the exact backend:

builds the full worker/all-but-one-firm/control design;

equilibrates, spectra-checks and residual-checks its information inverse;

estimates the preliminary full joint fit;

under fixed offset, forms y−Z
γ
	​

 and separately factors the pure-FE working design;

constructs the three target matrices and total identity;

evaluates observation deletion copywise or match deletion by weighted block Woodbury algebra;

applies direct deleted-information factorizations near its numerical boundary; and

checks plugin, correction and final difference separately. FILES/kss_bc/kss_bc.mata:2448–2598. 

Pasted text(20260814-161020) +1

Apart from the canonical-order and large-mass graph defects, I found no finite-algebra sign, normalization or deletion-unit error in these contractions.

JLA backend

The pure-FE service reconstructs the missing last-firm right-hand side, solves the full singular firm Laplacian on its zero-sum quotient, grounds only after convergence, reconstructs workers, and recomputes every worker and firm normal-equation residual. The batched routine invokes and gates each scalar right-hand side separately. FILES/kss_bc/kss_bc.mata:2818–2945. 

Pasted text(20260814-161020)

Joint controls use the FE inverse actions to build the low-dimensional Schur complement, compare it with the directly residualized-control cross-product, invert it through the registered dense gate, and recompute the full joint residual column by column. FILES/kss_bc/kss_bc.mata:2949–3089. 

Pasted text(20260814-161020) +1

The probe-independent rank certificate explicitly centers controls within worker–firm cells, whitens the within-cell scatter, forms each deletion loss with nonnegative formulas, directly eigendecomposes and inverse-checks every deleted scatter, and subtracts whitening and rounding margins from the final gap. FILES/kss_bc/kss_bc.mata:3684–3834. 

Pasted text(20260814-161020)

Those are valid conservative gates, subject to the broader numerical-envelope limitation above.

JLA correction and targets

The implementation correctly:

constrains estimated P and M through their common denominator;

uses coefficient one on the mixed fourth moment;

applies +B and −V in both observation and match inverse adjustments;

adds exact residualized-control leverage under joint nuisance;

forms worker, firm and total target contractions with common draws; and

defines covariance probe by probe as (T−W−F)/2. FILES/kss_bc/kss_bc.mata:4111–4195, 4214–4276. 

Pasted text(20260814-161020) +1

The target matrices use normalized stored-row target mass, give controls zero action, and implement total as worker plus firm plus twice covariance. FILES/kss_bc/kss_bc.mata:2266–2311. 

Pasted text(20260814-161020)

The literal-copy sufficient statistics are also algebraically correct: per-copy observation nonlinearities are retained until final multiplier aggregation; match contractions use the summed physical signs; and target directions implement per-copy target mass. FILES/kss_bc/docs/JLA_FINITE_PROJECTION.md:934–1045. 

Pasted text(20260814-161020)

7. Final corrected-target repair

This API12 repair is valid.

A finite row such as

P=(8×10
307
,1,2,3)

and the finite opposite-signed correction

B=(−8×10
307
,0,0,0)

produce a Stata/Mata missing value in the first element of P−B, although both input rows pass hasmissing(). The packet registers exactly this arithmetic witness. FILES/kss_bc/tests/stata/test_load.do:7104–7106. 

Pasted text(20260814-161020)

Both production backends now form the difference only after checking plugin and correction, then return NONFINITE_CORRECTED_TARGET if that completed row is nonfinite:

exact: FILES/kss_bc/kss_bc.mata:2590–2598; 

Pasted text(20260814-161020)

JLA: FILES/kss_bc/kss_bc.mata:4279–4289. 

Pasted text(20260814-161020)

The ado caller checks backend status before extracting or posting any result matrices. A failure clears prior e() results and posts only status="WITHHELD" and the typed failure; ereturn post is reached only after CONVERGED. FILES/kss_bc/kss_bc.ado:1741–1788, 1874–1884. 

Pasted text(20260814-161020) +1

Conclusion on this repair: both exact and JLA correctly withhold before any successful or partial point result is posted.

8. Counterexample and fixture replay

No Stata executable is present in the execution environment, so I do not represent the native .do assertions as having run here. I independently executed the five substantive supplied Python algebra/oracle modules—dense oracle, literal-frequency probes, JLA formula, block controls and Monte Carlo bias—and obtained 29 passing tests. I also independently reconstructed the production Mata arithmetic and control flow rather than treating those tests as dispositive.

Attack or fixture	Independent conclusion
Four-row exact cutoff	Both representations are correctly withheld at the cutoff boundary. Conservative and valid.
Native 14-row determinant-one witness	Original representation is admissible; transformed representation is conservatively withheld. The source assertions cover exact/JLA/auto, both nuisance modes, batches 1 and 2, seed 2, probes 2 and tolerance 10
−4
. Valid withholding, not proof of the universal claim.
New three-row interior-anchor witness embedded in nine-row K(2,2)	Both exact representations are accepted, but Z and −Z produce different physical canonical anchors and O(1)-different canonical matrices. Critical failure.
Six-row K(2,3) firm relabeling	Full quotient construction and recomputed residual logic are algebraically permutation equivariant; no grounding defect remains in the inspected implementation. The source test exercises two firm permutations and two batch sizes. FILES/kss_bc/tests/stata/test_semantics.do:7213–7237. 

Pasted text(20260814-161020)


Determinant-four JLA map	With the invariant JLA row order, the implementation uses the same canonicalized span in the registered fixture under both nuisance conventions. FILES/kss_bc/tests/stata/test_semantics.do:7184–7199. 

Pasted text(20260814-161020)


Positive zero, negative zero and zero-plus-valid control	Numeric zero columns are retained and rejected by rank gates rather than silently omitted. Valid.
Six-cycle control in the FE span	Exact and JLA full-fit gates withhold under both deletion conventions in the registered source fixture. Valid conservative failure.
Frequency two and literal expansion	Exact formulas and copywise JLA sufficient statistics agree with literal physical-copy semantics. Valid.
Automatic dispatch	Uses the retained full parameter count and does not silently retry with the other backend after a typed failure. Valid.
Opposite-signed finite rows	Exact and JLA now return NONFINITE_CORRECTED_TARGET before posting. Valid.
Observation physical-copy limit	Correctly checked before allocation.
Match JLA physical-copy limit	Missing; match leverage and all target passes can allocate physical-length random vectors without a typed gate. Critical production failure.
9. Required repairs before KB5 closure
Repair A — make anchor row order invariant on every backend

Canonicalization must occur in the same control-coordinate- and ID-invariant conceptual order for exact, JLA and both auto dispatches. Removing only controlvars from the exact sort is insufficient because encoded worker and firm labels can also change the order of eligible rows.

A robust repair should either:

order physical rows using invariant noncontrol semantics before canonicalization and withhold unresolved nonexchangeable ties; or

select anchors through a genuinely row-permutation-invariant rule based on invariant projector information, with typed withholding when the anchor set is not uniquely certifiable.

Register the nine-row Q versus −Q fixture above under exact and auto-exact, both nuisance modes. Assert equality of the row-aligned canonical matrices or typed withholding, not merely equality of final dense point estimates.

Repair B — make component physical-mass comparison exact

Either:

reject or type-withhold any input whose exact nonnegative integer physical total exceeds 2
53
; or

accumulate and compare integer masses through an exact expansion or multiword integer representation.

Pairwise or compensated floating summation reduces error but does not establish exact tie recognition. Register the two-K(2,2), L=2
52
 relabeling witness and require AMBIGUOUS_LARGEST_COMPONENT before dispatch in both labelings.

The same repair should cover N_physical, observation deletion-unit counts and any other metadata asserted to be literal integer counts.

Repair C — bound or stream every physical-sign allocation

While kssbc__physical_rademacher_sum() allocates a physical-length vector, the caller must enforce a pre-allocation limit for every JLA path that reaches it, including match leverage and the target pass.

The preferable production repair is a canonical counter-based or chunked generator that accumulates each stored-row S
r
	​

 without holding all physical signs, while preserving the fixed-seed conceptual stream under row splitting and regrouping. A simple per-row binomial draw is not automatically pathwise equivalent to the required canonical physical stream.

Register match and auto-to-JLA failures with a deliberately tiny physical_limit(), and separately exercise the target pass.

Repair D — replace the heuristic anchor envelope with a stated numerical theorem

The packet must supply a dimensioned a posteriori bound covering:

Gram formation;

inverse and Cholesky error;

computed-span error under Z↦ZT;

row-score and cutoff error;

every later residualization pivot; and

final canonical-basis or projector disagreement.

Use subordinate or explicitly dimension-adjusted norms. If the bound requires limits on controls, row counts, scaling or conditioning, enforce them as typed gates. Until then the envelope should be described as empirical hardening, not a certificate proving every accepted-path invariance claim.

10. Classification of remaining matters

Critical estimator or production-contract defects

Exact/auto-exact canonical basis depends on raw-coordinate row ordering.

Exact component selection can depend on identifier relabeling through rounded physical-mass accumulation.

Match/target JLA has an unbounded physical-sign allocation route.

Universal JLA anchor invariance lacks the required numerical proof.

Valid conservative withholding

The four-row and 14-row API12 anchor fixtures.

Full-fit and deleted-control rank certificates.

Explicit zero or collinear numeric controls.

Ambiguous probe order and ambiguous largest components at ordinary exactly represented masses.

Near-boundary deleted information and block inverse gates.

Observation physical-copy allocation boundary.

Repairs verified as effective

Full-firm quotient solve and post-convergence grounding.

Per-column scalar and batched residual gates.

Coefficient-one fourth moment.

Literal-copy observation nonlinearities.

General match control block and +B,−V signs.

Target normalization and probe-by-probe accounting.

Separate full and correction parameter metadata.

Stale semantic-runtime rejection.

Final corrected-target finiteness gate and no-partial-post behavior.

Outside this audit

Sampling inference, exact finite-R unbiasedness, uniform asymptotics, empirical independence assumptions, restricted-data qualification, licensing and unified-package release status remain outside the source-bound finite point-estimator question. The packet itself confines the obligation accordingly. 

Pasted text

KB5 closure decision

Do not close KB5. The required verdict is false, with critical source-level objections remaining before KB5-FINAL-PRODUCTION-CONTRACT-N can be satisfied.

