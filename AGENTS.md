# Agent instructions

## Repository purpose

This repository develops and audits the standalone `vckss` Stata/Mata
package, its optional Rust plugin backend, and its package-owned CMG numerical
component. The companion paper is maintained separately.

## Startup

Before substantive work:

1. Run `git status --short --branch` and preserve existing changes.
2. Read `vckss/AGENTS.md` and `vckss/PLAN.md`.
3. Read `vckss/docs/README.md` for the contract/evidence map.
4. For Rust work, read `rust/README.md` and `rust/TEST_PLAN.md`.
5. For CMG work, also read `vckss/cmg/AGENTS.md` and
   `vckss/cmg/STATUS.md`.
6. Use `./.venv/bin/python` for Python commands.

Use `main` and the current worktree unless the owner explicitly requests
otherwise. Do not create a branch/worktree, rewrite history, or overwrite
source-bound receipts and archived evidence.

## Scientific and numerical boundaries

- Preserve estimator, population, sample, deletion, weighting, nuisance, RNG,
  solver, routing, convergence, residual, resource, return, and caller-state
  contracts.
- Point estimates only: never post `e(V)` or call probe dispersion an
  econometric standard error.
- Performance forecasts, headroom, and timing targets are advisory; direct
  allocation and scientific/numerical gates are hard.
- Never copy restricted row-level data or licensed comparator source into the
  repository.
- Regenerate CMG targets through `vckss/cmg/tools/assemble.py`; do not
  hand-edit generated output.

## Development discipline

Add a focused regression for behavioral or numerical changes and keep
independent oracles independent of production code. Run the smallest relevant
gate while iterating, then the applicable integrated, clean-install, and native
qualification gates before closing.

Bind every qualification claim to an exact source SHA. A green Stata quick
receipt does not by itself qualify the Rust plugin; inspect the Rust/C jobs and
run the source-local plugin profile when the native boundary changes.

Trusted-patch files under `.ci/codex/` are single-use transport. Remove the
apply script/patch, commit message, and failed-apply receipt once the intended
change is already present or the handoff is complete.

## Minimum source gates

```bash
./.venv/bin/python -m pytest
./.venv/bin/python vckss/cmg/tools/assemble.py --all --check
```

When Stata/MP is available:

```bash
./.venv/bin/python vckss/tools/run_checks.py
```

Record exact commands, source SHA, versions, seeds, tolerances, failures, and
skipped external gates.

## Licensing

Follow `CODE_LICENSE.md` and the source-provenance records. GPL-3.0-only
selection does not itself authorize public release; retain the documented human
mathematical and license/provenance review gates.
