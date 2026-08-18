# Shared CMG core

This subtree develops the exact-hybrid, symmetric multigrid preconditioner
specified in [`cmg_plan.md`](../../cmg_plan.md). It targets only the weighted
two-way fixed-effect Schur systems used by `ppml_talo` and `kss_bc`.

API 1--5 is a clean-room CMG-inspired Mata design based on published
mathematics. API 6, developed under `CMG-MATA-1`, is a source-informed port of
the official CMG hierarchy architecture and is GPL-3.0-only. Runtime code is
Mata only: there is no C plugin, MEX file, executable, subprocess, or binary
interchange. API 5 remains the internal numerical and rollback reference. See
[`plans/CMG_MATA_1.md`](plans/CMG_MATA_1.md) and
[`docs/SOURCE_PROVENANCE.md`](docs/SOURCE_PROVENANCE.md).

Current status: **CMG-MATA-1 API 6 has completed local and SCC qualification
as an internal experimental KSS candidate; production and public release are
disabled**. See the
[`CMG-MATA-1 qualification report`](benchmarks/reports/CMG_MATA_1_2026-08-18.md).
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

Canonical API 6 uses a bulk-Mata maximum-edge profile, bounded forest splitting,
the official one-eighth weak-branch repair, and dense component packing on
hybrid and sparse quotient levels. Dense ordinary quotients use the retained
API 5 screened forest, which preserves degree-two and degree-three performance.
If a primary proposal misses the component-surplus reduction gate, one
deterministic normalized-heavy-edge fallback is attempted, followed by exact
binary Galerkin contraction. These paths change no edge weight, add no ridge,
and preserve the fixed symmetric quotient-SPD V-cycle. The hierarchy cap is
96 levels; the default edge/vertex complexity caps are 3/5.
Attempted-level diagnostics preserve typed failures. The dense terminal cap
remains 6,144.

The API 6 canonical template and every generated namespace artifact are
source-qualified and bound by the generated manifest. Generator API 3 records
the GPL/source-informed boundary and a target-specific numeric mode. The KSS
targets use `matalnum off`; PPML and the standalone test target retain
`matalnum on`. Generated runtimes expose `numeric_mode()` so package loaders
can bind that setting without changing CMG algebra.

CMG API 6 retains API 5's deterministic memory-envelope profile and bounded
row-chunked matrix action. For at least 512 planned RHSs, at least 16 GiB of
declared memory, and at most 6,144 hybrid vertices, it selects a directly factored
terminal when the predicted factor fits the registered dense-factor budget.
This bounded repeated-RHS policy avoids a hierarchy attempt that can fail its
fixed reduction gate and reduces CPU work when many RHSs reuse the factor.
Outside that policy, local calibration keeps `coarse_max=128` on small graphs
and selects 256 only for at least 2,048 vertices with at least 4 GiB declared
memory.

Local degree-matrix tests cover degrees 2--7, exact residual checks, the
installed KSS namespace, and an API-5 A/B path. On the 40,960-worker local
fixture, degree-four hierarchy setup is about 5.9 times faster than API 5;
degree-two and degree-three setup is unchanged within timing noise. These are
development measurements, not SCC qualification evidence.

The unresolved gates are a direct frozen-API-5/API-6 low-degree
complete-command regression comparison, the PPML adapter, named human
mathematical review, and human GPL/provenance review. The completed SCC
campaign validates 120 matched Stata/MATLAB P20 tasks, degree-two-through-seven
hierarchy and adversarial matrices, a 601-RHS P200 CMG block, and fixed CZ18
Stata/MATLAB runs. It does not authorize production or public distribution.

Historical API 4 evidence has source-bound Stata 19
forced-KSS timing, RSS, estimator-equality, and complete-residual evidence on
the registered moderate and weak synthetic graphs and the bounded
MATLAB-retained real-data ladder. API 5 adds installed automatic routing and a
large deterministic hierarchy gate. Candidate `5e2687c6` passes the full
601-RHS CZ18 estimator, but all three registered two-times-CZ18 P20 jobs fail
the same typed pre-RNG automatic-route gate. The production validator therefore
withheld admission and no full stress job ran. See the
[KSS-PROD-1 report](../../kss_bc/benchmarks/reports/KSS_PROD_1_2026-08-16.md).

The exact implementation boundary and handoff are recorded in
[`docs/IMPLEMENTATION_STATUS_2026-08-15.md`](docs/IMPLEMENTATION_STATUS_2026-08-15.md).
