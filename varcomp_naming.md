# `varcomp_hdfe` Command Naming and Interface Amendment

**Status:** Approved naming direction for the unified Stata package
**Package directory:** `varcomp_hdfe/`
**Public commands:** `varcomp_kss`, `varcomp_ppml`, and `varcomp_targetset`

## 1. Purpose

This document records the plan to replace the development command names
`kss_bc` and `ppmltalo` with a consistent public naming scheme and to remove
the proposed `varcomp_hdfe` wrapper command.

The final public command surface is:

```text
varcomp_kss          Linear KSS variance components
varcomp_ppml         PPML TALO variance components
varcomp_targetset    Optional method-specific sample construction and diagnostics
```

This is a naming and interface amendment to
`varcomp_hdfe_specification.md`. Where the earlier specification calls for the
commands `kss_bc`, `ppmltalo`, or a `varcomp_hdfe` wrapper, this document
supersedes those provisions. The statistical estimands, deletion semantics,
weight semantics, numerical requirements, diagnostics, and validation
obligations in the main specification remain in force.

This document does not rename code by itself. The actual renaming should occur
as a coordinated package change so ado files, Mata runtime loading, help files,
tests, examples, returned metadata, and installation records remain
consistent.

## 2. Naming decision

### 2.1 Public mapping

| Development name | Public name | Meaning |
|---|---|---|
| `kss_bc` | `varcomp_kss` | Linear KSS leave-out variance-component estimator |
| `ppmltalo` | `varcomp_ppml` | PPML variance-component estimator using TALO correction |
| proposed `varcomp_hdfe` wrapper | no public command | Removed from the initial interface |
| no prior command | `varcomp_targetset` | Optional method-specific sample construction and diagnostics |

The package itself remains named `varcomp_hdfe`, but no command named
`varcomp_hdfe` is required. The shared `varcomp_` prefix makes package
membership visible through Stata command names and tab completion.

### 2.2 Why the names change

The development names are understandable to developers but less coherent as a
public package:

- `kss_bc` identifies a correction but not the broader variance-component
  purpose of the command.
- `ppmltalo` combines a model and a correction in a compact name that is less
  discoverable to researchers looking for PPML variance components.
- a `varcomp_hdfe` wrapper would require users to choose a model and correction
  even though the initial package has one principal correction for each model.

The new names provide one recognizable package prefix and preserve the terms
most likely to be used by applied researchers:

- KSS is the established name for the linear leave-out estimator;
- PPML identifies the nonlinear model used by the TALO command; and
- target set describes the optional preparation and diagnostic stage shared by
  the two estimation workflows.

There is a deliberate difference in what the suffixes identify: `kss` names an
estimation method, while `ppml` names a model. This is acceptable because the
names optimize applied discoverability rather than formal taxonomic symmetry.
The commands must remove any ambiguity through their help text and returned
metadata.

In particular:

```stata
varcomp_kss:
    e(model)      = "linear"
    e(correction) = "kss"

varcomp_ppml:
    e(model)      = "poisson"
    e(correction) = "talo"
```

## 3. No package wrapper

The initial package will not expose a `varcomp_hdfe` wrapper command.

Researchers will call the appropriate estimator directly:

```stata
varcomp_kss ...
varcomp_ppml ...
```

This avoids:

- redundant `model()` and `correction()` choices when only one combination is
  supported for each estimator;
- a wrapper containing the union of many options that apply to only one model;
- another layer of parsing and error handling; and
- uncertainty over whether the wrapper changes defaults or results.

The absence of a wrapper does not imply separate packages. Both commands must
share common infrastructure, syntax conventions, result names, versioning,
documentation, and installation under `varcomp_hdfe/`.

A future orchestration or comparison command may be considered if the package
later supports multiple corrections per model or routinely needs to run
several estimators together. That possibility does not justify a wrapper in
the initial package.

## 4. Package layout after renaming

All user-facing and runtime files remain part of the single package directory:

