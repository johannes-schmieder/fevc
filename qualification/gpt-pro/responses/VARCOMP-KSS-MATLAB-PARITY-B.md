---
review_id: VARCOMP-KSS-MATLAB-PARITY-B
reviewer_surface: ChatGPT web through Chrome and native Computer Use upload
model: GPT-5.6 Sol
model_effort: Pro
review_date: 2026-08-18
source_commit: a405e7652da9a6657045090963b8b1ec5835a488
packet_sha256: 0a8dc8e4f81f1ea50de3e53b4c888cc3ee3c9ff83ffe47a03b26e058c4d7a1e2
chat_url: https://chatgpt.com/c/6a84f695-a798-83ea-9fe8-3d01d1d23745
review_status: completed_independent
verbatim_body_sha256: 7641683d665edde783d1e666854cd1cdc87ae639e599f3afb4207e51cd4fdffa
transport_note: The direct Chrome-extension chooser timed out; the same signed-in Chrome session uploaded the exact packet through the native macOS picker. Two extension connection errors were logged at upload time, but the attachment remained visible and the reviewer reported that the manifest and commit verified.
---

## Verbatim response

Independent review B — numerical architecture path to MATLAB parity
Scope, integrity, and evidence notation

I reviewed only the supplied packet. I verified every entry in MANIFEST.sha256 with sha256sum -c; all files passed. The request binds the review to commit a405e7652da9a6657045090963b8b1ec5835a488 and explicitly excludes restricted rows, licensed MATLAB source, raw scheduler logs, and the other independent review. See REQUEST.yaml:L1-L18 and SOURCE_MAP.md:L3-L8.

I use three labels throughout:

[M] Measured evidence: timings, counts, and validated receipts in packet reports.

[D] Source deduction: operation count or behavior derived directly from packet source.

[S] Speculation/design forecast: a falsifiable performance hypothesis, not a measured speedup.

I use these contract classes for proposed changes:

E — exact-path preserving: can preserve current atoms, logical ordering, floating-point reduction sequence, statuses, and posted values bit for bit.

T — tolerance preserving: changes the finite-precision path, but can preserve the registered estimator, RNG law and stream, probe/batch invariance, requested tolerance, complete per-RHS certificate, and typed failures.

R — reject or isolate: changes a public scientific or failure contract.

1. Independent verdict and complexity account
Verdict

The performance gap has three different causes, each governing a different regime:

The matched CZ18 P20 cold-command gap is primarily a boundary and preprocessing gap.
[M] Of the 305-second Stata command, 188.272 seconds—61.7%—are in import/selection, while hierarchy setup is only 6.442 seconds. MATLAB’s matched command is 43.802 seconds, giving the registered 6.96 ratio. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L288-L319.

[D] The remaining post-selection, post-hierarchy work is approximately 110.286 seconds. Therefore:

making the entire numerical core instantaneous would improve the 305-second cold command by at most 1.57×;

eliminating all selection and hierarchy time would still require the remaining core to improve by 2.52× to reach 43.802 seconds;

retaining 10% of selection time and the current hierarchy time would require the remaining core to improve by about 5.95×.

Consequently, cold one-shot parity cannot come from PCG or CMG optimization alone. A reusable prepared boundary, or a major redesign of selection/compression, is necessary for that comparison.

Ordinary CMG cells are governed mainly by repeated sparse traversal, ordering, allocation, and Mata bulk-array throughput—not hierarchy construction.
[M] At 1,024 firms, setup medians range from 0.351 to 4.324 seconds, while PCG reaches 27.878 seconds and leverage/target stages reach 19.348/23.320 seconds. Cell-specific Stata/MATLAB ratios range from 0.65 to 4.38, with median 2.59. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L217-L244.

[M] Holding all 54,074 RHS-equivalent actions fixed, changing batch() from 1 to 16 reduced numerical work from 42.796 to 16.418 seconds, a 2.607× range with unchanged accepted results and certificates. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L69-L95. This is strong evidence of fixed per-matrix-call costs and data-motion/allocation effects beyond mathematical action count.

The old density-four cliff was algorithmic, but the packet already contains a strong candidate remedy that is not yet production-qualified.
[M] The old 625,000-worker degree-four case spent 8,755.873 of 9,610 seconds in hierarchy setup but converged in 15 iterations; degree three took 35.478 seconds of setup. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L206-L230.

[M] The API-6 numerical hierarchy subsequently reduced the 15,625-firm degree-four fixture from 8,755.873 to 40.293 seconds locally, a 217-fold setup improvement. Its SCC hierarchy ladder has degree-four setup of 1.147, 7.847, and 13.148 seconds at 1,024, 4,096, and 10,240 firms, with endpoint slopes near 1.06 and no degree-five-to-seven cliff. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L36-L72 and L138-L168.

The current packet exposes that numerical core as API 7, but public/production release remains disabled; API 7 changes ownership and callable surface, not numerical construction or V-cycle algebra. FILES/varcomp_kss/cmg/STATUS.md:L3-L16; FILES/varcomp_kss/cmg/docs/API_CONTRACT.md:L21-L47.

The requested pure-Mata path should therefore be a portfolio:

qualify and promote the robust hierarchy;

build a plan-once, stream-many sparse execution architecture;

expose an optional prepared-data lifecycle for repeated commands;

reduce weak-graph action counts with deterministic coarse seeding/deflation, followed only later by canonical block or recycled Krylov methods;

calibrate batching and route selection from deterministic work, never post-RNG wall time.

No single item plausibly closes the matched 6.96 cold-command ratio.

Symbols used in the complexity account

Let:

R: stored rows entering command selection;

N: retained generic rows;

N
ϕ
	​

: literal physical copies when observation deletion requires copywise state;

C: compressed worker–firm coefficient cells;

W,F: retained worker and firm levels;

G: deletion units;

S: target strata;

k: control columns;

P: requested probes;

B
L
	​

,B
T
	​

: leverage and target batch widths;

p=W+F−1+k: exact-route parameter count;

V
ℓ
	​

,E
ℓ
	​

: vertices and edges at CMG level ℓ;

L: hierarchy levels;

i
j
	​

: iterations for RHS j;

A=∑
j
	​

i
j
	​

: RHS-equivalent Krylov actions;

Q: physical matrix-action calls over batches and iterations.

e(schur_actions) records the RHS-equivalent count, while e(schur_batches) records physical matrix calls. The distinction is part of the registered diagnostics. FILES/varcomp_kss/docs/FAILURES_AND_RETURNS.md:L25-L50.

Exact route: data flow and governing work

The auto dispatcher selects exact when p\leq\texttt{exact_limit()}. FILES/varcomp_kss/varcomp_kss.ado:L560-L574.

Exact data flow

Dense design construction.
vckss__design() materializes the N×p indicator/control design.
FILES/varcomp_kss/varcomp_kss.mata:L658-L681.

Information matrix and inverse certification.
The backend forms H=X
′
WX, equilibrates it, obtains eigenvalue rank and conditioning diagnostics, calls invsym(), and recomputes the inverse residual.
FILES/varcomp_kss/varcomp_kss.mata:L315-L358, vckss__inverse().

Fit and optional fixed-offset preliminary fit.
The exact driver forms the full fit, and under fixed offset performs the additional full joint fit before the pure-FE calculation.
FILES/varcomp_kss/varcomp_kss.mata:L747-L927, vckss__exact().

Dense target algebra.
vckss__targets() materializes three p×p targets. The route computes design*A, plug-in targets, and observation- or match-level target contractions.
FILES/varcomp_kss/varcomp_kss.mata:L684-L730, vckss__targets(); L929-L1049, vckss__exact().

Deletion correction.
Observation deletion uses per-row leverage and direct deleted-information checks near the numerical boundary. Match deletion constructs block factors and uses the dimension-reduced residual-maker action in vckss__low_rank_maker().
FILES/varcomp_kss/varcomp_kss.mata:L361-L447; L951-L1049.

Complete residual and rank gates.
The exact route recomputes full inverse/block residuals and factors deleted information directly near the forward-error boundary; it never silently substitutes a generalized inverse or ridge.
FILES/varcomp_kss/docs/NUMERICAL_ARCHITECTURE.md:L93-L122.

Exact complexity

[D]

T
exact
	​

=Θ(Np
2
+p
3
)+Θ(
g
∑
	​

m
g
	​

p
2
)+T
small deletion makers
	​

,

with storage

M
exact
	​

=Θ(Np+p
2
),

where m
g
	​

 is the number of stored rows in deletion block g.

For observation deletion, the current target-diagonal route performs three dense N×p by p×p actions through vckss__target_diagonal(), so target correction is another approximately 3Np
2
 term. FILES/varcomp_kss/varcomp_kss.mata:L740-L745.

The exact route is therefore governed by Np
2
 near exact_limit(), not by probe count. It is not the primary CZ18/P200 parity problem, but exploiting target structure can materially improve near-boundary exact workloads.

JLA route: data flow and governing work
1. Sample selection, semantic ordering, and compression

