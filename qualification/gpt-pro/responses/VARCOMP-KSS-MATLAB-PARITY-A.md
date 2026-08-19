---
review_id: VARCOMP-KSS-MATLAB-PARITY-A
reviewer_surface: ChatGPT web through Chrome and native Computer Use upload
model: GPT-5.6 Sol
model_effort: Pro
review_date: 2026-08-18
source_commit: a405e7652da9a6657045090963b8b1ec5835a488
packet_sha256: fe003482ab187b65ed16c971281c8aa6571e9a31d74d1329173ca90a588c38ae
chat_url: https://chatgpt.com/c/6a84f63e-46d4-83ea-963b-471038339541
review_status: completed_independent
verbatim_body_sha256: 06c1ff146a20ca11f71151ec02c0a7677ae8c698a596521f6f377182c9f12649
transport_note: The direct Chrome-extension chooser timed out; the same signed-in Chrome session uploaded the exact packet through the native macOS picker. No browser console warning or error was recorded for this chat.
---

## Verbatim response

Independent review A — Mata systems path to MATLAB parity

I verified every entry in MANIFEST.sha256 against the extracted packet. The packet identifies commit a405e7652da9a6657045090963b8b1ec5835a488, contains 51 source/evidence files, excludes restricted rows and licensed MATLAB source, and excludes the other independent review (SOURCE_MAP.md:3–8).

1. Source-bound verdict
Verdict: sufficient to choose the next milestone

The evidence is sufficient to make a go decision on a two-front optimization milestone:

Reduce command-boundary retained-sample preparation, mapping, sorting, importing, and compression.

Improve repeated-RHS execution by retaining immutable aggregation plans, reducing workspace churn, and avoiding physical work on converged columns.

It is not sufficient to promise MATLAB timing parity or to justify another wholesale hierarchy-construction rewrite.

The matched CZ18 P20 arithmetic is decisive:

Current matched stage	Seconds	Share of 305 s	Maximum command speedup if made free
Import/selection	188.272	61.73%	2.613×
CMG hierarchy setup	6.442	2.11%	1.022×
Everything else	110.286	36.16%	—

Source: FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:288–311.

Consequences:

A 40% reduction in the reported selection stage would move 305 seconds to approximately 229.7 seconds, a 1.33× command gain.

A 60% reduction would move it to approximately 192.0 seconds, a 1.59× gain.

Even deleting the entire 188.272-second selection stage leaves 116.728 seconds, still 2.66× the descriptive MATLAB command time of 43.802 seconds.

Therefore preparation work is the first milestone, but preparation alone cannot close the observed command gap. Repeated-RHS numerical work must improve too.

Making current hierarchy setup free would save only 6.442 seconds. It is no longer the highest-value target on CZ18.

The density-four setup pathology is historical, but must remain a regression gate

The API-5 density-four cliff was real: the historical completed shape took 9,610 Stata seconds versus 84.951 MATLAB seconds, with 8,755.873 seconds in hierarchy setup (FILES/varcomp_kss/benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md:240–263). API 6 reduced the 15,625-firm density-four setup from 8,755.873 to 40.293 seconds and the 1,024-firm fixture from about 8.13 to 0.785 seconds (FILES/varcomp_kss/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md:69–72).

The current hierarchy ladder reports density-four setup of 1.147, 7.847, and 13.148 seconds at 1,024, 4,096, and 10,240 firms, with an endpoint slope of about 1.06 (CMG_MATA_1_2026-08-18.md:138–162). The source commit uses API 7, but API 7 is explicitly an ownership/interface successor that does not alter hierarchy construction or V-cycle algebra (FILES/varcomp_kss/cmg/STATUS.md:3–16; FILES/varcomp_kss/PLAN.md:11–19).

Thus:

Preserve density four, degrees two through seven, and the historical fixture as mandatory regression tests.

Do not make another hierarchy grouping rewrite the next primary milestone.

Revisit hierarchy construction only if new stage profiling shows it again dominates a registered workload.

Pure-Mata opportunities versus comparator advantages

The packet supports emulating these comparator advantages in pure Mata:

Persistent canonical orders, panels, component projections, and index plans.

Fewer Stata/Mata crossings and fewer complete-row imports.

Bounded reusable solver scratch.

Active-column packing after individual RHS convergence.

In-place or destination-buffer Schur and CMG kernels.

Wider, memory-admitted matrix RHS traversal where measured beneficial.

Fused leverage and target contractions.

The packet does not support expecting pure Mata to reproduce the comparator’s compiled-kernel throughput. The report identifies compiled adjacency, indexing, forest, and preconditioner-application kernels plus persistent workspaces on the MATLAB side (CMG_MATA_1_2026-08-18.md:235–244). Those compiled kernels cannot be imported under the stated runtime boundary. MATLAB also used materially more memory on the matched CZ18 job: a 10,379,636 KiB process-tree peak and 49.096 GiB maximum virtual memory, versus approximately 3.126 GB process peak and 2.912 GiB maximum virtual memory for Stata (CMG_MATA_1_2026-08-18.md:288–319). Some comparator performance may therefore come from both compiled execution and a different time–memory tradeoff.

Processor evidence does not justify a parallelism-first milestone

The only true local four-versus-eight processor comparison improved command times from 9.983 to 9.109, 9.938 to 9.126, and 10.078 to 9.350 seconds: approximately 7.8–9.6%. SCC could not expose eight processors (FILES/varcomp_kss/benchmarks/reports/KSS_PROD_1_2026-08-16.md:358–380).

By comparison, changing batch from 1 to 16 with actions fixed reduced numerical work from 42.796 to 16.418 seconds, a 2.607× gain (KSS_NUMOPT_2_2026-08-18.md:88–95). The evidence favors data movement, layout, batching, and reuse before processor-count work. It does not establish a hard Stata/MP ceiling, because the packet contains no kernel-level CPU profile.

Material caveats

The current matched CZ18 P20 timings are one Stata run and one MATLAB run.

The current report exposes selection and hierarchy setup, but not a complete exclusive breakdown of compression, semantic ranking, lifecycle transition, restoration, Schur, CMG application, leverage contraction, and target contraction.

sample_selection_seconds stops at varcomp_kss.ado:566, before semantic rank construction and its sort at lines 576–596. The reported 188.272 seconds therefore does not cover all command-boundary preparation.

The historical detailed profiles come from earlier commits. They are valuable for mechanism localization but are not current-stage percentages.

There is no allocation trace or counter for physical RHS-column work after some columns converge.

The MATLAB comparator differs in target weights, RNG atoms and scheduling, tolerance, correction algebra, and, for some synthetic cells, frequency semantics. Timing comparisons diagnose throughput, not equal numerical work (CMG_MATA_1_2026-08-18.md:194–207, 235–244).

The API-6 qualification omitted a frozen API-5/API-6 complete-command median comparison on the degree-two/three sub-minute cells; this is a promotion caveat, not a correctness caveat (CMG_MATA_1_2026-08-18.md:332–347).

Recommended milestone acceptance target

I would name the reviewer-proposed milestone PREP-RHS-1 and require all of the following:

At least 35% reduction from mark through completed compression on matched CZ18 P20, measured with new exclusive timers.

At least 15% reduction in repeated-RHS numerical time on the f1024 P200 CMG block, with identical planned and accepted RHS counts.

At least 1.25× matched CZ18 P20 command speedup over the current source.

No degree/row-shape hierarchy setup regression above 5% median, and no density-four cliff.

No estimator, sample, deletion, random-atom, route, action-accounting, residual, failure, resource, or restoration contract regression.

