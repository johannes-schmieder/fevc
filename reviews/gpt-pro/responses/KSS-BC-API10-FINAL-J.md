---
review_id: KSS-BC-API10-FINAL-J
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 1d9eed35fdfcd0ece88f18c24ed354a9cad544fe62b5fc69a0233ecefbfa1bbd
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f1949-2984-83ea-80cd-fe57313e7d0a
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

False. KB5 closure must be denied.

The API-10 production contract has at least two independent critical accepted-path defects:

An explicitly supplied all-zero control—including a -0 control—is silently removed by the ado caller. The exact or JLA backend can then post estimates for a reduced model even though the requested worker–firm–control design is exactly singular. This bypasses the advertised all-control rank certificate before Mata is called.

The fixed-seed JLA result is not invariant to pure firm-ID relabeling or invertible control-basis reparameterization at allowed solver tolerances. The canonical physical-copy order remains unchanged, but grounding one encoded firm and adaptively stopping separate PCG solves changes the accepted numerical operator. I constructed finite, accepted probes(2) runtime states with material changes in posted estimates while every recomputed residual is below the registered gate.

Both contradict the packet’s explicit requirements that accepted fixed-seed output be invariant to ID relabeling and invertible control-span reparameterization and that every requested model with controls have an identified full fit. 

Pasted text

Line-specific closure decision: KB5-FINAL-PRODUCTION-CONTRACT-J is NOT CLOSED. Critical objections remain at kss_bc.ado:1428–1433 and in the interaction of kss_bc.mata:2507–2515, 2568–2679, and the permissive acceptance rule at 2641–2649.

Authentication

I independently recomputed the SHA-256 digest of PROMPT.md and every embedded body represented in MANIFEST.sha256, preserving each source’s transported end-of-file newline convention. All 31 manifest entries matched. The repository-path hashes in SOURCE_MAP.md also agree with the manifest. 

Pasted text +1

The outer deterministic ZIP bytes were not attached separately, so I could not recompute the stated ZIP digest 1d9eed…bbd; as the packet directs, that is only a transport limitation, not an estimator finding. 

Pasted text

Assumptions and scope

I treated only finite point estimation, sample/target semantics, deterministic and randomized numerical acceptance, and the public return contract. I did not infer econometric inference, theorem transfer, uniform asymptotics, restricted-data validity, licensing, or public-release readiness. Those exclusions are expressly imposed by the packet. 

Pasted text

The source-bound varcomp_hdfe package-unification requirements are outside this KB5 estimator audit because the owner decision explicitly defers unification and develops kss_bc as a sibling package. 

Pasted text

End-to-end implementation trace
Stage	Exact production path	JLA production path	Audit result
Parsing and sample freeze	Options, frequency and target validation, complete-case handling, factor expansion	Same	Mostly fail-closed, except the all-zero-control deletion described below. 

Pasted text


Runtime binding	API level, version, and semantic build token checked before dispatch	Same	Detectable stale builds fail closed. 

Pasted text


Graph/sample selection	Largest component, mover restriction where applicable, iterative worker-articulation pruning	Same	Component ties are conservatively withheld. 

Pasted text


Final encoding and ordering	Deterministic sort by encoded coordinates and stored fields	Outcome/per-copy-target primary key, with fail-closed checks on controls, coordinates, and match blocks	The sign order itself is correctly protected; the downstream solver is not equivariant. 

Pasted text


Full fit	Dense full design, equilibrated inverse, optional fixed-offset reduction	Worker-eliminated FE system, low-dimensional joint-control Schur system, rank certificate	Dense algebra is structurally correct. JLA rank logic is bypassed by the caller’s zero-column removal. 

Pasted text +1


Deletion correction	Per-copy observation denominator or general compressed match block	Copywise finite-projection adjustment or rank-one match adjustment plus exact control action	No coefficient or sign error found. 

Pasted text +1


Target contractions	Dense target matrices	Independent target-probe pass, common worker/firm directions, covariance by accounting	Algebraically correct. 

