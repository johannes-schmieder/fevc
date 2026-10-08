# Mean-centered component inference: implementation plan

Prepared October 8, 2026 against package source `7fb65c41`.
Status: accepted October 8, 2026; all five recommendations approved. Implementation in progress; qualification pending.

## Objective and decisions

Implement `centering(mean)` with `inference(highrank)` and `inference(q1)`
using the observed mean as a fixed constant in the inference calculation.
This is the owner's selected approximation. It does not condition the error
distribution on the observed mean and does not account for mean-estimation
uncertainty. Numerical MCSE remains a separate uncertainty calculation.

The owner accepted all five recommendations and explicitly authorized implementation, qualification, native adoption, and the planned commits/pushes. Tags, hosted releases and manuscript submission remain out of scope.

| Decision | Recommended baseline | Alternative and consequence |
| --- | --- | --- |
| Supported routes | All currently supported component-inference tuples, both references | Exact Mata first reduces initial scope; adding new tuples is a separate estimator/interface extension |
| Public interface | Existing Mean default also applies to component inference; concise display note and dedicated metadata | Explicit Mean or a consent option adds friction and another interface rule |
| Statistical validation | Independent algebra plus a newly registered bounded sampling assessment | Engineering-only support must remain experimental; full coverage confirmation makes scientific acceptance a prerequisite to enabling the feature |
| Native delivery | Rebuild, qualify and adopt all shipped platforms before claiming a complete updated distribution | Source-only or qualified Mac-only delivery leaves other binaries gated by a new capability; Windows may remain a blocker for a complete distribution |
| Existing unpublished commits | Review and include the four existing ancestors in the final package push | Otherwise keep the push pending; do not rewrite main or create a branch to evade the boundary |

At planning start the package checkout was clean and four commits ahead of `origin/main`:
`ed829fbd`, `24754269`, `623b156d`, `7fb65c41`. They include centering harnesses,
the Mean default and projection extension, prepared Mac/Linux payloads, and
candidate installation records. The paper checkout is dirty, including its
existing centering memos. Preserve all that work and stage only this task's
files/hunks. The owner has now requested execution of this plan.
During planning, unrelated changes appeared in `windows-ci.do` and
`windows-input-identity.json`; preserve them and recheck concurrent work before
making implementation changes or staging a checkpoint.

## 1. Freeze the statistical and interface contract

For retained working outcome `u`, calculate once per estimator call

```
c = sum_i f_i u_i / sum_i f_i
z_i = u_i - c.
```

Use the same centering convention as point estimation. Under joint nuisance,
`u=y`; under fixed offset, `u=y-Z gamma_hat`. Frequency weights represent
physical copies; target weights and projection weights do not enter `c`.
Keep coefficients, residuals, retained sample, target population, deletion
units, point estimates and point-MCSE definitions unchanged.

Recommended route scope:

| Route | Mean highrank/q1 scope |
| --- | --- |
| Exact Mata | Existing observation-deletion, unit-frequency tuples and current population, target-weight and nuisance options |
| Native JLA observation | Existing explicit generic/Counter-V1 structured-inference tuples, including their existing controls and unit-frequency rules |
| Native JLA match | Existing fixed-offset, mover-only structured-inference tuples, with existing frequency and target weights and declared deletion IDs |
| Component plus projection | Permit Mean only where the combined request is otherwise already supported |

Both structured variance models remain available on their existing tuples.
Do not add Mata JLA or Rust exact component inference, frequency-weighted
observation inference, match joint-nuisance inference, match/stayer hybrid
inference, new automatic routes, or a new q-selection rule. Corrected
component inference remains unsupported. Preserve current q0/q1 posting and
target-local availability rules; do not manufacture a Gaussian `e(V)` for
native q1 or substitute q0 intervals after a q1 failure.

Record the agreed contract in a new dated decision and validation registration;
do not edit historical confirmation manifests. Preserve the existing
observation-q1 calibration failure and all structured-variance/fixed-offset
limitations.

## 2. Derive and independently check the fixed-c calculation

Start with exact observation deletion. Let

```
M = I - X (X'X)^(-1) X'
B_t = X (X'X)^(-1) Q_t (X'X)^(-1) X'
r_ti = B_tii / M_ii
C_t = B_t - [diag(r_t) M + M diag(r_t)] / 2.
```

For FEVC's translation-invariant targets, `M 1 = B_t 1 = 0` and
`diag(C_t)=0`. With `z=u-c 1`, derive and check

