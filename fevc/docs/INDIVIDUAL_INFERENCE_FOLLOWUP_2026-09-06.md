# Bounded q0 repair and saved-draw q1 diagnosis

The q0 numerical repair is implemented: public match high-rank inference now
uses 512 spectral iterations rather than 128. **JLA stays at 200 probes.**
The eigensolver, spectral residual limit, variance fitters and statistical
formulas are unchanged. Observation inference already used 512 iterations;
match q1 still uses 128. There is no adaptive retry or hidden tolerance change.

The original 19,200-call development result remains **FAIL**. This separately
[registered follow-up](individual_inference_followup_v1.json) diagnoses that
result; it is not a replacement coverage experiment or confirmation. The
[follow-up receipt](individual_inference_followup_v1_result.json) binds its
inputs, source, binaries and retained local output. No new outcome draws,
paper changes, release action or external compute were used.

## q0: sufficient iterations, at a measurable cost

The replay includes every one of the 15 match-q0 cells, including the q1
campaign's q0 comparator, at existing replications 0, 133, 266 and 399. Four
unchanged q1 calls check identity. These 64 baseline/candidate pairs total
128 native calls. The old binaries reproduce the saved draws. The candidate
resolves all 32 selected status-6 target failures, with no new failure and
unchanged point estimates, numerical MCSE, scalar variances, covariance
matrices, fit diagnostics and Counter draws. Nonpositive-variance failures in
weak/null stress cases remain failures.

The maximum candidate spectral residual is 0.0009521, below the unchanged
0.002 limit. Every original regression residual also passes its existing
gate. There are 6,144 additional certified solves and 245,760 additional
admitted bytes per successful q0 call, for the extra solve receipts. No new
random draws or growing spectral basis are added. The median paired runtime
ratio is 1.887; median baseline and candidate call times are 0.269 and 0.509
seconds. These small local fixtures do not establish large-data performance.

An additional independent coefficient-space eigendecomposition checks all
four targets in the 15 first-replication fixtures. Its maximum relative
leading-root discrepancy is 0.024%; the maximum second-root discrepancy,
scaled by the true leading root, is 0.270%. Trace-square differences are at
most 1.081 reported Monte Carlo standard errors. This is consistent with
closely spaced, only approximately resolved modes. A small eigenvector
residual does **not** prove that the exact first and second roots have been
identified in order. q0 intervals do not use the leading-mode decomposition;
the spectral calculation supplies diagnostics and the existing numerical
availability gate. No exact-eigenvalue claim is added.

Stata now validates the spectral probe and iteration budgets in every target
receipt and posts the returned values. A new near-tied diffuse-geometry test
covers diagonal and CMG routes at the omitted 200-probe default; another test
rejects a corrupted spectral budget. The first integrated test correctly
caught a stale hard-coded reported iteration count after execution had
already switched to 512. The poster was corrected to use the validated
receipt. That failed log is retained separately from the final checks.

## q1: the two cases need different explanations

All 400 saved draws in each flagged cell and all four targets are analyzed;
the covariance target retains its nonprimary status. An independent
stationary-quartic calculation reproduces the saved ellipse endpoints to
maximum scaled error `4.4e-16`. Curvature identities agree, and deterministic
integration agrees with the 100,000-draw critical radii within the registered
simulation-error check (maximum CDF discrepancy 0.00103). These checks find
no interval-arithmetic or curvature-formula defect.

The following variants keep the outcomes and point estimates fixed. They are
diagnostic counterfactuals, **not alternative qualified intervals**.

| Flagged target | Original q1 | Exact integrated radius | Normal radius only | Opposite-half covariance |
| --- | ---: | ---: | ---: | ---: |
| Observation worker | 97.75% | 97.75% | 97.75% | 96.25% |
| Observation total | 97.75% | 97.50% | 97.25% | 94.50% |
| Match total | 98.25% | 98.25% | 97.25% | 97.75% |

“Exact integrated radius” removes critical-value simulation error, not
covariance-estimation error. “Normal radius only” substitutes 1.959964 for
the conservative q1 radius but retains the nonlinear ellipse image; it is
not a q0 interval and need not be valid for q1. “Opposite-half covariance”
uses the empirical covariance of the leading score and remainder in the
other 200, parity-indexed replications, with curvature and radius recomputed.
It neither recenters the point nor adjusts bias. That simulation-only
covariance estimate is noisy and cannot be used on an empirical dataset.

For observation worker inference, empirical leading and remainder variances
are both about 89% of their average modeled values. For observation total,
the leading variance ratio is 1.028 but the remainder ratio is 0.800. The
total target is actually diffuse in this design, and its mean curvature is
only 0.0132; its mean critical radius of 1.9706 is already close to normal.
Changing that radius has little effect. Replacing the covariance has a much
larger effect. Thus covariance overstatement relative to these saved draws,
particularly in the total remainder, is the main diagnostic lead. This does
not yet distinguish variance-fit bias, fixed-sketch approximation error, or
an unusually low empirical variance in 400 draws.

For match total, empirical leading and remainder variance ratios are 1.031
and 1.022: covariance magnitudes are close. Its mean curvature is 0.399 and
mean radius is 2.112. The normal-radius counterfactual reduces mean interval
width by about 7.6%, and coverage falls by one percentage point. Conservative
q1 construction therefore contributes, but this exercise does not explain
every remaining percentage point. It does not justify changing the critical
formula or relabeling the old coverage failure.

The q0 scalar SE ratios in the original q1 report are auxiliary calibration
diagnostics. They are not standard errors for the nonlinear q1 interval.
Similarly, no significant-looking change in this selected, 400-draw
exercise establishes independent coverage performance.

## Boundary and next step

Keep the bounded q0 repair and the unchanged q1 method. The useful remaining
scientific check is an exact population-covariance calculation on these same
fixed designs: compare the true DGP covariance with both the fitted covariance
and the realized 400-draw covariance. That would distinguish persistent
studentization error from simulation variation without another coverage
campaign or a new fitter. A fresh confirmation requires a separate prospective
decision, frozen source and unchanged acceptance rules; the original FAIL
cannot be erased by this diagnosis.

Engineering checks and their exact scope are in the linked receipt. Production
checks pass: 733 package tests, 21 separate diagnostic tests, deterministic
CMG assembly, Stata quick/full/install, six same-data Stata/native comparisons,
and focused Apple Silicon/Rosetta tests of the final wrapper. Production
Rust, ABI, build inputs and the existing three Mac binaries are unchanged;
this follow-up changes the Stata budget, validation/posting, help and tests.
Existing native-build evidence can therefore support those unchanged
artifacts, but cannot qualify the changed numerical default scientifically.
No Linux, Windows, full-scale performance or release claim is made.
