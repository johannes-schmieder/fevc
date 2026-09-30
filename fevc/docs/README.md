# Documentation

Start with the [usage guide](../README.md), [installation](../../INSTALLATION.md),
and the installed `help fevc`.

## Using the command

- [Estimator and sample](ESTIMATOR_CONTRACT.md): targets, controls, weighting,
  deletion, and retained population.
- [Inference](INFERENCE.md): supported requests, assumptions, and limitations.
- [Memory](MEMORY.md): optional budgets, forecasts, and returned diagnostics.
- [Failures and returned results](FAILURES_AND_RETURNS.md).
- [Changelog](../CHANGELOG.md).

## Methods and implementation

- [Numerical architecture](NUMERICAL_ARCHITECTURE.md).
- [Finite-projection JLA](JLA_FINITE_PROJECTION.md).
- [Canonical MCSE stored results](MCSE_RETURNS_2026-09-30.md).
- [Default MCSE interface](MCSE_DEFAULT_INTERFACE_2026-09-30.md),
  [all-point-probe MCSE derivation](ALL_PROBE_MCSE.md) and
  [implementation status](ALL_PROBE_MCSE_STATUS.md).
- [Control-basis certification](CONTROL_BASIS_CERTIFICATION.md) and
  [block-control derivation](BLOCK_CONTROL_DERIVATION.md).
- [Matrix-free component inference](MATRIX_FREE_COMPONENT_INFERENCE.md) and
  [individual inference interface](INDIVIDUAL_INFERENCE_INTERFACE.md).
- [Backend capabilities](RUST_MATA_PARITY.md).
- [Design decisions](DECISIONS.md).

## Development

- [Contributing](../../CONTRIBUTING.md) and [testing](../TESTING.md).
- [Current checkpoint](../PLAN.md).
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
