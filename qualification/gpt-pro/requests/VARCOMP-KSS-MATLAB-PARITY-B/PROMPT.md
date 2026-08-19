# Independent review B: numerical architecture path to MATLAB parity

You are the second of two independent reviewers. You have not seen and must
not infer the first review. Work only from the attached hash-bound packet.
Cite packet paths plus function/program names, and add line numbers when the
viewer exposes stable lines. Clearly label measured evidence, deductions from
source, and speculation.

## Objective

Perform a clean-room numerical-algorithms review of `varcomp_kss`, a pure
Stata/Mata 18/19 implementation of the Kline--Saggio--Sølvsten variance
component estimator with an internal combinatorial multigrid preconditioner.
Find architectural changes that reduce total work or improve asymptotic and
constant-factor behavior enough to approach the maintained MATLAB comparator,
without weakening any registered scientific or fail-closed contract.

The source is commit
`a405e7652da9a6657045090963b8b1ec5835a488`. No licensed MATLAB source and no
restricted data are included. The comparator has different target features,
randomization, and tolerance. Its timings diagnose throughput and scaling, not
equality of corrected estimates.

## Empirical facts to reconcile

Use the packet reports as authoritative.

- The exactly input-matched CZ18 P20 command comparison is 305 seconds in
  Stata versus 43.802 seconds in MATLAB, a 6.96 Stata/MATLAB ratio.
- On 1,024-firm CMG tasks, cell ratios range from 0.65 to 4.38 and the median is
  2.59; setup alone does not explain most ordinary cells.
- Density-four scale tasks exhibit a discontinuous hierarchy pathology: one
  completed KSS case took 9,610 seconds versus MATLAB's 84.951 seconds, while
  other density-three cases are far less extreme.
- The current design may solve 601 complete RHSs for P200 and certifies every
  RHS with a complete original-system residual. Previous pass/layout work cut
  matched Stata time substantially without changing action counts.

## Invariants and admissible design space

Preserve the estimator, retained sample, articulation/deletion fixed point,
frequency semantics, controls, target algebra, random-atom and stream
contract, probe-count and batch invariance, requested tolerance, complete
per-RHS residual certificate, typed failure/status vocabulary, resource
admission, caller-state restoration, and point-estimate-only output.

The shipped runtime must remain pure Stata/Mata 18/19, offline, deterministic,
and bounded. Do not propose dense observation-space or firm-by-firm matrices,
external Python/MATLAB/MEX execution, or relaxed residuals. You may consider
new sparse representations, operator fusion, block/recycled Krylov methods,
preconditioner reuse, hierarchy changes, or an optional lifecycle, but spell
out proof obligations, finite-precision hazards, memory bounds, and fallback
behavior. A compiled plugin may be discussed only as an explicitly separate,
non-shipped research branch—not as the answer to the requested pure-Mata path.

## Review task

Reconstruct the operation count and data flow for exact and JLA routes,
including compression, Schur elimination, CMG construction/application,
batched probes, leverage and target contractions, correction, and complete
residual certification. Diagnose which gaps are algorithmic and which are
Mata throughput limits.

Evaluate at least these alternatives:

1. hierarchy-construction changes that eliminate the degree/density-four
   discontinuity while retaining deterministic CMG certificates;
2. block, seeded, deflated, or recycled PCG across related RHSs;
3. fusion of Schur actions, preconditioner actions, sketches, contractions,
   and residual checks across batches;
4. reordering/layout choices for compressed cells, vertices, and incidence
   structures;
5. changing batch selection or solve scheduling under the current memory API;
6. route-selection changes driven by calibrated work rather than only current
   structural pilots, without accepting an uncertified result;
7. a safe prepared-data or reusable-workspace lifecycle as an optional public
   boundary.

For each, determine whether it can preserve deterministic outputs exactly,
preserve only the registered numerical tolerances, or would change the public
contract and therefore must be rejected or isolated.

## Required response

Return:

1. **Independent verdict and complexity account.** Identify the governing
   terms for ordinary, weak, high-density, and large-RHS regimes.
2. **Bottleneck hypotheses.** Rank them, cite source/report evidence, and give
   a discriminating experiment for each.
3. **Architectural portfolio.** At least ten concrete candidates with code
   locations, work/memory model, proof obligations, expected end-to-end
   speedup range and confidence, engineering effort, regression risk, and a
   quantitative promotion/kill rule.
4. **Detailed designs for the top three.** Supply enough pseudocode and state
   ownership/workspace detail to expose hidden passes, allocations, and
   residual-certification cost.
5. **Parity definition.** Specify fair cold, warm, command, stage, core,
   convergence, memory, and core-count comparisons given the comparator's
   scientific differences.
6. **Benchmark and red-team plan.** Include graph families, row/cell ratios,
   degrees, connectivity, probe counts, batch widths, processor counts,
   numerical adversaries, and reproducibility checks.
7. **Rejected alternatives and failure modes.** Include seductive algorithmic
   shortcuts that would bias sketches, break reproducibility, mask bad RHSs,
   or exceed bounded memory.
8. **Phased roadmap and uncertainty.** Order the next milestones and identify
   information that the packet cannot establish.

Prefer falsifiable designs over broad advice. Do not claim a speedup without
showing which measured stage or operation count it can plausibly reduce.