These are optimization gates, not claims of MATLAB output or runtime parity.

2. Critical-path reconstruction

Notation:

R: retained stored rows.

C: worker–firm coefficient cells.

G: deletion units.

S: exact target strata.

W,F: worker and firm levels.

P: probes.

B: batch width.

I: graph fixed-point passes.

L: CMG levels.

Total randomized solves are 1+P+2P=3P+1: 61 RHSs at P20 and 601 at P200.

Stage	Implementation location	Complexity and data movement	Packet evidence	Likely performance gap
Entry, runtime checks, state guards	FILES/varcomp_kss/varcomp_kss.ado:3–95, varcomp_kss; _vckss_impl begins at line 97	Constant work plus Mata loader/compile work on a source-cold call. Captures RNG and sort-jumbler state and reserves timers.	Caller RNG and runtime reset are explicit and mandatory.	Source-cold overhead should be reported separately, not optimized by weakening guards.
Requested/complete sample and option validation	varcomp_kss.ado:289–408, _vckss_impl	Multiple full-R count, summarize, markout, frequency, target, and FV expansion passes.	Current selection timer starts at line 289. CZ18 reports 188.272 seconds through post-prune ID preparation.	Avoidable full scans and Stata interpreter crossings, especially at high rows per cell. Failure ordering constrains fusion.
Initial IDs and stayer counts	varcomp_kss.ado:410–422	Two egen group() operations, a two-key sort, by generation, egen total, tag, and counts. Usually O(RlogR) plus several full-vector passes.	Optimization III already reduced default match egen group() passes from eight to five (KSS_NUMOPT_2_2026-08-18.md:53–60).	Five large egen/sort passes remain. Initial worker–firm pair topology is subsequently reconstructed in Mata helpers.
Exact total and graph import	varcomp_kss.ado:452–458, 486–511; FILES/varcomp_kss/varcomp_kss_graph.mata:175–220, vckss_graph__stata_prune	One frequency import for the exact total, then graph code imports the complete sample mask and four retained columns.	vckss_graph__stata_prune executes one full-sample st_data and four sample-specific imports at lines 215–220.	Duplicate import and index construction. The same numeric vectors are later imported again for compression.
Graph-pruning fixed point	varcomp_kss_graph.mata:243–403, vckss_graph__stata_prune; varcomp_kss.mata:2435–2710, vckss__largest_component, vckss__worker_firm_counts, vckss__worker_articulations	Approximately O(I(RlogR+V+E)) as implemented. Each pass may reconstruct pair ordering, components, worker counts, articulation CSR, or deletion-edge CSR.	The loop calls largest-component, pair-count, articulation, and bridge routines repeatedly at varcomp_kss_graph.mata:306–355.	Repeated sorts, pair coding, CSR allocation, and scalar loops. Benefit depends heavily on actual fixed-point pass count.
Retained total, ID redensification, semantic order	varcomp_kss.ado:528–596	A second retained-frequency import; two more egen group() calls; summaries; semantic target generation; egen group() across the semantic key; final sort.	Selection timing ends at line 566, while semantic ranking and sorting occur at lines 576–596.	Current headline selection timing understates complete preparation. Post-prune maps and row order are recomputed rather than returned by a retained Mata preparation state.
Compressed design construction	varcomp_kss.ado:619–755; FILES/varcomp_kss/varcomp_kss_scale.mata:479–741, vckss_scale__prepare; :1264–1356, vckss_srt__prepare	Seven separate st_data imports at lines 1285–1291; three canonical-ID sorts; cell, unit, stratum, worker, and firm ordering; compensated sufficient-statistic passes; several O(R) temporaries.	Rows-per-cell import coefficient is estimated at 2.620e-5 seconds per extra stored row and compression at 3.984e-7 per incremental RlogR−ClogC unit (KSS_NUMOPT_2_2026-08-18.md:265–282).	Re-imports vectors already used for graph selection. Several immutable grouping plans are built and then discarded by vckss_scale__compact.
Resource admission and data lifecycle	varcomp_kss.ado:785–920, 1278–1455	Resource forecasting; sample signature; disk-backed preserve; clear; numerical run; state reset; restore; signature and sample checks. Data movement is proportional to raw Stata dataset bytes.	Lifecycle is explicitly part of the resource contract (FILES/varcomp_kss/README.md:121–138). Current CZ18 report does not expose transition and restoration seconds separately.	Potentially significant at R/C>1, but presently unprofiled. Internal preparation reductions are safer than changing the lifecycle.
FE preparation and CMG cells/hierarchy	FILES/varcomp_kss/varcomp_kss_solver.mata:1581–1706, routed solver; vckss_cmg__cells_prepare, vckss_solver__hierarchy_cells	Cell preparation, preflight, deterministic hierarchy and terminal factors. Persistent hierarchy reused for all RHSs.	Current CZ18 hierarchy setup is 6.442 seconds. Current f1024 P20 setup is 0.351–4.324 seconds while PCG reaches 27.878 seconds (CMG_MATA_1_2026-08-18.md:178–187, 235–240).	Not the primary current gap. Retain per-level instrumentation and regression protection.
Fitted-outcome solve	FILES/varcomp_kss/varcomp_kss_scale_engine.mata:1137–1163, vckss_scale_eng__run_prepared; FILES/varcomp_kss/varcomp_kss.mata:1631–1975, vckss__fe_solve_matrix_backend	One FE RHS, followed by plug-in contractions and complete residual.	Historical CZ18 P200 fit was 36.871 seconds before later optimization (KSS_PROD_1_2026-08-16.md:289–312).	Small relative to 600 randomized RHSs, but it exercises the same allocation-heavy backend.
Leverage pass	varcomp_kss_scale_engine.mata:1165–1265	⌈P/B⌉ atom batches. Per batch: unit atoms, unit-to-cell scatter, FE transpose, batched solve, cell-to-unit prediction gather, residual matrices, five moment accumulations.	P200 means 200 leverage RHSs. Leverage/target stages reach 19.348/23.320 seconds on current f1024 P20 cells (CMG_MATA_1_2026-08-18.md:235–240).	Repeated sorting in scatter_sum, full-width solver allocations, and G×B temporary matrices.
Unit correction	varcomp_kss_scale_engine.mata:1267–1305; vckss_scale_eng__unit_adjust at :877–936	Scalar Mata loop over all G units, then unit-to-cell scatter.	Historical CZ18 P200 correction was 3,328.764 seconds, but that value includes nested leverage and target work (KSS_PROD_1_2026-08-16.md:300–312).	Scalar interpreter loop and repeated scatter plan creation. Exclusive current timing is absent.
Target pass	varcomp_kss_scale_engine.mata:1320–1416	⌈P/B⌉ atom batches but 2B solve columns per batch. Per batch: stratum-to-cell scatter, centering, full transpose, worker/firm balance, (W+F)×2B RHS allocation, solve, prediction, contraction.	P200 means 400 target RHSs. Current f1024 P200 command time grows only 1.9–7.3× relative to P20 despite 9.85× more RHSs, demonstrating useful batching/setup amortization (CMG_MATA_1_2026-08-18.md:120–136).	Workspace churn, repeated immutable scatter sorting, paired-column layout copies, and contraction temporaries.
Schur and preconditioner actions	varcomp_kss.mata:1692–1910; compressed callbacks at varcomp_kss_scale.mata:974–1063; CMG application at FILES/varcomp_kss/cmg/src/cmg_core.mata.in:2929–3017, 3314–3367	Each lockstep iteration applies a matrix Schur action and preconditioner. Logically inactive RHSs are zeroed, but physical actions retain the original matrix width. CMG recursively allocates projection, smoothing, action, residual, coarse-RHS, and correction matrices.	Optimization III reduced CZ18 P200 Schur time from 111.744 to 66.970 seconds with all 12,414 actions unchanged (KSS_NUMOPT_2_2026-08-18.md:107–119).	Strong evidence that layout/allocation matters independently of iterations. No counter currently measures physical zero-column work.
Full original-system residual	varcomp_kss.mata:1929–1960, vckss__fe_solve_matrix_backend	Coefficient reconstruction, prediction, weighted transpose and complete worker-plus-firm residual for every column. O(CB+(W+F)B) per solve call.	This is a mandatory acceptance contract (FILES/varcomp_kss/README.md:88–105; cmg/AGENTS.md:49–50).	It cannot be removed. Prediction and transpose buffers can be reused, but every RHS must still be certified.
Restoration and result posting	varcomp_kss.ado:1412–1455, 1634–2034	Reset compact state, clear, restore disk-backed dataset, validate signature/sample, construct matrices and ereturn post.	Point estimates only and no e(V) are explicit (FILES/varcomp_kss/docs/FAILURES_AND_RETURNS.md:3–15).	Restoration cost needs independent measurement. It cannot be omitted or silently cached away.
Main critical-path interpretation

