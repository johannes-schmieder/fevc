# Deletion-unit mover integration — September 26, 2026

The owner authorized integration into `main`, complete Mata/Rust support,
qualification of all five shipped plugins, source then package publication,
and removal of the original development worktree after final verification.
The earlier [Mata repair record](POOLED_DELETION_2026-09-25.md) remains historical.

## Implementation

Match mover eligibility is frozen from original complete-case deletion units.
Both graph implementations count active deletion units throughout pruning and
final certification. Repeated rows/copies do not add units. Distinct blocks at
one coefficient cell remain parallel edges. Original one-block stayers alone
are eligible for physical-observation augmentation; graph-dropped movers never
return as stayers. Default worker–firm blocks and observation semantics are
unchanged.

Rust match-component inference applies the same deletion-unit definition.
Existing worker-articulation, full/deleted rank, solver, residual, accounting,
memory and inference scope contracts remain in force. Connectivity under each
deletion is necessary; the retained worker-articulation restriction is stronger.

Readiness bit 15 (`VCKSS_CORE_DELETION_UNIT_MOVERS_V1_READY`) is additive to
the existing native capability receipt. Updated native/private preparation
uses the new semantics. The public wrapper protects older binaries before
preparation/RNG: eligible automatic requests use Mata; strict native requests
with parallel blocks return `RUST_PARALLEL_DELETION_UNSUPPORTED`. ABI layouts
and public syntax are unchanged.

## Development validation

Logs and exact command output are retained in ignored
`.local/deletion-unit-movers/`. The initial regression failed on the former
Rust selector, which excluded the same-firm multi-block worker. The corrected
graph passes its focused tests and an independent reachability/removal oracle
over all 728 nonempty three-worker/two-firm graphs with zero, one or two blocks
per cell, repeated rows and unequal physical masses.

The pooled regression uses independent physical expansion and literal deleted
QR fits. Its projection oracle now omits the last firm to match the documented
display normalization: unlike centered variance components, the projection
intercept depends on that choice. Existing indefinite covariance cases remain
withholding tests; an additional positive-covariance fixture must succeed.
No estimator tolerance or acceptance threshold is changed.

The match-component regression includes a worker whose twenty original blocks
share one model firm and checks successful q0/q1 execution, unchanged point
results, independent-unit counts and complete-system residuals. These are
engineering tests, not an extension of historical inference-coverage claims.

Development checks passed with Rust 1.85.1 (locked workspace/all-target tests,
strict Clippy, formatting and standalone backend tests), 829 Python tests,
CMG assembly verification, the focused routing and q0/q1 regressions, and the
Stata/MP 18 full pooled regression. All 256 outcome draws succeeded with seed
9252026; target biases were inside the predeclared Monte Carlo envelope.
Exact checks use scale-aware 1e-8 gates, randomized point checks use six
numerical MCSEs, and projection covariance uses the existing .005 relative
engineering gate while preserving original-system residual certification.

A bounded preparation timing diagnostic used 40,000 rows, 1,000 workers and
100 firms, two Stata processors and ten preparations per design. Baseline/new
totals were .785/.581 seconds for ordinary match blocks and .581/.600 seconds
for parallel blocks with unchanged mover status. This is a development smoke,
not a general performance claim. The original worktree source and useful logs
are backed up in the ignored integration evidence directory.

The integrated `./.venv/bin/python fevc/tools/run_checks.py` completed with
`FEVC LOCAL QUALIFICATION PASS`: quick/full Stata, clean installation, helper
migration, benchmark validators and the retained-sample bridge audit all pass.
The later capability-cache safeguard passed its focused routing regression.

## Qualification status

All-platform qualification and publication are in progress. Repository binaries
retain their prior identities until new source-bound gates complete. Final
receipts will identify exact source, binaries, platform checks and limitations.
The original development worktree remains preserved until final publication
and installation verification succeed.

## Platform qualification follow-up

Source `d6f5571dca34775025cd6e7500cef77896a2afc2` passed clean macOS
qualification (arm64, Rosetta x86-64, thin and universal artifacts, isolated
installs), GitHub source checks and all six stable/MSRV Rust CI jobs.

Linux job 7745310 compiled successfully but failed the progress-display test:
it assumed one allocation summary for exact match/both, although the existing
native execution runs a mover pass and a combined stayer pass with potentially
different forecasts. Callback timing determines whether both are observed.
The harness correction permits at most one forecast per actual pass and keeps
the single-method assertion; unchanged summaries remain covered by the Rust
tracker regression. No estimator, fixture, solver or scientific threshold
changes. The failed run remains failed, and Linux qualification must rerun.

The first private Windows run returned STATA_DRIVER_FAILED with no stage
detail. Its source archive and exact hosted candidate hashes match the planned
input. The instance stopped, transient objects were deleted and the lock was
released. It does not qualify the Windows candidate; investigation is ongoing.
