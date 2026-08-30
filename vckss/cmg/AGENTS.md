# CMG package-component rules

## Authorization and scope

The owner authorized the source-informed `CMG-MATA-1` implementation and
selected GPL-3.0-only for the component and containing package. CMG is owned
exclusively by `vckss`; it has no separate package, public command, or non-KSS
runtime target.

This subtree may contain the clean-room Mata core, the source-informed GPL Mata
two-way fixed-effect core, independent oracles, tests, benchmarks, deterministic
assembly tooling, and milestone evidence. It may not add a compiled CMG runtime,
plugin, MEX file, subprocess helper, or binary interchange layer.

Do not edit Git refs or the frozen manifest, manuscript, proof, release, state,
archive, application, or imported-provenance trees while doing CMG work.

## GPL and upstream-source boundary

The owner expressly authorizes using ideas from the official GPL CMG source
recorded in the provenance manifest and the maintained SCC source named by the
active plan. Treat imported bytes as read-only. Port only algorithms needed by
the Mata CMG hierarchy, retain notices, mark modifications, and record exact
paths, commits, hashes, and file-level derivation in
`docs/SOURCE_PROVENANCE.md` or its manifest.

Covered code uses SPDX identifier `GPL-3.0-only`; see `CODE_LICENSE.md` and
`LICENSES/GPL-3.0-only.txt`. Do not imply that the manuscript, data, proofs,
or whole repository share that license. Distributed binaries require complete
corresponding source and notices.

## Numerical contract

- Support two-way fixed effects and positive finite estimation weights only.
- Do not add hidden ridge, pseudoinverse repair, edge deletion, component
  selection, tolerance relaxation, sample changes, or probe changes.
- Before ordinary PCG uses a preconditioner, require it to be fixed, linear,
  symmetric, and positive definite on the registered component quotient.
- Never form observation-square, observation-parameter, or unbounded dense
  firm matrices.
- Predict and record every material allocation before allocating it.
- Preserve per-RHS status and require the package's complete original-system
  residual gate before accepting a solve.
- Hierarchy construction and routing consume no Stata RNG state.
- Keep the bounded dense terminal capped at 6,144 vertices. Large natural
  graphs must contract through deterministic component-aware aggregation.
- Preserve attempted-level diagnostics on hierarchy failure and deterministic
  aggregation across the registered graph and relabeling cases.
- Reuse one validated hierarchy and its terminal factors across all KSS RHSs.
  Shared matrix-RHS work must leave the V-cycle fixed, linear, symmetric, and
  quotient-SPD.

## Generation and workflow

- The deterministic assembler has exactly two targets: the shipped
  `vckss_cmg` runtime with `matalnum off` and checked-in `cmgtest` with
  `matalnum on`.
- Instantiate the canonical Mata template only through the assembler. Every
  generated symbol includes the namespace token, and generated sections record
  canonical and instantiated hashes.
- Release commands may check generated files but may not regenerate them.
- Work on one CMG milestone at a time. Add an independent failing test for a
  numerical repair and keep Python/SciPy oracles independent of the template.
- Record seeds, tolerances, versions, source hashes, platform, timings, and RSS
  for numerical evidence.
- Follow the root impact-based gate policy; do not rerun broad qualification
  merely because the SHA changed.
- Model review is evidence only. It cannot assign `checked`, `ai_reviewed`,
  or `independently_checked` without the required review class.