Pasted text +1


Posting	Corrected row posted to e(b) and e(kss)	Same	Main labels and accounting are correct; two secondary metadata mismatches remain. 

Pasted text

Findings
F1. Critical accepted-path defect: explicit zero controls are silently removed

The caller expands the controls and then retains a column only if at least one retained value is not numerically equal to zero:

stata
quietly count if `control' != 0 & `touse'
if r(N) local controlvars `controlvars' `control'

The accompanying comment refers specifically to omitted factor-variable bases, but the implementation makes no provenance check. It therefore treats an explicitly requested numeric zero column exactly like an automatically omitted factor base. 

Pasted text

After that removal:

control_count and the selected algorithm are computed from the reduced list;

the exact or JLA backend receives no record that the control was requested;

the JLA rank certificate is called only when the reduced controls_count > 0; and

the return label can state either an FE-only certificate or an all-control certificate covering only the surviving subset. 

Pasted text +2

This contradicts both the governing assumption that every model with controls have an identified full worker–firm–control fit and the package claim that every randomized calculation with controls passes the deterministic full-fit certificate. 

Pasted text +1

Smallest exact counterexample

Use the complete K
2,2
	​

 worker–firm graph:

y	worker	firm	explicit control z
0
	​


1	1	1	0
2	1	2	-0
3	2	1	0
5	2	2	0

Call:

stata
kss_bc y z0, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact)

The requested quotient has four columns—two worker indicators, one grounded firm indicator, and z0—but rank three. Every full worker–firm–control fit is therefore singular.

The caller nevertheless removes z0, dispatches the rank-three pure-FE model, and the exact backend accepts every observation deletion. A direct reconstruction of the code gives:

component	worker	firm	covariance	total
plug-in	1.5625	0.5625	0	2.125
correction	0.0625	0.0625	0	0.125
posted corrected	1.5	0.5	0	2.0

The exact backend’s pure-FE information and observation-deletion gates are the paths that accept this reduced design. 

Pasted text +2

This is not a harmless numerical normalization. The requested coefficient vector is unidentified, and the public command silently changes the model instead of returning SINGULAR_NUISANCE_BLOCK or SINGULAR_INFORMATION.

JLA bypass

Appending an explicit z0=0 to any accepted controlled JLA call produces the same backend inputs as the call without z0. For example, appending z0 to the packet’s 24-row c1 c2 fixture causes the caller to send only c1 c2; the rank certificate and posted positive deletion_rank_gap therefore certify a strict subset of the requested controls. The source fixture contains accepted joint and fixed-offset JLA paths for those surviving columns. 

Pasted text +1

The defect also survives:

frequency two;

literal expansion or partial regrouping;

observation or match deletion;

joint or fixed-offset nuisance treatment;

a zero column included alongside valid controls; and

positive or negative IEEE zero.

Classification: critical accepted-path defect.

F2. Critical accepted-path defect: canonical signs do not make the JLA solve invariant

The API-10 ordering gate correctly avoids using IDs, frequencies, stored-row labels, or raw control coordinates to choose the conceptual-copy sign order. The documentation consequently claims that accepted fixed-seed executions are pathwise invariant to control reparameterization, ID relabeling, sorting, and regrouping. 

Pasted text

That conclusion does not follow for the complete computation:

Final worker and firm IDs are re-encoded from the supplied identifiers. 

Pasted text

The implementation always grounds the final encoded firm and solves only the first firm_levels-1 coordinates. 

Pasted text

The Schur diagonal and therefore the PCG representation depend on that grounding. 

Pasted text

Each right-hand side is solved separately, with adaptive termination based on its own reduced residual. 

Pasted text +1

The public caller permits any tolerance() below one, while the final full-system gate permits a relative residual as large as 10*tolerance(). 

Pasted text +1

Thus the same canonical sign stream can be applied to two different accepted approximate inverse operators.

Six-row probes(2) firm-relabeling counterexample

Use unit frequencies, default target masses, observation deletion, probes(2), batch(1), and tolerance(.09):

