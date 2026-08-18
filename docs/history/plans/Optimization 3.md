# Optimization III — scalable matrix-free `kss_bc`

Use this document as the complete prompt for the next Codex optimization
thread. The thread is an implementation task, not a design review. Work
autonomously through the locally validated changes and then the SCC evidence
described below. Do not stop after improving the CZ18 case.

## Objective

Optimize `kss_bc` for genuinely large worker--firm datasets. CZ18 is a real,
useful qualification and profiling case, but it is not the architectural
target. The long-run target is approximately:

- 40,000,000 workers;
- 1,000,000 firms;
- 80--160 million worker--firm coefficient cells, corresponding to two,
  three, or four cells per worker; and
- an input-row count that must be modeled separately from the compressed cell
  count.

The reference feasibility envelope is one Stata process, no distributed
estimator, at most 128 GiB of total peak memory, and completion within 48
hours. Completion within 24 hours is the stretch target. The round does not
need to execute the full target. It must improve the scalable implementation
and produce measured, validated extrapolations that say whether each target
scenario is feasible. If a scenario is not feasible, identify the dominant
bottleneck and quantify the remaining gap.

The main implementation must remain matrix-free. It must not form or factor
an `F x F` dense matrix, allocate observation-space matrices, or introduce any
other `O(F^2)` main-path storage or work. A dense direct route may remain an
optional small-firm comparison after the scalable route works, but it is not
an Optimization III priority and must not enter the large-target forecast.

Multiple independent SCC experiments may run concurrently. This permission
does not authorize splitting one estimate across jobs or nodes: every
`kss_bc` estimate remains one Stata process.

## Start point, authority, and boundaries

Begin only after the current Optimization II / `KSS-STREAMLINE-1` thread has
finished and left a coherent handover. At startup:

1. Follow the root and `kss_bc/` `AGENTS.md` startup instructions.
2. Read the finalized KSS plan, handover, numerical architecture, testing
   guide, SCC harness, and the actual hot-path source.
3. Record the exact final Optimization II commit, worktree status, installed
   Stata versions, source-bundle hash, and current benchmark evidence.
4. Audit every item below against the finalized source. Do not reimplement an
   optimization already incorporated by Optimization II, and do not revert a
   current improvement merely to recreate an old comparison.
5. Create the new owner-authorized checkpoint `KSS-NUMOPT-2`, also called
   Optimization III, in the living KSS plan and handover.

Use `main` and the current worktree. Do not create or switch branches or
worktrees. Preserve pre-existing user changes. Optimization III may change
`kss_bc/**`, KSS integration notes in `cmg_plan.md`, and narrowly necessary
shared-CMG generator source and tests. This authorization covers a
target-specific KSS generation setting and, if the measured trigger below is
met, an accepted scalable KSS reduction kernel. Preserve behavior for every
other shared-CMG consumer and regenerate checked-in targets through the
repository generator; never hand-edit generated CMG output.

Do not change `ppml_talo/**`, `application/**`, `software/**`, `paper/**`,
`theory/**`, `proof-audit/**`, `state/**`, `archive/**`, or any frozen release.
M0--M13 remain complete and untouched; this is not a proof milestone.

Keep the statistical and public interfaces unchanged:

- point estimates only; do not add `e(V)`, standard errors, or AM intervals;
- do not change the estimator, finite-projection coefficient, sample,
  connectedness or deletion rules, target definitions, probe count,
  tolerances, convergence rules, or default routing semantics;
- preserve complete original worker-plus-firm normal-equation residuals for
  every accepted RHS;
- preserve caller data, `e(sample)`, RNG algorithm, streams, complete RNG
  state, sort-jumbler state, and typed failures; and
- do not introduce hidden regularization or silently relax numerical gates.

Performance thresholds in this document govern whether an optimization is
worth retaining. They are advisory performance evidence, not scientific
command gates: a statistically valid result must not be withheld merely
because a timing forecast is disappointing.

## Current architectural facts to verify, not assume

At the time this prompt was written, the compressed path was already
cell-based and matrix-free. Its FE Schur action used coefficient cells rather
than physical observations, and the lifecycle released raw row mappings
before the numerical peak. Build on that route instead of recreating the old
row-level generic operator.

The remaining scale concerns visible in the then-current source were:

