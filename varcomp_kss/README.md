# `varcomp_kss`: KSS point estimates in Stata/Mata

`varcomp_kss` is an internal-development Stata/Mata implementation of the
Kline--Saggio--Sølvsten leave-out bias correction for linear two-way
fixed-effect variance decompositions. `varcomp_kss` is the only public command
and package identity; the predecessor command is not installed as an alias.

## Public backend routing

The command accepts `backend(auto|mata|rust)` and `rng(stata|counter_v1)`.
Omitting `backend()` permanently selects the established Mata estimator; it is
not an alias for `auto`. Explicit `backend(mata)` and `backend(auto)` also stay
on Mata. They use the historical Stata RNG contract, whether `rng(stata)` is
explicit or omitted.

The initial Rust route is deliberately narrow and requires explicit
`backend(rust) rng(counter_v1) algorithm(jla) preconditioner(diagonal)` and a
numeric `batch(#)`. It supports match deletion, joint nuisance handling,
movers, `if`/`in`, frequency weights, `targetweight()`, `deletionid()`, and the
ordinary seed, probe, tolerance, iteration, and memory options. It accepts
`engine(auto|compressed)`. Controls, observation deletion, fixed-offset
nuisance, stayers, `probeorder()`, `wallseconds()`, automatic batching, exact
calculation, generic engine, CMG, and nondefault unforwarded structural limits
fail before native preparation. There is no fallback after preparation starts.
`backend(rust)` without explicit `rng(counter_v1)`, or `rng(counter_v1)` on a
Mata/auto call, is a typed error rather than an implicit RNG change.

Successful calls record backend and RNG request/selection receipts. Rust calls
also post the authoritative retained sample, graph/preparation/memory receipts,
lossless per-RHS native receipts, topology checksum halves, Counter-V1 contract,
and full-fit certification fields. These remain point estimates and numerical
diagnostics only; the command does not post `e(V)` or provide econometric
standard errors.

The command targets worker variance, firm variance, worker--firm covariance,
and the variance of their sum. It supports observation or actual-match
deletion, exact and improved-JLA calculations, joint or fixed-offset controls,
positive integer frequency weights, and separate target weights. Match-mode
headlines use a mover-only fit and target. `stayers(both)` is explicitly
withheld; no observation-level fallback is presented as match robust.
Every randomized calculation with controls, including fixed offset, requires
a deterministic probe-independent full-fit and deletion-rank certificate.
Explicit zero or collinear numeric controls are never treated as omitted
factor levels: they remain in the requested full design and trigger a typed
rank failure. The JLA FE solver works on the permutation-equivariant firm
quotient and applies the displayed last-firm normalization only after solving.
Matrix right-hand sides use independent scalar PCG recurrences in true
lockstep: one matrix Schur traversal serves all active columns, while stopping,
curvature, iteration, and complete residual gates remain per RHS.
An ID-free canonical control-span basis makes controlled exact and iterative
inverse actions independent of an invertible user control reparameterization
on accepted paths. Every controlled backend uses one outcome/target-based
semantic row order. The canonicalizer admits at most 32 controls and carries a
dimensioned numerical uncertainty envelope through its full-fit and deletion
conditioning gates; uncertified cases are withheld as
`AMBIGUOUS_CONTROL_BASIS`. Every JLA route has a typed `physical_limit()`
allocation boundary. Literal frequency totals above the exact binary64 integer
range are withheld before graph ranking. Both backends check the final
plug-in-minus-correction row separately before posting it.
`probeorder()` is an optional row-order tie-breaker and need not be unique.
The ordinary canonical order uses observed dense worker, firm, deletion-unit,
target, outcome, and control structure, so tied rows are not an independent
withholding condition.

The development build also contains an experimental compressed engine for the
common large-data design. It is eligible only for JLA, match deletion, no
controls, deletion units wholly contained in coefficient cells, an exactly
registered target-scale partition, fewer than `2^53` physical observations,
the runtime-scoped RNG contract, and a direct peak that fits the caller's
declared memory. `engine(auto)` selects this path when these gates pass.
Otherwise it selects the existing general engine when its own direct peak
fits. A forced
`engine(compressed)` call fails with the precise typed eligibility status;
an over-allocation general fallback fails as
`GENERIC_RESOURCE_ADMISSION_FAILED` instead of beginning probes.

The scale representation keeps three indices distinct: unique worker--firm
coefficient cells, actual deletion units, and exact target-scale strata within
cells. Multiple deletion IDs may share one coefficient cell. Target scales
are compared exactly; they are never merged by a tolerance. Cancellation-
sensitive grouped totals use compensated accumulation. The no-control match
specialization evaluates, without a row-sized deleted-residual vector,

