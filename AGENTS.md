# Agent instructions

## Repository purpose

This repository develops and audits the standalone `vckss` Stata/Mata
package, its optional Rust plugin backend, and its package-owned CMG numerical
component. The companion paper is maintained separately.

The owner priority is a fast Stata alternative to maintained MATLAB KSS that
returns the same statistical result on compatible problems. Development is
performance-first: corrected point-estimate equivalence and end-to-end MATLAB
competitiveness take precedence over pathwise floating-point identity,
backend-internal parity, and documentary completeness. The registered policy
is `vckss/docs/development_acceptance_v1.json`.

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

- Preserve the estimator, population, sample, deletion, weighting, nuisance,
  and corrected-target meanings. Solvers, reduction order, routing internals,
  iteration counts, and numerical representations may change to improve speed
  when the registered statistical-equivalence and hard-correctness gates pass.
- Point estimates only: never post `e(V)` or call probe dispersion an
  econometric standard error.
- Do not block a candidate on bitwise/ULP identity, a legacy fixed roundoff
  gate, equal iteration counts, or harmless plug-in/correction decomposition
  drift when the four corrected targets pass the registered development
  equivalence rule. Retain those comparisons as diagnostics.
- Complete original-system residuals, finite outputs, identification,
  accounting, direct allocation, typed failure/UserBreak, no hidden estimator
  change or post-RNG fallback, and caller-state/lifecycle restoration remain
  hard correctness gates.
- Performance forecasts and headroom remain advisory for runtime withholding,
  but measured MATLAB-relative complete-command performance is a primary
  candidate-promotion gate.
- There is no repository-wide development RAM ceiling. Historical benchmark
  envelopes remain facts about those frozen runs, not limits on new prototypes.
  New SCC work should request enough scheduler memory for the problem and
  retain honest forecast, admission, RSS, and accounting receipts.
- Early comparative benchmarks should favor scheduler availability over a
  fixed queue, CPU model, or exclusive node. Run the compared implementations
  sequentially in one task on one host, rotate their order, record the exact
  host/CPU/affinity, and make within-task paired ratios primary. Reserve
  homogeneous or exclusive hardware for a smaller confirmatory benchmark when
  absolute timings or cross-core scaling require it.
- Never copy restricted row-level data or licensed comparator source into the
  repository.
- Regenerate CMG targets through `vckss/cmg/tools/assemble.py`; do not
  hand-edit generated output.

## Development discipline

Add a focused regression for behavioral or numerical changes and keep
independent oracles independent of production code. Select gates from the
actual impact of the change: run the smallest relevant gate while iterating,
then only the integrated, clean-install, native, or platform gates that the
changed behavior can affect.

Do not rerun a large SCC array, broad cross-platform matrix, full native
qualification, or other expensive gate merely because the repository SHA
changed. Such work requires a compelling impact-based reason: the change can
affect the measured or qualified behavior, a benchmark is needed to answer the
active performance question, a release gate expressly requires it, risk cannot
be bounded with focused checks, or the owner specifically requests it. Prefer
focused tests, static checks, retained-receipt revalidation, or a small pilot
when they resolve the risk.

Qualification claims must identify the exact tested source. They may be
carried forward to a later source when a recorded compatibility review shows
that the relevant production source, build inputs, binaries, scientific input,
and acceptance semantics are unchanged. Record the tested source, current
source, changed paths, affected surface, checks run, claims reused, and any
remaining limitation. A documentation-, test-, CI-, packaging-, provenance-,
or evidence-workflow-only change does not invalidate unrelated estimator or
performance evidence. A green Stata quick receipt does not by itself qualify
the Rust plugin; inspect the Rust/C jobs and run the source-local plugin profile
when the native boundary changes.

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
