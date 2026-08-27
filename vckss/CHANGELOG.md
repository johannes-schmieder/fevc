# Changelog

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
  and state gates; the route remains private because the synthetic performance
  and memory gates fail. Official full-CMG repeated solves are the registered
  next bottleneck; alpha hardening and the benchmark PDF remain deferred.
- Vendor and pin standalone CMG `dbefbc5`, adopt Rust 1.85.1, and promote the
  scalar direct hybrid solver as `CMG_FULL_V2` with checked whole-command
  pre-RNG memory admission, actual-retained reconciliation, deterministic
  same-route residual refinement, cooperative UserBreak cancellation, and
  exactly-once lifecycle cleanup.
- Qualify the registered no-control match-JLA cell through explicit Rust and
  automatic backend selection on macOS and Linux. Source `dd39f04` is 1.386x
  matched MATLAB on the macOS headline and 1.758x MATLAB on SCC's fixed CZ18
  case while outperforming the private winner by 7.0% and 7.6%, respectively.
  Unsupported cells retain their prior routes; errors
  after full-CMG selection never fall back. Windows and public release remain
  deferred.

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