- production KSS and the generated KSS CMG target used `mata set matalnum on`;
- `kssbc_scale__canonical_ids`, `kssbc_scale__row_group_map`,
  `kssbc_scale__key_panel`, `kssbc_scale__stable_groupsum`, and
  `kssbc_scale__stable_colsum` still contained scalar or per-panel hot work;
- the compressed runtime persisted `unit_semantic_key` and
  `stratum_semantic_key` string vectors, while `kssbc_srt__keys` constructed
  fixed-width strings in a loop and the RNG API sorted string keys;
- every full operator application still needed worker- and firm-group
  aggregation over the cell graph; and
- the target workload reused a fixed operator across hundreds of RHSs, so
  memory traffic, segmented reductions, hierarchy application, and iteration
  counts could dominate at national scale.

Recheck these statements against the final Optimization II source and record
the result as an implemented/remaining/not-applicable coverage table.

## Existing evidence from the preceding thread

Treat all numbers in this section as unofficial leads. They were local
experiments, not source-bound qualification, and no implementation may depend
on temporary files from that work.

The local fixture used Stata 18 MP with four processors on an Apple M2 Ultra:
60,000 rows, 10,000 workers, 1,000 firms, and 200 probes. Representative warm
complete-command timings were:

| Candidate | Seconds | Change from original |
|---|---:|---:|
| Current code with `matalnum on` | 24.30 | baseline |
| `matalnum off` | 19.81 | 18.5% faster |
| `matalnum off` plus identity-scatter fast path | 18.09 | 25.6% faster |
| Identity scatter plus quad column reductions | 14.92 | 38.6% faster |
| Identity scatter plus quad general scatter | 16.83 | 30.7% faster |
| All tested quad reductions and scatter changes | 13.53 | 44.3% faster |

All 12 exported result cells matched. Only last-digit solver-residual changes
were observed, and the temporary all-quad variant passed all 33 Stata tests
available in that experiment.

Relevant kernel measurements were:

- current column sum: 1.534 seconds; quad version: 0.071 seconds;
- current stable grouped sum: 0.957 seconds; quad version: 0.022 seconds;
- identity scatter: 2.902 seconds; direct identity path: 0.002 seconds; and
- general scatter: 1.408 seconds; quad version: 0.127 seconds.

The current chained compensated column sum returned zero for
`(1e16, 1, -1e16)'`, while the quad version returned the correct value one
under both `matalnum` settings. A naive native `panelsum()` variant was a
little faster but failed adversarial cancellation and is not an acceptable
replacement.

At five million rows, vectorized physical-panel construction improved from
0.725 to 0.037 seconds, pair coding from 1.301 to 0.142 seconds, and the three
current Mata ID maps from 8.16 to 6.92 seconds. Vectorized worker-pair counting
regressed from 0.241 to 0.446 seconds and should not be adopted. A pointer
struct experiment saved about 200 MB at five million rows but applied mainly
to the generic-controls path, so it is not part of the first bundle.

PCG bookkeeping alone produced a roughly 4.2x kernel improvement but was
projected to save only five or six seconds on CZ18. Wider RNG batching did not
help and increased scratch; the tested workspace-reuse design was about twice
as slow. Do not repeat those exact candidates without new evidence that their
context has materially changed.

Dense Cholesky appeared plausible for approximately 10,000 firms, but its
factor-and-solve experiment extrapolated to roughly 230 seconds with a floor
of at least 1.68 GiB. That result does not scale to one million firms and is
not evidence for the target architecture.

Historical source-bound CZ18 evidence is context rather than the new
baseline. CZ18 has 8,201,888 retained rows, 117,529 workers, 10,603 firms, and
311,730 coefficient cells. Different source generations produced fixed P200
runs near 1,217 and 1,392 seconds; one user-cancelled job is not performance
evidence. Bind every new comparison to the finalized Optimization II source.

## Phase 1 — integrate the clear improvements in one shot

Implement the following as one coherent patch after the final-source audit.
Do not run a full benchmark or the full regression suite after each edit.
Compilation and narrow smoke checks during editing are appropriate; perform
the matched benchmark and full local validation after the bundle is complete.

1. **KSS-specific numeric mode**
   - Compile production KSS code with `matalnum off`.
   - Make shared-CMG generation target-specific so the KSS target is compiled
     off while `ppml_talo` and existing test targets retain their registered
     behavior unless separately measured and authorized.
   - Keep `matastrict on` and all explicit quad-precision operations.

