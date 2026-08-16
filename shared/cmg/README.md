# Shared clean-room CMG-inspired core

This subtree develops the exact-hybrid, symmetric multigrid preconditioner
specified in [`cmg_plan.md`](../../cmg_plan.md). It targets only the weighted
two-way fixed-effect Schur systems used by `ppml_talo` and `kss_bc`.

The core is not a port or behavioral clone of the imported GPL CMG software.
Implementation work must not inspect that source. The hierarchy is a
clean-room CMG-inspired design based on published mathematical descriptions.

Current status: **API 5 installed KSS production candidate; SCC qualification
is active**.
The exact hybrid builder,
deterministic hierarchy, symmetric scalar/batched V-cycle, package-specific
pullback maps, automatic-route decision logic, independent dense oracle, and
deterministic assembler are implemented and pass the local Python, Mata, and
namespace gates. The generated `kssbc_cmg` runtime is shipped by the internal
`kss_bc` package and is source/build guarded by its public ado command.
`ppml_talo` integration remains outside this milestone.

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

Canonical API 5 hardens hierarchy construction for hub/star, irregular, and
expander-like graphs. The registered screened forest remains the primary
aggregation. If it misses the fixed component-surplus reduction gate, one
deterministic component-aware normalized-heavy-edge fallback is attempted,
followed by exact binary Galerkin contraction. The fallback changes no edge
weight, adds no ridge, and preserves the fixed symmetric quotient-SPD V-cycle.
The hierarchy cap is 96 levels; the edge/vertex complexity caps remain 3/4.
Attempted-level diagnostics preserve typed failures. The dense terminal cap
remains 6,144.

The API 5 canonical template and every generated namespace artifact are
source-qualified in KSS-PROD-1 tests and bound by the generated manifest.

CMG API 5 retains API 4's deterministic memory-envelope profile and bounded
row-chunked matrix action. For at least 512 planned RHSs, at least 16 GiB of
declared memory, and at most 6,144 hybrid vertices, it selects a directly factored
terminal when the predicted factor fits the registered dense-factor budget.
This bounded repeated-RHS policy avoids a hierarchy attempt that can fail its
fixed reduction gate and reduces CPU work when many RHSs reuse the factor.
Outside that policy, local calibration keeps `coarse_max=128` on small graphs
and selects 256 only for at least 2,048 vertices with at least 4 GiB declared
memory.

The unresolved gates are the PPML adapter and final KSS SCC production
qualification. API 4 has source-bound Stata 19
forced-KSS timing, RSS, estimator-equality, and complete-residual evidence on
the registered moderate and weak synthetic graphs and the bounded
MATLAB-retained real-data ladder. API 5 adds installed automatic routing and a
large deterministic hierarchy gate; CZ18 and larger end-to-end evidence remain
open until the production SCC run passes.

The exact implementation boundary and handoff are recorded in
[`docs/IMPLEMENTATION_STATUS_2026-08-15.md`](docs/IMPLEMENTATION_STATUS_2026-08-15.md).
