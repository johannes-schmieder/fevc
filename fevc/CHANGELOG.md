# Pending changes

## Projection inference repair — 2026-09-02

- Replace the biased centered observation proxy with uncentered cross fitting
  and a symmetrized block identity for unrestricted within-match covariance.
  Exact Mata and sparse Rust/JLA now honor the default match-deletion
  mover/stayer partition; explicit observation deletion remains supported.
- Projection slopes are location-normalization invariant. The automatic
  intercept is normalization-dependent under last-firm-zero grounding.

# Changelog

## 0.5.0-alpha.1 — 2026-08-30

- Match the maintained MATLAB package's default match-deletion population:
  retained movers plus eligible attached one-firm stayers in one pooled fit
  and target. Movers retain declared match deletion; stayers use literal
  physical-observation deletion and are explicitly not match-robust.
  `stayers(movers)` is the mover-only opt-out.
- Make the combined result the primary `e(results)`, `e(b)`, `e(kss)`, and
  `e(sample)` contract, while retaining exact mover intermediates under
  `e(mover_*)` and compatibility aliases under `e(stayer_hybrid_*)`.
- Implement the mixed convention in Mata and Rust generic JLA, including
  joint leverage sketches, deterministic rank checks, memory admission,
  zero-stayer reduction, and exact-oracle regression coverage.

- Rename the public Stata command, package, help topic, and repository from
  `vckss` to `fevc` as a hard cut. No compatibility command or wrapper is
  installed.
- Keep the version at `0.5.0-alpha.1` and preserve the estimator, returned
  results, numerical gates, and routing behavior.
- Normalize every distributed Mata/runtime, private Ado-helper, and native
  plugin artifact filename to `fevc`; private Stata programs now use
  `_fevc_*`. Retain the established internal `vckss__*`, `__vckss_*`,
  `VCKSS_*`, CMG, Rust crate, C ABI, build, runner, and SCC identities.
- Install the result table as the autoloadable `_fevc_display.ado` helper so
  exact and Rust postprocessors remain display-safe after an in-session
  development-package replacement.
- Extract the planned-route projection receipt check into its existing private
  namespace so Stata can skip the no-projection branch without misparsing
  nested braces; numerical checks and posted results are unchanged.
- Preserve predecessor reports, receipts, reviews, migration records, and the
  earlier changelog under their original identities. Remove the duplicate
  `vckss/` working tree after pinning its complete Git tree and adding a
  browsable archive index; retain the unique CMG source-review manifest in the
  active vendor provenance directory.
- Add a deterministic, non-publishing portable source-archive builder driven
  by `fevc.pkg`, with exact file, metadata, receipt, and reproducibility tests.
- Harden clean-install coverage for help lookup and missing-plugin preflight
  fallback, and protect source-bound evidence logs from workspace cleanup.
- Reconcile the active plan with the completed rename qualification and close
  the repaired scale-bundle and Stata 19 harness issues without changing any
  estimator, routing, fallback, inference, RNG, or numerical semantics.

## 0.5.0-alpha.1 — in development

- Add opt-in Mata exact inference for unit-weight observation deletion.
  `inference(highrank)` posts a polarized joint covariance for the four
  established component targets; point-only calls remain unchanged and post no
  `e(V)`.
- Add `inference(q1)` rank-one weak-identification diagnostics and
  Anderson--Rubin-style intervals. Critical values and interval mapping are
  independently implemented from the published KSS formulas; no MATLAB source
  or table is distributed.
- Add worker- or firm-effect projections on an automatic constant and numeric
  covariates, with frequency- or target-mass weighting and separate KSS and
  naive covariance returns under `e(projection_*)`.
- Add an explicit scalable `project()` route for observation deletion and
  positive integer frequency weights through qualified Rust generic JLA with
  either diagonal PCG or forced CMG. Frequency weights are literal physical-
  copy counts. Both routes reuse the retained sparse solver, solve one
  coefficient-space loading per projection column, stream KSS and naive
  covariance accumulation, and reconcile complete-system residual,
  conditioning, PSD, memory, and result-schema receipts. Forced CMG reuses one
  admitted hierarchy across the full and fixed-effect solves and fails closed;
  automatic projection routing remains withheld.
- Add fail-closed capability routing, smoothing/covariance/eigen gates,
  deterministic inference seeds, caller-RNG restoration, an independent dense
  projection oracle, and focused tests for default compatibility, q=1 critical
  values, target identities, typed failures, and clean packaging.
- Keep component inference on Mata exact, and keep match-cluster projection,
  projection stayer hybrids, and non-generic Rust projection routes outside
  the initial scalable projection capability.

- Import standalone CMG `761a0f0` as the immutable pre-routing comparison
  checkpoint while preserving VCkss cancellation, memory admission, warm
  starts, and the contiguous-RHS bridge.
