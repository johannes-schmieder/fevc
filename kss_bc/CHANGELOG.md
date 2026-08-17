# Changelog

## 0.2.0-dev — unreleased

- Raised the internal estimator surface to API 19 with an explicitly
  experimental `engine(auto|compressed|generic)` dispatch. The compressed
  path is limited to JLA, match deletion, no controls, deletion units contained
  in coefficient cells, exact target-scale strata, a physical total below
  `2^53`, at most 16,383 probes, a registered runtime RNG contract, and a
  passing pre-probe resource forecast. Forced compressed calls reject with the
  typed eligibility reason. Automatic generic fallback is admitted only after
  its raw-resident memory and wall forecast passes; otherwise it returns
  `GENERIC_RESOURCE_ADMISSION_FAILED` before RNG.
- Added a canonical two-layer representation that independently counts
  worker--firm coefficient cells and actual deletion units, plus exact target-
  scale strata within cells. Multiple deletion IDs may share one cell. The
  cell FE transpose, Schur actions, diagonal preconditioner, worker
  reconstruction, fit/RSS, leverage and target RHSs, target contractions, and
  complete original-equation residuals run on compressed arrays. Target
  strata are never tolerance-merged, and cancellation-sensitive grouped sums
  use compensated accumulation.
- Added the exact no-control match specialization
  `D_g=E_g(m_g^-1+B_g*m_g^-2-V_g*m_g^-3)`,
  `K_c=sum_(g->c) Y_g*D_g`, with target correction draw
  `sum_c K_c*z_c^2`. It removes generic per-match eigendecompositions and the
  row-sized deleted-adjusted vector while preserving the coefficient-one
  finite-projection moments, conditioning and reciprocal-residual gates, and
  typed failures. Independent dense and general-engine oracles cover repeated
  rows, literal frequencies, and several deletion IDs in one cell.
- Registered the K1 RNG invariant and two `mt64s` candidates. Local Stata 18
  golden-vector, partition, row/ID invariance, state-restoration, call-shape,
  and timing evidence selects one stateful fixed-order stream per domain over
  repeated per-probe resets. Leverage and target remain separate. The
  production guard restores the caller's RNG algorithm, selected stream, and
  complete state on every exit. Large exact binomial sums use documented
  scalar calls chunked at `1e11`; scalar/vector behavior and the `2^53-1`
  total contract are tested separately. Source-bound Stata 19 K1 job 7201105
  matched every Stata 18 golden vector, restored every touched stream, and
  selected the per-domain cursor in all three production-shaped timing pairs.
  Stata 18--19 now share the explicit
  `KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19` contract; other runtimes fail closed.
- Added native disk-backed Stata preservation for the compressed command. It
  constructs the canonical state, forces the preserved caller DTA to disk,
  clears row data during peak Mata work, releases the compressed runtime, and
  restores data, metadata, dirty state, order, and exact `e(sample)` semantics.
  Selection/transition/work/restoration timings and memory snapshots are
  returned; no destructive scale-only mode is enabled.
- Added overlap-aware pre-RNG resource admission for raw Stata data,
  persistent cell/unit/stratum state, CMG hierarchy/factors, phase matrix-RHS
  scratch, sort/compression temporaries, solve-ahead storage, outputs and
  certificates, preservation overhead, and the maximum simultaneous
  allocation. Both compressed and generic routes enforce 25--30 percent
  memory and 50 percent wall-time headroom inside 56 GiB and 12 hours, and
  every later scale must reconcile forecasts against measured phase peaks,
  process RSS, and `qacct maxvmem`.
- Preliminary source-bound CZ24/CZ25 P200 jobs 7201420 and 7203635 exposed a
  persistent process-residency omission in otherwise conservative allocation
  forecasts. Both scientific estimates passed, while the fail-closed resource
  validator rejected scale progression. Resource API 4 and solver receipt API
  22 now expose and charge a separate 96-MiB `runtime_resident_bytes` family
  in every lifecycle phase. The charge is 1.5 times the largest measured
  63,906,719-byte omission, rounded upward to a 32-MiB boundary, before the
  independent 30-percent admission margin.
- Added a source-bound single-job SCC harness. Each experiment runs one Stata
  process; there are no shards or reducers. The provisional request reserves
  14 SGE `omp` slots at 4 GiB per slot while the driver independently verifies
  `c(processors)==4` and reports scheduler slots and application processors
  separately. Inputs stage to node-local `$TMPDIR`. Separate well-connected
  and ring fixtures distinguish ordinary dimensional scaling from adverse
  connectivity and deletion-safety stress.
- Added local API 19 algebra, compression, RNG, lifecycle, resource, route,
  fixture, command, package-layout, and SCC-validator tests. The complete
  residual for every fit, leverage, and target RHS uses the original worker
  and firm right-hand sides, full-firm zero-sum quotient, last-firm displayed
  grounding with its equation checked, and
  `max(1e-11,10*tolerance())`. The local P40/P200 fixture is correctness
  evidence only. CZ24/CZ25, CZ18, connected 2x, mandatory well-connected 4x
  P200, 8x/16x admission, and maintained-MATLAB timing remain unmeasured SCC
  gates. API 19 is not production-qualified or a public release.
