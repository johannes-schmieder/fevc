# `varcomp_hdfe`: Future Package-Unification Plan

**Status:** Deferred planning brief; package unification is not yet authorized
**Intended audience:** A new Codex task started after the component implementations have stabilized
**Package:** `varcomp_hdfe`
**Public commands:** `varcomp_kss`, `varcomp_ppml`, and `varcomp_targetset`

## 1. Purpose and timing

This document is the high-level handoff for eventually combining the linear
KSS and PPML TALO estimators into one Stata package. It records the intended
public direction and the guardrails for a future integration task. It is not a
detailed interface specification, a file-migration recipe, or authorization to
begin the work now.

The integration task should begin only after the owner explicitly authorizes
it and the then-current KSS and CMG handovers are stable enough to support a
coordinated migration. The new task must also recheck the current PPML TALO
status rather than assume that today's implementation remains unchanged.

At startup, the future task must follow the repository `AGENTS.md`, including
the mandatory handover checks and change-control rules. It should then read the
current plans, contracts, testing instructions, qualification reports, and
handover notes for:

- `kss_bc/`;
- `ppml_talo/`; and
- `shared/cmg/`.

Those component records, as they exist when the task starts, are authoritative
for estimator and solver details. This brief should not be used to override a
validated component contract or to infer that a pending candidate has become
production-ready.

Before editing code, the future task should record the source revisions being
integrated, identify any concurrent owners, inventory overlapping services and
public behavior, and present an execution plan for owner approval. Package
unification is a new software scope. It is not part of the completed numbered
proof program and does not reopen the frozen manuscript or releases.

## 2. Fixed direction

The following decisions are settled unless the owner explicitly reopens them:

| Item | Direction |
|---|---|
| Package identity | One Stata package named `varcomp_hdfe` |
| Linear estimator | Public command `varcomp_kss` |
| PPML estimator | Public command `varcomp_ppml` |
| Preparation and diagnostics | Optional public command `varcomp_targetset` |
| Package wrapper | No public `varcomp_hdfe` wrapper command |
| CMG | Shared internal numerical infrastructure, not a public command |

The two estimator commands should remain directly callable and recognizable as
different statistical procedures. `varcomp_targetset` should provide an
optional route for inspecting or preparing method-specific samples and
diagnostics; neither estimator should require users to run it first.

The shared `varcomp_` prefix identifies package membership. It does not require
the commands to expose identical options or conceal differences between the
linear KSS and PPML TALO estimators. Exact syntax, defaults, returned metadata,
and diagnostic presentation remain decisions for the future task.

CMG may eventually support the two-way fixed-effect numerical paths used by
both estimators. It should be incorporated only as an internal package service
after the relevant adapter and qualification gates pass. It should not become
a fourth public command or be described as a new estimator or correction.

## 3. Integration principles

### Preserve validated behavior

Package consolidation should not silently change an estimator. The current
component contracts control statistical estimands, target populations,
deletion units, weight semantics, sample construction, randomization, solver
tolerances, result withholding, and failure diagnostics. Changes to those
features require evidence, explicit discussion, and owner approval.

Numerical refactoring must continue to distinguish estimator behavior from
computational approximation. Numerical Monte Carlo error is not econometric
inference, and package integration does not authorize a new inference surface
or `e(V)`.

### Integrate incrementally

The future task should migrate and validate one bounded component at a time.
Each estimator should retain a comparison path until its package version has
passed equivalence, failure, installation, and relevant scale tests. Shared
services should be introduced only where their semantics are genuinely common.
Method-specific sample logic, assumptions, or diagnostics may remain separate.

Internal organization may change when that improves clarity, maintainability,
performance, or testing without changing public or statistical behavior. The
future task may choose the package layout, Mata namespaces, assembly approach,
adapter boundaries, and migration order based on the implementations and
evidence available at that time.

### Preserve provenance

Historical validation artifacts, review packets, benchmark reports, frozen
logs, and handovers should retain the command names and source paths under
which they were produced. Package migration should add traceable links from
new artifacts to their component sources rather than rewrite historical
evidence.

Licensing and distribution status must be rechecked before packaging or
release. Unification does not grant permission to redistribute imported,
unlicensed, restricted, or provenance-only material.

### Qualify CMG per estimator

The canonical shared CMG work lives under `shared/cmg/` during development.
Its existence does not by itself qualify a KSS or PPML runtime path. Each
estimator must preserve its own operator, quotient or grounding convention,
residual checks, random-state contract, failure semantics, and performance
gates.

