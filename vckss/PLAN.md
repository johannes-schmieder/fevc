# Active package plan

## Objective

Maintain the private `vckss` `0.5.0-alpha.1` candidate as a fast,
statistically equivalent Stata alternative to maintained MATLAB KSS on
compatible problems. Corrected-result equivalence and complete-command
performance are the primary development criteria; backend-internal identity is
not.

Candidate promotion follows
[`docs/development_acceptance_v1.json`](docs/development_acceptance_v1.json).
Durable behavior and release decisions live in
[`docs/DECISIONS.md`](docs/DECISIONS.md), and current backend/platform
coverage lives in
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md).

Windows qualification, public tagging, and distribution remain deferred.

## Current accepted state

- Point estimation remains the default. Exact-observation component inference
  and fixed-effect projection inference are explicit, capability-gated
  requests.
- Portable Mata and qualified Rust routes share the registered estimator,
  sample, target, weighting, failure, and complete-residual contracts without
  requiring pathwise floating-point identity.
- Rust-preferred automatic routing may fall back to Mata only during structural
  preflight, before native preparation and estimator RNG. Selected native
  failures fail closed.
- Exact, compressed JLA, generic JLA, and the package-owned `CMG_FULL_V2`
  route have source-bound qualification on their declared surfaces. A green
  quick lane alone is not native qualification.
- The accepted alpha evidence covers macOS arm64/Rosetta and SCC Linux x86-64.
  The completed comparative-scaling study and full-CMG production evidence
  remain source-bound; they need not be rerun for unrelated changes.
- The human package-boundary, corresponding-source, notice, provenance, and
  data-exclusion review is complete. Public release still requires a fresh
  exact-artifact decision.

Supporting contracts and evidence are indexed by
[`docs/README.md`](docs/README.md).

## Active checkpoint: scalable projection preconditioning

The explicit Rust generic-JLA projection route supports mover-only observation
deletion, literal-copy positive integer frequency weights, and diagonal PCG.
It preserves the established `e(projection_*)` result schema and uses Mata
exact as the independent oracle.

Exact source `96e7a66` completed the registered focused SCC comparison:

- all 6,000- and 24,000-row repetitions passed the statistical gates;
- maximum covariance-diagonal differences were below 0.48% and standard-error
  differences below 0.24%;
- diagonal PCG was 18.97 times slower than MATLAB at 24,000 rows; and
- the 96,000-row Rust solve failed to converge after 20,000 iterations, while
  the independently reconstructed MATLAB fit also reached its iteration limit.

The diagonal route is therefore numerically qualified on its accepted cells
but is not promoted for comparable large-data reach. The evidence and
interpretation are in
[`qualification/inference_matlab/SCALABLE_PROJECTION.md`](qualification/inference_matlab/SCALABLE_PROJECTION.md).

## Next work

1. Reuse the already qualified package-owned full-CMG hierarchy as the stronger
   explicit projection preconditioner. Do not change the point-estimation
   default, effective request, RNG stream, probe construction, estimand,
   covariance formula, or existing result schema.
2. Keep routing structural and pre-RNG. Explicit CMG must fail closed; it may
   not silently revert to diagonal after selection.
3. Reuse one admitted hierarchy across projection RHSs while preserving one
   certified inverse action per projection column, complete original-system
   residuals, projection-Gram conditioning, covariance PSD, and caller/native
   memory accounting.
4. Add focused parity tests against Mata exact and the accepted diagonal route,
   plus failure, receipt, state-restoration, deterministic-probe, memory, and
   generated-source checks.
5. Run the minimum source, package, and native gates implicated by the actual
   boundary change. Do not rerun a broad platform matrix or unrelated SCC
   study merely because the source SHA changes.
6. Only after focused qualification passes, run the registered projection
   comparison cells needed to decide large-data reach and MATLAB-relative
   complete-command performance. Record nonconvergence and right-censoring
   without imputation or relaxed gates.
7. Promote the route only if it preserves every scientific and resource gate
   and materially resolves the diagonal route's scaling limitation. Otherwise
   retain the evidence and keep the route unpromoted.

## Deferred and out of scope

- Windows qualification, a public tag, and public distribution.
- Match-cluster inference and unsupported frequency/stayer inference tuples.
- A command-surviving native cache or new public result interface.
- Changes to the estimator, target definitions, finite-projection formula,
  default point-only behavior, or accepted historical evidence.
- A replacement comparative-scaling array, broad platform matrix, or paper
  claim before a stronger projection route earns focused qualification.

## Acceptance and qualification

- Statistical equivalence, tolerance behavior, hard correctness, performance
  priority, qualification selection, and evidence reuse follow the registered
  development policy.
- Every accepted solve must pass identification, accounting, finite-output,
  complete original-system residual, direct-memory, typed-failure/UserBreak,
  and caller-state/lifecycle gates.
- Performance claims require compatible requests, samples, graph structure,
  probe count, tolerances, hardware/affinity records, and complete-command
  measurements. Kernel-only wins are insufficient.
- Keep independent oracles independent and historical receipts immutable.
- Documentation-only changes receive focused checks and carry forward
  unaffected scientific/performance evidence through impact review.

## Completion gates

The active checkpoint closes only when:

1. focused unit and Stata regressions cover accepted CMG projection behavior,
   strict failure, receipts, memory, residuals, covariance, and state;
2. generated CMG, Python, Rust, package, clean-install, and source-local native
   gates implicated by the change pass;
3. any performance or reach claim is bound to exact-source focused evidence;
4. the parity ledger, decisions, user documentation, and this plan describe
   the same supported surface; and
5. the worktree contains no disposable transport or generated drift and no
   public release action has been inferred from the private alpha.