```text
varcomp_hdfe/
    varcomp_kss.ado
    varcomp_kss.sthlp
    varcomp_ppml.ado
    varcomp_ppml.sthlp
    varcomp_targetset.ado
    varcomp_targetset.sthlp
    [shared Mata source and runtime files]
    [tests]
    [benchmarks]
    [examples]
    [package metadata and documentation]
```

The final distribution should not require separately installing `kss_bc` or
`ppmltalo`. Shared Mata routines should use a package-level internal namespace
rather than duplicating estimator-neutral code under the old command names.
Estimator-specific functions may retain method-specific internal names where
that makes the mathematics clearer.

The exact subdivision of shared Mata files is an implementation decision. It
must not change the fact that all ado commands and runtime files are installed
from `varcomp_hdfe/` as one package.

## 5. Consistent estimator syntax

`varcomp_kss` and `varcomp_ppml` should use the same option names and meanings
whenever the statistical concept is genuinely common.

The common grammar should resemble:

```stata
varcomp_kss depvar [controls] [fw=frequency] [if] [in], ///
    worker(worker_id) firm(firm_id)                     ///
    deletion(observation|match)                         ///
    [deletionid(unit_id) targetweight(weight)           ///
     nuisance(joint|fixedoffset) engine(...)            ///
     seed(#) tolerance(#) ...]
```

```stata
varcomp_ppml depvar [controls] [fw=frequency] [if] [in], ///
    worker(worker_id) firm(firm_id)                      ///
    deletion(observation|match|cluster)                  ///
    [deletionid(unit_id) targetweight(weight)            ///
     nuisance(joint|fixedoffset) engine(...)             ///
     seed(#) tolerance(#) ...]
```

The following concepts should have common names and semantics:

- `worker()` and `firm()` identify fitted FE coordinates and target effects;
- `deletion()` identifies the independent unit removed by the correction;
- `deletionid()` may identify actual matches or clusters separately from the
  fitted worker and firm coordinates;
- positive integer frequency weights represent literal physical copies;
- `targetweight()` defines target-population mass rather than estimation mass;
- `nuisance(joint|fixedoffset)` distinguishes complete joint treatment from a
  conditional nuisance approximation;
- `engine()` selects a numerically equivalent architecture;
- `seed()`, probe counts, batches, tolerances, and iteration limits control
  numerical approximation rather than the econometric estimand; and
- common target and status results use the same row and column names.

Options that have no meaningful counterpart should remain estimator-specific.
Examples include JLA options and stayer conventions for `varcomp_kss`, and
exposure, offset, positive-face, and PPML separation options for
`varcomp_ppml`. Superficial syntax symmetry must not disguise a difference in
the underlying estimator.

## 6. `varcomp_kss`

`varcomp_kss` is the public Stata implementation of the KSS linear leave-out
variance-component estimator. It replaces the development command `kss_bc`.

Its public description should identify:

- the linear worker–firm model;
- worker variance, firm variance, worker–firm covariance, and total variance;
- actual-match deletion as the principal and default application;
- observation deletion as a supported alternative;
- the large-model Johnson–Lindenstrauss algorithm;
- exact calculation for small models and validation;
- frequency and target weights;
- joint and fixed-offset controls;
- mover/stayer limitations under match deletion; and
- the absence of econometric inference in the initial release.

Illustrative calls are:

```stata
varcomp_kss log_wage age2 age3 i.year [fw=freq],       ///
    worker(person_id) firm(analysis_estab_id)          ///
    deletion(match) deletionid(actual_match_id)        ///
    algorithm(jla) probes(200) nuisance(joint)
```

```stata
varcomp_kss log_wage,                                  ///
    worker(person_id) firm(estab_id)                   ///
    deletion(observation) algorithm(jla)
```

The rename must not weaken the requirement that actual match identifiers can
differ from the fitted worker–firm FE pair.

## 7. `varcomp_ppml`

`varcomp_ppml` is the public PPML variance-component estimator using the TALO
point correction. It replaces the development command `ppmltalo`.

The command name emphasizes the model, while its title, help file, output, and
returned metadata must prominently identify TALO as the correction. It must
not present all possible PPML variance-component estimators as if they were
TALO.

Its public description should identify:

- PPML estimation with worker and firm effects;
- worker variance, firm variance, worker–firm covariance, and total variance
  on the linear-predictor scale;