2. **Quad reductions and scatter**
   - Replace the tested hot column sums, dot products, stable grouped sums,
     and general scatter reductions with cancellation-safe quad operations.
   - Add or retain the direct identity-scatter path when the destination map
     is exactly the identity.
   - Do not substitute naive `panelsum()` for a cancellation-sensitive sum.

3. **Proven compressed preprocessing**
   - Vectorize canonical-ID construction, row-to-group maps, key-panel
     boundaries, physical panels, and pair coding where those routines are
     actually reached by the compressed path.
   - Preserve exact stable/canonical order and repeated-pruning semantics.
   - Retain the current worker-pair counting implementation.

4. **Memory accounting**
   - Update the direct peak model for every changed persistent vector and
     scratch matrix.
   - Free obsolete row-level and mapping storage at the earliest semantically
     safe point.
   - Do not claim a memory gain from pointer or alias assumptions that Mata's
     allocator does not guarantee.

After integration, compare three warm P200 runs of the immutable final
Optimization II source with three warm runs of the candidate, excluding
first-use compilation. The intended result is at least a 20% median
complete-command improvement. If it is materially smaller than the
unofficial evidence suggests, profile the integrated bundle and use a bounded
ablation only to locate a regression; do not institute a test-every-edit
ritual retroactively.

## Phase 2 — make the representation scale

Proceed through the following packages in order. For each speculative
package, use a focused correctness test and a kernel or small end-to-end
benchmark first. Run the full suite only after deciding to retain the
integrated package.

### 2A. Remove persistent string RNG keys

Replace per-unit and per-stratum string identity with a collision-free numeric
canonical representation. Prefer canonical dense ranks plus numeric
permutations or bounded multi-column tuples. Do not use an unverified hash.
If a mixed-radix scalar encoding is used, prove from registered bounds that
every encoded integer is exact below `2^53`; otherwise retain separate numeric
columns.

The change must preserve the exact versioned RNG contract: runtime, seed,
domain, probe index, canonical atom identity and order, stream separation,
state advancement, golden vectors, row-order invariance, batching invariance,
route invariance, and caller-state restoration. Avoid retaining one string per
atom merely to satisfy an old internal API.

### 2B. Build a compact dual-ordered cell graph

Represent the coefficient cells for efficient worker-major and firm-major
passes without duplicating all cell payloads. Store canonical cell arrays once
and add only the order/index and panel-boundary structures needed for each
direction. Measure whether indirection or a second ordered payload is faster
and retain the design with the better end-to-end target forecast, not the
smallest microbenchmark alone.

Maintain an explicit byte inventory for:

- each raw observation;
- each coefficient cell and deletion unit;
- each worker, firm, target stratum, and hierarchy vertex/edge;
- each ordering, panel, and permutation vector;
- each active RHS column; and
- transition, hierarchy-build, solver, certificate, restoration, and output
  scratch.

At the target dimensions, worker, firm, and cell identifiers fit in unsigned
32-bit storage. If double-valued Mata index vectors become a dominant memory
coefficient, assess an internal compiled representation using 32-bit indices
and double-precision numeric payloads. Retain it only with platform-specific
build tests, bounds checks, deterministic behavior, and a Mata fallback. Do
not check in machine-specific binaries.

### 2C. Eliminate per-group interpreter overhead

The worker aggregation can contain tens of millions of groups, so a Mata loop
that calls a stable reducer once per worker is not a viable endpoint even if
it is fast on CZ18. Implement deterministic segmented reductions whose work
is linear in cells times active RHS width and whose scratch is bounded by an
explicit tile size.

First test the best Mata-native blocked/quad design. If it passes the
cancellation and reproducibility tests but the fitted per-group or
per-cell-RHS coefficient still makes the central target miss 48 hours, or if
per-panel call overhead remains the measured dominant term, implement a small
internal compiled segmented-reduction/operator kernel. It must use stable
pairwise, compensated, or extended-precision accumulation; expose no public
API; fail closed on invalid bounds; build on local macOS and SCC Linux; and
retain a correct deterministic Mata fallback.

Tile cells and RHS columns so no operator call materializes row-sized
matrices. Select batch widths from the direct memory envelope and measured
throughput. Record bytes and seconds per cell-RHS action, including gather,
aggregation, quotient projection, preconditioning, and complete residual
certification.

### 2D. Reduce raw-data passes and release rows early

