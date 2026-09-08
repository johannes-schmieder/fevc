# Isolated Veneto residual-probe candidate, 2026-09-08

Status: **ISOLATED_CANDIDATE_COMPLETE_NOT_PROMOTED**.
The owner approved the same-input candidate test proposed after the
[conditioning diagnosis](VENETO_CONDITIONING_DIAGNOSIS_2026-09-08.md).
Both q0 and q1 now complete and return all four intervals. This resolves the
observed computation failure in the isolated candidate; it is not coverage,
release, or production-integration evidence.

## Change and scope

The candidate estimates the small moment matrix as half the centered sample
covariance of `Z'((g-Pg)^2)`, replacing
`Z'diag(1-2 hhat)Z + Cov[Z'(Pg)^2]/2`. These target the same population matrix
when the latter uses exact leverage and the projection is exact. The new
representation avoids the old estimate's approximate-leverage subtraction.
It changes the finite-probe estimator, not just floating-point evaluation.
The [mathematical review](UNIFIED_GRAM_DIAGNOSIS_2026-09-07.md) records the
identity and its limitations.

Only an ignored source copy and its isolated Apple Silicon plugin were
modified. The direct term is still captured for diagnostics but no longer
added. The original small-matrix rank check, model, basis reduction, solver
certificates, prediction floor, sample, point corrections, q-specific
interval calculations and numerical draw addresses are unchanged. Settings
remain 200 JLA probes, 512 Gram probes, 1,000 covariance simulations, 128
spectrum probes, seed 8675309, and the existing q0/q1 spectral budgets. No
ridge, eigenvalue clipping, tolerance relaxation, model selection or fallback
was introduced. No new outcome draws were generated.

## Results

The 71,614-row input retains 12,828 independent scalar match units and 15
active variance terms. Both calls return rc=0, with identical captured Gram
matrices and fitted coefficients.

| Diagnostic | Result |
| --- | ---: |
| Smallest scaled Gram eigenvalue | 0.0002472795 |
| Largest scaled Gram eigenvalue | 10.2460532 |
| Reported Gram reciprocal condition | 0.0000241341 |
| Rank tolerance, unchanged | 1e-10 |
| Fit moment relative residual | 1.369e-13 |
| Maximum Gram projection residual | 5.573e-11 |
| Maximum complete inference solve residual | 4.501e-10 |
| Nonpositive/floored variance predictions | 72 / 12,828 (0.5613%) |
| Positivity floor, unchanged rule | 5.9903e-11 |

The previous matrix's negative eigenvalue was -0.0142961. The candidate
passes the existing conditioning gate without altering it. Raw variance
predictions range from -0.00909848 to 0.100342 before the existing floor;
a small floor share is not proof that flooring is innocuous.

All exported point, plug-in, correction, KSS and numerical-MCSE entries are
exactly equal to the archived current-source point-only call after parsing
their 17-digit exports. Retained sample signatures and dimensions also agree.
q0 posts its admitted joint covariance; q1 correctly does not post Gaussian
`e(V)`. Every target-specific q0 and q1 status is zero.

The candidate's nominal 95% intervals are:

| Target | Point estimate | q0 interval | q1 interval |
| --- | ---: | ---: | ---: |
| Worker variance | 0.044839 | [0.041071, 0.048607] | [0.041860, 0.049166] |
| Firm variance | 0.014577 | [0.011998, 0.017155] | [0.012638, 0.017868] |
| Worker–firm covariance | 0.002171 | [-0.000117, 0.004459] | [-0.000725, 0.003681] |
| Variance of their sum | 0.063757 | [0.061638, 0.065876] | [0.061634, 0.065882] |

For worker, firm and covariance targets, estimated leading squared-eigenvalue
shares are 0.267, 0.289 and 0.325; remainder-leading shares are 0.059, 0.066
and 0.077. Thus q0 computation alone should not be described as establishing
diffuseness. q1 removes a substantial leading mode here, but the diagnostics
do not prove its asymptotic conditions. For the variance of the sum the
leading share is about 0.000124 and q0/q1 intervals are nearly identical.

