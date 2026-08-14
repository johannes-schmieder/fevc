---
review_id: KSS-BC-API10-FINAL-I
reviewer_surface: ChatGPT web through @Chrome
model_picker_label: Pro
date: 2026-08-14
packet_sha256: 6eaf7d1fa3f2efe7b2f77321f2e6d63affe8a8fc2e5689709be8c9b6e7e1ecb9
verdict: "false"
review_status: ai_reviewed_once
safe_chat_reference: https://chatgpt.com/c/6a7f190b-dce8-83ea-842d-1faf28406180
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

FALSE. A critical objection remains. KB5 must not close.

The API-10 repairs correctly address the two advertised API-9 counterexample classes: the literal-copy fixed-seed stream is now representation-invariant on accepted, nonambiguous inputs, and the deterministic within-cell certificate prevents the six-cycle fixed-offset singular-control examples from being accepted. The copywise JLA algebra, coefficient-one fourth moment, block-control projection, inverse-adjustment signs, target normalization, and accounting identity also check out.

However, the public ado layer has an earlier rank-gate bypass: it silently removes every all-zero expanded control column, not merely factor-variable columns that Stata marks as omitted. Consequently, an explicitly requested zero control—and an all-zero control created by an invertible transformation of a redundant control basis—disappears before either backend sees it. The command can therefore post an accepted exact or JLA result although the requested full worker–firm–control design is singular. This contradicts the governing requirement that both nuisance conventions require an identified full design and that false acceptance or undisclosed rank repair is prohibited. 

Pasted text

Closure decision: KB5 = NOT CLOSED.

1. Packet authentication and review boundary

I recomputed the hashes of all 31 embedded manifest bodies, normalizing each delimited body to one trailing line feed. Every body matched its entry in MANIFEST.sha256, including PROMPT.md, REQUEST.yaml, and SOURCE_MAP.md. The manifest identifies the governing specification, production ado/Mata files, documentation, and validation sources. 

Pasted text

 The source map assigns the production caller to kss_bc.ado, the exact/JLA implementation to kss_bc.mata, and the regression fixtures to their stated roles. 

Pasted text

The outer deterministic ZIP bytes were not present in this text transport, so I could not independently recompute the stated outer ZIP SHA-256. As the packet directs, that is only a transport limitation and not an estimator finding. 

Pasted text

I also independently ran the four self-contained Python algebra/oracle modules covering dense leave-out identities, copywise frequency probes, JLA moments, and block controls: 27 tests passed. I did not treat that as proof. No Stata executable was available, so the .do files were audited as source rather than accepted as execution evidence.

2. Critical finding
C1 — Explicit zero controls are silently deleted before all full-rank gates

Severity: Critical
Affected paths: exact and JLA; joint and fixed-offset; observation and match deletion; explicit and auto algorithm selection.

Production behavior

After fvrevar, the ado loops over every expanded control and retains it only if at least one requested-sample value is nonzero:

stata
foreach control of local expanded_controls {
    quietly count if `control' != 0 & `touse'
    if r(N) local controlvars `controlvars' `control'
}

The accompanying comment explains the intended reason—factor-variable omitted bases are materialized as zero columns—but the code does not determine whether a zero column actually came from an omitted factor base. An explicitly supplied numeric control that is identically zero is treated in exactly the same way and silently disappears. No rank failure or warning is posted. 

Pasted text

The reduced controlvars list then determines the reported control count, parameter dimension, auto-algorithm choice, probe tie checks, and the controls sent to Mata. 

Pasted text +2

Therefore:

In the exact backend, the zero column is absent when full_design and the purported full information matrix are constructed. The fixed-offset branch is entered only when the reduced controls_count is positive. 

Pasted text +1

In JLA, full_joint is prepared from the reduced controls. The deterministic full-fit/deletion certificate is invoked only when the reduced controls_count > 0; the fixed-offset nuisance extraction has the same condition. 

Pasted text +1

Direct counterexample

Take any accepted packet fixture and add:

stata
generate double z0 = 0

Then insert z0 into the public control list. For example, modify the registered accepted JLA call to:

stata
kss_bc y z0 c1 c2, worker(worker) firm(firm)             ///
    deletion(match) deletionid(match)                    ///
    algorithm(jla) nuisance(joint)                       ///
    probes(2000) seed(20260814) tolerance(1e-12) nodisplay

The packet’s baseline call uses the same data and c1 c2 controls. 

Pasted text

 The ado removes z0, so every subsequent sample, graph, control matrix, probe order, random stream, solve, contraction, and posted estimate is the same as in that baseline call.

But the requested public design is

X
requested
	​