canonical row	y	worker	firm
1	-15	1	3
2	-8	2	2
3	6	2	3
4	11	2	1
5	13	1	1
6	14	1	2

This is the complete K
2,3
	​

 graph. It has one connected component, no worker articulation, three histories for each worker, and every observation deletion remains identified. Outcomes are unique, so the API-10 primary ordering key never ties.

Now compare that data with the pure relabeling that swaps firm labels 2 and 3. Use the following realized physical-copy Rademacher stream in the displayed canonical order:

leverage 1:  - + + - + +
leverage 2:  + - + + + +
target   1:  + + - - - +
target   2:  - + + - + +

Such vectors are authorized finite states under the packet’s declared copywise Rademacher law. 

Pasted text

For the original labels, the fitted-outcome PCG reduced residual after one iteration is approximately 0.4691 of its initial value, so it performs the second iteration and reaches the exact two-dimensional solution. After swapping firm labels 2 and 3, the first-iteration reduced ratio is approximately 0.02020, so the same tolerance(.09) stops after one iteration.

The independently reconstructed accepted results are:

diagnostic/result	original labels	firms 2 and 3 swapped
maximum recomputed solve residual	1.50×10
−16
	0.01587544
permitted full-residual gate	0.9	0.9
plug-in firm variance	45.50000000	45.44446140
total correction	72.63027984	71.58081630
corrected total	-26.88027984	-25.88635490
numerical MCSE, total	34.03429630	33.23961200

Both runs pass the implemented residual rule by a wide margin and post KSS_POINT_ESTIMATES_ONLY, but the same physical design and same conceptual-copy sign stream produce materially different results. The exact dense calculation is invariant under the relabeling; its corrected total is -70 in both encodings.

This is precisely the outcome prohibited by the contract’s ID-relabeling requirement. 

Pasted text

Invertible control-basis counterexample after a successful rank certificate

The same defect extends to controls because kssbc__fe_solve_matrix() adaptively solves each control and target right-hand side separately. Joint preparation then uses those approximate columns to construct the residualized controls and Schur complement. 

Pasted text +1

Using the packet’s 24-row fixture and its two controls Z=(c
1
	​

,c
2
	​

), compare Z with

Z
∗
=ZT,T=(
1
−3
	​

1
1
	​

),

so z
1
∗
	​

=c
1
	​

−3c
2
	​

, z
2
∗
	​

=c
1
	​

+c
2
	​

, and detT=4. The fixture itself is specified at packet lines 5008–5022. 

Pasted text

For both bases, the independently reconstructed deterministic deletion certificate has deletion_rank_gap ≈ 0.58823519. With probes(2) and tolerance(.09), the same conceptual-copy sign stream produces:

result	Z	ZT
maximum solve residual	0.03009038	0.02930059
plug-in total	0.21399566	0.21426658
correction, firm component	0.31507728	0.32692427
total correction	0.32443253	0.33456687
corrected total	-0.11043687	-0.12030029

Both residuals are below the allowed 0.9 gate, and both rank certificates pass. The certificate correctly establishes rank; it does not make the adaptive approximate inverse invariant to a change of control basis. The full joint residual gate at the end of the solve accepts the difference. 

Pasted text

Classification: critical accepted-path defect.

Systematic minimal-counterexample search
Requested attack	Result
Frequency 2	The stored-to-physical projection law, per-copy observation moments, and final multiplier aggregation are algebraically correct. The zero-control defect persists unchanged at frequency two.
Probes 2	Used directly in the accepted firm-relabeling and control-basis counterexamples above. Low probe count is not itself the defect; the defect is non-equivariant accepted solves.
Partial row splitting	No independent copy-mapping defect found. The implementation assigns physical signs before collapsing, retains copy-specific correlations, and averages only final nonlinear multipliers. 

Pasted text +1


Equal primary keys	Differing controls, worker–firm coordinates, or match blocks trigger AMBIGUOUS_PROBE_ORDER. Rows that pass the tie test are exchangeable for the implemented contractions. This is conservative withholding, not false acceptance. 

