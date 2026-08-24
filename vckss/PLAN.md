# Active package plan

## Objective

Produce a private `vckss` `0.4.0-alpha.1` release candidate with the Rust
backend at Mata feature parity, Rust-preferred automatic routing, macOS and BU
SCC qualification, and a complete reproducible benchmark report. Windows and
public distribution are deferred.

Scientific, numerical, sample, deletion, weighting, nuisance, RNG, residual,
accounting, memory, return-shape, and caller-state contracts remain hard gates.

## Baseline — 24 August 2026

Source `165a25cb3220a94c34656c2c22047ae0d60f26c8` was clean and matched
`origin/main`. The following baseline facts were established before alpha work:

- Python had one infrastructure failure because the mutable
  `.ci/stata/latest.json` pointer was incorrectly byte-locked by the rename
  inventory; archived per-SHA receipts were unaffected. Commit `0d63c29`
  repairs and tests that policy, after which all 375 tests pass.
- Generated CMG source is drift-free.
- Rust 1.81 formatting, Clippy, workspace tests, and Stata-backend tests pass.
- Public auto-exact is already implemented and qualified on macOS arm64 and
  Rosetta at its source-bound milestone.
- A protected cleanup removed 42,120 ignored disposable files and 3.98 GB of
  stale targets, logs, caches, traces, and obsolete local plugin artifacts;
  tracked evidence and development dependencies were retained.

The generated current feature ledger is
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md).

## Public alpha contract

- Omitted `backend()` and `backend(auto)` prefer Rust when the complete
  effective request is supported and the native runtime passes transport and
  compositional capability checks.
- A missing plugin or structurally unsupported effective request may fall back
  to Mata before native preparation and estimator RNG. Stale ABI/build ID,
  corrupt receipts, preparation, memory, rank, numerical, convergence,
  resource, and UserBreak failures fail closed without fallback.
- `backend(rust)` is strict; `backend(mata)` always selects Mata.
- Omitted `rng()` and `rng(auto)` select Counter-V1 on Rust and Stata RNG on
  Mata. Explicit `rng(counter_v1)` is strict Rust consent. Explicit
  `rng(stata)` selects Mata and conflicts with `backend(rust)`.
- The omitted algorithm is MATLAB-like `algorithm(jla)` with 200 probes.
  Explicit `algorithm(auto)` retains frozen exact-small/JLA-large selection.
- Other defaults remain match deletion, mover headline, joint nuisance, and
  automatic engine, preconditioner, and batch selection.
- Record requested/selected backend, fallback occurrence/reason, RNG,
  algorithm, engine, route, preconditioner, batch, memory, and selection
  reasons in `e()`. Alpha results post `e(status) == "ALPHA"` and never `e(V)`.

## Milestones

1. **M0 — baseline, parity ledger, and hygiene.** Keep source gates green,
   install protected dry-run/apply cleanup, and synchronize active docs.
2. **M1 — default routing.** Route from effective options rather than supplied
   flags; add `rng(auto)`, preflight-only fallback, strict explicit routes, and
   clean-install routing/state tests.
3. **M2 — broad public parity.** Admit automatic compressed/generic JLA,
   controls, deletion modes, weights, targets, deletion IDs, route/batch/wall
   options, and `probeorder()` wherever the native capability accepts them.
4. **M3 — exact stayer hybrid.** Implement the frozen mixed-deletion design
   with a versioned stayer-augmentation V1 lifecycle, mover headline, separate
   stayer correction, differential oracles, and lifecycle/resource tests.
5. **M4 — platforms and safety.** Requalify macOS arm64/Rosetta; build and
   qualify Linux x86-64 with Stata MP 19 on SCC; add Miri, fuzz, sanitizer,
   dependency, license, and SBOM evidence. Windows stays nonblocking.
6. **M5 — performance.** Instrument first; investigate graph fixed-point,
   ingestion/preparation, repeated-RHS batching, then deterministic parallel
   work. Retain only scientifically identical, independently reversible wins.
7. **M6 — benchmark report.** Run the registered synthetic and checksum-bound
   CZ18 matrix; publish compact data, figures, validation receipt, and a
   visually verified PDF without placeholders.
8. **M7 — alpha packet.** Freeze one source commit, build all artifacts from
   it, collect exact-SHA receipts, tag `v0.4.0-alpha.1`, and create a private
   prerelease with no public or Windows claim.

Use focused local red/green commits. Push after each completed milestone, run
the exact-SHA quick lane for every push, and require `plugin-build` after any
native boundary change. Receipt-only CI commits do not change the tested source.

## Performance acceptance

Rust must be at least twice as fast as Mata by median complete-command runtime
for the 8,192-firm/200-probe synthetic headline and the checksum-bound
CZ18/200-probe headline. No supported JLA benchmark cell may be more than ten
percent slower than Mata. Scientific, residual, accounting, direct-memory,
state-restoration, and typed-failure gates take precedence over timing.

The benchmark report compares public Rust and Mata with identical `vckss`
requests. The pinned maintained MATLAB comparator is descriptive and limited
to compatible no-control match-JLA cells because its RNG and legacy
finite-projection expression differ.

## Completion gates

The alpha candidate requires:

1. every alpha-required parity row qualified on its claimed platform;
2. Python, generated CMG, Rust fmt/Clippy/workspace/backend, Stata quick/full,
   integrated, clean-install, and source-local plugin gates passing;
3. macOS arm64/Rosetta and SCC Linux x86-64 exact-source receipts;
4. lifecycle, malformed-receipt, interruption, memory, safety, and benchmark
   gates passing without weakened requests;
5. a rendered and visually inspected benchmark PDF plus reproducible compact
   inputs and summaries;
6. clean local/remote `main`, no `.ci/codex/` transport, no disposable logs,
   and current documentation/Vault status; and
7. completed human mathematical and license/provenance review before any later
   public release. The private alpha does not make that public-release claim.