=[DFz
0
	​

c
1
	​

c
2
	​

],z
0
	​

=0,

so X
requested
′
	​

WX
requested
	​

 has an identically zero row and column and is singular. The governing estimator contract explicitly defines the model using the supplied Z and uses H
−1
, not an implicit column-space quotient for nuisance controls. 

Pasted text

 The control derivation likewise requires a positive-definite control Schur complement and expressly disclaims generalized-inverse or ridge handling of a failed nuisance block. 

Pasted text

The same bypass applies to:

the registered exact joint and fixed-offset fixtures; 

Pasted text

JLA fixed-offset; 

Pasted text

observation deletion; 

Pasted text

a command whose only control is zero, in which case JLA reports a no-control FE certificate and never invokes the control rank certificate.

Invertible control-basis violation

The same bug creates a literal basis-invariance counterexample.

Start with two duplicate requested controls,

z
1
	​

=z,z
2
	​

=z.

The two-column nuisance parameterization is singular and, because both columns are nonzero, the original basis reaches the rank gate and is withheld. Now use the invertible transformation

u=z
1
	​

−z
2
	​

=0,v=z
1
	​

+z
2
	​

=2z,

whose transformation matrix has nonzero determinant. In the transformed basis, the ado silently removes u and sends only v to Mata. An otherwise accepted one-control path can then post estimates. Thus two invertible parameterizations of the same requested control system can produce:

original basis: WITHHELD;

transformed basis: accepted point estimates.

That is not merely conservative withholding. It is false acceptance of the transformed, still-singular public design and directly contradicts the required control-reparameterization invariance. 

Pasted text

Required repair

The caller must distinguish semantically omitted factor-variable bases from explicit zero-valued controls.

A conforming repair should:

Exclude only columns that Stata’s factor-variable metadata explicitly marks as omitted/base artifacts.

Retain every explicitly requested numeric control, including an all-zero one, so the full-rank gate can reject it.

Avoid _rmcoll or any other silent rank-reducing rewrite; those would simply generalize the same contract violation.

Add registered exact and JLA tests for:

a lone explicit zero control;

a zero control added to an otherwise accepted controlled fixture;

duplicate controls before and after the invertible (z
1
	​

,z
2
	​

)↦(z
1
	​

−z
2
	​

,z
1
	​

+z
2
	​

) transformation;

both nuisance(joint) and nuisance(fixedoffset);

preservation of legitimate factor-variable omitted-base handling;

algorithm(auto), because the silent deletion also changes the parameter count and can change backend selection.

Alternatively, the owner would have to authorize a materially different contract defined on a canonical basis of the nuisance column span. That would require a representation-invariant rank-revealing reduction for all redundant nuisance columns, explicit disclosure, and revised specifications. The present packet authorizes none of those changes.

3. Reconstruction of the accepted production paths

Subject to the C1 upstream defect, the remaining production flow is internally coherent.

3.1 Public inputs, sample, runtime, and graph selection

The ado validates the deletion, algorithm, nuisance, stayer, probe, solver, rank, and block options before sample construction. It enforces positive integer frequency weights and nonnegative target weights with positive mass. Default target mass equals frequency, while an explicit target weight is treated as complete stored-row mass. 

Pasted text +1

Match deletion freezes all requested inputs and rejects any incomplete requested row rather than silently modifying a declared block. Observation deletion uses complete cases. 

Pasted text

Before either backend runs, the caller verifies API level, version, and the semantic build token. A loaded same-level runtime with another build ID is withheld rather than used. 

Pasted text

The graph path then:

selects the component with the greatest firm count and then greatest physical mass;

withholds exact ties;

restricts match headlines to movers;

iteratively removes worker articulation vertices and reselects the largest component;

removes workers with insufficient fitted-firm or physical-observation histories;

re-encodes the retained quotient. 

Pasted text

The component ranking uses only firm count and frequency mass. Its tie status is explicit, so it does not select a different sample under pure ID relabeling. 

Pasted text

After graph selection, the caller computes the worker, firm, deletion-unit, and parameter counts and chooses exact versus JLA under algorithm(auto). 

Pasted text

3.2 Exact × joint × observation

The exact backend constructs all worker indicators, all but one firm indicator, and the reduced control list, forms weighted information, passes it through the equilibrated spectral and inverse-residual gate, and computes the full coefficient vector. 

Pasted text

Under joint nuisance treatment, the working design remains the complete design. Targets are then built with zero action on the control coordinates, and all four plug-ins are obtained from the same target-weight normalization. 

Pasted text

For observation deletion, each physical copy has leverage x
r
′
	​

H
−1
x
r
	​