- Integrate the `d9fef06` connected vector-only routing candidate, source-bound
  at descendant `92a12f2` after its arithmetic-order-preserving strict-Clippy
  repair and identifiable-Laplacian benchmark correction, for non-regression
  qualification against the `761a0f0` checkpoint.
- Parallelize independent Schur-RHS assembly for multi-column full-CMG batches
  on the solver-owned thread pool while preserving each column's arithmetic
  order, the one-column/one-thread path, cooperative worker cancellation, and
  `CMG_FULL_V2`. Extend the checked pre-RNG batch-vector forecast for every
  concurrently live worker-scaled temporary.
- Fail candidate qualification, pilot, production, retry, and aggregation
  closed unless a hash-bound preparation receipt proves that Stata/MP licenses
  the four Stata processors needed by the benchmark; this distinguishes the
  16 scheduler slots and Rust/MATLAB target from the capped Stata/Mata
  application entitlement before launching large SCC arrays.
- Version the comparative input receipt at V6 and use a degree-three shallow
  hub-tree leaf-panel graph for the weak connected-vector cell. Five panels
  share each leaf and select spokes in a diameter-four tree with 1,601 hubs;
  the largest cell has 655,360 workers, 132,673 firms, and 788,032 canonical
  edges. It clears CMG's frozen 131,072-vertex and 350,000-edge vector floors
  while contracting completely in one level with zero plan bytes. Earlier
  cycle, chord, block, offset-ring, hub-ring, and six-hub repairs either failed
  the unchanged residual gate, built the same plan in both sources, or could
  not reach the vector floor. The final mapping passed a full local one-core
  200-probe estimator screen at maximum complete residual `6.5084e-6`. The
  registered graph/row/core/repetition matrix and scientific gates remain
  unchanged.

## 0.4.0-alpha.1 — in development

- Make corrected statistical-result equivalence and end-to-end MATLAB
  competitiveness the primary development gates. Register a scale- and
  numerical-MCSE-aware comparison policy; retain bitwise/ULP equality, equal
  iterations, and legacy fixed roundoff thresholds as nonblocking diagnostics
  while keeping estimator, residual, finite-output, accounting, memory,
  failure, and caller-state safety hard.
- Replace the alpha benchmark's exact-repeatability and Mata-speed promotion
  gates with V2 corrected-result equivalence and MATLAB-primary performance
  status. Until a source-bound maintained-MATLAB timing receipt is integrated,
  the analyzer reports `INCOMPLETE` rather than making an alpha claim.
- Begin the private performance-first alpha milestone for macOS arm64/Rosetta and
  SCC Linux x86-64; Windows and public release remain deferred.
- Register Rust-preferred automatic backend and MATLAB-like JLA/200-probe
  defaults as the alpha target, with Mata fallback limited to missing-runtime
  or unsupported-request preflight.
- Qualify the Rust-preferred default and effective-option planned JLA routes on
  macOS arm64, universal, and Rosetta at source `daca3e2`; omitted `rng()`
  resolves to Counter-V1 on Rust and Stata RNG on a preflight Mata fallback.
- Qualify semantic `probeorder()` tie breaking for public compressed and
  generic Rust JLA at source `3bc6a89`, including a versioned preparation ABI,
  exact memory accounting, receipt reconciliation, permutation/batch
  invariance, and clean installation.
- Qualify exact `stayers(both)` through a versioned native augmentation
  lifecycle at source `c199bf0`, including dense/Mata differential oracles,
  zero-RNG reconciliation, typed failures, and arm64/Rosetta clean installs.
- Qualify the Linux x86-64 candidate on BU SCC under Stata MP 19 at source
  `86e0711`, with successful Rust/C/ABI gates, full public suite, isolated
  clean installation, candidate/source hashes, and SGE job accounting.
- Add source-bound bounded Miri, C-shim ASan/UBSan, malformed-ABI libFuzzer,
  RustSec audit, license-inventory, and deterministic CycloneDX 1.5 SBOM
  evidence. Windows and final human public-release review remain deferred.
- Add a generated Rust/Mata parity ledger and a protected dry-run/apply cleanup
  tool. The initial cleanup removed 42,120 ignored files and 3.98 GB without
  touching tracked source or source-bound evidence.
- Stop byte-locking the mutable latest-CI pointer while retaining immutable
  per-SHA receipt protection.
- Preserve the private direct full-CMG performance wave and its registered
  hard-case decisions. The fixed-CZ18 alternating matrix reaches 2.0667x
  maintained MATLAB, while the synthetic alternating matrix reaches 1.4803x
  and modestly exceeds MATLAB peak RSS. Both pass the statistical, residual,
  and state gates; at that checkpoint the route remained private because the
  synthetic performance and memory gates failed. Official full-CMG repeated
  solves were the registered next bottleneck; the subsequent scalar
  production wave supersedes that decision without rewriting its evidence.
