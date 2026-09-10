# Windows inference qualification attempt, 2026-09-08

Outcome: **FAIL / STATA_DRIVER_FAILED; NOT QUALIFIED**. The exact committed
candidate was attempted once through the accepted private Windows wrapper.
The machine is stopped, transient transfer objects were deleted, and the
shared lock was released. No retry or infrastructure change was made.

Source: `f3098bc1369992fccbc1d276aac5fc65ceb3f404`, clean when packaged.
Run: `win-20260908T153447Z-26c5c40c`, profile `stata-do`.
Archive SHA-256:
`2be19cc707f5f8329dfdd24e6a06b02cbe0b4bf61a2cf6d7d5a1805475cdc287`.
Receipt SHA-256:
`ca18cfdfee84400c435c259f6e86f9d5b5960e0e5fb99a846920c6ef884b659a`.
The controller completed at `2026-09-08T15:50:54.7912735Z`; wrapper cleanup
completed afterward with exit status 51. Immutable sanitized evidence is under
[`INFERENCE-WINDOWS/f3098bc1369992fccbc1d276aac5fc65ceb3f404/`](../../rust/qualification/evidence/INFERENCE-WINDOWS/f3098bc1369992fccbc1d276aac5fc65ceb3f404/).

Commands from the repository root:

```text
./.venv/bin/python -m pytest -q fevc/tests/python/test_windows_ci.py
/Users/johannes/.codex/skills/windows/scripts/windows_ci.sh inspect
/Users/johannes/.codex/skills/windows/scripts/windows_ci.sh run --project fevc --profile stata-do
```

The two local Windows-harness tests passed. Inspection passed using the
accepted unattended identity and showed the machine stopped before the run.
The remote receipt passes execution identity, source hash, safe archive paths,
source manifest and shutdown-backstop checks. It marks Stata and project-test
checks false and gives only `STATA_DRIVER_FAILED`. It provides no compiler
result, precise failing stage, Stata error, binary hash or test marker.

Consequently this result does not establish whether the plugin built, loaded,
or reached an inference assertion. The elapsed time does not prove a timeout.
It is an opaque application failure, not evidence of a statistical failure.
Rust test results, actual toolchain/Stata versions, PE import/export checks,
Windows q0/q1 results and candidate binary collection remain unverified.
No expected seed or tolerance is represented as observed execution evidence.

The source driver would build the static-CRT Windows plugin, validate PE32+
x86-64, install into fresh PLUS, and run lifecycle and match-component tests
before its authoritative PASS marker. The match suite includes the current
2,048-Gram-probe assertion. The driver does not cover the observation and
individual-component suites or Windows Rust unit tests. Even a successful
run of that driver would therefore have remained bounded smoke evidence.

The accepted wrapper collects only its generic JSON receipt. It cannot return
the DLL or detailed sanitized project evidence needed for the five-binary
private payload or diagnose this failure. The reviewable
[bounded artifact-collection proposal](WINDOWS_ARTIFACT_COLLECTION_PROPOSAL_2026-09-08.md)
adds fixed artifacts and failure-stage enums while preserving source hashes,
private transfer, locking, cleanup and stopping. Its offline prototype passes
20 tests. It is not deployed; shared-controller maintenance authorization and
its required acceptance checks remain separate from ordinary qualification.

Next: enable the approved bounded collector through deliberate trusted
controller maintenance, diagnose this preserved failure, make at most a narrow
harness repair if warranted, and run one bounded retest before broader Windows
qualification. Then collect the successful exact binary, assemble the full
private archive, and validate that final archive in empty PLUS on Windows.
No opaque repeat, public publication, release, tag or estimator change is
justified by this receipt.

## Approved collector follow-up

The owner subsequently approved the bounded Windows collector extension.
The accepted baseline controller was recovered exactly and the offline
controller/wrapper/helper candidate, source inventory, fixed maintenance
payload and supplemental project harness are preserved under
`.local/diagnostics/windows-extension-20260908/`, with a complete hash manifest.
Ninety helper regressions and the two existing Windows-harness tests pass;
both shell scripts pass syntax checks. The frozen diagnostic inventory confirms
2,314 baseline files unchanged; the release build command and production
implementation are unchanged. This is offline diagnostic preparation only.

Deployment is blocked by authentication: the existing owner-attended
maintenance profile returned `AUTH_REQUIRED`, and the owner browser login
expired with exit 255. The approval remains valid; no new maintenance approval
is needed. No controller, skill installation, machine start or privilege
change occurred during this follow-up. The next step is successful
`aws login --profile windows-ci`, then exact candidate review, deployment and
required remote safeguard reacceptance. Routine tests retain the restricted
agent identity. Final-archive installation mode is not yet implemented or
accepted and remains a subsequent required gate.
