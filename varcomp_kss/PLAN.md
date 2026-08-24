# Active package plan

## Objective

Maintain `varcomp_kss` 0.3.0-dev as the sole public Stata command while
preserving every estimator, sample, deletion, weighting, nuisance, RNG,
routing, residual, resource, return-shape, and caller-state contract.

The permanent default remains Mata. Rust is an explicit opt-in backend whose
capability and execution receipts must reconcile compositionally.

## Current state — 24 August 2026

The active milestone is public Rust `algorithm(auto)`.

Completed and retained:

- V3 compositional request capability;
- V4 result export and V7 execution-plan receipts;
- explicit Rust exact estimation;
- explicit and planned generic JLA;
- planned compressed JLA and `engine(auto)` selection;
- structural pre-RNG route/batch/memory resolution;
- exact-V7 reconciliation and posting helpers;
- package, clean-install, and macOS qualifier source lists containing
  `_vckss_rust_reconcile_exact_v7.ado` and
  `_vckss_rust_post_exact_v7.ado`; and
- exact-limit, selection-reason, selected-engine, pre-RNG counter, and
  plan-memory bindings added in `3321f8e4c41589a16015061bb37b8ce467f97085`.

Current evidence:

- the licensed quick suite is green for
  `caec94d6aae0062267d69738fbf3f867896f2eb1`;
- the last source-local plugin build,
  `b000d290e011acdd82332ddec9e9b315b6672e9e`, failed in the direct
  `_vckss_rust_reconcile_exact_v7` test at `r(ok) == 1`;
- that failure occurred after the exact-limit binding change, so the remaining
  mismatch is elsewhere in the V7 exact receipt reconciliation;
- `75e83f8de3bfbdac2ac5e7c3031a9939baa392cc` adds a focused diagnostic that
  prints `r(detail)` and `return list` on the next plugin run; and
- redundant trusted-patch staging left by a manifest-already-present failure
  was removed in `ab71a9a3a507d5c185a7c85dcc78a60ba7f40da3`.

The package manifest itself is correct. The exact reconciler and poster are
already installed and included in the qualifier source manifest.

## Remaining milestone work

1. Run the source-local `plugin-build` profile on current `main` and capture the
   exact `EXACT_V7_RECONCILE_FAIL` detail from the focused test.
2. Repair the direct exact-V7 receipt mismatch without deleting or weakening
   any expected argument, plan, residual, accounting, memory, or pre-RNG gate.
3. Re-run the direct helper test until both native arm64 and universal/Rosetta
   paths pass where available.
4. Widen the public Rust option predicate only for the planned
   `algorithm(auto)` tuple. Preserve the permanent Mata default and all explicit
   consent/RNG requirements.
5. Extend `_vckss_rust_generic_planned` to recognize the planned exact result
   family (`selected_engine_code == 3`, exact RHS schema) before its current
   compressed/generic family switch.
6. Build and pass the exact preparation, graph, and capability contexts to
   `_vckss_rust_post_exact_v7`; release the native handle exactly once on every
   success or failure path.
7. Add a public command regression for
   `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)` selecting exact.
   It must verify:
   - requested versus selected algorithm and engine receipts;
   - `result_family == "exact"` and the V7 plan schema;
   - no estimator RNG consumption and complete caller RNG restoration;
   - exact results and accounting identities;
   - retained `e(sample)`, data signature, and sort restoration;
   - clean installation; and
   - typed failure on inconsistent or unsupported tuples.
8. Run the Python/generated-source gates, Stata quick and full suites, and the
   complete source-local plugin qualifier. Record exact-SHA receipts before
   calling the milestone complete.

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
