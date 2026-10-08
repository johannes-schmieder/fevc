# Private Windows runtime checks

Run the owner-controlled Windows skill's fixed `stata-do` profile from a
bounded exact-source snapshot. The repository-root `windows-ci.do` is the
only Stata entrypoint; it captures the project driver and writes
`WINDOWS_CI=PASS` only after the selected tests and project receipt succeed.
No public workflow invokes licensed Stata.

The transferred, temporary `windows-input-identity.json` selects
`runtime_profile: "smoke"` or `"full"`; omission means `"full"`. Keep this
machine-local identity out of source control. Both profiles build or verify
the exact native candidate, audit PE exports/imports, perform an isolated
`net install`, and exercise plugin lifecycle, exact/JLA default Mean point
estimates, the eight weighted Mean projection cells, and idle-registry
restoration. The smoke is a development diagnostic, not full Windows
qualification. When building privately it records
`PRIVATE_BUILD_ONLY_SMOKE` and does not claim the full Rust suites ran.

The full profile also retains all component-inference, pooled-deletion,
MCSE, and centering oracle tests. A private source build runs both locked
Rust workspace/backend suites; a prebuilt candidate must carry verified
hosted-CI source, build and Rust-test identities. The project receipt records
the selected profile, installed binary hash, and actual Rust-test scope.
Do not turn a smoke receipt into a full qualification claim.

Failure handling writes only an allowlisted stage name and numeric Stata
return code to `windows-project-failure.json`, then `WINDOWS_CI=FAIL` to the
terminal status file. This lets the fixed controller stop waiting promptly.
It never copies arbitrary exceptions, paths, environment values, or raw
Stata logs. A missing or malformed stage becomes `unknown`.

The currently documented fixed collector retrieves only its aggregate
`receipt.json`. It does not retrieve the project's failure/build/check JSON
or candidate binary. Therefore local project diagnostics do not by
themselves make a remote failure stage observable. Do not bypass that
boundary with cloud commands or infer which assertion failed from an
aggregate `STATA_DRIVER_FAILED`. A source-bound passing smoke establishes
only its declared smoke scope. Controller diagnostics or artifact collection
require a separately approved infrastructure change.

The older accepted-controller source uses a 15-minute deadline for Stata and
waits for the terminal status file, even if Stata has already exited. A
fresh private build plus full Rust/runtime suites may exceed that deadline;
the observed elapsed time of an old failure cannot distinguish a timeout
from an earlier unreported assertion failure. Keep build-only and full
qualification claims separate and use the smallest smoke first.