- the TALO estimation-noise and nonlinear-curvature correction;
- observation, qualified match, and qualified cluster deletion;
- exposure and offset support;
- frequency and target weights;
- joint and fixed-offset nuisance handling;
- separation, positive-support, rank, leverage, and block diagnostics; and
- the current absence of econometric inference.

An illustrative call is:

```stata
varcomp_ppml transitions controls [fw=freq],           ///
    worker(person_id) firm(analysis_estab_id)          ///
    deletion(match) deletionid(actual_match_id)        ///
    exposure(exposure) nuisance(joint)
```

If additional PPML corrections are added later, the package may introduce a
`correction()` option within `varcomp_ppml`. TALO remains the only correction
implied by the initial command.

## 8. `varcomp_targetset`

`varcomp_targetset` is an optional preparation and diagnostic command. It does
not estimate a variance component. It exposes the same method-specific sample
construction that the estimator commands use internally.

The command should require an explicit method:

```stata
varcomp_targetset depvar [controls] [fw=frequency] [if] [in], ///
    method(kss|ppml)                                         ///
    worker(worker_id) firm(firm_id)                          ///
    deletion(...) [deletionid(...) ...]
```

One public command is preferred to separate commands such as
`varcomp_kss_targetset` and `varcomp_ppml_targetset`. Internally, the KSS and
PPML builders remain distinct and may perform very different statistical
checks. The common command supplies a consistent way to request, inspect, and
record their outputs.

### 8.1 Optional workflow

Running `varcomp_targetset` must not be mandatory. Both estimator commands must
be self-contained and must construct the applicable sets automatically when no
prepared set is supplied.

The optional command is useful when a researcher wants to:

- inspect sample loss before expensive estimation;
- compare firm-grouping or deletion-unit definitions;
- understand an identification or separation failure;
- freeze a production sample and its diagnostics;
- reuse a qualified set across compatible specifications; or
- prepare a long-running Stata/SCC job.

### 8.2 Common outputs

The command should describe nested sets rather than return only a final
`e(sample)`. Subject to method-specific applicability, these may include:

- initial eligible observations;
- largest connected set;
- deletion-stable or leave-out set;
- fitted/interior estimation set;
- positive-support set;
- final estimation set;
- target population; and
- exclusions classified by reason.

It should support generating explicit sample and exclusion-reason variables
with a user-provided prefix. The exact generated variable names remain to be
specified, but they must be documented, collision-safe, and usable after
`e(sample)` is replaced by another command.

### 8.3 KSS diagnostics

With `method(kss)`, the command should be able to report:

- connected components and the selected largest connected set;
- the MATLAB-compatible leave-out connected set;
- articulation workers, bridges, and deletion-induced rank risks;
- numbers of workers, firms, observations, and matches;
- movers and stayers;
- observation- or match-level deletion counts;
- estimability of the requested KSS targets; and
- leverage or numerical diagnostics when the requested diagnostic level
  computes them.

### 8.4 PPML diagnostics

With `method(ppml)`, the command should be able to report:

- connected components;
- separated observations and their exclusion reasons;
- positive-outcome support by worker and firm;
- interior-fit and positive-face availability;
- full and positive information rank diagnostics;
- deletion-specific finite-fit conditions;
- observation or block leverage diagnostics; and
- whether the proposed TALO target set passes the current applicability gates.

Some PPML diagnostics require fitting the model and may cost almost as much as
the preparation phase of `varcomp_ppml`. The command must report what was
actually computed rather than imply that graph-only checks certify a PPML
interior fit.

### 8.5 Reuse and validation

A prepared target set must be bound to the configuration that produced it,
including at least:

- method and outcome;
- controls and nuisance fixed effects;
- worker and firm identifiers;
- deletion convention and deletion ID;
- frequency and target weights;
- exposure or offset where relevant; and
- options that affect sample selection or identification.

The estimator must validate this binding before reusing prepared results. It
must reject a stale, incompatible, or differently constructed set rather than
silently treating it as current. The exact storage mechanism—generated
variables, dataset characteristics, a compact manifest, or another
Stata-native representation—is an implementation decision requiring scale and
reliability tests.

