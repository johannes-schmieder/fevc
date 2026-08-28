# CMG package-component rules

## Authorization and scope

The owner authorized the source-informed `CMG-MATA-1` implementation on
2026-08-18 and selected GPL-3.0-only for the component and containing package.
CMG is now owned exclusively by `vckss`; it has no separate package,
public command, or non-KSS runtime target.

Files under `vckss/cmg/**` may implement the existing clean-room Mata core and
the source-informed GPL Mata two-way fixed-effect numerical core, independent
oracles, tests, benchmarks, assembly tooling, and milestone evidence. No
compiled CMG runtime, plugin, MEX file, subprocess helper, or binary
interchange layer is allowed. The assembler has exactly two targets: the
shipped `vckss_cmg` runtime with `matalnum off` and the checked-in `cmgtest`
target with `matalnum on`.

Use `main` and the current worktree. Do not create or switch branches or
worktrees. Do not edit the index, refs, frozen manifests, manuscript, proof,
release, state, archive, application, or imported provenance trees.

## GPL and upstream-source boundary

For `CMG-MATA-1`, the owner expressly authorizes reading and porting ideas from the
official GPL CMG implementation recorded in the source-provenance manifest and the
maintained SCC source bound in the milestone plan. Treat all imported bytes as
read-only. Port only algorithms needed by the Mata CMG hierarchy, retain
their copyright/license notices in the derivative Mata source, mark modifications, and record exact source
paths, commit IDs, hashes, and file-level derivation in
`docs/SOURCE_PROVENANCE.md` or its manifest.

Covered code uses SPDX identifier `GPL-3.0-only`; see `CODE_LICENSE.md` and
`LICENSES/GPL-3.0-only.txt`. Do not imply that the manuscript, data, proofs, or
whole repository are licensed. A distributed binary must be accompanied by
complete corresponding source and notices. Public distribution remains
subject to the milestone's final human license/provenance review.

## Numerical contract

- Two-way fixed effects only.
- Positive finite estimation weights only.
- No hidden ridge, pseudoinverse repair, edge deletion, component selection,
  tolerance relaxation, sample change, or probe change.
- The preconditioner must be fixed, linear, symmetric, and positive definite
  on the registered component quotient before ordinary PCG may use it.
- Never form observation-square, observation-parameter, or unbounded dense
  firm matrices.
- Predict and record every material allocation before allocating it.
- Preserve per-RHS status and require the package's original full-system
  residual gate before accepting a solve.
- Hierarchy construction and routing consume no Stata RNG state.
- The bounded dense terminal remains capped at 6,144 vertices. Large natural
  graphs must contract through deterministic component-aware multilevel
  aggregation; never replace hierarchy work with an unbounded dense factor.
- Preserve attempted-level diagnostics on hierarchy failure. Production
  aggregation must be deterministic on hubs, paths, barbells, irregular
  degree graphs, and canonical relabelings.
- Reuse one validated hierarchy and its terminal factors across all KSS RHSs.
  Matrix RHS applications may share traversal and terminal solves, but the
  V-cycle must remain fixed, linear, symmetric, and quotient-SPD.

## Workflow

1. Work on one `CMG` milestone at a time and update its evidence.
2. Add an independent failing test before or with every numerical repair.
3. Use `./.venv/bin/python` for every Python command.
4. Keep Python/SciPy oracle code independent of the production Mata template.
5. Record seeds, tolerances, versions, source hashes, platform, timings, and
   RSS for numerical evidence.
6. Run the smallest relevant test during development and only the handover,
   native, platform, or scale gates implicated by the change. Do not rerun a
   large benchmark or full qualification merely because the commit changed;
   reuse compatible evidence under the repository acceptance policy unless
   CMG/runtime behavior, benchmark meaning, or another material input changed.
7. Model review is evidence only. It cannot assign `checked`,
   `ai_reviewed`, or `independently_checked` without the repository's required
   review class.

## Generated artifacts

The canonical Mata template may be instantiated only by the deterministic
assembler. Every function, struct, constant, and helper must include the
namespace token. Generated sections record canonical and instantiated hashes.
Release commands may check generated files but may not regenerate them.
