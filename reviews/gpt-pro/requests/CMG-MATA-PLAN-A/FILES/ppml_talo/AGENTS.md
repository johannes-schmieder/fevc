# AGENTS.md

## Scope

This folder is the independent Stata/Mata implementation of the PPML targeted
analytic leave-out (TALO) point-estimate correction. It does not implement
standard errors. The first integration target is the Separations project, but
the package must remain usable without that project.

Only files below `ppml_talo/` may change on a package-development branch
unless the owner expands the scope. The owner expanded ST10 on 2026-08-12 to
permit exactly one downstream artifact: a test-only copy named
`Code_IEB/do/separations_ppml_talo_test.do` in a separate Separations
worktree/branch. The canonical `Code_IEB/do/separations.do` remains immutable.
In this repository, treat `software/`, `application/`, `paper/`, `theory/`,
`proof-audit/`, `archive/`, and `paper/releases/` as read-only inputs.

## Status language

Keep these claims distinct:

- `formula_verified`: finite-dimensional algebra and independent numerical
  oracles agree at registered tolerances.
- `ai_reviewed`: a stored GPT Pro review has been locally adjudicated.
- `theory_supported`: the implemented deletion unit and assumptions match an
  internally audited theorem contract.
- `independently_checked`: a named human reviewer signed a stored report.

Passing tests does not establish an asymptotic theorem. GPT Pro review cannot
set `independently_checked`.

The observation-level estimator begins as
`TALO_THEORY_APPLICABILITY_UNVERIFIED`. A cluster-deletion estimator is
experimental and unavailable by default until its derivation gate closes.

## Milestone workflow

1. Read `PLAN.md` before substantive work.
2. Work on one `ST` milestone at a time.
3. Set one milestone to `IN_PROGRESS`, record decisions and deviations as they
   occur, and update evidence before setting it to `COMPLETE`.
4. Add a failing test before or with each numerical repair.
5. Record seeds, tolerances, Stata version, platform, and source commit for
   numerical evidence.
6. Keep dense oracle code independent of matrix-free production code.
7. Do not silently regularize an unidentified target or singular solve. Stop
   with a typed status.
8. Never form an observation-by-observation matrix in the production path.
9. Preserve target linearity and exact accounting identities up to a declared
   numerical tolerance.
10. Run `tests/run_all.do` and the repository handover gate before a release.

## Mathematical review gate

Any new cluster deletion formula, Hessian identity, or material change to the
implemented statistic must receive two independent GPT Pro reviews using the
repository `gpt-pro-proof-review` workflow. Each review uses a fresh chat and
must not contain the other response or a desired verdict. Store the bounded
requests, verbatim responses, validation output, and a local adjudication under
`theory/reviews/gpt-pro/`. Verify every accepted equation locally.

Named human review remains required before a new cluster result may be labeled
`independently_checked` or used as theorem-supported production functionality.

## Runtime and dependency policy

- Production runtime: Stata and Mata only.
- Supported baseline: Stata/MP 18 locally and Stata/MP 19 on the cluster.
- Required estimation dependencies: `ppmlhdfe`, `reghdfe`, and `ftools`.
- Do not require Python, R, Julia, a compiler, or internet access at runtime.
- Python may generate offline oracle fixtures, but committed fixtures must be
  readable and testable from Stata alone.
- Avoid private or undocumented Stata APIs in released code.

## Performance rules

- Store group identifiers as dense integers and reuse their encodings.
- Use matrix-free design actions and grouped reductions.
- Stream probe batches; do not retain all randomized right-hand sides.
- Use double precision for accumulations and solver state.
- Report convergence, residual norms, probe Monte Carlo error, wall time, and
  estimated peak memory.
- Default thread behavior must be deterministic conditional on seed and Stata
  version.

## Git and release rules

- Commit milestone evidence with the code that produced it.
- Never commit raw restricted data, credentials, logs containing paths to
  restricted data, or large transient benchmark files.
- Export a versioned release bundle for Separations. Do not link that project
  to a moving source checkout.
- A release requires a clean tree, passing tests, compiled documentation, a
  checksum manifest, and a pinned source commit.
