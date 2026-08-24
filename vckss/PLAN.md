# Active package plan

## Objective

Maintain `vckss` 0.4.0-dev as the sole public Stata command while
preserving every estimator, sample, deletion, weighting, nuisance, RNG,
routing, residual, resource, return-shape, and caller-state contract.

The permanent default remains Mata. Rust is an explicit opt-in backend whose
capability and execution receipts must reconcile compositionally.

## Current state — 24 August 2026

The public Rust `algorithm(auto)` exact-selection milestone is locally
complete. The active work is the private repository/consumer cutover and
published exact-SHA CI evidence; this is not a public-release claim.

Completed and retained:

- hard-cut public/package/repository identity `vckss` at version
  `0.4.0-dev`, with no predecessor-command wrapper;
- source-bound relocation inventory and byte checks for archived evidence;
- CMG ownership API 8 and generator API 5 for the renamed target;
- V3 compositional request capability;
- V4 result export and V7 execution-plan receipts;
- explicit Rust exact estimation;
- explicit and planned generic JLA;
- planned compressed JLA and `engine(auto)` selection;
- structural pre-RNG route/batch/memory resolution;
- exact-V7 reconciliation and posting helpers;
- the narrowly admitted public
  `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)` path, including
  direct exact-family posting before either JLA family switch;
- exact zero-RNG, direct-memory, complete-residual, accounting, caller-state,
  `e(sample)`, release, and idle-registry regression coverage;
- package, clean-install, and macOS qualifier source lists containing
  `_vckss_rust_reconcile_exact_v7.ado` and
  `_vckss_rust_post_exact_v7.ado`; and
- exact-limit, selection-reason, selected-engine, pre-RNG counter, and
  plan-memory bindings added in `3321f8e4c41589a16015061bb37b8ce467f97085`.

Current evidence:

- the untouched receipt-tip failure at
  `f9fb00dc6254116a51853b1c2be7162b0a48368e` (source commit
  `7fcf1b20105a546e993e0b3186c842f05f1f79a8`) is preserved in
  `docs/EXACT_V7_BASELINE_FAILURE_2026-08-24.md`;
- the complete focused return list is preserved in
  that file; the direct mismatches were repaired in `c003895` and `b8f5855`
  with the diagnostic and all gates retained;
- the public feature is commit
  `6954da6e190680a65ac271b71a33ece8d0fcfab1`;
- Python (375 tests), generated CMG drift, Rust fmt/Clippy/workspace/backend,
  licensed Stata quick/full, and the integrated `run_checks.py` gates pass;
  and
- the clean source-local `plugin-build` receipt for `6954da6e190680a65ac271b71a33ece8d0fcfab1`
  passes thin arm64, thin x86_64/Rosetta, universal under both architectures,
  lifecycle, focused native routes, corrupt-receipt failures, and isolated
  clean installs.

The package manifest itself is correct. The exact reconciler and poster are
already installed and included in the qualifier source manifest.

## Remaining milestone work

1. Migrate the paper, Monte Carlo entrypoints, and vault operating memory while
   preserving source-bound historical result labels and the explicit two-step
   equivalence chain.
2. Rename the private GitHub repository and local checkout, update `origin`,
   and retain runner ID 21 with its historical installation/display name.
3. Push `main`, run quick and `plugin-build` under the new repository identity,
   and require success receipts bound to the final source SHA.
4. Fast-forward over any receipt-only commit and leave implementation, paper,
   and consumer worktrees clean with matching local/remote `main`.

Do not implement a post-RNG fallback, weaken receipt reconciliation to make the
test pass, or route an exact-selected result through a JLA poster.

## Deferred work

The retained PREP-RHS-1, FE-BUF-1, and PREP-BND-1 engineering results remain
valid for their source-bound candidates:

- `docs/PREP_RHS_1_RESULTS_2026-08-19.md`;
- `docs/FE_BUF_1_RESULTS_2026-08-19.md`; and
- `docs/PREP_BND_1_RESULTS_2026-08-19.md`.

`GRAPH-FP-1` remains the next performance investigation after the Rust
auto-algorithm milestone is closed or explicitly deferred. Keep graph
preparation changes separate from high-probe numerical work so performance
claims remain causal and bisectable. Timing and headroom targets are advisory;
scientific and direct-allocation gates are hard.

A public prepare/run/drop lifecycle remains a separate owner decision. No
invisible cache or command-surviving graph state is permitted.

## Completion gates

A change is complete only when applicable gates establish:

1. deterministic generated CMG source with no drift;
2. passing package, CMG, and retained comparator Python tests;
3. passing Stata quick/full and isolated install tests;
4. passing source-local Rust plugin qualification for every claimed native
   route and architecture;
5. exact-SHA receipts binding the tested source;
6. unchanged scientific, residual, accounting, routing, resource, and
   restoration contracts; and
7. no leftover `.ci/codex/` staging files.

Public release remains disabled until human license/provenance review is
complete.