. The code requires 1−h
r
	​

 to exceed the block tolerance, directly factors the one-copy deleted information near a numerical boundary, and multiplies the stored-row correction by its frequency. Thus a frequency-f
r
	​

 row contributes f
r
	​

 identical physical-copy terms rather than one weighted-row deletion. 

Pasted text

3.3 Exact × joint × match

For each declared match, the code builds the compressed physical-copy block with rows 
f
r
	​

	​

x
r
	​

, forms P
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

, checks I−P
g
	​

, and applies the block Woodbury residual identity. Controls may vary across stored rows in the match; no control mean-row substitution occurs. 

Pasted text

This agrees with the packet’s compression argument: within-copy contrast directions have identity residual-maker action and zero contraction against block-constant outcomes and residuals, so the compressed solve is isometric to literal expansion. 

Pasted text

3.4 Exact × fixed-offset × observation or match

The exact path first fits the complete joint model and obtains the full-sample nuisance coefficient. It then forms y
∗
=y−Z
γ
	​

, replaces the working design by the pure two-way FE design, and runs the same observation or match deletion algebra while holding the nuisance index fixed. 

Pasted text

The resulting convention is therefore conditional fixed-offset estimation, not joint leave-out nuisance re-estimation, as required. 

Pasted text

3.5 JLA common fit and full-rank gates

The matrix-free base service eliminates the diagonal worker block and applies PCG to the grounded firm Schur operator. It reconstructs worker coefficients and recomputes the residual under the original full two-way normal-equation operator before accepting a solve. 

Pasted text

 Multiple right-hand sides are solved and gated one column at a time, so a large neighboring column cannot hide a failed column. 

Pasted text

Joint controls use

U=R
A
	​

Z,S=U
′
WU,

through a low-dimensional FWL Schur complement. The code separately compares the cross-product Schur calculation with U
′
WU, inverses the low-dimensional block, and then recomputes the residual under the full joint system for every right-hand-side column. 

Pasted text +1

For every surviving JLA control list, including fixed-offset, the deterministic rank certificate is called before the outcome point estimate can be posted. It explicitly centers controls within worker–firm cells, whitens the within-cell scatter, computes the nonnegative scatter removed by each declared deletion, directly eigendecomposes and inverses every deleted whitened scatter, and requires a positive buffered gap. 

Pasted text +1

Under fixed-offset, the full joint solve is completed first, the fitted nuisance index is removed, and the working system is then replaced by the pure FE service. 

Pasted text

The defect is that all of these gates operate only on the controls that survive the ado’s zero-column deletion.

3.6 JLA × observation

For every leverage probe, the code draws one Rademacher sign for every physical copy, aggregates copy signs to stored-row sums for the FE solve, and retains the individual physical signs needed for the two copy-specific cross-correlations. 

Pasted text

It reconstructs, for each copy,

∑U,∑V,∑U
2
,∑V
2
,∑UV

from the common stored-row projection sums and the copy-specific first- and third-order correlations. The nonlinear constrained ratio and B,V adjustment are formed copy by copy. Only the final inverse multipliers are averaged within a stored row. 

Pasted text

Exact residualized-control leverage is subtracted from the FE residual share before applying

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

.

The signs match the reciprocal delta expansion. 

Pasted text

3.7 JLA × match

For each declared match, the FE probe contractions are

π
g
	​

=F
g
−1/2
	​

r∈g
∑
	​

f
r
	​

p
	​

r
	​

,
μ
	​

g
	​

=F
g
−1/2
	​

r∈g
∑
	​

S
r
	​

−
π
g
	​

.

The code uses exactly these contractions. 

Pasted text +1

It forms the estimated rank-one FE block, adds the full residualized-control projection U
g
	​

S
−1
U
g
′
	​

, checks and inverses the complete residual-maker block, and applies the delta adjustment

M
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

The implemented +B and −V signs are correct. 

Pasted text +1

3.8 Four target contractions and posting

The second pass draws fresh physical-copy signs and constructs the centered target direction using explicit stored-row mass divided among its copies. Its covariance is the target centering matrix diag(s)−ss
′
. 

Pasted text +1

Worker and firm target factors are solved jointly with zero nuisance RHS. Total projection is their sum, and covariance is defined probe by probe as

2
total−worker−firm
	​

.

Consequently, both the correction and its numerical MCSE obey the total accounting identity mechanically. 

Pasted text

The ado posts plugin, correction, corrected, and numerical-MCSE rows; e(b) and e(kss) contain corrected estimates, and no e(V) is posted. 

Pasted text +1

4. Required adversarial attacks
4.1 Six-copy probes(2) frequency split/regroup attack

