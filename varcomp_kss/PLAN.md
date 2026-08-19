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

## Current performance sequence

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

The next retained candidate is FE-BUF-1 at
`1cb441f20be0d747483cd8f81746af4187192d0f`. It reuses bounded solve-local
cell and worker destination buffers without changing logical RHS order or
complete residual certification. Local source-order reversal shows
`1.42--2.30%` complete-command, `7.65--8.61%` Schur, and `2.94--4.31%` PCG
improvements with exact science. Matched synthetic SCC pairs through F8192
show a median `3.29--6.85%` command reduction; the separate cold F15625
endpoint improves `5.70%`. The fixed 8,201,888-row CZ18 P20 holdout is
command-neutral (`0.0%` overall, `-0.35%` warm). The candidate remains useful
because it has no regression, improves exposed large-RHS paths, and reduces
allocator pressure with a bounded resource-model charge. See
`docs/FE_BUF_1_RESULTS_2026-08-19.md` and
`qualification/fe_buf1/`.

CZ18 now makes the next priority clearer: command-boundary selection and
preparation, not another Schur allocation change. The proposed independently
measured `PREP-BND-1` step consolidates requested-sample scans, grouping,
graph pruning/redensification, semantic ordering, and compressed-state
handoff in a command-local context. It must preserve Stata's string-ID,
factor-variable, and sort semantics, and it must not introduce an invisible
cross-command cache.

The implementation order is:

1. **complete:** add exclusive timing, operation, allocation, and active-width
   counters and close the current-hierarchy qualification gaps;
2. **complete:** retain exact unit/stratum scatter plans;
3. **current:** consolidate command-local sample, graph, ordering, and
   compression work;
4. **partly complete:** retain the qualified FE Schur destination buffers;
   separately measure any broader repeated-RHS workspace;
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
