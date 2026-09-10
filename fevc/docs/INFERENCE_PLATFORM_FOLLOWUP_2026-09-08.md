# Inference qualification follow-up, 2026-09-08

Owner requested the two remaining published-text citation checks, complete
private-payload validation, SCC Linux qualification with Telegram watch, and
private Windows qualification. Tested package source is
`f3098bc1369992fccbc1d276aac5fc65ceb3f404`; follow-up evidence does not change
that identity or the approximate-inference scientific scope.

## Citation checks

The companion paper's `PUBLISHED_TEXT_CHECKS_2026-09-08.md` records both
checks complete. Abowd, Kramarz, and Margolis (1999) claim 117 is verified
against published Econometrica pages 251, 252, and 290. Andrews et al. (2008)
claim 123 is verified against the owner-supplied published PDF, printed pages
674 and 679–682, with balanced two-period mobility assumptions and covariance
sign qualifications preserved. Literature and paper validation pass; all 17
updated literature-report pages were visually inspected. No library sign-in
remains needed.

## Private payload

[Private-payload checkpoint](PRIVATE_PAYLOAD_VALIDATION_2026-09-08.md)
records passing Mac source/binary compatibility, packaging regressions,
portable artifact/license gates, and a fresh supply-chain audit. The full
five-binary archive is still incomplete. The three Mac binaries and the exact-source Linux binary are prepared. It
still needs the Windows binary and final-archive installation tests on all
supported platforms. The deterministic exact-source export and archive/install
validator are prepared; 19 focused tests pass, including deliberate failures.

## SCC Linux submission

- Run: `20260908T160000Z-inference-f3098bc`.
- Job: `7493136`, scalar, four slots, 4G memory per core, two-hour limit.
- Remote root: `/projectnb/welfgr/vckss/runs/20260908T160000Z-inference-f3098bc`.
- Source archive SHA-256:
  `af7ec8588f837639aa07c736f230fbf37cd380f8c47453d6dd1ab2b91b0f3287`.
- Established `deploy_linux_bundle.sh` and `submit_linux_qualifier.sh`
  entrypoints were used; exact archive/source hashes passed before submission.
- Final state: **PASS**. Scalar qacct reports `failed=0`, `exit_status=0`,
  547 wall seconds and 8.287G maximum virtual memory. Source/binary identities,
  all 20 collected remote file hashes, qualifier/wrapper receipts and exact
  observation, match, individual-inference, full-suite and installation
  output markers were verified. The tested binary SHA-256 is
  `e731932f5d85c9fb16b7d260aff7934d3445142312adf0ca04003a1a0b2e98f1`.
- [Machine-readable Linux result](inference_linux_qualification_20260908_result.json)
  binds durable receipts, source manifest, accounting and sanitized logs under
  `rust/qualification/evidence/INFERENCE-LINUX-SCC/f3098bc1369992fccbc1d276aac5fc65ceb3f404/`.
  The binary remains in ignored local artifact storage.
- This qualifies the source-local Linux build, including V4/direct-Gram
  inference, not the future complete five-binary archive installation.
- Collection printed directory-permission metadata warnings from rsync;
  every file subsequently matched its remote SHA-256. The first independent
  marker check used the wrong spelling for the individual test's success
  marker; using its actual `FEVC INDIVIDUAL INFERENCE PASS` output completes
  verification without any source/test changes.
- The watch service was inspected (no active watches). This job completed
  during parallel work, before a watch handoff was needed. No active watcher
  was registered for an already completed and validated run.

The local submission receipt is
`.local/diagnostics/platform-followup-20260908/linux-submission.json`.
Collect only the qualifier's sanitized artifacts, receipts and accounting;
retain the binary in ignored local storage. Telegram accounting completion
alone does not establish software qualification.

## Windows boundary

The exact-source existing `stata-do` smoke **failed** with
`STATA_DRIVER_FAILED`. The receipt does not distinguish compilation, loading,
assertion or timeout; no cause is inferred and no opaque retry was performed.
The wrapper confirmed instance stopped, transient objects deleted and lock
released. [Windows report](WINDOWS_QUALIFICATION_2026-09-08.md) binds the exact
run and sanitized failure evidence.
Full Windows route testing and tested-binary collection remain separate from
that bounded lifecycle/match smoke.

[The bounded collection proposal](WINDOWS_ARTIFACT_COLLECTION_PROPOSAL_2026-09-08.md)
is authorized by the owner's subsequent instruction. Its collector candidate, helpers, tests and project harness are preserved in
ignored `.local/diagnostics/windows-extension-20260908/`; the independent
artifact/PE regression run passes 90 tests. Final-archive Windows installation
mode remains subsequent work. Deployment requires authentication to the existing
maintenance profile; the attempted AWS sign-in timed out (exit 255). This is an
authentication blocker, not an outstanding request for maintenance approval.
No controller deployment or Windows qualification success is claimed.

The repository source checks pass: `./.venv/bin/python -m pytest -q`
(798 passed in 73.55 seconds) and
`./.venv/bin/python fevc/cmg/tools/assemble.py --all --check` (exit 0).
Pytest emitted cleanup warnings for older sandbox-protected temporary test
folders after its passing result; those warnings are not failed tests.

No follow-up work has been committed, pushed, published or tagged.

## Remaining user action

Authenticate the existing `windows-ci` maintenance profile so the approved
controller extension can be deployed and reaccepted. The subsequent Windows
qualification and complete private-archive installation checks remain pending.

## Subsequent owner instruction

The owner supplied `Andrews-HighWageWorkers-2008.pdf` and explicitly approved
using the proposed Windows extension. Andrews claim 123 is now verified
against the published article (printed pages 674 and 679–682; PDF SHA-256
`3388c06ffe011d81bc3b9292b8cf72ff5dea131a114841506e04cfe0abc9fc05`).
Both published-text checks are complete; no library sign-in remains needed.
The companion literature and paper validation pass, and all 17 updated report
pages were visually inspected. Earlier library-login and maintenance-approval requests are superseded by
this instruction.
The bounded Windows collector maintenance is authorized and in progress;
its deployment/reacceptance and resulting qualification are not yet claimed.