```
theta_hat_t(c) = z' C_t z
g_t(c) = C_t z
Omega_hat_ts(c) = 4 g_t(c)' V_hat g_s(c)
                   - 2 tr(V_hat C_t V_hat C_s).
```

The common-vector formula describes the native structured family. Exact
Mata retains its target-specific variance smoothing and polarization rules;
it must not silently adopt the native common variance model.

Only the realized outcome/influence and applicable raw variance proxies are
centered. Covariance probes remain zero-mean error draws through the original
quadratic kernel. Do not subtract the observed `c` from these error draws and
do not subtract each draw's own sample mean. Target matrices, eigenmodes,
spectral diagnostics and critical-value simulation retain their meanings.

For q1, use the leading mode `(lambda,v)` and the matching remainder kernel
`C_R` formed from `B-lambda vv'` and its adjusted diagonal:

```
b_hat = v' z
V_b_LO = sum_i v_i^2 z_i e_hat_i,-i
R_hat = theta_hat - lambda (b_hat^2 - V_b_LO) = z' C_R z
g_R = C_R z
V_b = v' V_hat v
Cov_hat(b,R) = 2 v' V_hat g_R
Var_hat(R) = 4 g_R' V_hat g_R - 2 tr(V_hat C_R V_hat C_R).
```

Preserve the raw leave-out recenter separately from the positive variance
model. Retain the exact remainder-identity gate and existing curvature,
critical-value and ellipse-image calculation. Check the formula in the
exact target-specific family as well as the common-vector family.

For match inference, prove the physical-block/collapsed-row identity before
changing guards. In the weighted scalar representation the centering
direction is `d_g=sqrt(F_g)`, and

```
z_g = sqrt(F_g) (ubar_g-c).
```

It is not the unweighted mean of the scaled match outcomes. Keep one error
draw per independent declared match, and preserve fixed-offset uncertainty
omissions.

Document the distinction between fixed `c0=E[ubar|X]` and sample `ubar`:
in the unit-observation case their estimator difference is
`(ubar-c0) r' M epsilon`. The approximation requires this difference to be
negligible relative to the relevant target and q1 remainder sampling scales.
Do not claim that a large row count or a small observed point difference
alone establishes this condition.

## 3. Implement exact Mata and public returns

1. In `fevc_inference.mata`, retain original working outcomes, fits and
   residuals; introduce a separate centered component outcome. Use the
   existing stable centering helper.
2. Feed that outcome into the raw proxy, target-specific smoothing, each
   primitive and polarized influence calculation, and all q1 leading/remainder
   calculations. Preserve original coefficients for public and projection
   results; exploit the target's constant-shift invariance explicitly.
3. Keep `vckss_inf__simquad` on uncentered, zero-mean Gaussian error draws.
4. Permit Mean in `fevc__centering.ado`; continue to reject Corrected and all
   unsupported inference tuples before estimator RNG.
5. Update the inference runtime build identity and its loader/test checks so
   stale installed Mata code cannot silently implement the old calculation.
6. Add a dedicated return such as `e(inference_centering)` with values
   `uncentered` and `fixed observed mean`, plus an explicit omitted-mean-
   uncertainty indicator. Final names must follow existing return conventions.
   Clear them on failures and irrelevant requests. Reconcile them with
   `e(centering)` rather than overloading `e(mcse_centering)`.
7. Display a concise note for active Mean component inference and include it
   in inference diagnostic/replay output. Update inference-guarantee metadata
   to describe the working approximation instead of inheriting an unqualified
   coverage statement.

## 4. Enable and certify native JLA with a separate capability

Inspection shows `generic_jla.rs` already centers `working_y` after fitting
and before the component attachments. Observation influence/q1 code consumes
that vector; match collapse already produces `sqrt(F_g)(ubar_g-c)`.
The residual-moment fitter receives unchanged residuals. Reuse these paths
after verifying the identities; do not introduce duplicate fits or centering.

1. Narrow the rejection in `generic_jla.rs` and
   `ffi_engine.rs::validate_attachment_centering` to Corrected.
2. Add `component_centering_api=1` using readiness bit 16 in the existing
   native capability export, the public C header, C shim probe and
   `fevc_rust.ado`. Preserve every ABI request/result layout and the Windows
   145-symbol export inventory; no new export is needed. Keep point/projection
   capability meanings unchanged.