- Vendor and pin standalone CMG `dbefbc5`, adopt Rust 1.85.1, and promote the
  scalar direct hybrid solver as `CMG_FULL_V2` with checked whole-command
  pre-RNG memory admission, actual-retained reconciliation, deterministic
  same-route residual refinement, cooperative UserBreak cancellation, and
  exactly-once lifecycle cleanup.
- Qualify the registered no-control match-JLA cell through explicit Rust and
  automatic backend selection on macOS and Linux. Runtime source `4b6874e` is
  1.397x matched MATLAB on the macOS headline and 1.731x MATLAB on SCC's fixed
  CZ18 case while remaining faster than the private winner in both matrices.
  Unsupported cells retain their prior routes; errors
  after full-CMG selection never fall back. Windows and public release remain
  deferred.
- Preserve exact-source macOS, SCC, supply-chain, timing, memory, residual,
  and statistical receipts and publish the source-bound CMG-style benchmark
  report. No 2x-MATLAB, Windows, tag, or public-release claim is made.
- Inventory and remove 26,310 obsolete regenerable build, cache, CI-scratch,
  and local-plugin files totaling 4.187 GB while preserving tracked reports,
  failures, benchmark receipts, and qualification evidence.

## 0.4.0-dev — 2026-08-24

- Make a hard-cut public rename from `varcomp_kss` to `vckss`: install only
  `vckss.ado`, `vckss.sthlp`, and `vckss.pkg`, and post
  `e(cmd) == "vckss"` without a predecessor wrapper.
- Rename active developer/runtime entrypoints, plugin artifacts, CI markers,
  environment variables, package metadata, repository URLs, and build IDs to
  the `vckss` identity while retaining established private `_vckss_*`,
  `vckss__*`, and `VCKSS_*` interfaces.
- Retain archived reports, receipts, reviews, progress snapshots, manifests,
  and completed qualification evidence byte-for-byte. The unchanged v1 frozen
  inventory and source-bound v2 relocation inventory enforce that boundary.
- Record CMG ownership API 8 and generator API 5 for the renamed generated
  package target without changing its numerical implementation.
- Repair exact-V7 batch-applicability and poster plan-reason reconciliation
  without weakening any request, plan, residual, accounting, memory, or
  pre-RNG counter check.
- Admit explicit `backend(rust) rng(counter_v1) algorithm(auto) engine(auto)`
  requests through the frozen native V3/V4/V7 plan. When that plan selects
  exact, reconcile and post the exact family directly with zero estimator RNG,
  exact direct-memory admission, and exactly-once lifecycle cleanup.

## Unreleased — 2026-08-24

- Add the optional strict Rust backend with compositional request-capability,
  preparation, solve, result, release, and typed-error boundaries.
- Add deterministic exact, compressed-JLA, and generic-JLA result families,
  Counter-V1 randomized execution, complete original-system residual
  certification, and V7 pre-RNG execution-plan receipts.
- Add structural `engine(auto)`, route, CMG/diagonal fallback, batch, memory,
  wall-advisory, and counter-accounting receipts. Automatic choices are frozen
  before estimator RNG and never reroute after a later numerical or resource
  failure.
- Package the planned exact-V7 reconciler and poster and bind exact-limit,
  selection-reason, selected-engine, plan-memory, and zero pre-RNG counter
  fields in their tests.
- Expand source-local macOS plugin, clean-install, arm64/universal, exact,
  generic, compressed, routing, shared-atom, and differential test coverage.
- Preserve explicit `backend(mata)` and `rng(stata)` while making omitted and
  automatic backend requests Rust-preferred after capability preflight.
- Consolidate active documentation around one package README, one current plan,
  one testing guide, and an indexed contract/evidence directory. Remove
  redundant single-use trusted-patch staging after the exact-V7 helpers were
  already present in the package manifest.

## 0.3.0-dev — 2026-08-20

- Rework the help file in the `cellgraph` SMCL style and add three
  deterministic, self-contained AKM examples that execute from the Stata help
  browser through the installed `varcomp_kss_run` helper while preserving the
  caller's data.
- Add an applied default display and `e(decomposition)`, separating raw
  covariance from the additive `2 x covariance` sorting contribution and
  reporting plug-in and corrected shares of target-weighted outcome variance
  and worker--firm totals.
- Use fixed-width display tables with explicit column headers and readable
  component labels. Each executable example reports the population worker and
  firm variances, covariance, and total implied by its DGP before estimation.
- Add retained target- and frequency-weighted outcome variances, residual
  variance, and the descriptive full-model explained variance and share. The
  full-model fit includes controls and remains distinct from the KSS-corrected
  worker--firm target.
- Standardize recognized failure output with a plain-language reason,
  actionable remedy, technical status, troubleshooting link, and
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