The ado layer freezes variables, applies the graph/deletion fixed point, constructs dense IDs, and creates the semantic JLA order from model coordinates, target mass, outcome, controls, and optional probeorder(). FILES/varcomp_kss/varcomp_kss.ado:L540-L597.

The compressed constructor then:

canonicalizes worker, firm, and deletion IDs;

orders rows into worker–firm cells;

constructs cell sufficient statistics;

verifies each deletion unit lies in one coefficient cell;

constructs exact target strata;

constructs worker- and firm-major orders, panels, masses, and the Schur diagonal.

See FILES/varcomp_kss/varcomp_kss_scale.mata:L479-L705, vckss_scale__prepare().

[D] Compression is approximately O(RlogR) because of canonical grouping and comparison ordering, followed by O(R) aggregation. Its persistent compressed inventory is explicitly modeled as

8(9C+3W+4F)+32G+32S

bytes, excluding hierarchy, phase scratch, transition high-water, and restoration. FILES/varcomp_kss/docs/NUMERICAL_ARCHITECTURE.md:L23-L30.

2. Schur elimination

For the two-way FE system, worker coordinates are eliminated to form the firm mobility Laplacian

S
F
	​

=F
′
WF−F
′
WD(D
′
WD)
−1
D
′
WF.

The solve remains on the full-firm zero-sum quotient and grounds only after convergence. FILES/varcomp_kss/docs/NUMERICAL_ARCHITECTURE.md:L124-L164.

A Schur action performs grouped worker and firm reductions over the compressed cells. For b columns:

T
S
	​

(b)=Θ(b(C+W+F)),M
S
	​

(b)=Θ(b(C+W+F)).

The compact view provides transpose, prediction, Schur, diagonal, and reconstruction callbacks without rebuilding the generic design. FILES/varcomp_kss/varcomp_kss_scale.mata:L1089-L1144.

3. CMG construction

The current robust source:

builds a canonical heaviest-neighbor profile;

selects and cuts a forest;

applies the one-eighth weak-group repair;

assigns component-contained aggregates;

contracts exact positive Galerkin edges;

uses a normalized fallback and a retained dense-quotient specialization.

The main aggregation is in vckss_cmg__aggregate_gpl(), FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L1097-L1270. Level selection, complexity gates, fallback, and exact contraction are in vckss_cmg__hierarchy_mode(), L2527-L2795.

The preflight predicts degree-two, degree-three, and degree-four-plus hybrid edge/auxiliary counts and applies explicit linear scratch and structural-byte gates before allocation. FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L795-L864, vckss_cmg__preflight().

[D] The current contraction uses comparison ordering of quotient-edge keys, so a conservative source-level build account is

T
build
	​

=O(
ℓ=1
∑
L−1
	​

(E
ℓ
	​

logE
ℓ
	​

+V
ℓ
	​

logV
ℓ
	​

))

plus linear aggregation and certification passes. The registered edge/vertex-complexity caps and memory forecasts keep admitted work finite; the packet’s measured endpoint behavior is near linear on the tested graph families, but the source does not establish a universal linear theorem.

4. CMG application

A V-cycle performs smoothing, a graph action, residual restriction, recursive coarse solve, prolongation, a second graph action, and post-smoothing. FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L2929-L2964, vckss_cmg__apply_level().

For b columns,

T
M
	​

(b)=Θ(b
ℓ=1
∑
L
	​

(E
ℓ
	​

+V
ℓ
	​

))+T
terminal
	​

(b).

The terminal setup stores bounded dense Cholesky factors; application uses triangular solves. The dense terminal is capped at 6,144 vertices and admitted against the memory envelope. FILES/varcomp_kss/cmg/docs/API_CONTRACT.md:L35-L47.

5. Lockstep PCG

vckss__fe_solve_matrix_backend():

forms the reduced zero-sum RHS;

applies one preconditioner action to the whole matrix;

runs independent scalar PCG recurrences for each column;

shares matrix traversal across active columns;

recomputes the true quotient residual every 100 iterations;

zeros inactive columns;

grounds firms and reconstructs workers;

recomputes the complete original worker-plus-firm residual for every RHS.

FILES/varcomp_kss/varcomp_kss.mata:L1631-L1975.

This is batched scalar PCG, not block CG: the columns share traversal, but their Krylov spaces and scalar recurrences are independent.

A useful measured-time model is

T
PCG
	​

≈(α
S,0
	​

Q
S
	​

+α
S,1
	​

A
S
	​

)(C+W+F)+(α
M,0
	​

Q
M
	​

+α
M,1
	​

A
M
	​

)
ℓ
∑
	​

(E
ℓ
	​

+V
ℓ
	​

).

The A terms capture mathematical per-column work. The Q terms capture repeated allocation, dispatch, panel metadata, and matrix-call overhead. The fixed-action batch experiment demonstrates that the Q-dependent part is material.

6. Control preparation

With k>0, vckss__joint_prepare() performs k FE inverse actions to residualize controls, forms the k×k Schur complement, certifies its inverse, and records complete preparation residuals. FILES/varcomp_kss/varcomp_kss.mata:L2048-L2136.

Each later joint solve performs the FE inverse action, applies the small control correction, reconstructs the full fitted values, and recomputes the full joint-system residual. FILES/varcomp_kss/varcomp_kss.mata:L2176-L2272.

7. Leverage probes

For each leverage batch, the compressed route:

obtains G×B
L
	​

 literal-frequency atom sums;

scatters deletion-unit atoms to cells;

transposes to FE RHSs;

solves B
L
	​

 inverse actions;

gathers predictions back to deletion units;

accumulates five finite-projection moments with fixed eight-probe compensated tiles.

FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L1165-L1235, vckss_scale_eng__run_prepared(); the finite-projection identities and prohibition on early physical-copy pooling are documented in FILES/varcomp_kss/docs/JLA_FINITE_PROJECTION.md:L58-L138.

The leading non-solver leverage work is

Θ(P(G+C))+Θ(PG)

plus P certified inverse actions.

8. Correction construction

After leverage moments, the route forms the nonlinear finite-projection correction for every deletion unit. The current compressed match path calls vckss_scale_eng__unit_adjust() in a scalar loop over G, then scatters the unit correction weights to cells. FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L1224-L1307.

This is O(G+C) after the leverage sketch. For observation deletion, copywise nonlinear operations can require O(N
ϕ
	​

) state and may govern memory; pooling physical copies before the nonlinear operation would change the estimator and is prohibited.

9. Target probes and contractions

For each target batch, the route:

obtains S×B
T
	​

 exact-stratum atoms;

scatters them to cells;

applies exact target centering;

forms worker and firm scores;

constructs an interleaved 2B
T
	​

-column RHS;

performs 2B
T
	​

 certified inverse actions;

contracts correction weights with the returned prediction matrix.

FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L1320-L1416.

vckss_scale_eng__target_contract() already fuses worker, firm, covariance, and total contractions into one tiled cell traversal; this is existing work and should not be proposed again as new. FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L722-L760.

The non-solver target work is approximately

Θ(P(S+C))+Θ(PC),

plus 2P certified inverse actions.

10. Complete residual certification

Every accepted FE matrix solve computes fitted values, worker and firm left-hand sides, and a separate scale-relative residual for every column. No Frobenius norm or neighboring column can mask a bad RHS. FILES/varcomp_kss/varcomp_kss.mata:L1929-L1975.

Certification work is therefore

T
cert
	​

=Θ(n
rhs
	​

(C+W+F)),

and it is mandatory. It may be fused with other traversals, but not deleted or weakened.

Total RHS count

The registered plan is

n
rhs
	​

=3P+k+1+1{fixedoffset and k>0}.

FILES/varcomp_kss/varcomp_kss_solver.mata:L454-L463, vckss_solver__planned_rhs().

Thus the no-control compressed route performs 3P+1 complete inverse actions: one fit, P leverage actions, and 2P target actions. At P=200, that is 601 RHSs, matching the receipts.

Governing terms by regime
Regime	Governing term	Packet diagnosis	Required remedy
Cold CZ18 command	T
selection/import
	​

	188.272 of 305 seconds	Optional prepared boundary and selection/compression redesign
Ordinary, well-conditioned CMG	Q-dependent sparse traversal, allocation, scatter and V-cycle throughput	Setup small; fixed-action batch width changes work by 2.607×	Plan-once layouts, fused streaming, flat workspaces, calibrated batches
Weak connectivity / density two	A=∑i
j
	​

	Weak d3 reaches 162–164 iterations and about 9,100 actions versus 13–14 and 737–853 in ordinary d3	Coarse seeding, deflation, stronger hierarchy, then recycled/block Krylov
Old degree/density four	T
build
	​

	8,755.873-second setup with only 15 iterations	Qualify and promote API-7/API-6 robust hierarchy; stable contraction