The worsening at rows-per-cell above one is consistent with the source path:

The f1024 ratios rise materially from r1 to r8, reaching 4.38 at degree seven (CMG_MATA_1_2026-08-18.md:217–244).

CZ18 has 8,201,888 rows and 311,730 cells, approximately 26.3 rows per cell (KSS_NUMOPT_2_2026-08-18.md:97–105).

Before reaching the C-sized numerical representation, the implementation executes several R-sized Stata passes, repeated imports, sorts, group constructions, and then a disk-backed preserve/restore.

Therefore high R/C exposes a command-boundary gap that cannot be repaired by optimizing only the C-sized CMG hierarchy.

The repeated-RHS path already has an important strength: one matrix traversal serves all columns in a batch, and P200 demonstrates strong amortization. The missed reuse is inside and around those matrix traversals: immutable grouping plans are rebuilt, solve arrays are repeatedly allocated, CMG label projections re-sort each call, and converged columns remain present as zero columns in physical actions.

3. Ranked optimization portfolio

The speedup ranges below are planning estimates, not packet measurements. They are end-to-end ranges on the named decisive workload, overlap with one another, and must not be multiplied without remeasurement.

Cost: L = low, M = medium, H = high, VH = very high.

1. Consolidate the compressed-path retained-sample and import pipeline

Locations. FILES/varcomp_kss/varcomp_kss.ado:410–458, 486–596, 714–719; FILES/varcomp_kss/varcomp_kss_graph.mata:175–220, vckss_graph__stata_prune; FILES/varcomp_kss/varcomp_kss_scale.mata:1264–1293, vckss_srt__prepare.

Mechanism/type. Pass reduction, import reduction, sort reuse, and layout. Keep Stata’s validation and string-safe initial egen group() behavior, but import all compressed-eligible numeric columns once into a command-local Mata preparation state. Use the same vectors for exact totals, graph pruning, retained redensification, and compressed construction. Return graph_keep, final dense IDs, and diagnostics in one st_store operation. After the required semantic Stata sort, recover canonical order using a stable row token rather than re-importing all seven columns.

Expected effect. CZ18 P20 1.23–1.59× if it removes 30–60% of the measured 188.272-second selection stage. Confidence: medium because no substage profile exists.

Cost/risk/memory. H. Semantic risk: medium. Numerical risk: low. Memory must be neutral or lower relative to the current graph-plus-compression high-water: reuse or overwrite O(R) vectors rather than overlapping a second copy, and register exact bytes in Resource API.

Decisive benchmark. Matched CZ18 P20; f1024 r8 P20; fixed-C rows-per-cell ladder 1/2/4/8/26.3.

Kill criterion. Stop if the exclusive mark-through-compression stage falls by less than 35%, complete command improves by less than 1.25× on CZ18, or any sample, graph diagnostic, semantic rank, RNG atom, typed failure, or restored-state receipt differs.

2. Retain immutable unit-to-cell and stratum-to-cell aggregation plans

Locations. FILES/varcomp_kss/varcomp_kss_scale.mata:48–62, 85–137, design structs; :652–654, 668–670, plan construction; :1176–1202, vckss_scale__compact; FILES/varcomp_kss/varcomp_kss_scale_engine.mata:492–517, vckss_scale_engine__scatter_sum; calls at :1079–1088, 1195–1196, 1294–1296, 1350–1351.

Mechanism/type. Sort elimination and reuse. unit_cell_order/unit_cell_panel and strata.cell_order/cell_panel are constructed during preparation and then deliberately cleared by vckss_scale__compact. Retain them. Add a prepared scatter_plan with dense group count, identity flag, order, panel, and first-group vector, plus an _into reducer.

Expected effect. 1.03–1.15× on f1024 P200 and 1.02–1.10× on CZ18, depending on how often the identity shortcut currently fires. Confidence: medium-high.

Cost/risk/memory. M. Semantic and numerical risk: low if within-panel order remains identical. Additional persistent memory is approximately 8*(G+S)+32*C bytes for two order vectors and two C×2 panels; with CZ18’s G=S=C=311,730, about 15 MB. These arrays already exist temporarily, so this is a lifetime extension, not a new construction.

Decisive benchmark. f1024 P200 degree 3–7 at B32, plus CZ18 P20/P200. Count every order() and panelsetup().

Kill criterion. Stop if non-PCG leverage-plus-target time falls by less than 15% and command time by less than 3%, or if the cancellation fixture (1e16,1,-1e16) or target/accounting identities change.

3. Pack active RHS columns before Schur and preconditioner actions

Locations. FILES/varcomp_kss/varcomp_kss.mata:1695–1716, 1791–1910, vckss__fe_solve_matrix_backend; especially full-width calls at lines 1794, 1833, and 1868 and zeroing at 1826–1830, 1861–1862, and 1884–1887.

Mechanism/type. Batching and layout. Maintain all per-column PCG state and convergence decisions in original logical order, but gather only selectindex(active) columns into a bounded packed buffer for Schur and preconditioner calls, then scatter results back. Use a deterministic structural threshold to avoid packing when nearly every column remains active.

Expected effect. 1.00–1.18× end-to-end on f1024 P200; potentially larger on weak graphs with dispersed iteration counts. Confidence: low-medium until active-width counters exist.

Cost/risk/memory. H. Numerical risk: low-medium because a matrix call with fewer columns must reproduce the same per-column arithmetic. Semantic risk: low. A naïve implementation adds two F×B pack buffers; the preferred implementation aliases preallocated action/preconditioned workspace so peak memory remains neutral.

Decisive benchmark. f1024 P200 degrees 2–7, both r1/r8, with per-RHS iteration distributions and B1/B8/B32.

Kill criterion. Do not implement beyond instrumentation if logical active-column actions are at least 90% of full-width physical column actions. After implementation, kill if PCG time improves by less than 7%, packing consumes more time than it saves, or any RHS iteration/status/residual ordering changes unexpectedly.

4. Introduce a bounded FE solve workspace reused across fit, leverage, and target calls

Locations. varcomp_kss.mata:1678–1708, 1792, 1826–1837, 1929–1945, vckss__fe_solve_matrix_backend; callers at varcomp_kss_scale_engine.mata:1143–1145, 1197–1199, 1383–1392.