Pasted text


Zero / negative zero	Critical: explicit all-zero control columns are removed. A -0 frequency is rejected and a -0 target mass is correctly equivalent to zero mass.
Missing values	Frequency and target missingness are rejected on the requested scope; observation mode uses complete cases, while match mode fails closed if any frozen requested input is incomplete. 

Pasted text


Extreme finite values	No additional accepted point-estimate counterexample was needed for the verdict. However, physical-copy storage is allocated from sum(frequency) without a public physical-copy size gate, so extreme counts can cause an untyped allocation/runtime failure rather than a typed withholding. This is optional hardening, not demonstrated false acceptance. 

Pasted text +1


Transformed controls	Critical: the rank certificate can pass in both bases while separate adaptive solves produce different accepted estimates.
Relabeled IDs	Critical: swapping two firm labels changes the grounded quotient and PCG stopping while the conceptual-copy stream remains fixed.
Match labels	Separate declared matches sharing a fitted coordinate remain separate; cross-coordinate labels are rejected. No bypass found. 

Pasted text +1


Tied components	All inspected component-selection stages fail closed on ties in firm count and physical mass. This is conservative withholding. 

Pasted text +1


Stale Mata state	Mismatched API/version/build tokens are withheld before computation. A same-token modified runtime is not cryptographically authenticated, and a partial stale namespace can still cause an untyped load failure, but neither is an estimator false-acceptance finding for the authenticated supplied bodies. 

Pasted text

All-control rank-certificate audit
Certificate reached with all requested nonzero columns

Once the requested controls actually reach Mata, the deterministic certificate has the correct sufficient structure:

two-pass within-cell centering;

an equilibrated inverse of the full within-cell scatter;

explicit whitening-error subtraction;

nonnegative match scatter-loss decomposition;

direct eigendecomposition and inverse-residual checks for every deleted scatter; and

a final positive gap requirement. 

Pasted text +1

The underlying sufficiency argument is valid: positive-definite retained within-cell scatter rules out a nonzero control combination that becomes constant in every retained worker–firm cell. The packet also correctly labels the trace screen as conservative rather than necessary. 

Pasted text

Attack outcomes
Design	Audit result	Classification
Exactly FE-collinear, nonzero control	The joint Schur inverse or within-cell inverse rejects it; if approximate FE residualization creates a positive Schur, the deterministic within-cell certificate remains an independent barrier. 

Pasted text +1

	Correct withholding
Explicit zero or -0 control	Removed before Mata; neither Schur nor certificate sees it	Critical false acceptance
Nearly FE-collinear control	Exact rank can be certified after whitening, but the JLA path has no coefficient/fitted-value forward-error bound. The accepted transformed-basis example demonstrates the resulting whole-path defect.	Covered by F2
Pure rescaling	The low-dimensional inverse and certificate are deliberately equilibrated. No algebraic rank change found; overflow/underflow remains an extreme-value boundary.	No separate defect found
Ill-conditioned multi-control deletion	Every deleted whitened scatter is directly factored in addition to the trace test. No post-dispatch rank bypass found. 

Pasted text +1

	Correct or conservative withholding
Observation deletion	The certificate removes one physical copy even when a stored row has frequency greater than one. The remaining_frequency = cell_frequency-1 formula is correct. 

Pasted text

	Correct
Match deletion	The loss is deleted within-scatter plus the deleted/retained mean-gap term. This is the correct positive-semidefinite scatter loss. 

Pasted text

	Correct
Joint nuisance	Certificate runs before the point calculation	Correct, subject to F1/F2
Fixed-offset nuisance	The same full-fit and deletion certificate runs before the offset reduction. Requiring deleted control rank is stronger than necessary and is documented as such. 

Pasted text +1

	Conservative withholding

The rank certificate is therefore not algebraically disproved after a complete, nonzero control matrix reaches it. The closure failure is that the caller can remove a requested singular column before certification, and that certification of rank does not cure the accepted approximate-solver invariance failure.

