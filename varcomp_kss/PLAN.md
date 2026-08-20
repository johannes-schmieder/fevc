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

PREP-BND-1 is implemented through cumulative runtime candidate
`f06e29e3a5bbb27cfddf3ac48e9596d60b95dbfa`. PREP-MAP-1 returns retained
dense worker/firm maps from the graph selector; PREP-SEM-1 computes exact
per-copy target (`target/frequency`) semantic rank and order in Mata for the
narrow compressed,
no-control, match-deletion JLA path. The eligible path removes two retained-ID
grouping calls, one semantic grouping call, and one Stata sort. Exact,
which does not require semantic ordering, is unchanged. Controlled,
observation-deletion, forced-generic, and auto-generic-fallback JLA paths
retain the Stata semantic oracle.

The candidate is retained as a safe cumulative simplification, not a
large-data preparation speedup. Local source-order reversal improves complete
command time by `1.23--1.39%` and semantic ordering by `30.77%`. The fixed
8,201,888-row CZ18 P20 holdout is `0.22--0.86%` faster overall, but observed
preparation is `1.85--1.90%` slower and semantic ordering is `18.57--26.69%`
slower. The later numerical-work timing reduction is outside PREP-BND and is
not attributed to it. At synthetic F8192/P256, accepted AB and BA jobs are
`6.76%` and `1.00%` slower overall; preparation is `2.60%` faster and `5.08%`
slower. The AB total includes a warm numerical-work/Schur spike outside
PREP-BND-1, so it is not attributed to the boundary change. At the 18:46
collection cutoff original AB job 7236971 was incomplete; replacement job
7237620 was submitted before it completed and is the sole accepted AB source
in the frozen ledger. Job 7236971 later completed but remains superseded.
These timing summaries are descriptive, while the accepted structural, sample,
RNG, and scientific gates are qualification evidence. See
`docs/PREP_BND_1_RESULTS_2026-08-19.md` and `qualification/prep_bnd1/`.

The separately qualified MATLAB tracks sharpen the remaining gap. The
independent dense oracle agrees with Stata at about `1e-15`. In 30 same-host,
source-order-reversed maintained-MATLAB pairs, the Stata/MATLAB command ratio
is 0.62x at F64/P20, 1.08x at F256/P20, 1.67x at F1024/P20, 3.69x at
F256/P200, and 5.27x at F1024/P200. Three fresh fixed-CZ18 MATLAB calls have a
33.23-second median versus 333.02 seconds for the current Stata candidate.
Maintained-MATLAB corrected results are descriptive only because its legacy
correction, RNG schedule, and solver tolerance differ.

The implementation order is:

1. **complete:** add exclusive timing, operation, allocation, and active-width
   counters and close the current-hierarchy qualification gaps;
2. **complete:** retain exact unit/stratum scatter plans;
3. **complete:** return graph maps and move the narrow eligible semantic
   grouping/order boundary into Mata;
4. **partly complete:** retain the qualified FE Schur destination buffers;
   separately measure any broader repeated-RHS workspace;
5. pack active columns only if instrumentation shows at least 10% wasted
   physical work;
6. fuse Schur, certification, leverage, and target dataflow one boundary at a
   time; and
7. redesign CMG storage or test structural Krylov improvements only if the
   preceding profiles show those stages remain dominant.

Decision: `GRAPH-FP-1` is the next independently measured milestone. The fixed
CZ18 profile spends about 166 seconds in graph pruning, roughly half the
complete command and far more than retained mapping or semantic ordering. The
mixed F8192 preparation results do not overturn that benchmark-specific
priority. Begin by instrumenting component, mover, articulation, bridge,
deletion-sort, and active-mask work separately. Prototype a command-local
persistent edge/deletion/adjacency workspace only after that profile is
complete, and retain it only against the current selector as an exact oracle
and fallback. No graph state may survive the command. Use local exhaustive and
randomized multigraph gates before F256/F1024, then extrapolate wall time and
RSS before F4096/F8192 and CZ18.

After graph preparation, address the high-probe numerical slope in a separate
candidate. The P200 MATLAB comparison indicates that RNG, Schur, target, and
correction work—not graph preparation—drives that boundary. Keeping graph and
probe-work changes separate preserves causal performance claims and makes
regressions bisectable. The aspirational 35% preparation and 1.25x CZ18
command targets remain guides rather than retention thresholds. A public
prepare/run/drop lifecycle remains a separate owner decision; no invisible
cache is permitted.