The estimator remains responsible for final validation. A prepared set cannot
bypass method-specific rank, solver, leverage, or finite-fit gates.

## 9. Common returned metadata

The renamed estimator commands should expose a stable common core:

```text
e(cmd)             varcomp_kss or varcomp_ppml
e(package)         varcomp_hdfe
e(model)           linear or poisson
e(correction)      kss or talo
e(deletion)        observation, match, or cluster
e(engine)          selected numerical engine
e(nuisance)        joint or fixedoffset
e(results)         common variance-component table
e(sample)          final estimation sample where available
```

Additional metadata must continue to report weights, target mass, deletion
units, FE levels, connectedness, probe settings, solver performance,
applicability status, and method-specific diagnostics.

`varcomp_targetset` should use the same package and method labels and return a
common diagnostic schema supplemented by method-specific results.

No command may call numerical probe variability an econometric standard error.
The initial estimators must not post `e(V)` until their inference procedures
are separately implemented and qualified.

## 10. Rename and compatibility policy

Neither specialist command has yet been released as part of a stable public
`varcomp_hdfe` package. The preferred final package therefore exposes only the
new names:

```text
varcomp_kss
varcomp_ppml
varcomp_targetset
```

The coordinated rename must update:

- ado program definitions and internal entry points;
- ado and Mata runtime version handshakes;
- `e(cmd)`, `e(cmdline)`, package, model, and correction metadata;
- help files and cross-references;
- examples, benchmarks, adapters, and integration scripts;
- tests and expected status output;
- installation and package manifests;
- error messages and user-facing logs; and
- the main `varcomp_hdfe` specification where it still names the old commands
  or wrapper.

Temporary development aliases may be used during the transition if needed to
keep comparison tests runnable. They should be thin forwarding commands,
clearly deprecated, and excluded from the final advertised interface. A
permanent compatibility alias should be added only if evidence shows that the
development names have external users who need it.

Historical validation artifacts, review packets, frozen logs, and provenance
records must retain the command names under which they were created. Renaming
historical evidence would damage provenance and is not required for the public
package transition.

## 11. Decisions preserved and decisions left open

This naming amendment fixes the following decisions:

- the package is `varcomp_hdfe` and lives in `varcomp_hdfe/`;
- the public estimators are `varcomp_kss` and `varcomp_ppml`;
- no `varcomp_hdfe` wrapper is part of the initial interface;
- one optional command, `varcomp_targetset`, exposes method-specific sample
  construction and diagnostics;
- `varcomp_targetset` requires `method(kss|ppml)`;
- both estimators remain self-contained when the optional command is not run;
- common concepts use consistent syntax and returned metadata; and
- method-specific options and assumptions remain visible.

The following details remain open for implementation and user discussion:

- exact generated-variable names for `varcomp_targetset`;
- how a prepared set and its configuration signature are stored;
- whether preparation supports graph-only and full numerical diagnostic
  levels;
- whether any short-lived aliases are required during development;
- how aggressively internal Mata symbols are renamed during the first
  transition; and
- whether future additional corrections justify a `correction()` option or a
  separate comparison command.

These open decisions must not change the statistical estimand, deletion unit,
weight semantics, connectedness rules, or validation gates without explicit
discussion.

## 12. Completion criteria for the naming transition

The naming transition is complete when:

- the three public ado commands and help files are installed from
  `varcomp_hdfe/`;
- the final package documentation advertises only the three new commands;
- direct command calls use consistent common syntax;
- returned command, package, model, and correction metadata use the new
  contract;
- `varcomp_targetset` and the estimators call the same underlying
  method-specific preparation routines;
- prepared-set reuse cannot silently accept a stale or incompatible sample;
- old-name and new-name comparison tests establish numerical equivalence during
  migration;
- all examples, benchmarks, and clean-install tests pass under the new names;
- historical review and validation evidence retains its original provenance;
  and
- the main package specification has been reconciled with this amendment.

The transition changes naming and package organization. It must not change the
KSS or TALO estimators merely to make the public interfaces look more alike.
