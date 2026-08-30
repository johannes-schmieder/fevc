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
deletion, literal-copy positive integer frequency weights, and explicit
diagonal PCG or forced generic CMG. It preserves the established
`e(projection_*)` result schema and uses Mata exact as the independent oracle.
Automatic projection routing remains withheld.

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

The forced-CMG composition now reuses the generic-JLA model hierarchy already
implemented and qualified for planned generic solves. That hierarchy is shared
between the full W+F+Q and FE-only solvers, including the certified control
block. This is deliberately not `CMG_FULL_V2`, whose direct compressed hybrid
API is tied to the no-control match-deletion point-estimation cell.

The first local promotion probes preserve the common Counter-V1 formula path:

- at 6,000 rows, CMG and diagonal differ by at most `2.28e-11` across the
  three projection coefficients and full 3-by-3 covariance;
- 6,000-, 24,000-, and 96,000-row CMG commands all pass residual, PSD, and
  memory gates;
- worst projection-solve iterations are 17, 38, and 79 rather than 161, 625,
  and diagonal nonconvergence; and
- the 96,000-row CMG command completes locally in 338.7 seconds with an
  `8.85e-11` maximum projection complete residual and a 39.6 MB projection
  memory forecast.

The same three probes were then bound to exact source `33ede86`.  Every
statistical and numerical field was identical to the candidate run.  Command
times were 6.627, 42.609, and 344.04 seconds, while complete-command peak RSS
was 186.4, 303.5, and 451.4 MB.  These are exact-source local convergence and
resource observations, not a same-host MATLAB performance comparison or a
cross-platform reach claim.  Compact receipts are under
`qualification/inference_matlab/evidence/cmg_projection_local/33ede864111c319185949ede4ef6d2bcc44b1383/`.

The affected-surface implementation gates are green: pinned Rust formatting,
strict Clippy, and workspace/all-target tests; generated-CMG checks and its 28
tests; focused Python formula, weight, packaging, and parity tests; C shim and
ABI checks; source-local and isolated-install Stata projection tests; and the
1,002-row exact/MATLAB oracle. Two unchanged repository-wide harness defects
still prevent describing the broad suites as green: the scale-bundle allowlist
omits three previously installed runtime files, and Stata 19 rejects an
unchanged closing brace in `test_rust_public_generic.do`. Neither failure
reaches or exercises the new projection predicate.

## Next work

1. Carry forward unaffected exact/MATLAB formula evidence through the recorded
   compatibility review; do not rewrite immutable diagonal receipts.
2. Run the smallest same-host
   CMG/MATLAB comparison needed to decide MATLAB-relative complete-command
   performance and 96,000-row reach. Record nonconvergence and right-censoring
   without imputation or relaxed gates.
3. Do not launch a broad platform matrix, replacement diagonal array, or paper
   claim merely because the source SHA changes.

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
