# Rust backend test plan

This is the active native test plan. Dated implementation and CI snapshots are
kept under [`progress/`](progress/) and must not be treated as current
instructions. The authoritative milestone order is
[`../vckss/PLAN.md`](../vckss/PLAN.md).

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
./.venv/bin/python vckss/cmg/tools/assemble.py --all --check
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
./.venv/bin/python vckss/tools/run_checks.py
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

## Completed focused milestone: planned auto-exact

Commit `6954da6e190680a65ac271b71a33ece8d0fcfab1` closes these gates locally:

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

The focused direct test retains `EXACT_V7_RECONCILE_FAIL` and the returned
receipt list on any future reconciliation failure. The clean source-local
`plugin-build` receipt is green for thin arm64, thin x86_64/Rosetta, and
universal execution under both architectures, including clean installs.

## Active alpha qualification

The generated feature ledger is
[`../vckss/docs/RUST_MATA_PARITY.md`](../vckss/docs/RUST_MATA_PARITY.md).
Every alpha-required row must be `qualified` on its claimed platform before
tagging the release candidate.

The required new suites cover:

1. Rust-preferred omitted/automatic backend routing and `rng(auto)` with only
   missing-runtime or structurally unsupported preflight fallback to Mata.
2. Strict explicit backend/RNG mismatches and fail-closed stale, corrupt,
   preparation, memory, convergence, numerical, resource, and UserBreak exits.
3. Effective-option admission for automatic exact/compressed/generic result
   families, controls, deletion modes, weights, targets, deletion IDs,
   `probeorder()`, route, batch, and wall advisories.
4. Exact `stayers(both)` augmentation with independent dense and Mata oracles,
   mover-headline invariance, pooled target accounting, and lifecycle stress.
5. Linux x86-64 clean installation and real Stata MP 19 execution on SCC,
   bound to the exact source SHA and successful SGE accounting.
6. Miri, malformed-ABI fuzzing, C-shim sanitizers, dependency/license/SBOM,
   and registered performance/accuracy gates.

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

Windows Stata/plugin qualification, native Intel hardware qualification, a
command-surviving native cache, public distribution, and final public-release
mathematical/license/provenance approval remain deferred. Linux/SCC, safety,
large-scale RSS/performance, and Rust stayer parity are alpha gates.
