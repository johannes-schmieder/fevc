# Observation q1: individual intervals survive the joint-covariance failures

## Result

All 43 `dominant_common_t8/16` draws rejected by the previous confirmation's
joint covariance check have computable worker, firm and total q1 intervals.
Their leading/remainder covariance matrices are comfortably positive, not
borderline cases rescued by numerical tolerances. The same conclusion holds
with exact traces and independently constructed exact-design kernels.

The experiment also replayed the first 43 originally successful draws. Their
full-call results agree with the frozen confirmation output. All 86 calls and
344 target attempts were accounted for. The original 43 full-call rejections
still occur in the diagnostic executable: its capture block observes prepared
q1 results without bypassing or changing the joint covariance check.

This is **existing-draw diagnostic evidence**, not a new coverage confirmation.
The original 50,000-call confirmation remains **FAIL** under its unchanged
availability requirement. No production source, public option, fitted-variance
rule, tolerance or previous evidence was changed in this audit.

## What was checked

The [prospective protocol](observation_residual_moments_q1_target_audit_v1.json)
selected every one of the 43 original rejections and the first 43 successes
before this target-specific calculation. All use the original outcome keys,
numerical seed, 3,200 JLA probes, 512 Gram probes, 1,000 covariance probes,
128 spectral probes, spectral iteration/tolerance settings, and 100,000
Counter critical draws. There are no new outcomes, substitute draws, larger
probe budgets, regularization or covariance repairs.

The frozen confirmation source bundle was copied into a disposable local
build. Its sole core difference is a read-only capture block immediately
before `finish_component_covariance`. That block exports the already computed
numerical modes, remainder influences, fitted variances and probe moments, and
uses the existing target finalizers. The original full-call gate then runs.
This instrumented executable is a diagnostic, not a qualified native binary.
Capture evaluates the same deterministic target-keyed critical draws outside
the ordinary return path; on successful calls the original path repeats them.
Its runtime and extra calculations are not production resource receipts.

An independent Python calculation reconstructs the model projector, raw
leave-out recenter, rank-one remainder kernel, original Gaussian probe
variances, and exact traces. It checks interval endpoints through polynomial
stationary points of the ellipse image, independently of the core's angular
search. Separate Gaussian quadrature checks the simulated critical values.
The complete same-input calculation uses neither production covariance nor
production interval finalizers.

| Target | Rejected draws with computable q1 interval | Smallest standardized covariance determinant | Successful comparison draws reproduced |
|---|---:|---:|---:|
| Worker variance | 43/43 | 0.999875 | 43/43 |
| Firm variance | 43/43 | 0.999792 | 43/43 |
| Total variance | 43/43 | 0.986368 | 43/43 |

The unchanged determinant threshold is `1e-8`. All three targets remain
computable in every selected draw under the exact-trace native-kernel,
exact-design exact-trace, and true-variance-input diagnostics. The last arm
changes inputs in the same realized-outcome covariance estimator; it is not
the population sampling covariance.

The covariance-component interval is also numerically computable in all
86 draws, but it remains nonprimary: its exact remainder spectral share is
0.6484, so one-mode removal does not give it the required diffuse remainder.
For worker and firm targets, exact leading shares are 0.8407 and remainder
shares are 0.07295. The total target is already diffuse (leading share 0.03333,
remainder share 0.03448); its repeated leading eigenvalue allows different
numerically valid mode orientations. Exact-design and native-mode intervals
are therefore separate diagnostics, not required to be identical.

Among the selected rejected draws, the original-probe intervals cover truth
in 42/43 worker, 43/43 firm and 41/43 total cases. These are descriptive counts
conditional on the original rejection, **not estimates of unconditional
coverage**. They must not be substituted into the old confirmation to relabel
its result. No null-signal or arbitrary-heteroskedasticity claim follows.

## Why this is possible

The full primitive covariance matrix describes the worker-variance,
firm-variance and covariance estimates **jointly**. Its positivity check
protects joint inference and arbitrary linear combinations of those estimates.
The earlier exact-trace audit established that this matrix really is
inadmissible on these 43 draws with the fitted variance inputs. More trace
probes did not fix it.

An individual q1 interval uses a different, two-coordinate covariance matrix:
one coordinate is the leading-mode coefficient and the other is the
rank-one-subtracted remainder. A failure of the three-component matrix does
not imply failure of this target-specific pair. The current all-or-nothing
reporting path withholds both, because it checks the shared matrix first.

More precisely, write the model residual maker as `M`, the point kernel as
`C`, the leading mode as `v`, its eigenvalue as `lambda`, and the numerical
leave-out inverse diagonal as `m`. The remainder kernel used in the audit is

```text
R = C - lambda v v' + sym[diag(lambda v_i^2 m_i) M].
```

Thus `y' R y` equals the corrected point minus
`lambda [(v'y)^2 - sum_i v_i^2 y_i m_i (My)_i]`. The raw leave-out
recenter is retained; fitted positive variances do not replace it. With
`u = R y` and fitted variances `s_i`, the pair covariance has entries

