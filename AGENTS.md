# Agent instructions

## Scope

This repository develops and audits the standalone `fevc` Stata/Mata
package, its optional Rust backend, and its package-owned CMG component. The
companion paper and paper-specific evidence are maintained separately. The
repository is public-source prerelease infrastructure; a package tag, release
archive, and native binary distribution are separate owner decisions.

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
- Do not commit patch transport, temporary handoff, editor-session, or
  machine-local runner state.
- Keep licensed Stata execution local or in explicitly private infrastructure;
  public GitHub workflows must not target self-hosted licensed runners.
- Before changing repository visibility, scan the complete reachable history,
  references, and current tree for secrets and private paper artifacts.
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

For Monte Carlo, coverage, or qualification campaigns, treat the harness as
production code rather than disposable scaffolding:

- Before an SCC array, run the complete generator-to-validator-to-receipt path
  with a tiny local profile and then one representative compute-node task
  through the real launcher. Check success and deliberate failure exits,
  schema, expected row and cell counts, unique task keys, profile, seed,
  dimensions, replication counts, output paths, and receipt contents. Unit-test
  malformed, missing, duplicate, partial, and scientifically failing inputs.
- Freeze a machine-readable task manifest before a confirmation run. Bind it to
  the exact source commit or immutable source bundle, input identities, DGPs,
  estimands, target exclusions, dimensions, repetitions, semantic RNG keys,
  acceptance thresholds, and expected output inventory. Development runs may
  tune the harness or estimator; confirmation runs may not tune either.
- Verify cheaply that every fixture realizes its intended regime before doing
  repeated outcome draws. For component inference this includes diffuse
  `q=0`, one dominant mode with a diffuse `q=1` remainder, deliberately
  multi-mode cases, null/weak-signal cases, and correct, mild, and severe
  variance-model misspecification. A fixture name is not evidence; record the
  realized spectral, support, positivity, and identification diagnostics.
- Make array tasks order- and schedule-invariant. Address Counter RNG draws by
  canonical semantic cell and replication keys, keep outcome-free cross-fit
  folds fixed across replications, and test that design-equivalent rows remain
  in one fold. Give every task a unique output directory; never append from
  multiple tasks to one CSV. Write task results atomically and aggregate only
  after complete scheduler, log, schema, and inventory validation.
- Count and classify every attempted replication. Do not silently condition
  coverage on successful fits without also enforcing and reporting an atomic
  success-rate gate. Preserve raw per-cell output and have the final audit
  enumerate all failed cells; a fail-fast receipt identifies only the first
  failure and is not a complete failure summary.
- Keep development and confirmation evidence separate. A predeclared
  confirmation failure remains a failure even when it narrowly misses a gate;
  do not weaken a cutoff, exclude a target, or relabel a DGP after inspecting
  results. Diagnose numerical, harness, and scientific failures separately,
  change code or contract only with justification, and use a newly registered
  campaign for the next confirmation attempt.

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
