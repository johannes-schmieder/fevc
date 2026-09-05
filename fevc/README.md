# `fevc`: KSS point estimates in Stata

`fevc` is a prerelease Stata 18/19 implementation of the
Kline--Saggio--Sølvsten leave-out bias correction for linear two-way
fixed-effect variance decompositions. Point estimates and numerical
diagnostics remain the default. Version `0.5.0-rc.1` adds opt-in exact Mata
and supported explicit structured Rust/JLA observation-deletion and
fixed-offset match-deletion component inference, plus fixed-effect projection
inference with exact Mata and explicit
sparse Rust/JLA observation-or-match block covariance.

`fevc` is the only public command and package identity. No predecessor
alias is installed.

## Estimator surface

The command targets:

- worker-effect variance;
- firm-effect variance;
- worker--firm covariance; and
- variance of the worker-plus-firm sum.

The established Mata implementation supports exact and improved-JLA
calculation, match or observation deletion, joint or fixed-offset controls,
positive integer frequency weights, separate target weights, explicit
deletion IDs, structural B1/CMG routing, and the registered Stata RNG
contracts. Match deletion defaults to the current MATLAB population:
retained movers plus eligible attached one-firm stayers in one pooled fit and
target. Movers use match deletion; stayers use physical-observation deletion
and are never presented as match-cluster robust. `stayers(movers)` is the
explicit mover-only opt-out. Mata and Rust implement the combined convention
for exact and generic JLA through the versioned augmentation lifecycle.

The package preserves coefficient cells, deletion units, and exact
target-scale strata as separate objects. It never merges target scales by a
tolerance. Accepted calculations must pass identification, rank,
complete-original-system residual, accounting, finite-output, direct-memory,
and caller-state restoration gates.

## Backend routing

`backend()` and `rng()` are routing and consent surfaces.

| Request | Selected runtime |
|---|---|
| omitted `backend()` | Rust when the effective request passes preflight; otherwise preflight-only Mata fallback |
| `backend(mata)` | Mata |
| `backend(auto)` | same Rust-preferred automatic route as omission |
| `backend(rust)` | strict native route, subject to a compositional capability receipt |

Omitted `rng()` means `rng(auto)`: Counter-V1 on Rust and Stata RNG on Mata.
Explicit `rng(counter_v1)` pins strict Rust behavior; explicit `rng(stata)`
selects Mata and conflicts with `backend(rust)`. The omitted algorithm is
MATLAB-like JLA with 200 probes. Explicit `algorithm(auto)` retains the
exact-small/JLA-large native plan.

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

The `0.5.0-rc.1` milestone carries forward qualified effective-option admission,
Rust-preferred automatic routing with preflight-only Mata fallback, automatic
JLA selection, semantic `probeorder()` tie breaking, and exact plus generic-JLA
`stayers(both)` parity. The scalar direct hybrid-Laplacian route is
vendored and identified as `CMG_FULL_V2`. On qualified macOS and Linux builds,
the no-control match/joint/movers JLA cell with automatic engine,
preconditioner, and batch selection plus explicit `probeorder()` is available
through strict Rust and automatic backend/RNG routing. Other requests retain
their existing routes; Mata remains explicit and no selected native failure
falls back after preparation or estimator RNG. Windows remains deferred. The
generated gap ledger is
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md). A green quick suite is
not full plugin qualification.

## Opt-in inference

Inference is explicit and capability-gated. Omitting `inferencemodel()` keeps
the independent Mata exact target-specific procedure, with observation
deletion, movers, and unit frequency weights; omitted or automatic algorithm
selection resolves to exact for that request. The supported scalable component
capabilities instead require an explicit
`inferencemodel(structured_common|structured_leverage)` together with Rust
generic JLA, Counter-V1, observation deletion, movers, joint nuisance handling,
unit frequency, and an explicit diagonal or CMG solver. A separate explicit
match route requires `deletion(match) nuisance(fixedoffset) stayers(movers)
engine(generic)` with the same Rust/JLA/Counter/model/solver options. It permits
positive integer frequencies as regression mass, not independent matches.
Joint-control match inference and the stayer hybrid remain withheld.
Fixed-effect `project()` is separate and additionally
has a strict sparse Rust route under its documented block-covariance contract.