```text
V_b = sum_i s_i v_i^2
C_br = 2 sum_i s_i v_i u_i
V_r = 4 sum_i s_i u_i^2 - T_R,
```

where `T_R` is the original Gaussian-probe variance, or
`2 tr(R diag(s) R diag(s))` in the exact diagnostic. Its own positivity,
correlation-determinant, mode, identity and interval checks all pass here.
The q1 interval is the image of its confidence ellipse under
`lambda b^2 + r`. None of those target-local computations needs the rejected
three-component covariance matrix to be reported as valid.

This identifies an overly broad **withholding dependency for individual
intervals**, not a reason to weaken the joint covariance check. It does not
erase the fitted-variance issue in the joint matrix, or prove the statistical
assumptions in other designs.

## Next bounded implementation

The next step should be an explicit **internal marginal-only result contract**,
keeping the residual-moment fitter and numerical settings fixed:

1. Separate individual interval availability from joint covariance
   availability. Preserve fatal handling of shared identification, fit,
   solve, nonfinite and target-identity errors. Each q1 pair must still pass
   its own checks; no failed pair receives a replacement interval.
2. Preserve the current strict joint-request behavior. For a marginal-only
   request, an invalid joint matrix must be withheld with an explicit status,
   never clipped, silently filled, or exposed as a valid `e(V)`. This should
   be an explicitly selected result contract, not a post-failure fallback.
3. Add focused regressions for these rejections, successful-output invariance,
   target-local covariance failures, stream/accounting preservation, and
   withheld joint postestimation. Keep the public match interface untouched.
4. Prospectively register an existing-draw development assessment across the
   original observation matrix, including mild/severe misspecification and
   null cases. Count every requested target, not only returned intervals.
   For q0, a separate positive marginal variance check would be required;
   this q1 audit alone does not qualify q0 marginal reporting.
5. Only after the result contract and development checks are settled, decide
   on independent confirmation and public integration. Preserve every older
   failure, target exclusion and stated limitation.

This is a smaller and better-targeted next step than another change to the
variance fitter. It is not implemented or publicly promoted by this report.

## Numerical and source record

The 86-call replay and full independent audit took 56.86 seconds locally;
this is not a scaling benchmark. Maximum discrepancies were:

- fitted variance reconstruction: `5.81e-12` scale-relative;
- remainder influence: `3.68e-13` scale-relative;
- q1 numerical quantities: `3.20e-12` scale-relative;
- independently optimized interval endpoints: `5.74e-13` scale-relative;
- original-probe scalar versus dense matrix: `7.53e-16` scale-relative;
- direct remainder identity: `7.11e-15` absolute;
- critical-value reference CDF error: `0.000965`, below `0.003`;
- 64/128-node quadrature difference: `9.22e-15`.

The [machine-readable result](observation_residual_moments_q1_target_audit_v1_result.json)
records artifact identities and final checks.
Local raw evidence is under
`.local/diagnostics/residual-moment-q1-target-audit-20260906/`.
The full manifest is
`c19b478d90e41e9586bbd4f14b2a5e2a7bcce6a186091c830360a06fec9803a9`;
the captured export is
`9edfc453a6f886a5779acd906b1ed1369027c65d2dea476c5f70e95bf0a33b3b`;
the raw result is
`a279a2b4de129d003968576f5ca3f5f9abcb1d5fdb744f20871eafc7185c2e73`.

The source/input/raw-output archive contains 193 verified files, including all
ten source-bound original task files used for baseline comparisons. Its SHA-256
is `7832427ba66a487e6b4c1c94d46d5aa3dbc9a3f97aa89de58dc95e017d212087`
(42,225,480 bytes). Every member hash and the archived 86-call/344-target
inventory were revalidated.

The 25 focused algebra, interval and malformed-inventory tests pass before
and after replay. The two-call/eight-target pilot completes the full
generator-to-validator path and deliberately rejects duplicate CLI keys.
The ordinary 726-test Python suite, CMG assembly check, public-identity audit
and `git diff --check` pass. The integrated local runner also passes its
Python/CMG, Stata quick/full, isolated-install and benchmark/sample smoke
gates, ending with `FEVC LOCAL QUALIFICATION PASS`. Pytest emitted nonfatal
temporary-directory cleanup permission warnings. Exact invocations and
tool-returned log excerpts are retained in the local packet; excerpts are
not represented as complete terminal logs. These are engineering checks,
not a new coverage or native-plugin qualification.

The source baseline is dirty `main` at
`1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`, with exact file identities taken
from the previous confirmation manifest. All 143 frozen original source
files remain byte-identical in the live worktree. Only the disposable copied
`generic_jla.rs` gains the capture block. New experiment files, its protocol,
this report, machine result and active index/checkpoint updates are the
changed surface. Production/build inputs, public ABI, solver/RNG/fit behavior,
original campaign inputs and acceptance rules are unchanged. The previous
confirmation's failures and successful-calibration observations remain
applicable only with their original scope; they do not qualify the new
diagnostic executable or the proposed marginal-only contract.

No new native platform, SCC, Windows, release, paper or public distribution
work was performed.