Large P	3P+k+1+δ certified RHSs, RNG, and batch memory	P200 has 601 complete RHSs; setup amortizes but traversal remains	Larger admitted batches, stage scheduling, recycling/deflation
Exact near exact_limit()	Np
2
+p
3
	Dense target and deletion algebra	Structured target actions and reduced materialization
2. Ranked bottleneck hypotheses and discriminating experiments
1. Cold-boundary import, selection, and lifecycle

[M] Evidence. The CZ18 P20 import/selection stage is 61.7% of Stata command time. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L288-L307.

[D] Hypothesis. A large portion is work that can be reused when the retained design, frequency, deletion partition, target strata, and options are unchanged. The current lifecycle deliberately clears data before numerical work and resets the prepared runtime afterward. FILES/varcomp_kss/varcomp_kss.ado:L1210-L1457; FILES/varcomp_kss/varcomp_kss_scale_runtime.mata:L9-L15.

Discriminating experiment. On an immutable input, report:

fresh selection/compression/solve;

fresh solve using an explicitly prepared validated handle;

second and tenth solve from that handle;

an invalidation run after changing one frequency, one ID, one target weight, and one option separately.

Promotion signature: first-run overhead no more than 5%, later command at least 2× faster on CZ18 P20, and every stale change rejected before estimator RNG.

2. Repeated sparse traversal and Mata matrix-call overhead

[M] Evidence. Identical 54,074 mathematical actions vary by 2.607× over batch widths. Ordinary 1,024-firm cells have small setup but substantial PCG/leverage/target time. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L88-L95; FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L235-L244.

[D] Hypothesis. Repeated materialization, gathers, panel handling, recursive temporaries, and interpreter dispatch are a first-order cost in the ordinary regime.

Discriminating experiment. Freeze hierarchy, RHS matrices, iteration counts, and action count. Replay:

Schur action only;

preconditioner action only;

complete certificate only;

no-op matrix callback with identical allocation shapes;

current whole solve.

Report seconds per physical call and per active cell-column. If the no-op and fixed-action replay retain a large batch-width slope, the gap is implementation throughput rather than conditioning.

3. Density-four hierarchy construction

[M] Evidence. Old setup is 246.8× slower at density four than density three; the robust hierarchy gives a 217× local setup improvement on the former fixture. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L223-L230; FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L69-L72.

[D] Hypothesis. The main architectural correction exists, but production qualification is incomplete. The report specifically lacks the predeclared direct frozen-API-5/API-6 complete-command median comparison for ordinary degree-two/three cells. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L332-L347.

Discriminating experiment. Run frozen API-5/API-7 complete commands on identical d2/d3 cells and direct API-7 full commands on mixed-degree averages 3.0, 3.25, 3.5, 3.75, 4.0, and 4.25 at the former pathological scale.

Promotion signature: d2/d3 regression no greater than 10%, no density-four discontinuity over 2× after normalizing by edge/vertex count, and all exact contraction and residual certificates pass.

4. Weak-graph iteration count

[M] Evidence. Weak d3 cases require 162–164 iterations and roughly 9,100 actions, versus 13–14 iterations and 737–853 actions in the comparable ordinary d3 cells. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L212-L221.

[D] Hypothesis. Here the gap is primarily algorithmic: even a 2× faster action leaves too many actions.

Discriminating experiment. Record the per-RHS iteration spectrum, not just maximum iteration, for path, barbell, lollipop, cluster-chain, and weak-link graphs. Replay the same RHSs with:

current PCG;

hierarchy-derived initial coarse seed;

fixed structural deflation;

canonical stage-local recycling;

fixed-width block PCG.

The experiment should hold the CMG hierarchy and complete residual gate fixed. Promotion requires at least a 50% reduction in RHS-equivalent actions on weak families without more than 5% command regression on ordinary families.

5. Repeated ordering inside compressed scatter

[D] Evidence. vckss_scale__prepare() constructs unit_cell_order/panel and target-stratum cell_order/panel, but vckss_scale__compact() discards both. vckss_scale_engine__scatter_sum() therefore calls order() and panelsetup() whenever the mapping is not identity. FILES/varcomp_kss/varcomp_kss_scale.mata:L652-L655 and L1176-L1201; FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L492-L516.

The run performs four certification scatters before fitting, one correction scatter, one leverage scatter per leverage batch, and one target scatter per target batch. Therefore, with ordinary nonidentity mappings, it invokes at least

5+⌈P/B
L
	​

⌉+⌈P/B
T
	​

⌉

comparison-order/panel constructions per command.

Discriminating experiment. Retain the prepared order/panel vectors and replay identical atoms and RHSs. Time scatter alone and the complete leverage/target stages. An exact-output A/B should be possible because the same order and stable reducer can be reused.

6. CMG application allocation and graph-action layout

[D] Evidence. The ordinary recursive apply allocates compatible, iterate, residual, coarse RHS, correction, and action matrices at successive levels. The existing reusable workspace allocates three matrices per level but still constructs coarse RHS and graph-action temporaries. FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L2814-L2866, L2929-L2964, and L3076-L3248.

The current solver explicitly disables the retained workspace because the measured implementation was slower than ordinary batched apply. FILES/varcomp_kss/varcomp_kss_solver.mata:L1692-L1697.

Discriminating experiment. Replay the same hierarchy and RHS matrices under:

current recursive apply;

current disabled workspace;

a flat, nonrecursive ping-pong workspace;

a flat workspace plus arc-block layout.

Record allocation count, bytes written, graph-action seconds, and complete solve seconds. Merely turning on the existing workspace is not a valid candidate; the experiment must test a redesigned kernel.

7. RNG and finite-projection accumulation

[M] Evidence. RNG consumed 100.809 of 904 seconds in the CZ18 P200 Optimization III run. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L97-L118.

[D] Bound. Even an infinitely fast RNG stage would improve that full command by only about 1.13×. It is important but not governing.

Discriminating experiment. Generate byte-identical atoms from a frozen cursor using current scalar/chunk logic and a new blocked vectorized provider. Compare atom matrices, rngstate(), caller-stream restoration, and stage time. Any atom or terminal-state difference kills the exact-output candidate.

8. Structural route and batch policy

[D] Evidence. Work models for diagonal and CMG already exist and avoid wall-clock routing, but the active route is primarily structural: if eligible CMG builds successfully, it is selected; otherwise automatic fallback occurs only before RNG. FILES/varcomp_kss/varcomp_kss_solver.mata:L473-L541 and L1508-L1725.

Batch selection uses deterministic memory forecasts and chooses admitted widths, but the current policy has limited platform-specific throughput calibration. FILES/varcomp_kss/varcomp_kss.ado:L785-L832.

Discriminating experiment. On a held-out matrix, force diagonal and CMG plus every admitted batch width. Fit versioned cost coefficients on one training half and calculate route regret on the holdout. Promotion requires selected time within 10% of the faster certified forced route and selected batch within 5% of the fastest admitted batch, without using live wall time or post-RNG fallback.

3. Architectural portfolio

The speed ranges below are [S] hypotheses, tied to the measured stage they could reduce. They are not promises.

1. Qualify and promote the robust API-7 hierarchy

Location / class: vckss_cmg__aggregate_gpl() and vckss_cmg__hierarchy_mode(), FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L1097-L1270 and L2527-L2795. T for the numerical hierarchy change relative to the older production path; the API-7 ownership-only release step itself is E.

Work and memory: Prevents the degree-four auxiliary-vertex hierarchy from retaining nearly the full fine graph over many levels. Uses the existing bounded structural and scratch forecasts.

Proof obligations: Component-contained deterministic aggregates; exact positive Galerkin contraction; quotient-SPD V-cycle; edge/vertex-complexity and reduction gates; unchanged complete original-system residual; no edge clipping or ridge.

Speed hypothesis: pathological old d4 end-to-end 5–10.7×, high confidence for setup and medium confidence for total command; ordinary cells 0.95–1.05×.

Effort/risk: medium effort, medium regression risk. Promote only after the missing frozen d2/d3 complete-command A/B is at most 10% slower, the former d4 fixture improves by at least 100× in setup, and all residual, component, complexity, memory, and deterministic replay gates pass. Kill promotion if any ordinary d2/d3 median regresses over 10%.

2. Stable dense-label Galerkin contraction

Location / class: vckss_cmg__contract_graph(), FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L2076-L2184. E if it reproduces the existing lexicographic (u,v,contributor) order and summation sequence; otherwise T.

Work and memory: Replace comparison sorting of dense coarse endpoint labels with stable counting/radix passes. Edge collapse becomes O(E+V), with O(E+V) temporary integer/double vectors. It does not remove every hierarchy ordering operation, but directly addresses quotient-edge contraction.

Proof obligations: Dense endpoint range; stable contributor order within each duplicate edge; exact total weight; no missing or nonpositive edge; identical component labels; exact L
c
	​

=P
′
LP; scratch below the existing construction cap.

Fallback: Current comparison-sort contraction, selected before RNG, if scratch admission or a construction certificate fails.

Speed hypothesis: hierarchy stage 1.2–2.0× on d4–d7; current-API-7 high-density end-to-end 1.00–1.50×; confidence medium.