Move canonical densification, cell compression, deletion-unit construction,
and exact target-stratum construction toward a single low-pass pipeline.
Avoid repeated Stata `egen group()` calls, full sorts, and simultaneously
resident row-length temporary variables when the same canonical map can be
constructed once in Mata. Preserve all pruning, deletion, frequency,
target-weight, and caller-restoration semantics.

Separate two feasibility questions in every report:

1. whether the compressed numerical estimator fits and finishes; and
2. whether the Stata-resident raw dataset plus compression transition fits.

An in-core solver forecast must not be presented as an end-to-end dataset
forecast when raw input dominates. A new destructive, file-based, or
out-of-core public command mode is outside this round and requires a separate
owner decision.

### 2E. Optimize CMG and repeated-RHS solves only after the operator

Keep hierarchy construction and application `O(C + W + F)` in memory and
avoid rebuilding equivalent orderings or maps. Profile hierarchy setup,
coarse transfers, preconditioner applications, terminal solves, and complete
residual checks separately.

After the representation and operator are stable, test deterministic
deflation or recycled/block PCG across the fixed production RHS sequence.
Retain such a solver change only when it reduces complete-command time and all
RHSs still pass the original-system residual gate. An iteration reduction or
a faster bookkeeping microkernel alone is insufficient.

## Local validation policy

Use the smallest relevant tests while editing, then apply these integrated
gates:

1. Run the existing adversarial reduction fixtures, including a grouped and
   ungrouped cancellation case equivalent to `(1e16, 1, -1e16)' == 1`.
2. Verify registered exported estimates, accounting identities, exact target
   grouping, RNG golden vectors and complete state, row and batch invariance,
   large canonical identifiers, weak connectivity, typed failures, and every
   complete solver residual.
3. Run the local P200 benchmark with four Stata processors and record cold
   wall, three warm command repetitions, component timers, peak RSS, batch
   widths, iterations, actions, and numerical differences.
4. Run the complete Python/static KSS suite, Stata quick/full and clean-install
   suites when Stata is available, the package checks, and the repository
   checks required by the active `AGENTS.md` before closing a retained bundle.

A deeper candidate qualifies for retention if it produces either:

- at least a 10% repeatable complete-command gain on CZ18 or a representative
  scale rung; or
- a scale-enabling improvement, such as at least a 15% reduction in a
  dominant byte/pass coefficient or changing a target scenario from over to
  under the 128-GiB or 48-hour envelope.

Report a miss and remove the speculative candidate at its own boundary. Do
not discard the valid baseline or unrelated worktree changes.

## Scaling experiment design

Parameterize the target rather than extrapolating only from a CZ18 row
multiple. Use:

- `W = 40,000,000` workers;
- `F = 1,000,000` firms;
- `C/W = 2, 3, 4`, hence `C = 80, 120, 160 million` cells; and
- `R/C = 1, 8, 26.3` raw rows per cell, with 26.3 as the approximate CZ18
  anchor.

Use the observed finalized-CZ18 ratios for deletion units and target strata
when no separate target information exists, and show their coefficients
explicitly so those assumptions can be replaced. Include both
`replicated_blocks` and adverse `ring`/weak-connectivity behavior. Report
connector volume and do not mistake copy-meta-graph spectral diagnostics for
the spectrum of the full graph.

Create separate deterministic synthetic ladders for raw compression and the
already-compressed numerical engine. The compressed ladder uses target aspect
ratio `W/F = 40`:

| Target fraction | Workers | Firms | Cells at 2/3/4 per worker |
|---:|---:|---:|---:|
| 1/64 | 625,000 | 15,625 | 1.25m / 1.875m / 2.5m |
| 1/32 | 1,250,000 | 31,250 | 2.5m / 3.75m / 5m |
| 1/16 | 2,500,000 | 62,500 | 5m / 7.5m / 10m |
| 1/8 central only | 5,000,000 | 125,000 | 15m |

Run all three cell-density cases through 1/16 and the central case at 1/8.
Run weak-connectivity central cases at 1/32 and 1/16. For raw preprocessing,
use the central three-cell design with eight rows per cell: 15m, 30m, and 60m
rows at 1/64, 1/32, and 1/16. Run the 120m-row 1/8 preprocessing case only if
the 1/16 measurement gives a direct allocation and wall request that fits the
selected SCC resource limits.

Each synthetic result must record raw rows, cells, workers, firms, deletion
units, strata, graph and hierarchy dimensions, RHS counts, iterations,
actions, batch widths, stage times, CPU, peak memory, seed, source and input
hashes, and success or typed failure.

