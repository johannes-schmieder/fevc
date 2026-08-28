# Current decisions

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

- Point estimates only; no `e(V)` and no econometric interpretation of probe
  dispersion.
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
- GPL-3.0-only selection does not itself authorize public release. Human
  license/provenance review remains mandatory.