```stata
fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(observation) inference(highrank)

fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(observation) inference(q1) level(95)

fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(observation) stayers(movers) nuisance(joint)      ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) inference(highrank)               ///
    inferencemodel(structured_common)

fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(observation) stayers(movers) nuisance(joint)      ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(cmg) inference(q1)                           ///
    inferencemodel(structured_common)

fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(observation) project(education experience)        ///
    projecteffect(firm)

fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(observation) project(education experience)        ///
    projecteffect(firm) backend(rust) rng(counter_v1)           ///
    algorithm(jla) engine(generic) preconditioner(cmg)
```

The named structured models impose additional conditional-variance
assumptions; they are not unrestricted-heteroskedastic KSS variance-product
inference. Severe omitted variance drivers can invalidate standard errors and
intervals without changing component point estimates. `q=0` requires diffuse
kernel and influence contributions. `q=1` treats one leading mode explicitly
and requires a diffuse remainder; its uniform asymptotic guarantee is at least
nominal and can be modestly conservative. The target-specific spectral and
influence diagnostics remain necessary: neither a user request nor successful
execution proves the required asymptotic condition, and no automatic `q`
selection is performed.

The projection CMG route is the planned generic model preconditioner. It
shares one hierarchy between the full and fixed-effect solvers and supports
the same controls and positive integer frequency weights as generic JLA. It is
distinct from `CMG_FULL_V2`, whose public cell remains the specialized
no-control match-deletion point-estimation route. Automatic solver selection
is not admitted for `project()`; callers must request `diagonal` or `cmg`.

For the typical whole-match use case:

```stata
fevc log_wage i.year, worker(person_id) firm(establishment_id) ///
    deletion(match) nuisance(fixedoffset) stayers(movers)       ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) inference(highrank)               ///
    inferencemodel(structured_common)
estat diagnostics
```

Use `inference(q1)` only for a target with one dominant mode and a diffuse
remainder. This is **fixed-offset approximate match inference, ignoring
nuisance-control estimation uncertainty**. It assumes independent matches and
a correctly specified structured aggregate-match variance model; dependence
within a match is absorbed by that aggregate variance. Few controls do not
guarantee negligible omitted uncertainty. The independent match q0/q1
confirmations pass in their declared regimes. Corrected observation q1 retains
an unresolved SE-calibration failure; its old confirmation is not relabeled
as passed. See the [RC scope decision](docs/fixed_offset_match_interface_v1.json).

Only accepted component inference posts the four-target `e(V)`. Projection
coefficients and covariances are stored separately under `e(projection_*)`.
Stata's standard `lincom` works on the posted component `e(b)`/`e(V)`; the
maintained MATLAB routine `lincom_KSS` instead corresponds to FEVC
`project()` and is not the same postestimation operation.
The implementation, formulas, diagnostics, and interpretation boundary are in
[`docs/INFERENCE.md`](docs/INFERENCE.md).

## Postestimation

After a successful `fevc` call, the default display is intentionally compact.
Use the additive `estat` views when a reproducible audit needs the full stored
state:

```stata
estat decomposition, full
estat sample
estat computation
estat diagnostics
```

`estat decomposition, full` reports the plug-in, estimated-bias, corrected,
and outcome-variance-share accounting. `estat sample` reports retained mover
and eligible-stayer counts. `estat computation` reports the effective backend,
algorithm, solver, RNG, and resource plan. `estat diagnostics` reports
convergence, residual, identification, and inference diagnostics that apply to
the selected result family. Unsupported views fail explicitly rather than
reconstructing state from display text.

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
net install fevc, from("/absolute/path/to/fevc/fevc") replace
```

For a development installation made before the runtime filenames were
normalized to `fevc`, first run `ado uninstall fevc` and then install again.
This one-time clean reinstall removes obsolete files that `replace` may leave
on the PLUS path. Run `discard` after installation, or restart Stata, so no
program or Mata definition from the earlier build remains cached in memory.

The tracked package manifest ships portable Ado/Mata source and Rust boundary
helpers, not plugin binaries. The macOS qualifier builds, audits, signs, stages,
and clean-installs temporary native artifacts. Other platforms require their
own qualification.

The repository also has a deterministic, non-publishing constructor for the
portable source package. Check it without writing an artifact, or build from a
clean committed checkout into a directory outside the repository:

```bash
./.venv/bin/python fevc/tools/build_release_artifact.py --check
./.venv/bin/python fevc/tools/build_release_artifact.py \
    --output-dir /private/tmp/fevc-release-artifact