Optimization III stores those identities as exact numeric ranks and keeps one
cell payload with worker-major and firm-major orderings. The scale engine
exposes that payload to the generic solver through a compact callback view,
so no second cell design is retained. The package and its generated CMG
runtime compile with `matalnum off`; the component's checked-in `cmgtest`
target compiles with `matalnum on`.
The public estimator, RNG, routing, tolerance, and complete-residual contracts
are unchanged.

```text
D_g = E_g (m_g^-1 + B_g m_g^-2 - V_g m_g^-3),
K_c = sum_{g -> c} Y_g D_g,
target correction draw = sum_c K_c z_c^2.
```

These quantities inherit the general engine's definitions, conditioning
tests, reciprocal-residual checks, coefficient-one finite-projection formula,
and typed failures. Independent dense and general-engine oracles are the
acceptance boundary; regrouping need not produce bitwise-identical floating-
point output.

Direct random compression is valid only where the numerical kernel consumes
the sum of `F` independent signs, whose exact law is
`2*Binomial(F,1/2)-F`. Leverage uses this law at deletion units. Target probes
use it only within an exact per-copy target-scale stratum at a coefficient
cell; unequal scales remain separate strata. Frequency one is the degenerate
fast case. Observation deletion, a cross-cell deletion unit, or a target
construction that cannot be represented by exact strata uses the general path
or a typed resource/eligibility rejection. A binary64-exact integer total by
itself does not certify the Stata RNG call contract.

Every fit, leverage, and target RHS is certified against the original
frequency-weighted normal equations, not only the graph system:

\[
r_w=b_w-\left[d_w\alpha_w+
  \sum_{c:w_c=w}F_c\gamma_{f_c}\right],\qquad
r_f=b_f-\left[e_f\gamma_f+
  \sum_{c:f_c=f}F_c\alpha_{w_c}\right].
\]

Here `F_c` is a cell's physical-observation mass, `d_w` and `e_f` are its
worker and firm mass sums, and `b_w,b_f` are the original RHS blocks.
The solver works on the full-firm zero-sum quotient, then displays the last
firm at zero and checks that grounded coordinate's original firm equation as
well. The certificate is the combined worker/firm Euclidean residual divided
by the Euclidean norm of the original RHS, or the absolute residual for a
zero RHS. Acceptance requires at most `max(1e-11,10*tolerance())`. A graph or
Schur residual alone is never sufficient.

JLA probes follow a runtime-scoped logical contract. A random atom depends on
the runtime contract, master seed, leverage or target domain, logical probe
index, and canonical identity within the observed IDs. Batch width, memory tiling,
solver route, iteration history, processor count, and phase scheduling do not
alter the atoms. Leverage and target use separate `mt64s` domains. Local K1
evidence selects one fixed-order stateful stream per domain over repeated
per-probe stream resets. The caller's RNG algorithm, selected stream, and
complete state are restored on every exit. Stata 18 and 19 use distinct named
contracts even though the frozen K1 evidence found matching vectors. An
unregistered runtime fails closed for JLA; exact mode needs no production RNG
registration. Production uses streams 1 and 2 and has no legacy 16,383-probe
registry limit. Arbitrary relabeling may change a valid draw, while row order,
batching, and route do not.

For a compressed estimate, the command builds the canonical compressed state,
uses Stata's native disk-backed `preserve`/`clear` lifecycle to release the raw
dataset before peak Mata scratch, frees the large Mata state, and restores the
caller data and exact `e(sample)` semantics. It records memory around
selection, the compression transition, numerical work, and restoration. The
resource forecast separately accounts for a fixed Stata/runtime residency
charge, a row-scaled sorting/compression allocator high-water reserve, raw
Stata data, persistent cell,
deletion-unit and target-stratum state, the CMG hierarchy/factors,
phase-specific matrix RHS scratch, sort/compression temporaries, solve-ahead
storage, output/certificate storage, preservation overhead, and their maximum
overlap. Because Stata may retain freed transition arenas in process RSS, the
compressed numerical forecast also takes the maximum of the live nonsolver
allocation and the complete transition high-water plus numerical-only phase
scratch and solve-ahead storage, then adds the accepted routed solver
allocation. The direct peak must fit `memory_gib()`. The 30-percent memory
headroom and 50-percent wall allowance are advisory planning diagnostics and
do not withhold a command whose direct allocation is safe.

This package provides point estimates and numerical diagnostics only. It does
not post `e(V)` or provide econometric confidence intervals.
The default output reports plug-in, bias-correction, and KSS-corrected target
levels; an additive worker variance + firm variance + 2 x covariance panel;
and shares of target-weighted outcome variance and the corresponding
worker--firm total. A separate descriptive fit panel uses frequency weights
and includes controls. `e(decomposition)` and the retained-sample variance
scalars expose these quantities programmatically without changing the
established scientific result matrices.
KSS-NUMOPT-2 is an internal scalability checkpoint. Its synthetic ladder and
target forecasts do not qualify a production dataset or authorize a full
target run. The covered implementation is GPL-3.0-only. Public release remains
withheld until the documented human license/provenance review is complete.