Result: repaired for accepted paths.

The API-10 primary key is outcome plus per-copy target mass. It excludes encoded worker, firm, match, frequency, stored-row labels, and raw control coordinates. A tied primary-key group is accepted only if controls, worker–firm coordinate, and—under match deletion—deletion block all agree. 

Pasted text

For the packet’s six-copy fixture, a frequency-two row and its two literal rows therefore consume the same two conceptual copy positions. Observation JLA retains the two correlation sums for each copy and performs every nonlinear operation before regrouping. 

Pasted text

 The registered fixture compares the frequency-two representation with literal expansion at R=2. 

Pasted text

I independently enumerated all 64
2
=4096 two-probe Rademacher pairs for that six-copy design. The largest discrepancies between literal expansion and compressed formulas were:

projected-copy values: 2.22×10
−16
;

centered target directions: 5.55×10
−17
;

the five reconstructed copywise moment sums: 7.11×10
−15
.

These are floating-point accumulation differences, not finite-projection algorithm differences. The API-9 pooled-residual-ratio counterexample is closed.

4.2 Row order and batch size

Result: repaired for accepted paths.

The ado re-sorts to the invariant primary key before the random stream begins. Within an accepted tie class, rows are identical in all nonexchangeable semantics, so changing their order merely permutes signs among exchangeable physical copies. Sequential RNG consumption makes the logical stream independent of batch(). 

Pasted text

The code’s batched solver also solves each column separately and records the maximum column residual, so batching cannot alter acceptance through a pooled Frobenius test. 

Pasted text

4.3 Worker and firm ID relabeling

Result: repaired for the estimator stream and graph selection.

The random ordering does not use encoded IDs. The graph criterion uses firm count and physical mass and withholds ties instead of breaking them by encoded values. 

Pasted text

 The omitted-firm quotient changes under a relabeling, but target quadratic forms and fitted projections are invariant to the normalization. The packet’s source tests exercise both exact and JLA relabeling. 

Pasted text +1

4.4 Control-basis transformation

Result: locally correct on retained, full-rank controls; globally false because of C1.

For an identified control matrix Z and nonsingular T,

U
∗
=UT,S
∗
=T
′
ST,U
∗
(S
∗
)
−1
U
∗
′
=US
−1
U
′
,

so the fitted control projection, block control action, fixed-offset index, generalized trace losses, and target estimates are algebraically invariant. The production block split implements this projection. 

Pasted text

The packet’s trace test also checks one ordinary nonsingular transformation. 

Pasted text

But the governing universal claim fails when the requested controls are redundant: an invertible transformation can expose an all-zero column, which the ado removes. C1 therefore defeats the overall control-basis verdict even though the downstream FWL algebra is correct.

4.5 Tied invariant keys

Result: correct fail-closed behavior.

A tied key spanning different controls, worker–firm coordinates, or match blocks returns AMBIGUOUS_PROBE_ORDER; exact mode remains available. 

Pasted text

 The supplied fixture confirms that exact mode can evaluate a tied design while JLA withholds rather than sort by IDs. 

Pasted text

Within an accepted tie class, physical copies are exchangeable for the fit, deletion calculation, and target contractions. Permuting or repartitioning those copies does not change any summed correction.

4.6 Six-cycle fixed-offset singular-control attacks

Result: the advertised API-9 counterexample is closed.

In both six-cycle constructions, the control is constant within every worker–firm cell and lies exactly in the firm-FE span:

tiny scale: z=10
−10
×(1,2,2,0,0,1);

ordinary scale: firm-specific values with δ=10
−5
, tested with solver tolerance 10
−4
. 

Pasted text +1

I independently formed both designs. Their smallest singular values were approximately 1.99×10
−17
 and 4.91×10
−17
, respectively, while their within-cell control scatters were exactly zero.

For JLA, the deterministic certificate uses the original controls—not the approximate FE residuals—and therefore encounters singular within-cell scatter before any leverage realization or point estimate can authorize a nuisance dimension. 

Pasted text

 Because the certificate is called immediately after full-joint preparation and before the outcome solve, loosening PCG tolerance cannot bypass it. 

Pasted text

For exact calculation, the complete dense information matrix includes the control and fails the equilibrated spectral gate. 

Pasted text

Thus no approximate solve can manufacture acceptance in the supplied six-cycle classes. C1 is a distinct pre-gate bypass: the six-cycle controls are nonzero and therefore survive the ado preprocessing.

5. Algebraic verification
Obligation	Verdict	Basis
Literal physical-copy probe law	Pass	One sign is drawn per physical copy, stored rows receive sign sums, and copywise state is retained where nonlinear operations require it. 

