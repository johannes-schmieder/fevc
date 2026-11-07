# Windows CMG qualification checkpoint — November 7, 2026

The implementation is committed at `7cc07a56` and targets fevc **0.5.2**.
It enables automatic and explicit shared full-CMG point, projection and
component-inference routes on Windows. Exact and explicit diagonal Stata
routing retain their existing behavior. CMG remains unmodified upstream
`e5613e81fb3b03c78de98658b372646fa802aabb`, with explicit 128-level configuration.

**Windows qualification and publication are pending.** No new binary has been
adopted into the installation payload. All five checked-in plugins retain
their released 0.5.1 hashes. The local Mac qualifier temporarily stages its
outputs after testing; those exact task-produced files were restored after
preserving the qualified candidates.

All 951 Python tests, the Rust workspace checks, generated-source/vendor checks,
and the integrated local Stata quick/full and installation suites pass.
Native Mac arm64 and Rosetta x86-64 thin/universal/installed qualification pass.
SCC Linux jobs `8496384` (smoke) and `8496385` (full native/installed) pass with
`failed=0` and `exit_status=0`; all 66 collected files match their remote hashes.
Windows-target Rust compilation passes, but is not Windows runtime evidence.

The [status and binary hashes](qualification-status.json) bind these claims to
the exact build source. Original source-bound receipts are copied under
`evidence/` without modification. Qualified binaries, the corresponding-source
archive, full sanitized logs, local validation logs and the accepted collector
update are retained in ignored local storage `.local/windows-cmg-20261107/`.

Hosted Windows package build and the complete Rust and SPI matrices now pass.
The [hosted checkpoint](hosted-checkpoint.json) records exact runs, binary hashes,
all three failed CI attempts and the [test-only compatibility review](hosted-compatibility.json).
The owner approved temporary branch `qualify/windows-cmg-20261107`, to be deleted
after release. Owner-maintenance authentication, collector repair cycle 1
and the required restricted backend acceptance now pass. The first private
full-CMG smoke also passes at exact build source C and candidate DLL hash; see
the [source-bound smoke record](evidence/windows-smoke/runtime.json). It covers
PE audit, clean installation, lifecycle, point/projection estimates, CMG boundary
checks and idle registry, with independently verified cleanup and stopped state.

All six runtime and installation groups also pass under the corrected full
confirmation registration; see the [confirmation checkpoint](confirmation-runtime-checkpoint.json).
The original incumbent request failed because it forced an unavailable Windows
0.5.1 CMG route. Its failure, the corrected native development pilot and both
registrations remain preserved. The new campaign compares supported incumbent
routes and the common weighted-observation mover population, with unchanged
candidate bytes and acceptance thresholds.

Remaining work follows the [Windows qualification workflow](../../rust/stata_backend/WINDOWS_CMG_QUALIFICATION.md):
finish the twelve fixed performance runs, adopt the five exact binaries only
if all scientific and timing cells pass, publish 0.5.2, and verify actual
published fresh/replacement installations. The assembled candidate already
passes [local installation and source checks](local-package-checkpoint.json).
No final-registration confirmation measurement is resampled or excluded; the
schedule stops on the first failed group.

Native Intel hardware remains untested; the Mac x86-64 evidence is Rosetta.
The initial development harness failures and superseded smoke are identified in
the status file. They do not constitute scientific or performance acceptance.
