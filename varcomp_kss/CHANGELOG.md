# Changelog

## 0.3.0-dev — 2026-08-18

- Rename the public Stata command, package, help topic, and shipped files to
  `varcomp_kss`; do not install a predecessor-command alias.
- Rename active private Ado/Mata namespaces and runtime build identifiers to
  the `vckss` family while preserving estimator and numerical contracts.
- Internalize CMG under `varcomp_kss/cmg`, expose only the package and test
  generator targets, and remove the unused non-KSS pullback.
- Record CMG API 7 and generator API 4 as ownership/interface boundaries over
  the numerically qualified API 6 implementation.

The byte-exact predecessor changelog is preserved under `docs/history/`.
