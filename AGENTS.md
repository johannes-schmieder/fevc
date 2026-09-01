# Agent instructions

## Scope

This repository develops and audits the standalone `fevc` Stata/Mata
package, its optional Rust backend, and its package-owned CMG component. The
companion paper is maintained separately.

The current objective and checkpoint live in `fevc/PLAN.md`. Candidate
promotion follows the registered
`fevc/docs/development_acceptance_v1.json` policy. Detailed scientific
contracts and accepted decisions live under `fevc/docs/`; do not duplicate
them here.

## Startup

Before substantive work:

1. Run `git status --short --branch` and preserve existing changes.
2. Read `fevc/AGENTS.md`, `fevc/PLAN.md`, and
   `fevc/docs/README.md`.
3. For Rust work, read `rust/README.md` and `rust/TEST_PLAN.md`.
4. For CMG work, also read `fevc/cmg/AGENTS.md` and
   `fevc/cmg/STATUS.md`.
5. Use `./.venv/bin/python` for Python commands.

## Repository-wide guardrails

- Work on `main` and the current worktree unless the owner directs otherwise.
  Do not create a branch/worktree, rewrite history, or discard unrelated work.
- Treat accepted exact-SHA receipts, reviews, source manifests, archived
  reports, and benchmark evidence as immutable. Transient development smokes
  are diagnostic runs: preserve their useful logs, but do not promote every
  failed attempt into publication-grade evidence or a compatibility lineage.
- Do not copy restricted row-level data or licensed comparator source into the
  repository.
- Regenerate CMG targets only through `fevc/cmg/tools/assemble.py`; never
  hand-edit generated output.
- Keep trusted-patch files under `.ci/codex/` single-use. Remove transport
  files after a successful application or completed handoff.
- Do not push, publish, tag, stage a release, or mutate external state without
  explicit authorization.

## Engineering style

- Make the smallest coherent change that preserves the registered contracts.
- Prefer existing representations and direct control flow over speculative
  abstraction. Generalize only to remove demonstrated duplication or enforce
  a tested invariant.
- Protect performance-sensitive paths from aesthetic rewrites. Measure changes
  to algorithms, allocation, data layout, parallelism, or routing.
- Add a focused regression for behavioral or numerical changes. Keep dense,
  brute-force, and comparator oracles independent of production code.
- Comments should explain scientific invariants, numerical safeguards,
  ownership, licensing, platform constraints, or non-obvious performance
  choices. Remove comments that merely narrate code or preserve obsolete plans.
- Avoid unrelated cleanup, especially in a dirty worktree.

## Evidence and validation

Select gates by affected behavior. A new SHA alone does not justify a large SCC
array, broad platform matrix, full native qualification, or other expensive
run. Escalate only when the change affects that surface, focused checks cannot
bound the risk, an active benchmark or release decision needs the evidence, or
the owner requests it.

For SCC launcher, environment, or wrapper work, start with one 1--4 core
end-to-end smoke using the real entrypoint and a small input. Use scheduler
dependencies for an approved multi-stage campaign; monitoring must not become
the stage controller. After one narrow implementation fix and remote retest,
stop and report another operational failure rather than continuing an
unbounded repair loop.

Evidence may be carried to a later source only through a recorded compatibility
review identifying both sources, changed paths, affected surface, unchanged
production/build/binary/input/acceptance identities, checks run, claims reused,
and limitations. Quick Stata CI does not qualify the Rust plugin.

Minimum source gates:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

When Stata/MP is available, use
`./.venv/bin/python fevc/tools/run_checks.py`. Record exact commands,
versions, seeds, tolerances, failures, and skipped external gates.

## Licensing

Follow `CODE_LICENSE.md` and the source-provenance records. GPL-3.0-only
governs covered code. Public tagging and release remain separate owner
decisions and require the recorded human and exact-artifact checks.
