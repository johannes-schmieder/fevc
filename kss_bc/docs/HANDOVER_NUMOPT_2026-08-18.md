# KSS-NUMOPT-2 handover

## Checkpoint

- Milestone: `KSS-NUMOPT-2` / Optimization III.
- Owner authorization: implement `Optimization 3.md`, 2026-08-18.
- Branch/worktree: `main`, current worktree.
- Immutable Optimization II commit:
  `3fbec9f9393dd3bf5ce45bfbb7e1e03d74e954e4`.
- Immutable Optimization II bundle SHA-256:
  `c69159f6dc1278159e4f30b53580308457d6f4a47c63502c0da31478889a455d`.
- Proof milestone: none. M0--M13 and every protected paper/proof/state path
  remain unchanged.
- Status at this checkpoint: implementation and local qualification complete;
  source-bound SCC scale matrix and target forecasts pending.

## Retained implementation

1. Production KSS and generated KSS CMG artifacts use target-specific
   `matalnum off`. PPML and standalone CMG tests remain `on`; generated
   `numeric_mode()` and package loader checks reject stale modes.
2. Hot column/dot reductions use quad operations. Identity scatter bypasses
   aggregation; general scatter remains stable. Canonical IDs, row maps,
   key panels, pair codes, and physical panels use vectorized construction.
3. Unit and stratum RNG identities are exact numeric canonical ranks. The
   registered runtime/seed/domain/probe/atom ordering and golden draws are
   unchanged; caller state restoration remains a hard gate.
4. One canonical cell payload supports worker-major and firm-major orders.
   The generic routed solver receives a compact external FE view with
   callbacks for transpose, prediction, Schur action, reconstruction,
   diagonal application, and the full original-equation certificate.
5. Cancellation-sensitive preprocessing uses a deterministic 65,536-group
   tiled Neumaier reducer. Positive integer totals may use native segmented
   sums only under the exact `<2^53` mass bound. Repeated operator aggregation
   uses native panels and retains the complete residual acceptance gate.
6. The ado reuses initial worker/firm graph maps and the graph deletion map,
   removing repeated full-data grouping passes. Retained worker/firm IDs are
   still redensified after fixed-point pruning.
7. Resource API 8 charges `8*(9*C+3*W+4*F)` persistent cell/order/panel bytes,
   `32*G` deletion-unit bytes, `32*S` target-stratum bytes, and
   `8*5*(W+F)` compact routed working-vector bytes. Raw, transition,
   hierarchy, phase scratch, output, restoration, and process-residency
   families remain separate.

## Local evidence

The matched four-processor P200 fixture has 60,000 stored rows, 10,000
workers, 1,000 firms, 30,000 cells/deletion units, 33,529 target strata, seed
8,675,309, batch 16, diagonal routing, and 601 complete right-hand sides.
Optimization II warm command times were 27.992, 28.003, and 27.974 seconds
(median 27.992). The final pre-commit candidate warm times were 17.833,
17.916, and 17.910 seconds (median 17.910), a 36.0-percent improvement.
Iterations and Schur/preconditioner actions remained 92 and 54,074. Maximum
exported-result relative difference was `1.11e-16`; complete residual and
target identity gates passed.

Local qualification passed the shared-CMG generator/Python/Mata/namespace
and hierarchy gates; 211 KSS Python tests; KSS quick/full Stata suites; clean
install; benchmark, oracle, and sample smokes; and the root handover, proof,
and paper checks. The failed early ablation that placed compensated grouping
inside every iterative operator action was stopped after 330 seconds and was
not retained. Recycled/block PCG and a compiled 32-bit kernel were not added:
the representation/operator bundle already clears the local gain threshold,
and SCC coefficients must establish a scale trigger first.

## SCC continuation

Use the committed clean source bundle. Submit the matched baseline/candidate
CZ18 P200 jobs with the existing restricted-data harness. Submit the
independent synthetic matrix with `submit_numopt2_scale.sh`: strong P20
density 2/3/4 at 1/64, 1/32, and 1/16; strong central density 3 at 1/8; weak
central cases at 1/32 and 1/16; and strong central R/C=8 at 1/64, 1/32, and
1/16. Admit 120-million-row preprocessing only after inspecting the 60-million
row measurement. Collect qacct only after completion and validate every task
with `validate_numopt2_scale.py`; never infer success from qstat absence.

Fit stage-specific byte and wall coefficients on 1/64--1/16. Reserve 1/8 for
out-of-sample error. Forecast P200 low/central/high targets separately for the
compressed numerical engine and R/C=1/8/26.3 raw-input transition, with at
least 20-percent resource headroom and explicit model-error ranges. Do not
run the full target automatically.