Nonlinear coefficients, signs, normalizations, and contractions
Verified

Mixed fourth-moment coefficient: the coefficient on
(M−P)m(P,M) is one, not two. The Hessian derivation and production code agree. 

Pasted text +1

Observation reciprocal signs:

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


is implemented with the correct plus sign on bias and minus sign on variance. 

Pasted text +1

Match inverse adjustment signs: the block adjustment has +B and -V, matching differentiation with h=1−m. 

Pasted text +1

Target normalization: explicit targetweight() is stored-row mass and is not multiplied by frequency; the default mass equals frequency. Both dense targets and randomized target directions follow that convention. 

Pasted text +1

Literal-copy observation mapping: the code retains the two copy-specific cross-correlations, reconstructs the five copywise moment sums, performs the nonlinear inverse calculation per copy, and only then averages final inverse weights inside a stored row. 

Pasted text +2

Match contraction: the implemented 
F
g
	​

	​

-scaled projected and residual contractions equal the literal expanded-copy rank-one match directions. 

Pasted text +1

Low-dimensional joint-control factorization: the FWL Schur formulas, coefficient reconstruction, and full-system residual recomputation have the correct signs and dimensions. 

Pasted text +1

Accounting identity: total is constructed as worker plus firm plus twice covariance in the plug-in and correction paths; covariance target draws are derived probe by probe from the common total direction. 

Pasted text +1

Main result labels: the four columns and four rows of e(results), and the posting of the corrected row to e(b) and e(kss), agree with the documented contract. 

Pasted text +1

Residual denominator qualification

The code correctly uses a scale-relative denominator for each nonzero right-hand side and an absolute residual for a zero right-hand side. Batched columns cannot mask one another. 

Pasted text

The defect is not the formula for the residual ratio. It is treating a ratio as large as 10*tolerance, with tolerance allowed arbitrarily close to one, as sufficient for pathwise estimator invariance or reliable inverse action.

Noncritical findings
D1. Documentation mismatch: e(parameters) changes meaning in fixed-offset mode

In exact fixed-offset mode, parameters is reset to the pure worker–firm dimension after the full joint nuisance fit, and that reduced number is returned. 

Pasted text +1

JLA similarly reports base_parameters + cols(working_joint.controls); fixed-offset working_joint has zero control columns, although a full controlled fit was performed. 

Pasted text +1

The metadata documentation simply refers to “parameters” without defining whether that means full-fit or correction-system dimension. 

Pasted text

Classification: documentation/return mismatch. Post separate full_parameters and correction_parameters.

D2. Documentation mismatch: the failure catalog is not complete

Production paths can emit, among others:

INVERSE_RESIDUAL_FAILED; 

Pasted text

BLOCK_INVERSE_FAILED; 

Pasted text

CONTROL_SCHUR_RESIDUAL_FAILED. 

Pasted text

These labels are absent from the nominally complete failure catalog. 

Pasted text

Classification: documentation mismatch.

H1. Physical-copy allocation has no typed global size gate

Observation JLA constructs arrays of length sum(frequency) and each probe draws that many Bernoulli values. The only public size gates cover coefficient dimension and stored deletion-block size. Extreme but finite frequency totals can therefore produce a raw Mata allocation/runtime failure rather than a typed WITHHELD status. 

Pasted text +1

Classification: optional hardening, not false acceptance.

H2. Runtime token is semantic, not cryptographic

The version/API/build-token check correctly catches supplied stale-token states, but it cannot authenticate a modified Mata body that self-reports the expected token. Missing Mata files and some partial stale-namespace failures also exit outside the complete typed-withholding path. 

Pasted text

Classification: optional runtime hardening; not an estimator finding against the authenticated supplied source.

Conservative withholding confirmed

The following are conservative rather than critical defects:

AMBIGUOUS_PROBE_ORDER for tied ID-free keys spanning different controls, coordinates, or match blocks; 

Pasted text

AMBIGUOUS_LARGEST_COMPONENT for component-ranking ties; 

