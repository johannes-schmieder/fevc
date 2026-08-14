# `varcomp_hdfe`: Package Specification

**Status:** Working specification for discussion and implementation
**Package:** Stata/Mata variance-component estimators for high-dimensional fixed-effect models
**Primary commands:** `varcomp_hdfe`, `kss_bc`, and `ppmltalo`

## 1. Purpose and status of this document

`varcomp_hdfe` is a Stata package for estimating quadratic variance components
in models with high-dimensional worker and firm effects. The package provides a
common applied-research interface while retaining separate estimators for
linear and Poisson models:

- `kss_bc` implements the leave-out correction of Kline, Saggio, and
  Sølvsten (KSS) for linear two-way fixed-effect models;
- `ppmltalo` implements targeted analytic leave-out (TALO) point corrections
  for PPML two-way fixed-effect models; and
- `varcomp_hdfe` is the package-level wrapper that validates common options and
  dispatches explicitly to the requested model and correction.

This document describes the intended statistical contract, public interface,
numerical capabilities, validation requirements, and package organization. It
is not a milestone ledger or a file-by-file implementation plan. Algorithms
may be improved during development as long as the statistical estimator and
public contract remain unchanged.

The specification uses three levels of commitment:

1. **Core requirement.** A defining part of the package. A material change
   requires discussion with and approval from the package owner.
2. **Provisional default.** The intended initial behavior. It may be revised
   when derivations, benchmarks, or user experience provide a reason, but the
   change must be discussed with the owner before it is adopted.
3. **Open implementation choice.** The implementation may compare alternatives
   and choose among numerically equivalent approaches. It must bring the issue
   back to the owner if the choice affects the estimand, assumptions, public
   syntax, results, runtime dependencies, or practical usability.

Unless marked otherwise, statements using “must” are core requirements and
statements using “should” describe provisional defaults.

## 2. Package organization and identity

The complete user-facing package must live under the repository directory:

```text
varcomp_hdfe/
```

All commands and their runtime support are parts of this package. In
particular, the distributable source must include at least:

```text
varcomp_hdfe/
    varcomp_hdfe.ado
    varcomp_hdfe.sthlp
    kss_bc.ado
    kss_bc.sthlp
    ppmltalo.ado
    ppmltalo.sthlp
    [shared Mata source/runtime files]
    [tests, examples, benchmarks, and package documentation]
```

The exact subdivision of Mata source, tests, and documentation is an open
implementation choice. The following constraints are core requirements:

- `varcomp_hdfe`, `kss_bc`, and `ppmltalo` are commands in one package, not
  separately installed packages with duplicated internals.
- Shared Mata code must have a package-specific namespace and must not collide
  with unrelated commands in a user's Stata session.
- Installation must make all three commands and their help files available.
- The package must have a common version identifier, citation information,
  dependency statement, changelog, and installation test.
- Existing `ppmltalo` work may be incorporated or migrated into this directory,
  but migration must preserve validated behavior and provenance. This
  specification does not itself authorize overwriting or discarding the
  current implementation.
- No production ado or Mata runtime file belonging to these estimators should
  remain outside `varcomp_hdfe/` once the unified package is assembled.

The public package name is `varcomp_hdfe`. The established specialist command
name is `ppmltalo` without an underscore. `kss_bc` retains the underscore to
make clear that it reports a KSS bias correction.

## 3. Statistical models and common targets

### 3.1 Linear model

The linear model underlying `kss_bc` is

\[
y_r = \alpha_{i(r)} + \psi_{j(r)} + z_r'\gamma + \varepsilon_r,
\]

where `worker()` identifies the \(\alpha\) effects, `firm()` identifies the
\(\psi\) effects, and the optional variables in \(z_r\) are nuisance
parameters rather than variance-component targets.

The core KSS estimator must accommodate millions of ungrouped worker effects
and tens of thousands of firm effects. Binned or otherwise grouped identifiers
are permitted, but they are not the architecture's primary use case.

### 3.2 Poisson model

The PPML model underlying `ppmltalo` is

