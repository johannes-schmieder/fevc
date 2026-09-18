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