The Optimization III implementation, source-bound measurements, model
contract, and target assessment are recorded in
`benchmarks/reports/KSS_NUMOPT_2_2026-08-18.md`. Its committed evidence is
aggregate-only; restricted source rows remain on authorized SCC storage.

The retained PREP-RHS-1 optimization, archive-isolated local timing, and
135-job SCC scale qualification are documented in
`docs/PREP_RHS_1_RESULTS_2026-08-19.md`. It delivers a repeatable 2.7--3.1%
complete-command improvement on the local P200 fixture with no scientific or
contract regression; SCC timing is treated only as host-confounded scale and
correctness evidence.

The retained FE-BUF-1 candidate and its source-order-reversed local,
same-host synthetic SCC, and fixed 8,201,888-row CZ18 comparisons are
documented in `docs/FE_BUF_1_RESULTS_2026-08-19.md`. The new solve-local
destination buffers improve exposed repeated-RHS paths and reduce modeled
allocation volume without changing science. Matched synthetic pairs remain
favorable through the separately qualified F15625 endpoint. CZ18 P20 is
command-neutral, so dataset row count alone is not the relevant exposure and
the next optimization target is the command-boundary selection/preparation
pipeline rather than another Schur micro-optimization.

See [PLAN.md](PLAN.md), [the decision record](docs/DECISIONS.md), and
[the source ledger](docs/SOURCE_PROVENANCE.md). The mathematical and numerical
contracts are recorded in [ESTIMATOR_CONTRACT.md](docs/ESTIMATOR_CONTRACT.md),
[BLOCK_CONTROL_DERIVATION.md](docs/BLOCK_CONTROL_DERIVATION.md), and
[NUMERICAL_ARCHITECTURE.md](docs/NUMERICAL_ARCHITECTURE.md).

An internal Mata-only install from a checkout is:

```stata
net install varcomp_kss, from("/absolute/path/to/varcomp_kss/varcomp_kss") replace
```

The tracked internal manifest ships the portable Rust helper, but not plugin
binaries. The macOS artifacts are ignored build products and exist only after
the qualifier has staged them. For its clean-install check, the qualifier
generates a temporary local manifest naming those verified artifacts, then
proves the complete strict Rust lifecycle under native arm64 and, when
available, Rosetta x86_64. This is not Linux, Windows, native-Intel, scale,
production, or public-release qualification. Build instructions are in
[the Stata plugin boundary README](../rust/stata_backend/README.md).

The implemented strict source-local Rust form is:

```stata
varcomp_kss log_wage [fw=freq], worker(person_id) firm(establishment_id) ///
    deletion(match) deletionid(actual_match_id) targetweight(target_mass) ///
    algorithm(jla) engine(compressed) preconditioner(diagonal) batch(8) ///
    backend(rust) rng(counter_v1) probes(200) seed(8675309)
```

A typical large-data call is:

```stata
varcomp_kss log_wage age2 age3 i.year [fw=freq],                 ///
    worker(person_id) firm(analysis_establishment_id)       ///
    deletion(match) deletionid(actual_match_id)             ///
    algorithm(jla) nuisance(joint) targetweight(target_mass) ///
    probes(200) batch(auto) engine(auto)                     ///
    preconditioner(auto) memory_gib(16)                      ///
    seed(8675309)
```

The package runs from Stata/Mata 18 or 19. Python and MATLAB are validation
oracles only and are not runtime dependencies.

Version 0.3.0-dev installs the source-informed GPL CMG component and its KSS
routing adapter. CMG API 7 is an ownership and interface successor to the
numerically qualified API 6 core; it does not change the numerical algorithm.
`preconditioner(auto)` makes a structural decision before the production probe
stream is initialized. Small systems and unavailable CMG setups use diagonal
B1; an eligible, successfully constructed hierarchy uses CMG. There are no
trial solves or projected-work cutoffs. `preconditioner(diagonal)` and
`preconditioner(cmg)` force a route; forced CMG fails closed and never falls
back. `memory_gib()` is any positive direct allocation envelope and defaults
to 4 GiB. `batch(auto)` uses the existing percentage rules as width-selection
heuristics, while the complete direct-peak forecast is the allocation gate.
`wallseconds()` is optional planning metadata.

KSS-PROD-1 and KSS-SCALE-1 evidence remains in the repository, including the
2x failures that exposed the pilot/work gate. KSS-SCALE-1 is owner-stopped;
its fixed SCC ladder is no longer a requirement. The installed command remains
internal candidate software.
See [the KSS-PROD-1 report](benchmarks/reports/KSS_PROD_1_2026-08-16.md) and
[the active plan](PLAN.md).
