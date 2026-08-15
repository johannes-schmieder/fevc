# Shared clean-room CMG-inspired core

This subtree develops the exact-hybrid, symmetric multigrid preconditioner
specified in [`cmg_plan.md`](../../cmg_plan.md). It targets only the weighted
two-way fixed-effect Schur systems used by `ppml_talo` and `kss_bc`.

The core is not a port or behavioral clone of the imported GPL CMG software.
Implementation work must not inspect that source. The hierarchy is a
clean-room CMG-inspired design based on published mathematical descriptions.

Current status: **standalone local candidate with a forced KSS test adapter**.
The exact hybrid builder,
deterministic hierarchy, symmetric scalar/batched V-cycle, package-specific
pullback maps, automatic-route decision logic, independent dense oracle, and
deterministic assembler are implemented and pass the local Python, Mata, and
namespace gates. Generated `ppml_talo` and `kss_bc` artifacts exist only in
this subtree. The KSS adapter lives under `kss_bc/tests/support/`, is loaded
only by tests and benchmarks, and is not an installed estimator path or public
runtime option.

Layout:

- `src/`: canonical namespace-tokenized Mata source;
- `oracle/`: independent Python dense/reference algebra;
- `tests/`: Python and Stata/Mata tests;
- `benchmarks/`: kernel and end-to-end benchmark drivers and reports;
- `tools/`: deterministic assembly and evidence validation;
- `docs/`: mathematical, numerical, provenance, and API contracts.

The selected Stata 18 application path is `@CMG_NS@__apply()`. A bounded
reusable-workspace implementation remains available for regression and future
platform testing, but local 10,000- and 100,000-vertex benchmarks found it
about 63--64% slower and it is not selected for package integration. See the
[workspace and solver report](benchmarks/reports/CMG6_WORKSPACE_AND_SOLVER_2026-08-15.md).

Run all standalone gates with:

```bash
./.venv/bin/python shared/cmg/tools/run_checks.py
```

CMG API 2 adds a deterministic memory-envelope profile and removes the fixed
eight-column graph-action cap while retaining bounded row chunks. Local
calibration keeps `coarse_max=128` on small graphs and selects 256 only for at
least 2,048 vertices with at least 4 GiB declared memory.

The unresolved gates are the PPML adapter (CMG7), end-to-end forced KSS
estimator and comparative RSS evidence, Stata 19/SCC qualification (CMG10),
external model review, and package-specific promotion (CMG11). KSS automatic
routing remains disabled.

The exact implementation boundary and handoff are recorded in
[`docs/IMPLEMENTATION_STATUS_2026-08-15.md`](docs/IMPLEMENTATION_STATUS_2026-08-15.md).