Fit stage-specific models rather than one total linear rule:

- bytes per row, cell, deletion unit, stratum, worker, firm, hierarchy edge,
  hierarchy vertex, and RHS column;
- preprocessing seconds per raw row and per sorting/compression pass;
- operator seconds per cell-RHS action;
- RNG seconds per atom-probe;
- hierarchy construction and application coefficients; and
- iteration/action distributions by connectivity design.

Fit on the 1/64--1/16 measurements and reserve 1/8 as an out-of-sample check.
If predicted and observed 1/8 time or memory differ by more than 20%, do not
issue a simple linear target projection. Diagnose the nonlinearity, fit a
piecewise or otherwise evidence-supported model, and widen the forecast
range. Present low, central, and high target forecasts with at least 20%
resource headroom rather than a single unsupported point estimate.

## SCC execution

No SCC submission occurs before the local gate. Use the established guarded
KSS deployment, submission, validator, and collection workflow. Inspect live
SCC state and quota when execution starts; do not copy local dirty files that
are outside the frozen source bundle.

For every job:

- use `-P welfgr` and normally leave queue and host unrestricted;
- run one Stata process and set and verify four actual Stata processors;
- choose 14 or 16 reservation slots normally, with `mem_per_core` and `h_rt`
  derived from a measured predecessor plus justified headroom rather than
  requesting 128 GiB automatically;
- keep representative rungs at or below 12 hours when feasible; request a
  longer inseparable run only when measurements justify it;
- use a fresh sortable run ID, immutable source bundle, frozen task/input
  manifest, and isolated run directory;
- stage appropriate high-I/O data under `$TMPDIR` and copy required receipts
  and outputs back before exit; and
- preserve restricted data under authorized `/projectnb/welfgr/` storage and
  never place it in Git or ordinary local artifacts.

After the local bundle passes, submit the independent experiment matrix
concurrently. At minimum this consists of:

1. matched finalized-Optimization-II and Optimization-III CZ18 P200 jobs;
2. the compressed synthetic rungs above;
3. the weak-connectivity diagnostics; and
4. the admitted raw-preprocessing rungs.

Use separate scalar jobs or guarded array tasks as supported by the current
harness. Independent baseline, candidate, density, connectivity, and scale
tasks do not need to wait for one another. Preserve genuine scientific stage
dependencies, and never describe a diagnostic task as qualification for
another task merely because both completed.

Accept a run only when all three evidence layers agree:

1. complete SGE accounting with `failed=0` and `exit_status=0` for every
   expected task;
2. a Stata application success marker with no fatal error and all numerical,
   residual, lifecycle, and resource assertions passing; and
3. expected outputs that pass the source-bound external validator and all
   schema, dimension, completeness, and scientific checks.

Record job IDs, run IDs, requested resources, host and queue, wall and CPU,
`qacct maxvmem`, source and input hashes, application logs, validation
receipts, and collected paths. Absence from `qstat` is not success. Preserve
failed evidence under its original run ID and use a new run ID for a retry.

Do not attempt the full 40-million-worker dataset automatically in this
round. The SCC ladder is evidence for a feasibility decision, not permission
for an unbounded production run.

## Required final report and stopping rule

Do not close Optimization III merely because CZ18 becomes faster. Close only
after the retained scalable changes and evidence support a target-level
assessment. The final report must contain:

- the final Optimization II baseline and Optimization III source hashes;
- an implemented/remaining/rejected recommendation table;
- local and SCC before/after timings with uncertainty and exact test context;
- phase-by-phase time, memory, pass-count, iteration, and action models;
- separate compressed-solver and raw-input feasibility conclusions;
- low (`C=80m`), central (`C=120m`), and high (`C=160m`) forecasts for each
  registered `R/C` case;
- whether each case fits within 128 GiB and its expected wall under the
  24-hour and 48-hour thresholds, including headroom and model error;
- the dominant bottleneck and quantified remaining improvement if a case
  fails;
- every test and SCC evidence layer run, including failures and reused
  evidence; and
- files changed, files deliberately untouched, unresolved risks, and the next
  recommended optimization.

Update the KSS plan, numerical architecture, testing guidance, changelog,
benchmark evidence, and handover to match the retained implementation. Do not
make a production, public-release, licensing, inference, or full-target
feasibility claim beyond what the measured evidence supports.
