# Internal CMG component

This directory contains the GPL-3.0-only CMG numerical component used by the
`fevc` Stata/Mata package. It is not a shared library, separate Stata
package, or public command.

CMG API 9 propagates the public optional memory budget and warning/error/off
policy. With no budget, memory forecasts do not change execution; only an
explicit strict budget enables forecast rejection. The numerical hierarchy,
symmetric V-cycle, deterministic aggregation, terminal cap and complete
residual gates remain unchanged from the qualified API 6 core. API 8 recorded
the `fevc` generated-target identity; API 7 removed the unused non-KSS pullback
and established the package-owned runtime identity. See the
[current memory guide](../docs/MEMORY.md) for timing, scope and returns.

The canonical template is `src/cmg_core.mata.in`. The deterministic generator
has exactly two targets:

- `../fevc_cmg.mata`: shipped namespace `vckss_cmg`, `matalnum off`;
- `generated/cmg_test.mata`: test namespace `cmgtest`, `matalnum on`.

`generated/manifest.json` binds generator API 5, target paths, namespaces,
numeric modes, and SHA-256 hashes. Generated Mata files must never be edited
directly.

Run the component gates from the repository root:

```bash
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
./.venv/bin/python -m pytest fevc/cmg/tests -q
./.venv/bin/python fevc/cmg/tools/run_checks.py
```

The source-informed API-6 qualification remains recorded verbatim in
`benchmarks/reports/CMG_MATA_1_2026-08-18.md`. API 9 does not turn that historical report into new
numerical qualification. Current ownership and release status are in
`STATUS.md`; mathematical, API, and provenance boundaries are in `docs/`.

Public distribution remains disabled until the documented human
license/provenance review of the exact package and corresponding-source
boundary is complete.
