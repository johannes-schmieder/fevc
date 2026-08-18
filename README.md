# `varcomp_kss`

This repository contains the standalone Stata/Mata implementation of
Kline--Saggio--Sølvsten leave-out bias-corrected point estimates for linear
two-way fixed-effect variance decompositions.

The repository has one package-owned implementation layout:

- `varcomp_kss/`: installable Stata command, Mata runtime, tests, benchmarks, and
  implementation documentation;
- `varcomp_kss/cmg/`: the internal GPL-3.0-only CMG component, including its
  canonical source, deterministic generator, tests, and provenance; and
- `docs/history/` and `reviews/`: immutable predecessor plans and evidence.

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
./.venv/bin/python varcomp_kss/cmg/tools/assemble.py --all --check
```

The default Python gate includes the retained MATLAB-harness tests as well as
the package and CMG-component tests.

When Stata/MP is available, the integrated local gates are:

```bash
./.venv/bin/python varcomp_kss/tools/run_checks.py
```

See `varcomp_kss/README.md`, `varcomp_kss/TESTING.md`, and
`varcomp_kss/cmg/README.md` for the command, package, numerical, and SCC
workflows.

## License and distribution status

The CMG implementation and a distributed KSS package containing it are
licensed GPL-3.0-only as recorded in `CODE_LICENSE.md`. Public distribution
still requires the documented human review of the exact package boundary,
upstream notices, corresponding-source bundle, and third-party/data
exclusions.
