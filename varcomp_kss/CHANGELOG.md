# Changelog

## 0.3.0-dev — 2026-08-20

- Rework the help file in the `cellgraph` SMCL style and add three
  deterministic, self-contained AKM examples that execute from the Stata help
  browser through the installed `varcomp_kss_run` helper while preserving the
  caller's data.
- Add an applied default display and `e(decomposition)`, separating raw
  covariance from the additive `2 x covariance` sorting contribution and
  reporting plug-in and corrected shares of target-weighted outcome variance
  and worker--firm totals.
- Add retained target- and frequency-weighted outcome variances, residual
  variance, and the descriptive full-model explained variance and share. The
  full-model fit includes controls and remains distinct from the KSS-corrected
  worker--firm target.
- Standardize recognized failure output with a plain-language reason,
  actionable remedy, technical status, troubleshooting link, and the new
  `e(withholding_detail)`, `e(withholding_reason)`, and
  `e(withholding_suggestion)` metadata.
- Extend source, clean-install, package-layout, output-identity, failure, and
  executable-help tests without changing the estimator or established result
  matrices.

- Rename the public Stata command, package, help topic, and shipped files to
  `varcomp_kss`; do not install a predecessor-command alias.
- Rename active private Ado/Mata namespaces and runtime build identifiers to
  the `vckss` family while preserving estimator and numerical contracts.
- Internalize CMG under `varcomp_kss/cmg`, expose only the package and test
  generator targets, and remove the unused non-KSS pullback.
- Record CMG API 7 and generator API 4 as ownership/interface boundaries over
  the numerically qualified API 6 implementation.

The byte-exact predecessor changelog is preserved under `docs/history/`.
