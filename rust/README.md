# Native backend

This directory contains the Rust backend for `fevc`. Users of the complete
binary distribution will not need Rust or a compiler. For installation, see
[the package instructions](../INSTALLATION.md).

## Source layout

- `crates/vckss-core/`: estimator, graph, exact/JLA algorithms, solvers,
  planning, memory accounting, and inference.
- `crates/vckss-plugin/`: native lifecycle and versioned ABI.
- `stata_backend/`: Stata C shim, platform builds, and installation checks.
- `vendor/cmg/`: CMG source with its upstream license and provenance.
- `experiments/`: independent research adapters and development harnesses.

Private internal identifiers retain their established names and ABI meanings.
`fevc` is the public package and command.

## Outcome centering

The core supports None, Mean and Corrected in exact and JLA point execution.
Mean changes only the retained working-outcome correction factor. Corrected
exact shares one coefficient-space system; Corrected JLA uses the existing
full/two-half leverage pools. Its numerical covariance is Mean's with the
increment fixed. Centering API 1 configures a prepared generation before
solve independently of numerical API 2 and without changing old ABI layouts.

The repository Mac arm64, Rosetta x86-64, universal and Linux x86-64 plugins pass
full platform and explicit Rust centering checks. Windows remains unchanged
and lacks active centering while its build/private qualification are pending. See
[the centering contract](../fevc/docs/CENTERING.md) and
[the Stata boundary](stata_backend/README.md) for lifecycle and restrictions.

## Development

Use the pinned toolchain and locked dependencies. See [TEST_PLAN.md](TEST_PLAN.md)
for affected-surface gates and [the native build guide](stata_backend/README.md)
for platform requirements. The current checkpoint is maintained in
[fevc/PLAN.md](../fevc/PLAN.md).

The core provides exact, compressed JLA, and generic JLA result families.
Backend selection, capability checks, memory admission, RNG planning, original
system residual certification, and caller-state restoration follow the
[numerical contract](../fevc/docs/NUMERICAL_ARCHITECTURE.md) and
[development acceptance policy](../fevc/docs/development_acceptance_v1.json).
See also [RNG_CONTRACT.md](RNG_CONTRACT.md),
[memory planning](../fevc/docs/MEMORY.md), and
[inference](../fevc/docs/INFERENCE.md).

Current feature and platform status is in the
[capability ledger](../fevc/docs/RUST_MATA_PARITY.md). Historical results qualify
only their recorded sources; they do not qualify the current worktree.

The code is GPL-3.0-only, subject to [source provenance](SOURCE_PROVENANCE.md)
and the included third-party notices.
