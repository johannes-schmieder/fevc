# CMG API 8 component contract

## Ownership and generated surfaces

CMG is an internal component of `vckss`. It has no public Stata command,
independent package, or non-KSS adapter. The canonical template exposes only
the package quotient pullback `@CMG_NS@__apply_kss()` in addition to the core
graph, hierarchy, workspace, and apply interfaces.

Generator API 5 creates exactly two namespace artifacts:

| Target | Namespace | `matalnum` | Location |
| --- | --- | --- | --- |
| package | `vckss_cmg` | `off` | `vckss/vckss_cmg.mata` |
| test | `cmgtest` | `on` | `vckss/cmg/generated/cmg_test.mata` |

Every generated struct, function, constant, and helper carries its namespace.
The manifest binds the canonical source, generator API, namespace, numeric
mode, generated-section hash, complete-artifact hash, and output path.

## Version boundary

API 8 records the `vckss` package and generated-target identity. Its numerical
core is the
source-informed GPL API-6 implementation qualified under `CMG-MATA-1`.
The hierarchy, V-cycle, resource forecasts, deterministic tie-breaking,
terminal policy, and status meanings are unchanged. A package loader must
require:

- `vckss_cmg__api_level() == 8`;
- `vckss_cmg__numeric_mode() == "off"`;
- design label `gpl-cmg-mata-degree3-hybrid-v8-vckss-component`.

Predecessor or mismatched runtimes fail closed before estimator RNG.

## Core objects and status

The component retains typed cells, graph, hierarchy, workspace, preflight,
route, apply, and diagnostic objects. Exact string status `CONVERGED` is the
only successful status. Invalid input, unsafe scaling, resource exhaustion,
hierarchy failure, terminal failure, incompatible RHSs, and nonfinite output
remain typed failures; no path regularizes, changes the sample, or relaxes a
tolerance.

The bounded dense terminal remains capped at 6,144 vertices. Hierarchy and
workspace allocations must fit the package-provided direct-memory envelope.
One validated hierarchy and its factors may be reused across KSS right-hand
sides while the V-cycle remains fixed, linear, symmetric, and quotient-SPD.

## Package boundary

The component supplies a preconditioner action, not an estimator convergence
certificate. `vckss` retains the exact original-system action,
per-right-hand-side recurrence gates, worker reconstruction, and complete
worker-plus-firm residual check. A post-RNG CMG failure withholds the affected
calculation; it never triggers a route change.

All covered source is GPL-3.0-only under `CODE_LICENSE.md`. Public distribution
still requires the documented human license/provenance review.
