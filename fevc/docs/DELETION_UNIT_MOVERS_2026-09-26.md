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

All five plugins pass source-bound build and runtime qualification. The
[manifest](../../native/deletion-unit-movers-20260926/manifest.json) preserves
actual build identities; the [package receipt](../../native/deletion-unit-movers-20260926/package.receipt.json)
binds the assembled payload. Public installation verification passed on the published package. Its source
and binary identities are bound by the final publication record below.

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
changes. The failed run remains failed. Retest job 7745342 at `31cf2ea7` passed the
full suite (including 256/256 pooled outcome draws), isolated installation,
and wrapper checks. Scheduler accounting reports failed=0, exit_status=0,
1,108 seconds elapsed, four slots and peak virtual memory 9.175G.
The exact qualified binary has SHA-256
`b743e9d8f58d6ac6b39d44a9f62ec4e894d72207f22cb6c45563c19eeeeeeb64`.

The first private Windows run returned STATA_DRIVER_FAILED with no stage
detail. Its source archive and exact hosted candidate hashes match the planned
input. The instance stopped, transient objects were deleted and the lock was
released. It does not qualify the Windows candidate; the limited receipt does not
establish the failing stage or root cause.

The Windows retest uses the already compiled CI candidate directly. The first
driver repeated a complete private build and Rust workspace suite inside the
licensed Stata process; that is unnecessary for exact-artifact testing and its
opaque failure did not identify a scientific assertion. For supplied candidates,
the bounded driver now performs the canonical PE audit, isolated installation,
all existing Stata runtime gates and installed-hash verification. Rust source
and standalone binary-unit checks retain their successful hosted-CI provenance
instead of being labelled as private checks. The fallback private-build path
remains available. No shared runner, machine or timeout policy is changed.


Private Windows retest `win-20260926T112550Z-3d85addd` passed with harness
source `c3ab9e52`, using hosted build 36237445599 from `d6f5571d` unchanged.
The controller confirms licensed Stata and project-test success, archive/source
integrity, stopped instance, deleted transient objects and released lock. The
[exact-artifact record](../../native/deletion-unit-movers-20260926/evidence/windows-qualification.json)
binds the candidate, snapshot and receipt. The frozen driver reaches PASS only
after PE/dependency audit, isolated installation, lifecycle, observation and
individual inference, match q0/q1 including one-firm multi-block workers, pooled
and positive projection regression, installed hash and idle-registry checks.
The fixed controller supplies aggregate source-bound PASS, not raw Stata logs
or individual project-check JSON. Rust tests retain hosted-CI provenance.

Mac qualification used Rust 1.85.1, Apple clang 21.0.0, macOS 26.6.2 and
Stata/MP 19; the source manifest binds 222 inputs. Linux used Rust 1.85.1,
glibc 2.28 and Stata/MP 19. Windows used Rust 1.85.1 static-CRT x86-64 MSVC
and private Stata/MP 19. Exact commands, versions, artifact hashes and route
scope are in each platform receipt. Source checks and all six hosted Rust
stable/MSRV jobs also passed at `c3ab9e52` (runs 36238744612 and 36238744595).

The new [compatibility reviews](../../native/README.md) compare qualified source
manifests with package source `c3ab9e52`: installed Ado/Mata, compiled Rust/C,
locked dependencies, scientific inputs and thresholds are unchanged from the
qualified builds. The changed progress assertion and private Windows harness
are named explicitly. Intel Mac runtime uses Rosetta. Existing unsupported
Windows routes and inference-coverage limitations remain unchanged; no new
statistical-coverage, representative-scale or release-tag claim is made.


## Final package assembly

`build_native_release.py --profile complete --repository-dir` assembled the
five qualified artifacts from clean source `c3ab9e52`. The deterministic package
digest is `03f6a9cf491576835fd4bc13fc8079cee13f6feb748efcba8d57db79357521de`;
54 catalog/payload files include 52 installed files. The root catalog removes
the obsolete Windows-testing-pending label. No release archive is published.
Windows receipts preserve original CRLF bytes in Git so recorded hashes remain
verifiable after cloning. Earlier qualification files are unchanged.

The first final Python suite passed 828 tests and rejected newly untracked
provenance files through the source-bundle guard. Staging those reviewed files
resolved the guard; its focused retest passed. No source allowlist or test
threshold was weakened. Detailed logs remain in the ignored evidence directory.


The final source rerun passed all 829 Python tests; generated CMG/parity and
whitespace checks passed. Fresh and replacement installs of the staged root
catalog passed all 52 installed-file hashes and native runtime gates under
Stata/MP 19 on macOS arm64. See [validation](../../native/deletion-unit-movers-20260926/validation.json)
and [staged installation](../../native/deletion-unit-movers-20260926/staged-install.json).


## Public installation and final checkpoint

Package commit `72da5542fb8a72593357d578ccb161e65dd48ac1` is published on
`main`. Fresh and replacement installations passed through both `net install`
and `github install` on Mac arm64 and Linux x86-64, under Stata/MP 19. Each
of the eight cases verified all 52 installed-file hashes, native reporting,
the README example and caller-data restoration, help/decomposition, match q0/q1,
the deletion-unit and positive projection regression, and an idle registry.
[Publication evidence](../../native/deletion-unit-movers-20260926/publication.json)
binds the package, both platform receipts and Linux scheduler accounting.
Windows exact-artifact isolated installation is recorded separately; both
public commands were not claimed as Windows runtime checks.

Linux public-install job 7745395 failed before launching Stata because the
system Python 3.6.8 lacks `subprocess.run(text=True)`. Its failure remains
recorded. The single environment-only retry used installed Python 3.12.4 after
verifier-import, subprocess and complete input-hash preflight checks. Job
7745408 passed all four cases with scheduler failed=0 and exit_status=0.
The package, verifier, fixtures, thresholds and qualified binary were unchanged.
A transient scheduler outage delayed execution; no duplicate job was submitted.

The published source checks and all six hosted Rust stable/MSRV jobs passed
(runs 36240285184 and 36240285275). This final documentation/evidence update
preserves every catalog and installed-payload byte from the tested package;
actual binary build identities remain `d6f5571d` and `31cf2ea7`.

All 31 original changed source files and useful development logs were preserved
in ignored `.local/deletion-unit-movers/original-worktree/`; original and backup
hashes were rechecked. The owner-authorized worktree removal follows final
evidence publication and verification of the matching remote `main`.
