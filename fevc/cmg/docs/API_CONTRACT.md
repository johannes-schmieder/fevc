# CMG API 9 component contract

## Ownership and generated surfaces

CMG is an internal component of `fevc`. It has no public Stata command,
independent package, or non-KSS adapter. The canonical template exposes only
the package quotient pullback `@CMG_NS@__apply_kss()` in addition to the core
graph, hierarchy, workspace, and apply interfaces.

Generator API 5 creates exactly two namespace artifacts:

| Target | Namespace | `matalnum` | Location |
| --- | --- | --- | --- |
| package | `vckss_cmg` | `off` | `fevc/fevc_cmg.mata` |
| test | `cmgtest` | `on` | `fevc/cmg/generated/cmg_test.mata` |

Every generated struct, function, constant, and helper carries its namespace.
The manifest binds the canonical source, generator API, namespace, numeric
mode, generated-section hash, complete-artifact hash, and output path.

## Version boundary

API 9 adds the command's optional budget and warning/error/off policy to the
API 8 package identity. Its source-informed GPL numerical core derives from
the API-6 implementation qualified under `CMG-MATA-1`. Hierarchy algebra,
V-cycle, deterministic tie-breaking, terminal cap and numerical status
meanings are unchanged. A package loader must require:

- `vckss_cmg__api_level() == 9`;
- `vckss_cmg__numeric_mode() == "off"`;
- design label `gpl-cmg-mata-degree3-hybrid-v9-memory-policy`.

Predecessor or mismatched runtimes fail closed before estimator RNG.

## Core objects and status

The component retains typed cells, graph, hierarchy, workspace, preflight,
route, apply, and diagnostic objects. Exact string status `CONVERGED` is the
only successful status. Invalid input, unsafe scaling, resource exhaustion,
hierarchy failure, terminal failure, incompatible RHSs, and nonfinite output
remain typed failures; no path regularizes, changes the sample, or relaxes a
tolerance.

The bounded dense terminal remains capped at 6,144 vertices. Hierarchy and
workspace forecasts are checked against a package-provided budget only when
that budget is explicit and its policy is `memorycheck(error)`. With an
advisory or absent budget, an over-budget forecast alone does not reject
construction or application. Actual allocation failure, overflow, structural
caps and receipt invariants remain binding. Standalone diagnostics retain
their prior strict numeric semantics. See [MEMORY.md](../../docs/MEMORY.md).
One validated hierarchy and its factors may be reused across KSS right-hand
sides while the V-cycle remains fixed, linear, symmetric, and quotient-SPD.

## Package boundary

The component supplies a preconditioner action, not an estimator convergence
certificate. `fevc` retains the exact original-system action,
per-right-hand-side recurrence gates, worker reconstruction, and complete
worker-plus-firm residual check. A post-RNG CMG failure withholds the affected
calculation; it never triggers a route change.

All covered source is GPL-3.0-only under `CODE_LICENSE.md`. Public distribution
still requires the documented human license/provenance review.
