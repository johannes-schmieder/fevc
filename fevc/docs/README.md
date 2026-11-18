# Documentation

Start with the [usage guide](../README.md), [installation](../../INSTALLATION.md),
and the installed `help fevc`.

## Using the command

- [Estimator and sample](ESTIMATOR_CONTRACT.md): targets, controls, weighting,
  deletion, and retained population.
- [Outcome centering](CENTERING.md): None/Mean/Corrected, exact/JLA,
  Mean projection, Stata's frequency-weighted mean, the KSS mean
  distinction, MCSE, native availability and local timing evidence.
- [Inference](INFERENCE.md): the repeat-measurement standard errors of the
  components (the default), block projections, assumptions, simulations and
  limitations, and the models removed in 0.6.0. The derivation and the
  comparison with KSS are in the
  [standard-error document](../../docs/feasible-standard-errors.pdf).
- [Pooled mover–stayer inference](POOLED_COMPONENT_INFERENCE.md) and its
  [development assessment](POOLED_COMPONENT_ASSESSMENT.md): design records of
  the structured models removed from the command in 0.6.0.
- [Memory](MEMORY.md): optional budgets, forecasts, and returned diagnostics.
- [Failures and returned results](FAILURES_AND_RETURNS.md).
- [Changelog](../CHANGELOG.md).

## Methods and implementation

- [Numerical architecture](NUMERICAL_ARCHITECTURE.md).
- [Finite-projection JLA](JLA_FINITE_PROJECTION.md).
- [Canonical MCSE stored results](MCSE_RETURNS_2026-09-30.md), the dated
  baseline record; [current returns](FAILURES_AND_RETURNS.md) include
  centering metadata and [centering assumptions](CENTERING.md).
- [Default MCSE interface](MCSE_DEFAULT_INTERFACE_2026-09-30.md),
  [all-point-probe MCSE derivation](ALL_PROBE_MCSE.md) and
  [implementation status](ALL_PROBE_MCSE_STATUS.md).
- [Control-basis certification](CONTROL_BASIS_CERTIFICATION.md) and
  [block-control derivation](BLOCK_CONTROL_DERIVATION.md).
- [Matrix-free component inference](MATRIX_FREE_COMPONENT_INFERENCE.md) and
  [individual inference interface](INDIVIDUAL_INFERENCE_INTERFACE.md): design
  records of the structured models removed in 0.6.0.
- [Backend capabilities](RUST_MATA_PARITY.md).
- [Design decisions](DECISIONS.md).

The dated validation, installation and qualification reports below retain
their original tested sources and payloads. Current centering semantics are in
[CENTERING.md](CENTERING.md); exact candidate qualification and publication
status are in [native provenance](../../native/README.md).

## Development

- [Contributing](../../CONTRIBUTING.md) and [testing](../TESTING.md).
- [Current checkpoint](../PLAN.md).
- [Accepted Mean component inference plan](MEAN_COMPONENT_INFERENCE_PLAN.md):
  implementation, independent algebra, bounded sampling assessment, companion
  technical note and exact-source platform qualification.
- [Repeat-measurement standard errors: method note](repeat_measurement/README.md):
  the note of November 13, 2026 that derived the method of
  [INFERENCE.md](INFERENCE.md) for a prototype (LaTeX source and PDF), its
  control-term addendum of November 14, the prototype's code, aggregate
  results and development log, and where the command differs from the note.
- [Feasible Standard Errors for Leave-Out Variance Components](../../docs/feasible-standard-errors.pdf)
  derives the intervals used by the command and compares them with the KSS
  standard errors. [LaTeX source](../../docs/feasible-standard-errors.tex).
- [Standard errors: technical supplement](standard_errors/standard_errors.tex): the
  leave-out residual method, removed before release, whose pass (pseudo-noise
  trace, control term, cost and memory accounting) the repeat-measurement
  standard errors of [INFERENCE.md](INFERENCE.md) share; its assumptions,
  diagnostics, simulation evidence and implementation status. LaTeX source;
  build with `latexmk -pdf`.
- [Automatic standard errors: proposal](AUTOMATIC_INFERENCE_PROPOSAL.md): diagnosis of the
  earlier inference routes, the design choice and the first prototype evidence. The
  supplement supersedes its implementation plan.
- [Mean component inference assessment](MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md):
  complete attempt accounting, 22 retained exact-Mata availability failures,
  bounded native screens and material numerical sensitivity.
- [Mean component sampling registration](mean_component_inference_validation_v1.json),
  [pre-assessment harness correction](mean_component_inference_validation_v1_amendment1.json)
  and [fixed-outcome numerical registration](mean_component_inference_numerical_v1.json).
- [Default MCSE interface validation](MCSE_DEFAULT_VALIDATION_2026-09-30.md).
- [MCSE native publication and installation](MCSE_NATIVE_INSTALL_2026-09-30.md).
- [All-probe MCSE optimization and qualification](ALL_PROBE_OPTIMIZATION_2026-09-30.md).
- [Mata all-probe MCSE performance and validation](MATA_ALL_MCSE_OPTIMIZATION_2026-09-30.md).
- [Model-operator interrupt experiment](INTERRUPT_CHUNKS_2026-09-29.md).
- [Review fixes and deferred proposals](REVIEW_FIXES_2026-09-29.md).
- [Control-span reliability candidate](CONTROL_SPAN_REPAIR_2026-09-28.md).
- [Control-basis repair qualification](CONTROL_BASIS_REPAIR_2026-09-27.md).
- [Deletion-unit mover integration](DELETION_UNIT_MOVERS_2026-09-26.md).
- [Subsample repair and compatibility](SUBSAMPLE_REPAIR_2026-09-26.md).
- [Development acceptance policy](development_acceptance_v1.json).
- [Rust backend](../../rust/README.md) and [CMG component](../cmg/README.md).
- [Benchmark harnesses](../benchmarks/README.md).
- [Source provenance](SOURCE_PROVENANCE.md) and [code licensing](../../CODE_LICENSE.md).
- [Native distribution preparation](RC_BINARY_PAYLOAD.md).
- [Historical material](../../docs/ARCHIVE.md).
