# Current decisions

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
- No hidden regularization or silent change of sample, target, deletion,
  nuisance, algorithm, tolerance, probe count, or route is permitted.

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
