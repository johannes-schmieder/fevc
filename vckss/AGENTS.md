# Package agent instructions

## Scope and startup

This directory is the sole public `vckss` Stata package. The command
implements KSS leave-out bias-corrected point estimates for linear worker--firm
variance decompositions. CMG is an internal package component under `cmg/`; the
optional Rust plugin is an explicitly selected backend, not a replacement
package.

Before substantive work:

1. Read the repository-root `AGENTS.md`.
2. Read this file and `PLAN.md`.
3. Read `docs/README.md` to locate the authoritative contract or evidence.
4. For CMG changes, also read `cmg/AGENTS.md` and `cmg/STATUS.md`.
5. Work on `main` and the current worktree unless the owner says otherwise.
6. Preserve existing changes and never rewrite source-bound historical
   evidence or exact-SHA receipts.

Use `./.venv/bin/python` for Python commands. Do not infer a new scale run,
benchmark, or release claim from an older receipt.

## Active development objective

The current milestone is the private `0.4.0-alpha.1` release candidate:
make `vckss` a statistically equivalent, end-to-end speed-competitive Stata
alternative to maintained MATLAB KSS on compatible hard problems. Rust/Mata
feature parity, exact internal numerical identity, and additional evidence are
secondary to corrected-result equivalence and measured performance. `PLAN.md`
records the exact current state and milestone order; the comparison rule is
registered in `docs/development_acceptance_v1.json`.

Windows qualification, a public release, and a command-surviving native cache
remain out of scope.

Trusted-patch files under `.ci/codex/` are single-use transport. A clean
handoff contains no `apply.py`, `apply.patch`, `commit-message.txt`, or
`last-apply.json`.

## Statistical contract

Development requires equivalence of statistical results, not identical
floating-point paths. For each of the four corrected targets, compare candidate
`a` and reference `b` using `s=max(1,abs(a),abs(b))`. Deterministic or common-
draw comparisons pass when `abs(a-b)<=1e-8*s`. Randomized comparisons pass when
the difference is no larger than the greater of that floor and `0.25` times
the combined numerical MCSE. A comparator without numerical MCSE requires a
registered repeated-seed distribution. Bitwise equality, ULP equality, equal
iteration counts, and legacy fixed roundoff gates are diagnostics rather than
candidate-promotion blockers.

Preserve all of the following:

- point estimates only; never post `e(V)` or call probe dispersion an
  econometric standard error;
- worker variance, firm variance, worker--firm covariance, and variance of
  their sum as the four target columns;
- match deletion as the default, with `deletionid()` independent of coefficient
  cells and mover-only match headlines;
- `nuisance(joint)` versus `nuisance(fixedoffset)` semantics;
- positive integer frequency weights as literal physical-copy counts;
- explicit target weights as stored-row mass, not multiplied by frequency;
- the frozen retained sample, target population, deletion fixed point, and
  every accounting identity;
- coefficient cells, deletion units, and exact target-scale strata as distinct
  indices; and
- typed withholding for unsupported, unidentified, singular, nonestimable,
  unconverged, or resource-inadmissible requests.

The improved-JLA finite-projection correction uses coefficient one on the
mixed fourth moment:

`B = R^-1 { M m(P^2) - P m(M^2) + (M-P) m(P,M) }`.

The coefficient-two MATLAB expression is a legacy comparator only.

## Numerical contract

- Never construct a production observation-by-observation matrix.
- Solve on the full-firm zero-sum quotient; ground a displayed firm coordinate
  only after convergence and still check its original equation.
- Scale complete residuals by the original RHS Euclidean norm, or use the
  absolute residual for a zero RHS.
- Enforce `max(1e-11,10*tolerance())` on every accepted RHS.
- A graph, Schur, recursive, or reduced residual never substitutes for the
  complete original worker-plus-firm, or worker-plus-firm-plus-control,
  residual.
- Keep grouped cancellation-sensitive sums stable or compensated.
- Preserve every rank, inverse, reciprocal, maker, control-basis, deletion,
  accounting, and finite-output gate. Do not add hidden regularization.

These runtime correctness checks do not imply pathwise equality with Mata or
another backend. A candidate that clears the registered corrected-result
equivalence rule may use different reductions, stopping points, and numerical
representations. The public `tolerance()` option and complete-residual threshold
do not change silently as part of a development comparison.

## Backend, routing, RNG, and resources

The alpha target makes omitted `backend()` and `backend(auto)` prefer Rust
when a complete effective-request capability check succeeds. Missing native
runtime or a structurally unsupported tuple may fall back to Mata only before
native preparation and estimator RNG. `backend(rust)` remains strict and
`backend(mata)` remains an explicit Mata route.

Request capability, algorithm, engine, solver route, batch widths, memory, and
every permitted pre-RNG fallback must reconcile with returned receipts.
Automatic resolution is structural, frozen before estimator RNG, and may not
reroute after a later memory, rank, setup, numerical, or resource failure.

- Exact uses no estimator RNG and has no iterative preconditioner.
- JLA uses the registered runtime-scoped RNG contract and separate leverage and
  target domains.
- Caller RNG algorithm, stream, complete state, data, `e(sample)`, and sort
  state must be restored on every exit.
- Explicit CMG fails closed. Automatic CMG-to-diagonal fallback is permitted
  only before RNG and must be recorded.
- `memory_gib()` is the hard direct-allocation envelope.
- Wall forecasts, headroom percentages, performance models, and timing targets
  are advisory unless a concrete scheduler or allocation limit is being
  enforced.

## Development and evidence discipline

For a behavioral or numerical repair:

1. Add or retain a focused failing regression.
2. Keep dense/brute-force oracles independent of production code.
3. Run the smallest relevant gate while iterating.
4. Before closing, run the applicable Python, generated-source, Stata, clean
   install, and plugin qualification gates.
5. Record exact source SHA, commands, versions, seeds, tolerances, failures,
   and skipped external gates.
6. Require an exact-SHA receipt for every qualification claim.

Quick Stata CI is not plugin qualification. A Rust route is qualified only by
the source-local plugin profile and its exact-SHA receipt. Advisory benchmark
misses do not invalidate a scientifically accepted command, but MATLAB-relative
complete-command performance is a primary promotion criterion for new backend
architectures.

## Historical evidence and licensing

Do not edit byte-bound predecessor reports, receipts, review packets, source
manifests, or archived benchmark outputs. Summaries may point to them but may
not silently reinterpret them.

Follow `../CODE_LICENSE.md`, `docs/SOURCE_PROVENANCE.md`, and CMG provenance
records. GPL selection does not itself authorize public release. Public
distribution still requires the documented human license/provenance review.
