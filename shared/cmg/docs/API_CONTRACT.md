# CMG-MATA-V1 standalone API contract

## Status and boundary

This document describes the local standalone API generated from
`shared/cmg/src/cmg_core.mata.in`. The API level is `3`. It is not installed in
`ppml_talo` or `kss_bc`, and no package runtime may call it until the relevant
package owner hands off the files and the package-specific tests pass.

All identifiers are instantiated from `@CMG_NS@`. The current generated
namespaces are `cmgtest`, `ppmltalo_cmg`, and `kssbc_cmg`. The core creates no
Mata globals and does not read or advance Stata's RNG.

## Required preparation sequence

1. Call `@CMG_NS@__cells_prepare(worker, firm, weight, worker_key, firm_key)`.
   Worker and firm indices must be dense positive integers. Keys must be
   finite, unique within type, and invariant to semantic relabeling. Weights
   must be finite and strictly positive. Duplicate worker--firm cells are
   summed after one recorded power-of-two normalization.
2. Call `@CMG_NS@__preflight(cells, planned_rhs,
   canonical_keys_available, memory_envelope_bytes, options)` before estimator
   RNG initialization. A converged result may still select `DIAGONAL`; only
   `PILOT_DIAGONAL` authorizes deterministic diagonal pilots.
3. If pilots justify setup, call `@CMG_NS@__hybrid_build(cells)` and then
   `@CMG_NS@__hierarchy_build(graph, options)`. Every non-`CONVERGED` status is
   a typed setup failure. The caller may fall back only before estimator RNG
   initialization and only under package policy.
4. Use `@CMG_NS@__route_decide(...)` with exactly four deterministic pilot
   statuses and iteration counts. `AUTO` selects CMG only if all CMG pilots
   converge within the cap, the iteration gate passes (or diagonal is capped),
   and calibrated CMG work is at most 80% of diagonal work. Forced CMG failure
   is never converted into diagonal success.
5. Apply the selected fixed preconditioner inside ordinary per-column PCG.
   The core result is not a convergence certificate. The package must retain
   its original operator, recurrence checks, and fresh complete-system
   residual gate.

## Application entry points

`@CMG_NS@__apply(hierarchy, right_hand_side)` applies one fixed symmetric
V-cycle to one or more hybrid-graph columns. This is the selected Stata 18
path. It returns an `@CMG_NS@__apply_result` containing `status`, `message`,
`value`, `levels_visited`, and `edge_passes`.

`@CMG_NS@__apply_kss(hierarchy, firm_rhs)` implements the KSS component-quotient
congruence. Its input and output have one row per firm.

`@CMG_NS@__apply_ppml(hierarchy, grounded_rhs, ground)` implements the adjoint
grounded PPML pullback. It currently requires exactly one connected component;
the input and output omit the stated firm ground.

`@CMG_NS@__workspace_init(hierarchy, batch_capacity, memory_cap_bytes)` and
`@CMG_NS@__workspace_apply(...)` implement a preallocated bounded workspace.
They are retained for regression, dirty-reuse checks, and future platform
calibration. They are not the selected Stata 18 runtime path because the
current implementation was materially slower than `__apply()`.

`@CMG_NS@__diagnostics(hierarchy)` returns level counts, fine dimensions,
component count, edge/vertex complexity, structural and dense-factor byte
forecasts, and a per-level table. Byte fields exclude Mata allocator overhead
and are not peak RSS.

## Default numerical options

| Field | Default | Meaning |
|---|---:|---|
| `kappa` | 8 | high-degree conductance threshold |
| `target_size` | 4 | deterministic aggregate target |
| `aggregate_cap` | 8 | hard aggregate-size cap |
| `min_reduction` | 0.20 | minimum vertex reduction per nonterminal level |
| `max_edge_complexity` | 3 | cumulative edge-complexity cap |
| `max_vertex_complexity` | 4 | cumulative vertex-complexity cap |
| `max_levels` | 32 | hierarchy-depth cap |
| `coarse_max` | 128 | largest component sent to terminal factorization |
| `omega` | 2/3 | constrained Jacobi weight |
| `action_scratch_bytes` | 64 MiB | transient graph-action cap |
| `construction_scratch_bytes` | 1.5 GiB | preflight construction cap |
| `dense_factor_bytes` | 64 MiB | terminal factor-storage cap |

`@CMG_NS@__options_resource(memory_envelope_bytes, fine_vertices,
planned_rhs)` derives a deterministic memory-rich profile. It caps graph-action
scratch at 1 GiB, construction scratch at 8 GiB, and dense-factor storage at
512 MiB while preserving caller headroom. API 3 selects
`coarse_max=fine_vertices` only when there are at least 512 planned RHSs, at
least 16 GiB of declared memory, at most 1,536 fine vertices, and the predicted
dense factor fits `dense_factor_bytes`. The factor allocation is checked again
against actual components before allocation. This is a bounded repeated-RHS
CPU optimization; it does not form an observation-square or
observation-parameter matrix. Otherwise, for at least 2,048 fine vertices and
at least 4 GiB, the profile raises `coarse_max` to 256. The 128 default remains
in force for smaller graphs because a wider recursive terminal did not pass
the registered low-RHS small-barbell no-regression gate. The core processes all
RHS columns in one graph-action call, with the arc-row chunk derived from
`action_scratch_bytes`.

Changing target size, aggregate cap, sweep count, or smoother within an active
solve would change the fixed preconditioner contract and is not supported.

## Status rules

Only exact string status `CONVERGED` denotes a valid returned object or
application. An empty or nonfinite result is always a failure. Representative
typed failures include invalid identifiers/keys/RHSs, unsafe weight ranges,
allocation or complexity limits, changed component counts, stalled hierarchy,
terminal Cholesky or residual failure, incompatible singleton RHSs, pullback
failure, and preconditioner breakdown.

After estimator RNG initialization, no CMG application failure may trigger a
route switch. The package must withhold the affected calculation. Before RNG
initialization, `AUTO` may rebuild package solver state under diagonal only as
specified by the package adapter contract.

## Assembly contract

Run `./.venv/bin/python shared/cmg/tools/assemble.py --all` only during
development. Release and validation paths use `--all --check`. Each artifact
header and `generated/manifest.json` bind generator API, namespace, canonical
template SHA-256, generated-section SHA-256, and complete-artifact SHA-256.
Package loaders must additionally bind their own package API/build identifiers;
that loader work is not yet implemented.