- Promoted API 18 with an installed public routing surface:
  `preconditioner(auto|diagonal|cmg)`, a declared 1--56 GiB
  `memory_gib()` envelope, and `batch(auto)` plus positive integer batches.
  Route selection completes before the production probe stream. Forced CMG
  fails closed; automatic diagonal fallback is limited to typed CMG
  preflight, construction, and pilot boundaries and preserves the originating
  status and message. Automatic choice uses deterministic structural-work
  scores, a bounded B1 fallback envelope, and the canonical 80-percent CMG
  work gate; live pilot timings are diagnostic only.
- Installed the generated clean-room CMG runtime and production KSS solver
  adapter beside the base Mata runtime. Package and clean-install tests now
  require all three numerical layers and the graph runtime.
- Replaced the match sample selector with a deletion-unit multigraph fixed
  point. Distinct deletion IDs remain parallel edges at a shared coefficient
  coordinate. Bridge units are removed simultaneously between deterministic
  largest-component, mover, insufficient-history, and articulation passes.
  Every accepted match sample carries a final zero-bridge certificate.
  Observation deletion retains the previous selector.
- Added public route, typed fallback, multigraph, fixed-point, runtime guard,
  and clean-install regression tests. The package remains internal because no
  public software license has been selected.
- Batched leverage and target contractions now reuse invariant panels,
  projections, and matrix RHS work. Exact one-level CMG terminals bypass PCG
  bookkeeping but still recompute every complete original-system residual.
  `batch(auto)` deterministically selects 8--64 columns from retained size,
  probes, processors, and a hard 35-percent scratch forecast. The persistent
  FE design and maximum concurrent solver allocation have a separate hard
  65-percent forecast; both memory failures are typed and occur before RNG.
- Raised the generated clean-room CMG core to API 5. Component-aware
  normalized-heavy-edge fallback aggregation supplies deterministic progress
  on hubs, paths, barbells, irregular graphs, and expanders while preserving
  component counts, Galerkin contraction, bounded terminals, and typed
  attempted-level failure diagnostics.
- Retained the reusable CMG workspace as an equality-tested API but selected
  ordinary batched applications in production. At 32,768 hybrid vertices the
  workspace was 34 percent slower at batch four and 74 percent slower at batch
  sixteen, consistent with the earlier 100,000-vertex slowdown.
- Added the content-addressed KSS-PROD-1 SCC DAG with Stata 18/19 estimator
  smokes, exact four-processor SCC license and timing cells, CZ24/CZ25 automatic
  and forced-route comparisons, five batch widths, CZ18 calibration/full-200,
  and a connected two-copy CZ18 stress case. Every estimator job emits the
  complete per-RHS residual table and a format-stable retained-match file.
  Capability run `20260816T035454Z-c3cb6a3` established that SCC's Stata 18
  and 19 modules are four-core-only; eight-processor behavior is measured by
  the local MP8 suite instead of being claimed from unavailable SCC capacity.
- Added the tracked local MP4/MP8 batch-scaling driver and hardened SCC
  evidence validation for legitimate zero-removal fixed points, aggregate-CSV
  identity rounding, and reason-text-independent direct-B1 routing. The first
  successful CZ18 multilevel preflight measured 1,490 command seconds, so the
  source-bound retry limit is 2,100 seconds under the registered
  `ceil(1.25*1490+120)` safety rule.
- Completed the immutable `5e2687c6` KSS-PROD-1 qualification. Installed
  Stata 18/19, CZ24/CZ25, batch/reproducibility, and the full 601-RHS CZ18
  estimator pass. All three parallel two-times-CZ18 P20 calibrations withhold
  before RNG because neither B1 nor CMG passes every bounded route gate. The
  validator therefore refuses the phase and no P200 stress job is submitted.
  Version 0.2.0-dev remains an internal, non-production-qualified candidate.

## 0.1.0-dev — unreleased

- Raised the forced test-only shared core to CMG API 4 after the first
  all-mover API 3 job remained above its 1,536-vertex policy cap and failed
  closed as `HIERARCHY_STALLED`. The memory-rich repeated-RHS terminal now has
  a 6,144 hybrid-vertex hard cap, covering the registered firm-plus-auxiliary
  envelope while retaining the 512-RHS, 16 GiB, and dense-factor-budget gates.
  Forced-C benchmark output now records fine hybrid vertex and edge counts.
- Completed the bounded Stata 19 MATLAB-retained real-data ladder. Small exact,
  B1, and forced C pass; a moderate post-oracle pair passes; and the natural
  256,472-row all-mover pair records 416.750 seconds for B1 versus 101.096
  seconds for C, a 4.122x command speedup. Estimator `mreldif` is `1.2879e-10`,
  maximum complete residuals are `9.9978e-11`/`4.0524e-13`, and peak RSS is
  443,140/490,172 KiB. Automatic routing remains disabled and the installed
  package remains B1-only.