3. Require the capability in `fevc.ado` before preparation/RNG for native
   Mean component inference. Native combined component/projection requests
   remain unsupported for Mean and None. Existing combined exact-Mata requests
   use the matching Mata runtime. None remains compatible with otherwise
   supported older native runtimes.
4. Treat absent capability metadata as zero; clear cached scalar state before
   and after a probe. Preserve strict Rust/Counter requests and existing
   pre-RNG fallback rules without inventing a new route.
5. Update header/layout/export checks, C transport mocks, FFI lifecycle tests,
   platform build audits and capability inventory. Check configure-before-
   attachment and configure-after-attachment validation.
6. Verify both ordinary and planned/direct/queued execution paths, diagonal
   and CMG solvers, receipt reconciliation, allocation planning and caller
   restoration. Generated CMG code needs no scientific change.

## 5. Add focused regressions before enabling public guards

Use independent dense oracles confined to tests; production must remain
matrix-free. Dense construction must not call production target/influence
operators. Same-route shifted-outcome equivalence complements, but does not
replace, the independent oracle.

Required algebra and behavior checks:

- Fixed-c point/influence/trace identities and all primitive cross-covariances.
- q1 raw recenter, leading/remainder covariance, direct remainder identity,
  independent eigen/ellipse checks and target-local unavailable statuses.
- Mean on `u` equals None on externally shifted `u-c` with identical semantic
  draws; default equals explicit Mean; explicit None retains its behavior.
- Additive outcome shifts and outcome-unit rescaling, with the appropriate
  powers for point estimates, SEs and covariance matrices.
- Frequency-copy and original-physical-block versus collapsed-match oracles,
  including unequal masses, target weights, controls and pooled deletion IDs.
- Four-target accounting and highrank `lincom`; supported combined projection
  requests; unchanged point results and ordinary point MCSE when attaching
  inference. Use registered MCSE-aware comparison where existing observation
  attachment ordering differs from point-only ordering.
- Exact Mata's smoother/polarization and both native structured models; default
  and partial Gram/probe batches; supported thread/order/batch changes and
  unchanged semantic RNG domains.
- Negative variance fits, nonpositive/singular/indefinite covariance, weak
  support, stale capability and runtime identity, corrupt receipts, unsupported
  requests, cancellation/reuse and memory-policy boundaries.
- Data, sample, sort, RNG algorithm/stream/state, native release and idle state
  on success, error and UserBreak; no hidden fallbacks or tolerance changes.

Extend or register tests alongside `test_inference.do`, the three native
component/individual/match tests, centering options, MCSE attachments, backend
routing, `lincom`, display/estat and clean installation. Preserve explicit
None regression references; replace only obsolete Mean-rejection assertions.
Register new focused tests in quick/full and affected installed-platform
profiles. Test genuine q0 and eligible q1 cases, not merely successful calls.

## 6. Register and run statistical validation

There is no accepted Mean-inference coverage policy yet. Freeze a new
registration rather than repurposing an old pass or relaxing a failed gate.

### Independent moment layer

For small Gaussian designs, form the actual Mean kernel `K=A' C A`, where
`A=I-1 a'` and `a'u=ubar`. Calculate exact bias and covariance of the actual
Mean estimator and its q1 decomposition. Compare them with the genuinely
fixed-population-mean oracle and the feasible variance calculation. This
separates approximation error from variance-model and numerical error without
requiring large Monte Carlo experiments.

### Development pilot

First run a two-draw end-to-end generator/estimator/validator/receipt smoke,
including deliberate failures. Then use 100--200 outcomes per selected cell
for development only. Certify the intended spectrum, remainder diffuseness,
leverage, support, positivity, identification and mean-weight concentration
before drawing outcomes. Record pilot timing to size the final assessment.

### Bounded sampling assessment (recommended baseline)

Proposed primary grid: three existing inference families (exact Mata,
structured native observation, structured native fixed-offset match), two
references (highrank and eligible q1), and two increasing independent-unit
counts: 12 route/regime/size cells. Use correctly specified heterogeneous
variance fixtures, selecting the primary targets by outcome-free diagnostics.
Proposed precision is 2,000 outcome replications per cell, with approximately
0.49 percentage-point binomial MCSE at 95% coverage. Freeze final dimensions,
counts and numerical budgets after the pilot and before assessment outcomes;
if the cost exceeds the agreed bounded scope, revise the registration first.

