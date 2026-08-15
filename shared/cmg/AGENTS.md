# CMG shared-core rules

## Authorization and scope

The owner authorized implementation of `cmg_plan.md` on 2026-08-14. This is a
new solver milestone series, separate from the numbered proof program,
`ppml_talo` ST milestones, and `kss_bc` KB milestones.

Files under `shared/cmg/**` may implement only the clean-room CMG-inspired
two-way fixed-effect numerical core, independent oracles, tests, benchmarks,
assembly tooling, and milestone evidence. Package adapters may change only
after the coordinator records an ownership handoff from any active package
thread.

Use `main` and the current worktree. Do not create or switch branches or
worktrees. Do not edit the index, refs, frozen manifests, manuscript, proof,
release, state, archive, application, or imported provenance trees.

## Clean-room boundary

Do not open, read, copy, translate, execute, or derive implementation details
from CMG source under `application/veneto-kss/**` or any other imported
upstream implementation. Implement only from the mathematical formulas and
published papers identified in `cmg_plan.md`.

Record the mathematical source, authoring scope, test oracle independence, and
canonical-source hash. The repository has no public software license. Do not
publish or redistribute generated artifacts.

## Numerical contract

- Two-way fixed effects only.
- Positive finite estimation weights only; zero-weight PPML face handling
  remains package-owned.
- No hidden ridge, pseudoinverse repair, edge deletion, component selection,
  tolerance relaxation, sample change, or probe change.
- The preconditioner must be fixed, linear, symmetric, and positive definite
  on the registered grounded/quotient space before ordinary PCG may use it.
- Never form observation-square, observation-parameter, or unbounded dense
  firm matrices.
- Predict and record every material allocation before allocating it.
- Preserve per-RHS status and require the package's original full-system
  residual gate before accepting a solve.
- Hierarchy construction and routing consume no Stata RNG state.

## Workflow

1. Work on one `CMG` milestone at a time and update its evidence.
2. Add an independent failing test before or with every numerical repair.
3. Use `./.venv/bin/python` for every Python command.
4. Keep Python/SciPy oracle code independent of the production Mata template.
5. Record seeds, tolerances, versions, source hashes, platform, timings, and
   RSS for numerical evidence.
6. Run the smallest relevant test during development and the repository
   handover/full gates before claiming a milestone complete.
7. Model review is evidence only. It cannot assign `checked`,
   `ai_reviewed`, or `independently_checked` without the repository's required
   review class.

## Generated artifacts

The canonical Mata template may be instantiated only by the deterministic
assembler. Every function, struct, constant, and helper must include the
namespace token. Generated sections record canonical and instantiated hashes.
Release commands may check generated files but may not regenerate them.
