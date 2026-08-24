# Rust backend implementation plan

**Status:** active implementation on `codex/rust-backend-implementation`  
**Package:** `vckss`
**Target:** Stata 18 and later on 64-bit Windows, Linux, Intel macOS, and Apple Silicon macOS  
**License boundary:** GPL-3.0-only for the CMG-derived Rust component and a distributed plugin containing it

## Decision

The production Rust route will own the complete numerical architecture. The Stata ado layer remains the stable public command and owns syntax, factor-variable materialization, caller-state protection, and `e()` posting. Rust owns marked-sample ingestion, canonical identifiers, graph fixed-point selection, compression, exact calculations, matrix-free fixed-effect operators, diagonal and CMG preconditioning, PCG, randomized inverse actions, KSS corrections, complete residual certification, deterministic parallelism, and resource receipts.

Mata remains a compatibility backend and independent oracle until the Rust route has full feature parity and cross-platform qualification. The Rust route must never call back into Mata.

## Current CMG baseline

The current `main` branch contains the improved API-7 CMG component. The old density-four setup pathology is closed and is not an architectural premise for this work. Rust must first reproduce the current hierarchy semantics, safeguards, router behavior, and diagnostics. Density-four and adjacent graph families become permanent regression fixtures so that the repaired behavior cannot regress.

No alternative preconditioner replaces CMG merely because it reduces iteration counts. Setup time, hierarchy memory, application time, repeated-RHS cost, full residuals, and failure rates are all part of the decision.

## Non-negotiable scientific contracts

- Point estimates only; probe dispersion is numerical Monte Carlo error, not econometric inference.
- Worker variance, firm variance, worker-firm covariance, and total variance retain their current definitions and accounting identity.
- Coefficient cells, deletion units, and target strata remain distinct indices.
- Match deletion remains the default and targets movers under the current retained-sample fixed point.
- Frequency weights are literal positive integer physical-copy counts. Explicit target weights are stored-row target mass and are not multiplied by frequency weights.
- The full-firm zero-sum quotient is the solve space. Display grounding occurs only after convergence.
- Every accepted right-hand side passes the complete original worker-plus-firm normal-equation residual using the current normalization and `max(1e-11, 10*tolerance())` threshold.
- No hidden ridge, pseudoinverse repair, graph change, tolerance relaxation, probe change, or fallback after estimator RNG begins.
- Caller data, `e(sample)`, RNG state, and sort-jumbler state are restored on every exit.

## End-state architecture

```text
Stata ado command
  syntax, factor variables, sample marker, preserve/restore, e() posting
       |
       v
small C shim using the official Stata plugin interface
  caller-thread-only SPI access and panic-safe Rust dispatch
       |
       v
Rust session and capability layer
  ABI handshake, typed state, cancellation, receipts
       |
       +-------------------------+
       |                         |
       v                         v
preparation engine           numerical engine
  validation                  exact finite oracle
  canonical IDs              FE/Schur operators
  graph fixed point           diagonal preconditioner
  C/G/S compression           improved CMG
  target plans                scalar/batched PCG
  memory admission            JLA/KSS correction
       |                         |
       +------------+------------+
                    v
 deterministic parallel runtime
```

## Distribution

A single byte-identical native file cannot span operating systems. The package will ship one self-contained plugin per operating-system family:

```text
vckss_rust_windows_x64.plugin
vckss_rust_linux_x64.plugin
vckss_rust_macos.plugin       # universal x86_64 + arm64
```

The final plugin statically contains the Rust core and its Rust dependencies. It must not require a separately installed Rust runtime, Python, Java, Julia, BLAS, OpenMP, CUDA, or package manager. Linux targets an intentionally old glibc baseline. Windows uses the MSVC ABI. The two macOS slices are combined and tested as a universal binary.

## Public routing

The intended public selector is:

```stata
vckss ..., backend(auto)
vckss ..., backend(mata)
vckss ..., backend(rust)
```

`backend(auto)` selects Rust only after a capability handshake and before estimator RNG. Unsupported features fall back to Mata with an explicit receipt. `backend(rust)` fails with a typed error rather than silently changing algorithm or sample. Existing `engine()` and `preconditioner()` meanings remain unchanged.

## Native lifecycle

The same plugin binary implements staged commands:

```text
capabilities -> prepare -> solve -> export -> release
```

`prepare` copies numeric values from the Stata SPI into Rust-owned columnar buffers, validates them, selects and certifies the graph, and constructs compressed state. It returns a generation-checked opaque handle and writes the retained-sample flag. After a safe preserve/clear transition, `solve` performs the entire numerical calculation without Stata callbacks. `export` fills preallocated Stata matrices/scalars. `release` is idempotent and frees every native allocation.

Only the caller thread may invoke Stata SPI routines. Worker threads operate exclusively on Rust-owned memory. No pointer into Stata-managed storage survives an SPI call.

## Data and graph architecture

The principal dimensions are retained rows `R`, workers `W`, firms `F`, coefficient cells `C`, deletion units `G`, target strata `S`, controls `Q`, probes `P`, and active batch width `B`.

Dense identifiers use `u32` when cardinality permits and a checked `u64` path otherwise. Offsets, counts, physical mass, prefix sums, and byte calculations use checked 64-bit arithmetic. Outcomes and weights use `f64`. The implementation is columnar and never creates per-row heap objects.

