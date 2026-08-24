# Testing and qualification

This file is the active package testing guide. Dated performance reports and
Rust checkpoints are evidence, not instructions. Start with
[`PLAN.md`](PLAN.md) for the current milestone and
[`docs/README.md`](docs/README.md) for the documentation map.

## Source gates

Use the repository interpreter:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python vckss/cmg/tools/assemble.py --all --check
```

The Python gate covers package, CMG, packaging, retained comparator, and static
contract tests. Generated CMG source must remain drift-free.

For Rust source changes also run:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

## Licensed Stata profiles

From the repository root:

```bash
./ci/run_stata_ci.sh quick
./ci/run_stata_ci.sh full
```

The integrated local command is:

```bash
./.venv/bin/python vckss/tools/run_checks.py
```

A successful suite prints its explicit terminal PASS marker. Stata 18 batch
launchers can return shell status zero after a do-file error, so the marker and
machine-readable `stata_rc` are authoritative.

The quick suite is the ordinary source gate. The full suite is required for a
substantial package or numerical change. Both include routing, sample,
restoration, result-identity, and typed-failure coverage appropriate to their
profile.

## Native plugin qualification

A meaningful change to Rust estimator code, the ABI/C shim, native Ado
boundary, exact/compressed/generic posting, package native helpers, or native
route tests requires:

```bash
./ci/run_stata_ci.sh plugin-build
```

The source-local qualifier runs locked Rust/C gates, authenticates the Stata
SPI, builds and audits thin/universal macOS candidates, exercises native arm64
and available Rosetta paths, and performs isolated clean-install tests. See
[`../rust/stata_backend/README.md`](../rust/stata_backend/README.md) and
[`../rust/TEST_PLAN.md`](../rust/TEST_PLAN.md).

A green quick receipt is not native qualification. Require
`.ci/stata/results/<tested-sha>.json` with the exact profile and status, then
inspect the workflow's Rust/C and Stata jobs. Receipt-only `[skip ci]` commits
are bookkeeping rather than the tested source.

## Current focused regression

The active native milestone is a public Rust `algorithm(auto)` request whose
pre-RNG plan selects exact. The direct exact-V7 test lives in
`tests/stata/test_rust_planned_compressed_post.do` and prints
`EXACT_V7_RECONCILE_FAIL` plus the returned receipt list when the reconciler
fails.

Closing the milestone requires:

- direct exact-V7 receipt reconciliation without weakening any field;
- public exact-family admission/dispatch and exact poster integration;
- request/selection/result-family and execution-plan receipts;
- exact scientific and accounting identities;
- zero estimator RNG consumption and complete caller RNG restoration;
- retained `e(sample)`, data signature, sort, release, and idle registry;
- corrupt/inconsistent receipt failures; and
- passing quick, full, clean-install, and plugin-build profiles at the exact
  final source SHA.

## Hard acceptance checks

Tests must preserve:

- supported input, retained sample, target population, deletion unit, frequency
  and target semantics;
- exact and JLA agreement with independent small-design oracles where their
  contracts coincide;
- identification, canonical control-basis, deletion-rank, inverse/maker, and
  finite-output gates without hidden regularization;
- every accepted RHS's complete original worker-plus-firm or
  worker-plus-firm-plus-control residual at
  `max(1e-11,10*tolerance())`;
- `total = worker + firm + 2*covariance` and
  `corrected = plugin - correction`;
- structural pre-RNG algorithm, engine, route, batch, memory, wall, fallback,
  and Counter receipts;
- direct allocation within `memory_gib()`; and
- caller data, `e(sample)`, RNG algorithm/stream/state, and sort restoration on
  success, typed failure, and UserBreak.

Exact estimation does not require production RNG registration. JLA uses
separate runtime-scoped leverage and target domains. Pathwise draws must be
stable under row permutation, batching, route, processor count, and scheduling
within the same observed-ID design. Arbitrary ID relabeling may change a valid
draw.

## Structural routing and resources

`preconditioner(auto)` is structural. It does not run pilot RHSs or use a
projected-work withholding cutoff. Explicit CMG fails closed; automatic setup
fallback to diagonal is permitted only before estimator RNG and must be
recorded. Actual solves still must converge and pass the complete residual
gate.

`memory_gib()` is the hard direct-allocation envelope. Wall forecasts, the
registered 30-percent memory headroom, 50-percent wall allowance, timing
models, and automatic batch percentages are planning diagnostics or selection
heuristics, not scientific withholding gates.

## Optional performance and cluster evidence

Performance work must use complete-command, source-order-controlled evidence
and preserve all scientific/structural checks before interpreting time or RSS.
The retained source-bound reports are indexed in
[`docs/README.md`](docs/README.md). Do not rewrite an old report for newer
source.

When SCC evidence is useful, submit one scalar SGE job containing one Stata
process. Freeze slots, Stata processors, memory per core, wall time, fixture,
dimensions, probes, seed, batch, source hash, and input hash. Restricted data
remain under authorized storage and must never enter this repository.

Historical comparator timings are descriptive unless the estimator, sample,
weights, targets, RNG, and tolerances coincide. Preserve an unweakened failure
rather than modifying the comparator or input to obtain a number.
