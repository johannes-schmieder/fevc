# Active package plan

## Objective

Maintain `varcomp_kss` 0.3.0-dev as the sole Stata command and package
identity while preserving the estimator and numerical behavior qualified
under its predecessor name.

## Current boundaries

- Public command and installed files use `varcomp_kss`; no predecessor alias
  is shipped.
- Private Ado/Mata symbols use `vckss`, `_vckss_`, and `VCKSS` prefixes.
- CMG is an internal package component under `cmg/`. API 7 and generator API 4
  are ownership/interface changes over the API-6-qualified numerical core.
- Stable scientific vocabulary includes KSS method labels, `e(kss)`, result
  statuses, estimator options, tolerances, RNG contracts, and return shapes.
- Historical reports, receipts, reviews, plans, and source hashes remain
  byte-identical evidence and are not current instructions.

## Completion gates

1. Generated CMG sources pass deterministic drift checks and expose only the
   package and test targets.
2. All package, CMG, and retained MATLAB-harness Python tests pass.
3. Stata quick/full, clean-install, namespace, forced/automatic CMG, and
   benchmark smoke gates pass when Stata/MP is available.
4. Deterministic source bundles close over tracked package-owned source.
5. The active-name audit rejects predecessor package, shared-library, and
   removed non-KSS CMG surfaces outside hash-bound historical records.
6. Rename equivalence compares isolated predecessor and successor processes;
   scientific, structural, return-shape, caller-state, and RNG records match
   exactly after registered identity transitions. Timing and measured
   code-footprint fields remain in raw evidence and may differ only under the
   explicit finite/nonnegative normalization registry.

Public release remains disabled until human license/provenance review is
complete.

## Next performance milestone

The source-bound dual GPT Pro review at
`../qualification/gpt-pro/adjudications/VARCOMP-KSS-MATLAB-PARITY.md` adopts
`PREP-RHS-1` as the next optimization milestone. It is advisory until each
candidate passes its own benchmark and contract gates.

The first retained candidate is
`95d3950c4cfea669c2a244ad98f6fed03ae8ca19`. It adds diagnostic profiling,
retained deletion-unit/target-stratum scatter plans, bulk graph/compression
imports, deterministic active-RHS packing, vectorized unit adjustment, and
audited dead-gather removal. Baseline-first and candidate-first archive runs
show `2.697%` and `3.110%` complete-command improvements with exact local
science. A 135-job SCC matrix passes through F8192; its cross-host timings are
descriptive, while its structural, scientific, residual, and resource gates
are qualification evidence. See
`docs/PREP_RHS_1_RESULTS_2026-08-19.md` and
`qualification/prep_rhs1/`.

This safe cumulative gain is retained even though it does not meet the
aspirational 1.25x command target. A general FE destination-buffer workspace,
a new flat CMG arena, and any public prepared lifecycle remain separate
follow-on decisions; the previously regressive CMG workspace stays disabled.

The implementation order is:

1. add exclusive timing, operation, allocation, and active-width counters and
   close the remaining current-hierarchy qualification gaps;
2. retain exact unit/stratum scatter and component-projection plans;
3. consolidate command-local sample, graph, ordering, and compression work;
4. add bounded repeated-RHS workspaces and destination buffers;
5. pack active columns only if instrumentation shows at least 10% wasted
   physical work;
6. fuse Schur, certification, leverage, and target dataflow one boundary at a
   time; and
7. redesign CMG storage or test structural Krylov improvements only if the
   preceding profiles show those stages remain dominant.

The first major gate is at least a 35% reduction in mark-through-compression
time and a 1.25x matched CZ18 P20 complete-command improvement, with unchanged
sample, graph, rank, random-atom, residual, resource, failure, and caller-state
contracts. A public prepare/run/drop lifecycle remains a separate owner
decision; no invisible cache is permitted.
