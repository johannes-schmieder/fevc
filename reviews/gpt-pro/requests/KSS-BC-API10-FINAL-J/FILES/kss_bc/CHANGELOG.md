# Changelog

## 0.1.0-dev — unreleased

- Authorized sibling-package development on `main`.
- Locked the KSS point-estimation, mover-headline, source, license, and
  improved-JLA coefficient contracts.
- Added exact dense observation and general match-block corrections with
  literal frequency semantics and independent Python oracles.
- Added the worker-eliminated matrix-free solver, joint-control FWL system,
  improved JLA, batched probe stream, target contractions, and conditional
  numerical MCSE.
- Added MATLAB-compatible iterative articulation-worker pruning, typed
  withholding, public return metadata, Stata 18 suites, and an isolated
  install smoke test.
- Added component timing, a production-shaped synthetic benchmark, a
  clean-room MATLAB/Stata paired oracle, and source-bound BU SCC submission
  and three-layer evidence validators.
- An interim API-level-7 implementation pooled exchangeable copy residual
  squares before the nonlinear ratio. Adversarial review showed that this did
  not reproduce literal expanded observation JLA at finite probe counts.
- Added a deterministic within-cell trace certificate so randomized
  joint-control calculations cannot accept a deletion-induced rank loss from
  a favorable low-probe draw.
- Raised the internal Mata API level to 8: batched joint solves now gate the
  worst relative residual column instead of one aggregate Frobenius residual,
  and the deletion-rank certificate subtracts its measured whitening error
  plus a rounding margin from the reported lower bound. The certificate also
  uses explicit two-pass cell centering and nonnegative scatter-loss formulas
  to avoid cancellation under large cell means or control reparameterization.
  Every low-dimensional deleted whitened scatter is directly eigendecomposed
  and inverse-residual checked in addition to the conservative trace screen.
- Added a dense-backend forward-error proxy and direct deleted-information
  factorization near Woodbury rank boundaries. A registered ill-conditioned
  two-control design that previously returned a point estimate despite exact
  deletion rank loss is now withheld by both backends.
- Included every JLA match-block inverse residual in the posted maximum
  numerical residual diagnostic.
- Raised the internal Mata API level to 9. Observation JLA now retains the
  two sufficient cross-probe correlations for each physical copy, performs
  every nonlinear finite-projection adjustment copywise, and averages only
  final inverse multipliers within a stored row. Weighted and literal-expanded
  executions now share the same canonical copy stream and agree at finite
  probe counts. Connected-component ties on firm count and physical mass now
  withhold as `AMBIGUOUS_LARGEST_COMPONENT` instead of selecting by encoded
  identifiers.
- Bound the ado caller to an exact Mata semantic build token and fail closed
  on stale same-level runtimes. Removed the stored-row-count shortcut so the
  dense backend can accept full-rank literal-copy designs with residual degrees
  of freedom. Narrowed the control-Schur diagnostic documentation to the JLA
  backend that actually prepares it.
- Raised the internal Mata API level to 10. Fixed-seed JLA now orders the
  conceptual physical-copy stream by outcomes and per-copy target mass rather
  than encoded worker, firm, match, frequency, stored-row labels, or raw
  control coordinates. Rows tied on that invariant key may share a stream
  position only when their controls agree and they are exchangeable within one
  worker--firm coordinate and, in match mode, one deletion block. Other ties
  fail closed as `AMBIGUOUS_PROBE_ORDER` and can still be evaluated with the
  deterministic exact backend. Registered tests cover ID relabeling, control
  reparameterization, and a partial frequency split at fixed seed.
- Every API-10 JLA full control fit, including `nuisance(fixedoffset)`, now
  passes the probe-independent within-cell full-fit and deletion-rank
  certificate before any point estimate can be posted. This prevents
  approximate FE residualization from manufacturing a tiny positive Schur
  complement for a control exactly in the FE span. PCG stopping and
  recomputed residual gates are also scale-relative for every nonzero right
  hand side. Registered match/observation fixtures cover small and ordinary
  control scales plus an explicitly loose allowed solver tolerance.
