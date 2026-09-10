# `fevc`

This repository develops and audits the standalone `fevc` Stata/Mata
implementation of Kline--Saggio--Sølvsten leave-out bias-corrected point
estimates for linear two-way fixed-effect variance decompositions. The
`0.5.0-rc.1` source also provides opt-in econometric inference and
fixed-effect projection inference, including explicit fixed-offset match
q0/q1 component inference. An optional Rust estimation and inference backend
and the package-owned CMG numerical component are included.

The source repository is prepared for public development. The package remains
a prerelease: no public tag, release archive, or native binary distribution has
been issued.

The current source-local Mac candidate uses one residual-moment variance
fitter for observation and fixed-offset match deletion, with separate
individual-interval/joint-covariance reporting, 200 default JLA probes and
2,048 direct residual Gram probes. `inferencegramprobes()` controls Gram
precision separately; approximate model-based inference retains calibration
and specification caveats.
See the [candidate contract](fevc/docs/INDIVIDUAL_INFERENCE_INTERFACE.md).
The [completion report](fevc/docs/INFERENCE_COMPLETION_2026-09-08.md)
records the passing local Mac, replay and paper checks. Historical scientific
passes remain source-specific; other platforms and public distribution remain
separate.

The companion working paper is maintained separately in the sibling
`fevc-paper` repository. Its active repository and PDF filenames use the
`fevc` identity; frozen predecessor and migration records retain old names.

Comparator naming and the two code versions are documented in the [KSS Matlab source and version note](fevc/docs/SOURCE_PROVENANCE.md#kss-matlab-package-terminology-and-version).

## Repository map

- [`fevc/`](fevc/README.md): installable command, Mata runtime,
  Rust-facing Ado boundary, help, tests, benchmarks, and active package plan.
- [`fevc/docs/`](fevc/docs/README.md): documentation index for
  scientific contracts, numerical architecture, decisions, provenance, and
  retained engineering results.
- [`fevc/cmg/`](fevc/cmg/README.md): internal GPL-3.0-only CMG
  component, deterministic generator, tests, and provenance.
- [`rust/`](rust/README.md): optional native backend source, active native test
  plan, dated progress snapshots, and Stata plugin boundary.
- [`qualification/`](qualification/) and [`reviews/`](reviews/): source-bound
  evidence and independent reviews.
- [`docs/history/`](docs/history/) and [`docs/migration/`](docs/migration/):
  immutable predecessor and migration records.

The current development milestone and exact handoff state are recorded in
[`fevc/PLAN.md`](fevc/PLAN.md). Historical reports and exact-SHA
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
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

Licensed Stata is intentionally local-only. When Stata/MP is available on an
authorized machine, run:

```bash
./.venv/bin/python fevc/tools/run_checks.py
```

See [`fevc/TESTING.md`](fevc/TESTING.md) for the full test and
qualification taxonomy.

## Install from a checkout

In Stata, point `net install` at the package directory in this checkout:

```stata
net install fevc, from("/absolute/path/to/this/repository/fevc") replace
```

This checkout manifest is portable source only. The requested RC payload
will include every Mac, Linux and Windows plugin; its source-bound preparation
and private testing are described in [RC binary preparation](fevc/docs/RC_BINARY_PAYLOAD.md).
Rust-only fixed-offset match inference requires the native payload or a
qualified local build.

See [`fevc/README.md`](fevc/README.md) for command examples, supported
capabilities, and optional native-backend details.

## License and distribution status

The CMG implementation and a distributed KSS package containing it are
GPL-3.0-only as recorded in [`CODE_LICENSE.md`](CODE_LICENSE.md). The human
review of the package boundary, upstream notices, corresponding source, and
third-party/data exclusions was completed on 29 August 2026. The repository
is ready for public source development, but no public package release, tag, or
native binary distribution has yet been issued.
