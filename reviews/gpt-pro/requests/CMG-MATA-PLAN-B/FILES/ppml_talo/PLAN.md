# PPML TALO for Stata/Mata: living implementation plan

Last updated: 2026-08-13
Branch: `software/ppml-talo-st11-independent-deletion`
Package root: `ppml_talo/`
Current milestone: **ST11E — IN PROGRESS (comparison closing; all-CZ identification gate failed)**
Target first release: `0.1.0`

## 1. Objective and non-negotiable scope

Build a production-quality, point-estimate-only implementation of the PPML
targeted analytic leave-out correction in pure Stata/Mata. It must support
AKM-style worker–firm decompositions with millions of rows without constructing
an \(n\times n\) matrix. The package will fit PPML through `ppmlhdfe`, compute
quadratic plug-in targets from saved fixed effects, and compute the complete
nonlinear TALO correction with matrix-free information solves.

The package does **not** provide econometric standard errors. Randomized
algorithms must report numerical Monte Carlo uncertainty, which is not a
sampling standard error.

Observation-level deletion remains the default production contract. ST10
reopens the experimental cluster extension for frequency-weighted deletion of
an entire supplied worker--firm match. The Separations convention treats those
match cells as independent even when an original worker contributes to more
than one cell. That convention is owner-approved for descriptive evaluation;
the transfer from the observation-level theorem remains unverified. The
Separations project receives a checksum-pinned internal experimental export
after the standalone package passes the registered gates.

For this plan, “usable for Separations” has five independent meanings:

1. the requested economic estimand and PPML specification match the command;
2. the retained data rows match the estimator's deletion and independence
   unit;
3. the realized worker--firm and positive-outcome graphs pass identification,
   face, leverage, and conditioning gates;
4. the exact production settings pass numerical, memory, runtime, and
   portability qualification on the target system; and
5. Separations installs a versioned release and records a reproducible,
   non-sensitive result manifest.

A fast synthetic benchmark establishes only part of item 4. It cannot resolve
the estimand, independence, support, or theorem-applicability questions.

## 2. Frozen statistical contract

For independent rows, let

\[
 H=X'\widehat W X,\quad A=H^+,\quad C=XAX',\quad
 s_Q=XAQ\widehat\beta,
\]

and let the complete observation-space half-Hessian be

\[
 G_Q=XAQAX'-C\operatorname{diag}(\widehat\mu\circ s_Q)C.
\]

With leverage \(h_i=\widehat\mu_i C_{ii}\), the analytic deleted prediction is

\[
 \widetilde\eta_{i,-i}=\widehat\eta_i-
 \frac{h_i}{1-h_i}\frac{Y_i-\widehat\mu_i}{\widehat\mu_i},\qquad
 \widetilde\mu_{i,-i}=\exp(\widetilde\eta_{i,-i}).
\]

The implemented point estimator is

\[
 \widehat\theta_Q^{\mathrm{TALO}}
 =\widehat\beta'Q\widehat\beta-
 \sum_i G_{Q,ii}Y_i(Y_i-\widetilde\mu_{i,-i}).
\]

The observation-deletion implementation evaluates this statistic without
forming \(A\), \(C\), or \(G_Q\). The experimental whole-match path uses a
different bounded architecture: it never forms an observation-space matrix,
but it does form exact dense coefficient-space full and positive information
matrices. With at most 200 worker bins, 200 firm bins, year effects, and
the registered controls, this keeps the dense dimension below the current
`dense_limit(3000)`. It computes every deletion-block spectrum exactly. The
final linear trace contraction can use randomized observation-space probes or
deterministically enumerate the coefficient basis. The latter uses exactly
the quotient dimension in trace directions and has zero randomized MCSE. This
design trades coefficient-space cubic work for a no-false-accept spectral gate
and must be qualified on the realized CZ designs before it is called usable at
scale.

## 3. Public interface target

Primary form after ST10C:

```stata
ppmltalo depvar [varlist] [fw=frequency] [if] [in],      ///
    worker(worker_id) firm(firm_id)                      ///
    [absorb(id3) deletion(observation|match)              ///
     exposure(varname) offset(varname)                   ///
     targetweight(varname) leverageprobes(#) probes(#)   ///
     batch(#) seed(#) tolerance(#) score_tolerance(#)    ///
     maxiter(#)                                          ///
     leverage_limit(#) exact exact_limit(#)               ///
     traceexact                                           ///
     block_limit(#) meanmax(#) meanexceedshare(#)         ///
     scratch(path) keepsingletons allowdrops nodisplay]
```

Version 0.1 always returns worker variance, firm variance, their covariance,
and the variance of their sum. Frequency weights are accepted only with
`deletion(match)`: the likelihood weight is the number of exact duplicate
person-period rows represented by a stored row, while the entire supplied
worker--firm pair is deleted as one block. Without `targetweight()`, target
mass is the frequency weight (or one for unweighted data). With
`targetweight()`, the supplied value is total stored-row target mass and is
never multiplied by the frequency weight. Frequency-weighted observation
deletion remains rejected because its deletion semantics are ambiguous.

Advanced target form (post-0.1 unless completed safely):

```stata
qspec("wf=variance(worker+firm); wt=covariance(worker,timefe)")
```

The command returns plug-in values, corrections, corrected values, numerical
diagnostics, identification diagnostics, and typed status codes in `e()`. It
must never imply that its probe error is an econometric standard error.

## 4. Architecture

```text
ppmltalo.ado                 command parsing, ppmlhdfe orchestration, e() API
ppmltalo.mata                production operators, solvers, corrections
src/ppmltalo_core.mata       development source included by ppmltalo.mata
src/ppmltalo_dense.mata      small-problem oracle, never production
src/ppmltalo_graph.mata      connectivity and deletion-identification checks
src/ppmltalo_rng.mata        deterministic probe generation and batching
tests/                       Stata-first unit/oracle/MC/integration/scale tests
theory/                      estimator and numerical-integration companion
benchmarks/                  reproducible runtime/memory harnesses
integration/separations/     pinned-export instructions and smoke test
release/                     versioned package assembly and checksums
```

The current candidate keeps the dense oracle and matrix-free implementation in
the installable `ppmltalo.mata` so clean-install testing exercises the exact
runtime artifact. Splitting development sources under `src/` is deferred until
the ST1 algebra stabilizes; any split must generate or assemble that artifact
reproducibly and cannot introduce a runtime compiler dependency.