```

The resulting archive contains one `fevc/` directory with `stata.toc`,
`fevc.pkg`, and exactly the files listed by the package manifest. Extract it and
point `net install` at that directory. The adjacent JSON receipt binds the
archive hash, source commit, version, and every packaged file. This procedure
does not build native plugins, tag, publish, or create a public release.

The owner-selected `0.5.0-rc.1` installation payload is the **complete native
package**, not that portable-only artifact. Preparation and private platform
qualification are underway. `tools/build_native_release.py` generates its
installation manifest with all Mac arm64/x86_64/universal, Linux x86_64 and
Windows x86_64 plugins, requiring source-bound passing evidence for every
file. See [RC binary preparation](docs/RC_BINARY_PAYLOAD.md). Extract the
completed native archive and point `net install` at its `fevc/` directory;
restart Stata after replacing a previously loaded plugin. No completed or
publicly distributed native RC is claimed until the final platform and
exact-artifact installation gates pass.

A normal Rust-preferred call is:

```stata
fevc log_wage age2 age3 i.year [fw=freq],              ///
    worker(person_id) firm(establishment_id)                   ///
    deletion(match) deletionid(actual_match_id)                ///
    nuisance(joint) targetweight(target_mass)                  ///
    memory_gib(16) seed(8675309)
```

An explicit strict Rust JLA call is:

```stata
fevc log_wage [fw=freq],                                ///
    worker(person_id) firm(establishment_id)                   ///
    deletion(match) deletionid(actual_match_id)                ///
    targetweight(target_mass) algorithm(jla) engine(auto)      ///
    preconditioner(auto) batch(auto) backend(rust)             ///
    rng(counter_v1) probes(200) seed(8675309)
```

The qualified full-CMG cell omits weights, controls, target weights,
`deletionid()`, and supplies a stable observation key:

```stata
fevc log_wage, worker(person_id) firm(establishment_id)       ///
    deletion(match) nuisance(joint) stayers(movers)            ///
    probeorder(observation_key) algorithm(jla) engine(auto)     ///
    preconditioner(auto) batch(auto) backend(auto) rng(auto)    ///
    probes(200) seed(8675309)
```

On a qualified macOS/Linux runtime this posts
`e(cmg_backend) == "CMG_FULL_V2"`. Its phase defaults are `1e-10` for the fit
and deterministic outcome solve and `1e-6` for randomized probes; an explicit
`tolerance()` overrides both.

Append `backend(mata) rng(stata)` to select the portable Mata implementation
explicitly.

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

The historical alpha full-CMG decision, exact platform summaries, compact
receipts, and CMG-style benchmark PDF are under
[`benchmarks/full_cmg_production/`](benchmarks/full_cmg_production/). Runtime
source `4b6874e` is 1.397x matched MATLAB on the registered macOS headline and
1.731x MATLAB on SCC's fixed CZ18 case. These pass the selected alpha gate but
do not establish the longer-run 2x target as achieved.

## License and release status

The package-owned CMG implementation and a distributed package containing it
are GPL-3.0-only. The documented human package-boundary and provenance review
was completed on 29 August 2026. The source is prepared for public development,
but no public package release, tag, or native binary distribution has yet been
issued.
See
[`../CODE_LICENSE.md`](../CODE_LICENSE.md) and
[`docs/SOURCE_PROVENANCE.md`](docs/SOURCE_PROVENANCE.md).