\[
E[y_r\mid X] = \mu_r
= \exp\{\alpha_{i(r)} + \psi_{j(r)} + z_r'\gamma + o_r\},
\]

where \(o_r\) may be an offset or log exposure. The TALO estimator and its
identification conditions remain governed by the PPML theory and the
`ppmltalo` numerical specification. The wrapper must not describe KSS as a
substitute for TALO or TALO as a substitute for KSS.

### 3.3 Common variance-component targets

Both specialist commands must report the same four target categories, defined
over the requested target population and target weights:

1. variance of the worker effect;
2. variance of the firm effect;
3. covariance of the worker and firm effects; and
4. variance of the sum of the worker and firm effects.

The total must satisfy the accounting identity

\[
V(\alpha+\psi)=V(\alpha)+V(\psi)+2\operatorname{Cov}(\alpha,\psi)
\]

for plug-in and corrected estimates, up to declared floating-point or random-
projection error. Common randomized directions should be used where this is
needed to make the identity hold probe by probe.

Controls, nuisance fixed effects, offsets, and exposures must receive zero
target action unless a future explicitly specified target says otherwise.
They remain part of estimation and may affect the correction.

The package must distinguish:

- weights used to estimate the model;
- frequency weights describing repeated physical observations; and
- weights defining the population over which a variance component is
  calculated.

The target quadratic form and its normalization must be constructed from the
full retained sample and frozen across deletions.

## 4. Fixed effects, deletion units, and independence

The design identifiers and the independent leave-out unit are separate
concepts throughout the package.

- `worker()` and `firm()` identify coefficient coordinates and target effects.
- `deletion()` identifies the kind of leave-out calculation.
- `deletionid()` may supply an independent deletion partition that is
  different from the worker and firm coefficient identifiers.

This distinction is essential for the Separations application. Individuals
remain ungrouped, while sufficiently small establishments may be combined to
retain a larger connected set. The correct match deletion unit is nevertheless
the actual person–establishment pair, not the person–grouped-establishment FE
cell. A production call must therefore be able to use, for example,

```stata
worker(person_id) firm(analysis_establishment_id)       ///
deletion(match) deletionid(actual_person_estab_match)
```

Multiple actual matches may share the same worker–firm coefficient coordinates
and must remain distinct deletion units.

### 4.1 Observation deletion

`deletion(observation)` removes one physical observation. Its identifying
assumption is independence at the physical-observation level, with the precise
KSS or TALO conditions stated by the specialist command.

### 4.2 Match deletion

`deletion(match)` removes all physical observations belonging to a worker–firm
match and permits unrestricted dependence within that declared match. Distinct
matches are treated as independent unless future theory supports a different
dependence structure.

When `deletionid()` is absent, the provisional compatibility convention is to
define a match from the interaction of `worker()` and `firm()`. When the fitted
firm identifier combines actual establishments, the user must supply an actual
match through `deletionid()` to obtain actual-match deletion.

Actual-match deletion does not allow arbitrary dependence across all matches
of the same worker. The commands and documentation must state this limitation
plainly.

### 4.3 General cluster deletion

`ppmltalo` may support mutually exclusive supplied clusters when its theory and
numerical gates permit. A general `deletion(cluster)` option for `kss_bc` is a
desirable extension but is not required for replacement of `leave_out_KSS.m`.
The initial complete KSS command must support observation and match deletion.
The wrapper must reject combinations not supported by the selected estimator
rather than silently changing the deletion unit.

### 4.4 Connectedness and leave-out estimability

The package must report how the estimation sample, largest connected set, and
leave-out connected set were constructed. It must never silently drop to a
different connected component or deletion convention.

For MATLAB compatibility, `kss_bc` should initially reproduce the sample-
selection and pruning conventions of `leave_out_KSS.m`, including its treatment
of articulation workers and observations with insufficient histories. A less
conservative deletion-unit-specific graph criterion may be investigated. It
must be validated theoretically and numerically and discussed with the owner
before it replaces the compatibility convention.

## 5. Public command interface

### 5.1 Package wrapper

The package-level interface is:

```stata
varcomp_hdfe depvar [controls] [fw=frequency] [if] [in], ///
    worker(worker_id) firm(firm_id)                      ///
    model(linear|poisson) correction(kss|talo)           ///
    [common and estimator-specific options]
```

Both `model()` and `correction()` must be explicit. The initial valid mappings
are:

| Model | Correction | Command used |
|---|---|---|
| `linear` | `kss` | `kss_bc` |
| `poisson` | `talo` | `ppmltalo` |

Inconsistent combinations must return a clear error. The wrapper must not
infer the statistical model from the outcome, choose an estimator based on
the data, or run several methods merely because several are installed.

The wrapper is responsible for:

- parsing and validating the common interface;
- checking model–correction compatibility;
- forwarding estimator-specific options without changing their meaning;
- displaying a common variance-component table; and
- returning a common core set of `e()` results.

The specialist commands remain directly callable and authoritative for their
estimator-specific options and diagnostics. The wrapper must add negligible
runtime relative to calling the specialist command directly.

Future corrections, including cross-fitting, may be added to the wrapper after
their statistical and return-value contracts are specified. They are not part
of the initial package requirement.

### 5.2 Illustrative KSS calls

The principal Separations-style call is expected to resemble:

```stata
kss_bc log_wage age2 age3 i.year [fw=freq],             ///
    worker(person_id) firm(analysis_establishment_id)   ///
    deletion(match) deletionid(actual_match_id)         ///
    algorithm(jla) probes(200) batch(8)                 ///
    nuisance(joint) targetweight(employment_weight)     ///
    seed(8675309)
```

Observation leave-out and a small exact calculation should be available as:

```stata
kss_bc log_wage, worker(person_id) firm(firm_id)        ///
    deletion(observation) algorithm(jla)

kss_bc log_wage, worker(worker_bin) firm(firm_bin)     ///
    deletion(match) algorithm(exact)
```

These examples establish intent, not an immutable spelling for every tuning
option. Option names shared with `ppmltalo` should remain symmetric unless the
same name would conceal a genuine difference in statistical meaning.

## 6. `kss_bc` statistical requirements

### 6.1 Required equivalence to `leave_out_KSS.m`

`kss_bc` is intended to be a full Stata/Mata replacement for the point-
estimation functionality of `leave_out_KSS.m`, not an observation-only or
small-model approximation. It must reproduce, subject to documented numerical
tolerances:

- the standard two-way AKM fit and normalizations;
- observation and match leave-out corrections;
- the plug-in and KSS-corrected firm variance;
- the plug-in and KSS-corrected worker–firm covariance;
- the plug-in and KSS-corrected worker variance;
- exact and Johnson–Lindenstrauss leverage/bias calculations;
- the improved constrained estimates of \(P_{gg}\) and \(M_{gg}\);
- the finite-projection nonlinear correction used by the current MATLAB
  routine;
- the connected-set and leave-out-set construction; and
- the special estimability issue for worker effects of stayers under match
  deletion.

Econometric inference and second-stage projection inference may be postponed.
The initial command must not post `e(V)` or construct conventional confidence
intervals. Its internal organization should retain quantities that are likely
to be required when KSS inference is added later.

### 6.2 Match deletion as the primary method

Actual-match leave-out is a first-release core requirement and the default KSS
deletion method. It must be a production-scale path, not a wrapper around
repeated regression calls.

For a pure two-way model, all observations in a conventional worker–firm match
have the same FE design row. The implementation may collapse such a match to
weighted sufficient statistics when this is algebraically exact. The collapse
must be keyed by the declared deletion unit. It must not combine separate
actual matches merely because their fitted FE coordinates are equal.

If jointly estimated controls vary within a match, the implementation must
retain the required within-match design information and use the corresponding
block leave-out algebra. It may not replace the block by a mean row unless an
equivalence result establishes that the replacement is exact for the requested
estimator.

### 6.3 Worker variance and stayers

Deleting the only match of a stayer also removes all information identifying
that worker effect. Match leave-out therefore does not identify the same
worker-variance correction for stayers as it does for movers.

The command must expose this issue rather than hiding it in a single headline
number. At minimum, returned results and documentation must distinguish:

- the match-based worker variance for the population on which it is leave-out
  estimable, normally movers; and
- any all-worker quantity that uses the MATLAB observation-level fallback for
  stayers and its additional assumptions or bound interpretation.

The precise display labels and option controlling the stayer convention are
provisional. Silently presenting the MATLAB fallback as an unrestricted
match-robust worker variance is prohibited.

### 6.4 Johnson–Lindenstrauss approximation

The main large-model algorithm must implement the improved JLA calculation
used by the current MATLAB package. It must include:

- reproducible Rademacher projections;
- randomized inverse actions for leverage and target-specific bias weights;
- simultaneous estimation of projection and residual-maker diagonals;
- the constrained ratio that keeps estimated leverage and residual leverage
  in their valid interval and makes them sum to one;
- the fourth-moment terms used to correct bias from division by an estimated
  residual leverage;
- target-specific firm, worker, and covariance contractions;
- common random draws where needed for covariance and accounting identities;
- batching that leaves the probe sequence invariant up to floating-point
  accumulation order; and
- diagnostics for convergence, realized probe count, seeds, leverage range,
  and random-projection variability.

The default number of JLA projections should initially be 200 for MATLAB
compatibility. Users must be able to set the number of probes, seed, batch
size, solver tolerance, and iteration limit. The package should provide a
clearly labeled numerical approximation diagnostic for each corrected target
where a defensible diagnostic can be constructed. Such a diagnostic is not an
econometric standard error.

An apparent discrepancy between the supplied JLA derivation and MATLAB code in
the coefficient on a mixed fourth-moment term must be resolved against the
source code, algebra, exact results, and simulation evidence. This
specification deliberately does not prejudge that resolution. A change from
the MATLAB formula must be documented and discussed with the owner.

### 6.5 Exact algorithm

`algorithm(exact)` must provide deterministic calculations for small or
moderate coefficient sets. It should factor the identified information matrix
once and obtain exact leverage and target bias quantities by inverse actions or
Woodbury identities. It must not refit the full regression once per deletion
unit in production.

The exact backend serves four purposes:

- a useful estimator when the FE dimension is small;
- a numerical oracle for JLA;
- a comparison target for the matrix-free solver; and
- a foundation for tiny brute-force leave-out tests.

Size and memory limits must be explicit. If exact calculation would exceed a
registered safe limit, the command must stop with a typed diagnostic rather
than start an unexpectedly expensive computation or fall back to JLA without
telling the user.

### 6.6 Controls and nuisance parameters

Two nuisance modes are required:

1. `nuisance(joint)` is the intended default. Controls are estimated jointly
   with the worker and firm effects, and their uncertainty and movement under
   deletion enter the information inverse and KSS correction.
2. `nuisance(fixedoffset)` reproduces the broad behavior of
   `leave_out_KSS.m`: estimate the nuisance coefficients on the full sample,
   subtract the fitted nuisance index, and apply the pure two-way correction
   while holding that index fixed.

The fixed-offset result must be labeled as conditional on the estimated
nuisance index. It must not be described as algebraically equivalent to joint
leave-out estimation merely because the full-sample OLS coefficients agree
under Frisch–Waugh–Lovell calculations.

Low-dimensional joint controls should be handled exactly with a Schur
complement or an algebraically equivalent method. An implementation may stage
development by validating the MATLAB-compatible fixed-offset mode first, but a
complete release claiming control support must include the joint mode. If the
block-JLA calculation required by within-match control variation reveals a
statistical or computational obstacle, implementation must stop and discuss
the available restricted or approximate contracts with the owner.

## 7. Frequency and target weights

### 7.1 Frequency weights

The package accepts positive integer frequency weights representing literal
copies of otherwise identical physical observations. It must reject
noninteger, nonpositive, missing, or nonfinite frequency weights. Other Stata
weight types are outside the initial contract.

The deletion semantics are:

- under observation deletion, one physical copy is deleted at a time; and
- under match or cluster deletion, every physical copy assigned to the unit
  is deleted together.

These semantics must agree with literal data expansion. Merely multiplying a
stored row by the square root of its frequency is not sufficient if that would
turn the stored row into the observation deletion unit.

For JLA on collapsed data, the implementation may generate aggregated random
quantities without expanding the rows. Their distribution and fourth moments
must reproduce the explicitly expanded calculation under the stated frequency
semantics. This is an open technical design issue subject to exact expansion
tests.

### 7.2 Target weights

`targetweight()` supplies nonnegative weights defining the empirical target
population. These weights are distinct from frequency or estimation weights.

The provisional shared convention is:

- without `targetweight()`, each physical observation receives equal target
  mass, so a stored row's default mass includes its frequency; and
- an explicit target weight is already the total target mass assigned to the
  stored row and is not automatically multiplied by its frequency.

The command must report the target-weight sum and normalization and must reject
negative or nonfinite target weights. A different convention may be adopted
only after discussion because it changes the estimand.

## 8. Matrix-free KSS architecture

Performance of the matrix-free KSS implementation is a core requirement. The
main application has millions of worker coefficients and tens of thousands of
firm coefficients, so a dense coefficient-space system cannot be the primary
engine.

For the pure two-way linear model, the implementation should exploit the block
system

\[
H =
\begin{pmatrix}
D'WD & D'WF\\
F'WD & F'WF
\end{pmatrix}.
\]

Because the worker block \(D'WD\) is diagonal, the solver can eliminate all
worker coordinates exactly and apply PCG to the firm-side Schur complement

\[
S_F=F'WF-F'WD(D'WD)^{-1}D'WF.
\]

This system is a weighted firm-mobility Laplacian. Its action and transpose
group operations must be available without constructing an observation-by-
parameter matrix, a parameter inverse, or an observation-by-observation
matrix.

The production engine must support:

- exact worker-block elimination;
- quotient-safe normalization;
- scalar and batched right-hand sides;
- reconstruction of worker coordinates or fitted values as needed;
- low-dimensional joint nuisance augmentation;
- recomputed full-system residual checks for every accepted solve;
- bounded memory as observations and workers grow; and
- streaming or chunked accumulation of match-level outputs.

The choice of PCG preconditioner is open. The MATLAB routine benefits from a
graph-Laplacian CMG preconditioner and parallel execution, neither of which has
a direct Mata equivalent. Candidate approaches may include diagonal or exact-
Schur diagonals, spanning-tree or mobility-graph preconditioners, recycled
Krylov information, and carefully reused `reghdfe` machinery. Benchmarks must
decide among them.

The implementation must not change the estimator, reduce probes, loosen
tolerances, or change the retained sample merely to meet a performance target.
If a pure Mata implementation cannot meet practical production requirements,
an optional compiled Stata plugin may be considered, but adding that runtime
architecture requires owner discussion and a documented deployment analysis.

## 9. `ppmltalo` requirements within the package

`ppmltalo` remains the specialist command for PPML TALO point estimates. Its
validated estimator-specific specification, theory labels, positive-support
requirements, leverage and block gates, exposure/offset handling, and
curvature correction must be preserved when it is incorporated into
`varcomp_hdfe/`.

At the package level, `ppmltalo` must:

- use the same meanings for `worker()`, `firm()`, deletion IDs, frequency
  weights, target weights, nuisance modes, engines, seeds, and typed failures
  wherever those meanings are statistically common;
- retain estimator-specific options where PPML genuinely differs from KSS;
- report the same four common targets and common result columns;
- distinguish PPML noise and curvature corrections where the validated
  decomposition permits;
- preserve the explicit theory-applicability status of development results;
  and
- refrain from posting econometric `e(V)` until PPML inference is implemented
  and qualified.

KSS and TALO may share solvers, target operators, and data structures, but they
must not share formulas merely for software symmetry. KSS corrects estimation-
noise bias in a linear model. TALO also addresses PPML nonlinearity and uses
the fitted PPML information matrix.

## 10. Shared internal services

The package should maintain one estimator-neutral implementation of operations
that have the same mathematical meaning. Likely shared services include:

- sample marking and missing-value checks;
- deterministic encoding of numeric or string identifiers;
- FE level maps, quotient normalizations, and connected-component summaries;
- separate design-coordinate and deletion-unit representations;
- frequency expansion and deletion accounting;
- match and cluster compression;
- worker/firm design actions and transposed grouped sums;
- scalar and batched matrix-free solves;
- low-dimensional Schur complements;
- dense reference factorizations;
- target-weight centering and quadratic target actions;
- reproducible random-projection streams;
- compensated or otherwise stable accumulation;
- numerical residual and conditioning diagnostics;
- typed status and failure records; and
- common display and `e()` result construction.

The exact internal API and source-file boundaries are open implementation
choices. Shared code must be used only when equivalence is established; it
must not force the two estimators into a common numerical representation when
their natural algorithms differ.

## 11. Returned results and display

All three commands are `eclass` commands. The specialist commands must return
their full estimator-specific results, while the wrapper must expose a stable
common core.

The common results matrix should have one row for each of:

```text
worker_variance
firm_variance
worker_firm_covariance
total_variance
```

and at least the following conceptual columns:

```text
plugin
bias_correction
corrected
numerical_mcse_or_error
```

Exact column names are provisional until the existing `ppmltalo` return
contract and the KSS prototype are compared. The common matrix must not force
an inapplicable component to zero without a label. Estimator-specific matrices
may provide the KSS noise correction, TALO noise/curvature decomposition,
stayer/mover results, exact leverage summaries, or other diagnostics.

At minimum, common `e()` metadata must identify:

- command and package version;
- model and correction;
- estimation and target sample sizes;
- worker and firm level counts;
- deletion type and number of deletion units;
- engine and algorithm;
- nuisance mode;
- frequency and target-weight conventions;
- probe counts and seeds when randomized;
- solver tolerance, iterations, and maximum residual;
- connectedness and sample-selection status;
- estimator applicability/status label; and
- whether numerical approximation error or econometric inference is
  available.

The commands must mark the estimation sample through `e(sample)` when Stata's
data state permits it. They must not export raw identifiers in logs or failure
artifacts by default.

Numerical randomization error, solver error, and econometric sampling
uncertainty must be separately named. No column called “standard error” may
refer only to JLA or trace-probe variation.

## 12. Diagnostics and failure behavior

The package must favor explicit withholding over returning a number whose
requirements were not met. Typed diagnostics must cover at least:

- invalid or missing IDs;
- invalid frequency or target weights;
- no usable observations;
- disconnected or unidentified designs;
- deletion-induced loss of rank or connectedness;
- nonestimable match deletions;
- excessive observation or block leverage;
- singular or ill-conditioned nuisance blocks;
- PCG nonconvergence or unacceptable recomputed residuals;
- dense/exact size limits;
- nonfinite predictions, corrections, or accumulations; and
- unsupported combinations of estimator, deletion type, engine, or options.

Diagnostics should report aggregate counts, extrema, and stages sufficient to
debug a problem while avoiding disclosure of raw worker, firm, or match IDs.
Dropping problematic units, selecting another component, or changing the
estimand must require an explicit user option and must be recorded in the
returned results.

## 13. Performance requirements

The main KSS performance benchmark must resemble the production Separations
application:

- millions of observations and ungrouped workers;
- tens of thousands of establishment-effect coordinates;
- some grouped small establishments;
- actual person–establishment match deletion;
- realistic match lengths and mobility;
- frequency weights and low-dimensional controls; and
- enough JLA probes for production accuracy.

Benchmarks must separately record:

- data preparation and connected-set time;
- model-fit time;
- preconditioner setup time;
- JLA or exact correction time;
- total time;
- solve iteration and residual distributions;
- retained observations, workers, firms, and matches; and
- peak resident memory.

Stata and MATLAB comparisons must use the same retained sample, identifiers,
controls, weights, target definition, deletion convention, probe count, solver
tolerance, and hardware where possible. Numerical agreement and performance
qualification are separate gates.

Runtime within approximately twice the MATLAB routine on the same problem is a
provisional engineering goal, not a statistical requirement or release
promise. The practical release requirement is that the Stata-only estimator is
usable in the production Separations environment. If benchmarks fall well
short, the implementation should diagnose operator passes, iteration counts,
batching, preconditioning, memory traffic, and parallelism before proposing a
change to statistical behavior.

## 14. Validation requirements

The validation suite must include independent checks rather than relying only
on agreement between two paths that share the same code. At minimum it must
cover:

### Statistical and algebraic checks

- brute-force observation and match refits on tiny datasets;
- exact dense KSS calculations against direct linear algebra;
- JLA convergence toward exact leverage and target corrections;
- reproduction of `leave_out_KSS.m` point estimates;
- resolution and testing of the improved-JLA finite-projection formula;
- mover/stayer decomposition under match leave-out;
- exact joint-control calculations against a dense full system;
- fixed-offset results against MATLAB's residualization convention;
- target accounting identities; and
- Monte Carlo evidence that the KSS correction removes the intended linear-
  model bias under the stated independence assumptions.

### Data-semantics checks

- actual matches that share the same fitted FE coordinates remain separate;
- singleton matches agree with observation deletion when their assumptions and
  weights coincide;
- integer-frequency results agree with literal data expansion;
- target weights alter the target but not the fitted model;
- results are invariant to ID relabeling, permissible normalization, row
  ordering, and harmless data sorting; and
- missing values and user `if`/`in` restrictions produce a reproducible sample.

### Numerical checks

- dense and matrix-free engines agree on overlapping feasible problems;
- scalar and batched solvers agree;
- results are stable across batch sizes for a fixed probe stream;
- reported residuals are recomputed from the original full-system operator;
- exact mode is deterministic and seed independent;
- JLA results are reproducible for a fixed seed;
- multiple seeds and probe counts reveal the expected approximation behavior;
- weak-mobility and ill-conditioned networks fail safely or meet registered
  tolerances; and
- large-scale benchmarks measure both runtime and peak memory.

### Package checks

- clean installation exposes all three ado commands from `varcomp_hdfe/`;
- direct specialist calls and wrapper calls return the same estimates;
- help examples run on shipped synthetic data;
- package version and dependency diagnostics agree across commands; and
- no runtime step requires MATLAB, Python, Julia, or access to restricted
  project data.

## 15. Runtime environment and dependencies

The production package must run from Stata in an environment where MATLAB is
unavailable. Stata/MP 18 and 19 are the initial target versions. The package
may depend on established Stata packages such as `reghdfe`, `ftools`, and
`ppmlhdfe` where their roles are declared and tested.

Python, MATLAB, Julia, and external command-line tools may be used as
development or validation oracles, but they must not be production runtime
dependencies. Restricted Separations data must not be distributed with the
package.

A compiled plugin is not part of the initial pure Stata/Mata requirement. It
may be proposed if measured Mata limitations prevent acceptable performance,
provided the proposal covers portability, installation, source availability,
fallback behavior, reproducibility, and production-environment constraints.
The owner must approve that change.

## 16. Flexibility and decisions requiring discussion

The following are core requirements and must not change without owner
approval:

- one package located in `varcomp_hdfe/` containing all ado commands;
- separate public commands plus an explicit package wrapper;
- explicit `model()` and `correction()` choices in the wrapper;
- full KSS point-estimation replacement for `leave_out_KSS.m`;
- production-scale matrix-free JLA as the main KSS engine;
- actual-match and observation deletion in `kss_bc` from the initial release;
- separation of fitted FE IDs from the deletion ID;
- frequency weights with literal physical-copy semantics;
- controls with both joint and MATLAB-compatible fixed-offset treatment;
- dense exact calculations for small models and validation;
- the four shared variance-component targets;
- no econometric inference or `e(V)` until separately implemented and
  qualified; and
- no silent changes to samples, estimands, independence assumptions, weights,
  probes, or numerical tolerances.

The following are provisional defaults:

- match deletion for `kss_bc`;
- `nuisance(joint)` when controls are supplied;
- 200 JLA projections;
- the MATLAB-compatible connected/leave-out-set construction;
- automatic engine selection based on registered size limits;
- equal physical-observation target weighting in the absence of
  `targetweight()`; and
- a performance objective near MATLAB runtime on comparable hardware.

The implementation is encouraged to improve batching, memory layout,
preconditioning, Krylov reuse, graph algorithms, sufficient-statistic
compression, and internal source organization without seeking approval for
every engineering change. It must pause and discuss the issue with the owner
when evidence suggests any of the following:

- a stated estimator cannot be implemented as specified;
- an approximation is needed where the specification calls for an exact
  calculation;
- MATLAB behavior appears incorrect or internally inconsistent;
- a different sample or connectedness rule would be materially better;
- controls or frequency weights require narrower supported semantics;
- performance requires a compiled dependency or reduced functionality;
- a public option or default should change;
- an existing `ppmltalo` contract would be broken by package unification; or
- validation exposes a conflict between theoretical and software contracts.

When such an issue arises, Codex should report the evidence, explain the
statistical and computational consequences, present the viable alternatives,
and recommend a choice. This specification is intended to guide a robust
implementation, not to force an implementation to conceal newly discovered
problems or mechanically follow a design that evidence shows should change.

## 17. Definition of a complete initial package

The initial `varcomp_hdfe` package is complete when:

- all source and runtime artifacts reside under `varcomp_hdfe/`;
- `varcomp_hdfe`, `kss_bc`, and `ppmltalo` install and run as one package;
- the wrapper dispatches explicitly and agrees with direct command calls;
- `kss_bc` reproduces the required MATLAB point estimates in observation,
  actual-match, JLA, and exact modes;
- the matrix-free KSS engine is qualified on a production-shaped Separations
  benchmark with ungrouped workers and many establishment effects;
- joint and fixed-offset controls, frequency weights, and target weights pass
  their equivalence tests;
- `ppmltalo` retains its qualified estimator-specific behavior after package
  integration;
- all common and estimator-specific results are clearly labeled;
- failure paths withhold invalid results and return actionable diagnostics;
- the documented runtime dependencies match the production environment; and
- the help files explain assumptions, deletion units, weight semantics,
  approximations, and the absence of econometric inference.

Completion of these software requirements does not by itself establish that
the assumptions of KSS or TALO are appropriate in a particular empirical
application. Each command must continue to report its model and dependence
contract so an applied researcher can judge that question.
