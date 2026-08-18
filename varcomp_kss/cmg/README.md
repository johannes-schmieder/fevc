# Internal CMG component

This directory contains the GPL-3.0-only CMG numerical component used by the
`varcomp_kss` Stata/Mata package. It is not a shared library, separate Stata
package, or public command.

CMG API 7 is an ownership and interface-only successor to the numerically
qualified API 6 implementation. The numerical hierarchy, symmetric V-cycle,
resource model, deterministic aggregation, terminal cap, and package residual
gates are unchanged. API 7 removes the unused non-KSS pullback and gives the
component a package-owned runtime identity.

The canonical template is `src/cmg_core.mata.in`. The deterministic generator
has exactly two targets:

- `../varcomp_kss_cmg.mata`: shipped namespace `vckss_cmg`, `matalnum off`;
- `generated/cmg_test.mata`: test namespace `cmgtest`, `matalnum on`.

`generated/manifest.json` binds generator API 4, target paths, namespaces,
numeric modes, and SHA-256 hashes. Generated Mata files must never be edited
directly.

Run the component gates from the repository root:

```bash
./.venv/bin/python varcomp_kss/cmg/tools/assemble.py --all --check
./.venv/bin/python -m pytest varcomp_kss/cmg/tests -q
./.venv/bin/python varcomp_kss/cmg/tools/run_checks.py
```

The source-informed API-6 qualification remains recorded verbatim in
`benchmarks/reports/CMG_MATA_1_2026-08-18.md`. API 7 does not claim new
numerical qualification. Current ownership and release status are in
`STATUS.md`; mathematical, API, and provenance boundaries are in `docs/`.

Public distribution remains disabled until the documented human
license/provenance review of the exact package and corresponding-source
boundary is complete.
