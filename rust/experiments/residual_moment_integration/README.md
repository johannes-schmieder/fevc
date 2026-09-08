# Internal native residual-moment inference

This development harness calls the real generic-JLA observation estimator and
its internal residual-moment attachment on every outcome. It does not expose a
Stata option, change a default, or promote the candidate. The prospective
protocol is `fevc/docs/observation_residual_moments_integration_v1.json`.

`build.py` mechanically rebinds the immutable observation RC example and the
preceding follow-up adapter, then appends `adapter.rs`. The independent
coefficient-space oracle is research code, never linked into production core.
The build requires the pinned Rust 1.85.1 toolchain and local Cargo cache.

From the repository root, with a fresh output directory:

```bash
./.venv/bin/python rust/experiments/residual_moment_integration/build.py OUTPUT/build --tests
OUTPUT/build/integration-tests
./.venv/bin/python -m pytest -q rust/experiments/residual_moment_integration/test_harness.py
./.venv/bin/python rust/experiments/residual_moment_integration/build.py OUTPUT/build
./.venv/bin/python rust/experiments/residual_moment_integration/run.py OUTPUT/build/integration OUTPUT/tiny --tiny
./.venv/bin/python rust/experiments/residual_moment_integration/run.py OUTPUT/build/integration OUTPUT/development
```

The runner freezes the protocol, source bundle, generated adapter, executable
hash and task inventory before outcomes. Each task has an atomic output and
validated receipt. Call failures and target-local failures are retained; each
native attempt has one paired exact-oracle attempt. Thus the development run
contains 800 native calls, 3,200 native target attempts and 3,200 paired oracle
target attempts. The two numerical seeds share the same 100 outcomes per
design; they are not 200 independent statistical replications.

The exact arm reuses the native fitted variances, not oracle true variances.
A second independent calculation uses the native maker/target diagonals to
evaluate the realized point kernel and exact covariance diagonal. This isolates
the covariance-probe discrepancy from the JLA kernel approximation. Numerical
critical values are compared with independent quadrature at the same native
curvature. The scalar point MCSE is reported only for the three primitive
targets; their sum is not a valid total-target MCSE.

The native tests cover dense Gram agreement, outcome-free geometry, stable-key
row permutation, fitter batch invariance, Counter accounting, point invariance
to variance-learning streams, exact memory admission, cancellation, explicit
CMG/diagonal agreement, and unsupported inputs. These generated-oracle tests
must be run explicitly; they are not part of Cargo's workspace target discovery.

The 100-draw development slice cannot resolve the preceding 96.60% exact-input
joint-control overcoverage failure or establish general coverage. It does not
replace a newly registered end-to-end confirmation or public-boundary tests.