Recorded command times are 67.099 seconds for q0 and 30.032 seconds for q1.
Runs partially overlapped on the same host and included diagnostic output;
these are not timing comparisons or benchmarks.

## Checks and evidence

Evidence root: `.local/diagnostics/veneto-residual-probe-20260908/`.
The frozen pre-run manifest binds all copied source files, the original
production source/binaries, the existing input and the unchanged paper driver.
All these identities were rechecked after execution.

- An independent six-dimensional, three-node Gaussian quadrature verifies
  the population covariance identity to an absolute tolerance of 1e-12.
- A new Rust test compares the candidate with an independent two-pass sample
  covariance on the exact same 512 probes, to 1e-12. It also verifies that
  perturbing leverage while holding the basis and projection fixed leaves
  the candidate Gram unchanged, and checks Counter accounting.
- Eleven focused Rust tests pass, including that new test, dense and live
  quotient-solver comparison, batch invariance, exact population identities,
  rank/input/memory rejection, invalid callback certificates, cancellation,
  positivity failure checks and stable centered covariance.
- Two old tests were explicitly excluded from this focused run because they
  assert properties specific to the old decomposition: exact rather than
  simulated `Z'Z` when `P=0`, and rejection caused by an inconsistent supplied
  leverage vector. This is not a full-suite pass. Their replacement contracts
  and full regression coverage belong to any later integration.
- The separate output audit verifies source-bound archived point/sample
  equality, complete target statuses, solver/probe settings, q0 normal-interval
  arithmetic, and q1 covariance positivity and ellipse-image endpoints using
  one million angular points. Maximum q1 endpoint discrepancy is 1.36e-14.
  This checks arithmetic conditional on the returned covariance and critical
  value; it does not independently validate their statistical calibration.

Commands used the pinned Rust 1.85.1 toolchain and an isolated
`CARGO_TARGET_DIR`, with `--release --locked --offline`:

```text
cargo test --manifest-path <copy>/rust/Cargo.toml -p vckss-core --lib direct_residual_candidate
cargo build --manifest-path <copy>/rust/stata_backend/Cargo.toml
cargo test --manifest-path <copy>/rust/Cargo.toml -p vckss-core --lib residual_moments::tests -- --skip zero_projection_matches_independent_ols_and_accounting --skip high_leverage_does_not_imply_rejection_but_indefinite_gram_does
```

Local Stata-MP 19 used eight processors and `RAYON_NUM_THREADS=8`, invoking
the unchanged paper `run_unified_match_scaling.do` with the copied source,
existing Veneto CSV, separate q0/q1 output directories, mode and `1000`.
`check.py freeze`, `check.py audit` and `check_outputs.py` were run through
the repository `./.venv/bin/python`. Receipts are exclusive-create and were
not overwritten. No broad platform, full production-suite, scaling or
coverage campaign was run.

SHA-256 identities:

- Manifest: `3f2a8165c6c5fb625d11ceca1c163cf6f1345b9cbd73046d3628f1fdb528e9d3`.
- Result: `6adffac10a86b3cb52377465e56fed38294e3fed981200544dc6b66e0521844f`.
- Output audit: `1eeae44b00683fd2faa31cab3110eabd2da4045221fd6bd3ec74f29caf0e1b29`.
- Isolated plugin: `b6b09f5af913b5feb0f619d025bcbb340356d231b963b1930e8e49e763147151`.
- Input: `f9cbbc16626bfc2848fcca945cf5d2c869c0354252f00e9000f66042d9aa2c36`.

## Next decision

This is a promising common-fitter candidate for both deletion modes, not a
reason to add separate q0/q1 or observation/match fitters. Before production
adoption, perform a bounded numerical-seed stability check and replay the
existing saved-outcome validation fixtures under a prospectively fixed scope.
Assess conditioning, flooring, interval availability and SE/coverage changes;
do not tune cutoffs or select successful seeds after observing results.
If satisfactory, integrate with appropriate legacy-interface protection,
focused failure regressions, native/Stata checks, and a precise paper-formula
and evidence update. No new large Monte Carlo campaign is implied. Production
and the manuscript remain unchanged by this isolated test.
