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
- Status at this checkpoint: complete measured optimization and target
  infeasibility assessment; no full target was run.

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
(median 27.992). The committed candidate warm times were 17.825, 17.848,
and 17.889 seconds (median 17.848), a 36.24-percent improvement.
Iterations and Schur/preconditioner actions remained 92 and 54,074. Maximum
exported-result relative difference was `1.11e-16`; complete residual and
target identity gates passed.

Local qualification passed the shared-CMG generator/Python/Mata/namespace
and hierarchy gates; 226 KSS Python tests; KSS quick/full Stata suites; clean
install; benchmark, oracle, and sample smokes; and the root handover, proof,
and paper checks. The failed early ablation that placed compensated grouping
inside every iterative operator action was stopped after 330 seconds and was
not retained. Recycled/block PCG and a compiled 32-bit kernel were not added:
the representation/operator bundle already clears the local gain threshold,
and SCC coefficients must establish a scale trigger first.

## SCC evidence and continuation

The immutable bundle has two matched CZ18 passes, fourteen validated
synthetic passes, and two externally validated censored density-four rungs.
Job 7214613 reached its registered 17,820-second application timeout. Job
7214616 and extended jobs 7218423/7218424 were stopped after that result made
the target decision irreversible. No job remains queued or running. The
maintained MATLAB comparison has eleven validated task-shape pairs; four
MATLAB PCG runs hit their iteration cap and are marked numerically unaccepted.

The final model and evidence manifest are under
`benchmarks/reports/evidence/KSS_NUMOPT_2_2026-08-18/model/`. Only the
80-million-cell `R/C=1` case passes 128-GiB admission, and it misses 48-hour
admission. Every other target misses both memory and wall admission. The next
thread should target density-four CMG hierarchy setup and then the repeated-
RHS segmented operator. An out-of-core raw lifecycle needs separate authority.