Pasted text


Observation sufficient statistics	Pass	The implemented A
2
	​

,A
4
	​

,C
1
	​

,C
3
	​

 formulas reproduce all five literal-copy raw sums. 

Pasted text +1


Coefficient-one mixed fourth moment	Pass	The delta bias has one, not two, copies of (M−P)m(P,M); production uses that expression. 

Pasted text +1


Constrained P/M ratio	Pass	Projection and residual moments are divided by their common positive sum; failed denominators withhold the entire command. 

Pasted text


Match FE contraction	Pass	The 
F
g
	​

	​

-normalized projection and residual contractions equal literal-copy block directions. 

Pasted text


Block-control projection	Pass	P
[A,Z]
	​

=P
A
	​

+US
−1
U
′
, with the complete varying-control block retained. 

Pasted text


Observation inverse signs	Pass	1/
m
+B/
m
2
−V/
m
3
. 

Pasted text


Match inverse signs	Pass	Base inverse +Bg
′
−Vg
′′
/2. 

Pasted text


Explicit target normalization	Pass	Stored mass t
r
	​

 is divided among f
r
	​

 copies and generates diag(s)−ss
′
. 

Pasted text


Zero target action on controls	Pass downstream	Target matrices place worker/firm/covariance action only in FE coordinates. 

Pasted text


Total accounting	Pass	Plug-in total is constructed as worker + firm + 2 covariance; target-probe covariance is derived from the same total draw. 

Pasted text +1


Full control rank on every public path	Fail	Explicit zero controls can be removed before exact or JLA rank validation. 

Pasted text

6. Noncritical findings and classifications
Conservative withholding, not defects

Component ties. Withholding when firm count and physical mass tie is necessary to avoid an encoded-ID sample choice. 

Pasted text

Probe-order ties. Withholding nonexchangeable identical-key rows is conservative but complies with the pathwise fixed-seed requirement. 

Pasted text

Within-cell control certificate. Positive within-cell scatter is a sufficient, not necessary, control-rank condition. Valid controls identified only through between-cell nonadditive variation can be withheld. That is disclosed. 

Pasted text

Fixed-offset deletion-rank check. Requiring deleted control scatter to remain full rank even though fixed-offset deletions do not re-estimate controls is stronger than necessary but explicitly authorized as conservative. 

Pasted text

Finite-probe boundary events. An exactly estimable deletion can receive zero or nonpositive estimated residual leverage and be withheld. The implementation makes no conditional-unbiasedness claim after gate selection. 

Pasted text

Optional numerical hardening

The computed finite-projection variance is algebraically a nonnegative empirical square, but production evaluates it through a cancellation-prone combination and then truncates small negative values to zero. A more robust implementation would accumulate the corresponding squared quantity directly or apply a scale-relative negativity gate. This is not the present closure blocker. 

Pasted text

Metadata/documentation repairs

In exact fixed-offset mode, out.information_rcond is the minimum of the full-joint and pure-FE working reciprocal condition numbers. 

Pasted text

 The documentation instead describes e(information_rcond) as the reciprocal conditioning of the complete identified information matrix. 

Pasted text

 Either the field or its documentation should be renamed or clarified.

Likewise, exact fixed-offset reports the pure-FE working parameter count rather than the dimension of the full model that was fitted to obtain the nuisance index. 

Pasted text

 This is a metadata-definition issue, not an error in the four point estimates.

Outside the KB5 estimator verdict

The packet explicitly excludes econometric inference, finite-R exact unbiasedness, uniform asymptotics, licensing, restricted-data qualification, and public release. 

Pasted text

 Package unification is also intentionally deferred by the owner decision, despite the longer-term package specification. 

Pasted text

 Neither issue changes the finite-estimator finding above.

7. Final line-specific verdict

KSS-BC-API10-FINAL-I verdict: FALSE.

The six-copy finite-probe regrouping attack is repaired.

Row order, batch size, ID relabeling, and nonambiguous copy regrouping are pathwise invariant under the implemented stream.

The six-cycle tiny/default and ordinary/loose fixed-offset singular-control attacks fail closed.

The copywise moments, coefficient-one term, block-control projection, inverse signs, target normalization, and accounting identity are correct.

But an explicit all-zero control is silently removed in kss_bc.ado before the full-design rank gates, and an invertible transformation of duplicate controls can therefore change a singular public fit from withholding to acceptance. 

Pasted text

Critical objection remaining before KB5 closure: YES.
KB5 closure decision: DO NOT CLOSE until C1 is repaired and registered across exact/JLA, joint/fixed-offset, observation/match, and auto-dispatch paths.