Mechanism/type. Allocation and reuse. Create a caller-owned workspace with capacity max(1, leverage_batch, 2*target_batch) for RHS partitions, quotient vectors, PCG state, action, packed columns, reconstructed coefficients, predictions, and residual certification. Add _into backend entry points. Reset only used columns.

Expected effect. 1.02–1.12× on P200; 1.01–1.07× on P20. Confidence: medium.

Cost/risk/memory. H. Aliasing and stale-column risk: medium. Numerical and semantic risk: low when initialization receipts are explicit. Peak memory should be neutral or lower because repeatedly returned matrices are replaced by one admitted arena.

Decisive benchmark. P200 B32 has one fit call, seven leverage calls, and seven target calls. Record allocation counts and allocated bytes by shape.

Kill criterion. Stop if counted large-matrix allocations do not fall by at least 70%, PCG-plus-certification time does not fall by 8%, or zero-width/stale-column tests fail.

5. Fuse the compressed Schur action into destination buffers

Locations. FILES/varcomp_kss/varcomp_kss_scale.mata:974–995, vckss_scale__op_schur_action; :1012–1063, worker/firm callbacks; FILES/varcomp_kss/varcomp_kss.mata:1260–1266, vckss__group_sum.

Mechanism/type. Layout, vectorization, and allocation reduction. The current path constructs fitted, a weighted C×B matrix, a worker mean, another C×B residual expression, and gathers values[row_order,.] inside each group sum. Add _group_sum_into and _schur_action_into routines that write into preallocated output and scratch, use the existing worker-major and firm-major plans, and avoid repeated whole-matrix return values.

Expected effect. 1.04–1.18× end-to-end where Schur is material. Confidence: medium-high; the previous round reduced CZ18 Schur time 40.07% with actions unchanged, proving that representation changes can materially improve this kernel (KSS_NUMOPT_2_2026-08-18.md:107–119).

Cost/risk/memory. H. Numerical risk: medium if summation order changes; preserve exact current row orders. Memory should fall by one or more C×B temporaries.

Decisive benchmark. Standalone Schur kernel and f1024 P200 degrees 3–7, reporting seconds per cell-column action.

Kill criterion. Stop if kernel time improves by less than 15%, command time by less than 5%, or actions/residuals/accepted estimator outputs exceed existing oracle tolerances.

6. Precompute the KSS firm-component projection plan once

Locations. FILES/varcomp_kss/cmg/src/cmg_core.mata.in:3314–3331, vckss_cmg__project_labels; calls at :3352–3361, vckss_cmg__apply_kss; workspace equivalent at FILES/varcomp_kss/varcomp_kss_solver.mata:777–816; context at :738–775.

Mechanism/type. Sort and associative-work elimination. Every CMG application validates labels using uniqrows(sort(label)), computes order(label), and calls panelsetup, twice per preconditioner application. Store the firm-component label, order, panel, component sizes, and validated dense-range receipt in vckss_solver_cmg_context. Replace both calls with project_labels_plan_into.

Expected effect. 1.01–1.08× command; 5–15% of preconditioner-application overhead where firms and action counts are large. Confidence: high that the work is redundant, medium on magnitude.

Cost/risk/memory. L–M. Risk: low. Memory: O(F), approximately one order vector and one component panel, with no F×B persistent matrix.

Decisive benchmark. CMG apply microbenchmark at F=1,024, 10,603, and larger, B1/B8/B32; record label sorts per application.

Kill criterion. Stop if preconditioner_seconds improves by less than 5% on at least four of the six f1024 degree 2–7 cells or any component-mean projection differs.

7. Fuse leverage post-solve gathering and moment accumulation

Locations. FILES/varcomp_kss/varcomp_kss_scale_engine.mata:1207–1214; vckss_scale_eng__moment_add at :653–697.

Mechanism/type. Allocation reduction, tiling, and fusion. The current code materializes unit_projection, unit_random_scaled, and unit_random_residual, then creates square and product tiles. Traverse bounded unit tiles, gather prediction, form the two scalar expressions, and update the five compensated accumulators directly. Preserve probe-column order and the existing width-eight moment tiles.

Expected effect. 1.02–1.10× command on P200, with a larger memory benefit than wall benefit. Confidence: medium; this code was already optimized in Optimization III.

Cost/risk/memory. M. Numerical risk: medium because moment regrouping is sensitive. Memory reduction can approach three G×B matrices plus temporary square/product matrices, replaced by bounded unit tiles.

Decisive benchmark. Leverage-only kernel at G=30,000, 311,730, and f1024 synthetic shapes; B8/B16/B32.

Kill criterion. Stop if exclusive leverage post-solve time improves by less than 10%, peak leverage scratch by less than 15%, or compensated-moment and finite-projection oracle tests fail.

8. Reuse and fuse target-stage RHS and contraction buffers

Locations. varcomp_kss_scale_engine.mata:1323–1415; vckss_scale_eng__balance_score at :549–575; vckss_scale_eng__target_contract at :726–760.

Mechanism/type. Allocation, layout, and reuse. Preallocate the maximum (W+F)×2B target RHS. Write balanced worker scores directly into odd columns and firm scores directly into even columns. Reuse centering and reference-scale buffers. Pass prediction directly to a tiled contraction destination instead of constructing aliases and returned matrices.

Expected effect. 1.02–1.10× command on P200. Confidence: medium.

Cost/risk/memory. M. Numerical and semantic risk: medium because odd/even column identity, worker/firm balancing, and target draw order are registered. Memory should be neutral or lower.

Decisive benchmark. Exclusive target non-PCG time on f1024 P200 and CZ18 P200, with 2P logical RHS accounting.

Kill criterion. Stop if target non-PCG time improves by less than 10%, command by less than 3%, or target draw rows, identities, MCSE, or logical RHS numbering change.

9. Build a new width-specific CMG application arena; do not enable the current workspace

Locations. FILES/varcomp_kss/cmg/src/cmg_core.mata.in:2384–2459, projection/smoothing/coarse routines; :2929–3017, recursive ordinary apply; existing workspace at :2814–2866, 3194–3247.

Mechanism/type. Allocation and bounded workspace. The ordinary recursion repeatedly creates compatible, out, action, residual, coarse_rhs, and correction. The existing three-matrix-per-level fixed-capacity workspace is slower and zeroes unused columns. Design a new arena around the measured width, destination-buffer graph actions, and two or three explicitly aliased matrices per active level. It must preserve the same symmetric V-cycle sequence.

Expected effect. 1.05–1.25× command on CMG-heavy synthetic cells; 1.02–1.12× on current CZ18. Confidence: low-medium because the prior workspace experiment regressed.

Cost/risk/memory. VH. Numerical risk: high: aliasing can destroy fixed linearity, symmetry, or quotient-SPD. Memory must be forecast as 8*k*B*sum(level_vertices) plus graph-action scratch, with a fixed small k.

Decisive benchmark. CMG apply kernels at B1/B4/B16/B32 for degrees 2–7 and disconnected/adversarial graphs, followed by full PCG tests.

Kill criterion. Require at least 15% apply improvement at B1, B8, and B32 with no target cell regressing more than 3%. Kill on any symmetry, linearity, component, residual, or memory-receipt failure.

10. Reuse graph topology across the pruning fixed point

Locations. FILES/varcomp_kss/varcomp_kss_graph.mata:243–390, vckss_graph__stata_prune; FILES/varcomp_kss/varcomp_kss.mata:2459–2710, component/count/articulation helpers.

Mechanism/type. Algorithmic reuse and layout. Construct worker–firm coordinate edges, deletion-unit edges, deletion panels, and CSR topology once. Maintain row, edge, and vertex active masks as the fixed point removes units or workers. Component, worker-degree, articulation, and bridge traversals then operate on the same topology rather than re-sorting selected rows and rebuilding CSR.

