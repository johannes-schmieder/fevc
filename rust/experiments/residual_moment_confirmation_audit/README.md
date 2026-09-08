# Existing-draw covariance diagnosis

This is separate from the frozen confirmation harness. It selects **every**
whole-call PSD rejection in `dominant_common_t8/16`, plus the first successful
draw as a reconstruction check. It requires the complete 50,000-call result
before running and binds the diagnostic to that result, source, executable,
selected replication inventory and original numerical/outcome seeds.

```sh
./.venv/bin/python -m pytest -q rust/experiments/residual_moment_confirmation_audit/test_audit.py
OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 ./.venv/bin/python rust/experiments/residual_moment_confirmation_audit/run.py RUN/confirmation RUN/build RUN/failure-audit
```

`RUN` denotes the original local confirmation directory. The diagnostic output
directory must not exist. The unchanged confirmation-generated source is
extended only with a data exporter; no estimator or original harness is edited.
An independent dense projection reconstructs the variance fit. Factorized
Gaussian quadratic forms reproduce the original 1,000-probe covariance, and
independent exact traces distinguish probe error from fitted-variance effects.
The successful check must match both native fitted predictions and its full
primitive covariance before any rejected draw is classified.

The true-error-variance comparison uses the **same realized-outcome covariance
estimator**, with known variance inputs. It is not the positive population
sampling covariance. Likewise, positive worker, firm and total marginal
variances do not establish valid q1 intervals: that would require a separate
target-specific covariance and interval audit. No rejected draw is rescued,
no PSD cutoff is relaxed, and the original confirmation status is unchanged.
