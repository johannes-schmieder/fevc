# Current decisions

## Exact-observation inference

- Version `0.5.0-alpha.1` adds inference only by explicit request. Point-only
  behavior and the absence of `e(V)` on default calls are unchanged.
- The first capability is Mata exact with observation deletion, movers, and
  unit frequency weights. Omitted or automatic algorithm selection resolves
  to exact for such a request. Rust, JLA, Counter-V1, match-cluster inference,
  frequency-weight inference, and stayer-hybrid inference fail closed.
- `inference(highrank)` posts a polarized joint covariance for the three
  primitive targets and maps it to the four established targets.
  `inference(q1)` additionally posts rank-one weak-identification diagnostics
  and Anderson--Rubin-style intervals from repository-authored simulation and
  ellipsoid mapping.
- `project()` is a separate fixed-effect linear-projection surface. Its KSS
  covariance and naive residual-squared comparison are stored under
  `e(projection_*)`; projection alone never populates component `e(V)`.
- The maintained MATLAB package is a behavioral reference only. Its source and
  critical-value table are not licensed for copying and are not included.
  The implementation follows the published formulas and ships as
  GPL-3.0-only repository-authored source.
- Materially negative smoothed variances or indefinite covariance estimates
  withhold the complete request. Only tiny registered roundoff values may be
  set to zero, and all such cleanups are receipted.

## Comparative-scaling right-censoring

- On 29 August 2026, the owner accepted the largest `strong_d2` one-core
  MATLAB cell as right-censored rather than authorizing another long rerun.
  Maintained MATLAB reached the registered 10,800-second estimator limit in
  all three position-balanced repetitions in both immutable production
  generations. The affected historical task IDs are 61, 62, and 63
  (`scale_strong_d2_n1966080_c1_r1` through `r3`).
- The accepted study therefore contains 297 complete same-host tasks and 891
  successful estimator calls. The three MATLAB lower bounds remain in a
  separate source-bound censor ledger. Partial Mata and Rust results from
  those sequential tasks do not enter the accepted call ledger.
- The entire graph--size--core cell is excluded from route rankings, paired
  ratios, and one-to-many-core scaling. This is a declared missing-cell rule,
  not an imputation or a relaxed numerical/application gate.
- Future comparative-scaling generations must not submit those three exact
  experiments. The executable record is
  [`future_exclusions.json`](../benchmarks/comparative_scaling/future_exclusions.json).

## Development priority and equivalence

- The primary product objective is a fast Stata alternative to maintained
  MATLAB KSS that returns the same statistical result on compatible problems.
  Corrected-result equivalence is first, end-to-end MATLAB competitiveness is
  second, and pathwise/backend-internal parity is secondary.
- The machine-readable policy is
  [`development_acceptance_v1.json`](development_acceptance_v1.json). For a
  corrected target pair `a,b`, let `s=max(1,abs(a),abs(b))`. Deterministic
  comparisons accept `abs(a-b)<=1e-8*s`; common-draw randomized comparisons
  additionally allow ten percent of the larger reported numerical MCSE.
  Independent-draw comparisons accept the greater of the scale floor and `6`
  times combined numerical MCSE. Comparators without MCSE use a registered
  repeated-seed distribution.
- The primary result comparison is the four corrected targets. Plug-in and
  correction rows, bitwise/ULP equality, iteration counts, counters, and
  reduction order remain valuable diagnostics but do not block a faster
  candidate whose corrected estimates pass the registered rule.
- Legacy source-bound receipts keep their original thresholds and statuses.
  Active development may reinterpret a legacy roundoff miss under the current
  policy without rewriting historical evidence.
- Test and qualification selection is impact-based. A new commit SHA alone
  does not require a full rerun. Large SCC arrays, broad platform matrices, and
  full native profiles require a material affected-surface reason, a benchmark
  or release question, risk that focused checks cannot bound, or an explicit
  owner request.
- Exact-source receipts remain immutable, but unaffected claims may be carried
  forward through a recorded compatibility review that identifies both source
  states, the changed paths, the relevant unchanged production/build/binary/
  input/acceptance identities, focused checks, reused claims, and limitations.
