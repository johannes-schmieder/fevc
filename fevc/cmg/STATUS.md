# CMG component status

- Ownership: internal component of `fevc`
- Runtime namespace: `vckss_cmg`
- Component API: 8
- Generator API: 5
- Numerical basis: API-6-qualified source-informed GPL Mata core
- Standalone release: disabled; source ships only inside the FEVC prerelease

API 8 records the package and generated-target rename to `fevc`; generator
API 5 emits only the renamed target identity. Neither change
alters hierarchy construction, V-cycle algebra, routing, resource forecasts,
numerical tolerances, or estimator acceptance gates.

The complete API-6 local and SCC qualification report remains byte-identical
at `benchmarks/reports/CMG_MATA_1_2026-08-18.md`. A standalone CMG package is
not planned; any FEVC release still requires review of the exact distribution.
