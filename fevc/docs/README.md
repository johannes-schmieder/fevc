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
- [`RELEASE_HARDENING_2026-08-31.md`](RELEASE_HARDENING_2026-08-31.md):
  changed-surface review, evidence carry-forward, and final RC gate record.
- [`../AGENTS.md`](../AGENTS.md): mandatory agent constraints.
- [`DECISIONS.md`](DECISIONS.md): durable package, backend, routing, evidence,
  and release decisions.
- [`development_acceptance_v1.json`](development_acceptance_v1.json): active
  performance-first corrected-result equivalence and MATLAB-competitiveness
  policy for candidate promotion and differential development tests.
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
- [`INFERENCE.md`](INFERENCE.md): opt-in exact-observation high-rank and q=1
  component inference, exact observation/match fixed-effect projections, and
  the explicit sparse Rust/JLA block-projection route. Its focused architecture and scaling protocol are
  in the [archived scalable-projection record](../../docs/history/VCKSS_ARCHIVE.md#scalable-projection-and-inference).
- [`FAILURES_AND_RETURNS.md`](FAILURES_AND_RETURNS.md): typed failures and
  returned results/diagnostics.
- [`RUST_MATA_PARITY.md`](RUST_MATA_PARITY.md): generated current backend and
  platform parity ledger for the alpha release gates.
- [`../benchmarks/fevc_matlab_2026/STATUS.md`](../benchmarks/fevc_matlab_2026/STATUS.md):
  accepted 2026 comparative-scaling campaign checkpoint.

## Ownership and provenance

- [`SOURCE_PROVENANCE.md`](SOURCE_PROVENANCE.md): package source ledger.
- [Archived MATLAB package comparison](../../docs/history/VCKSS_ARCHIVE.md#development-result-reports):
  publication-era versus maintained MATLAB package comparison and the exact
  current VCkss benchmark pin.
- [`../../CODE_LICENSE.md`](../../CODE_LICENSE.md): repository licensing and
  release boundary.
- [`../cmg/README.md`](../cmg/README.md),
  [`../cmg/STATUS.md`](../cmg/STATUS.md), and
  [`../cmg/docs/SOURCE_PROVENANCE.md`](../cmg/docs/SOURCE_PROVENANCE.md):
  internal CMG component.

## Retained engineering results

These reports are source-bound evidence. Do not edit them to describe newer
source; add a new report or update the active plan instead.

- [`PREP_RHS_1`, `FE_BUF_1`, and `PREP_BND_1` archived results](../../docs/history/VCKSS_ARCHIVE.md#development-result-reports)

Additional exact-SHA evidence lives under `../../qualification/`, and
independent reviews live under `../../reviews/`.

The private full-CMG architectural spike and its rejected-promotion decision
are documented in
[`../benchmarks/full_cmg_spike/DECISION_REPORT.md`](../benchmarks/full_cmg_spike/DECISION_REPORT.md).
It is source-bound performance evidence, not a qualified public backend. Its
historical numerical gate remains recorded; active development interprets the
candidate under `development_acceptance_v1.json`.

The renewed route's registered hard-case decisions are the accepted
[`fixed-CZ18 P200 checkpoint`](../benchmarks/full_cmg_spike/CZ18_P200_MATRIX_CHECKPOINT.md)
and the non-promoted
[`synthetic P200 decision`](../benchmarks/full_cmg_spike/SYNTHETIC_P200_MATRIX_DECISION.md).
CZ18 clears the 2x target, while synthetic is 1.4803x MATLAB and modestly above
MATLAB peak RSS. The latter identifies official full-CMG repeated solves as
the next performance boundary and explicitly defers hardening and the
benchmark PDF.

## Historical records

Repository-level `../../docs/history/` and `../../docs/migration/` contain
immutable predecessor and migration records. Rust dated checkpoints live under
`../../rust/progress/`. They are not current development instructions.

## Automation boundary

Public GitHub workflows run source-only checks on hosted runners with read-only
repository permissions. Licensed Stata qualification remains an explicitly
local or private operation described in [`../TESTING.md`](../TESTING.md).
Historical exact-SHA receipts under `.ci/stata/results/` remain immutable
evidence, not active runner state.
