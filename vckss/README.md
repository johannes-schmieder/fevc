# `vckss`: KSS point estimates in Stata

`vckss` is an internal-development Stata 18/19 implementation of the
Kline--Saggio--Sølvsten leave-out bias correction for linear two-way
fixed-effect variance decompositions. It reports point estimates and numerical
diagnostics only. It does not post `e(V)` or provide econometric confidence
intervals.

`vckss` is the only public command and package identity. No predecessor
alias is installed.

## Estimator surface

The command targets:

- worker-effect variance;
- firm-effect variance;
- worker--firm covariance; and
- variance of the worker-plus-firm sum.

The established Mata implementation supports exact and improved-JLA
calculation, match or observation deletion, joint or fixed-offset controls,
positive integer frequency weights, separate target weights, mover-only match
headlines, explicit deletion IDs, structural B1/CMG routing, and the registered
Stata RNG contracts. Mata exact also offers the separately labelled
`stayers(both)` mixed-deletion point hybrid; the mover result remains the
headline and stayers are never presented as match-cluster robust.

The package preserves coefficient cells, deletion units, and exact
target-scale strata as separate objects. It never merges target scales by a
tolerance. Accepted calculations must pass identification, rank,
complete-original-system residual, accounting, finite-output, direct-memory,
and caller-state restoration gates.

## Backend routing

`backend()` and `rng()` are explicit consent surfaces.

| Request | Selected runtime |
|---|---|
| omitted `backend()` | Mata |
| `backend(mata)` | Mata |
| `backend(auto)` | Mata |
| `backend(rust)` | strict native route, subject to a compositional capability receipt |

The default is intentionally not native auto-selection.

The Rust backend now contains three result families:

- **exact**: deterministic dense exact calculation; estimator RNG and iterative
  preconditioning are not applicable;
- **compressed JLA**: no-control match specialization with registered
  coefficient-cell, deletion-unit, target-stratum, and Counter-V1 semantics;
- **generic JLA**: general worker--firm/control path with planned diagonal or
  CMG routing and complete per-RHS receipts.

Automatic algorithm, engine, route, and batch decisions are resolved from the
materialized retained structure before estimator RNG and are frozen. There is
no post-selection numerical or resource fallback. An eligible automatic
CMG-to-diagonal fallback is allowed only during pre-RNG setup and is recorded.

### Current development boundary

Explicit Rust exact and planned compressed/generic JLA routes have dedicated
source-local tests. Public `algorithm(auto)` is qualified when the native plan
selects exact, including direct exact-family posting and zero estimator RNG.

The private alpha milestone now broadens public admission from effective
options, makes automatic routing Rust-preferred with preflight-only Mata
fallback, adds `probeorder()` and exact `stayers(both)` parity, qualifies Linux
on SCC, and publishes scale evidence. The generated gap ledger is
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md). A green quick suite is
not full plugin qualification.

## Scientific and numerical invariants

Every accepted RHS is checked against the original frequency-weighted normal
equations, not only a graph or Schur system. Acceptance requires a complete
relative residual no larger than `max(1e-11,10*tolerance())`, with an absolute
residual for a zero RHS.

The solver works on the full-firm zero-sum quotient and checks the original
grounded firm equation after display normalization. Controlled paths use a
canonical, ID-free control-span basis and fail closed when its numerical
uncertainty cannot certify rank or deletion actions.

JLA uses separate leverage and target RNG domains. Logical atoms are invariant
to row permutation, batching, solver route, processor count, and scheduling
within the same observed-ID design. Arbitrary ID relabeling may change a draw
but may not change validity or deterministic exact results. Caller data,
`e(sample)`, RNG algorithm, stream, complete state, and sort state are restored
on every exit.

The finite-projection formula and detailed contracts are documented in
[`docs/JLA_FINITE_PROJECTION.md`](docs/JLA_FINITE_PROJECTION.md),
[`docs/ESTIMATOR_CONTRACT.md`](docs/ESTIMATOR_CONTRACT.md), and
[`docs/NUMERICAL_ARCHITECTURE.md`](docs/NUMERICAL_ARCHITECTURE.md).

## Installation from a checkout

For the portable Mata package:

```stata
net install vckss, from("/absolute/path/to/vckss/vckss") replace
```

The tracked package manifest ships portable Ado/Mata source and Rust boundary
helpers, not plugin binaries. The macOS qualifier builds, audits, signs, stages,
and clean-installs temporary native artifacts. Other platforms require their
own qualification.

A normal Mata call is:

```stata
vckss log_wage age2 age3 i.year [fw=freq],              ///
    worker(person_id) firm(establishment_id)                   ///
    deletion(match) deletionid(actual_match_id)                ///
    nuisance(joint) targetweight(target_mass)                  ///
    algorithm(jla) engine(auto) preconditioner(auto)           ///
    probes(200) batch(auto) memory_gib(16) seed(8675309)
```

An explicit source-local Rust JLA call is:

```stata
vckss log_wage [fw=freq],                                ///
    worker(person_id) firm(establishment_id)                   ///
    deletion(match) deletionid(actual_match_id)                ///
    targetweight(target_mass) algorithm(jla) engine(auto)      ///
    preconditioner(auto) batch(auto) backend(rust)             ///
    rng(counter_v1) probes(200) seed(8675309)
```

Exact Rust calls use `algorithm(exact)`; estimator RNG is not consumed. A Rust
call with `algorithm(auto) engine(auto) rng(counter_v1)` can select and post the
exact family when its retained identified dimension is within `exact_limit()`.

## Documentation and evidence

Start with [`docs/README.md`](docs/README.md). The active development plan is
[`PLAN.md`](PLAN.md), test taxonomy is [`TESTING.md`](TESTING.md), public change
history is [`CHANGELOG.md`](CHANGELOG.md), and typed failures/returns are in
[`docs/FAILURES_AND_RETURNS.md`](docs/FAILURES_AND_RETURNS.md).

Performance reports and qualification directories are source-bound evidence.
They do not authorize a different source revision, a production dataset, or a
public release.

## License and release status

The package-owned CMG implementation and a distributed package containing it
are GPL-3.0-only. Public redistribution remains disabled until the documented
human license/provenance review is complete. See
[`../CODE_LICENSE.md`](../CODE_LICENSE.md) and
[`docs/SOURCE_PROVENANCE.md`](docs/SOURCE_PROVENANCE.md).