Predeclare a smaller diagnostic grid for homoskedastic errors, heavy tails,
mild and severe variance misspecification, weak/null signal, deliberately
multimode targets, concentrated match mass and estimated nuisance offsets.
Use both native variance models as primary/sensitivity fits where applicable.
All four targets must be reported, with q1 eligibility and diagnostic-only
targets fixed before outcomes. Do not turn a failed primary cell into a
diagnostic cell after seeing its results.

Pair actual Mean and genuinely fixed-c0 calculations using common outcome
draws; supplement with true-variance/dense references. **Recompute the observed
mean on every outcome replication, but never within inference error probes.**
Record point bias/true SD, approximation discrepancy/true SD, SE calibration,
leading/remainder covariance error, interval length, directional misses,
coverage among successful fits, all-attempt coverage and availability. Count
every attempt and enumerate all atomic and target-specific failure categories.

A fixed-outcome repeated-seed/budget diagnostic separates point JLA, Gram,
covariance and critical-value approximation error from sampling error. Exact
Mata and native smoothing families have different maintained models and are
judged against their own oracles, not forced to return identical intervals.

Before final outcomes, specify numerical oracle tolerances, success-rate
requirements and substantive thresholds for bias/SD, SE calibration and
coverage degradation, incorporating simulation uncertainty. Distinguish q1's
at-least-nominal target from an inappropriate symmetric penalty for conservative
coverage. The owner-selected evidence level determines whether these are
descriptive assessment screens or public-enablement confirmation gates.
Engineering errors always block completion. New material approximation
failures in intended primary regimes require diagnosis and reporting before
promotion. Existing historical limitations remain limitations.

### Harness and execution contract

Freeze source commit or immutable source bundle, binary/input hashes, semantic
RNG keys, DGPs, targets, exclusions, dimensions, counts, thresholds and expected
inventory. Keep development and assessment RNG domains separate. Test missing,
duplicate, partial, malformed, wrong-source/wrong-seed and scientifically
failing results; require appropriate nonzero exits. Validate reordered/sharded
execution, unique atomic outputs and complete aggregation. No shared CSV appends.

Start locally. If the cost justifies SCC, use the cluster workflow, a real
1--4-core end-to-end compute-node smoke, then scheduler-dependent stages.
Follow the bounded operational-repair rule. No broad performance campaign is
part of this feature by default.

## 7. Write the companion technical note

Create a new standalone file, preserving the existing dirty memos:

`/Users/johannes/Git/varcomp_hdfe/fevc-paper/technical-memos/centering/mean_centered_component_inference.tex`

Planned contents:

1. Scope, source cutoff, working outcome, retained physical-frequency mean,
   supported routes and the fixed-c approximation.
2. Exact observation algebra and fixed-c unbiasedness; distinction from
   conditioning on an estimated mean.
3. Highrank influence/covariance, target-specific versus common variance fits,
   and why inference error draws remain uncentered.
4. q1 score, raw recenter, remainder identity, joint covariance, curvature and
   ellipse mapping, with the target-specific identification assumptions.
5. Weighted match collapse and its physical-block interpretation.
6. What is omitted when `c` is estimated; exact Gaussian actual-Mean moments,
   the product remainder and relevant negligibility conditions.
7. JLA and numerical uncertainty, distinct from econometric uncertainty.
8. Independent tests and new assessment results, all failures and limitations,
   with exact source/input/binary identities and reproducibility commands.
9. Examples and explicit software-support versus statistical-evidence claims.

Ground literature claims in the local published KSS paper and the relevant
primary sources; record exact citations/pages for substantive claims. Do not
present the approximation as a new uniform coverage theorem. Use a small
companion reproduction directory for new synthetic code and machine-readable
results; keep large/raw artifacts in the appropriate evidence store.

Open the standalone LaTeX source in the built-in editor, use its compiler for
diagnostics, and visually inspect the rendered document. Produce the PDF via
the supported export/project build path if it is included in the paper commit;
do not commit editor-session files, auxiliary builds or temporary transport.
Writing outside the package workspace must use the authorized filesystem
permission mechanism; the user has already specified the destination.

## 8. Synchronize active documentation

Update public syntax, examples, support tables, restrictions, inference
assumptions, returns and installed runtime requirements in:

- `fevc/fevc.sthlp`, `fevc/README.md`, root installation/usage guidance where
  relevant, and `fevc/CHANGELOG.md`.
- `CENTERING.md`, `INFERENCE.md`, `MATRIX_FREE_COMPONENT_INFERENCE.md`,
  `INDIVIDUAL_INFERENCE_INTERFACE.md`, `FAILURES_AND_RETURNS.md`, `DECISIONS.md`
  and `docs/README.md`.
