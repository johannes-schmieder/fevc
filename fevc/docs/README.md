# Documentation index

Start with the [package guide](../README.md), [current checkpoint](../PLAN.md),
[test guide](../TESTING.md), and [changelog](../CHANGELOG.md).
The [historical evidence index](EVIDENCE_INDEX.md) retains earlier reports and
registrations, including failed campaigns and superseded implementation plans.

## Active contracts

- [Estimator](ESTIMATOR_CONTRACT.md): sample, targets, weighting and deletion.
- [Numerical architecture](NUMERICAL_ARCHITECTURE.md),
  [control-basis certification](CONTROL_BASIS_CERTIFICATION.md),
  [block-control derivation](BLOCK_CONTROL_DERIVATION.md), and
  [finite-projection JLA](JLA_FINITE_PROJECTION.md): numerical safeguards.
- [Inference](INFERENCE.md),
  [component inference](MATRIX_FREE_COMPONENT_INFERENCE.md), and
  [individual inference interface](INDIVIDUAL_INFERENCE_INTERFACE.md): supported
  requests, scientific scope and approximate-inference limitations.
- [Memory forecasts](MEMORY.md): optional budgets, warning/error/off policy,
  forecast timing, returned diagnostics and accuracy limits.
- [Failures and returns](FAILURES_AND_RETURNS.md): typed withholding and results.
- [Backend parity](RUST_MATA_PARITY.md): capability evidence and platform gaps.
- [Development acceptance](development_acceptance_v1.json) and
  [decisions](DECISIONS.md): registered comparison and promotion rules.

## Latest source-bound checkpoints

- [256-lane control preparation](CONTROL_LANES_256_2026-09-10.md) and
  [result record](control_lanes_256_v1_result.json): repaired audit discrepancies,
  numerical/lifecycle/memory tests and measured performance at Mata API 24.
  The [original repair](CONTROL_BASIS_REPAIR_2026-09-10.md) and
  [its result](control_basis_repair_v1_result.json) preserve the earlier failures
  and 13.2% overhead before the follow-up.
- [Memory implementation](MEMORY_FORECAST_2026-09-10.md),
  [result record](memory_forecast_v1_result.json), and
  [documentation compatibility](MEMORY_DOCUMENTATION_2026-09-10.md).
- [Inference completion](INFERENCE_COMPLETION_2026-09-08.md) and
  [result record](inference_completion_v1_result.json): 2,048 direct residual
  Gram probes, local Mac checks and replay within the documented approximate
  inference scope. Historical calibration failures remain failures.
- [Platform follow-up](INFERENCE_PLATFORM_FOLLOWUP_2026-09-08.md): Linux PASS
  at `f3098bc`, Windows smoke failure and incomplete private binary payload.
  These receipts do not qualify later runtime changes on those platforms.

## Public source and distribution

[Public-source preparation](PUBLIC_SOURCE_READINESS.md) records the cleanup,
history audit and remaining boundary. [Source provenance](SOURCE_PROVENANCE.md)
and [code licensing](../../CODE_LICENSE.md) govern the repository;
[RC binary preparation](RC_BINARY_PAYLOAD.md) describes the separate native
artifact and installation requirements.

Public GitHub workflows use hosted runners with read-only repository
permissions. Licensed Stata checks remain local or explicitly private.
See the [Rust guide](../../rust/README.md), [native test plan](../../rust/TEST_PLAN.md),
and [CMG guide](../cmg/README.md) for their component workflows.

Immutable predecessor and migration records remain under
[`docs/history/`](../../docs/history/) and [`docs/migration/`](../../docs/migration/).
Exact-source receipts, reviews, benchmark evidence and Rust progress snapshots
remain at their original paths; none is an instruction to rerun an old campaign.