Effort/risk: medium/high effort, medium numerical risk. Promote at ≥30% hierarchy reduction on d4–d7, no ordinary setup regression over 5%, and peak scratch within forecast. Kill if edge weights or hierarchy diagnostics differ outside the intended contract class.

3. Retain deletion-unit and target-stratum scatter plans

Location / class: plans are built in vckss_scale__prepare() but dropped in vckss_scale__compact(); repeated ordering occurs in vckss_scale_engine__scatter_sum(). FILES/varcomp_kss/varcomp_kss_scale.mata:L652-L670, L1176-L1201; FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L492-L516. E.

Work and memory: Remove repeated order() and panelsetup() calls. Additional persistent storage is approximately

8(G+S+4C)

bytes for the two order vectors and two C×2 panel matrices.

Proof obligations: The stored plans must be the exact plans generated during preparation; stable_groupsum() must receive the same row order and panels; plan dimensions and group keys must be recertified before RNG.

Fallback: If plan storage is not admitted, use the existing transient sorting path.

Speed hypothesis: leverage/target non-solver work 1.2–2.0×, ordinary full command 1.05–1.25×, confidence medium.

Effort/risk: low/medium effort, low regression risk. Promote if scatter-stage time falls ≥20%, full ordinary command falls ≥8%, output and diagnostics are bitwise identical, and peak memory remains within the direct model. Kill if full command gain is under 5% on large G,S fixtures.

4. Narrow dual CSR-like incidence layouts

Location / class: the compressed design currently stores one cell payload plus worker- and firm-order vectors. FILES/varcomp_kss/docs/NUMERICAL_ARCHITECTURE.md:L5-L12; FILES/varcomp_kss/varcomp_kss_scale.mata:L675-L705. E if reduction order is unchanged; otherwise T.

Work and memory: Store narrow worker-major and firm-major incidence arrays—endpoint, weight, and payload index—rather than repeatedly gathering the common payload through order vectors. Expected extra storage is roughly 24C–48C bytes, depending on whether endpoint and scale fields can be shared.

Proof obligations: Scientific cell payload remains unique; semantic IDs stay separate from storage order; worker and firm panel reductions occur in the same canonical order; direct memory forecasts include both layouts.

Fallback: Existing single-payload/dual-order view if the duplicate incidence layout is not admitted.

Speed hypothesis: Schur and transpose actions 1.1–1.4×, ordinary end-to-end 1.03–1.20×, confidence medium-low.

Effort/risk: medium effort, medium layout risk. Promote if fixed-action operator time falls ≥15%, memory rises no more than the forecast and 10% of direct peak, and no ordinary cell regresses over 5%.

5. Fuse final prediction, complete certificate, and downstream contractions

Location / class: final reconstruction/certificate is in vckss__fe_solve_matrix_backend(), while leverage moments and target contraction traverse returned predictions afterward. FILES/varcomp_kss/varcomp_kss.mata:L1929-L1975; FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L1206-L1214 and L1400-L1415. Normally T; an exact-order implementation could be E.

Work and memory: During the mandatory final cell traversal, compute:

fitted cell values;

worker and firm normal equations;

per-RHS residual scales;

quarantined leverage projections or target contractions.

Commit the downstream accumulators only after every RHS passes its full certificate. This can remove one prediction materialization and one subsequent C×B traversal.

It is not valid to fuse the Schur action and preconditioner application across a PCG iteration: the latter depends on the updated residual after the former and the scalar recurrences.

Proof obligations: Per-column residual remains authoritative; no contractions are posted for a failed batch; target and moment reduction order is deterministic; target identities remain separately checked.

Speed hypothesis: solver-plus-contraction stage 1.2–1.6×, ordinary full command 1.05–1.25×, confidence medium.

Effort/risk: high effort, medium/high numerical risk. Promote if certified batch time falls ≥25%, peak scratch falls or rises no more than 5%, and all adversarial cancellation tests stay within the registered roundoff envelope.

6. Redesign CMG application as a flat ping-pong workspace

Location / class: current recursive and workspace paths are in FILES/varcomp_kss/cmg/src/cmg_core.mata.in:L2814-L2866, L2929-L2964, and L3076-L3248. The existing workspace is disabled at FILES/varcomp_kss/varcomp_kss_solver.mata:L1692-L1697. E if operation order is retained; likely T after graph-kernel restructuring.

Work and memory: Preallocate compatible/iterate/work matrices for every level, use explicit forward and backward level loops, and reuse one admitted arc-contribution buffer. Avoid recursive return matrices, repeated J() construction, and coarse-RHS ownership ambiguity. Current workspace storage is approximately

24B
ℓ
∑
	​

V
ℓ
	​


bytes before action scratch.

Proof obligations: Fixed linear symmetric preconditioner; identical projection at every level; no aliasing between parent and child buffers; unused columns zeroed; workspace belongs to one invocation or explicit prepared handle; memory gate precedes RNG.

Fallback: Existing ordinary apply selected before RNG if the flat workspace is not admitted. A post-RNG workspace error is a typed failure, not a route change.

Speed hypothesis: preconditioner stage 1.3–2.0×, ordinary full command 1.05–1.25×, weak full command 1.10–1.40×, confidence low/medium.

Effort/risk: high effort, high implementation risk. Promote only if apply time falls ≥30%, no benchmark cell is over 5% slower, and scalar/batched symmetry and complete residual tests pass.

7. Stage-specific batch selection and active-column scheduling

Location / class: current batch admission is in FILES/varcomp_kss/varcomp_kss.ado:L785-L832; inactive columns are zeroed in vckss__fe_solve_matrix_backend(), FILES/varcomp_kss/varcomp_kss.mata:L1791-L1911. T.

Work and memory: Select distinct leverage and target widths from a calibrated cost model because target solves contain 2B
T
	​

 columns. Add deterministic active-column compaction at fixed checkpoints when many columns have converged, retaining a mapping back to logical RHS order. Compaction changes matrix width but not scalar recurrences for retained columns.

Proof obligations: Logical probe order and atoms independent of batch(); per-RHS diagnostics map back exactly; batch width never exceeds current memory forecasts; no adaptive change based on wall time; complete residual for every column.

Speed hypothesis: versus current automatic batching, end-to-end 1.00–1.30×, confidence medium-high; the measured B1-to-B16 2.607× span is an upper bound, not an expected gain from the current auto policy.

Effort/risk: medium effort, medium numerical risk. Promote if automatic time is within 5% of the fastest admitted forced width over the benchmark matrix and active compaction reduces physical work ≥15% on heterogeneous-iteration batches. Kill if common-probe outputs vary beyond the registered batch tolerance.

8. Calibrated deterministic route selection

Location / class: deterministic work estimators exist in vckss_solver__diagonal_work() and vckss_solver__cmg_work(), but production routing is structural. FILES/varcomp_kss/varcomp_kss_solver.mata:L473-L541 and L1508-L1707. T.

Work and memory: Use versioned offline coefficients for Schur cell passes, hierarchy setup, V-cycle edge/vertex passes, and terminal solves. Inputs must be structural counts, registered processor count, Stata version class, planned RHS count, and admitted batch—not measured current-run wall time.

Proof obligations: Route chosen before estimator RNG; both routes retain complete residual certification; a selected route’s post-RNG failure withholds rather than switching; no valid command is withheld solely because a forecast is pessimistic.

Fallback: Existing structural route if the calibration version is unavailable or its feature vector is outside the qualified envelope.

Speed hypothesis: 1.00× on already-correct cells, potentially several-fold where structural routing chooses a materially slower backend; confidence medium but highly shape-dependent.

Effort/risk: medium effort, medium policy risk. Promote when held-out regret is ≤10% relative to the faster certified forced route, with zero post-RNG route changes and no new resource failures.

9. Hierarchy-derived initial coarse seed

Location / class: requires extending vckss__fe_solve_matrix_backend() around its zero initial firm coefficient and residual setup, FILES/varcomp_kss/varcomp_kss.mata:L1692-L1756. T.

Work and memory: Construct a fixed structural quotient basis Z from selected CMG aggregates, with bounded rank r. Certify

E=Z
′
S
F
	​

Z,

then initialize each RHS with

x
0
	​

=ZE
−1
Z
′
b,r
0
	​

=b−S
F
	​

x
0
	​

,

and run otherwise unchanged PCG from x
0
	​

. This is safer than immediate deflated CG because ordinary CG permits an arbitrary certified initial guess.

Memory is O(rF+r
2
), or less with implicit prolongation.

Proof obligations: Z depends only on the fixed hierarchy, not atoms, batch, processor scheduling, or earlier RHSs; quotient compatibility; full-rank and residual gate for E
−1
; complete original-system certificate unchanged.

Speed hypothesis: weak/d2 end-to-end 1.2–3.0× if the coarse basis captures the slow modes; ordinary 0.98–1.05×; confidence low/medium.

