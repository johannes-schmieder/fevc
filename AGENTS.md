# Agent Instructions

## Repository purpose

This repository develops and audits the standalone `kss_bc` Stata/Mata
implementation and its shared CMG numerical core. It does not contain the PPML
paper, PPML estimator, or the companion KSS working paper.

## Startup

Before substantive work:

1. Run `git status --short --branch`.
2. Read `kss_bc/AGENTS.md`, `kss_bc/PLAN.md`, and the relevant handover.
3. For CMG work, also read `shared/cmg/AGENTS.md`, the active CMG plan, and
   `cmg_plan.md`.
4. Use `./.venv/bin/python` for every Python command.
5. Run the smallest relevant test before editing and the applicable integrated
   gates before completion.

Use `main` and the current worktree unless the owner explicitly requests
another branch or worktree. Preserve pre-existing changes and never rewrite
historical evidence, source-bound receipts, or review packets.

## Scientific and numerical boundaries

- Preserve the estimator, target population, deletion unit, weighting,
  nuisance, RNG, solver, convergence, residual, and failure contracts.
- Point estimates only: do not post `e(V)` or describe probe dispersion as an
  econometric standard error.
- Treat performance thresholds and extrapolations as advisory evidence, not
  scientific result-withholding gates.
- Never copy restricted row-level data or licensed comparator source into this
  repository.
- Regenerate checked-in CMG targets through `shared/cmg/tools/assemble.py`;
  never hand-edit generated CMG output.

## Validation

The minimum source gates are:

```bash
./.venv/bin/python -m pytest
./.venv/bin/python shared/cmg/tools/assemble.py --all --check
```

When Stata/MP is available, also run:

```bash
./.venv/bin/python kss_bc/tools/run_checks.py
./.venv/bin/python shared/cmg/tools/run_checks.py
```

Record exact commands, versions, seeds, tolerances, source commit, failures,
and skipped external-resource gates in completion reports.

## Licensing

Follow `CODE_LICENSE.md` and the file-level provenance records. GPL selection
does not by itself authorize public release; retain the documented human
license/provenance review gate.
