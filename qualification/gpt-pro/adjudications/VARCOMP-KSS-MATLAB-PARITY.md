# Dual GPT Pro adjudication: MATLAB-parity optimization

Status: `ai_reviewed_twice`

Review date: 2026-08-18

Source commit: `a405e7652da9a6657045090963b8b1ec5835a488`

Decision owner: package maintainer

Qualification status: advisory; no performance candidate is qualified by this review

## Evidence and independence

Two separate GPT-5.6 Sol Pro chats reviewed independently identified packets over the same 51-file source and evidence set. Neither reviewer received the other packet, prompt, chat, or response before both responses were captured.

| Review | Focus | Packet SHA-256 | Verbatim-body SHA-256 | Chat |
|---|---|---|---|---|
| A | Mata systems and execution path | `fe003482ab187b65ed16c971281c8aa6571e9a31d74d1329173ca90a588c38ae` | `06c1ff146a20ca11f71151ec02c0a7677ae8c698a596521f6f377182c9f12649` | [Review A](https://chatgpt.com/c/6a84f63e-46d4-83ea-963b-471038339541) |
| B | Numerical architecture and algorithms | `0a8dc8e4f81f1ea50de3e53b4c888cc3ee3c9ff83ffe47a03b26e058c4d7a1e2` | `7641683d665edde783d1e666854cd1cdc87ae639e599f3afb4207e51cd4fdffa` | [Review B](https://chatgpt.com/c/6a84f695-a798-83ea-9fe8-3d01d1d23745) |

The local packet verifier passed every manifest entry and `unzip -t` passed for both ZIPs. The manifest checker also emits one harmless formatting warning because each manifest has a trailing blank line; the uploaded bytes are retained unchanged. The response-body hashes cover the exact browser text without the Markdown record's final newline.

Both reviews are source-bound technical advice, not an oracle result or a performance receipt. Their proposed speed ranges are hypotheses to be tested, not promises. The MATLAB comparator also performs different numerical work, so timing convergence is not evidence of estimator equality.

## Joint diagnosis

The reviewers independently reached the same central conclusion: the next milestone should optimize complete command preparation and repeated-RHS data movement, not begin with another hierarchy rewrite or a high-risk Krylov redesign.

The matched CZ18 P20 boundary is dominated by import, requested-sample work, graph/sample selection, and adjacent preparation: 188.272 seconds out of 305 seconds, or 61.7%. Current hierarchy construction is only 6.442 seconds, or 2.1%. Meanwhile, fixed-action batching evidence shows that better layout and matrix-call amortization matter more than the observed 4-to-8-core gain. Repeated ordering, returned matrices, whole-width actions after some columns converge, and repeated allocation are therefore the most credible next targets.

The shared recommendations are:

1. Add exclusive timers and explicit operation, allocation, and active-width counters before changing algorithms.
2. Retain immutable deletion-unit and target-stratum scatter plans instead of rebuilding `order()` and `panelsetup()` state.
3. Consolidate the command-local retained-sample, graph, semantic-order, and compression pipeline to reduce imports, sorts, and full-column passes.
4. Reuse bounded solve workspaces and destination buffers across fit, leverage, and target calls.
5. Pack active columns only if measurement shows at least 10% physical work is currently wasted on converged columns.
6. Preserve the current robust hierarchy as a regression surface. Close its remaining degree-two/three and former degree-four qualification gaps, but do not make hierarchy construction the main optimization project.
7. Do not enable the existing CMG workspace; prior evidence shows large regressions. Any replacement must be a new measured-width, alias-audited flat arena.
8. Keep route and batch selection deterministic, structural, versioned, and decided before estimator RNG. Never use live timings or post-RNG fallback.
9. Treat an explicit prepared-data lifecycle as a separate public-API decision. There must be no invisible cross-command cache.
10. Defer structural seeding, deflation, recycling, and block PCG until profiling shows action count—not per-action throughput or command preparation—remains dominant.

Neither review recommends changing probe count, tolerance, correction semantics, deletion units, target strata, random-atom order, per-RHS stopping, complete residual certification, typed failures, or caller-state restoration. Both reject a compiled plugin as the shipped solution because the package's active boundary is pure Stata/Mata 18/19.

## Differences and resolution

Review B places qualification of the existing robust hierarchy first and would expose a prepared lifecycle relatively early. It also gives more weight to stable Galerkin contraction, dual incidence layouts, coarse seeding, block PCG, and recycling. Review A begins with instrumentation, low-risk command-local reuse, and preparation/dataflow changes; it leaves the public lifecycle and algorithmic solver changes until the internal one-shot path is exhausted.

The reconciled strategy uses Review A's implementation order while adopting Review B's hierarchy work as a prerequisite qualification gate. This resolves the apparent disagreement cleanly:

- Close the known hierarchy evidence gap now, but do not rewrite the hierarchy or count that work as the next speed milestone.
- Optimize one-shot command-local preparation before designing a public persistent handle.
- Build internal plans and workspaces so a later explicit lifecycle can reuse them without dictating today's public API.
- Keep Review B's algorithmic portfolio as a staged research queue with independent promotion and kill gates.

This ordering maximizes reversibility and makes performance attribution possible. A preparation rewrite, active-column PCG, and CMG arena must not be combined in one candidate.

## Adopted milestone: `PREP-RHS-1`

The next performance milestone is `PREP-RHS-1`: exact-output command preparation and repeated-RHS throughput. It has eight checkpoints.

### 0. Establish the measurement and hierarchy baseline

Add exclusive timers for validation, grouping, graph import and each pruning category, redensification, semantic ordering, compression imports and maps, resource modeling, lifecycle transition/restoration, hierarchy subphases, fit, leverage, target, unit adjustment, and complete residual certification. Count imports/stores, sorted elements, panel constructions, graph passes, material allocations and copies, logical versus physical column actions, active width by PCG iteration, and per-RHS iteration distributions.

At the same time, complete the existing API-7 full-command qualification on ordinary degree-two/three cases and the former degree-four boundary. Require no ordinary median regression greater than 10%, no restored density-four cliff, and all construction, component, residual, memory, and deterministic-replay gates. This is a release gate, not a new optimization candidate.

### 1. Retain exact-output plans

Implement separately and benchmark separately:

- immutable unit-to-cell and stratum-to-cell scatter plans;
- one firm-component projection plan per CMG context;
- removal of the apparently dead `fitted = firm_coefficient[design.firm,.]` assignment only after a direct source/test audit proves it unused;
- path compression or bulk semantic minima only if the new microprofiles show they are material.

Promotion gate: at least 5% complete-command gain on one decisive f1024 P200 cell, at least 15–20% improvement in the directly affected scatter or projection stage, no benchmark regression above 3–5%, exact plan/rank equality, and unchanged resource admission and scientific contracts.

### 2. Consolidate command-boundary preparation

Introduce a command-local preparation object that imports compressed-eligible numeric columns once, reuses graph vectors, returns the final active mask and dense maps together, and feeds compressed construction without seven re-imports. Keep Stata as the initial oracle for arbitrary string-ID grouping, semantic ranks, and canonical sort order. Reuse or overwrite O(R) vectors so the resource high-water is neutral or lower.

Promotion gate: at least 35% reduction from mark-through-compression and at least 1.25x complete-command improvement on matched CZ18 P20. Sample, graph diagnostics, semantic ranks, exact totals, random atoms, typed failures, resource receipts, data, sort state, and RNG state must remain unchanged.

### 3. Reuse bounded repeated-RHS workspaces

Add a caller-owned FE workspace and destination-buffer entry points for fit, leverage, and target. Capacity is bounded by the admitted maximum logical width. Reset every used column explicitly and keep complete original worker-plus-firm residual certification per RHS.

Promotion gate: at least 70% fewer material large-matrix allocations and at least 8% improvement in PCG plus certification time, with no stale-column, aliasing, route, residual, or failure-contract regression.

### 4. Pack active columns only where justified

If checkpoint 0 shows that logical active actions are below 90% of physical full-width actions, gather active columns into preallocated buffers at deterministic checkpoints and scatter results back to their original logical order. Do not adapt from runtime timings or random values.

Promotion gate: at least 7% PCG improvement on the gated cells and at least 15% less physical work, with identical logical action accounting, per-RHS iterations/statuses, residuals, and atom order. Otherwise retain the full-width path.

### 5. Fuse one dataflow boundary at a time

Evaluate independently:

- compressed Schur actions that write into destination buffers;
- final prediction plus complete residual certification and quarantined leverage/target contractions;
- tiled leverage gathering and compensated moment accumulation;
- target RHS construction and contractions using reused buffers;
- narrow dual incidence layouts, only if admitted memory stays within the direct model.

Each change must meet its own kernel kill criterion. The cumulative target is at least 15% improvement in the f1024 P200 numerical core. Reduction order, full certificates, and target/correction identities remain authoritative.

### 6. Redesign CMG storage only if it remains dominant

If exclusive profiles still identify CMG apply or Galerkin contraction as leading costs, prototype a new flat ping-pong arena or stable dense-label contraction. Do not enable the existing slower workspace. Require a fixed symmetric linear operation, quotient-SPD checks, explicit alias and memory receipts, and ordinary fallback chosen before RNG.

Promotion gate: at least 15% apply improvement across B1/B8/B32 for the conservative arena gate, with a preferred 30% target, and no cell more than 3–5% slower. A contraction rewrite needs at least 30% hierarchy-stage reduction on degree-four through degree-seven cases and no ordinary setup regression above 5%.

### 7. Keep later work independently killable

Only after checkpoints 0–6 should the project test, in order: hierarchy-derived coarse initialization, fixed structural deflation, stage-local canonical recycling, and a fixed-width block PCG microkernel. Each candidate must use structural information independent of probe values and public batch partition, preserve per-RHS acceptance, and retain the scalar solver as a deterministic fallback.

An explicit prepare/run/drop lifecycle is a separate owner decision after one-shot improvement. If approved, require a complete data/options/build signature, explicit invalidation and release, no owned RNG cursor, stale-handle rejection before RNG, first-call regression no greater than 5%, and at least 1.5x repeated-call gain; the stronger target is 2x on the second matched CZ18 P20 call.

## Speedup arithmetic and parity boundary

The current matched comparison is 305 seconds versus MATLAB's 43.802 seconds, a 6.96x gap. The adopted milestones narrow that gap but do not justify a parity claim:

| Scenario | Projected Stata time | Gain from 305 s | Remaining ratio to MATLAB |
|---|---:|---:|---:|
| Remove 35% of the 188.272 s preparation block | 239.105 s | 1.276x | 5.459x |
| Also remove 15% of the 110.286 s remaining numerical block | 222.562 s | 1.370x | 5.081x |
| Aspirational: remove 60% of preparation and 25% of that numerical block | 164.465 s | 1.854x | 3.755x |

These calculations overlap with some proposed optimizations and are planning bounds, not additive promises. Eliminating all measured selection and hierarchy work gives a rough repeated-call upper bound near 2.77x, which is why a prepared lifecycle can matter for repeated workloads but cannot establish one-shot parity.

## Required benchmark matrix

Every promoted candidate should be measured at clearly labeled boundaries: source-cold command, source-warm command, filesystem-first-read proxy, and isolated numerical-kernel warm. MATLAB reports must separately identify pool startup, import/conversion, maintained command call, serialization, and teardown.

Minimum coverage:

- matched CZ18 P20 and, when admitted, P200;
- f1024 P20/P200, degrees 2–7, rows-per-cell 1 and 8;
- fixed-C rows-per-cell ladder 1, 2, 4, 8, and 26.3;
- weak-connectivity degree-two/three families and the historical degree-four cliff;
- batch widths 1, 2, 4, 8, 16, 32, and 64;
- actual processor counts 1, 2, 4, and 8 where available;
- graph fixed points with no removal, repeated removal categories, alternating categories, tied components, parallel deletion units, and typed invalid inputs;
- cold/warm one-shot and, only if separately approved, explicit prepared runs.

Small kernels use one warm-up and seven measured repetitions. Decisive f1024 cells use at least five fresh-process repetitions, other matrix cells at least three, and matched CZ18 baseline/candidate runs at least three each in interleaved order. Preserve all runs and report median, minimum, maximum, median absolute deviation, host, runtime, actual processors, and direct memory receipts.

## Rejected or deferred paths

| Path | Decision |
|---|---|
| Parallelism-first optimization | Rejected: measured 4-to-8-core gains are much smaller than layout/batch opportunities. |
| Another hierarchy rewrite as the next milestone | Rejected: preserve and qualify the current robust hierarchy instead. |
| Enable the existing CMG workspace | Rejected: prior B4/B16 and large-case regressions are decisive. |
| Live-time route selection or post-RNG fallback | Rejected: violates reproducible, pre-RNG routing and failure contracts. |
| Reduced probes, looser tolerance, graph-only residual, or coupled batch stopping | Rejected: changes estimator or acceptance semantics. |
| Parallel/reordered random generation | Rejected unless atom matrices and terminal states are byte-identical. |
| Random-history-dependent recycling | Rejected: public batch/probe history must not change the solve path. |
| Dense firm-by-firm or observation-space objects | Rejected: violates the bounded-memory architecture. |
| Compiled plugin as the shipped implementation | Rejected: outside the pure Stata/Mata runtime boundary. A non-shipped experiment may estimate a compiled-kernel ceiling later. |
| Invisible automatic cross-command cache | Rejected: any reusable state must have an explicit owner, signature, receipt, and drop operation. |
| Public prepared lifecycle | Deferred for a separate owner decision after one-shot improvements. |
| Coarse seeding, deflation, recycling, block PCG | Deferred until exclusive profiles show action count is the remaining bottleneck. |

## Decision

Proceed with `PREP-RHS-1`, beginning with instrumentation and qualification evidence, then exact-output plan reuse, command-boundary preparation, and bounded repeated-RHS workspaces. Do not change the estimator or public API in this review round. Do not launch large cluster benchmarks until the owner supplies or approves the required scale and resources. Revisit a public prepared lifecycle only as an explicit follow-on decision.

This adjudication reconciles the two advisory reports. It does not authorize or qualify any performance implementation by itself.
