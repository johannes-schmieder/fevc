# CMG-MATA API 6 contract

## Status and boundary

This document describes the API generated from
`shared/cmg/src/cmg_core.mata.in`. The canonical API level is `6`. CMG-MATA-1
ships the `kssbc_cmg` namespace in the internal `kss_bc` production candidate;
`ppml_talo` does not install or call it in this milestone.

All identifiers are instantiated from `@CMG_NS@`. The current generated
namespaces are `cmgtest`, `ppmltalo_cmg`, and `kssbc_cmg`. The core creates no
Mata globals and does not read or advance Stata's RNG.

API 6 is a GPL-3.0-only, source-informed Mata derivative of the official CMG
hierarchy routines. It contains no plugin, MEX file, executable, subprocess,
or binary interchange. Generated namespace files are regenerated,
hash-checked, and loader-bound before package integration and every
source-bound SCC bundle. API 5 remains callable internally through
`@CMG_NS@__hierarchy_v5()` only as a measurement and rollback reference.

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

`@CMG_NS@__diagnostics(hierarchy)` returns committed-level counts, fine
dimensions, component count, edge/vertex complexity, structural and
dense-factor byte forecasts, and a per-level table. API 6 retains API 5's
`hierarchy_status`, `hierarchy_message`, and attempted-level diagnostics
when hierarchy construction failed after a valid level attempt. The attempted
table columns are attempt number, vertices, edges, components, proposed coarse
vertices, component-surplus reduction, cumulative edge complexity, cumulative
vertex complexity, and terminal flag. Parallel string vectors preserve the
exact attempted status, message, and method. Diagnostic extraction itself
returns `CONVERGED`; callers must inspect `hierarchy_status` before treating the
hierarchy as usable. Byte fields exclude Mata allocator overhead and are not
peak RSS.

## Default numerical options

| Field | Default | Meaning |
|---|---:|---|
| `kappa` | 8 | high-degree conductance threshold |
| `target_size` | 4 | deterministic aggregate target |
| `aggregate_cap` | 8 | hard aggregate-size cap |
| `min_reduction` | 0.20 | minimum vertex reduction per nonterminal level |
| `max_edge_complexity` | 3 | cumulative edge-complexity cap |
| `max_vertex_complexity` | 5 | cumulative vertex-complexity cap |
| `max_levels` | 96 | hierarchy-depth cap |
| `coarse_max` | 128 | largest component sent to terminal factorization |
| `omega` | 2/3 | constrained Jacobi weight |
| `action_scratch_bytes` | 64 MiB | transient graph-action cap |
| `construction_scratch_bytes` | 1.5 GiB | preflight construction cap |
| `dense_factor_bytes` | 64 MiB | terminal factor-storage cap |

`@CMG_NS@__options_resource(memory_envelope_bytes, fine_vertices,
planned_rhs)` derives a deterministic memory-rich profile. It caps graph-action
scratch at 1 GiB, construction scratch at 8 GiB, and dense-factor storage at
512 MiB while preserving caller headroom. API 6 retains API 5's terminal policy and selects
`coarse_max=fine_vertices` only when there are at least 512 planned RHSs, at
least 16 GiB of declared memory, at most 6,144 hybrid vertices, and the predicted
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

At each hybrid or sparse-quotient nonterminal level, API 6 constructs a
canonical heaviest-neighbor profile, roots mutual pairs, bounds forest height,
cuts branches with more than two vertices on both sides, applies the official
one-eighth selected-tree repair, and packs the resulting components densely.
All of these steps are Mata bulk operations. A dense ordinary quotient with
no hybrid auxiliaries and `E/V>8` uses API 5's screened forest directly: on
that regime the one-eighth repair would detach most single-edge nominations
before repeating the same work in the fallback. This specialization preserves
degree-two and degree-three performance.

Reduction is measured on component surplus, `(V-C)`, so singleton components
do not distort the gate. If the selected primary aggregation misses the fixed
20% bound, the core tries API 5's deterministic component-aware normalized
heavy-edge fallback. It greedily matches edges by descending
`w/sqrt(d_u d_v)`, then descending raw weight and canonical endpoint keys, and
packs unmatched vertices by canonical key within the same certified
component, with the existing cap of eight. No path adds an edge, ridge,
diagonal shift, or weight floor. Failure to meet the same reduction bound
remains typed `HIERARCHY_STALLED`.

Preflight reports a conservative linear construction scratch forecast
`8*(32E+38V)` bytes. This dominates both the grouping and exact contraction
forecasts before either allocation is attempted.

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
template SHA-256, target-specific `matalnum` mode, generated-section SHA-256,
and complete-artifact SHA-256. The generated `numeric_mode()` accessor returns
that literal mode. KSS targets are `off`; PPML and standalone tests remain
`on`.
Package loaders additionally bind their own package API/build identifiers. The
current KSS adapter requires CMG API 6 and solver adapter API 26.
