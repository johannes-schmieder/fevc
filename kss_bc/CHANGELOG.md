# Changelog

## 0.1.0-dev — unreleased

- Raised the internal Mata API to 15. Diagonal B1 and forced test-only CMG now
  share one lockstep batched PCG kernel and therefore share every quotient,
  recurrence, restart, grounding, reconstruction, failure, and complete
  residual check. The public ado has no preconditioner option, the package
  manifest does not ship CMG, and no automatic route is enabled.
- Added a true end-to-end forced-CMG public-estimator test and bounded
  easy/moderate/weak benchmark drivers. At 10,000 workers, 1,000 firms, and
  200 probes, local Stata 18 command speedups are 1.35x on moderate and 3.60x
  on weak; easy CMG retains a typed `HIERARCHY_STALLED` failure. Estimator
  differences pass the registered matrix-relative `2e-9` gate and every RHS
  passes the fresh complete-system residual gate.
- Added four-slot, 64 GB SCC wrappers with a mandatory measured 90-minute
  projection and a 5,400-second process timeout. A separate checksum-bound,
  read-only Separations wage harness prepares `logrwage-xb`, compares B1 with
  forced CMG on an identical retained sample, and invokes the maintained
  MATLAB LeaveOutTwoWay implementation as a 200-probe timing reference.
  Restricted rows and retained-match files remain SCC-only.
- Matched the real-data preparation and estimator key contract to the
  Separations KSS export:
  `persid estabid time` identifies physical observations, while registered
  analysis worker/firm units may repeat within the coarser analysis period.
  A synthetic regression test preserves these valid aggregate duplicates and
  records their count in preparation metadata.
- Added an opt-in `probeorder()` physical-observation key for discrete
  outcomes whose outcome/target key ties across distinct model coordinates.
  The key must be complete and globally unique, is never inferred, and only
  refines exact ties. Existing streams and non-tied order are unchanged. The
  Separations harness records its unique SCC-only observation key and tests
  row-order, ID-relabeling, batch, residual, and equal fixed-seed RNG end-state
  invariance.
- Kept prepared MATLAB reference rows worker-contiguous and chronological and
  independently sorts the four-column reference input by worker, period, and
  firm before invoking the maintained LeaveOutTwoWay code. This changes no
  row, outcome, model coordinate, target, or Stata probe assignment.
- Raised the internal Mata API to 14. Matrix right-hand sides now use true
  lockstep diagonal PCG with one matrix Schur traversal per iteration,
  independent per-RHS recurrences and statuses, periodic explicit residual
  drift checks, unchanged quotient grounding and worker reconstruction, and a
  fresh complete worker-plus-firm residual for every accepted RHS. The former
  scalar loop remains a test-only B0 oracle.
- Added separated setup, Schur-action, preconditioner-application, PCG,
  leverage, target, and backend timings; RHS-equivalent and physical-batch
  counts; and stage-coded per-RHS iteration/residual diagnostics. The synthetic
  SCC driver exports these fields and total runtime.
- Added a forced test-only KSS adapter for shared CMG API 2 plus easy,
  moderate, and weak benchmarks. CMG remains absent from the installed
  package and automatic routing remains disabled because all promotion gates
  have not passed.

- The first million-row, 200-probe SCC attempt reached its 12-hour scheduler
  limit with stable memory and CPU use but before Stata returned. The scale
  harness now requests 18 hours for the large case. Dimensions, probes,
  numerical tolerances, and acceptance gates are unchanged; a regression test
  pins the revised request.

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
- Two fresh independent API-10 audits found separate pre-closure defects.
  The ado layer discarded ordinary all-zero controls before the full-design
  rank gates, and the matrix-free solver grounded one encoded firm before PCG,
  so a loose allowed stopping tolerance could make accepted JLA estimates
  depend on which raw firm label became the base.
- Raised the internal Mata API level to 11. Factor-variable preprocessing now
  removes only terms that Stata explicitly marks omitted; user-supplied zero
  and collinear columns reach and fail the exact or JLA rank gates. Registered
  failures cover exact, JLA, both nuisance and deletion conventions, both
  automatic-dispatch choices, and an invertibly transformed redundant basis.
  The FE service now solves the full singular firm Laplacian on its zero-sum
  quotient and applies public last-firm grounding only after convergence.
  Full residual validation includes every worker and firm equation. A fixed-
  seed, maximum-tolerance test requires invariant results after simultaneous
  worker, firm, and deletion-ID relabeling, including the reviewer's six-row
  `K(2,3)` counterexample and every possible displayed base firm.
- API level 11 also maps every accepted control span to a weighted,
  row-anchor canonical basis before iterative inverse actions. The reviewer's
  determinant-four transformation is registered at `probes(2)` under joint
  and fixed-offset conventions. User solver tolerance is capped at `1e-4`;
  the former `tolerance(.09)` attack is a typed invalid-tuning failure.
  Separate full-fit and correction-system parameter counts are posted, the
  failure catalog covers all inverse residual labels, and observation JLA has
  a typed pre-allocation `physical_limit()` gate.
- Two fresh API-11 audits did not close KB5. One produced a native-replayable
  boundary witness where two invertible control bases both passed the fuzzy
  anchor rule but generated different fixed-seed JLA results. The other run
  stalled before a final verdict but exposed an unchecked final subtraction:
  finite plug-in and correction rows can have a nonfinite difference.
- Raised the internal Mata API level to 12. Canonical anchor selection now
  propagates a forward-error envelope from whitening and inverse residuals,
  withholds uncertainty at the anchor margin, and rejects pivot scores near
  the eligibility cutoff as `AMBIGUOUS_CONTROL_BASIS`. The 14-row determinant-
  one witness is registered natively across exact/JLA/auto, both nuisance
  conventions, two batch sizes, `probes(2)`, and `tolerance(1e-4)`. Exact and
  JLA now check the completed subtraction separately and withhold
  `NONFINITE_CORRECTED_TARGET` before posting any result.
- Two fresh API-12 audits remained false. They identified raw-control/ID row
  ordering in exact mode, a dimension-free anchor envelope, unbounded
  literal-sign allocation in match/target JLA, and rounded component-mass
  comparisons above `2^53`.
- Raised the internal Mata API level to 13. Controlled exact, JLA, and both
  automatic dispatches now share an outcome/per-copy-target semantic order
  that never uses raw controls or encoded IDs; unresolved nonexchangeable ties
  fail closed. The canonicalizer caps controls at 32, converts column residuals
  to dimensioned operator bounds with positive conditioning denominators,
  accounts for Gram, Cholesky, score/cutoff, projector, anchor, and span error,
  and propagates its envelope through full-fit and deletion margins.
- Added an exact incremental `2^53` physical-total gate before graph ranking,
  extended `physical_limit()` to every JLA path, and registered both reviewers'
  accepted-path and component-mass counterexamples under native Stata.
- The paired SCC oracle now imports MATLAB floating-point columns as Stata
  doubles. This preserves the CSV precision needed by its registered
  cross-language tolerances instead of applying Stata's default float storage.
- The synthetic scale ladder now uses four discrete within-match control rows
  with registered first- and second-pivot anchor margins at every planned
  scale. The former sinusoidal control produced an incidental row score at the
  fail-closed canonical eligibility boundary in the 200,000-row case. Its
  iterative tolerance is the ladder's independently validated `1e-8` output
  ceiling rather than a stricter scale-dependent smoke-test setting.
