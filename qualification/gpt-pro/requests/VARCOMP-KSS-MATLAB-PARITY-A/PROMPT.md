# Independent review A: Mata systems path to MATLAB parity

You are the first of two independent reviewers. You have not seen and must not
infer the other review. Evaluate only the hash-bound packet attached to this
chat. Cite packet paths plus function/program names, and add line numbers when
the viewer exposes stable lines. Treat general experience as hypothesis, not
packet evidence.

## Objective

Produce a decision-ready systems and performance review of `varcomp_kss`, a
pure Stata/Mata 18/19 implementation of the Kline--Saggio--Sølvsten variance
component estimator with an internal combinatorial multigrid preconditioner.
Identify the most promising code changes for approaching the maintained MATLAB
comparator's performance while preserving the registered estimator and failure
contracts.

The source is commit
`a405e7652da9a6657045090963b8b1ec5835a488`. The packet contains no restricted
rows and no licensed MATLAB source. The comparator evidence is descriptive:
its target features, random stream, and tolerance are not identical. Do not
claim numerical-output parity from timing comparisons.

## Measured problem to explain

Use the packet reports rather than these highlights whenever there is a
conflict.

- On the exactly input-matched CZ18 P20 holdout, the current Stata command took
  305 seconds and MATLAB 43.802 seconds: 6.96x at the command boundary.
- On the synthetic CMG matrix, median Stata/MATLAB command ratios rise from
  below one on tiny graphs to 2.59 at 1,024 firms, with 1,024-firm cells from
  0.65 to 4.38.
- In the Optimization III scale ladder, most completed shapes favor MATLAB;
  density-four cases expose an especially severe hierarchy-construction
  pathology, including 9,610 versus 84.951 seconds on one completed shape.
- A previous optimization round already reduced matched CZ18 P200 Stata time
  from 1,279 to 904 seconds without changing actions or accepted results.

## Non-negotiable boundaries

Do not recommend a change that silently alters:

- the estimation sample or graph-pruning fixed point;
- match versus observation deletion semantics or frequency-weight expansion;
- controls and fixed-offset target algebra;
- random-atom identities, streams, probe ordering, batch invariance, or caller
  RNG restoration;
- complete original-system residual checks, per-RHS acceptance, typed failures,
  resource admission, or state restoration;
- point-estimate-only output or the absence of `e(V)`.

Runtime must remain pure Stata/Mata 18/19 with no Python, MATLAB, compiled MEX,
or other external runtime dependency. Memory must remain explicitly bounded;
no observation-space or firm-by-firm dense construction is admissible. A
proposal may introduce an optional new public lifecycle only if it is clearly
separated from safe internal changes and justified as a distinct owner choice.

## Review task

Reconstruct the hot execution path from ado entry through sample preparation,
compression, graph pruning, solver/CMG construction, batched repeated-RHS
solves, leverage/target contractions, correction, and restoration. Then inspect
the implementation for avoidable passes, copies, allocations, sorts, string or
associative work, scalar loops, Mata interpreter boundaries, repeated panel
setup, workspace churn, insufficient batching, cache-hostile layouts, and
missed reuse across RHSs.

Pay special attention to:

1. the density-four CMG hierarchy setup pathology;
2. repeated Schur/preconditioner actions and PCG workspace reuse;
3. leverage and target stages on hundreds of RHSs;
4. stored-row import/compression/restoration costs at rows-per-cell above one;
5. whether processor-count and batch behavior indicate exploitable Mata/Stata
   parallelism or a hard runtime ceiling;
6. which MATLAB advantages can plausibly be emulated in pure Mata and which
   depend on compiled kernels or process-level parallelism.

## Required response

Return the following sections:

1. **Source-bound verdict.** State whether the current evidence is sufficient
   to choose the next optimization milestone and list any material caveats.
2. **Critical-path reconstruction.** A stage table with implementation
   locations, complexity/data movement, measured evidence, and likely gap.
3. **Ranked optimization portfolio.** At least ten candidates. For each give
   exact code locations; mechanism; whether it is pass reduction, allocation,
   vectorization, layout, reuse, batching, or algorithmic; expected end-to-end
   speedup range and confidence; implementation cost; numerical/semantic risk;
   memory impact; the decisive benchmark; and a quantitative kill criterion.
4. **Top three implementation sketches.** Give concrete pseudocode or
   function-level change plans and the invariants/tests that must surround
   them. Be specific enough for a maintainer to implement without guessing.
5. **Measurement plan.** Define cold/warm boundaries, stage timers, operation
   counters, allocation/workspace receipts, task matrix, repetitions, and how
   to compare fairly with MATLAB.
6. **Rejected ideas.** Name tempting proposals that violate contracts, cannot
   work in Mata, merely move costs, or are unsupported by evidence.
7. **Phased roadmap.** Order reversible checkpoints and say what evidence is
   required before proceeding to more invasive changes.
8. **Uncertainty.** List conclusions that depend on unprovided profiling or
   MATLAB internals.

Do not settle for generic advice such as "vectorize more". Tie every major
recommendation to the supplied source and a falsifiable performance test.