Effort/risk: medium/high effort, medium numerical risk. Promote if weak-family actions fall ≥50%, seed setup is under 10% of saved work, and ordinary commands regress no more than 5%. Kill if the basis frequently fails its small-matrix gate or duplicates CMG coarse correction without reducing iterations.

10. Canonical fixed-width block PCG microkernel

Location / class: replace independent scalar recurrences inside vckss__fe_solve_matrix_backend(), FILES/varcomp_kss/varcomp_kss.mata:L1753-L1911. T.

Work and memory: Solve fixed internal microblocks of q, such as 4 or 8, using matrix Gram recurrences. This may improve cache reuse and reduce physical traversals, but adds O(q
2
F) inner products/orthogonalization and small q×q rank decisions. The public batch can feed a queue; q must remain fixed and independent of the public batch() option.

Proof obligations: Deterministic block rank threshold; symmetric positive block Gram; per-column convergence and residual; canonical fallback for dependent columns; no Frobenius acceptance; same logical RHS order.

Fallback: On block rank loss, solve the same RHSs with existing scalar lockstep PCG using the same preconditioner and atoms. This is an internal solver fallback, not a CMG-to-diagonal route change.

Speed hypothesis: large-P full command 1.10–1.80×, weak cases potentially higher; confidence low.

Effort/risk: very high effort, high regression risk. Promote only if physical matrix batches fall ≥30%, solver core falls ≥20%, block breakdown occurs in under 1% of qualified batches, and every fallback reproduces the scalar certificate.

11. Canonical stage-local Krylov recycling

Location / class: new state around the repeated calls from vckss__jla_backend() and vckss_scale_eng__run_prepared(). FILES/varcomp_kss/varcomp_kss.mata:L3111-L3884; FILES/varcomp_kss/varcomp_kss_scale_engine.mata:L1137-L1416. T.

Work and memory: Retain at most r certified Ritz or correction directions within a stage. Leverage and target use separate recycle spaces. Update only after fixed logical microblocks, in probe order, with canonical signs and deterministic rank truncation. Memory is O(rF+r
2
).

Proof obligations: Public batch() cannot change update epochs; processor scheduling cannot change the basis; target recycling cannot depend on how many leverage probes were requested; common probe prefixes use the same basis history; only fully certified RHSs enter the space.

Fallback: Drop the recycle space deterministically and continue with current PCG if its Gram or rank certificate fails.

Speed hypothesis: ordinary full command 1.05–1.60×, weak/large-P 1.20–2.50×, confidence low.

Effort/risk: very high effort, high reproducibility risk. Keep experimental until solver time falls ≥15% across at least three unrelated graph families and every probe-count, batch, row-order, and processor replay passes. Kill immediately on any batch- or scheduling-dependent atom/result path.

12. Explicit prepared-data and reusable-workspace lifecycle

Location / class: the package already has an internal preparation/clear/run/restore boundary, but resets it after every command. FILES/varcomp_kss/varcomp_kss.ado:L1210-L1457; FILES/varcomp_kss/varcomp_kss_lifecycle.ado:L29-L264; FILES/varcomp_kss/varcomp_kss_scale_runtime.mata:L9-L176. E.

Work and memory: Expose an explicit bounded handle that owns:

retained sample and semantic-order fingerprint;

compressed payload and scatter plans;

FE view;

selected immutable hierarchy and factors;

admitted workspace;

API/build/options/version identifiers.

The handle must not own or advance an RNG cursor. Each estimate opens fresh leverage and target cursors from the registered seed.

Proof obligations: Exact input/options hash; explicit invalidation and drop; no hidden unbounded cache; caller data restored after prepare and every run; resource admission includes persistent bytes; stale handle fails before RNG; first invocation remains equivalent to the current command.

Speed hypothesis: charged first call 0.95–1.00×; subsequent identical-design calls 1.8–2.7× on a CZ18-like boundary, confidence medium. The upper arithmetic bound from removing all measured selection and hierarchy work is approximately 2.77×; actual reuse will be lower.

Effort/risk: high effort, high API/state risk. Promote if first call regresses ≤5%, second call improves ≥2× on CZ18 P20, handle bytes match the direct model, and all mutation/invalidation/restoration red-team tests pass.

13. Blocked, byte-identical RNG atom generation

Location / class: vckss_rng__open_cursor() and vckss_rng__cursor_next(), FILES/varcomp_kss/varcomp_kss_rng.mata:L708-L843. E only.

Work and memory: Reduce per-stratum/probe Mata call overhead while retaining the exact domain stream, canonical semantic order, logical probe sequence, binomial/Rademacher support, and terminal generator state. Scratch remains O(Bmax(G,S)).

Proof obligations: Atom matrices bitwise identical for every tested chunking; cursor state identical; leverage and target domains disjoint; caller streams captured and restored; unsupported runtime remains fail-closed.

Speed hypothesis: RNG stage 1.5–3.0×, but P200 full-command 1.02–1.10× because RNG is about 11.2% of the cited command; confidence medium.

Effort/risk: medium effort, high contract sensitivity. Promote only with exhaustive call-shape replay, byte-identical atoms and state, and ≥1.5× RNG-stage improvement. Any atom difference rejects this as an exact-output optimization.

14. Structured exact-target actions

Location / class: vckss__targets(), vckss__target_diagonal(), and vckss__exact(), FILES/varcomp_kss/varcomp_kss.mata:L684-L745 and L747-L1049. T, or E only if multiplication and reduction order is reproduced.

Work and memory: Apply worker, firm, and covariance targets through centered group actions instead of materializing all three p×p matrices and independently multiplying design*A into each target diagonal. Exploit the identities involving worker shares, firm shares, and worker–firm cell shares.

Potential reduction is from several Np
2
 passes toward Np group passes plus p
2
 coefficient-space work, subject to deletion-block requirements.

Proof obligations: Same target matrices algebraically; controls have exact zero target rows/columns; omitted-firm invariance; direct deleted-information gate unchanged; complete block residuals retained.

Speed hypothesis: exact-route full command 1.3–2.5× near exact_limit(), JLA commands 1.00×; confidence medium.

Effort/risk: medium/high effort, medium numerical risk. Promote if near-limit exact fixtures improve ≥1.5×, all dense-oracle comparisons and statuses agree within registered tolerances, and memory falls or remains neutral.

4. Detailed designs for the top three
Top design 1: production-grade robust hierarchy with stable contraction

This design combines candidates 1 and 2. The immediate priority is qualification and promotion of the existing robust numerical core; stable contraction is a second, separable optimization.

State ownership
cmg_build_input
    immutable cells, canonical worker/firm keys
    planned RHS count
    memory envelope
    versioned options


cmg_build_scratch
    aggregation arrays
    raw quotient edge arrays
    two radix/counting permutations
    count/offset arrays
    certification work


cmg_hierarchy
    immutable graph per level
    aggregation and restriction panels
    component projections
    inverse degrees
    terminal factors
    structural/dense/action byte accounts

Build scratch is command-local and released after construction. The hierarchy is immutable and may be reused across all RHSs, as already allowed by the component contract.

Proposed build
function build_hierarchy(cells, planned_rhs, memory):
    options   = options_resource(memory, fine_firms, planned_rhs)
    preflight = preflight(cells, planned_rhs, canonical_keys=1,
                          memory, options)
    if preflight says DIAGONAL:
        return PRE_RNG_DIAGONAL


    graph = build_exact_hybrid_graph(cells)


    for level = 1 .. options.max_levels:
        certify_components_weights(graph)


        if terminal_policy_accepts(graph):
            factor_and_certify_terminals(graph)
            return hierarchy


        if graph has auxiliaries or edge_count <= 8 * vertex_count:
            aggregate = aggregate_gpl(graph)
        else:
            aggregate = retained_dense_quotient_specialization(graph)


        if aggregate fails reduction/complexity certificate:
            aggregate = normalized_component_contained_fallback(graph)


        raw_u, raw_v, raw_w, contributor =
            map_edges_through_aggregate(graph)


        canonicalize each endpoint pair:
            u = min(raw_u, raw_v)
            v = max(raw_u, raw_v)
            discard only exact self loops


        # Stable LSD ordering. Original contributor order is retained for ties.
        perm = stable_counting_sort(1..E, secondary_key=v, range=1..Vc)
        perm = stable_counting_sort(perm, primary_key=u, range=1..Vc)


Why this addresses the discontinuity

The old density-four path retained 640,625 hybrid vertices over seven levels. The current GPL-based aggregate path explicitly handles degree-four-plus auxiliary vertices and applies branch splitting, weak-group repair, canonical labels, and exact contraction. The packet’s API-6 measurements already show that this removes the former setup cliff. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L36-L72.

Stable dense-label contraction then targets the remaining comparison sort in vckss_cmg__contract_graph(). It does not change aggregation mathematics and is independently killable.

Finite-precision hazards

Edge summation order. Stable counting passes must reproduce the current (u,v,contributor) order. Otherwise this is a T-class change.

