# Current decisions

## Package identity

- `vckss` is the only public command, help topic, package manifest, and
  installed package identity.
- No predecessor-command compatibility wrapper is installed.
- Active private Mata symbols use `vckss*`; Ado helpers use `_vckss_*`; package
  globals use `VCKSS_*`.

## Backend consent and defaults

- Omitted `backend()`, explicit `backend(mata)`, and `backend(auto)` select the
  established Mata implementation.
- Rust is explicit opt-in. It may not become the default through a documentation
  change, availability probe, or performance heuristic.
- `rng(counter_v1)` is accepted only on the strict Rust route. Exact-selected
  native execution consumes no estimator RNG even when an auto request required
  the Counter-V1 capability contract.

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
- CMG API 7 and generator API 4 are ownership/interface successors over the
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