- Hard correctness still includes the same estimand/sample/target semantics,
  finite results, identification and accounting, complete original-system
  residuals, no hidden regularization or post-RNG estimator fallback, direct
  memory admission, typed failure/UserBreak, and caller-state/lifecycle
  restoration.
- A candidate is speed-competitive when its registered median complete-command
  time is no slower than maintained MATLAB on the compatible comparison. The
  development target remains approximately one-half MATLAB time. Kernel-only
  or solver-only wins are diagnostic rather than sufficient.

## Package identity

- `vckss` is the only public command, help topic, package manifest, and
  installed package identity.
- No predecessor-command compatibility wrapper is installed.
- Active private Mata symbols use `vckss*`; Ado helpers use `_vckss_*`; package
  globals use `VCKSS_*`.

## Backend consent and defaults

- The alpha contract makes omitted `backend()` and `backend(auto)` prefer Rust
  only after a complete effective-request capability preflight.
- Missing runtime or a structurally unsupported tuple may fall back to Mata
  before native preparation and estimator RNG. Stale/corrupt receipts and all
  later failures fail closed.
- `backend(rust)` is strict and `backend(mata)` is explicit Mata.
- Omitted `rng()` and `rng(auto)` select Counter-V1 for Rust or Stata RNG for
  Mata. `rng(counter_v1)` is strict Rust consent; `rng(stata)` selects Mata and
  conflicts with strict Rust.
- Omitted `algorithm()` selects MATLAB-like JLA with 200 probes. Explicit
  `algorithm(auto)` retains exact-small/JLA-large structural planning.

## Automatic planning

- Algorithm, engine, solver route, fallback, batch width, memory, and wall
  planning resolve from the materialized retained structure before estimator
  RNG.
- The V7 execution plan is frozen. A later memory, rank, setup, convergence, or
  numerical failure cannot trigger a different estimator or engine.
- Explicit CMG fails closed. Automatic CMG-to-diagonal fallback is allowed only
  before RNG and must be recorded.
- Exact plans use no iterative route, probe batch, or estimator counter range.

## Scientific and numerical acceptance

- Point estimation remains the default; only an explicit accepted
  exact-observation component-inference request posts `e(V)`. Probe dispersion
  has no econometric interpretation.
- Match headlines are mover-only and preserve the declared deletion unit.
- Frequency weights are literal positive integer copies; explicit target
  weights are stored-row target mass.
- Coefficient cells, deletion units, and exact target-scale strata remain
  separate.
- Every accepted RHS passes the complete original-system residual gate, target
  accounting identities, rank/estimability gates, direct-memory admission, and
  finite-output checks.
- Omitted `tolerance()` uses `1e-10` for fit/deterministic solves and `1e-6`
  for randomized probes. An explicit value overrides both; effective phase
  tolerances and residual gates are receipted.
- No hidden regularization or silent change of sample, target, deletion,
  nuisance, algorithm, explicit tolerance, probe count, or route is permitted.

## Results and failure behavior

- Request, selection, execution-plan, memory, residual, counter, and
  caller-state receipts are part of the development contract.
- A result family is posted only by its matching poster: exact, compressed JLA,
  or generic JLA.
- Unsupported, inconsistent, unavailable, singular, unconverged, or
  resource-inadmissible states fail with a typed status. They do not fall back
  to a scientifically different command.

## CMG ownership

- CMG is a package component, not a shared library or independent release.
- Its deterministic generator produces only the shipped `vckss_cmg` runtime
  and the checked-in `cmgtest` target.
- CMG API 8 and generator API 5 are ownership/interface successors over the
  numerically qualified API 6 core.

## Evidence and release

- Historical reports, receipts, reviews, logs, manifests, and benchmark outputs
  are immutable source-bound evidence.
- Quick CI does not qualify the native plugin. Native claims require the
  source-local plugin profile and an exact-SHA receipt.
- Advisory timing or headroom misses do not withhold a scientifically and
  directly memory-safe command.
- GPL-3.0-only governs covered code. The human package-boundary,
  corresponding-source, notice, provenance, and data-exclusion review was
  completed on 2026-08-29. Public tagging and release remain separate owner
  decisions, with a fresh exact-artifact check required before conveyance.