- `fevc/TESTING.md`, `rust/README.md`, `rust/TEST_PLAN.md` and native-boundary
  documentation for the capability and its tests.
- The machine-readable Rust/Mata parity ledger and its generated view,
  applicable native manifests and the current `fevc/PLAN.md` checkpoint.

State that Mean is now allowed only on the existing supported inference
tuples; explicit None remains available and Corrected is still withheld.
Keep observed-mean and nuisance-offset omissions distinct. Update the display,
estat help/diagnostics and installed-package checks consistently. Change the
package manifest only if new installed runtime/help files actually require it.

Update current documents, not immutable historical reports, source manifests
or qualification receipts. The new paper note has its own source cutoff;
do not silently rewrite the existing paper's source-bound claims or include
unrelated manuscript edits.

## 9. Engineering, platform and artifact gates

Run focused checks while iterating, then the affected source gates:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
rustup run 1.85.1 cargo fmt --manifest-path rust/Cargo.toml --all -- --check
rustup run 1.85.1 cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets --locked -- -D warnings
rustup run 1.85.1 cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
rustup run 1.85.1 cargo test --manifest-path rust/stata_backend/Cargo.toml --all-targets --locked
./.venv/bin/python fevc/tools/run_checks.py
./ci/run_ci_profile.sh plugin-build
```

The integrated checker already includes Python, CMG, Stata quick/full and
installation gates. Individual commands above are available for focused
iteration; avoid redundant reruns on unchanged source once their evidence is
complete. Native qualification still needs its clean-source profile.

Include C header/transport/export audits and the new focused tests in their
actual profile registration. Require application PASS markers and exact-source
receipts, not shell status alone. The native boundary change requires clean
source-local plugin qualification; dirty development results are separate.

Check complete-command overhead on a small representative grid if allocation,
copying or solve counts change. Record memory reconciliation and timings without
claiming representative-scale performance. Never change registered numerical
tolerances to pass a test.

For the chosen native-delivery scope, test the final staged and isolated-
installed binaries on each claimed platform. Mac thin arm64, Rosetta x86-64
and universal profiles do not establish native Intel hardware qualification.
Linux and Windows need their own source/binary-bound runtime checks. Use private
licensed Stata infrastructure only. If the existing Windows collector boundary
blocks qualification, report it separately and follow the agreed incomplete-
distribution policy; do not repair shared infrastructure without authorization.

Adopt only qualified intended payloads, generate their source/provenance
records, and repeat package/hash/install checks for changed distribution bytes.
Retain old candidate manifests unchanged. No tag, hosted release or manuscript
submission is implied by a source push.

## 10. Commit, push and final audit

1. Resolve the interview and save the accepted plan/registration before
   implementation. Recheck both repositories' status and instructions.
2. Implement and test in the existing main worktrees. Parallelize independent
   Mata, native, oracle and memo tasks with clear file ownership; serialize
   shared guards, metadata, integration and final review.
3. Make a clean local implementation/test checkpoint for source-bound native
   qualification and any frozen sampling run. The owner's requested commit
   phase permits this checkpoint; a final documentation commit follows results.
4. Bind receipts to the tested source. Record any carry-forward from that
   source to documentation-only commits with changed paths, checks, unchanged
   identities, reused claims and limitations.
5. Audit/stage only task changes. In the paper repository, commit the new note,
   its intended PDF/reproduction files and minimal task-owned index changes;
   preserve all unrelated existing edits and untracked files.
6. Review the complete outgoing commit range, including the four existing
   package ancestors if approved. Check source, binary, data and licensed-code
   boundaries. Ordinary private licensed logs and machine-local state stay out.
7. Push each intended repository's main branch to its verified origin, without
   force, only after its agreed gates. If the remote has advanced, inspect and
   integrate without rewriting history or discarding local work.
8. Verify the remote commit identities, installed-artifact scope if applicable,
   and final working-tree state. Report package/paper SHAs, tests, assessment
   results, platform scope, skipped/failed gates and any outstanding blockers.

Completion means both highrank and q1 implement the agreed approximation on
the agreed routes, independent and public tests pass, statistical evidence is
honestly classified, the technical note compiles, active docs agree with the
code, and the agreed commits and pushes are verified. An incomplete platform
set or failed confirmation is never relabelled as a completed qualification.
