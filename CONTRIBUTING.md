# Contributing to fevc

The installable Stata package is in `fevc/`; the optional native backend is in
`rust/`. Start with [AGENTS.md](AGENTS.md), the
[current checkpoint](fevc/PLAN.md), and the [documentation index](fevc/docs/README.md).

## Local setup

Use Python 3.13 and a repository-local environment:

```bash
python3.13 -m venv .venv
./.venv/bin/python -m pip install -r requirements/dev.txt
```

Run the source checks:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

When licensed Stata/MP is available locally:

```bash
./.venv/bin/python fevc/tools/run_checks.py
```

See the [test guide](fevc/TESTING.md) for selecting checks for numerical,
native, installation, and platform changes. Licensed Stata runs belong on
local or explicitly private infrastructure.

## Changes

Keep changes focused. Add an independent regression for estimator or numerical
behavior, and follow the [acceptance policy](fevc/docs/development_acceptance_v1.json).
Do not edit generated CMG output directly; use `fevc/cmg/tools/assemble.py`.
Source and provenance notices must accompany distributed binaries.

## Generated files and historical results

Write run output, logs, and receipts under ignored `.local/` or `output/`.
Keep reusable fixtures and harness source in the repository. Historical
campaign outputs and rename-era records are described in
[the archive note](docs/ARCHIVE.md); they are not required for ordinary development.

Preview disposable local artifacts with
`./.venv/bin/python fevc/tools/clean_workspace.py --dry-run` before applying
cleanup. Private data and comparator source must stay outside the public tree.
