# Internal observation residual-moment confirmation

This local harness adds a registered 20-cell confirmation entrypoint to the
unchanged native integration experiment. It does not expose a Stata option or
alter the estimator. The protocol is
`fevc/docs/observation_residual_moments_confirmation_v1.json`; scientific gates
are imported unchanged from the original observation-inference campaign.

Run from the repository root with `./.venv/bin/python`. Use a fresh directory
for every command that produces evidence; accepted outputs are never replaced.

```sh
./.venv/bin/python rust/experiments/residual_moment_confirmation/build.py RUN/build
./.venv/bin/python rust/experiments/residual_moment_confirmation/build.py RUN/build --tests
RUN/build/confirmation-tests
./.venv/bin/python -m pytest -q rust/experiments/residual_moment_confirmation/test_harness.py
./.venv/bin/python rust/experiments/residual_moment_confirmation/run.py preflight RUN/preflight --exe RUN/build/confirmation
./.venv/bin/python rust/experiments/residual_moment_confirmation/run.py verify-shards RUN/shards --exe RUN/build/confirmation --preflight RUN/preflight/receipt.json
./.venv/bin/python rust/experiments/residual_moment_confirmation/run.py run RUN/tiny --exe RUN/build/confirmation --preflight RUN/preflight/receipt.json --profile tiny
./.venv/bin/python rust/experiments/residual_moment_confirmation/run.py run RUN/confirmation --exe RUN/build/confirmation --preflight RUN/preflight/receipt.json --profile confirmation
```

`RUN` above is a placeholder for a fresh local evidence directory, not an
environment variable. Complete package source gates and record prerequisites
before starting confirmation. The run freezes source, executable, protocol,
preflight and all task keys before outcomes, then starts up to eight local
processes. Do not edit frozen sources while it runs. Tiny and confirmation
profiles use different semantic outcome masters; the numerical stream is fixed.

Each task writes a private gzip partial, validates all attempted calls and
targets, atomically renames the raw output, and emits a source-bound receipt.
Aggregation checks all 200 confirmation tasks and all 400,000 target rows,
including unavailable intervals. A scientific failure returns exit code 2
after the complete inventory has been audited; it is not an infrastructure
failure and must not be relabeled or retried with changed gates. Interrupted or
corrupt output is not accepted as partial confirmation.

The paired exact calculation uses the native fitted variances. It is diagnostic
and unavailable when the native call cannot export them. Only the native arm
is gated. Null/weak-signal and multi-mode-covariance limitations, all previous
failures, and the separate fixed-offset match RC remain unchanged. This is a
small-design, single-numerical-stream experiment, not a scalability benchmark
or native-platform/public-release qualification.
