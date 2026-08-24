# Rust backend implementation status

Status snapshot: 2026-08-22. This document describes the source state in the
commit that contains it. Development remains on `main` at version
`0.3.0-dev`. The global native ABI version remains 1, every published legacy
layout and symbol remains frozen, and the legacy independent support mask
remains 38.

This is a development checkpoint, not a public-release or production-
qualification claim. Point estimates and numerical diagnostics only are
implemented; the package does not post `e(V)` or econometric standard errors.
Omitted `backend()` and `backend(auto)` continue to select Mata.

## Executive status

The Rust backend now contains working exact and generic W+F+Q estimators,
Counter-V1 JLA, diagonal and CMG solvers, structural algorithm/engine routing,
independent batch planning, direct memory admission, advisory wall-work
receipts, cancellation, and lossless numerical receipts. The previously
exposed explicit Rust exact and explicit generic-JLA routes remain usable.

The next planned native surface is implemented through Rust and the C shim:

- request capability V3;
- solve and interrupt V4;
- a frozen pre-RNG execution-plan receipt; and
- detailed receipt V7, whose first 840 bytes are the unchanged V6 layout.

The private Ado wrapper currently contains the complete V3 capability-query
slice, but does not yet invoke solve V4 or reconcile/export V7. Consequently,
the new automatic algorithm, engine, preconditioner, batch, and wall-planning
matrix is not yet a supported public route. Existing public and legacy native
entry points remain intact.

Separately, the Mata backend now implements the explicitly labelled
`stayers(both)` exact mixed-deletion point hybrid. It does not change the
mover headline, `e(b)`, graph receipts, or `e(sample)`. Rust stayer-hybrid
parity remains future work.

## Implemented estimator core

### Exact estimator

The Rust exact estimator supports the full worker--firm zero-sum quotient,
match and observation deletion, joint and fixed-offset nuisance controls,
frequency weights as physical copies, stored-row target mass, deletion IDs,
and up to 32 canonical controls. It returns all four point targets and checks
the complete original worker--firm--control residual, deletion rank, control
basis, target accounting, finite outputs, and direct memory boundaries.

Exact estimation uses no estimator RNG. Structural `algorithm(auto)` selects
exact when the checked identified dimension

`W + F - 1 + Q <= exact_limit`

and never reroutes because of a later memory, rank, or numerical failure.
Exact receipts record the JLA engine and preconditioner as not applicable.

### Generic JLA estimator

The generic matrix-free estimator supports W+F+Q models, match and observation
deletion, joint and fixed-offset nuisance controls, frequency and target
weights, deletion IDs, and 0--32 canonical controls. Counter-V1 probe atoms
are keyed by semantic entities rather than storage order.

The generic solver supports diagonal PCG, CMG PCG, and the registered
automatic rule:

- diagonal when `F < 256` or planned RHS count is below 8;
- otherwise CMG, with setup-only fallback to diagonal when automatic routing
  encounters an eligible setup/resource failure before RNG;
- forced CMG fails closed.

The full and FE solvers share one prepared CMG hierarchy when selected. Every
accepted RHS completes the original residual gate. No solver or engine
fallback is allowed after Counter addressing begins.

### Compressed JLA adapter

The legacy no-control match-deletion engine remains bitwise compatible. An
additive planned adapter now resolves its actual prepared route before RNG,
uses compressed-specific lifetime forecasts, supports independent automatic
or explicit leverage and target batches, and carries advisory wall, memory,
thread, route, fallback, and Counter receipts.

Automatic compressed widths use the Mata-compatible cap of 32. Explicit
widths preserve the request and use `min(request, probes)` as the active
width, including admitted requests above 32.

### Counter and resource receipts

Receipts distinguish rather than conflate:

- logical probe atoms;
- unique packed Counter words;
- represented Bernoulli trials; and
- generator evaluation work.

This distinction matters for observation deletion, whose current leverage
implementation evaluates each physical sign twice while preserving one
stateless Counter identity. Successful receipts report exact planned and
actual counts; every successful frozen plan reports zero Counter work before
plan freeze. Partial counts are not claimed on an error because the current
error object carries no solved execution receipt.

Direct memory accounting covers preparation, retained state, solver setup,
CMG persistence and workspaces, control projections, batch phases, RHS export,
result export, and caller copies. Automatic batch choices use deterministic
candidate ladders and are finalized before RNG. `wallseconds()` is advisory;
it cannot reroute, resize, relax a tolerance, or withhold a scientifically
valid result.

## Structural algorithm and engine routing

Algorithm and engine requests are resolved in one pure pre-RNG plan. Receipts
retain requested and selected values separately.

- `algorithm(auto)` uses only the retained W/F/Q dimensions and
  `exact_limit()`.
- Exact with `engine(auto|generic)` records selected engine not applicable.
- Exact with `engine(compressed)` is rejected.
- JLA `engine(auto)` selects compressed only for a certified eligible
  no-control match problem with the registered physical Counter plan;
  otherwise it selects generic.
- Forced compressed requests fail when scientifically ineligible.
- Batching, wall forecasts, memory pressure, and numerical behavior never
  change the scientific engine choice.

Omitted and automatic backend requests remain Mata by policy. There is no
backend-level automatic fallback in this checkpoint.

## Native ABI and private boundary

The following are implemented in Rust and declared in the C header without
changing global ABI version 1:

- capability request/receipt V3 (120/160 bytes);
- solve request V4 (288 bytes) and its interruptible form;
- execution-plan receipt V1 (1000 bytes); and
- detailed receipt V7 (1840 bytes), with V6 ending at offset 840.

Static Rust and C layout fences cover sizes, offsets, and prefixes. The V3
capability signature binds the complete request, including independent phase
modes, setup-fallback permission, wall value, and the frozen V2 prefix.

