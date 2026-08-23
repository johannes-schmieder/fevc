# Compressed V4 result-family checkpoint

Date: 2026-08-23

## Source state

The private no-control match-deletion planned request now has a dedicated Stata regression at `varcomp_kss/tests/stata/test_rust_planned_compressed.do`.

The regression freezes the native V3/V4/V7 behavior for `engine(auto)` when the pre-RNG planner selects the compressed engine. In particular, it checks the compressed RHS receipt family rather than interpreting that family as generic RHS-V2, reconciles the V7 execution plan, verifies complete residual certificates and accounting identities, and certifies lifecycle release back to idle.

Commit `f61081a575881335e74a02369c39c9880d81cc52` additionally wires this regression into the comprehensive macOS qualifier for:

- thin arm64;
- universal plugin on arm64;
- thin x86_64 under Rosetta; and
- universal plugin under Rosetta.

It also updates qualifier scope and tested-route receipts so compressed V4/V7 coverage is claimed only when that test actually runs.

## Qualification boundary

This is a private native-boundary checkpoint, not public-route qualification. The public e-class planned runner still assumes the generic RHS-V2 family and therefore still withholds compressed-eligible no-control match-deletion `engine(auto)`.

The transformed source must receive its own exact-SHA quick receipt. A later comprehensive `plugin-build` run is required after public family-aware reconciliation before the compressed public route may be called qualified.

## Next milestone

Refactor the public planned result reconciler and posting layer to branch on the frozen selected engine and RHS schema:

1. retain the existing generic RHS-V2 reconciliation unchanged for selected generic;
2. add a separate compressed RHS-V1 reconciliation using V7 as the authoritative plan receipt;
3. post only family-valid diagnostics rather than manufacturing generic control or memory receipts for compressed results;
4. add public route, result, accounting, and state-restoration tests; and
5. only then admit compressed-eligible no-control match-deletion `engine(auto)` through the public option router.