Dense-label range. Count arrays must be sized from certified coarse vertex counts before allocation.

Integer exactness. Endpoint and contributor indices must remain below the exact binary64 integer limit.

Positive cancellation. Input edge weights are positive; a nonpositive contracted weight is a typed construction failure.

Tie handling. Canonical graph keys and aggregate labels cannot use row order or encoded-ID accidents.

Memory. The two permutations and count arrays must fit the existing preflight scratch cap.

Fallback behavior

If API-7 hierarchy construction fails before RNG under preconditioner(auto), retain the registered typed pre-RNG diagonal fallback.

If forced CMG fails, return FORCED_CMG_FAILED; do not switch.

If a hierarchy or preconditioner fails after RNG, withhold; never route-switch. This is required by FILES/varcomp_kss/cmg/docs/API_CONTRACT.md:L49-L55.

Quantitative gate

Complete frozen API-5/API-7 d2/d3 medians: API 7 no more than 10% slower.

Former d4 fixture: at least 100× setup improvement and no more than 2× normalized discontinuity at the d3/d4 mixture boundary.

Stable contraction: at least 30% faster hierarchy setup on d4–d7 at 4,096 and 10,240 firms.

Zero failures of exact contraction, component, positivity, complexity, terminal, memory, or complete-residual gates.

No ordinary complete-command regression over 5% from stable contraction.

Top design 2: immutable prepared plan plus streamed compressed kernels

This design combines candidates 3–6 and 12, but it should be implemented in two promotion tiers.

Tier E: retain plans, reuse immutable prepared state, and eliminate allocations without changing reduction order.

Tier T: fuse traversals and introduce new layouts only after Tier E establishes the attainable exact-output gain.

State ownership
vckss_prepared_plan  # immutable, explicitly named and bounded
    build/API/Stata-version identifiers
    complete input/options fingerprint
    retained sample/deletion fixed-point certificate
    compressed cell payload
    worker/firm orders and panels
    deletion-unit and target-stratum scatter plans
    FE operator view
    selected hierarchy and factors
    admitted batch capacities
    direct byte inventory


vckss_run_workspace  # mutable, owned by one invocation
    leverage and target atom buffers
    cell/worker/firm RHS matrices
    PCG vectors
    CMG flat workspace
    quarantined residual/contraction accumulators
    moment subtotal and compensation
    per-RHS diagnostics


vckss_rng_state  # never stored in the plan
    leverage cursor
    target cursor
    saved caller stream state
Preparation
prepare(dataset, options):
    run the registered complete-case and graph/deletion fixed point
    construct canonical semantic ordering
    compress rows into cells, deletion units, and target strata
    certify that units and strata reproduce cell totals
    retain all scatter orders and panels
    construct FE view and route before estimator RNG
    construct/certify hierarchy if selected
    admit persistent plan + maximum run workspace bytes
    fingerprint all scientific inputs and relevant options
    restore caller data and return an explicit plan handle

The first implementation should bind all outcome, frequency, target, deletion, control, and ordering inputs. Reusing only a “structural” subset across changed outcomes is potentially safe, but it multiplies invalidation proof obligations and should not be the first public lifecycle.

Run
run(plan, seed, P, tolerance, output_options):
    validate handle version, API, options, fingerprint, and memory
    open fresh leverage and target RNG cursors
    allocate or borrow one admitted run workspace


    # Fit
    rhs = transpose_cells(plan, cell_outcome_sum)
    fit = solve_and_certify(rhs)
    do not expose prediction until complete certificate passes


    # Leverage
    for logical probes 1..P in admitted B_L:
        atoms_G = leverage_cursor.next(B_L)
        atoms_C = scatter_with_retained_plan(atoms_G)
        rhs      = transpose_cells(plan, atoms_C)
        solved   = solve_and_certify(rhs)


        in one final cell/unit traversal:
            build quarantined projections and five moments
        commit moments only if every RHS certificate passed


    construct every deletion-unit finite correction
    scatter correction weights using retained plan


    # Target
    for logical probes 1..P in admitted B_T:
        atoms_S = target_cursor.next(B_T)
        atoms_C = scatter_with_retained_plan(atoms_S)
        center and form interleaved 2*B_T RHS
        solved = solve_and_certify(rhs)


        in the mandatory final cell traversal:
            accumulate quarantined worker/firm/covariance/total contractions
            accumulate complete worker and firm equations
        commit contractions only if every RHS certificate passed


Current hidden passes removed

[D] The current compact run reconstructs ordering metadata at least

5+⌈P/B
L
	​

⌉+⌈P/B
T
	​

⌉

times. Retaining plans removes these comparison-order passes.

The current final solve constructs a prediction matrix, performs a weighted transpose for the complete certificate, returns the prediction, then leverage or target code traverses the prediction again. A quarantined fused final traversal can reduce this to one cell pass while retaining the exact per-RHS acceptance decision.

The current CMG application recursively constructs intermediate matrices. A flat workspace can eliminate most transient ownership and allocation.

Memory model

Existing compressed persistent bytes are:

8(9C+3W+4F)+32G+32S.

Retained scatter plans add approximately:

8(G+S+4C).

A three-matrix-per-level CMG workspace adds approximately:

24B
ℓ
∑
	​

V
ℓ
	​

,

plus the separately capped graph-action scratch. Every term must be included in the direct resource model before RNG.

Proof obligations

Staleness: complete fingerprint and version validation before RNG.

Caller state: dataset, sorted order, modified flag, sample marker, filename metadata, and RNG streams restored exactly. The current lifecycle already certifies these categories. FILES/varcomp_kss/varcomp_kss_lifecycle.ado:L176-L264.

No hidden cache: explicit handle count and explicit drop; no unbounded LRU.

Reduction order: Tier E must use the same row orders, panels, stable group sums, eight-probe moment tiles, and logical probe sequence.

Residual: fused contractions remain quarantined until every complete worker-plus-firm residual passes.

Memory: no optimistic reuse assumptions across selection, transition, numerical work, and restoration.

Failure: a stale or inadmissible handle fails before RNG; a post-RNG numerical failure withholds.

Quantitative gate

Tier E: bitwise identical outputs, diagnostics, atoms, and terminal RNG states; ≥8% ordinary command improvement; no memory-model underprediction.

Prepared boundary: first call ≤5% slower; second and tenth calls ≥2× faster on CZ18 P20; mutation tests always invalidate before RNG.

Tier T fusion: ≥25% lower combined final-certificate/contraction time; no accepted residual above the current gate; no command cell over 5% slower.

Top design 3: deterministic repeated-RHS Krylov ladder

The safest repeated-RHS program is not “implement block CG immediately.” It is a staged ladder:

fixed structural initial coarse seed;

fixed structural deflation if seeding succeeds;

canonical stage-local recycling;

fixed-width block PCG only after the first three are understood.

Canonical logical RHS order

The solver must define a global order independent of public batching and processors:

control-preparation RHS 1..k
fit RHS
fixed-offset extra RHS, if any
leverage probes 1..P
target probe 1 worker column, target probe 1 firm column
target probe 2 worker column, target probe 2 firm column
...

Leverage and target recycle spaces remain separate. A change in batch() only changes how many canonical RHSs are materialized together; it cannot change recycle update epochs.

Stage A: structural coarse seed
Z = deterministic_basis_from_fixed_hierarchy(max_rank=r)


E = Z' * S * Z
certify E is SPD and its inverse residual passes


