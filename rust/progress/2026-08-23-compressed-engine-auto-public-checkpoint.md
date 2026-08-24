# Compressed `engine(auto)` public-route checkpoint

Date: 2026-08-23

## Implemented source

The public Rust router admits the no-control, match-deletion `engine(auto)`
tuple into the planned V4/V7 JLA runner. This admission remains deliberately
narrow:

- `backend(rust)` and `rng(counter_v1)` are explicit;
- `algorithm(jla)` and `engine(auto)` are explicit;
- match deletion and movers-only semantics are preserved;
- the request has no materialized controls;
- engine and solver routing are frozen before estimator RNG; and
- the native planner may select compressed only under its registered
  scientific eligibility rule.

The current substantive source checkpoint is
`4022118ff417337b5e3f9dc72604c4391e0125c1`.

## Reconciliation and test repairs

Qualification exposed several independent test and public-boundary defects.
The source now preserves the following contracts:

1. Compressed V7 reconciliation treats leverage and target batch phase modes
   independently rather than forcing both to equal one legacy aggregate mode.
2. A selected compressed result posts compressed-family names and receipts
   rather than generic-only fields.
3. Standalone qualifier execution explicitly loads `varcomp_kss.ado`.
4. Target strata remain distinct from coefficient cells and deletion units.
   The public fixture has 48 cells, 48 deletion units, and 96 target strata
   because its two stored rows per cell have distinct per-copy target masses.
5. The internal public-post harness creates its disposable marked-sample
   variable after freezing the caller data signature, so active `e(sample)`
   and raw-data restoration are tested simultaneously.
6. For `preconditioner(auto)`, the compressed V7 reconciler receives the
   native plan's actual selected route. A small quotient may select the
   registered exact/direct route; the request is not incorrectly forced into
   a diagonal receipt.
7. The public matrix now tests both sides of that registered rule: a small
   exact/direct case and a larger diagonal case, while forced CMG remains
   fail-closed and separately covered.
8. The older public-generic fixture now asserts the same exact/direct route for
   its small quotient instead of retaining a stale diagonal expectation.
9. The public-generic restoration check uses a Stata-compatible local macro
   name; the prior 33-character name failed before the state assertion could
   execute.

No solver threshold, fallback rule, residual gate, target definition, RNG
contract, or scientific estimator behavior was relaxed to obtain these
repairs.

## Evidence boundary

Exact source `acd76318b50105996cb9b74acf335d13e2cecf81` was tested by
`plugin-build` in run `32674661037`. Its normal-push Rust quick step passed,
and the arm64 public-generic route passed its engine, route, receipt, residual,
accounting, memory, RNG, and lifecycle assertions through the final restoration
check. Stata then rejected the 33-character local macro name
`compressed_preauto_sortedby_after` with RC 198. Source
`4022118ff417337b5e3f9dc72604c4391e0125c1` shortens only that test identifier.

The repaired source is not yet qualified. It still requires:

1. an exact-SHA `quick` receipt with Stata/process RC 0;
2. independent verification that the corresponding workflow Rust quick step
   passed; and
3. a successful exact-SHA `plugin-build` receipt covering Rust fmt, strict
   Clippy, workspace tests, C/ABI checks, native arm64, Rosetta x86_64,
   universal candidates, direct route tests, and clean installation.

Public distribution and the human license/provenance gate remain closed.

## Next step

Obtain exact quick evidence for this source descendant, then run the
comprehensive macOS qualifier. If it passes, record the exact qualified SHA
and run matrix in the principal progress/status documents and restore the
default requested profile to `quick`.