V4 validates capability and signature, reconciles the prepared generation,
resolves the retained-data estimator/engine plan, completes route/batch/wall/
memory admission before RNG, then executes exactly the selected exact,
compressed, or generic estimator. Successful state stores the immutable plan
used for execution.

The C shim supports the additive V3/V4 argument counts, V7 fetch and V6-prefix
reconciliation, and cleanup that preserves the primary export error. The
private `varcomp_kss_rust.ado` wrapper currently exposes only the completed V3
capability-query portion. V4 solve dispatch, V7 scalar/matrix reconciliation,
private corruption tests, and private licensed-Stata lifecycle tests are the
next unfinished unit.

## Public command surface at this checkpoint

The already integrated public Rust routes remain deliberately explicit:

1. Exact Rust route for `backend(rust) algorithm(exact)`, including controls,
   both deletion modes, joint/fixed-offset nuisance handling, weights,
   targets, samples, and deletion IDs within the exact size limits.
2. Generic Counter-V1 JLA for the explicit tuple
   `backend(rust) algorithm(jla) engine(generic)
   preconditioner(diagonal) batch(#) rng(counter_v1)`.
3. The legacy compressed no-control match route under its existing explicit
   restrictions.

The V4 planned route is not public yet. In particular, public Rust
`algorithm(auto)`, `engine(auto)` across compressed/generic selection,
`preconditioner(auto|cmg)`, automatic independent batches, and
`wallseconds()` must remain withheld until the private V4/V7 boundary and
public receipt reconciliation are complete.

## Mata stayer hybrid

`backend(mata) algorithm(exact) deletion(match) stayers(both)` now returns a
secondary all-worker point hybrid while preserving the mover result as the
headline. The hybrid:

- starts from the final robust mover sample;
- adds original complete-case one-firm stayers attached to retained mover
  firms when their physical history is at least two;
- never reclassifies graph-dropped movers as stayers;
- fits one combined model and uses one pooled target normalization;
- deletes mover matches as blocks and stayer physical copies one at a time;
- supports joint and fixed-offset controls, frequency weights, explicit
  stored-row target mass, and all four targets; and
- reports separate mover-match and stayer-observation corrections with an
  explicit statement that stayers are not match-cluster robust.

Observation deletion, JLA, and Rust requests for `stayers(both)` fail with
typed unsupported statuses. A requested hybrid failure withholds the whole
command rather than silently posting only the mover headline.

The independent regression oracle constructs its own grounded W+F design,
pooled target quadratics, and literal refits. It separately verifies mover and
stayer correction rows for joint, fixed-offset, and frequency-expanded
fixtures without calling the production design or target helpers.

## Current green validation evidence

The following gates passed for the source slices they cover:

- Rust core: 190 executable tests plus doc tests.
- Rust core all-target strict Clippy, formatting, and diff checks.
- Rust plugin: 55 tests, including V3 signature/layout, all three V4 branches,
  V7 plan mapping, and V4 UserBreak lifecycle.
- C header static ABI compilation.
- C shim strict compilation with `-Wall -Wextra -Werror`.
- Standalone C interrupt and error-transport tests, including V3/V4 argument
  routing, V7 lifecycle, schema/export corruption, and primary-error cleanup.
- Mata stayer hybrid focused and semantics tests.
- Stata quick and full suites for the completed public/Mata slices.
- Python package tests: 375 passed.
- Package-layout tests: 24 passed.
- Deterministic CMG assembly check.
- Integrated local qualification for the completed Mata/public slices:
  `VARCOMP_KSS LOCAL QUALIFICATION PASS`.

Because the private V4/V7 wrapper slice is intentionally incomplete, a final
post-integration workspace check, strict workspace Clippy/format pass,
licensed private V4/V7 Stata suite, package rebuild, and source-bound native
qualifier have not yet been run and are not claimed here.

## Packaging and qualification boundary

The tracked package remains portable and does not ship native binaries. The
local macOS plugin is a developer/qualification artifact. No plugin containing
the new V4/V7 source has been rebuilt, staged, or bound to a qualification
receipt.

The expanded macOS qualifier previously exercised arm64 and Rosetta thin,
universal, and install matrices, but intentionally withheld its final receipt
because Rust sources changed during execution. It must be rerun after source
quiescence. Rosetta is compatibility evidence, not native Intel qualification.

Still open:

- complete private solve V4 and detailed V7 reconciliation;
- expose the planned route through the public command without changing
  omitted/automatic backend defaults;
- implement Rust exact parity for the separate stayer hybrid;
- rebuild the local native plugin and rerun source-bound macOS qualification;
- native Intel, Windows, and Linux Stata qualification;
- representative scale, RSS, and Rust-versus-Mata performance evidence;
- sanitizer/Miri/fuzz and release-security evidence where applicable;
- SBOM/release packet and documentation closure; and
- final independent human mathematical and license/provenance review.

Public distribution remains disabled until the human license/provenance gate
is complete.

## Exact resume point

Resume in this order:

1. Finish V4 solve dispatch in `varcomp_kss_rust.ado`.
2. Reconcile and return every V7 execution-plan field, including selected
   route/fallback, both batch phases, wall work, memory, threads, and Counter
   accounting.
3. Add private Stata success, corruption, nonconvergence, UserBreak,
   restoration, release, and idle-state tests for exact/compressed/generic V4.
4. Run full Rust/C/private-Stata gates and obtain independent ABI review.
5. Expand the public Rust routing matrix and `e()` receipts, preserving Mata
   for omitted `backend()` and `backend(auto)`.
6. Implement and independently verify Rust exact stayer-hybrid parity.
7. Update remaining user documentation, rebuild the native plugin, rerun the
   source-bound qualifier, and only then consider the atomic version/release
   closure.
