# Public-source readiness checkpoint — 2026-09-02

## Status

The `fevc` source tree is prepared for public development at version
`0.5.0-alpha.1`. This is a source-readiness checkpoint, not a package release:
no public tag, release archive, native binary distribution, or visibility
change is implied.

The companion paper and paper-specific coefficient-one memo/evidence live in
the separate `fevc-paper` repository. They are intentionally absent from the
reachable FEVC history.

## Current objective

Keep the public source repository small, reproducible, and honest about its
qualification boundary while preserving accepted source-bound evidence.
Candidate promotion continues to follow
[`docs/development_acceptance_v1.json`](docs/development_acceptance_v1.json).
Scientific and numerical contracts remain in [`docs/`](docs/README.md).

The active post-checkpoint scientific development is a separately identified
matrix-free Rust generic-JLA component-inference method. Its internal
oracle-variance layer and explicit experimental structured-variance `q=0` and
`q=1` routes are described in
[`docs/MATRIX_FREE_COMPONENT_INFERENCE.md`](docs/MATRIX_FREE_COMPONENT_INFERENCE.md).
It does not alter the public-source readiness state, the existing exact Mata
inference surface, or any default. The recommended architecture keeps strict
unrestricted KSS variance products distinct from the pragmatic common
structured FEVC variance model; no unrestricted-KSS parser mode is reserved or
currently planned. The common cross-fitted regression,
leverage-only sensitivity, component covariance, spectral diagnostics, and
`q=1` confidence-set layer are implemented through a versioned plugin/Stata
attachment. Exposure is explicit and experimental, never automatic; fitted-
variance qualification now has an independent oracle harness and fail-closed
validator. The 2026-09-03 confirmation passed all registered `q=0`, Gaussian
common-model `q=1`, and mild-misspecification gates, and exposed severe
misspecification as intended, but failed the correctly specified
leverage-heteroskedastic and t8 `q=1` firm-coverage cells. Promotion is
therefore withheld pending larger-dimension evidence or a justified estimator
repair. The registered V2 diagnosis replaces the dense moderate-dimension
bottleneck with an exact diagonal-plus-low-rank oracle, retains a tiny dense
identity test, and has immutable, shard-independent local/SCC campaign
plumbing. The source-bound V2 development campaign completed 20,000 attempts
at dimensions 16--64. It passed every gate except oracle-variance t8 firm
coverage at dimension 64 (0.972, MCSE 0.0052). Because this occurred on the
oracle path while maximum mode and remainder-influence concentration declined,
the registered classification is `q1_reference_or_remainder_problem`, not a
structured-smoother failure. The V3 audit identified and implemented a
center/covariance mismatch: q=1 must recenter its leading square with the raw
leave-out mode variance product, while the positive structured variance model
enters only covariance and studentization. The corrected path,
remainder-identity gate, 100,000-draw public critical minimum, independent
critical/ellipse oracles, and factorized campaign are registered in
`docs/structured_inference_qualification_v3.json`. The clean source-bound V3
smoke passed, and its development campaign completed all 20,000 attempts with
zero process failures. Every registered gate passed except oracle-variance t8
firm coverage at dimension 64, again 0.972 with MCSE 0.0052. The raw-recenter
remainder identity error was `3.3e-13`, the leading and remainder variance
ratios were 0.982 and 0.989, and the leading-mode and remainder-influence
concentrations continued to decline. The frozen V3 classification remains
`q1_recenter_reference_or_remainder_problem`; its evidence is unchanged.

The preregistered V4 diagnosis has now resolved that question for the frozen
dimension-64 design. Its clean source-bound SCC campaign used 20,000
calibration and 10,000 independent evaluation replications for Gaussian and
standardized-t8 outcomes plus paired exact joint-Gaussian reference draws.
Production q=1 covered 0.9591 under Gaussian errors and 0.9584 under t8 errors
(MCSE about 0.0020); fixed-population covariance and each one-component
covariance hybrid produced effectively identical coverage and endpoints.
Oracle q=0 covered 0.9497 and 0.9488. The exact Gaussian reference covered
0.9581 at the parabola vertex and 0.9593 at the actual nuisance value, while
held-out calibration of the actual shortest required radius covered
0.9483--0.9510. This is not a critical-value bug: the KSS
Andrews--Mikusheva construction uses a maximal-curvature circle to obtain a
uniform at-least-nominal guarantee, not an exact finite-sample
shortest-distance law for a particular parabola. The theoretical radius was
modestly conservative in every V4 cell. Gaussian/t8 distribution diagnostics
were nearly indistinguishable, production/fixed covariance agreed, and the
exact remainder identity held below `1.7e-12`; no production correction is
scientifically justified. The earlier 0.972 estimate differs from V4's 0.9584
by about 2.4 combined Monte Carlo standard errors and does not persist in the
larger independent sample. Exact registrations, results, hashes, and SCC
accounting are recorded in
`docs/structured_inference_diagnostic_v4_result.json`.

