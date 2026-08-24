# Documentation index

This directory separates active contracts from source-bound engineering
evidence. Start with the active package files before reading a historical
report.

## Active package guidance

- [`../README.md`](../README.md): command overview, backend routing,
  installation, and current development boundary.
- [`../PLAN.md`](../PLAN.md): authoritative current milestone and next steps.
- [`../TESTING.md`](../TESTING.md): local, Stata, plugin, SCC, and evidence
  taxonomy.
- [`../CHANGELOG.md`](../CHANGELOG.md): user- and developer-visible changes.
- [`../AGENTS.md`](../AGENTS.md): mandatory agent constraints.
- [`DECISIONS.md`](DECISIONS.md): durable package, backend, routing, evidence,
  and release decisions.
- [`../../rust/README.md`](../../rust/README.md) and
  [`../../rust/TEST_PLAN.md`](../../rust/TEST_PLAN.md): current native backend
  architecture and qualification gates.

## Scientific and numerical contracts

- [`ESTIMATOR_CONTRACT.md`](ESTIMATOR_CONTRACT.md): estimator, population,
  deletion, weighting, nuisance, and target definitions.
- [`NUMERICAL_ARCHITECTURE.md`](NUMERICAL_ARCHITECTURE.md): model systems,
  quotient conventions, solvers, residual certification, and scale engines.
- [`BLOCK_CONTROL_DERIVATION.md`](BLOCK_CONTROL_DERIVATION.md): controlled block
  deletion derivation and rank conditions.
- [`JLA_FINITE_PROJECTION.md`](JLA_FINITE_PROJECTION.md): improved-JLA
  finite-projection correction.
- [`FAILURES_AND_RETURNS.md`](FAILURES_AND_RETURNS.md): typed failures and
  returned results/diagnostics.
- [`RUST_MATA_PARITY.md`](RUST_MATA_PARITY.md): generated current backend and
  platform parity ledger for the alpha release gates.

## Ownership and provenance

- [`SOURCE_PROVENANCE.md`](SOURCE_PROVENANCE.md): package source ledger.
- [`../../CODE_LICENSE.md`](../../CODE_LICENSE.md): repository licensing and
  release boundary.
- [`../cmg/README.md`](../cmg/README.md),
  [`../cmg/STATUS.md`](../cmg/STATUS.md), and
  [`../cmg/docs/SOURCE_PROVENANCE.md`](../cmg/docs/SOURCE_PROVENANCE.md):
  internal CMG component.

## Retained engineering results

These reports are source-bound evidence. Do not edit them to describe newer
source; add a new report or update the active plan instead.

- [`PREP_RHS_1_RESULTS_2026-08-19.md`](PREP_RHS_1_RESULTS_2026-08-19.md)
- [`FE_BUF_1_RESULTS_2026-08-19.md`](FE_BUF_1_RESULTS_2026-08-19.md)
- [`PREP_BND_1_RESULTS_2026-08-19.md`](PREP_BND_1_RESULTS_2026-08-19.md)

Additional exact-SHA evidence lives under `../../qualification/`, and
independent reviews live under `../../reviews/`.

## Historical records

Repository-level `../../docs/history/` and `../../docs/migration/` contain
immutable predecessor and migration records. Rust dated checkpoints live under
`../../rust/progress/`. They are not current development instructions.

## CI operations

The licensed Mac runner and exact-SHA receipt mechanism are documented in
[`../../STATA_CI_RUNNER.md`](../../STATA_CI_RUNNER.md).

Trusted transformation files under `.ci/codex/` are single-use transport. A
clean handoff has no staged apply script, patch, commit-message file, or failed
apply receipt.