The future task should decide whether and how CMG is exposed only after
package-specific adapter, equivalence, memory, portability, scale, and routing
evidence is current. An estimator may retain its existing preconditioner or
make CMG experimental if the evidence does not support broader use.

## 4. High-level work phases

### Phase A: Reground and reconcile

- Run the repository startup sequence and confirm the authorized scope.
- Read the current component contracts, plans, handovers, and qualification
  evidence.
- Record source commits, ownership boundaries, runtime dependencies, licensing
  constraints, and public behavior that must survive migration.
- Identify conflicts between component interfaces and distinguish statistical
  differences from naming or presentation differences.
- Resolve owner-level decisions before writing the integration implementation
  plan.

### Phase B: Design the unified package

- Choose the package layout, build and runtime boundaries, common versioning,
  installation structure, and migration order.
- Define the smallest useful shared services and keep estimator-specific logic
  behind explicit boundaries.
- Specify the initial behavior of `varcomp_targetset` from the mature component
  sample and diagnostic APIs.
- Decide compatibility aliases and transition policy based on actual external
  use rather than assuming they are needed.

### Phase C: Migrate the estimators

- Introduce `varcomp_kss` and `varcomp_ppml` incrementally, with component-to-
  package regression comparisons for every supported mode.
- Preserve complete failure and withholding behavior as well as successful
  point estimates.
- Update help, examples, package metadata, and user-facing terminology with
  each migrated command.
- Keep unrelated estimator improvements outside the migration unless they
  receive their own scope and validation.

### Phase D: Integrate qualified shared services

- Consolidate genuinely common utilities after equivalence tests establish
  that the shared implementation preserves both callers.
- Add CMG only to estimator paths whose current package-specific gates pass.
- Keep safe existing numerical routes available wherever CMG remains
  unqualified or unsuitable.
- Bind shared artifacts to the package build in a reproducible, drift-detecting
  way selected by the future task.

### Phase E: Validate and prepare the package

- Run component, cross-command, failure, clean-install, portability, and
  relevant scale gates.
- Confirm that direct estimator calls are self-contained and that optional
  target-set preparation cannot bypass final estimator validation.
- Reconcile package documentation with the implemented behavior and known
  limitations.
- Produce a completion report covering migrated sources, tests, provenance,
  unresolved limitations, and release readiness.

## 5. Decisions intentionally left open

The future task should make the following decisions from current evidence:

- exact command syntax, common option names, defaults, and returned metadata;
- the data model, storage, diagnostics, and reuse contract for
  `varcomp_targetset`;
- temporary compatibility aliases for the development command names;
- internal directory layout, Mata namespaces, loaders, build artifacts, and
  adapter boundaries;
- which services should be shared and which should remain method-specific;
- CMG option names, visibility, fallback policy, routing rules, and defaults;
- estimator migration order and the duration of comparison paths;
- installation, dependency, licensing, and release mechanics; and
- the exact qualification matrix required by the implementations available at
  integration time.

The implementer may revise internal and provisional choices when tests,
benchmarks, portability, or usability evidence supports the change. The task
must return to the owner before changing the package identity, fixed public
command surface, statistical estimator, supported dependence interpretation,
runtime dependency class, or the meaning of a reported result.

## 6. Completion conditions

Package unification is complete when:

- one installable `varcomp_hdfe` distribution contains the three public
  commands and all required runtime support;
- `varcomp_kss` and `varcomp_ppml` retain the validated behavior and failure
  semantics of the component versions selected for migration;
- `varcomp_targetset` is optional, documented, and consistent with the
  estimator-owned sample and diagnostic logic;
- common user-facing concepts are coherent without forcing false symmetry
  between the estimators;
- component regression suites, package integration tests, clean-install tests,
  and the repository gates pass;
- included shared services have cross-caller equivalence tests and documented
  ownership;
- no estimator enables an unqualified CMG path, and non-CMG routes remain
  available wherever required by the qualification evidence;
- help files and package metadata state assumptions, limitations, numerical
  approximations, and inference status accurately;
- historical evidence and source provenance remain traceable and unchanged;
  and
- the owner has reviewed the completion report and authorized any distribution
  or release step.

Meeting these software conditions does not establish that KSS or TALO
assumptions apply in a particular empirical setting, and it does not upgrade
the mathematical review status of the underlying theory.