Pasted text

trace-certificate failure even where a direct exact design might remain full rank; 

Pasted text

fixed-offset deletion-rank certification even though controls are held fixed; 

Pasted text

finite-probe zero or nonpositive denominator events in an exactly estimable design. The packet correctly disclaims conditional unbiasedness after numerical selection. 

Pasted text

Required repairs
Repair 1: preserve control provenance and reject explicit zero columns

The caller must distinguish:

a column that Stata explicitly marks as an omitted/base factor-variable artifact; from

an explicitly requested numeric control that happens to be zero on the retained sample.

Only the first may be removed. An explicit zero or negative-zero control must be retained into the rank calculation or immediately return SINGULAR_NUISANCE_BLOCK.

The same rule must apply to one zero column included alongside otherwise valid controls. control_count, automatic exact/JLA selection, e(parameters), and deletion_rank_certificate must be based on all requested non-omitted columns.

Required regression tests:

zero and negative-zero control alone;

zero column plus two valid controls;

exact and JLA;

observation and match deletion;

joint and fixed-offset;

frequency two and literal expansion;

an actual omitted factor base that remains safely ignored.

Repair 2: make numerical inverse actions equivariant or weaken the contract explicitly

The present combination of encoded-firm grounding, adaptive per-column PCG stopping, and a 10*tolerance acceptance rule cannot support exact pathwise invariance.

A contract-preserving repair requires one of the following:

a permutation-equivariant quotient solver, such as a projected full-Laplacian solve under a symmetric normalization rather than grounding an encoded firm;

an ID-free canonical grounding rule, with withholding when the invariant grounding criterion ties;

basis-invariant block inverse actions with a common approximation operator rather than separately stopped scalar PCG solves; or

convergence to a registered operator/energy-norm or forward-error bound sufficiently strong that any remaining representation difference is explicitly bounded and reported.

Merely tightening the final residual from 10*tolerance to tolerance does not establish pathwise invariance, but the current permission of tolerance() values arbitrarily close to one must in any event be removed. At minimum, solver tolerance must be bounded by the rank and block margins and by a defensible forward-error certificate.

Required regression tests must freeze the conceptual sign stream and include:

the six-row K
2,3
	​

 firm-swap counterexample above;

every possible choice of grounded firm;

the packet fixture under T=(
1
−3
	​

1
1
	​

);

probes(2), default probes, and the largest allowed tolerance;

scalar and batched right-hand sides;

joint and fixed-offset nuisance modes;

weakly conditioned mobility systems.

An accepted run must either return the same result under these transformations at the stated contract precision or withhold with a typed numerical status.

Repair 3: repair secondary metadata

Post separate full-fit and correction-system parameter counts, complete the failure-label documentation, and add a typed physical-copy allocation limit.

Uncertainty

The zero-control defect is an exact source-level control-flow result and does not depend on any numerical emulation.

A Stata/Mata executable was not available in the audit environment. The two PCG counterexamples were therefore reconstructed independently from the supplied grouped-sum, Schur, PCG, joint-solve, moment, and target formulas. Their acceptance margins are wide: the largest reconstructed residual is about 0.0301 against an implemented gate of 0.9. The packet does not specify the seed-to-bit mapping of Mata’s pseudorandom generator, so the examples fix the realized Rademacher runtime state rather than assert a particular integer seed. That is sufficient under the question’s explicit “finite design or runtime state” quantifier and the packet’s declared copywise Rademacher law.

I found no additional algebraic sign, coefficient, normalization, copy-mapping, target-contraction, or deleted-scatter error after a complete nonzero control matrix reaches Mata. I do not claim a uniform floating-point proof over every extreme finite input.

KB5 closure decision

Do not close KB5.

The exact caller counterexample alone proves that the public command can post a point estimate outside the requested full-model contract. The JLA counterexamples independently prove that the new conceptual-copy ordering does not deliver the promised whole-command invariance because accepted approximate inverse actions remain dependent on encoded grounding and control basis.

Critical objection remains before KB5 closure.

