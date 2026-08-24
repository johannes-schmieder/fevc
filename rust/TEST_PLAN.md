# Rust backend test plan

This is the active native test plan. Dated implementation and CI snapshots are
kept under [`progress/`](progress/) and must not be treated as current
instructions. The authoritative milestone order is
[`../varcomp_kss/PLAN.md`](../varcomp_kss/PLAN.md).

## Evidence rules

- Bind every claim to the exact tested source SHA and profile.
- A licensed Stata receipt does not by itself prove Rust formatting, Clippy, or
  workspace tests; inspect the workflow jobs as well.
- A green quick profile is not plugin qualification.
- Stata batch process status is not authoritative by itself; require the
  profile receipt and explicit PASS markers.
- Preserve failures and exact diagnostics. Do not weaken a gate or change the
  requested estimator merely to obtain green output.
- Receipt-only `[skip ci]` commits are bookkeeping, not the tested source.

## Routine source gates

Run from a clean checkout:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python varcomp_kss/cmg/tools/assemble.py --all --check
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

When Stata is available, also run:

```bash
./ci/run_stata_ci.sh quick
./ci/run_stata_ci.sh full
```

The integrated package command is:

```bash
./.venv/bin/python varcomp_kss/tools/run_checks.py
```

## Plugin qualification

A meaningful change to Rust estimator code, the plugin ABI, the Ado native
boundary, result posting, packaging, or native route tests requires:

```bash
./ci/run_stata_ci.sh plugin-build
```

The comprehensive macOS profile runs locked Rust and C gates, SPI
authentication, ABI/layout checks, thin and universal builds, binary audits,
native arm64 lifecycle and clean-install tests, and Rosetta x86_64 coverage
when available. The exact profile and source SHA must be present in
`.ci/stata/results/<sha>.json` with `status == "success"`.

This evidence qualifies only the recorded macOS candidate and routes. It does
not qualify native Intel hardware, Linux, Windows, production scale, or public
release.

## Current focused milestone: planned auto-exact

The next implementation must close all of these gates:

1. Direct V4 exact result export reconciles through
   `_vckss_rust_reconcile_exact_v7` with the exact limit, algorithm/engine
   selection reasons, plan complexity, memory, residual, accounting, and zero
   pre-RNG counter facts intact.
2. The public Rust option predicate admits only the intended planned
   `algorithm(auto)` tuple and preserves explicit backend/RNG consent.
3. `_vckss_rust_generic_planned` recognizes the exact result family before its
   compressed/generic switch and constructs the exact preparation, graph, and
   capability contexts.
4. `_vckss_rust_post_exact_v7` posts exact results, V7 plan fields, and
   `e(sample)` and releases the native handle exactly once.
5. A public command test covers
   `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)` selecting exact.
6. The test verifies requested/selected algorithm and engine, result family,
   exact identities, no estimator RNG consumption, caller RNG/data/sort
   restoration, clean install, and typed corrupt/inconsistent receipt failure.
7. Quick, full, and plugin-build profiles pass at the exact final source SHA.

The focused direct test currently prints `EXACT_V7_RECONCILE_FAIL` and the
returned receipt list when reconciliation fails. Keep that diagnostic until
this milestone is closed.

## Hard acceptance contracts

All accepted native routes must preserve:

- the retained sample, target population, deletion unit, nuisance convention,
  frequency and target semantics, and four target columns;
- identification, control-basis, deletion-rank, reciprocal/maker, accounting,
  and finite-output gates;
- the complete original-system residual for every RHS at
  `max(1e-11,10*tolerance())`;
- structural pre-RNG algorithm, engine, route, fallback, batch, memory, wall,
  and Counter receipts;
- direct allocation within `memory_gib()`;
- exact request/selection/result-family reconciliation; and
- caller data, `e(sample)`, RNG algorithm/stream/state, sort state, release,
  and idle registry restoration on success, error, and UserBreak.

Point estimates only are implemented. Probe dispersion is numerical Monte
Carlo error, not an econometric standard error, and `e(V)` must not be posted.

## Deferred qualification

Keep these separate from the active auto-exact repair:

- Rust parity for the separately labelled Mata `stayers(both)` exact hybrid;
- native Intel, Linux, and Windows Stata/plugin qualification;
- sanitizer, Miri, fuzz, and release-security evidence;
- representative large-N RSS and performance comparisons; and
- final independent mathematical and human license/provenance review.
