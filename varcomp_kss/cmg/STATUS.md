# CMG component status

- Ownership: internal component of `varcomp_kss`
- Runtime namespace: `vckss_cmg`
- Component API: 7
- Generator API: 4
- Numerical basis: API-6-qualified source-informed GPL Mata core
- Production/public release: disabled

API 7 changes package ownership and removes the unused non-KSS callable
surface. It does not alter hierarchy construction, V-cycle algebra, routing,
resource forecasts, numerical tolerances, or estimator acceptance gates.

The complete API-6 local and SCC qualification report remains byte-identical
at `benchmarks/reports/CMG_MATA_1_2026-08-18.md`. Public release still requires
human mathematical and GPL/provenance review of the exact distribution.
