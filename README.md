# `varcomp_kss`

This repository develops and audits the standalone `varcomp_kss` Stata/Mata
implementation of Kline--Saggio--Sølvsten leave-out bias-corrected point
estimates for linear two-way fixed-effect variance decompositions. It also
contains an optional, explicitly selected Rust plugin backend and the
package-owned CMG numerical component.

The companion working paper is maintained separately in the sibling
`varcomp_kss_paper` repository.

## Repository map

- [`varcomp_kss/`](varcomp_kss/README.md): installable command, Mata runtime,
  Rust-facing Ado boundary, help, tests, benchmarks, and active package plan.
- [`varcomp_kss/docs/`](varcomp_kss/docs/README.md): documentation index for
  scientific contracts, numerical architecture, decisions, provenance, and
  retained engineering results.
- [`varcomp_kss/cmg/`](varcomp_kss/cmg/README.md): internal GPL-3.0-only CMG
  component, deterministic generator, tests, and provenance.
- [`rust/`](rust/README.md): optional native backend source, active native test
  plan, dated progress snapshots, and Stata plugin boundary.
- [`qualification/`](qualification/) and [`reviews/`](reviews/): source-bound
  evidence and independent reviews.
- [`docs/history/`](docs/history/) and [`docs/migration/`](docs/migration/):
  immutable predecessor and migration records.
- [`STATA_CI_RUNNER.md`](STATA_CI_RUNNER.md): private licensed-runner operation
  and receipt semantics.

The current development milestone and exact handoff state are recorded in
[`varcomp_kss/PLAN.md`](varcomp_kss/PLAN.md). Historical reports and exact-SHA
receipts are evidence, not instructions for the next change.

## Local setup

Use Python 3.13 and the repository-local environment:

```bash
python3.13 -m venv .venv
./.venv/bin/python -m pip install --upgrade pip
./.venv/bin/python -m pip install -r requirements/dev.txt
```

Run the deterministic source gates with:

```bash
./.venv/bin/python -m pytest
./.venv/bin/python varcomp_kss/cmg/tools/assemble.py --all --check
```

When Stata/MP is available, run:

```bash
./.venv/bin/python varcomp_kss/tools/run_checks.py
```

See [`varcomp_kss/TESTING.md`](varcomp_kss/TESTING.md) for the full test and
qualification taxonomy.

## License and distribution status

The CMG implementation and a distributed KSS package containing it are
GPL-3.0-only as recorded in [`CODE_LICENSE.md`](CODE_LICENSE.md). Public
distribution remains disabled until the documented human review of the exact
package boundary, upstream notices, corresponding source, and third-party/data
exclusions is complete.
