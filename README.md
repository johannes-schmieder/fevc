# `varcomp_kss`

This repository contains the standalone Stata/Mata implementation of
Kline--Saggio--Sølvsten leave-out bias-corrected point estimates for linear
two-way fixed-effect variance decompositions.

The repository preserves the development layout used by the implementation:

- `kss_bc/`: installable Stata command, Mata runtime, tests, benchmarks, and
  implementation documentation;
- `shared/cmg/`: canonical GPL-3.0-only CMG source, generator, generated
  targets, tests, and provenance records; and
- root planning, handover, licensing, and historical KSS/CMG review records.

The companion working paper is maintained separately in the sibling
`varcomp_kss_paper` repository.

## Local setup

Use Python 3.13 and create a repository-local environment:

```bash
python3.13 -m venv .venv
./.venv/bin/python -m pip install --upgrade pip
./.venv/bin/python -m pip install -r requirements/dev.txt
```

Run the deterministic Python and generated-source gates with:

```bash
./.venv/bin/python -m pytest
./.venv/bin/python shared/cmg/tools/assemble.py --all --check
```

The retired `kss_bc/benchmarks/matlab_scale/tests/` suite is preserved as
historical harness material but is not part of the current diagnostic bundle
or default gate.

When Stata/MP is available, the integrated local gates are:

```bash
./.venv/bin/python kss_bc/tools/run_checks.py
./.venv/bin/python shared/cmg/tools/run_checks.py
```

See `kss_bc/README.md`, `kss_bc/TESTING.md`, and `shared/cmg/README.md` for
the command, package, numerical, and SCC workflows.

## License and distribution status

The CMG implementation and a distributed KSS package containing it are
licensed GPL-3.0-only as recorded in `CODE_LICENSE.md`. Public distribution
still requires the documented human review of the exact package boundary,
upstream notices, corresponding-source bundle, and third-party/data
exclusions.