- Raised the forced test-only shared CMG core to API 3. For at least 512
  planned RHSs, at least 16 GiB of declared memory, and no more than 1,536
  hybrid vertices, its resource profile may spend the registered dense-factor
  budget on one exact terminal factor. The allocation remains preflighted and
  the estimator's operator, recurrence, tolerance, probes, and complete
  residual checks are unchanged. Low-RHS and larger graphs retain the API 2
  policy.
- Raised the SCC MATLAB-retained sample adapter request to 16 GiB per each of
  four slots after job `7189318` hit a host-level Java virtual-memory mapping
  failure under the former request. The failed attempt is preserved with
  `failed=0`, `exit_status=1`; no estimator ran. Replacement adapter job
  `7189360` reconstructed the identical aggregate sample and passed.
- Raised the internal Mata API to 17 after SCC job `7188878` showed that a
  smaller rank-deficient projection could trigger the same LAPACK failure as a
  wide block. Every exact and JLA match now passes through one
  dimension-adaptive residual-maker helper: it uses the observation-space
  maker when that is smaller and the reduced Woodbury maker otherwise. It
  eigendecomposes the positive-definite maker, never the singular projection,
  and recomputes complete action residuals in observation space.
- Preserved job `7188878` as failed-closed evidence: source `64c8b59`, four
  seconds wall, exit 1, 137,108 KiB peak RSS, and the unchanged typed message
  `block projection eigenvalue calculation failed`. B1 and CMG again remained
  unsubmitted.
- Raised the internal Mata API to 16. Exact match blocks wider than the
  coefficient dimension and every JLA match correction now use an
  algebraically equivalent low-rank Woodbury residual-maker solve. The helper
  checks the reduced spectrum and inverse, then recomputes every requested
  action's complete observation-space residual. It avoids the former dense
  match-by-match projection, eigendecomposition, and inverse without changing
  the finite-projection formulas.
- Registered dense-versus-low-rank action equality and literal-copy versus
  frequency-weight exact equality. The prior SCC exact diagnostic on the
  MATLAB-retained sample is preserved as failed-closed job `7188809`:
  `block projection eigenvalue calculation failed`, exit 1, three seconds,
  and 133,652 KiB peak RSS. B1 and CMG were not submitted from that failed
  gate.
- Added an SCC-only, checksum-bound adapter for the retained match set from a
  successful maintained MATLAB run. It reconstructs all physical rows from
  the parent prepared DTA, repeats KSS graph pruning, and independently removes
  match bridges to a fixed point. Exact, B1, and forced CMG can now be compared
  on that one audited sample without changing the production KSS selector or
  making MATLAB a runtime dependency.
- Added exact-route SCC timing and a validator that requires exact/B1 plug-in
  equality, B1/CMG estimator equality, complete residuals, fixed probes, seed,
  tolerance and sample hash, peak RSS, and successful `qacct`. Every route is
  still admitted only with a measured projection at or below 90 minutes and
  is stopped after 5,400 seconds.
- At the owner's direction, cancelled obsolete scalar-B0 job `7185180` after
  9:15:07 wall time. Final accounting records exit 137, four slots,
  132,808.540 CPU seconds, and 1.329 GiB maximum virtual memory. It is labeled
  `USER_CANCELLED_OBSOLETE_B0`, not accepted benchmark evidence.

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
- Source-bound Stata 19 SCC end-to-end jobs record C/B1 command gains of
  1.725x on moderate and 4.620x on weak, estimator differences below
  `2.1e-11`, complete residuals below `1e-10`, and peak RSS below 122 MiB.
  Easy C again fails closed. The read-only CZ24 wage ladder rejects a
  nonestimable match deletion under both B1 and forced C, so no full-input
  scale-up or automatic route is enabled.
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
- Replaced the real-data small sample's arbitrary first-ID prefix with a
  deterministic dense mover core ranked by overlap through high-degree firms,
  mover degree, and the raw worker key. The frozen slice is shared by B1, CMG,
  and MATLAB; preparation records the selection rule and density boundary.
- Made the read-only MATLAB reference add the maintained CMG subtree
  recursively and bind `CMG/MATLAB/cmg_sdd.m` to its own SHA-256. The first
  bounded SCC attempt exposed the missing path and failed before estimation;
  it remains preserved as failure evidence.
- Added a checksum-bound, run-local build of the nine maintained MATLAB CMG
  hierarchy MEX sources. SCC binaries are written only below the KSS run
  directory, never into the read-only Separations checkout; MEX setup time and
  the canonical source-manifest SHA-256 are recorded separately. The loader
  rehashes after compilation and verifies that `graphprofile` resolves to the
  run-local binary before estimation. It separately checksum-binds and builds
  the maintained double-preconditioner source family required by parallel JLA.
  A MATLAB-only aggregate validator records descriptive reference evidence
  without requiring or relabeling a failed B1/CMG estimator comparison.
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