for each canonical microbatch B:
    X0 = Z * solve(E, Z' * B)
    R0 = B - S * X0
    project R0 onto firm quotient


    run existing lockstep scalar PCG from X0, R0
    reconstruct workers
    certify every original full-system RHS

Using X
0
	​

 only changes the initial guess. It does not alter the operator, preconditioner, estimator, or acceptance gate.

Suitable Z candidates include bounded aggregate-indicator contrasts from a selected CMG level. The basis rank must be capped and chosen from structure alone.

Stage B: augmented/deflated scalar PCG

Only after Stage A shows action reduction should the implementation consider a fixed deflation projector. The proof must establish:

quotient compatibility;

symmetry in the relevant inner product;

positive definiteness on the complementary space;

deterministic rank handling;

unchanged complete certificate.

A poorly implemented deflation projector can make an otherwise SPD recurrence nonsymmetric, so this is not a drop-in optimization.

Stage C: stage-local recycling
U_stage = structural basis only


for microblock t in fixed logical order:
    seed current RHSs from U_stage
    solve with scalar or block kernel
    certify every RHS


    candidate_vectors =
        deterministic Ritz/residual directions from certified solves


    canonicalize signs using a semantic coordinate rule
    append in logical RHS order
    rank-reveal with fixed threshold and complete small-matrix residual
    truncate to the fixed rank cap r

Leverage and target start from independent structural spaces. Target recycling may not depend on the number or convergence history of leverage probes.

Stage D: block microkernel

For a fixed q-column microblock:

R = B - S*X
Z = M^{-1}R
P = Z


repeat:
    AP = S*P
    G  = P'AP
    H  = R'Z


    rank/eigen certify G and H
    if rank loss:
        send affected columns to scalar PCG with same backend and atoms


    Alpha = solve(G, H)
    X = X + P*Alpha
    R = R - AP*Alpha


    check each column independently
    periodically recompute each true quotient residual


    Znew = M^{-1}R
    Beta = solve(H, R'Znew)
    P = Znew + P*Beta

This pseudocode omits several numerical-stability refinements intentionally: the actual implementation would require deterministic reorthogonalization or a block-CG variant with a well-defined rank-deflation rule.

Memory

For recycle rank r and microblock q:

M
repeat
	​

=O((r+q)F+(r+q)
2
)+O(q
ℓ
∑
	​

V
ℓ
	​

),

plus existing cell and certificate scratch. The rank and width caps must be set by the direct memory API before RNG.

Hazards and fallbacks

Block rank loss: deterministic scalar fallback on the same RHSs, same preconditioner, and same atoms.

Loss of conjugacy: periodic true quotient residual already exists and remains active.

Heterogeneous convergence: active-column compaction must preserve logical mapping.

Batch dependence: internal q and recycle epochs are fixed, not chosen from public batch().

Probe-count dependence: stage spaces have prefix semantics and leverage/target are isolated.

Bad recycle basis: discard it and continue scalar PCG; do not change route.

Certificate failure: withhold the command.

Quantitative gate

Structural seed alone: weak-family actions down ≥50%, ordinary command regression ≤5%.

Recycling: solver time down ≥15% on at least three graph families, zero batch/probe/process replay failures.

Block kernel: physical matrix batches down ≥30%, solver time down ≥20%, fallback under 1%, every RHS fully certified.

No repeated-RHS method is promoted based only on maximum iteration; report total actions, physical batches, per-RHS iteration distribution, and complete residuals.

5. Parity definition

Because the comparator has different target features, randomization, tolerance, and correction algebra, “parity” must be a vector of boundary-specific performance comparisons, not equality of corrected estimates. The packet itself makes that qualification. FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:L126-L147; FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L194-L207.

Required boundaries
Comparison	Start	Stop	Required reporting
Cold end-to-end	Process launch and raw/native input availability	Process exit after output and caller-state restoration	Startup, import/conversion, selection, pool/MEX/runtime setup, estimator, serialization, teardown
Cold command	Validated input already resident in each implementation’s registered native command format	Command returns	Command time, excluding external wrappers
Warm command	Runtime and code loaded; input resident; no retained estimator plan unless explicitly stated	Command returns	At least three calls after one warm-up
Prepared warm	A validated reusable plan already exists	Estimate returns	Preparation time and bytes reported separately; amortized costs for 1, 2, 5, 10 uses
Stage	Entry to a named analogous stage	Exit from that stage	Selection/compression, hierarchy, RNG, fit, leverage, target, correction, complete certificate
Core action	Fixed operator, hierarchy, RHS matrix, and initial state	All requested certified solves return	RHS actions, physical batches, seconds/action, seconds/cell-column, iteration distributions
Convergence	Same declared problem shape	Each implementation’s own acceptance boundary	Requested tolerance, maximum and per-RHS full residual, nonconvergence status; never count an uncertified output as parity
Memory	Same boundary as the timing row	Peak over the relevant process tree	Direct model, process-tree RSS, virtual memory, scheduler envelope, persistent plan bytes
Core count	Same task	Same stop	Four effective application workers/cores; numerical libraries constrained against oversubscription

The maintained MATLAB cold/warm report explicitly separates executable startup, MEX setup, import, pool creation, estimator call, serialization, and teardown; those should not be collapsed into one ratio against a narrower Stata boundary. FILES/varcomp_kss/benchmarks/reports/KSS_PROD_1_2026-08-16.md:L382-L421.

Fair CZ18 reporting

The registered 305/43.802 ratio remains a valid command-throughput receipt under the packet’s boundary. It should be accompanied by:

Stata import/selection: 188.272 seconds;

Stata hierarchy: 6.442 seconds;

Stata remaining command: approximately 110.286 seconds;

MATLAB command: 43.802 seconds;

both process-tree memory peaks;

four effective application workers/cores;

explicit note that MATLAB’s CSV preparation wrapper wall is outside its command boundary.

FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L288-L319.

Statistical and numerical qualification

A comparator timing row must state:

whether frequency semantics are literal-row matched;

target definitions and target weights;

probe count and seed label;

RNG algorithm and stream differences;

requested tolerance and maximum iterations;

whether every result passed its own complete residual;

whether corrected estimates are merely displayed or subject to an equality gate.

An unconverged MATLAB cell may remain a throughput observation but cannot be described as accepted numerical parity. The same applies symmetrically to Stata.

Repetitions and hardware

Use at least three fresh-process repetitions for cold comparisons and at least five in-process repetitions for warm/prepared comparisons. Randomize implementation order across identical node classes, report median and median absolute deviation, pin or report NUMA/socket placement, and retain process-tree RSS and scheduler records. Cross-host reversals should remain labeled unexplained unless a separate experiment establishes the cause, consistent with FILES/varcomp_kss/benchmarks/reports/KSS_PROD_1_2026-08-16.md:L423-L430.

6. Benchmark and red-team plan
Structural benchmark matrix
Scale

Firms F: 16, 32, 64, 256, 1,024, 4,096, 10,240, and 15,625.

Worker/firm ratios: 4, 11.1 (CZ18-like), 16, 40, and 64.

Cells C: chosen from the degree profiles below.

Stored-row/cell ratios R/C: 1, 2, 8, and 26.3.

Deletion-unit/cell ratios G/C: 0.25, 1, and 4 where scientifically valid.

Target-stratum/cell ratios S/C: 0.25, 1, and 4.

Degree and density boundary

Use both exact regular degrees and mixtures:

degree 2, 3, 4, 5, 6, 7;

mixtures with mean 3.25, 3.5, and 3.75;

mixed heavy-tail degrees with the same mean;

a targeted sweep where 0%, 10%, 25%, 50%, 75%, and 100% of workers have degree at least four.

The purpose is to distinguish a true degree-four branch discontinuity from smooth edge/vertex scaling.

Connectivity and graph families

At every admitted scale include:

path;

ring;

star;

irregular;

expander;

tied-weight;

one-heavy-edge;

log-spread weights;

barbell;

lollipop;

cluster chain;

disconnected components;

singleton components;

near-disconnected two-cluster graph with a tunable weak link;

many parallel deletion IDs at common worker–firm coordinates;

articulation-heavy graphs before the deletion fixed point.

The packet’s current robust hierarchy has already passed 13 of these families; the new variants must preserve that baseline. FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:L164-L168.

Estimator dimensions

Probes P: 2, 20, 40, 200, and 1,000.

Public batch: 1, 2, 4, 8, 16, 32, and 64 only when admitted.

Internal Krylov microblock q: 1, 2, 4, 8.

Recycle/deflation rank r: 0, 4, 8, 16, 32, 64.

Controls k: 0, 1, 8, and 32.

Deletion: observation and match.

Nuisance: joint and fixed offset.

Preconditioner: forced diagonal, forced CMG, automatic.

Processors: 1, 2, 4, and 8.

Memory envelopes: 1, 4, 16, and 56 GiB where the task is otherwise admissible.

Required performance metrics

For every cell record:

complete command, selection, compression, hierarchy, fit, leverage, target, correction, RNG, Schur, preconditioner, PCG, and complete-certificate times;

planned RHSs and accepted RHSs;

RHS-equivalent Schur/preconditioner actions;

physical Schur/preconditioner batches;

per-RHS iterations and residuals;

hierarchy levels, terminal size, hybrid vertices/edges, edge and vertex complexity;

number and time of scatter-plan constructions;

workspace allocations and bytes written where instrumentation permits;

direct modeled peak, process-tree RSS, and virtual memory;

route and batch prediction versus forced-route/forced-batch oracle;

caller data and RNG restoration.

Numerical adversaries

Positive weights spanning 10
12
, already represented in the CMG qualification.

Frequency total exactly 2
53
, one below it, and one above it.

Cancellation patterns such as large opposite outcomes with small residual differences.

Near-singular control spans and invertible rescalings Z↦ZT.

Deletion makers with minimum eigenvalues immediately above and below the registered gate.

Zero reduced RHS columns mixed with difficult nonzero columns.

Nearly collinear probe RHSs to trigger block-rank loss.

A batch with highly heterogeneous convergence, such as iterations (2,3,5,100,150,…).

More than 100 iterations to exercise explicit residual replacement.

Tiny Schur-diagonal ratios and weak-link Laplacians.

Multiple components, isolated vertices, and exact component-ranking ties.

Nonfinite intermediate candidates that must be withheld before posting.

Reproducibility checks

For each accepted fixture:

repeat under stored-row permutations;

repeat under every admitted public batch;

repeat under processor counts 1, 2, 4, and 8;

compare fresh versus prepared handle;

compare current scatter sorting versus retained plans;

compare recursive versus flat workspace;

compare common probe prefixes for P=20,40,200;

verify leverage and target cursor terminal states;

verify caller RNG streams before and after;

verify caller dataset signature, sample marker, sort order, filename, and modified flag;

verify every per-RHS status and complete residual;

rerun with semantic ties and with an explicit valid probeorder().

Arbitrary identifier relabeling should not incorrectly be required to produce identical pathwise atoms: the ado contract states that a relabeling may produce another valid draw. Row order, batching, and solver route must not change the registered semantic order. FILES/varcomp_kss/varcomp_kss.ado:L576-L597.

Overall promotion gates

A production candidate must satisfy all of the following:

no changed retained sample, deletion fixed point, target algebra, frequency semantics, or controls;

no changed probe atoms or terminal cursor state for E-class changes;

T-class changes remain inside all registered numerical comparison gates;

every accepted RHS passes the complete original-system residual;

no partial target is returned after one RHS or block fails;

observed peak remains inside both the direct forecast and scheduler envelope;

no hidden route change after RNG;

improvement repeats on at least two machines or node classes;

median performance gain exceeds noise by a predeclared margin;

no benchmark family regresses over its candidate-specific kill threshold.

7. Rejected alternatives and failure modes
Shortcut	Decision	Failure
Replace the complete worker-plus-firm residual with the reduced quotient residual	R	Can accept bad reconstruction or incompatible full RHSs
Use one Frobenius residual for a whole block	R	A large neighboring column can mask a failed RHS
Certify only the fit or a sample of probes	R	Breaks the complete per-RHS contract
Reduce or adapt P to hit a time target	R	Changes the registered probe count and finite-projection estimator
Reuse the same atoms for leverage and target	R	Breaks the independent-domain covariance and MCSE construction
Pool physical copies before observation-level nonlinear correction	R	Changes the finite-P estimator; explicitly prohibited by the copywise derivation
Switch CMG to diagonal after a scientific atom has been drawn	R	Violates route-before-RNG and can make output depend on a failure path
Drop difficult RHSs or post partial targets	R	Contradicts fail-closed all-requested-target behavior
Edge sparsification, weight clipping, or heuristic deletion of weak links	R	Changes the Schur operator and can bias inverse actions
Ridge, pseudoinverse, or diagonal loading after rank/breakdown failure	R	Changes estimator and failure vocabulary; packet explicitly forbids such repair
Dense N×N, observation-space, or unrestricted F×F matrices	R	Violates the pure sparse/bounded design and memory API
Put all 601 or more RHSs in one matrix	R	Unbounded memory and loss of current admission guarantees
Naive recycle basis updated whenever a public batch completes	R	Makes output depend on batch() and scheduling
A hidden global hierarchy cache keyed only by dimensions	R	Stale scientific data can be silently reused
Accept a stale prepared handle and “verify later”	R	RNG and work may already have occurred before discovering mismatch
Comparator-specific looser tolerance	R	Weakens the registered residual contract
Mixed-precision actions with only a final double residual	Isolate	May be research-worthy, but changes recurrence behavior and failure rates; requires a separate qualification
Compiled plugin as the shipped answer	Isolate	Violates the requested pure Stata/Mata runtime boundary
External Python, MATLAB, MEX, or subprocess execution	R	Violates offline pure-Mata deployment
Firm-by-firm dense solves	R	Prohibited dense firm matrix and adverse F
2
/F
3
 scaling
Skip the direct deleted-information check near the rank boundary	R	Can accept a numerically nonestimable deletion
Use live wall-clock pilots for route selection	R	Can introduce host-load-dependent numerical paths and nondeterministic routing

The typed failure catalog already requires that failures not be repaired by an undisclosed ridge, changed component, changed deletion unit, reduced probe count, or relaxed tolerance. FILES/varcomp_kss/docs/FAILURES_AND_RETURNS.md:L97-L169.

A compiled sparse plugin could be maintained as a separate non-shipped research branch to estimate the Mata throughput ceiling. It is not a solution to the requested pure-Mata path and should never become an implicit runtime dependency.

8. Phased roadmap and uncertainty
Phase 0 — close the existing hierarchy qualification

Run the missing frozen API-5/API-7 complete-command d2/d3 A/B.

Run current API-7 full-command density-boundary sweeps at the former pathological scale.

Complete the required mathematical and provenance reviews.

Promote only if the existing ≤10% ordinary-regression gate and all construction/certificate gates pass.

This removes the catastrophic known regime before more speculative solver work.

Phase 1 — exact-output sparse execution improvements

Implement, in order:

retain deletion-unit and target-stratum scatter plans;

instrument scatter, certificate, graph-action, and allocation costs;

remove avoidable J()/copy allocation while retaining operation order;

prototype the flat CMG workspace, but do not promote unless it beats ordinary apply;

add the explicit internal plan object without yet exposing a public lifecycle.

These changes have the best ratio of falsifiability to scientific risk.

Phase 2 — streamed compressed kernel

Introduce quarantined fusion of prediction, full residual, and leverage/target contractions.

Add narrow dual incidence layouts only where memory admission permits.

Calibrate leverage and target batch widths separately.

Add fixed-checkpoint active-column compaction.

Target: at least a 1.25× numerical-core improvement on ordinary P200 with unchanged action count and full certificates.

Phase 3 — optional public prepared lifecycle

Expose explicit prepare/run/drop operations.

Bind the first version to all scientific inputs and options.

Keep RNG state invocation-local.

Report preparation time, persistent bytes, reuse count, and invalidation reason.

Red-team every caller-state and stale-handle path.

This phase is essential for approaching the matched cold-command comparator on repeated workloads. It must remain optional so one-shot command semantics remain simple and auditable.

Phase 4 — repeated-RHS algorithm research

Structural coarse initial seed.

Fixed structural deflation only after seed evidence.

Stage-local recycling with fixed logical epochs.

Fixed-width block microkernel last.

Each step must be independently promotable and independently killable. Do not combine block, deflation, and recycling into one opaque experiment.

Phase 5 — route, RNG, and exact-route cleanup

Versioned deterministic route coefficients and held-out regret test.

Byte-identical blocked RNG generation.

Structured exact-target actions.

Reassess whether remaining ordinary time is predominantly an irreducible Mata throughput limit.

At that point a non-shipped plugin experiment can be used only to estimate the compiled-kernel ceiling, not to replace the pure-Mata result.

What the packet cannot establish

Allocation/garbage-collection attribution. Stage timers do not reveal per-function allocation counts or bytes copied inside Mata.

The exact cost of repeated order()/panelsetup() in CZ18. Source shows the repeated calls, but no timer isolates them.

Current API-7 complete-command behavior at the former 625,000-worker d4 scale. The packet has strong local and smaller-scale hierarchy evidence, not that exact full-command rerun.

The missing frozen d2/d3 complete-command A/B. The packet explicitly identifies this as an unclosed promotion gate.

Spectral correlation among the 601 RHSs. Without stored Krylov/Ritz diagnostics, block and recycling gains are uncertain.

MATLAB’s internal action count and stage-equivalent kernel work. Licensed source is absent and the scientific paths differ.

How much of the 188.272 seconds is reusable import, grouping, graph selection, compression, or unavoidable validation.

Demand for repeated same-design commands. That determines the practical value of a public prepared lifecycle.

Hardware portability of batch and layout optima. The packet has strong receipts but not enough architectures to set universal coefficients.

Prevalence of exact-route workloads near exact_limit(). Exact-route target restructuring may be valuable but is not shown to affect the headline cases.

A universal asymptotic hierarchy theorem. The measured slopes and complexity gates are good finite evidence, not a proof.

Whether pure Mata can reach 43.802 seconds on the one-shot matched CZ18 boundary. The boundary decomposition shows what must improve, but not the attainable implementation ceiling.

Final decision

The most defensible numerical-architecture path is:

Immediately qualify and promote the existing robust API-7 hierarchy, because it appears to eliminate the catastrophic degree-four setup regime without changing the estimator or residual contract.

Make the compressed execution plan persistent within a command and optionally across commands, retaining scatter plans, hierarchy, and bounded workspaces. This is mandatory for addressing the 61.7% CZ18 boundary share.

Reorganize ordinary sparse work around plan-once, stream-many kernels, especially preplanned scatters, narrow dual layouts, flat CMG workspaces, and quarantined fusion of prediction, full certification, and contractions.

Attack weak graphs with fixed structural coarse seeding first, then deflation, recycling, and block PCG in increasing order of numerical and reproducibility risk.

Use deterministic calibrated route and batch models, never live wall time and never post-RNG route changes.

The packet supports a credible route to materially narrower parity—particularly for prepared and repeated-RHS comparisons—but it does not support a claim that one isolated optimization will achieve MATLAB command parity. The measured decomposition requires both boundary reuse and a multi-fold numerical-core improvement for the matched CZ18 P20 case, while the density-four and weak-connectivity regimes require separate algorithmic remedies.
