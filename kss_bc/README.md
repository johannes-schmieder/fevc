# `kss_bc`: KSS point estimates in Stata/Mata

`kss_bc` is an internal-development Stata/Mata implementation of the
Kline--Saggio--Sølvsten leave-out bias correction for linear two-way
fixed-effect variance decompositions. It is developed beside the existing
`ppml_talo` command. Package unification is deferred.

The command targets worker variance, firm variance, worker--firm covariance,
and the variance of their sum. It supports observation or actual-match
deletion, exact and improved-JLA calculations, joint or fixed-offset controls,
positive integer frequency weights, and separate target weights. Match-mode
headlines use a mover-only fit and target. `stayers(both)` is explicitly
withheld; no observation-level fallback is presented as match robust.
Every randomized calculation with controls, including fixed offset, requires
a deterministic probe-independent full-fit and deletion-rank certificate;
ambiguous designs are withheld for exact verification instead of being passed
by solve error or a favorable leverage draw.
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
Discrete outcomes may supply an explicit complete and unique `probeorder()`
physical-observation key to refine otherwise ambiguous outcome/target ties.
The command never infers such a key; existing calls retain the original
stream and fail closed when their semantic order remains ambiguous.

API 19 also contains an explicitly experimental scale engine for the common
large Separations design.  It is eligible only for JLA, match deletion, no
controls, deletion units wholly contained in coefficient cells, an exactly
registered target-scale partition, fewer than `2^53` physical observations,
at most 16,383 probes, a registered runtime RNG contract, and a passing
pre-RNG memory and wall-time forecast.  `engine(auto)` selects this path when
all gates pass.  Otherwise it selects the existing general engine only when
that engine independently passes its resource forecast.  A forced
`engine(compressed)` call fails with the precise typed eligibility status;
an inadmissible general fallback fails as
`GENERIC_RESOURCE_ADMISSION_FAILED` instead of beginning probes.

The scale representation keeps three indices distinct: unique worker--firm
coefficient cells, actual deletion units, and exact target-scale strata within
cells. Multiple deletion IDs may share one coefficient cell. Target scales
are compared exactly; they are never merged by a tolerance. Cancellation-
sensitive grouped totals use compensated accumulation. The no-control match
specialization evaluates, without a row-sized deleted-residual vector,

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

API 19 probes follow a versioned logical contract. A random atom depends only
on the contract version, master seed, leverage or target domain, logical probe
index, and canonical semantic atom identity/order. Batch width, memory tiling,
solver route, iteration history, processor count, and phase scheduling do not
alter the atoms. Leverage and target use separate `mt64s` domains. Local K1
evidence selects one fixed-order stateful stream per domain over repeated
per-probe stream resets. The caller's RNG algorithm, selected stream, and
complete state are restored on every exit. Each supported Stata runtime needs
a registered golden vector; an unregistered runtime fails closed. Source-bound
K1 job 7201105 established identical Stata 18 and 19 golden vectors, so both
runtimes use contract `KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19`.

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
overlap. Admission includes 25--30 percent memory and 50 percent wall-time
headroom and must remain within 56 GiB and 12 hours.

This package provides point estimates and numerical diagnostics only. It does
not post `e(V)` or provide econometric confidence intervals. API 19 is an
experimental, locally tested scale candidate; SCC scale qualification is
pending. It has no public release license and must not be published or
redistributed.

See [PLAN.md](PLAN.md), [the decision record](docs/DECISIONS.md), and
[the source ledger](docs/SOURCE_PROVENANCE.md). The mathematical and numerical
contracts are recorded in [ESTIMATOR_CONTRACT.md](docs/ESTIMATOR_CONTRACT.md),
[BLOCK_CONTROL_DERIVATION.md](docs/BLOCK_CONTROL_DERIVATION.md), and
[NUMERICAL_ARCHITECTURE.md](docs/NUMERICAL_ARCHITECTURE.md).

An internal install from a checkout is:

```stata
net install kss_bc, from("/absolute/path/to/ppml-variance/kss_bc") replace
```

A production-style call is:

```stata
kss_bc log_wage age2 age3 i.year [fw=freq],                 ///
    worker(person_id) firm(analysis_establishment_id)       ///
    deletion(match) deletionid(actual_match_id)             ///
    algorithm(jla) nuisance(joint) targetweight(target_mass) ///
    probes(200) batch(auto) engine(auto)                     ///
    preconditioner(auto) memory_gib(56) wallseconds(43200)   ///
    seed(8675309)
```

The package runs from Stata/Mata 18 or 19. Python and MATLAB are validation
oracles only and are not runtime dependencies.

Version 0.2.0-dev installs the clean-room CMG core and its KSS routing adapter.
`preconditioner(auto)` makes a deterministic preflight and pilot decision
before the production probe stream is initialized. `preconditioner(diagonal)`
and `preconditioner(cmg)` force a route; forced CMG fails closed and never
falls back. Automatic CMG setup or pilot ineligibility may fall back only to
the same diagonal lockstep solver and is returned with its typed original
status and reason. `memory_gib()` declares a 1--56 GiB allocation envelope;
the default is 4 GiB. Probe scratch is hard-bounded to 35 percent, and the
persistent FE design plus maximum concurrent solver allocation is hard-bounded
to the other 65 percent before routing or estimator RNG. `batch(auto)`
deterministically selects an evidence-backed canonical width from 8 through 64
after retained dimensions are known and before solver routing or random
probes. Its processor cap is 32 through four processors and 64 with eight or
more, additionally bounded by probe count and 35 percent of the declared
memory envelope. Explicit positive integer batches, including 128, remain
supported when their forecast fits.

KSS-PROD-1 candidate `5e2687c6` passes CZ24, CZ25, and the full 601-RHS
CZ18 estimator, but it is not production-qualified. Three independent
two-times-CZ18 P20 calibrations all withheld at the typed pre-RNG automatic
route gate, so the registered validator did not admit a full P200 stress run.
The API 19 scale work supersedes that route only through the experimental
single-process compressed candidate described above. Its local algebra, RNG,
lifecycle, resource, fixture, and command gates are implemented. Final-source
CZ24/CZ25 resource calibration passes; CZ18 and 2x/4x SCC qualification remain
pending. All scale and maintained-
MATLAB performance forecasts are hypotheses until replaced by measured,
source-bound runs. The installed command remains internal candidate software.
See [the KSS-PROD-1 report](benchmarks/reports/KSS_PROD_1_2026-08-16.md) and
[the active plan](PLAN.md).
