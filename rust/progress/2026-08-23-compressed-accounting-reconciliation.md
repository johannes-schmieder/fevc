# Compressed accounting reconciliation checkpoint

Date: 2026-08-23

Status: implementation landed; source-bound qualification pending.

## Defect isolated

The macOS plugin qualifier reached the native compressed V7 result and failed in the Stata reconciliation layer. The native solve, residual, memory, route, and Counter-V1 receipts were otherwise admissible.

The reconciler incorrectly required two diagnostics with different schemas to be numerically equal:

- the legacy `accounting_residual`, which is normalized by each result row's component scale; and
- `actual_accounting_residual`, which is the maximum absolute result identity residual.

## Repair

Commit `4c85d0768dbfeab0a0868936fabe5e6489984c39` recomputes both truths from the immutable 4-by-4 compressed result, validates each native diagnostic against its matching definition, and removes the invalid cross-equality assumption. No estimator, solver, graph, RNG, route-selection, or posting formula changed.

The private planned-compressed test now independently checks both reconciled diagnostics.

## Qualification boundary

The next source-bound checks are:

1. licensed Stata and Rust quick gates;
2. the macOS plugin-build qualifier for thin and universal arm64 binaries;
3. Rosetta x86_64 cases when available;
4. isolated clean-install coverage of the compressed reconciler, poster, and public automatic-diagonal and forced-CMG routes.

No claim is made here for public release, `backend(auto)` selecting Rust, `algorithm(auto)`, stayers, Windows, Linux, native Intel hardware, broad numerical parity, or production-scale performance.