Canonical IDs and semantic order are independent of hash iteration, thread scheduling, and permitted input row permutations. Worker-major and firm-major cell orders are explicit so Schur actions need neither sorting nor floating-point atomics.

Rust implements the complete current graph fixed point: deterministic largest component selection, mover/degree restrictions, worker articulation detection, deletion-unit bridge detection, active-mask updates, repetition until stable, and independent final certification. Parity requires exact agreement with Mata on retained observations, maps, component choice, fixed-point iterations, removals, and certificates.

## Fixed-effect operator and exact oracle

For the identified weighted block system, Rust eliminates the diagonal worker block and applies the firm-side Schur operator matrix-free. Scalar and tiled matrix right-hand sides share group traversals. Reconstruction and the complete original-equation residual are separate mandatory kernels.

A Rust exact engine assembles and factors only small identified systems below strict dimension and memory thresholds. It provides an independent finite oracle for graph, Schur, PCG, leverage, correction, and target tests and is never routed on large systems.

Controls and nuisance parameters use the current reviewed block algebra, low-dimensional dense solves, and the same full-system certificate. They are part of the endpoint, not a permanently unsupported exception.

## PCG and improved CMG

The baseline PCG state machine includes zero-RHS success, finite-input checks, positive curvature and preconditioner checks, reliable residual replacement, stagnation and maximum-iteration statuses, independent status for every logical RHS, interruption points, and final complete-system certification.

The first repeated-RHS optimization is batched scalar PCG: operator and CMG traversals are shared, but each column retains an independent recurrence and acceptance decision. Block or recycled Krylov methods remain experimental until they improve end-to-end time without weakening any gate.

Rust ports the current improved API-7 CMG hierarchy before introducing new algorithms. Immutable level state is separated from setup and application workspaces. One validated hierarchy and terminal factorization are reused across all KSS right-hand sides. Requirements include deterministic aggregation, bounded complexity, a dense terminal cap of 6,144 vertices, fixed linear symmetric V-cycles, quotient-SPD behavior, typed failure, attempted-level diagnostics, and exact memory forecasting before allocation.

## RNG, probes, and corrections

Two versioned modes are planned: a Stata-compatibility contract where exact reproduction is maintainable, and `VCKSS-COUNTER-V1`, a domain-separated counter-based contract for deterministic parallel execution.

Logical atoms are keyed by contract version, master seed, domain, probe number, canonical identity, and subdraw. Thread count, scheduling, routing, and batch width cannot change them. Compressed Rademacher sums preserve the exact registered distribution; approximations are never silently substituted.

Probe work streams by admitted batch:

```text
generate atoms -> construct RHS -> batched PCG/CMG
-> full residual check -> accumulate corrections/targets -> release batch
```

All KSS correction formulas, deletion-unit aggregations, target strata, centering, and accounting identities are typed Rust kernels with deterministic or compensated accumulation.

## Parallelism and memory

One long-lived deterministic thread pool is created per command. Fixed contiguous partitions and fixed merge trees make results scheduling-independent. Parallel phases include post-SPI validation, integer sorting, aggregation, degree counts, worker/firm operator phases, CMG smoothing, batched RHS work, and target reductions.

Every phase has a checked receipt:

```text
peak = persistent + phase + batch + threads + reserve
```

Large allocations are fallible and occur only after admission. Ingestion/sort arrays are freed after compression, graph setup arrays after certification, and CMG construction arrays after hierarchy finalization. Workspaces are reused. A later optional external-memory preparation route uses versioned, checksummed temporary partitions inside the same plugin binary when raw Stata data and native preparation cannot coexist in memory.

## Receipts, tests, and qualification

The Rust route records backend/build/ABI versions, contracts, dimensions, graph iterations, topology hashes, CMG levels and complexity, RHS and iteration summaries, reduced and full residuals, phase wall times, modeled and measured memory, RNG domains, batch widths, threads, route/fallback decisions, and target accounting residuals.

Qualification layers are pure Rust unit/property tests, explicit finite matrix oracles, Rust/Mata differential fixtures, row-permutation/relabeling/thread/batch metamorphic tests, density-four CMG regression families, failure injection and sanitizers, licensed Stata integration on all targets, and scale benchmarks approaching 30 million workers, 1 million firms, and 20 years.

Performance reports ingestion, graph, compression, CMG setup, solve, correction, export, complete wall time, and process peak memory separately. Iteration-count gains alone are not qualification.

## Delivery sequence

- [x] Contract review and current improved CMG baseline identified.
- [x] Rust workspace, checked types, deterministic partition runtime, capability ABI, and cross-platform CI.
- [ ] Canonicalization, compression, and topology receipts.
- [ ] Complete graph fixed point and parity fixtures.
- [ ] Matrix-free FE operator, exact oracle, and full residual kernel.
- [ ] Diagonal scalar and batched PCG.
- [ ] Current improved CMG hierarchy and V-cycle.
- [ ] Counter RNG, probe pipeline, KSS correction, and targets.
- [ ] Controls, nuisance modes, and all deletion modes.
- [ ] Stata C shim, staged context lifecycle, and standalone artifacts.
- [ ] `backend()` ado routing and full `e()` receipts.
- [ ] Cross-platform Stata qualification and scale benchmarks.
- [ ] Human mathematical, licensing, and provenance review before public release.

Each checked item requires code, tests, and a pushed commit. Compilation or source inspection alone does not establish numerical parity or production qualification.