Observation-mode production design actions operate on dense integer group codes. A vector
\(v\) in coefficient coordinates is mapped to rows by summing its FE-block
entries; the transpose action uses `panelsum()` after sorting by reusable group
codes. Weighted information actions use
\(v\mapsto X'(\widehat\mu\circ Xv)\). Solves use preconditioned conjugate
gradients on an identified quotient. The production path first eliminates the
full worker-indicator block by an exact Schur complement, applies PCG only to
the remaining firm block, then reconstructs worker coefficients. Every solve
must pass a freshly recomputed double-precision full-system relative-residual
check. This operational check is not a rigorous exact-real residual or
forward-error certificate. Additional nuisance FE blocks remain disabled in
observation mode until the multiway quotient-rank gate closes.

Match mode builds an identified dense quotient with worker FE, firm FE, one
nuisance FE, and numeric controls. It accumulates \(p\times p\) information
matrices directly from stored rows, inverts the full and positive-outcome
matrices after explicit residual checks, and evaluates each worker--firm
deletion through exact coefficient-local Woodbury kernels. The engine stores
neither an \(n\times n\) matrix nor an \(n\times p\) design. The design is
acceptable for the locked 100- and 200-bin configurations only if SCC
timing and measured RSS pass ST10E--ST10F. In `traceexact` mode it streams
batches of coefficient standard-basis vectors through the same implicit
design and block-kernel actions. This adds \(p\) deterministic directions and
zero trace MCSE without allocating either forbidden observation-space object.

## 5. Milestones and acceptance gates

### P0 — synchronize prior work — COMPLETE

Evidence (2026-08-11):

- Full repository check passed before synchronization.
- Prior authorized changes committed as `d9a975f` and `04fa427`.
- `main` and `origin/main` synchronized at `04fa427`.
- Development branch `software/ppml-talo-stata` created from that commit.

### ST0 — governance, scaffold, and reproducibility baseline — COMPLETE

Deliverables:

- Local `AGENTS.md`, this living plan, package skeleton, README, and changelog.
- Environment probe recording Stata flavor/version and dependency versions.
- Minimal installer metadata and a smoke test that loads Mata sources.
- Repository-scope declaration: only `ppml_talo/**` changes.

Exit gate:

- Clean install into a temporary `PLUS` directory.
- `tests/run_all.do, quick` exits zero on Stata/MP 18.
- Plan records exact evidence and remaining limitations.

### ST1 — estimator algebra and cluster research gate — COMPLETE

Deliverables:

- `theory/ppmltalo_numerics.tex` derives the independent-row statistic and
  every matrix-free identity used by code.
- Dimension, sign, normalization, and target-linearity audit.
- Derivation of any proposed cluster-delete extension, clearly labeled
  experimental.
- Exact small matrices compare analytic expressions to finite differences and
  exact refits.
- Two independent GPT Pro reviews and one local adjudication for each critical
  new derivation.

Exit gate:

- All accepted equations have executable checks.
- No open critical objection in the adjudication.
- Observation-level contract may advance to `formula_verified`.
- Cluster functionality remains disabled unless its own gate passes; named
  human review is still required for `independently_checked`.

### ST2 — dense Mata oracle — COMPLETE

Deliverables:

- Small-design construction of \(X,H,A,C,G_Q\).
- Exact observation-level TALO for arbitrary symmetric \(Q\).
- Exact leave-one-out PPML diagnostic for tiny samples.
- Typed failures for rank, face, leverage, overflow, and nonconvergence.

Exit gate:

- Dense Mata agrees with high-precision committed fixtures and existing
  independent Python/JAX fixtures at registered tolerances.
- Outcome-scale, quotient, row-order, label, and accounting invariances pass.

### ST3 — matrix-free HDFE operators and solvers — COMPLETE

Deliverables:

- Reusable FE encoding and row/coefficient actions.
- Diagonal/Jacobi preconditioner and quotient-safe PCG.
- Multi-right-hand-side batching where Mata memory permits.
- Robust recomputed-residual checks, iteration limits, stagnation and
  breakdown statuses.

Exit gate:

- Operators match dense matrices on exhaustive small designs.
- Solves match dense Cholesky/pseudoinverse actions.
- No production path allocates an \(n\times n\) object.

### ST4 — leverage and Hessian-diagonal engine — COMPLETE

Deliverables:

- Deterministic exact mode for small problems.
- Batched Hutchinson/Rademacher mode for large problems.
- Common random numbers across accounting targets.
- Online mean/variance accumulation and stopping diagnostics.
- Complete nonlinear curvature term, not the linear-model-only term.

Exit gate:

- Randomized estimates are unbiased in fixed-design replication tests.
- Fixed-design calibration of the reported rowwise and trace MCSE meets the
  registered simulation criterion; accounting identities hold probe by probe.
  No simultaneous confidence bound is claimed for the leverage gate.
- Seed reproducibility and batch-size invariance pass.

### ST5 — production `ppmltalo` command — COMPLETE

Deliverables:

- `ppmlhdfe` fit orchestration with named saved FE variables.
- Worker, firm, covariance, and total targets with weighted centering.
- `if`/`in`, exposure, offsets, target-population weights, and singleton
  policy. Frequency weights remain gated on an explicit collapsed-row deletion
  contract.
- Stable `e()` contract, progress output, cleanup on error, and no data damage.

Exit gate:

- Help examples run from a clean session.
- Results match dense oracle on small samples and remain invariant to FE base
  normalization.
- Failure statuses are actionable and never silently replaced by ridge output.

### ST6 — identification and failure diagnostics — COMPLETE

Deliverables:

- Connected-component and quotient checks for every requested target.
- Leave-one-out support/deletion checks appropriate to the sampling unit.
- Separation/interior-fit, leverage, finite-index, and solver gates.
- Machine-readable withholding reason plus human-readable remediation.

Exit gate:

- Adversarial disconnected, bridge, singleton, zero-margin, separated, and
  near-singular fixtures produce the registered status.

### ST7 — comprehensive statistical and numerical test suite — COMPLETE

Suites:

- unit: grouped reductions, operators, RNG, PCG, centering, target algebra;
- oracle: dense versus matrix-free and exact LO diagnostics;
- identification: graph/quotient/face/deletion failures;
- Monte Carlo: plug-in/TALO bias, consistency sequences, accounting,
  randomized numerical error;
- integration: installed command, replay, saved estimates, Separations-shaped
  data;
- regression: every previously minimized failing seed;
- portability: Stata/MP 18 and 19.

Exit gate:

- Quick suite completes locally; full suite completes on the cluster.
- Every stochastic test records seed and uses a prespecified tolerance.
- Tests separate finite numerical evidence from asymptotic claims.

### ST8 — scale and performance qualification — PAUSED

Benchmark ladder: 10k, 100k, 1m, 5m, and 10m observations with AKM-like degree
distributions in the supported pure two-way worker--firm model. Stable,
positive-skewed, and Poisson-with-zeros outcomes are separate profiles.
Three-FE inputs remain a typed negative-scope test until multiway rank
certification exists; they are not presented as a supported scale result.

Registered acceptance target on the SCC production profile:

- Stata/MP 19, four licensed/active processors, four scheduler slots, and a
  64 GB allocation (`mem_per_core=16G`);
- 10m rows complete within 12 hours;
- estimated peak resident memory below 56 GB;
- no hidden dense matrix scaling in observations or FE count;
- deterministic result under fixed seed and settings.

The provenance-bound local Stata/MP 18 and SCC Stata/MP 19 ladders have passed
through ten million rows at the release probe settings, including measured
operating-system RSS and SCC scheduler evidence. On the registered four-core
SCC envelope, 10m completed in 4,114 Stata seconds with 14.109 decimal GB peak
RSS. This closes the synthetic platform-scale subgate, not realized-network
conditioning or the provisional 400+400 target-data configuration.

### ST9 — documentation, packaging, and release — PENDING

Deliverables:

- `ppmltalo.sthlp` with syntax, returned results, examples, failure catalog,
  assumptions, and interpretation.
- Compiled `theory/ppmltalo_numerics.pdf` and source.
- `.pkg`, `stata.toc`, semantic version, changelog, license decision, release
  manifest, checksums, and reproducibility report.

Exit gate:

- Fresh temporary install passes quick tests and help examples.
- Documentation equations match code and reviewed theory packet.
- Release archive hash and source commit are recorded.

### ST10 — pinned Separations integration — IN PROGRESS

ST10 is a gated production-readiness program, not a single adapter call. Its
work-package gates close in the order below. Candidate code and tests may be
developed ahead of an SCC gate to avoid idle time, but no later package may be
promoted to complete before its dependencies close. A failed gate is retained
as evidence and blocks downstream production use unless the owner authorizes a
separately scoped repair.

```text
ST10A model contract -> ST10B data preflight -> ST10C scope decision
        -> ST10D adapter -> ST10E realized-data qualification
        -> ST10F target-platform qualification -> ST10G pinned release
        -> ST10H interpretation review and go/no-go

ST7 portability ------------------------^          |
ST8 SCC performance --------------------^          |
ST9 release documentation -------------------------^
```

#### ST10A — owner-signed estimand and data contract — COMPLETE

Create `integration/separations/MODEL_CONTRACT.md` from an owner-approved
mapping. It must state:

- the outcome, its support, units, and economic interpretation;
- whether the outcome is a count, binary event, duration, rate numerator, or
  another nonnegative quantity;
- any exposure or offset, its units, positivity rule, and zero-exposure policy;
- worker and firm identifiers and the intended meaning of each fixed effect;
- every required time effect, other absorbed effect, continuous regressor,
  interaction, or restriction;
- the population functional: log-mean worker variance, firm variance,
  covariance, and total, or a different target;
- row-, worker-, spell-, match-, firm-, or exposure-weighted target population;
- the physical row unit, whether rows are frequency-collapsed, and the unit
  whose deletion defines the correction;
- the conditional independence or dependence convention, including repeated
  observations within worker, firm, match, time, or geography;
- the analysis universe, connected-set rule, mover rule, singleton rule,
  missing-data rule, and all upstream sample restrictions;
- anticipated PPML separation exclusions and whether any sample change is
  substantively authorized;
- whether the project needs only corrected point estimates or also needs
  econometric standard errors or confidence intervals.

Compatibility decision:

- **Supported route:** exactly two worker/firm FE blocks, one retained row per
  independent deletion unit, no frequency likelihood weights, supported
  exposure/offset, supported quadratic targets, and point estimates only.
- **Narrower owner-approved route:** the owner changes the project
  specification or population before seeing TALO results and documents why the
  supported command still answers the intended question.
- **Extension route:** any third FE, continuous regressor, custom target,
  frequency-collapsed deletion unit, cluster deletion, or inference request
  opens a separate mathematical and software milestone before ST10 continues.
- **Blocked route:** if none of the preceding routes preserves the intended
  question, record `BLOCKED_MODEL_SCOPE`; do not run a misleading substitute.

An estimated nuisance effect may not be inserted as a fixed offset merely to
avoid implementing its contribution to the information matrix and correction.
Any new cluster deletion formula, target-Hessian identity, or material change
to the statistic triggers the two-review GPT Pro gate and named-human review
requirements in `AGENTS.md`.

Exit gate:

- The owner and implementing econometrician sign the model contract.
- Every requested feature is classified as supported, deferred, or blocking.
- The target weighting and deletion unit are fixed before project results are
  inspected.

#### ST10B — privacy-safe whole-data preflight — IN PROGRESS

Develop a read-only Stata preflight that operates inside the authorized
Separations environment and exports aggregate diagnostics only. Restricted
rows and identifiers remain outside this repository. The committed artifact
is the schema and code; the project records the restricted report's checksum
and controlled location.

Required diagnostics:

- requested and complete-case row counts, file width, storage types, missing
  values, outcome minimum/maximum, zero share, and exposure validity;
- worker, firm, and worker--firm cell counts; repeated-cell distribution;
- worker and firm observation-degree quantiles and maxima;
- positive-outcome degree quantiles, zero-positive workers/firms, and positive
  margin counts;
- mover shares, firms per worker, workers per firm, and parallel-edge counts;
- full-graph connected components and row-edge bridges;
- positive-outcome graph coverage, connected components, and row-edge bridges;
- the exact number and reason of any `ppmlhdfe` separation or singleton drops,
  fit iterations, retained sample, and score diagnostic;
- a structural memory estimate using the actual row and FE counts, plus the
  memory occupied by retained Stata variables;
- graph-preserving pilot-sample definitions for later qualification. A random
  row subset is not an acceptable proxy when it changes connectivity.

Preflight decisions must distinguish:

- genuine nonexistence or nonidentification;
- conservative rejection by the implemented positive-support certificate;
- poor numerical conditioning despite identified full and deleted fits; and
- an upstream data-construction or model-specification error.

Exit gate:

- The exact production sample construction is versioned and frozen.
- No unapproved PPML row drop remains.
- The registered rich quotient and every supplied whole-match deletion pass
  both the exact full-information and positive-information block gates, or
  ST10 is blocked with the exact aggregate reason.
- The preliminary resource calculation fits the proposed allocation with a
  prespecified safety margin.

#### ST10C — conditional scope-gap resolution — CANDIDATE IMPLEMENTED; GATE PENDING

This work package is marked `NOT_REQUIRED` when ST10A and ST10B fit the current
contract. Otherwise, resolve only the authorized gap on its own branch and
with its own tests and review gate.

The locked contract requires the extension route. The current candidate adds
integer frequency likelihood weights, whole supplied-match deletion, one
nuisance FE, numeric controls, the registered target-mass semantics, exact
full and positive block spectra, and a dense coefficient-space engine bounded
by `dense_limit()`. Two independent Pro reviews accepted the finite formulas
with repairs. The local adjudication records those repairs and explicitly
does not treat the post-packet production implementation as Pro-reviewed.

Potential routes include:

- expose already-supported `exposure()` or `offset()` through the adapter;
- make target weighting and numerical settings configuration-driven;
- refine the conservative deleted-face certificate without admitting unsafe
  samples;
- implement a multiway quotient and solver only after a scalable rank
  certificate exists;
- implement cluster deletion only after its deleted-face, block-memory,
  numerical, scale, asymptotic-transfer, and human-review gates close;
- implement collapsed-row or frequency-weight behavior only after the
  sampling/deletion unit is mathematically fixed;
- implement a custom quadratic target only after deriving and testing its
  target action and complete half-Hessian contribution;
- open a separate inference project if standard errors are required.

Exit gate:

- The gap is either closed with formula, oracle, failure, scale, documentation,
  and review evidence, or ST10 remains blocked.
- No unsupported feature is emulated through preprocessing that changes the
  intended estimator without an explicit owner decision.

#### ST10D — production adapter and result contract — CANDIDATE IMPLEMENTED; GATE PENDING

Replace the fixed demonstration harness with a configuration-driven adapter
while preserving a minimal stable interface. Deliverables:

- an exact no-exposure/no-offset assertion for this configuration;
- explicit target-population weight and sample-restriction inputs;
- recorded correction probes, batch size, random seed, score tolerance, rank
  tolerance, block limit, dense limit, and probability-gate settings rather
  than undocumented hard-coded choices;
- startup checks for Stata flavor/version and exact `ppmlhdfe`, `reghdfe`, and
  `ftools` versions;
- successful and failed output rows under one versioned schema, with point
  estimates absent after withholding;
- plugin, correction, TALO, trace MCSE, weighted score, exact full/positive
  block spectra, inverse residuals, design dimensions, PPML-drop, runtime,
  memory, and platform fields;
- release version, source commit, release SHA-256, configuration checksum,
  data-manifest reference, and output checksum;
- atomic result publication so an interrupted run cannot appear complete;
- logs that contain no raw identifiers, observations, or restricted paths;
- a schema validator and actionable exit code for every failure path.

Adapter tests must cover direct-command equivalence, exposure/offset
equivalence where applicable, target weights, approved and unapproved sample
drops, success, every project-relevant typed withholding status, process/CSV
status disagreement, interrupted output, replay, and clean installation.

Exit gate:

- An authorized small Separations-shaped fixture matches a direct standalone
  call at registered tolerances.
- The adapter emits a complete failure record when the estimator withholds and
  never emits point estimates from a failed run.
- The result validator rejects altered provenance, settings, schema, or
  accounting.

#### ST10E — target-data numerical qualification — IN PROGRESS

Qualify the realized network and outcome separately from the synthetic ST8
ladder. Use the exact specification, target weights, exposure/offset, and
sample defined in ST10A--ST10B.

The registered qualification design must include:

- graph-preserving small and intermediate pilots plus the full sample;
- PPML-only timing and convergence evidence separated from correction timing;
- nested correction-trace probe counts of 100, 200, and 400;
- at least two independent seeds at every registered probe count;
- fixed batch size unless a separately recorded memory/runtime comparison
  justifies another value;
- point estimates, trace MCSEs, weighted-score residual, exact full and
  positive block spectra, information-inverse residuals, PPML iterations, and
  accounting errors;
- convergence of reported digits across nested probes and seeds under an
  economic-unit tolerance registered before the full result is inspected;
- explicit recognition that trace MCSE measures randomized trace contraction
  only and is not an econometric standard error;
- preservation of every failure, censored run, and attempted setting.

The registered target-data ladder uses 100, 200, and 400 probes, batch eight,
weighted-score tolerance `1e-8`, rank tolerance `1e-10`, and exact block limit
`.8`. It escalates to 800 probes only under the rule in the model contract.
Final settings must be chosen from target-data evidence and then frozen; they
may not be weakened merely to obtain a successful run.

The completed CZ20 100-bin randomized ladder triggered the 800-probe
escalation. At 800, every two-seed point gap is below `0.0005`, but every
target's trace MCSE remains above `0.00025`. Reaching the MCSE threshold by
randomization would require an estimated order of 100,000 probes for the
worker target. This is a computational ceiling, not permission to change the
tolerance. The recorded amendment is to qualify the algebraically identical
deterministic coefficient-basis trace when the bounded quotient is below
`dense_limit()`. It must:

- agree with an independent dense observation-space oracle at the registered
  finite-algebra tolerance;
- be invariant to seed and batch width up to declared floating-point
  accumulation tolerance;
- report `correction_probes=0`, `trace_directions=p`, `trace_mcse=0`, and
  `numerical_mcse_scope=NONE`;
- pass two isolated GPT Pro challenges and local adjudication of the trace
  identity and its code mapping; and
- pass the exact CZ20 100-bin and 200-bin qsub pilots before becoming the
  frozen Separations setting.

The 100/200/400/800 randomized files, checksums, scheduler records, and failed
MCSE gate remain immutable qualification evidence. Exact trace removes only
numerical probe error; it does not change the economic estimand, the deletion
unit, the independence assumption, or the open theorem-applicability gate.

Exit gate:

- Full-sample PPML converges with the approved retained population.
- Both coefficient-space information inverses pass their recomputed residual
  checks.
- Every exact full block eigenvalue is below `.8`, and every exact positive
  block eigenvalue is strictly below one at the registered rank tolerance.
- TALO values are stable to the registered economic-unit tolerance; accounting
  holds at the registered numerical tolerance.
- The run finishes within the registered operational window and leaves enough
  memory headroom for the actual retained Stata dataset.

#### ST10F — Stata 19/Linux and SCC production qualification — PENDING

Run the package and the exact Separations configuration in the target
environment. Required evidence:

- clean offline installation of the pinned dependencies and candidate bundle;
- Stata/MP 19 Linux quick and full suites;
- clean-install and adapter-fixture tests;
- target-data pilot and full run with scheduler exit status, elapsed/CPU time,
  maximum RSS, slots, memory request, host/module information, and logs;
- deterministic fixed-seed results across reruns on the same supported
  platform and documented cross-version comparisons;
- a narrow input dataset containing only required variables, with storage
  types reviewed so unrelated wide data do not consume the memory margin;
- retained failure evidence for timeout, out-of-memory, PPML, identification,
  leverage, or solver failures.

The jobs request eight scheduler slots and 8 GB per slot, for 64 GB total.
SCC job `7148595` established that the installed Stata/MP 19 license permits
only four active processors; every job records that realized count. The
resource gates remain less than 56 GB maximum RSS and less than 12 hours.
Eight requested slots are an allocation requirement, not a claim of eight-way
Stata execution. If the actual sample is larger or operational needs differ,
register another resource envelope before running it.

Exit gate:

- ST7 portability and ST8 SCC performance gates close on the exact candidate
  commit.
- The full intended 400-probe configuration, or the prequalified deterministic
  coefficient-basis configuration, passes on the target data and platform.

#### ST10G — release, pinned installation, and reproducibility — PENDING

Complete ST9 and build the Separations candidate from a clean tree. Deliver:

- license decision, semantic version, changelog, help, compiled companion, and
  release notes stating the supported and unsupported domains;
- archive `SOURCE_COMMIT`, internal file manifest, archive SHA-256, dependency
  versions, and reproducibility report;
- a clean temporary `PLUS` installation and offline installation instructions;
- a pinned copy or approved artifact location for Separations; never a path to
  a moving development checkout;
- a rollback procedure that selects the previous bundle by hash;
- a run manifest binding release, configuration, input-data provenance,
  output, validation result, and scheduler evidence.

Exit gate:

- Reinstalling only the pinned archive reproduces the authorized fixture and
  validates the full result schema.
- The release contains no raw or derived restricted data, credentials, or
  machine-specific private paths.

#### ST10H — interpretation review and final go/no-go — PENDING

Produce `integration/separations/QUALIFICATION_REPORT.md` with the exact scope
of the approved use. It must separate:

- computational availability from statistical identification;
- the log-conditional-mean target from realized-outcome inequality or causal
  effects;
- numerical probe MCSE from econometric standard errors;
- finite software evidence from theorem applicability;
- internal and AI review from named-human independent review.

The implementing econometrician maps the realized degree, leverage, target
weight, moment, independence, and support diagnostics to the applicable
theorem assumptions. A named human reviewer is required before assigning
`theory_supported` or `independently_checked`. Without that review, successful
results retain `TALO_THEORY_APPLICABILITY_UNVERIFIED` and may be used only under
an explicitly approved descriptive interpretation.

Final status is exactly one of:

- `READY_POINT_ESTIMATE_THEORY_SUPPORTED`;
- `READY_DESCRIPTIVE_APPLICABILITY_UNVERIFIED`;
- `BLOCKED_MODEL_SCOPE`;
- `BLOCKED_DELETION_UNIT_OR_DEPENDENCE`;
- `BLOCKED_IDENTIFICATION_OR_FACE`;
- `BLOCKED_NUMERICAL_CONDITIONING`;
- `BLOCKED_PLATFORM_OR_RESOURCES`; or
- `BLOCKED_RELEASE_OR_PROVENANCE`.

Overall ST10 exit gate:

- All ST10A--ST10H packages are complete or explicitly `NOT_REQUIRED`.
- Separations consumes a release hash and frozen configuration, never a moving
  checkout.
- The authorized fixture and production output pass the registered validator.
- Version, commit, dependency versions, data/config/output checksums, seed
  pair, probe counts, tolerances, hardware, timings, maximum RSS, PPML drops,
  diagnostics, statuses, limitations, and approval signatures are archived
  together.
- No report implies that point-estimate software supplies econometric standard
  errors, causal identification, or theorem applicability that was not
  separately established.

### ST11 — independent deletion units and hybrid PPML TALO engines — IN PROGRESS

ST11 is the owner-authorized extension required after the exact CZ20 200-bin
worker-bin by firm-bin deletion failed the preregistered block limit at
`0.8354554567751412`. That failure and all earlier ST10 evidence remain frozen.
ST11 does not weaken the `.8` block gate, the observation `.5` leverage gate,
the positive-face gate, rank tolerances, or numerical residual thresholds.

The public statistical contract separates coefficient identifiers from the
deletion unit:

- `worker()` and `firm()` always define the PPML fixed effects and the four
  variance targets;
- `deletion(observation)` deletes one physical observation and rejects
  `deletionid()`;
- `deletion(cluster)` requires a mutually exclusive `deletionid()`;
- `deletion(match)` without `deletionid()` retains legacy worker--firm-cell
  deletion, while `deletion(match)` with `deletionid()` deletes the supplied
  actual match without requiring constant worker/firm coordinates;
- string deletion identifiers receive a deterministic internal encoding and
  raw identifiers never enter logs or result artifacts.

Positive integer frequency weights represent exact duplicate physical rows.
In observation mode, deleting one observation removes one copy and repeats the
one-copy correction contribution by the frequency. In cluster or match mode,
deleting a unit removes all physical copies assigned to that unit. Default
target mass is physical-observation mass; an explicit `targetweight()` remains
total stored-row target mass and is not multiplied by the frequency.

Low-dimensional controls and nuisance fixed effects are jointly estimated by
default. The complete information inverse and every deletion movement include
their covariance with worker and firm effects. An optional
`nuisance(fixedoffset)` diagnostic holds the fitted nuisance index fixed and
returns `CONDITIONAL_ON_ESTIMATED_NUISANCE_INDEX`; it is never described as
complete joint TALO. Ordinary-FWL outcome residualization is prohibited.

The hybrid numerical contract is:

- `engine(auto|matrixfree|dense)`, with `auto` using dense coefficient-space
  calculations when the active quotient is at most `dense_limit(3000)` and
  the augmented matrix-free engine for larger observation-deletion designs;
- supplied-cluster and actual-match designs above the dense or local limits
  return typed unsupported-size failures rather than changing estimators;
- `trace(auto|exact|randomized)`, with exact coefficient-basis trace through
  `traceexact_limit(500)` and randomized trace with numerical MCSE otherwise;
- the legacy `traceexact` option remains an alias;
- default limits are 128 nuisance coordinates, 10,000 stored rows per deletion
  block, and 500 raw active local coordinates; weighted local rank is reported
  separately and a future exact rank-revealing reduction may reduce this
  conservative work limit.

No ST11 path posts `e(V)` or claims econometric inference.

#### ST11A — contracts, derivations, and review — CANDIDATE COMPLETE, AI REVIEWED

- Update the model/API contracts and companion derivation for one-copy
  observation deletion, all-copy cluster deletion, generic block Woodbury
  actions, and joint low-dimensional nuisance information.
- Maintain an independent dense oracle and executable algebra fixtures.
- Run two isolated GPT Pro reviews of the statistical mapping and two isolated
  GPT Pro reviews of the Schur/certificate/trace mapping. Each review uses a
  fresh Pro chat, a bounded packet, a verbatim stored response, validation,
  and local adjudication. An unresolved critical objection reopens the
  obligation and blocks dependent qualification.
- Model review can establish at most `ai_reviewed`; named human review remains
  required for `independently_checked`.

#### ST11B — generic dense deletion engine — CANDIDATE COMPLETE, LOCALLY VERIFIED

- Compress each deletion block to its active worker, firm, and nuisance
  coordinates; allow a block to span multiple coordinates.
- Form only coefficient-space and bounded block-local matrices, never an
  observation-by-observation matrix.
- Compute exact full/positive spectra, deleted predictions, Woodbury actions,
  deterministic or randomized traces, and typed block/rank/face/leverage/
  nonfinite failures.
- Preserve legacy `MATCH_*` statuses and add corresponding `CLUSTER_*`
  statuses.

#### ST11C — augmented matrix-free observation engine — CANDIDATE COMPLETE, LOCALLY VERIFIED

- Retain the two-FE solver and add the exact low-dimensional Schur complement
  `V=H_F^{-1}H_FZ`, `S=H_Z-H_ZF V` for full and positive information.
- Include nuisance movement in every inverse-information action and require
  equilibrated small-block and reconstructed full-system residual checks.
- Solve Schur-preparation columns no looser than `1e-12`, compare the
  algebraically identical `H_ZF V` and `V' H_F V` forms, and add only
  nonnegative residual-based padding before leverage gates. Label this an
  operational double-precision certificate, not an exact-real error bound.
- Extend graph and positive-support accounting to frequency multiplicities.
- Combine a conservative two-FE spanning-tree resistance certificate with
  the exact nuisance leverage term; return a typed certificate failure when
  the unchanged `.5` gate cannot be certified.

#### ST11D — tests, documentation, and package qualification — COMPLETE

- Add expanded/collapsed, singleton-cluster, same-bin/different-match,
  generic-cluster, dense/matrix-free, controls/year, joint/fixed-offset,
  exposure/offset, invariance, accounting, trace, failure, and finite Monte
  Carlo tests.
- Require dense/matrix-free agreement within `2e-9`, randomized trace within
  six reported MCSEs of exact trace, and deterministic exact trace with zero
  numerical MCSE and seed invariance.
- Rebuild the help file, README, testing guide, changelog, result schemas, and
  companion PDF. Clean installation and both Stata/Python oracle suites must
  pass before SCC work.

#### ST11E — Separations actual-match qualification — IN PROGRESS

- The actual deletion identifier is person by original `estabid`, not
  `analysis_estabid`. Construct an opaque identifier before duplicate collapse
  and never export raw identifiers.
- Create a new checksum-bound restricted frozen input from the same upstream
  panel and frozen sample. Preserve old frozen inputs and failures.
- On CZ20, compare unchanged cross-fitting, legacy bin-cell TALO, and actual-
  match TALO at both 100 and 200 bins on the same frozen inputs and binned FE
  targets. The historical cross-fit retains its one-index nuisance and
  replication-specific connected-set conventions; report those departures
  from the joint-nuisance/full-target TALO specification explicitly. A future
  specification-matched cross-fit must be separately labeled rather than
  substituted silently.
- Submit every cluster computation through `qsub`; never compute on the login
  node. Both actual-match pilots must pass all unchanged gates before the
  all-CZ workflow is launched.
- If both pass, run all CZs at 100 and 200 bins using actual-match TALO only.
  If either fails, retain the exact typed failure and permit only a bounded
  replay; do not merge, drop, or relax thresholds.
- Every report states that actual-match deletion permits within-match
  dependence but does not address unrestricted same-worker dependence across
  employers. The canonical Separations production script remains unchanged.

## 6. Test tolerances (initial registry)

These are starting values and must be calibrated from evidence, never loosened
only to make a failure disappear.

| Check | Absolute | Relative | Notes |
|---|---:|---:|---|
| dense algebra | `1e-10` | `1e-9` | well-conditioned tiny designs |
| finite-difference Hessian | `2e-6` | `2e-5` | step-size sweep required |
| PCG recomputed floating residual | — | `1e-10` | quick tests; production configurable; not an exact-real certificate |
| accounting identity | `1e-9` | `1e-9` | common probes required |
| exact LO diagnostic | `1e-6` | `1e-5` | analytic deletion is approximation |
| MC bias/consistency | design-specific | design-specific | fixed seeds and power study |
| Separations accounting | `1e-10` | `1e-10` | plugin, correction, and TALO; common probes |
| Separations nested-probe stability | `0.0005` | — | trace-only randomization; escalate to 800 under the signed rule |
| Same-platform fixed-seed replay | exact recorded fields | exact | wall time and measured resource fields excluded |
| Cross-version result comparison | preregistered | preregistered | Stata 18/19 differences recorded, never silently accepted |
| Provenance and result schema | exact hash/schema match | — | any altered release, configuration, input manifest, or output is rejected |

## 7. Risk register

| Risk | Consequence | Mitigation | Status |
|---|---|---|---|
| Match formula is invalid | biased experimental result | finite algebra has two Pro reviews, adversarial oracles, and exact block gates; theorem/scale/human gates remain explicit | formula repaired; applicability and production gates open |
| `ppmlhdfe` saved-FE contract changes | incompatible releases | version probe and integration tests on 18/19 | tested for local 2.3.0 and SCC 2.3.3; future versions open |
| Dense match information factorization dominates cost | unusable at the registered second resolution | bounded coefficient quotient, `dense_limit()`, exact residual checks, CZ20 timing before escalation | 200-bin pilot fits within time/memory, but the registered `.8` block gate withholds before exact trace |
| Ill-conditioned network stalls PCG | long or inaccurate run | quotient checks, worker-block Schur elimination, exact Schur diagonal, robust recomputed residual, typed stop, explicit weak-ring stress profile | typed weak-ring failure retained; conditioning gate open |
| Rich match quotient is rank deficient | unidentified target or nonunique solve | exact dense full and positive rank/inverse checks on the registered one-nuisance-FE design | locally tested; target gate open |
| Randomization breaks accounting | incoherent decomposition | target-linear probe construction and probe-level tests | local/SCC accounting pass; target-data gate open |
| Registered randomized MCSE requires excessive probes | target run cannot meet numerical precision within its window | retain the failed ladder; use the exact coefficient-basis trace when `p` is bounded; require oracle, two-review, and SCC timing gates | observed at CZ20 100-bin; deterministic candidate under review |
| Mata memory copies | peak-memory failure | in-place blocks, views, streaming, instrumented benchmarks | local and SCC synthetic 10m measured RSS pass; target data open |
| Separation/zero margins | formula unavailable | AI-reviewed conservative pure two-way deleted-face certificate; never ridge silently | mitigated for supported row deletion |
| Test oracle shares implementation bug | false confidence | independent dense Mata, exact refits, committed external fixtures | open |
| Supplied match cells are not economically independent | correction has no audited transfer theorem despite successful execution | owner-signed working convention, permanent applicability warning, descriptive-only use, named-human gate | accepted experimental limitation; ST10H open |
| Time FE or controls are omitted from information | current command answers a different question | rich design estimates both jointly; test target actions of nuisance terms are zero while their information remains present | candidate implemented; ST10E open |
| Sparse positive outcomes fail to span workers or firms | PPML or deleted fit unavailable; conservative certificate withholds | whole-data positive-support preflight, frozen sample rule, typed distinction between genuine and conservative failure | open; ST10B blocker |
| Synthetic mobile benchmark understates dense rich-design cost | production timeout or factorization bottleneck | separate PPML/correction timings and qualify each realized CZ at nested probes on target hardware | open; ST10E/ST10F |
| Trace MCSE is mistaken for sampling uncertainty | overstated econometric precision | exact block gates, nested probes, independent seeds, prespecified point-stability tolerance, explicit no-SE documentation | mitigated locally; target qualification open |
| Wide Separations dataset consumes benchmark memory margin | out-of-memory despite acceptable structural estimate | run from a narrow typed input, measure Stata dataset memory and scheduler RSS, retain headroom | open; ST10B/ST10F |
| Unapproved `ppmlhdfe` drops change the target population | internally consistent result for the wrong sample | default refusal, exact drop ledger, owner-approved frozen upstream restrictions | open; ST10A/ST10B |
| Adapter loses failure or provenance information | failed run mistaken for a result | atomic versioned success/failure schema, validator, release/config/data/output hashes | open; ST10D/ST10G |
| Restricted identifiers leak through logs or artifacts | confidentiality violation | aggregate preflight, scratch-captured validator output, post-validation log scan, external restricted report with checksum only | wrapper regression and qsub portability pass; target-data review remains permanent |

## 8. Decision log

| Date | Decision | Reason | Revisit trigger |
|---|---|---|---|
| 2026-08-11 | Package root is `ppml_talo/`, not existing `software/` | Owner instruction; independent development | owner change |
| 2026-08-11 | Separations uses a pinned release export | reproducibility and isolation | release workflow changes |
| 2026-08-11 | Runtime is pure Stata/Mata | deployment environment | deployment expands |
| 2026-08-11 | Point estimates only | explicit scope | separate inference project |
| 2026-08-11 | Observation-level deletion first; cluster extension gated | repository theory currently covers row independence | ST1 closes cluster gate |
| 2026-08-11 | Two independent GPT Pro reviews for new critical derivations | owner request and repository review policy | never for current scope |
| 2026-08-11 | Batch independent PCG right-hand sides, default eight | reduces Mata/grouped-action overhead while retaining per-RHS residual checks | SCC memory/runtime evidence |
| 2026-08-11 | Treat estimate plus 3.290527 row MCSE as an engineering diagnostic only | leverage-sketch draws do not justify a calibrated simultaneous confidence bound | a proved/calibrated replacement |
| 2026-08-11 | Defer frequency weights from 0.1 | the deletion unit for a collapsed row must be explicit before weights can enter both score and leave-out maps | Separations sampling-unit mapping is fixed |
| 2026-08-11 | Eliminate the full worker block with an exact Schur complement before PCG | worker indicators have diagonal information and dominate coefficient count in AKM designs | benchmark or adversarial conditioning evidence favors another solver |
| 2026-08-11 | Call the solver gate an operational recomputed double-precision residual check | two Pro audits showed that same-precision arithmetic is not a rigorous exact-real or forward-error certificate | higher-precision residual or proved accumulation bound |
| 2026-08-11 | Disable public `absorb()` | worker--firm connectivity does not certify rank after a third FE block | scalable multiway quotient-rank certificate and adversarial tests |
| 2026-08-11 | Use a conservative bridge-free positive-support certificate in production | it is linear-time and admits a clean no-unsafe-acceptance implication; an exact SCC oracle measures false rejection | scale evidence or a scalable exact dynamic-deletion algorithm |
| 2026-08-11 | Reject one-level firm quotients in the development release | zero-column nonfirst blocks were not part of the implemented slice contract | explicit zero-block implementation and tests |
| 2026-08-11 | Separate stable engine-scale and stochastic fit-stress benchmark profiles | external PPML convergence should not obscure matrix-free correction scaling | both profiles qualify at target sizes |
| 2026-08-11 | Separate heterogeneous-mobility and weak-ring network profiles | the original near-cycle generator confounded observation-count scaling with a vanishing spectral gap | both profiles and real-data degree/connectivity diagnostics qualify on SCC |
| 2026-08-12 | Restrict ST8 positive scale claims to the supported two-way public model | public `absorb()` is intentionally disabled, so a three-FE benchmark cannot qualify the released path | multiway quotient-rank certificate and public option reopen |
| 2026-08-12 | Export estimate vectors and the source commit in every benchmark row | timing alone cannot establish fixed-seed repeatability or accounting consistency | benchmark evidence contract changes |
| 2026-08-12 | Derive qualification provenance in a clean-tree wrapper | manual full-hash entry is error-prone and produced one detected metadata typo | benchmark platform or Git workflow changes |
| 2026-08-12 | Treat local Stata 18 and SCC/Stata 19 as distinct ST8 subgates | local measured evidence qualifies the implementation on one supported platform but cannot establish Linux portability or scheduler accounting | SCC and Stata 19 evidence are recorded |
| 2026-08-12 | Require a signed Separations model and deletion-unit contract before adapter changes or production runs | computational success cannot establish that the implemented statistic answers the project question | owner-approved ST10A record changes |
| 2026-08-12 | Do not use an estimated nuisance FE as a fixed offset to bypass unsupported dimensions | doing so omits its information and correction contribution | a reviewed equivalence proof for the exact intended estimator |
| 2026-08-12 | Treat synthetic, realized-network, and target-platform qualification as separate evidence | the stable mobile ladder isolates the correction engine and cannot certify real PPML convergence or graph conditioning | all three evidence layers pass |
| 2026-08-12 | Keep 400+400 probes and `1e-10` adapter settings provisional | those exact settings have not been measured at 10m or on Separations data | ST10E freezes evidence-based settings |
| 2026-08-12 | Require a result row for both success and typed failure in production integration | missing output must not be confused with a successful or ignorable run | result-schema policy changes |
| 2026-08-12 | Revise the SCC scale envelope from eight to four active Stata processors while retaining 64 GB total memory | scheduled job 7148595 recorded a four-processor Stata/MP 19 license and rejected `set processors 8` with `r(198)` | SCC license or production platform changes |
| 2026-08-12 | Keep byte-exact replay as a same-host criterion and report cross-host floating differences separately | same-host 1m replay was exact apart from wall time; a different SCC node changed nine stored fields only at `1.5e-20` or less among estimate/MCSE fields | deterministic cross-node arithmetic is implemented or exact replay policy changes |
| 2026-08-12 | Define the Separations outcome as the binary within-CZ employment-to-unemployment transition and use PPML without exposure or offset | this preserves the current economic specification | owner changes the estimand |
| 2026-08-12 | Preserve year FE and the generated education-specific quadratic and cubic age controls as estimated nuisance terms | omitting their information contribution would change TALO | the project specification changes |
| 2026-08-12 | Delete the supplied worker-bin by firm-bin cell and assume those cells are independent for the experimental correction | this is the owner's chosen correction unit even though an original worker may appear in several cells | a transfer theorem or another dependence convention is adopted |
| 2026-08-12 | Interpret `[fw]` as duplicate person-period likelihood mass in match mode; default target mass equals frequency and explicit target weights are already total row mass | exact expanded/collapsed equivalence fixes both score and target semantics | the physical-row or target-population contract changes |
| 2026-08-12 | Freeze the common E--U sample and supported FE maps upstream; fail after any PPML/TALO drop or support failure | TALO results may not select their own comparison population | a new sample is approved before inspecting results |
| 2026-08-12 | Use CZ 20 as the pilot, then qualify all CZs at 100 and 1,000 worker/firm bins with 100/200/400 probes and two seed pairs | this measures functional, stability, and production configurations | superseded on 2026-08-13 after the 1,000-bin positive-face gate failed |
| 2026-08-13 | Replace the future 1,000-by-1,000 qualification resolution with 200 by 200 while preserving the 100-bin baseline and all failed 1,000-bin evidence | the owner selected a coarser second resolution after exact qsub diagnosis proved that whole-match deletion at 1,000 bins loses positive-support quotient rank | the 200-bin pilot fails identification or the owner changes the economic specification |
| 2026-08-13 | Preserve the preregistered `.8` block-eigenvalue safety gate after the 200-bin pilot reaches `0.8354554568` | the diagnosed block is below the rank boundary of one but does not satisfy the numerical qualification criterion registered before inspection | the owner authorizes a separately reviewed numerical-contract amendment or chooses another resolution/specification |
| 2026-08-12 | Require absolute point stability below `0.0005`, numerical MCSE below `0.00025`, block spectral upper diagnostic below `0.8`, runtime below 12 hours, and RSS below 56 GB | three-decimal reporting and the 64-GB SCC envelope | preregistered criteria are revised before result inspection |
| 2026-08-12 | Request eight SCC scheduler slots and 64 GB even if the Stata license activates only four processors | owner requires the larger scheduler envelope; active processors are recorded rather than inferred | SCC policy or Stata license changes |
| 2026-08-12 | Keep the downstream integration standalone and descriptive; never edit canonical `separations.do` or replace its wage correction | this is an internal experimental comparison with theorem applicability unverified | named human review and an explicit production-replacement decision |
| 2026-08-12 | Preserve the failed randomized MCSE gate and switch the bounded match production candidate to an exact coefficient-basis trace | at 800 probes seed gaps pass but MCSE remains 0.00065--0.00293; approximately 100,000 worker probes would be needed, while the 100-bin quotient has 206 exact directions | the exact trace fails oracle/review/SCC gates or a future randomized method meets the unchanged tolerance more efficiently |
| 2026-08-13 | Separate `worker()`/`firm()` coefficient identifiers from a new `deletionid()` | actual person--establishment matches can be smaller than worker-bin by firm-bin cells and answer the approved deletion experiment without changing target coefficients | owner changes the deletion or target estimand |
| 2026-08-13 | Use original `estabid` with the person identifier for Separations actual matches | owner selected the physical establishment relationship; `analysis_estabid` remains the PPML construction input where already specified | signed project contract changes |
| 2026-08-13 | Make joint nuisance information the default and expose fixed nuisance only as a conditional diagnostic | ordinary PPML does not admit outcome-FWL residualization and fixed nuisances omit deletion-induced nuisance movement | a reviewed equivalence result establishes a narrower exact case |
| 2026-08-13 | Use a documented hybrid dense/matrix-free architecture behind one public estimator | bounded cluster blocks admit exact coefficient-space Woodbury calculations, while large observation deletion requires matrix-free actions | numerical evidence supports another estimator-invariant engine |
| 2026-08-13 | Require four isolated GPT Pro challenges across the two new formula rounds | the owner explicitly requested Pro review and the package contract requires two independent reviews for every material cluster/statistic change | never for ST11 formula acceptance |
| 2026-08-13 | Launch the expensive all-CZ qualification only for actual-match deletion and only after both CZ20 resolutions pass | bin-cell failures remain comparison evidence; the new empirical goal is actual-match deletion | owner authorizes another preregistered qualification matrix |
| 2026-08-13 | Preserve the existing split-sample comparator exactly and disclose its nuisance/support differences | the project comparator estimates one full-sample `xb` index in each half and evaluates each replication on its split-sample connected set; replacing it would create a new cross-fit estimator | owner requests a separately labeled specification-matched cross-fit extension |

## 9. Progress log

### 2026-08-13 — ST11 opened

- Re-ran the mandatory repository startup sequence. Handover integrity,
  environment doctor, project state, frozen release validation, and nine
  handover tests pass under the prepared Python 3.13 environment. The machine's
  legacy `python` command remains Python 2.7 and is not used for repository
  checks.
- Created branch `software/ppml-talo-st11-independent-deletion` from clean
  local `main` at `140769899fdab5bb044a08fb2e70aed92b0ef08f`.
- Owner-authorized scope is limited to `ppml_talo/` plus the pre-existing
  separately controlled test-only Separations hook. Frozen repository paths
  and the canonical Separations production script remain immutable.
- ST11A is `IN_PROGRESS`. ST10's 100/1,000/200-bin evidence and the exact
  `MATCH_BLOCK_LIMIT` result remain preserved without reinterpretation.

### 2026-08-13 — ST11 local candidate and first review round

- Added `deletionid()`, `deletion(observation|cluster|match)`,
  `engine(auto|matrixfree|dense)`, `trace(auto|exact|randomized)`, and
  `nuisance(joint|fixedoffset)` to the public command. Worker and firm IDs
  remain the coefficient/target coordinates. Numeric or string deletion IDs
  are encoded separately and never reported in failure artifacts.
- Implemented exact duplicate-frequency semantics. Observation deletion removes
  one represented physical copy; cluster and match deletion remove every copy
  in the supplied unit. Default target mass equals frequency, while an explicit
  target weight is total frozen stored-row mass. Expanded/collapsed tests cover
  both deletion conventions.
- Replaced the constant-coordinate match kernel with a generic local design
  compressor. The dense engine admits blocks spanning multiple worker, firm,
  nuisance, and control coordinates, subject to explicit quotient, block-size,
  and local-rank limits. It retains exact full/positive spectra, Woodbury
  movements, deterministic or randomized trace, and typed prefix-specific
  failures. Legacy bin-cell status labels remain unchanged.
- Added the complete joint-nuisance matrix-free inverse using
  `V=H_F^{-1}H_FZ` and the conditional Schur block. Every inverse action and
  row correction now includes nuisance movement. Full and positive gates use
  exact two-FE leverage for small quotients, a spanning-tree resistance upper
  bound for larger quotients, and exact residualized nuisance leverage. The
  fixed-offset route is explicitly labeled conditional and never residualizes
  the PPML outcome.
- Added the independent Python ST11 oracle, joint-Schur Mata unit suite, and
  public integration tests for singleton equality, same-bin/different-match
  blocks, string IDs, frequency expansion, controls/year, dense/matrix-free
  calibration, joint/fixed-offset differences, target/accounting invariance,
  and `e(V)` absence. The local Stata quick suite and independent oracle pass.
- The first two isolated GPT Pro reviews are archived verbatim and validated.
  Both conditionally accepted the central deletion and Schur algebra. Their
  shared objections were repaired in the contract, companion derivation, and
  oracle: deletion-stratified collapse, a frozen target matrix, identity-basis
  Schur checks, fixed-offset non-equivalence even when full-sample cross
  information is zero, exact/near nuisance collinearity, positive-rank loss,
  and one-copy versus all-copy frequency boundaries. The adjudication records
  `ai_reviewed` only.
- The second pair of isolated GPT Pro implementation reviews is archived
  verbatim, validated, and adjudicated. One response contains both an initial
  fail verdict and a later conditional pass; the adjudication preserves both
  and applies the adverse disposition. Critical frozen-packet defects were
  repaired: the weighted one-copy sketch now uses the information projection,
  every public matrix-free observation path has a deterministic certificate,
  positive information retains the common quotient, the legacy dense inverse
  reports a measured residual, final outputs have Mata/ado finite gates,
  accounting totals are forced, and post-fit failures cannot leak `e(V)`.
- Schur preparation now uses a tighter solve, an independent cross-form
  coherence check, and nonnegative residual padding before unchanged gates.
  Equilibrated reciprocal-condition diagnostics cover the nuisance Schur and
  dense full/positive/local systems. This remains operational floating-point
  evidence rather than a proved exact-real perturbation bound.
- The repaired quick and full Stata suites pass, including the reviewers'
  public four-row fweight counterexample, nuisance-order/tolerance tests,
  deliberately corrupted Schur preparation, expanded/collapsed deletion
  semantics, typed post-fit cleanup, and the joint-nuisance Monte Carlo. The
  clean-install smoke test, all three independent Python oracles, 30 package
  Python tests, and all four GPT Pro record validators also pass locally.
- Rebuilt the 13-page companion PDF and visually inspected its complete page
  set plus full-size numerical and status pages. The repository-wide
  `tools/run_checks.py --scope full` gate then passed with exit code zero,
  including handover integrity, all proof/application suites, both frozen
  paper builds, source audit, and finite verification. The remaining ST11D
  subgate is the source-bound Stata 19/Linux portability run through `qsub`.
- Separations adapter schema v3 now requires the explicit definition
  `actual_person_original_estabid`, joint nuisance information, dense routing,
  and generic rich-block certificates. A fixture proves that 160 supplied
  actual matches remain separate from 80 shared worker-bin by firm-bin cells.
  The frozen schema-v2 bin-cell comparison remains byte-contract compatible.
- Qsub job `7165258` searched the permitted SCC home/project locations for the
  separately controlled historical test hook and completed without finding a
  copy. No canonical Separations file was inferred, recreated, or changed.
  ST11E remains blocked on recovering that authorized hook or an owner-approved
  replacement path; numerical work continues only through qsub.
- Source-bound SCC portability job `7165684` retained a typed test failure with
  scheduler `failed=0`, process exit one, four slots, and no point-estimate
  output. Stata 19 conservatively classified every `1e-3`--`1e-12` nuisance
  perturbation as collinear, whereas the fixture required at least one case to
  converge. No estimator tolerance changed. The portability repair adds
  clearly identified `1e-1` and `1e-2` cases before the same near-collinear
  tail and continues to require valid inverse/residual certificates for every
  case that is accepted.
- Repaired clean commit `9f96df8ce55ddf1b51281b8dd46cbc74862759a6`
  has scoped archive SHA-256
  `de1a46c0501be26ddc3f95c093b7e814cbb82a3205d96d2c72dc8875e9035095`.
  SCC qsub job `7165793` passed Stata/MP 19 quick, full, and clean-install
  suites with scheduler `failed=0`, process exit zero, four slots, 65 seconds
  wall time, and 8.577 GiB maximum virtual memory. The staged archive hash and
  every log hash matched the job metadata. This closes ST11D; it does not
  close ST11E or establish suitability on the frozen Separations network.
- The controlled hook was subsequently recovered from the clean isolated
  Separations worktree at historical commit `7797189` and amended only on its
  `ppml-talo-test` branch. Commit
  `e8f440133a6d3a8140e35db60ba1e62c57165925` constructs the deletion ID from
  original `persid` by original `estabid` on the saved physical rows, then
  exact-collapses within that opaque ID. A standalone Stata fixture confirms
  that different actual matches sharing one worker-bin by firm-bin cell remain
  distinct. Canonical `separations.do` remains untouched.
- Qsub manifest job `7166223` bound the unchanged restricted panel SHA-256
  `8c8b30f6749eef1e2d0f355f33db78fb04e30ce2bfa174ea2eb806436cb5ae4c`
  to hook SHA-256
  `ddc2f617427dae203ce407e1e7211ddc941e27495e7fcfc1f4805605a3575ba5`;
  the new input-manifest SHA-256 is
  `98762b5d9972a33eac283f24792f5253fb5c7c46bc8ed488da80313b0c01b0ef`.
  The exact 100-bin actual-match CZ20 pilot is qsub job `7166249`. No result is
  recorded before scheduler completion and validation.
- Qsub job `7166249` subsequently passed the unchanged exact 100-bin
  actual-match gate. The validated four-row result covers 792,176 stored rows,
  4,354,235 physical observations, and 119,877 supplied deletion units. Its
  maximum full- and positive-deletion eigenvalues are `0.05496055035` and
  `0.07233116177`, respectively, against the registered `.8` limit. Full,
  positive, and local systems pass their unchanged rank/residual gates; the
  estimator used 206 deterministic coefficient-basis trace directions and
  joint nuisance information. Qacct records scheduler/process exit zero,
  eight slots, 1,389 seconds, and 11.868 GiB maximum virtual memory.
- The successful 100-bin result advances only the paired pilot gate. The
  checksum-bound 200-bin actual-match pilot was submitted through qsub as job
  `7166453`; it must pass every unchanged identification, positive-face,
  block-leverage, numerical, time, and memory condition before ST11E or the
  all-CZ workflow can advance.
- Qsub job `7166453` passed the unchanged 200-bin gate with 792,176 stored
  rows, 4,354,235 physical observations, and the same 119,877 actual-match
  blocks. Maximum full- and positive-deletion eigenvalues are
  `0.06960639122` and `0.12169374346`; both information residuals are below
  `2e-14`. The joint-nuisance exact trace uses 389 quotient directions.
  Qacct records scheduler/process exit zero, eight slots, 1,409 seconds, and
  11.817 GiB maximum virtual memory. The paired CZ20 actual-match gate is now
  complete without changing any threshold, so the checksum-bound actual-only
  all-CZ workflow may advance. The same-frozen-input cross-fitting and legacy
  bin-cell comparison, pinned bundle, all-CZ execution, and named-human gates
  remain open.
- Source-bound portability job `7166619` passed the latest clean archive's
  Stata 19/Linux quick, full, and clean-install suites. All-CZ prebuild job
  `7166653` then passed with 10 checksum-bound CZ panels, 3,662 seconds wall
  time, and 13.969 GiB maximum virtual memory. Configuration job `7167553`
  generated the complete 20-cell actual-match task matrix, and orchestration
  job `7167632` registered 10 wage jobs plus 20 held cells. Every cell
  published an aggregate result. Final validation attempt `7169584` failed
  before reading evidence because `qsub -b y /bin/bash -c` did not preserve
  the collector's two positional arguments; it exited 198 and created no
  summaries or qacct directory. The failure is retained. A dedicated
  `sge_all_cz_validate.sh` wrapper now provides an unambiguous, tested qsub
  contract; result and resource qualification remains open until that wrapper
  passes.

### 2026-08-13 — ST11 all-CZ failure and same-input comparison execution

- Final qsub validation enumerated the complete actual-match matrix rather
  than stopping at the first failure. All ten 100-bin cells and five 200-bin
  cells are available. At 200 bins, CZ19, CZ21, and CZ26 retain
  `PPML_SCORE_RESIDUAL`; CZ25 and CZ27 retain
  `MATCH_POSITIVE_FACE_FAILURE`. Qacct evidence covers every prebuild, wage,
  and cell job and contains no time or memory failure. The 15 available cells
  are not promoted as a partial qualification.
- Diagnostic source `52da7b4` retains the measured score before the unchanged
  `1e-8` gate. Frozen-input qsub replays measured residuals
  `2.22300229282e-8`, `1.86688924664e-7`, and `1.02595777958e-8` for CZ19,
  CZ21, and CZ26. Source `616de8f` then tightened only the internal PPML fit
  tolerance to `1e-12`; qsub retries `7171317`--`7171319` still withheld all
  three, and status-only qsub job `7171450` confirmed the same exact status.
- CZ25 and CZ27 fail at deleted positive-information eigenvalue one, not at
  the `.8` block limit or a numerical inverse residual. No automatic merge,
  drop, gate relaxation, or point publication was attempted. These two
  failures independently block the registered all-CZ production handoff even
  if a future fitting repair clears the score residuals.
- Checksum-bound existing cross-fit jobs `7171302` and `7171303` and
  actual-match jobs `7171315` and `7171316` completed with scheduler/process
  exit zero. Corrected legacy bin-cell jobs `7171483` and `7171486` also
  completed: 100 bins is available, while 200 bins reproduced the registered
  `MATCH_BLOCK_LIMIT` withholding. The first bin-cell attempts `7171313` and
  `7171314` exposed an empty positional-argument shift; `84efc5a` repaired it.
  Two later attempts, `7171364` and `7171436`, failed read-only preflight on a
  misspecified frozen-input path and never reached Stata. All four failures
  remain orchestration evidence.
- The comparison preserves the project's existing cross-fit method. It fits
  the full-sample nuisance index `xb` as one numeric control within each half
  and evaluates each replication on its split-sample connected set. TALO
  jointly estimates the original age controls and year effects and freezes the
  full target population. The final report must show the support counts and
  treat this as a material method difference, never an equality test.
- Inspection of a failed expected-availability wrapper showed that Python
  validator traceback text could be appended after the log privacy scan.
  Commit `e18d575` captures validator output in scratch, suppresses restricted
  paths, and scans retained logs afterward. This is an evidence-wrapper repair
  only; it does not alter the statistical estimator or reinterpret any prior
  result. Its path-scoped archive SHA-256 is
  `2bb5b29b7ea0c440ff4f8942bd06f0e2bdae6ac4ca318c81087ff0af4748b2ac`.
  Qsub portability job `7171505` passed Stata 19/Linux quick, full, and clean-
  install suites with scheduler/process exit zero, 37 seconds wall time, and
  8.659 GiB maximum virtual memory.

### 2026-08-11 — P0

- Ran mandatory repository startup and full validation gate.
- Diagnosed and normalized a CRLF-only working-copy anomaly in an imported C
  file; committed content remained unchanged.
- Audited untracked files for credentials and large artifacts.
- Committed approved prior work and fast-forwarded `origin/main`.
- Created `software/ppml-talo-stata` from synchronized `main`.

### 2026-08-11 — ST0 opened

- Declared package-local rules, immutable surrounding scope, milestones,
  status vocabulary, performance targets, and review gates.

### 2026-08-11 — ST0 closed and ST1 opened

- Added installable development metadata, a version-only command, Mata API
  probes, a help scaffold, local environment evidence, and Stata-first tests.
- Local quick suite passed under Stata/MP 18 on Apple Silicon.
- Clean `net install` into `/private/tmp/ppmltalo-install.wnuCVN` passed and
  installed only `ppmltalo.ado`, `ppmltalo.mata`, and `ppmltalo.sthlp`.
- Verified local dependencies: `ppmlhdfe` 2.3.0, `reghdfe` 6.12.3, and
  `ftools` 2.49.1.
- Remaining limitation: the command is intentionally `SCAFFOLD_ONLY`; ST1 now
  owns all estimator algebra and the cluster-extension decision.

### 2026-08-11 — ST1 candidate implementation checkpoint

- Added the complete observation-space half-Hessian, singleton exponential
  analytic deletion, experimental cluster Woodbury deletion, matrix-free
  target-shared trace identity, and projection-sketch identities to the LaTeX
  companion.
- Added an independent Python algebra oracle covering directional and mixed
  finite differences, score-consistent exact row and cluster refits,
  coefficient-basis invariance, exhaustive Rademacher trace identities,
  singleton reduction, target linearity, and near-rank-loss deletion.
- Added dense Mata row/cluster oracles and fixed numerical fixtures. Cluster
  execution remains disabled in the public command.
- Built an observation-level command prototype with cached grouped HDFE
  operators, quotient reconstruction of saved `ppmlhdfe` effects, diagonal
  PCG, batched multi-right-hand-side solves, leverage sketches, complete
  curvature trace probes, and worker/firm/covariance/total target actions.
- Fixed-seed probe generation is batch-size invariant by construction. Unit
  tests compare batch sizes one and eight to `2e-13` and enforce the accounting
  identity probe by probe.
- Local Stata/MP 18 quick and full suites passed. The 40-replication Monte
  Carlo reported plug-in/TALO biases of `0.069484`/`0.013622` at degree 5 and
  `0.008574`/`-0.005494` at degree 20. This is finite numerical evidence, not a
  consistency proof.
- The clean temporary `net install` smoke test passed. The five-page companion
  compiled without unresolved references.
- The batched 100k-row, three-FE local benchmark with 40+40 probes completed in
  `50.082` seconds, with maximum PCG residual `9.797e-09` and 477 iterations.
  A post-documentation 10k smoke run with 10+10 probes and batch four completed
  in `0.895` seconds. RSS was not measured; the 1m--10m SCC ladder remains open.
- Two fresh, mutually isolated ChatGPT Pro reviews were launched from frozen
  packet hashes `5fa60b08...413b8` and `877cf8a4...34d4`. ST1 remains open until
  both verbatim responses and the local adjudication are stored and no critical
  algebra objection remains.
- Candidate work resembling ST2--ST8 deliverables is intentionally recorded
  here as prototype evidence. Those milestones remain pending until ST1 closes
  and their own complete exit gates are run.

### 2026-08-11 — ST1 closed and ST2 opened

- Archived two independent Pro responses verbatim and validated both review
  records. Both verdicts were `valid_with_repairs`; neither found a false
  finite-dimensional identity.
- Accepted and implemented the requested repairs: exact-score qualification
  and gate, one-step ALO wording, column orientation, conditional probe
  expectations, exact-projector qualification, explicit half-scaled AKM
  covariance action, and a narrower Moore--Penrose interpretation.
- Expanded the oracle with score-consistent row/cluster refits, off-root score
  failure, mixed Hessian directions, nonsymmetric trace actions, complete
  singleton reduction, local cluster block actions, quotient and redundant
  pseudoinverse invariance, approximate-projector failure, dependent-probe
  failure, and near-rank-loss deletion.
- Added a nontrivial score-root Mata fixture and production score-residual
  reporting. The repaired full Stata suite, Python oracle, clean install, and
  six-page LaTeX build pass.
- Stored the combined local adjudication at
  `theory/reviews/gpt-pro/adjudications/ST1-TALO-NUMERICS.md`. The finite
  formulas are now `formula_verified` and `ai_reviewed` only.
- Cluster mode remains disabled. Deleted-face certification, nonlinear sketch
  error, scale qualification, the cluster asymptotic transfer, and named human
  review remain outside ST1 and block production cluster use.

### 2026-08-11 — ST2 closed and ST3 opened

- Added an exact worker-block Schur complement to the scalar and batched
  matrix-free solvers. A dense unit identity checks
  `S = Hrr - Hr1 inv(H11) H1r`; reconstructed solutions still pass the public
  full-information residual check.
- Corrected the reduced-system stopping scale to use the original
  right-hand-side norm, with an internal factor-of-two margin for
  reconstruction roundoff. The minimized 100k failure had stopped at
  `1.0002268e-08` against a requested `1e-08` full residual.
- The repaired local full Stata suite and independent Python algebra oracle
  pass. A clean temporary `net install` at
  `/private/tmp/ppmltalo-install.hGP9yc` also passes.
- With seed `991827`, tolerance `1e-8`, and batch eight, the 100k three-FE
  benchmark with 40 leverage and 40 correction probes completed in `47.670`
  seconds, maximum residual `4.958e-09`, and maximum 234 iterations. This
  improves the pre-Schur `50.082` seconds and 477 iterations but is not a
  release-scale result.
- A one-million-row local smoke test with 10+10 probes and batch eight was
  terminated at the preset ten-minute cap without an estimator result or CSV.
  The pre-Schur run had the same censored outcome. ST8 therefore remains open;
  the solver needs further performance work before the SCC 5m/10m ladder.
- Added a callable dense-Mata exact leave-one-out PPML refit oracle. It uses an
  ascent line search, normalized score and Newton-step convergence gates, rank
  and index checks, and typed deleted-face/nonconvergence failures.
- Exact deleted predictions agree with independently computed NumPy fixtures
  to `2e-9`. Separate fixtures reject deletion-induced rank loss, an all-zero
  deleted face, and an intentionally insufficient iteration budget.
- Dense target curvature and row/cluster TALO already match fixed fixtures and
  the independent Python finite-difference/refit oracle. Outcome scale,
  coefficient quotient/basis, row order, target-weight scale, and accounting
  invariances pass their registered tolerances. ST2 is complete.
- ST3 is now in progress. The operator, Schur solve, and batching prototypes
  pass current dense comparisons, but explicit stagnation classification and
  broader adversarial conditioning coverage remain before closure.

### 2026-08-11 — ST3 double-Pro solver audit and numerical repair

- Ran two fresh, isolated Pro audits from packet hashes `fd17f137...a0fd7`
  and `e0a48eba...a057ed`. Both independently found the Schur algebra and
  exact-arithmetic scalar/batched PCG correct and found the same decisive
  finite-precision failure: naive squared norms classify a representable
  nonzero `1e-200*b` as zero.
- Archived both responses verbatim, validated their records, and stored the
  local adjudication at
  `theory/reviews/gpt-pro/adjudications/ST3-SCHUR-SOLVER.md`.
- Replaced squared norms with scaled sums of squares, added exact componentwise
  zero detection, normalized each RHS column before solving, rejected empty or
  nonfinite inputs, accepted an initially sufficient reconstructed iterate,
  and made the freshly recomputed full residual the operational acceptance
  rule. The solver no longer mutates caller RHS objects.
- Expanded unit coverage to a successful multidimensional/multi-iteration
  solve, full Schur basis, nonproportional mixed batches, exact-zero and
  transformed-zero columns, `1e-200` and `1e200` scale invariance, and invalid
  inputs. The repaired quick suite passes.
- The repaired full Stata suite, independent Python algebra oracle, and clean
  temporary `net install` at `/private/tmp/ppmltalo-install.i9Ts71` pass. All
  four stored Pro response records pass the repository validator.
- Narrowed every residual claim to an operational recomputed
  double-precision check. A rigorous exact-real/forward-error certificate is
  not claimed.
- Accepted the reviewers' three-way rank counterexamples. Public `absorb()` is
  disabled pending a scalable multiway quotient-rank certificate. The earlier
  100k three-FE timing remains historical internal evidence, not a supported
  public-command configuration.
- In the supported two-FE public configuration, the local 10k benchmark with
  10+10 probes and batch four completed in `0.576` seconds, with maximum
  residual `2.972e-09` and 27 iterations. The 100k benchmark with 40+40 probes
  and batch eight completed in `35.950` seconds, with maximum residual
  `4.978e-09`, 232 iterations, and a `0.105` GB engineering memory estimate.
  These are Stata/MP 18 Apple-Silicon wall times; the memory figure is not
  measured RSS.
- ST3 remains open for per-column batch failure isolation/statuses, periodic
  explicit residual replacement or an attainable-precision policy, and
  broader near-singular network tests.

### 2026-08-11 — ST3 closed and ST4 opened

- Added per-column batched-solve statuses and iteration counts. A breakdown or
  exhaustion in one column no longer terminates healthy recurrences; the
  aggregate solve still fails unless every column passes its final residual
  gate.
- Replaced PCG dot-product accumulation with Mata's `quadcross()`, retained
  scaled Euclidean norms, and rejected requested solver tolerances below
  `1e-15` as outside the package's supported double-precision range.
- Every 100 iterations, scalar and batched solvers explicitly replace the
  recursive residual without restarting the recurrence. Five hundred
  consecutive iterations without a new best residual receive a typed
  `STAGNATED` status.
- Added a connected 150-worker/150-firm chain fixture that requires 149
  iterations and crosses the residual-replacement boundary. Scalar and
  nonproportional batched solves pass independent full-system residual checks.
- Added mixed-batch tests for exact-zero, transformed-zero, converged, and
  exhausted columns, plus an independent-scalar fallback test for one-way
  designs. The quick Stata suite passes.
- The production path remains matrix free in observations. Dense matrices are
  confined to the small oracle and test fixtures. ST3 is complete; ST4 owns
  randomized leverage and curvature qualification.

### 2026-08-11 — ST4 closed and ST5 opened

- Added public `exact` mode for small Stata-only diagnostics. It constructs the
  quotient design and complete dense curvature oracle in Mata, uses no probes,
  reports zero numerical MCSE, and enforces independent row and parameter
  limits through `exact_limit()` (default 500, hard maximum 2,000).
- Added 80-stream fixed-design calibration tests with 64 probes per stream.
  Across the nine leverage coordinates, nominal pointwise 95% MCSE intervals
  covered the exact leverage between `0.875` and `0.975`. Across four
  conditional trace targets, coverage ranged from `0.925` to `0.963`.
- The same test requires replication means to lie within five empirical
  standard errors of the dense truth and empirical standard deviations to be
  within factors `0.65` and `1.45` of the mean reported MCSE. These thresholds
  were fixed before observing the passing result.
- Common probes retain the worker + firm + twice-covariance identity for every
  draw. Welford accumulation, seed and batch invariance, and the complete
  nonlinear curvature term pass the existing unit and oracle tests.
- The fixed-design MCSE checks are numerical calibration evidence only. They
  do not create a simultaneous leverage confidence bound or an econometric
  standard error. ST4 is complete; ST5 owns the public-command contract.

### 2026-08-11 — ST5 closed and ST6 opened

- Completed the `ppmlhdfe` orchestration, saved-effect quotient
  reconstruction, weighted worker/firm/covariance/total targets, `if`/`in`,
  exposure/offset equivalence, target weights, singleton policy, and explicit
  sample-drop gate. Frequency weights remain intentionally deferred.
- Added progress messages for the PPML fit and numerical phase unless
  `nodisplay` is requested. All temporary variables remain Stata tempvars.
- Posted the four corrected point estimates as `e(b)` without posting `e(V)`;
  `estimates store` and `estimates restore` now work without implying that
  econometric standard errors exist. The richer plug-in, correction, TALO,
  numerical-error, solver, score, and status contract remains in `e()`.
- The integration fixture confirms the dataset signature and row order before
  and after estimation, restores saved estimates, checks exact-mode replay,
  and verifies exposure/offset, outcome-scale, row-order, target-weight-scale,
  normalization, and accounting invariance.
- The clean-install test now runs a documented exact-mode estimation from the
  installed ADO/Mata/help artifacts, beyond the version/API smoke test.
- Numerical withholding clears point results and returns a nonzero code; no
  failure path inserts ridge output. ST5 is complete; ST6 owns the remaining
  identification and deleted-face obligations.

### 2026-08-11 — ST6 graph and positive-margin checkpoint

- Added an iterative, linear-time Tarjan low-link algorithm on the bipartite
  worker--firm multigraph. Parallel observation edges are distinct, so an
  individual duplicate edge is not falsely labeled a bridge.
- The command now withholds before randomized work if deleting any row would
  disconnect the identified worker--firm quotient, or if deleting a positive
  row would remove the last positive outcome margin for its worker or firm.
- Added direct complete-graph and 299-edge tree tests, plus public-command
  fixtures for a connected deletion-fragile tree and a redundant graph with
  deletion-fragile positive margins.
- Numerical-engine failures now leave `e(status)=WITHHELD` and a typed
  `e(withholding_status)` while clearing all point-estimate matrices. Success
  reports the two failure counts and `e(deletion_gate_status)`.
- ST6 remains in progress because graph rank and one-dimensional positive
  margins are not a complete general certificate for every deleted PPML face.
  The package retains `TALO_THEORY_APPLICABILITY_UNVERIFIED` and does not
  upgrade these necessary gates into a sufficiency claim.

### 2026-08-11 — ST6 closed and ST7 opened

- Strengthened the preflight into a conservative sufficient certificate for
  every one-row-deleted fit in the supported pure two-way model. Positive
  support must span every worker and firm, be connected, and have no row-edge
  bridge; the full graph must have no bridge.
- Two isolated Pro reviews independently reconstructed the Poisson recession
  cone, normalization, offset invariance, positive-graph proof, and full-graph
  rank equivalence. Both verdicts were `valid_with_repairs`; neither found an
  unsafe accepted two-way design.
- Accepted and repaired every material finding: the cancellation-prone
  outcome-margin subtraction now uses positive integer degrees; returned
  component counts are literal; skipped positive-bridge scans return missing;
  internal calls enforce exactly two blocks; one-firm quotients receive a
  typed unsupported status; and the exact deleted-refit oracle accepts finite
  offsets.
- Added direct passing, positive-bridge, missing-node, zero-row, parallel-edge,
  extreme-outcome-scale, three-block scope, one-firm, and conservative-
  rejection fixtures. Exact deleted PPML refits confirm two deliberately
  conservative rejections remain finite after every deletion.
- Added a materially independent positive-component/zero-arc SCC oracle. It
  exhausts 2,160 connected row-labeled 2-by-2 designs: 268 pass the production
  certificate, 18 are conservative rejections, and zero accepted designs are
  unsafe.
- Stored both verbatim review records and the local adjudication under
  `theory/reviews/gpt-pro/`; all six project Pro records validate. The pure
  two-way deleted-face result is `formula_verified` and `ai_reviewed`, not
  human `independently_checked` and not an asymptotic result.
- The repaired quick suite and independent Python algebra oracle pass. ST6 is
  complete; ST7 owns broader statistical, portability, and regression
  qualification. The public formula-applicability label remains unchanged.

### 2026-08-11 — ST7 local qualification checkpoint

- The repaired full Stata/MP 18 suite passes, including the 2,160-pattern exact
  face oracle, public failure catalog, installed-command contract, 80-stream
  randomized calibration, and the fixed bias/degree sequence. The independent
  Python algebra oracle passes at `atol=2e-10`, `rtol=2e-9`.
- Added a standalone testing guide that maps every suite to its obligation,
  seeds, tolerances, interpretation, scale profile, and release gate.
- Added and tested a pure-Stata Separations adapter. Its local fixture exports
  four accounting rows plus version, numerical, identification, and platform
  diagnostics while preserving the row-deletion sampling-unit warning.
- Added a Grid Engine Stata/MP 19 scale-ladder job and a checksum-manifested
  release-bundle builder. These are infrastructure, not passing cluster or
  release evidence.
- Optimized the deleted-face preflight: all-positive data reuse the full bridge
  scan, positive degrees use grouped reductions, and a second HDFE sort/build
  is avoided. The 10k 10+10-probe benchmark improved from `0.837` to `0.597`
  seconds with unchanged residuals and iterations.
- A one-million-row `stochastic_positive` 2+2-probe attempt reached external
  `ppmlhdfe` nonconvergence `r(430)` after about four minutes, before producing
  a TALO result. The command now types this as `PPML_FIT_FAILED`, and the
  benchmark harness writes failure rows rather than losing them.
- ST7 remains in progress until Stata/MP 19 Linux and cluster full-suite
  portability evidence is recorded. ST8 scale qualification remains open.

### 2026-08-11 — ST7 solver and million-row checkpoint

- Added the exact two-way Schur diagonal from worker--firm cell weights and
  cached the two-way firm row/coefficient maps. Dense tests equate the cached
  diagonal to the explicit Schur matrix diagonal.
- A symmetric block Gauss--Seidel PCG experiment on the current two-way test
  design required the same 231 iterations as exact-diagonal Schur PCG and was
  slower (`1.270` versus `0.675` seconds), so it was not promoted into the
  production path.
- Split scale evidence into `mobile` and `weak_ring` network profiles. The
  former uses deterministic heterogeneous worker mobility; the latter retains
  the original near-cycle topology as an adversarial conditioning test.
- On local Stata 18 Apple Silicon, `mobile` completed in `0.332` seconds at
  10k rows (10+10 probes; 11 iterations), `2.711` seconds at 100k rows (40+40;
  15 iterations), and `27.746` seconds at one million rows (40+40; 17
  iterations). The one-million run reported maximum residual `4.298e-09`,
  leverage upper diagnostic `0.266390`, and a `1.069` GB structural memory
  estimate. RSS was not measured.
- The earlier one-million `weak_ring` run reached the 5,000-iteration leverage
  solve gate in `108.822` seconds. It is now typed
  `LEVERAGE_SOLVE_MAXITER`; the package does not regularize or report a point
  estimate after that failure.
- The repaired full local Stata suite passes after these changes. ST7 still
  requires Stata/MP 19 Linux evidence; ST8 still requires measured SCC RSS and
  the SCC 5m/10m ladder.
- The same `mobile` configuration with 40+40 probes passed locally at five
  million rows in `150.886` seconds (18 iterations, residual `4.387e-09`,
  `5.346` GB structural memory estimate) and ten million rows in `327.280`
  seconds (19 iterations, residual `4.774e-09`, leverage upper `0.288394`,
  `10.692` GB structural estimate). These are full-command runs, but neither
  estimate is measured RSS.
- An attempt to stage commit `8d7639d` on SCC for Stata 19/Linux testing was
  refused by the configured account with `Permission denied
  (keyboard-interactive,hostbased)`. No credentials were requested, entered,
  or inspected. Cluster portability, 100+100-probe performance, measured RSS,
  and scheduler evidence remain open.

### 2026-08-12 — ST7 local evidence-contract checkpoint

- Extended each benchmark CSV to bind the source commit, all fixed seeds and
  solver settings, realized zero share, and all four plug-in, correction,
  TALO, and trace-MCSE vectors. Failure rows retain the same schema.
- Added the `poisson_zeros` outcome profile: a fixed-seed, correctly specified
  two-way Poisson draw that exercises PPML fitting, skewness, zeros, the
  positive-support certificate, and the correction together.
- Added a quick-suite integration test for successful stable and
  Poisson-with-zeros evidence rows. It checks source binding, solver residual,
  outcome-profile metadata, and the worker + firm + twice-covariance identity
  for plug-in, correction, and TALO values.
- The repaired full Stata 18 suite and independent Python oracle pass. The
  benchmark harness change does not alter estimator algebra or invoke a new
  mathematical review gate.
- A manual qualification invocation bound otherwise valid local results to a
  mistyped 40-character provenance string. The discrepancy was detected
  before evidence was registered. Added a clean-tree local wrapper that
  derives `HEAD`, refuses existing outputs, captures the Stata log and OS
  resource accounting separately, and exits with the Stata/time status.
- Corrected the structural memory units: future rows report decimal GB and
  binary GiB in distinct fields. Earlier `memory_estimate_gb` values used a
  binary denominator and are interpreted as GiB in the historical entries.

### 2026-08-12 — ST7/ST8 local Stata 18 qualification checkpoint

- Ran a clean-tree, source-bound 10k, 100k, 1m, 5m, and 10m `stable mobile`
  ladder with 100 leverage probes, 100 correction probes, and batches of
  eight. The exact source was
  `06137ded3332b860178d556e7a91ca77dad5a4d2`.
- The ten-million-row command completed in `629.950` Stata seconds and
  `631.16` operating-system seconds. Measured peak RSS was `19.656180`
  decimal GB (`18.306244` GiB), the maximum recomputed residual was
  `4.912e-09`, and the maximum solve used 19 iterations. This passes the
  local analogs of the 12-hour and 56-GB thresholds.
- Repeated the one-million-row run independently. Every CSV field other than
  wall time was identical, including all point estimates, MCSEs, solver
  diagnostics, identification diagnostics, seeds, settings, and provenance.
- A one-million-row Poisson profile with 24.4 percent zeros and a 100k
  stochastic-positive fit-stress profile passed at 100+100 probes. The
  adversarial one-million-row weak-ring profile with 2+2 probes was safely
  withheld as `LEVERAGE_SOLVE_STAGNATED`; no point estimate fields were
  populated.
- Added an evidence validator for source binding, fixed settings, accounting,
  residual and leverage gates, runtime/RSS thresholds, repeatability, outcome
  profiles, and typed weak-ring withholding. Added a versioned report and raw
  evidence checksum manifest under `benchmarks/reports/`.
- Hardened the local wrapper to propagate the benchmark CSV return code. This
  is necessary because the macOS Stata batch executable can exit zero after a
  do-file ends with a nonzero Stata return code.
- Added injectable Stata/timer executables and a deterministic shell
  regression test for successful, benchmark-failure, malformed-CSV,
  absent-CSV, and nonzero-process wrapper paths. Production retains
  `/usr/bin/time` as the default resource timer.
- The local Stata 18 test and performance subgates pass. ST7 remains in
  progress pending Stata 19/Linux test portability. ST8 remains in progress
  pending SCC scheduler evidence and representative-data conditioning work.

### 2026-08-12 — ST10 Separations readiness plan expanded

- Replaced the three-line integration placeholder with eight gated work
  packages covering the estimand and row unit, privacy-safe whole-data
  preflight, conditional feature extensions, production adapter, realized-data
  numerical stability, Stata 19/Linux deployment, pinned release, and final
  interpretation review.
- Registered explicit stop routes for additional fixed effects, continuous
  controls, clustered or collapsed deletion units, custom targets, and
  inference. These are not treated as routine adapter options.
- Separated synthetic engine performance, realized-network qualification, and
  target-platform qualification. The existing 10m local result remains valid
  evidence for its stated 100+100, `1e-8`, Stata 18 Apple-Silicon profile.
- Made the current adapter's 400+400, `1e-10`, leverage-limit-0.5 settings
  provisional until nested-probe, multi-seed, realized-data evidence is
  recorded.
- No estimator, adapter, theorem, test, manuscript, or release artifact changed
  in this planning checkpoint. No new mathematical review gate was triggered.

### 2026-08-12 — ST7/ST8 SCC qualification resumed

- Restored noninteractive SCC access and verified Grid Engine plus the
  `stata-mp/19` module from the login node. No Stata process was run there.
- Replaced the sequential scale job with five scheduled array tasks so every
  ladder size receives its own scheduler status and resource record.
- Added a separate scheduled portability job for the quick suite, full suite,
  environment/dependency capture, and clean installation.
- Bound both jobs to a full source commit and staged archive SHA-256, preserved
  result/resource/Stata logs without overwrite, and propagated a nonzero
  benchmark CSV status even if the Stata process exits zero.
- Added an SCC evidence validator for artifact hashes, Stata/dependency
  versions, qacct status, numerical gates, accounting, runtime, and measured
  RSS. A deterministic complete-contract test passes and verifies that a
  post-metadata result alteration is rejected. Passing SCC evidence remains
  pending until these exact scripts run from a committed clean archive on
  compute nodes.
- This checkpoint changes cluster and test infrastructure only. It does not
  change the estimator, mathematical statistic, or review status and does not
  trigger a GPT Pro mathematics gate.
- SCC job `7148575` reached a compute node and produced the environment CSV,
  then stopped because Stata 19/Linux named the automatic log
  `portability.log` rather than the dotted basename anticipated by the first
  harness revision. The failed attempt is retained; the harness now encodes
  the observed Linux naming rule before resubmission.
- Repaired job `7148585` passed the quick suite, full suite, and clean-install
  smoke test under Stata/MP 19 on Linux using the staged `a26ea8b` archive.
  Its environment has `ppmlhdfe` 2.3.3, `reghdfe` 6.13.1, and `ftools`
  2.50.0, so cross-version compatibility passed relative to the local Stata
  18 dependency set. Scheduler accounting and final evidence registration are
  still pending.
- The same environment reported four active Stata processors under an
  eight-slot allocation. The next candidate explicitly requests the granted
  slot count inside every Stata process, records active/licensed/machine
  processor counts, adds the processor count to each benchmark row, and makes
  expected dependency versions validator inputs. No scale job will be
  submitted until the processor request is tested through `qsub`.
- Scheduled job `7148595` recorded `processors_lic=4`, rejected the explicit
  eight-processor request with `r(198)`, and stopped before scale work. The
  revised SCC envelope is registered as four slots with four active Stata
  processors and 16 GB per slot for scale tasks. The 64-GB total allocation,
  56-GB measured-RSS ceiling, and 12-hour ten-million-row limit are unchanged.
- Four-slot job `7148602` accepted the explicit processor request and passed
  the quick and full suites, but the portability harness stopped after the
  clean-install run because Stata named its automatic log `plus 4.log` once
  the processor argument was added. The attempt is retained and the exact
  observed name is encoded before the final portability resubmission.
- Final portability job `7148614` passed the quick suite, full suite, and
  clean installation on Stata/MP 19/Linux with four requested and active
  processors. It used dependencies `ppmlhdfe` 2.3.3, `reghdfe` 6.13.1, and
  `ftools` 2.50.0 and exited cleanly under qacct.
- Grid Engine array `7148645` passed the complete 10k--10m stable-mobile
  ladder at 100+100 probes. The 10m task completed in 4,114.027 Stata seconds,
  used 14.109 decimal GB measured RSS, reached residual `4.912e-09` in at most
  19 iterations, and had leverage upper diagnostic 0.178318. Every task had
  scheduler exit zero and accounting error below `1e-12`.
- Same-host repeat job `7148701` reproduced every 1m CSV field exactly except
  wall time. Cross-host repeat `7148688` differed in nine final stored fields,
  with maximum absolute estimate/MCSE difference `1.50e-20`; this is retained
  as cross-node stability rather than exact replay.
- The source-bound SCC validator passed on the portability record, five scale
  tasks, all CSV/resource/Stata-log hashes, qacct records, resource gates, and
  same-host repeat. Added a versioned SCC qualification report and raw-evidence
  checksum manifest.
- ST7 is complete. ST8 remains in progress for realized Separations-network
  conditioning and production probe-setting qualification. No estimator
  formula, theorem, manuscript, or review status changed; no GPT Pro
  mathematics gate was triggered.

### 2026-08-12 — ST10 owner specification locked

- The owner fixed the outcome, nuisance specification, targets, target
  population, frozen sample, frequency-collapse meaning, match deletion unit,
  and conditional cell-independence convention in the implementation thread.
- ST10 is reopened on `software/ppml-talo-separations-match`. ST8 is paused so
  only ST10A is in progress. The resulting extension is experimental and
  retains `TALO_THEORY_APPLICABILITY_UNVERIFIED`.
- The only authorized downstream edit is a test-only Separations copy on its
  own worktree/branch. The canonical script and wage correction remain
  untouched.
- SCC Stata work must be submitted through `qsub`; the login node may only
  stage files, submit and inspect jobs, and collect aggregate evidence.
- Added the signed model and data contract at
  `integration/separations/MODEL_CONTRACT.md`. Every requested feature is
  classified: match deletion, frequency weights, one nuisance FE, and numeric
  controls enter the reviewed extension; econometric inference and canonical
  production replacement remain outside scope. ST10A is complete and ST10B
  is in progress.

### 2026-08-12 — ST10B--ST10D candidate implementation checkpoint

- Added exact integer-frequency expansion formulas, weighted whole-match
  Woodbury deletion, two-sided correction mass, explicit stored-row target
  mass, rich local compression, and separate full/positive information gates
  to the companion and independent oracles.
- Two isolated GPT Pro reviews accepted the central finite formulas and found
  unsafe unpivoted-QR truncation, ambiguous positive-block notation,
  score-only exact-refit stopping, and an invalid strict row-count condition.
  The candidate uses SVD with reconstruction checks, explicit positive
  information, score-plus-step stopping, all-zero-face rejection, and square
  full-rank refit tests. The adjudication does not treat post-packet production
  code as Pro-reviewed.
- Added API 7 match mode with positive integer frequency weights, one nuisance
  FE, numeric controls, exact coefficient-space full/positive inverses, exact
  whole-match spectra, exact analytic deleted means, and trace-only
  randomization. It allocates no observation-space square or design matrix;
  its dense cost is quadratic in memory and cubic in the bounded quotient
  dimension.
- Added typed public gates, probability diagnostics, a configuration-driven
  adapter, an atomic success/failure schema and validator, expanded/collapsed
  and rich-face regressions, an SCC shape preflight, and a test-only
  Separations copy. The canonical Separations script remains untouched.
- The local quick and full Stata suites, weighted Python oracle, adapter
  validator tests, two-pass LaTeX build, nine-page visual PDF review, and clean
  temporary installation pass. The clean installation exercises both
  observation and rich match modes.
- SCC qsub preflight job `7154178` read the existing CZ20 saved E--U result and
  exported aggregate diagnostics only. It has 3,774,998 expanded rows,
  3,590,373 saved-index patterns, and 89,329 supplied cells, with at most 211
  patterns per cell. It also has 41,406 worker levels, 1,417 firm levels, and
  fitted-mean maximum 1.583. This is not the registered 100- or 1,000-bin
  probability-repaired design, so it is shape evidence only and cannot close
  ST10B. Corrected qsub job `7154190` fixed finite-value counting and confirms
  12,105 blocks above 100 patterns, zero above 500, 8 slots, 4 active Stata
  processors, 12 seconds wall time, and 717,712 KB maximum RSS. Both runs are
  retained; neither is relabeled as a registered-bin pilot.
- The isolated downstream copy parses locally. A local end-to-end call stops
  in the baseline project's `group2hdfe` path before reaching the hook under
  the desktop dependency set. Therefore the exact integration check remains
  the qsub CZ20 pilot under the project environment.
- Added the qsub-only pilot pipeline. A scheduled manifest job hashes the
  restricted panel, wage map, and isolated test copy. The prepare job rebuilds
  CZ20 at the approved 100 or 1,000 groups, requires pinned dependency
  versions, publishes the first four-target result, and freezes the exact
  collapsed design inside the restricted SCC directory. Independent scheduled
  adapter jobs then reuse that immutable file for 100/200/400 probes and two
  seeds. Results now report actual quotient dimension, worker/firm/nuisance
  levels, control count, and estimator wall time so group realization can be
  audited rather than inferred from filenames.
- Removed shell command tracing from every Separations qsub wrapper before the
  pilot candidate was run. The first staged archive was superseded without a
  Stata pilot because tracing would have copied restricted paths into wrapper
  logs; no source rows or identifiers were exposed.
- Added a deterministic six-result probe-ladder validator before the pilot. It
  requires two distinct seeds at 100, 200, and 400 probes, verifies frozen
  design/provenance fields, stores input checksums, enforces the registered
  stability and final-MCSE thresholds, and emits a typed 800-probe escalation.
  The sanitized intermediate archive was superseded before any Stata pilot so
  this gate is part of the pinned final candidate.
- SCC job `7154286` was the first exact CZ20 100-bin prepare attempt. It
  requested eight slots and 64 GB, ran on an econ compute node, and failed
  safely before reading the panel: 0.22 seconds process time, 35,476 KB maximum
  RSS, scheduler exit 2, and no result or frozen input. Linux Stata selected an
  unanticipated automatic batch-log basename, so the wrapper removed scratch
  before retaining the diagnostic. Both prepare and reuse wrappers now capture
  the first generated `.log` file independent of basename. The failed job is
  retained and the corrected candidate must rerun through qsub.
- Corrected job `7154333` preserved the full Stata diagnostic and established
  that the saved CZ20 panel predates the owner-approved sample contract:
  `common_akm_sample` is absent. It used eight slots on an econ compute node,
  exited safely without a result or frozen input, and recorded 37,608 KB
  maximum RSS. Treating the stale panel flag or stale saved wage map as current
  would change the population. The prepare driver now runs the current
  `prepare_common_akm_sample` on the CZ panel and re-estimates the current wage
  AKM in job scratch before the E--U grouping, PPML repair, and TALO hook. The
  manifest binds the source panel and isolated code; no derived wage map is an
  external pilot input.
- Candidate `82b140d` passed the exact Stata 19/Linux quick, full, and clean
  installation job `7154371` in 35 scheduler seconds with 199,864 KB maximum
  RSS. Scheduled job `7154370` bound its inputs successfully. Real-data job
  `7154411` then rejected the old CZ20 file because it also lacks `sep_risk`;
  no result or frozen dataset was published. It used eight scheduled slots,
  seven wallclock seconds, and 931,016 KB maximum RSS. This ruled out the stale
  CZ extract and motivated binding the full upstream panel; the next job then
  determined which current transformation stages that panel still required.
- Job `7154440` showed that the 7.46 GB upstream panel intentionally precedes
  `analysis_cz`; it exited through the missing-output guard after one scheduler
  second and 35,520 KB maximum RSS, with no published result or frozen data.
  Inspection of the current Separations transformation identifies the exact
  missing stage: `build_cz_risk_sets` must run on the full upstream panel before
  selecting CZ20, because permanent firm CZ assignment uses all observed firm
  locations. The prepare driver now follows that exact order, then applies the
  CZ-local common-sample and wage-map stages.
- Exact-transform job `7158687` completed the 54.5-million-row risk-set rebuild
  and entered current common-sample connectivity, then exposed a downstream
  compatibility condition: pinned `group2hdfe` 1.01 executes `drop _m` for
  Stata's `_merge` marker. The pilot driver had disabled variable abbreviation,
  unlike the canonical Separations environment. It exited safely after 878
  seconds with 12,057,056 KB maximum RSS and no published result or frozen
  data. The driver now explicitly enables standard Stata abbreviation for the
  isolated batch. Its fixture reproduces `_merge`/`_m`, so future tests fail if
  this required compatibility setting is removed.
- Compatibility job `7158767` passed the estimator's typed availability,
  identification, probability, numerical-residual, and provenance checks on
  its four-row aggregate output. It used eight slots, 1,343 wallclock seconds,
  and 12,054,940 KB maximum RSS. The realized design has 4,354,235 expanded
  observations, 648,245 exact stored patterns, 100 worker bins, 93 supported
  firm bins, nine years, six controls, and quotient dimension 206. The TALO
  engine itself took 57.66 seconds with four active Stata processors. The
  scheduled wrapper nevertheless rejected the evidence because non-quiet
  `adopath ++` printed the restricted package root. The isolated Separations
  test copy now suppresses that display; the canonical file remains untouched.
  SCC's default `python3` is also version 3.6, too old for the aggregate
  validator. Both scheduled wrappers now pin `python3/3.12.4` before validation.
  The rejected result remains diagnostic evidence only and its frozen data is
  not reused by the next candidate.
- The first 100-probe result reports trace MCSE 0.00887 for worker variance,
  0.00216 for firm variance, 0.00231 for covariance, and 0.00508 for total.
  These exceed the registered 0.00025 qualification threshold. Do not loosen
  the threshold: complete the registered 100/200/400 two-seed ladder, emit the
  prescribed 800-probe escalation, and then reassess the probe ceiling with a
  recorded amendment if 800 remains plainly insufficient.
- Privacy-safe candidate job `7158896` closed the first 100-bin configuration:
  scheduler exit 0, 1,148 wallclock seconds, and 12,059,176 KB maximum RSS.
  Both scheduled logs passed the restricted-path scan and the Python 3.12
  aggregate validator passed. Before launching frozen-data reuse jobs, source
  review found the same non-quiet `adopath ++` in `run_match_adapter_env.do`.
  No reuse job was submitted. That entry point now suppresses the display.
  Since the package source hash changes, the accepted first result remains
  evidence for its exact candidate but its frozen data is not mixed into the
  successor candidate's probe ladder.
- Successor candidate `468fafd` passed the SCC Stata 19/Linux quick, full,
  clean-install, archive-hash, and log-privacy gates in qsub job `7159007`;
  scheduled manifest job `7159006` bound the exact upstream inputs under
  manifest SHA-256 `d04bf7149a3d0b4b26eb24c51996b5d3f1189179dc3167165f91521ae8a75e86`.
  The 12 registered CZ20 100/1,000-bin, 100/200/400-probe, two-seed
  configurations were generated from those hashes and made read-only before
  estimation.
- Prepare job `7159021` requested eight slots and 64 GB and ran only through
  qsub on `econ@scc-ei3`. It completed the global 54.5-million-row risk-set
  rebuild and CZ20 PPML fit, but no result or frozen dataset was published:
  the scheduled wrapper created the result directory but not the distinct
  parent of `PPMLTALO_FROZEN_DATA`, so Stata stopped at the frozen save with
  `r(603)`. Scheduler accounting records exit 2, 2,162 seconds, and 11.868 GB
  maximum virtual memory; GNU time records 12,048,768 KB maximum RSS. The
  privacy gate then correctly rejected the retained Stata log because the
  diagnostic contained the restricted path. A regression test now requires
  both output parents, and the wrapper creates the frozen-data parent before
  launching Stata. This failed candidate is retained and cannot supply frozen
  input to a later probe run.
- Repaired candidate `9d0fd6b` passed scheduled manifest job `7160879` and the
  Stata 19/Linux quick, full, clean-install, archive, and privacy gates in job
  `7160880`. Prepare job `7160939` then published the valid CZ20 100-bin
  frozen design and first result after 1,238 scheduler seconds with 12,058,772
  KB GNU-time maximum RSS (11.868 GB scheduler maximum virtual memory). The
  immutable package archive SHA-256 is
  `7912eb3b69aa1692245b11917e0a5aa1f4939a9ef713def2ec9e6fe7da250066`;
  the input-manifest SHA-256 is
  `09a47f0979ceb7aeb27662525fabe1fe849bbed319b6517ee3dbc630067fcbdb`.
- Scheduled jobs `7161319`--`7161323` completed the remaining 100/200/400
  two-seed randomized cells in 57--135 seconds each, and jobs `7161487` and
  `7161488` completed the required 800-probe escalation in 187 and 184
  seconds. Every job exited zero, passed privacy/provenance/schema validation,
  and used approximately 1.2 GB scheduler maximum virtual memory while
  reusing the frozen design. The eight-cell ladder report SHA-256 is
  `f7501cf2864ff75127bafaea980509e081729540c83488717dd32569ef649b23`.
- At 800 probes, absolute two-seed TALO gaps are `0.00025985` (worker),
  `0.00015962` (firm), `0.00027944` (covariance), and `0.00045864` (total), so
  the `0.0005` stability gate passes. The two MCSEs remain
  `0.00263/0.00293`, `0.000840/0.000874`, `0.000648/0.000720`, and
  `0.00169/0.00176`, respectively, so the unchanged `0.00025` MCSE gate
  fails. The validator reports `requires_probe_ceiling_reassessment=true`;
  no result is promoted by relaxing a threshold.
- Added an API-8 deterministic coefficient-basis trace candidate. It evaluates
  `tr(G_Q K)` through `p` standard-basis directions using only batched implicit
  `X`, `X'`, and block-`K` actions, reports zero randomized MCSE, and preserves
  worker/firm/covariance/total accounting. Independent Python algebra and
  Stata dense-oracle tests pass, including seed and batch invariance. The full
  local Stata suite, independent oracles, Python contract tests, companion PDF
  build and visual check, and full repository gate pass. The implementation
  and user documentation are complete at this candidate checkpoint.
- Candidate source commit `df3877d21bf6b4008e8bc48c87533db36b68e065`
  and the authorized downstream test-hook commit
  `7797189b063e85b5e83c1feb7c89a2c7e7f6334d` were exported as path-scoped
  immutable archives. Their SHA-256 values are
  `9e3ba40c7a3a8dd861ce793552e09e2fc0a074b7266334cc336744375e4b7f2a`
  and
  `0b117ce3bb1f6b58fec6b9c7e76dba48d5d6680ffa8487aa6c4cffe33578cd37`.
  The extracted hook hash remains
  `6c5dab24ef087ba37e4f59478f88375af5694267f0e1166888cfe391aaf86146`;
  the canonical Separations script remains untouched.
- SCC qsub job `7161651` passed the Stata 19/Linux quick, full, and clean-
  install suites in 36 scheduler seconds with four active processors and
  8.192 GB scheduler maximum virtual memory. Dependencies are `ppmlhdfe`
  2.3.3, `reghdfe` 6.13.1, and `ftools` 2.50.0. Independently scheduled
  manifest job `7161652` exited zero and bound the restricted panel and test
  hook under manifest SHA-256
  `a3af44191e1653273028c9cfa957f4e0d407c34815beb2e1a7bea32190b32956`.
  The dedicated candidate report records the generated-artifact hashes.
- The two isolated ST10E request packets are frozen, but Chrome page
  interaction remained unavailable after the documented fresh-window retry.
  No response or adjudication is fabricated, no self-review substitutes for
  Pro, and no exact target-data job is launched past that gate. The two-review
  gate and exact CZ20 100/1,000-bin qsub qualification remain in progress;
  this entry does not promote the candidate before those gates close.

### 2026-08-13 — ST10E Pro review recovered and repairs implemented

- A fresh Profile 4 Chrome window restored page control. The native file
  chooser still failed to expose the hidden upload input, so each frozen
  packet was transmitted in its own fresh Pro chat as one pasted-text
  attachment with exact file delimiters and the registered packet SHA-256.
  Both responses were recovered verbatim and pass the repository review-record
  validator. Review A returned `valid_with_repairs`; Review B returned a
  conditional pass with release-guard repairs required.
- Both reviewers independently accepted the coefficient-basis trace identity,
  the half-curvature factor, the collapsed frequency factors, and the Mata
  target mapping. They independently identified the same implementation gaps:
  deletion-key identifier constancy, information/predictor coherence,
  nonfinite final output, stable accumulation, exact total accounting,
  irrelevant exact-mode probe validation, and the inaccurate n-by-p memory
  claim. Review A also noted that the frozen packet omitted the ado wrapper.
- The accepted findings are implemented with typed guards, quad-precision
  direction inner products, streamed compensated direction totals, forced
  accounting, exact-mode `probes(1)`, and corrected memory/MCSE documentation.
  Regressions cover the concrete malformed-block counterexample, incoherent
  inputs, extreme nonfinite targets, direct block-kernel action, severe
  cancellation, exact RNG preservation, batches `1`, `3`, `p-1`, `p`, and
  `p+1`, literal expansion under explicit/default target masses, and a
  nonlinear offset.
- The local Stata/MP 18 full suite, both independent Python algebra oracles,
  and all 17 Python contract tests pass. The ten-page companion rebuilds and
  its rendered pages pass visual inspection. The local clean-tree wrapper is
  intentionally deferred until the repair commit exists. The adjudication
  does not claim that the repaired source or public ado wrapper was re-reviewed
  verbatim.
- ST10E remains `IN_PROGRESS`. Source changes invalidate the predecessor SCC
  portability candidate for release qualification. Freeze a clean repair
  commit, rerun the clean-tree wrapper, export and validate a new archive, run
  Stata 19/Linux portability only through qsub, and only then launch the exact
  CZ20 100-bin and 1,000-bin qsub qualifications. Include `ppmltalo.ado` in
  any repaired-candidate re-review packet.
- The first post-review CZ20 100-bin qsub attempt, job `7162633`, exited 2
  after 1,064 scheduler seconds and published no result or frozen design.
  Stata exited zero after rebuilding the upstream design, but the test-only
  hook remained disabled because the submission omitted `PPMLTALO_ENABLE=1`;
  the wrapper then correctly rejected the missing outputs. The failure is
  retained as orchestration evidence. `sge_cz20_prepare.sh` now requires the
  explicit opt-in and fails before Stata starts when it is absent. Freeze and
  requalify this source repair before retrying the 100-bin qsub gate.

### 2026-08-13 — exact CZ20 execution and scale-invariant information repair

- Commit `e7a73004224dc1bfd24b102133b460b0f42e4b42` passed the clean local
  qualification wrapper. Its archive SHA-256 is
  `c68b89baf0910b57d33f3835e16862821cc4d2f11b135c6c536a99c245e04f7e`.
  SCC qsub portability job `7162661` passed the Stata 19/Linux quick, full,
  and clean-install suites; manifest job `7162662` bound the inputs under
  SHA-256 `25a21ae8e46fb41cafc72cec38df02dc9eb202f89141ccb5bd8f2bc7d1c0db16`.
- Exact 100-bin CZ20 qsub job `7162673` passed every typed estimator,
  provenance, schema, accounting, and privacy gate. It finished in 1,155
  scheduler seconds with 11.817 GB maximum virtual memory. The realized
  quotient has 206 directions. The four TALO values are `0.5367947471`,
  `0.1814635484`, `-0.03555102325`, and `0.6471562490`; trace MCSE is exactly
  zero under its documented trace-only interpretation.
- Exact 1,000-bin job `7162710` completed PPML but the estimator returned
  `FULL_INFORMATION_SINGULAR`; the aggregate validator therefore withheld the
  result. This failure remains evidence and no point estimate was promoted.
  Its PPML quotient has 1,688 coordinates. Source inspection localized the
  rejection to raw `rank(H)` before any block or trace calculation.
- Aggregate-only qsub diagnostic `7162749` ran on the frozen failed design and
  established a coordinate-scale false rejection. Raw full and positive
  information ranks were 1,687, while diagonal equilibration produced rank
  1,688 for both. Their diagonals range from about one to `1.76e11`; scaled
  minimum/maximum eigenvalue ratios are `8.52e-5` and `8.27e-5`. Equilibrated
  inverse residuals are `6.10e-14` and `5.58e-14`, and back-transformed
  residuals remain below `3.4e-10`. The job exited zero in 70 seconds with
  1.020 GB maximum virtual memory.
- The implementation now applies invertible diagonal equilibration to the
  full, positive, local-kernel, and local-deletion systems. A unit regression
  multiplies an identified control by `1e12`: raw rank fails, the equilibrated
  point estimates agree with the original units, and an exactly duplicated
  coordinate remains singular. The local full Stata suite, both independent
  Python oracles, all 18 Python contract tests, the rebuilt companion, and the
  full repository audit pass. ST10E remains `IN_PROGRESS` until a clean commit
  and SCC portability rerun, and fresh exact 100-bin and 1,000-bin qsub pilots
  pass.

### 2026-08-13 — equilibrated candidate qualification and all-CZ execution graph

- Equilibrated source commit `599f11995c630e9897dcc55b6b61896fc54996ea`
  passed the clean local qualification wrapper and full repository audit. Its
  archive SHA-256 is
  `428a8907738dc54e7bdd1f36ae9e1825685490049b6e78aa22de0a2ed0628c1a`.
  SCC qsub portability job `7162758` passed the Stata 19/Linux quick, full,
  and clean-install suites; qsub manifest job `7162761` bound the restricted
  inputs under SHA-256
  `6b97450c107b454b2e6e4289841dfac87704f5b89a85d8a2fa740bf2869c8b2a`.
- Exact 100-bin CZ20 qsub job `7162767` passed after 1,119 scheduler seconds
  with 11.817 GB maximum virtual memory. The TALO engine used 81.496 seconds,
  both equilibrated inverse residuals were below `1.1e-14`, and the four TALO
  values agree with the predecessor candidate through displayed precision.
- Review A requested a materially different batch-width comparison. Exact
  replay job `7162989` used batch one on the same frozen design, exited zero
  after 145 seconds with 904.254 MB maximum virtual memory, and agreed with
  batch eight to about `4e-12` or better on the corrected components. The
  production batch remains eight because it completed the trace contraction
  materially faster.
- Added the full-CZ qualification execution graph. One qsub prebuild performs
  risk-set/common-sample work once and persists read-only per-CZ panels with
  individual SHA-256 values and a checksum-bound panel manifest.
  The submission layer validates the entire paired task matrix before
  launching jobs, submits one reusable wage fit per CZ, and holds independent
  exact 100- and 1,000-bin E--U/TALO cells on that wage result. Cells preserve
  the restricted E--U maps, frozen TALO designs, sanitized logs, and resource
  evidence; the aggregate validator refuses partial, unavailable, duplicate,
  overwritten, or provenance-altered matrices.
- The paired result gate is accompanied by an exact qacct coverage validator.
  Every registered wage and E--U job must have scheduler and process exit zero,
  eight slots, wall time below twelve hours, and maximum virtual memory below
  56 GiB. Missing and extra accounting records both fail.
- Deterministic two-CZ tests exercise configuration generation, task
  completeness, provenance tampering, result aggregation, overwrite refusal,
  and the qsub dependency graph through a mock scheduler. The all-CZ code is
  candidate infrastructure until a clean commit passes local/SCC portability
  and the registered target jobs finish.
- Exact 1,000-bin CZ20 job `7162987` ran past the repaired full and positive
  information inverses, then withheld at `MATCH_POSITIVE_FACE_FAILURE`. It
  exited the wrapper nonzero after 1,485 scheduler seconds with 11.868 GB
  maximum virtual memory and published no point estimate. This is progress
  beyond the predecessor raw-rank false rejection, not a passed gate. The
  package now retains privacy-safe block-stage and local-metric diagnostics on
  typed match failures; a fresh standalone qsub replay on the frozen design
  must classify the failure as local-kernel, local-rank, or eigenvalue-one
  before ST10B/ST10E can proceed at 1,000 bins.
- The standalone qsub adapter now has an explicit expected-withholding mode
  for that replay. It validates the exact named status and ordinary
  provenance while retaining availability as the unmodified qualification
  default; diagnostic evidence therefore cannot be mistaken for a passed
  cell.
- Frozen-design qsub replay `7163236` classified the 1,000-bin failure at
  anonymous block ordinal `10,395`, stage 3: the positive-deletion spectrum
  has maximum `0.9999999999999991` and numerical minimum
  `-1.40115121263e-19`. The full-block maximum is `0.4555427448`, while the
  full and positive inverse residuals are `6.57e-14` and `5.58e-14`.
  Accordingly this is a deleted positive-support rank loss under the current
  match unit, not the repaired raw-scale rank artifact. The replay published
  no point estimates. Its first wrapper exited 2 because Stata/Linux returned
  process status zero after the do-file captured estimator return code 3498;
  the wrapper now delegates expected-withholding success exclusively to the
  validated CSV status and provenance contract.
- Corrected implementation commit
  `706e38a9e6ca3047c809f420bede0022f42bb02e` has scoped archive SHA-256
  `0959f795c937e49c112f3b945679c220784938e2befe7c822e9a72fe5a1a9df5`.
  SCC portability qsub job `7163516` passed quick, full, and clean-install
  Stata 19/Linux gates with scheduler/process exit zero, 23 seconds wall time,
  and 8.785 GB maximum virtual memory. Corrected expected-withholding qsub job
  `7163517` passed its result/provenance validator and scheduler/process exit
  gates in 78 seconds with 1019.598 MB maximum virtual memory. Its result
  SHA-256 is
  `94e5a12618c1f9f045ea96b84b7cb09f719c2dc7c463cc06926e3c08528307fd`.
  It reproduced stage 3, anonymous block ordinal `10,395`, and positive
  maximum eigenvalue `0.9999999999999991` while exporting no point estimate.
- This evidence blocks the registered all-CZ paired qualification before its
  expensive launch: the exact 1,000-bin whole-match deletion is not identified
  on the pilot design. Proceeding requires an owner-approved change to the
  economic deletion unit, bin/sample construction, or estimator contract.
  The implementation does not infer that choice, silently merge cells, drop
  the offending match, or relax the face gate.

### 2026-08-13 — ST10E reopened for the 200-by-200 resolution

- The owner replaced the future 1,000-by-1,000 qualification cell with
  200 by 200. This changes only the second worker/firm bin resolution. The
  binary E--U outcome, no-offset PPML model, common sample, year and age
  controls, target mass, supplied whole-match deletion unit, independence
  convention, exact coefficient-basis trace, tolerances, and resource gates
  remain unchanged.
- The failed 1,000-bin configurations, jobs, typed statuses, checksums, and
  diagnostic conclusion remain immutable historical evidence. They are not
  relabeled as 200-bin evidence and will not be included in the new paired
  result matrix.
- Reopened engineering milestones are ST10D configuration/orchestration and
  ST10E target-data qualification. The implementation must accept exactly
  100 or 200 groups in the Separations launchers, generate a complete
  CZ-by-{100,200} task matrix, reject 1,000 as a new production
  configuration, and update schema-contract and mock-scheduler tests.
- The next target-data gate is an exact CZ20 200-bin prepare job submitted
  through qsub from a clean, checksum-bound source archive. Only if it returns
  an available point estimate may the all-CZ paired 100/200 execution graph be
  launched. No automatic coarsening, match dropping, or face-gate relaxation
  is authorized.
- The local 100/200 implementation checkpoint is complete. All launchers,
  configuration generators, result validators, help text, and deterministic
  fixtures now accept exactly 100 or 200 and reject 1,000 as a new production
  request. The focused Python contracts (16 tests), all package Python
  contracts (29 tests), Stata quick and full suites, both independent
  numerical oracles, shell/Python syntax checks, and the full repository audit
  pass. This was local evidence only; the following qsub records close the
  SCC portability check and classify the exact CZ20 200-bin gate.
- Clean source commit `db9a70ea53495b0559c75fec0e9d7c18f6ae31cb`
  has scoped archive SHA-256
  `e07f713f00a46fd23eca343ef8951c7397fd82e2b7c3b759ddf6d228c97d3d42`.
  SCC portability qsub job `7163601` passed Stata 19/Linux quick, full, and
  clean-install suites with scheduler/process exit zero, four slots, 39
  seconds wall time, and 8.516 GiB maximum virtual memory.
- Exact CZ20 200-bin qsub job `7163602` rebuilt the registered full-panel
  sample and froze the exact design, then withheld every point estimate at
  `MATCH_BLOCK_LIMIT`. It used eight slots, 1,105 seconds, and 11.817 GiB
  maximum virtual memory. The wrapper's nonzero availability result is the
  intended gate behavior; result SHA-256 is
  `935b9569adb8df82d086a910acd614a2e36eead5120c4b3e045fccd567bcc558`.
- Bounded frozen-design qsub replay `7163620` passed its exact expected-
  withholding validator with scheduler/process exit zero, 60 seconds, and
  904.055 MiB maximum virtual memory. Anonymous block ordinal `28,591`, stage
  5, has full deletion eigenvalue `0.8354554567751412`, exceeding the
  preregistered `.8` limit while remaining below the rank boundary of one.
  The maximum positive-block eigenvalue among earlier processed blocks is
  `0.4160970897557096`; full and positive information inverse residuals are
  `1.97e-14` and `1.87e-14`. The replay result SHA-256 is
  `e4929515c70f4e90efb2f3caceacc2d2824aa88ba0c0af45d725517cf8ed9336`.
- The diagnosed block is not a deletion-rank failure, but the run does not
  establish full-design identification: the engine stops before checking that
  block's positive face and the final five blocks. The 200-bin design is not
  qualified at the registered numerical safety limit, so the all-CZ paired
  launch remains blocked. No threshold was relaxed and no match, observation,
  or point estimate was dropped or published.

## 10. How to adapt this plan

Change this file in the same commit as any material scope or design change.
Record the old decision, the new decision, evidence, affected milestones, and
whether acceptance criteria changed. Never rewrite completed evidence. If a
new discovery invalidates an earlier milestone, mark that milestone
`REOPENED`, link the failing test or review objection, and block dependent
milestones until repaired.