Expected effect. Typical command 1.00–1.10×; graph-heavy multi-pass cases could see 1.3–2.0× graph-stage improvement. Confidence: low-medium because current CZ18 graph substage timing and pass count are not exposed.

Cost/risk/memory. H–VH. Semantic risk: high because the exact fixed point, tie handling, parallel deletion units, and first failure must remain unchanged. Memory O(E+V+G), replacing repeated temporaries rather than adding an observation-square object.

Decisive benchmark. Adversarial fixtures requiring multiple insufficient-worker, articulation, and bridge passes; current CZ18; graph-only timers.

Kill criterion. Stop if multi-pass graph time falls by less than 25%, CZ18 command by less than 3%, or any retained mask or graph diagnostic differs.

11. Add path compression and bulk root accounting to largest-component selection

Locations. FILES/varcomp_kss/varcomp_kss.mata:2435–2441, vckss__union_find_root; :2498–2557, vckss__largest_component.

Mechanism/type. Scalar-loop and algorithmic improvement. The root finder walks parent chains without path compression. Add a pointer-based path-halving or full-compression helper while leaving edge and union order unchanged. Compute final root vectors once before firm-count, mass, and retained-row passes.

Expected effect. 1.00–1.03× command; 15–40% of largest-component time on adverse graphs. Confidence: medium for the local kernel, low for command impact.

Cost/risk/memory. L–M. Risk: low-medium. Root identity must remain the result of the unchanged union order. Memory: one V-length root vector.

Decisive benchmark. Long-chain and barbell graph fixtures plus current graph ladder.

Kill criterion. Stop if largest-component time improves by less than 15% or any tie/selected-component result changes.

12. Vectorize deletion-unit adjustment while preserving first-failure behavior

Locations. FILES/varcomp_kss/varcomp_kss_scale_engine.mata:877–936, vckss_scale_eng__unit_adjust; scalar caller loop at :1275–1293.

Mechanism/type. Vectorization. Evaluate the scalar formulas for all units as vectors, construct an exact per-unit status code in the same within-unit priority order, and select the first failing unit in canonical unit order. Compute valid deleted masses and reciprocal residuals in bulk.

Expected effect. 1.00–1.05× command, potentially 15–40% of the exclusive unit-adjustment section. Confidence: medium.

Cost/risk/memory. M. Failure-contract risk: medium. Numerical risk: low-medium. Memory: a bounded number of G-length vectors.

Decisive benchmark. CZ18 P20/P200 correction-only timer and synthetic cases with a failure injected at the first, middle, and last unit for every typed status.

Kill criterion. Stop if exclusive unit-adjustment time improves by less than 15%, command by less than 2%, or the first typed failure/message changes.

13. Replace scalar semantic group minima with a bulk exact-rank reducer

Locations. FILES/varcomp_kss/varcomp_kss_scale.mata:1218–1247, vckss_srt__group_min and vckss_srt__ranks; calls at :1333–1338.

Mechanism/type. Scalar-loop reduction and sort consolidation. vckss_srt__group_min runs one Mata interpreter loop per deletion unit and target stratum; uniqueness then performs uniqrows(sort()). Implement one bulk (group, semantic_rank) ordering or a tiled exact-min reducer, producing minima and uniqueness receipts together.

Expected effect. 1.00–1.04× command; potentially material inside compression when G,S are hundreds of thousands. Confidence: medium-low without a substage timer.

Cost/risk/memory. M. RNG-contract risk: medium because these exact ranks identify random atoms. Memory O(G+S) plus bounded sorting scratch already charged to compression.

Decisive benchmark. Compression-only CZ18 and G/S scale ladder, with exact unit and stratum rank-vector equality.

Kill criterion. Stop if compression improves by less than 5% or any rank, atom, batch-invariance, or caller-RNG test differs.

14. Recalibrate the static compressed batch policy, including B64

Locations. FILES/varcomp_kss/varcomp_kss.ado:785–807.

Mechanism/type. Batching. The loop considers 16/32/64, but the compressed path hard-caps selected candidates at 32. Run a fixed offline calibration and encode a structural, memory-admitted policy—never a runtime timing probe. Keep sequential logical atom generation and route-before-RNG behavior.

Expected effect. 1.02–1.12× on some P200 cells; confidence: low-medium. Existing evidence establishes large gains through B16 but does not establish B64 superiority for CMG.

Cost/risk/memory. L. Numerical risk: low under the registered regrouping tolerances; RNG risk: low if logical probe order is unchanged. Memory rises according to existing B-linear formulas and must pass complete resource admission.

Decisive benchmark. B8/B16/B32/B64 over f1024 degrees 2–7, r1/r8, P20/P200 and both diagonal/CMG routes.

Kill criterion. Do not change the default unless median command improvement is at least 8%, no registered cell regresses more than 5%, and every memory forecast and atom identity passes.

15. Conditional deterministic graph-derived deflation or recycling

Locations. FILES/varcomp_kss/varcomp_kss.mata:1631–1975; CMG hierarchy data in FILES/varcomp_kss/cmg/src/cmg_core.mata.in.

Mechanism/type. Algorithmic. If action counts remain dominant after layout changes, construct a small deterministic deflation space solely from graph/hierarchy structure before random probes. Apply it identically for every RHS and batch. Do not derive the space from earlier random RHSs, iteration history, or batch membership.

Expected effect. 1.10–1.50× numerical stage if Schur/preconditioner actions fall at least 25%. Confidence: low.

Cost/risk/memory. VH. Numerical and semantic risk: high. The augmented method must remain a fixed linear symmetric quotient-SPD operation and must preserve per-RHS PCG and full residual gates. Memory is bounded by O(kF+kB) for small admitted k.

Decisive benchmark. Weak-connectivity P200 cells with high action counts, then all degree/adversarial CMG gates.

Kill criterion. Stop if actions fall by less than 25%, numerical time by less than 15%, results become batch-dependent, or fixed-linearity/symmetry/residual tests fail.

16. Optional explicit prepared-data public lifecycle — separate owner decision

Locations. Current lifecycle at FILES/varcomp_kss/varcomp_kss.ado:1278–1455; public contract at FILES/varcomp_kss/README.md:121–138.

Mechanism/type. Public lifecycle/API. An explicit prepare, estimate using prepared_handle, and release interface could amortize selection, graph construction, compression, and perhaps preservation over repeated estimates on an unchanged frozen dataset. The handle must bind a data signature, sample options, deletion rule, target, controls, build IDs, and memory receipt. There must be no invisible automatic cache.

Expected effect. Repeated identical-data calls could improve by approximately 1.5–2.6×; one-shot MATLAB comparisons gain nothing. Confidence: medium for repeated-use economics, not for adoption.

Cost/risk/memory. VH. State, stale-data, restoration, and public-API risk: high. Persistent memory/disk must be explicit and user-releasable.

Decisive benchmark. Ten repeated estimates with only probe count/seed changed, compared with ten ordinary safe commands.

Kill criterion. Do not ship unless repeated workloads gain at least 1.5×, every stale-handle mutation is rejected, and one-shot command behavior remains unchanged. This must not be bundled with the safe internal optimization milestone.

4. Top three implementation sketches
4.1 Consolidated compressed-path preparation state

The first implementation should preserve Stata’s current input-validation and arbitrary string-ID behavior. It should not begin by reimplementing all egen group() semantics in Mata.

Proposed structures and flow
mata
struct vckss_prep_state {
    string scalar status
    string scalar message