V4 is development diagnosis only. It does not retroactively pass V3, authorize
confirmation or promotion, or qualify a release binary. Another same-cell
development run is not justified. If public promotion remains the goal, the
next scientific gate is the separately preregistered V5 source-bound
confirmation in `docs/structured_inference_confirmation_v5.json`. It freezes
the complete 80-row structured observation-deletion `q=0`/`q=1` matrix, the
unchanged V1 coverage and misspecification rules, a new semantic outcome RNG
domain, the corrected raw `q=1` recenter, and the deterministic qualification
critical. The new sharded harness binds every attempted replication to its
source, registration, manifest, and binary, while keeping the deliberately
multi-mode covariance-target `q=1` rows as adverse diagnostic/atomic-usage
cases rather than coverage claims. Registration alone is not evidence. One
complete local tiny path and a real SCC smoke must pass before the registered
200-task, 200,000-row confirmation is run. Only a passing confirmation can
precede a separate promotion and release-binary qualification decision;
grouped-match `q=0` remains the subsequent implementation slice.

## Accepted package state

- Point estimation remains the default. Exact-observation component inference
  and fixed-effect projection inference are explicit, capability-gated
  requests.
- Match deletion and `nuisance(joint)` remain the defaults. The default target
  combines retained movers with eligible attached one-firm stayers; use
  `stayers(movers)` for the mover-only convention.
- Portable Mata and qualified Rust routes share the registered estimator,
  sample, target, weighting, failure, and complete-residual contracts without
  requiring pathwise floating-point identity.
- Rust-preferred automatic routing may fall back to Mata only during
  structural preflight, before native preparation and estimator RNG. A
  selected native failure fails closed.
- The default display is compact. `estat decomposition, full`, `estat sample`,
  `estat computation`, and `estat diagnostics` expose the stored audit state.
- Accepted native evidence covers its declared macOS arm64/Rosetta and SCC
  Linux x86-64 surfaces. Windows Stata/plugin qualification remains deferred.
- The human package-boundary, corresponding-source, notice, provenance, and
  data-exclusion review was completed on 2026-08-29. Release approval still
  applies to the exact artifact that would be distributed.

The completed 2026 FEVC/MATLAB scaling campaign is summarized in
[`benchmarks/fevc_matlab_2026/STATUS.md`](benchmarks/fevc_matlab_2026/STATUS.md).
Earlier exact-SHA reports, receipts, reviews, and failure records remain
immutable evidence rather than current instructions.

## Public automation and local qualification

Public GitHub workflows use hosted runners, read-only repository permissions,
exactly pinned actions, and source-only Python/CMG or Rust checks. Licensed
Stata runs only on an authorized local machine or explicitly private
infrastructure; there is no public self-hosted Stata workflow.

Minimum source gates are:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
```

When Stata/MP is locally available, the integrated qualification command is:

```bash
./.venv/bin/python fevc/tools/run_checks.py
```

Use `fevc/tools/clean_workspace.py --dry-run` before `--apply`. The cleaner
removes only ignored, untracked build/cache/log output and protects accepted
evidence, qualification records, reviews, and `.venv`.

## Checkpoint completion criteria

1. Python and generated-CMG checks pass from the cleaned source tree.
2. The integrated licensed-Stata qualification prints its terminal PASS marker
   when selected by the affected surface.
3. Package, identity, history, license/provenance, parity, artifact, and source
   layout checks pass.
4. Public workflows contain no self-hosted or write-capable job, and third-party
   actions are pinned to exact commits.
5. The current tree and complete reachable history pass private-artifact and
   secret-pattern scans.
6. `git diff --check` and final repository status are clean.

## Owner actions after this checkpoint

- Change repository visibility only after reviewing the final diff and scan
  results.
- Before or immediately after the visibility change, remove any obsolete
  self-hosted runner registration and repository Actions secrets in GitHub,
  then configure branch protection for `main`.
- Decide separately whether to designate an RC, create a tag or GitHub release,
  publish a source archive, or distribute native binaries.
- Decide whether Windows qualification or any deferred scientific work is a
  prerequisite for a later package release.