    real colvector row_token
    real colvector worker0
    real colvector firm0
    real colvector deletion0
    real colvector frequency
    real colvector outcome
    real colvector target


    real colvector active
    real colvector worker
    real colvector firm
    real colvector deletion


    real scalar exact_initial_mass
    real scalar exact_retained_mass
    struct vckss_graph_receipt scalar graph
    struct vckss_scale_design scalar design
}


void vckss_prep__freeze_from_stata(
    string scalar touse,
    string scalar row_token,
    string scalar initial_worker,
    string scalar initial_firm,
    string scalar deletion,
    string scalar frequency,
    string scalar outcome,
    string scalar target)
{
    idx = selectindex(st_data(., touse) :== 1)


    // One matrix import, followed by column views/copies whose lifetimes

The current semantic egen group() and sort can remain initially. Before any sort, generate a stable exact row token. After the semantic sort:

mata
void vckss_prep__adopt_semantic_order(
    string scalar touse,
    string scalar row_token,
    string scalar semantic_rank)
{
    ordered = st_data(., (row_token, semantic_rank), touse)


    // Map the required canonical Stata order back to cached numeric columns.
    p = inverse_permutation_from_exact_token(
        state.row_token, ordered[.,1])


    reorder_active_state_in_place(p)
    state.semantic_rank = ordered[.,2]


    state.design =
        vckss_scale__prepare(
            state.worker, state.firm, state.deletion,
            state.frequency, state.outcome, state.target,
            state.rank_tolerance)
}

This gives a safe first optimization:

One main compressed-path import instead of graph imports followed by seven compression imports.

One retained numeric state across graph pruning and compression.

Stata remains the oracle for semantic rank and canonical row ordering.

No public cross-command cache is introduced.

The state is reset on every exit by the existing outer guard.

A later, separately gated step could implement compressed-no-control semantic ordering directly in Mata, but only after exact rank-vector equivalence has been demonstrated.

Required invariants

touse before and after must match exactly.

Initial and retained exact physical totals must equal current values.

Graph active mask and all graph diagnostics must match.

Match versus observation deletion paths remain separate; this optimization initially applies only to compressed-eligible match deletion.

Final worker, firm, and deletion IDs must identify the same equality classes and canonical ordering.

Semantic rank vector must be exactly identical.

No random provider is initialized during preparation.

Every allocation and high-water overlap is added to the resource model.

State reset, data restoration, and RNG restoration remain mandatory on every exit.

Required tests

Numeric and string worker/firm/deletion IDs.

Explicit deletionid() and default match deletion.

Missing requested rows and MATCH_INPUT_MISSING.

Frequency weights, target weights, exact 2
53
 boundary.

Stayers and mover-only samples.

Multi-iteration insufficient-worker/articulation/bridge fixed points.

Tied largest components and first typed failure.

Row permutations and repeated physical-copy regrouping.

Optional probeorder().

P20/P200; B1/B32; diagonal/CMG.

Exact equality of sample, graph fields, semantic ranks, atom vectors, action counts, accepted results within existing oracle gates, and caller state.

4.2 Immutable aggregation-plan API

The plan should be built once from a validated dense group vector.

mata
struct vckss_scatter_plan {
    string scalar status
    real scalar input_rows
    real scalar groups
    real scalar identity
    real colvector order
    real matrix panel
    real colvector first_group
}


struct vckss_scatter_plan scalar
vckss_scatter_plan__build(real colvector group, real scalar groups)
{
    plan.input_rows = rows(group)
    plan.groups = groups
    plan.identity =
        (rows(group) == groups & all(group :== (1::groups)))


    if (plan.identity) {
        plan.status = "CONVERGED"
        return(plan)
    }


    plan.order = order(group, 1)
    plan.panel = panelsetup(group[plan.order], 1)
    plan.first_group = group[plan.order[plan.panel[.,1]]]


    // Validate dense bounds, integer identity, and unique panel labels once.
    ...
    return(plan)
}


real scalar vckss_scatter_plan__sum_into(
    struct vckss_scatter_plan scalar plan,
    real matrix values,
    real matrix out)

Add to the compact design:

mata
struct vckss_scale_design {
    ...
    struct vckss_scatter_plan scalar unit_to_cell_plan
    struct vckss_scatter_plan scalar stratum_to_cell_plan
}

Build them from unit_cell and strata.cell in vckss_scale__prepare and do not clear them in vckss_scale__compact. Replace every call in vckss_scale_eng__run_prepared.

The same API can later support CMG’s firm-component projection, but that should be a separate small change because projection subtracts means rather than scattering sums.

Required invariants

Same canonical within-group row order.

Same Neumaier compensation path for cancellation-sensitive data.

Exact integer totals continue using the existing exact integer reducer.

Sparse group labels still produce explicit zero rows where permitted.

Identity plans return the same data without aliasing a buffer that will be mutated.

The plan is immutable after preparation.

Its bytes are included in persistent-state and transition high-water receipts.

Required tests

Identity group vector.

Every group represented once.

Highly unbalanced panels.

G>C, S>C, multiple units or strata per cell.

Cancellation case (1e16,1,-1e16).

Positive exact integer totals near 2
53
.

B1/B8/B32 and final partial batches.

Old/new equality for all four startup certifications, leverage moments, cell correction weights, target directions, target identities, and typed failures.

4.3 Repeated-RHS execution context with active-column packing

Implement this in three reversible steps:

Add counters and a projection plan.

Add reusable FE buffers without active packing.

Add active packing only if measured inactive-column waste is material.

mata
struct vckss_fe_solve_workspace {
    real scalar capacity


    real matrix worker_rhs
    real matrix full_firm_rhs
    real matrix full_rhs
    real matrix reduced_rhs


    real matrix coefficient
    real matrix residual
    real matrix preconditioned
    real matrix direction
    real matrix action


    real matrix packed_argument
    real matrix packed_result
    real colvector active_index


    real matrix prediction
    real matrix full_residual
}


struct vckss_solve_result scalar
vckss__fe_solve_matrix_backend_ws(
    struct vckss_fe_design scalar design,
    real matrix rhs,
    real scalar tolerance,
    real scalar maxiter,
    struct vckss_solver_backend scalar backend,
    pointer(struct vckss_fe_solve_workspace scalar) scalar ws)
{
    width = cols(rhs)
    initialize_used_columns(ws, width)


    // Current column-specific initialization and exact-zero handling.
    ...

The packing threshold must be a deterministic structural rule fixed by offline benchmarks. It must not depend on measured runtime, route timing, or random values.

For CMG, extend vckss_solver_cmg_context with:

mata
struct vckss_label_projection_plan scalar firm_component_plan

and use it before and after every V-cycle. Do not enable the existing workspace as part of this step; the packet already shows it is slower.

New counters

Logical Schur RHS actions: existing active-count total.

Physical Schur column actions: actual columns passed to kernels.

Logical preconditioner applications.

Physical preconditioner column applications.

Active-width integral by iteration.

Pack/unpack bytes.

Large-matrix allocation count and bytes.

Explicit residual replacement count by RHS.

Complete residual certification bytes and seconds.

Required invariants and tests

Original logical column order never changes.

Random atoms and target odd/even RHS identities never change.

Per-RHS recurrence, stopping decision, iteration number, and typed failure remain independent.

Explicit quotient residual is still recomputed at iteration 100, 200, and so on for every active RHS.

Complete original worker-plus-firm residual remains mandatory for every column.

Zero RHSs, initially converged RHSs, and columns converging on different iterations are covered.

Test batches with active columns at the beginning, middle, and end.

Test one RHS exceeding maxiter() while neighbors converge.

Test PCG breakdown in a non-first column.

Test exact-terminal backend.

Test disconnected components and singleton compatibility.

Compare B1, B2, B8, B32 and final partial batches.

Require unchanged route, atom streams, logical action counts, accepted status, complete residuals, and caller RNG/data state.

5. Measurement plan
5.1 Cold and warm boundaries

Report at least four boundaries rather than one ambiguous “cold” time:

Source-cold command: fresh Stata process; Mata runtimes not loaded; prepared DTA already present locally. Includes loader and Mata compilation.

Source-warm command: same Stata process after one nonmeasured load/compile call; restore or reload the identical DTA before every measured command.

Filesystem-first-read command: first command after copying the DTA to a unique path. This is a cache proxy; do not claim OS page cache was flushed unless the host actually provides that control.

Numerical-kernel warm: prepared compact state and hierarchy already constructed inside an isolated harness; measures Schur, CMG apply, leverage reduction, and target contraction separately.

For MATLAB, report pool startup, data import/conversion, command call, serialization, and teardown separately. Use the maintained command boundary for the headline comparison, just as the current report does.

5.2 Exclusive stage timers

Existing nested timers must not be summed. Add exclusive timers for:

Parse/runtime guards.

Requested sample and frequency/target validation.

FV expansion and markout.

Initial worker/firm grouping.

Stayer pair sort/count.

Exact initial physical total.

Graph import.

Deletion-panel validation.

Initial component.

Mover filtering.

Each fixed-point category: component, worker count, articulation, bridge.

Retained total.

Post-prune ID redensification.

Semantic key generation.

Semantic egen group.

Semantic sort.

Compression imports.

Canonical worker/firm/deletion maps.

Cell construction.

Unit construction.

Stratum construction.

Sufficient statistics.

Semantic unit/stratum minima.

Resource modeling.

Disk preserve/clear transition.

FE base preparation.

CMG cell preparation, preflight, hierarchy construction, and terminal factors, per level.

Fit solve and fit contractions.

Leverage RNG, scatter, transpose, PCG, gather, and moments.

Unit adjustment and unit-to-cell correction scatter.

Target RNG, scatter, centering, transpose/balance, RHS construction, PCG, and contraction.

Complete full-system residual certification.

Compact-state release.

Data restoration/signature check.

Return posting.

The current selection timer ending before semantic ranking makes this decomposition especially important (varcomp_kss.ado:566–596).

5.3 Operation and data-movement counters

Record:

st_data and st_store call count.

Rows, columns, and total scalar elements imported or stored.

Number of egen group, sort, Mata order, and panelsetup operations.

Elements sorted by each operation.

Graph fixed-point passes and calls to largest-component, pair count, articulation, and bridge routines.

Pair and deletion edges constructed per pass.

Stable reducer panels, offsets, and tile counts.

Unit-to-cell and stratum-to-cell scatter calls.

CMG component-label sorts and projections.

CMG level visits, edge passes, and graph-action chunks.

Logical and physical Schur/preconditioner column actions.

Per-RHS iteration distribution, not only the maximum.

Active width at every PCG iteration.

Explicit residual replacement and restart counts.

RNG atom counts by domain, probe, and batch.

Complete residual certifications by stage and RHS.

5.4 Allocation and workspace receipts

Mata does not provide a complete allocator profiler in the packet, so initially add explicit receipts around every material J() or returned matrix:

Shape, width, predicted bytes, phase, and lifetime.

Persistent versus scratch classification.

Maximum simultaneous admitted bytes by phase.

Number of allocations by shape class.

Bytes copied into slices such as values[row_order,.].

FE workspace capacity and used width.

CMG arena bytes by level.

Pack/unpack bytes.

Transition and restoration high-water receipts.

Process RSS and qacct maximum virtual memory as external checks, not replacements for direct allocation accounting.

A successful instrumented run should reconcile predicted live bytes with Resource API’s phase model and explain any persistent process-RSS high-water caused by retained allocator arenas.

5.5 Task matrix

Use the following minimum matrix:

Matched production holdout

CZ18 P20, exactly current DTA hash, seed, tolerance, route policy, memory envelope, and four actual processors.

CZ18 P200 for accepted candidates when resource and scheduling gates permit.

Synthetic command matrix

Firms: 16, 32, 64, 256, 1,024.

Degrees: 2–7.

Rows per cell: 1 and 8.

P20 and P200 at f1024.

Include weak-connectivity cases.

Hierarchy-only matrix

Firms: 1,024, 4,096, 10,240.

Degrees 2–7.

Historical 15,625-firm density-four fixture.

All 13 adversarial graph families.

Rows-per-cell isolation

Hold C,W,F,G,S fixed.

R/C=1,2,4,8,26.3.

Separate frequency-compressed rows from literal repeated rows where semantics allow, and label the distinction.

Batch and processor matrix

Batch 1, 2, 4, 8, 16, 32, 64.

Processor count 1, 2, 4, 8 where actually available.

Report requested and actual processor counts.

Graph fixed-point fixtures

No removals.

Multiple insufficient-worker passes.

Multiple articulation passes.

Multiple bridge passes.

Alternating removal categories.

Tied largest components.

Parallel deletion units and cross-coordinate invalid units.

5.6 Repetitions and statistics

Small kernels and small complete commands: one warm-up plus seven measured repetitions.

f1024 P20/P200 cells: at least five fresh-process command repetitions for the decisive cells and three for the remainder.

CZ18 accepted baseline and candidate: at least three complete fresh-process runs each, interleaved baseline/candidate rather than all baselines first.

Preserve every run; do not drop outliers.

Report median, minimum, maximum, median absolute deviation, host, runtime version, actual processors, and memory receipts.

Attribute node/host differences descriptively, not causally, unless controlled.

5.7 Fair MATLAB comparison

Match and report:

Exact input DTA hash.

Stored-row count and row/cell shape.

Worker/firm/cell dimensions.

P label and seed label.

Four effective computational workers/processors.

Numerical-library oversubscription policy.

Command boundary versus wrapper/import/pool boundary.

Cold/warm status.

Process-tree memory and virtual memory.

Every comparison table must also state that target weights, random atoms and schedule, tolerance, correction formula, and some frequency semantics differ. Do not use timing similarity as evidence of numerical-output equality.

6. Rejected ideas
Tempting proposal	Why rejected
Another density-four hierarchy rewrite as the next milestone	API 6 removed the measured cliff, current density-four setup passes the scale ladder, and current CZ18 hierarchy setup is only 2.11% of command time. Keep it as a regression gate instead.
Turn on the existing reusable CMG workspace	It was 34% slower at B4, 74% slower at B16, and about 63% slower in earlier large tests (FILES/varcomp_kss/docs/NUMERICAL_ARCHITECTURE.md:314–320). A new arena must prove itself against ordinary apply.
Add a C plugin, MEX, Python, MATLAB, GPU, or external executable	Violates the pure Stata/Mata 18/19 runtime boundary. The current package explicitly loads no compiled helper (CMG_MATA_1_2026-08-18.md:16–30).
Form a dense firm-by-firm inverse or observation-space projection	Violates the explicit bounded-memory and no-dense-F×F/observation-space contracts (FILES/varcomp_kss/cmg/AGENTS.md:42–54).
Reduce probes, change tolerance, stop on graph residual, or skip complete residuals	Silently changes the estimator or acceptance contract. Every RHS must pass the original worker-plus-firm residual.
Use one Frobenius residual for a whole batch	A successful neighboring column must not mask a failed RHS; per-RHS acceptance is registered.
Change frequency expansion, delete observations instead of matches, or approximate target strata	Violates literal-copy, deletion-unit, and exact target-scale semantics (FILES/varcomp_kss/README.md:50–86).
Generate random probes in parallel batches or reorder probe consumption	Risks random-atom, stream, probe-order, batch-invariance, and caller-state contracts (README.md:107–119).
Recycle Krylov vectors from earlier random RHSs	Makes the effective solve dependent on batch partition and prior random columns unless very carefully reformulated. It is unsupported by current evidence and should not be confused with deterministic graph-derived deflation.
Couple RHSs with block CG stopping or curvature decisions	The registered implementation uses independent scalar recurrences in lockstep. Coupled block decisions can change per-RHS failures and iteration acceptance.
Default to a larger batch solely because B16 beat B1	B64 is unqualified for the CMG matrix, memory grows linearly with width, and wider matrices can be cache- or allocator-hostile. Use a static benchmarked policy.
Treat eight processors as the route to parity	The only true 4→8 evidence is a roughly 8–10% command gain, much smaller than batching and layout gains.
Introduce an invisible cross-command cache	Risks stale data, changed restoration semantics, and hidden memory. An optional prepared-data lifecycle requires explicit public ownership and signatures.
Make destructive or out-of-core operation the default	Violates the existing safe preserve/restore lifecycle and is explicitly a separate owner decision (KSS_NUMOPT_2_2026-08-18.md:330–335).
Remove caller data, RNG, or sort-state restoration checks	These are registered success/failure contracts, not optional diagnostics.
Replace compensated reductions with ordinary sums	The package has adversarial cancellation gates; this could change finite-projection and target accounting behavior.
Move expensive work outside an existing timer and report a faster stage	Merely changes attribution. All optimization gates must use complete-command and exclusive stage time.
Infer numerical parity from the MATLAB timing cells	The comparator does different numerical work and has no equality gate.
Optimize out.target_draws or the final P×4 mean first	These are small compared with RHS solving, scattering, and prediction. There is no packet evidence that they are material.
7. Phased roadmap
Phase	Reversible checkpoint	Required evidence before proceeding
0. Instrumentation	Add exclusive stage timers, import/sort counters, allocation receipts, active-width counters, and physical column-action counters. No algorithm change.	Timer coverage reconciles with command wall; nested versus exclusive stages are clear; counters stable across repetitions; all existing tests pass.
1. Low-risk immutable reuse	Retain unit/stratum scatter plans; precompute CMG firm-component projection plan; remove the dead fitted = firm_coefficient[design.firm,.] assignment at varcomp_kss.mata:1934; bulk semantic minima if independently profitable.	At least 5% gain on one decisive f1024 P200 cell, no cell regression above 3–5%, exact plan/rank equality, unchanged memory admission and contracts.
2. Command-boundary preparation	Introduce command-local preparation state, consolidate imports, reuse graph vectors, return final maps, and feed compressed construction without seven re-imports. Keep Stata semantic sorting initially.	At least 35% reduction from mark through compression and 1.25× CZ18 P20 command gain; exact sample, graph, rank, RNG, failure, resource, and restoration receipts.
3. Repeated-RHS workspace	Add bounded FE solver workspace and destination-buffer backend. Do not yet alter active width.	At least 70% reduction in material allocations and 8% PCG/certification gain; no stale-column or aliasing failure.
4. Active-column actions	Enable deterministic packing only where Phase 0 proves at least 10% physical zero-column waste.	At least 7% PCG gain on the gated cells; same logical actions, per-RHS iterations/statuses, complete residuals, and atom order.
5. Schur, leverage, and target fusion	Add _into Schur grouping, leverage tile fusion, target RHS reuse and contraction fusion one at a time.	Each change clears its own kill criterion; cumulative f1024 P200 numerical gain at least 15%; CZ18 remeasured after every change.
6. New CMG application arena	Only if preconditioner applications remain a leading current stage after Phases 1–5. Implement a measured-width, bounded, alias-audited arena—not the existing workspace.	At least 15% kernel gain across B1/B8/B32 and no degree/adversarial regression; fixed linearity, symmetry, quotient-SPD, and memory proofs/tests pass.
7. Deterministic deflation	Only if action count, rather than per-action throughput, remains dominant.	At least 25% action reduction and 15% numerical gain with batch-invariant fixed structural basis and unchanged complete residual/failure contracts.
8. Optional prepared-data lifecycle	Separate public owner decision after internal one-shot work is exhausted.	Repeated-call gain at least 1.5×, explicit signatures and release, no stale-state acceptance, unchanged ordinary command semantics.

Every phase should be committed and benchmarked separately. Do not combine a preparation rewrite, active-column PCG, and CMG arena in one candidate: a regression would be difficult to localize, and the performance evidence would not reveal which mechanism succeeded.

8. Uncertainty

Current selection decomposition. The packet gives 188.272 seconds through post-prune ID setup but not separate times for validation, egen, Stata sorts, graph import, graph fixed point, retained total, and post-prune redensification.

Untimed semantic preparation. Semantic rank and final sort occur after the selection timer stops, so complete preparation may be larger than the reported figure.

Current compression decomposition. Seven imports, canonical maps, cell/unit/stratum construction, sufficient statistics, and semantic minima are not timed separately.

Current CZ18 numerical profile. The current source report does not provide exclusive Schur, CMG apply, leverage postprocessing, target postprocessing, unit adjustment, full residual, transition, and restore times.

Physical inactive-column waste. Per-RHS iterations are returned, but the packet does not summarize active width by iteration or physical columns traversed after some RHSs converge.

Allocation behavior. There is no direct Mata allocation/copy trace. Some assignments may benefit from internal copy-on-write behavior; others may force complete matrices. Explicit receipts and microbenchmarks are needed.

Current graph fixed-point cost. Historical CZ18 evidence reports one fixed-point pass, but the current matched P20 report does not expose graph pass count or graph substage time.

CMG per-level application costs. The packet reports aggregate preconditioner time and level counts, but not projection, smoothing, graph action, coarse restriction, terminal solve, and prolongation separately.

API-5/API-6 command comparison. The missing direct complete-command A/B on degree-two/three cells remains a formal performance caveat.

MATLAB internals. Licensed source is absent. The report identifies compiled kernel categories and persistent workspaces, but not their exact allocation strategy, batching, active-RHS handling, or stage profile.

Unequal comparator work. Target, RNG, correction, tolerance, and frequency differences prevent attributing the full Stata/MATLAB ratio to implementation quality alone.

Stata/MP scaling. The available 4→8 comparison is small and uses diagonal routing. It does not establish how individual Mata sorting, panelsum, matrix multiplication, CMG traversal, disk preserve, or memory allocation scale by processor count.

B64 behavior. Current compressed auto selection is capped at 32, and the packet does not qualify B64 across the current CMG matrix.

Lifecycle I/O variability. The contribution of filesystem cache, preserve-file placement, and restore bandwidth to high-R/C commands is unmeasured.

Graph-derived deflation efficacy. It is plausible only if iteration/action counts remain limiting after per-action optimization; the packet supplies no deflation experiment.

Ultimate pure-Mata ceiling. The packet shows substantial gains from bulk arrays and batching, but contains no instruction-level profile from which to estimate the remaining irreducible gap to compiled C/MEX kernels.

The strongest source-bound decision is therefore: optimize complete preparation and repeated-RHS data movement next; keep hierarchy construction as a protected regression surface; defer algorithmic recycling and any public lifecycle until exclusive profiling proves they are necessary.
