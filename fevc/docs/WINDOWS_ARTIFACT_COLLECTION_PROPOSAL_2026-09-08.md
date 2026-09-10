# Bounded Windows artifact collection proposal

Status: **OWNER APPROVED; OFFLINE CANDIDATE; NOT DEPLOYED**. Prepared for the owner's request
on 2026-09-08 to qualify the new inference candidate and validate the complete
private payload. Production source is
`f3098bc1369992fccbc1d276aac5fc65ceb3f404`.

## Present boundary and blocking gap

The accepted local wrapper is
`/Users/johannes/.codex/skills/windows/scripts/windows_ci.sh`.
It transfers exact source, runs the fixed machine-local controller through the
restricted custom document, retrieves only `receipt.json`, and deletes the run
objects and stops Windows. Its current receipt confirms source and overall
Stata/project status but does not return the compiled plugin, its build JSON,
or per-test evidence. Therefore a wrapper PASS alone cannot satisfy the
complete five-binary private payload contract or full Windows qualification.

The source's `windows-ci.do` builds and checks the PE32+ candidate, clean-installs
the source package, and runs plugin lifecycle and match-component inference.
It does not run the observation and individual-component suites or Windows
Rust workspace tests. The registered direct-Gram default is covered by the
match test's 2,048-probe assertion; that does not replace the missing routes.

## Exact proposed change

1. Extend the fixed Windows controller's **fevc / stata-do** result handling
   with exactly three allowed result basenames:
   `fevc_rust_windows_x64.plugin` (maximum 64 MiB), `windows-build.json`
   (4 KiB), and `windows-project-checks.json` (4 KiB). Other project/profile
   behavior remains unchanged. Do not accept arbitrary output paths, globs,
   archives, log names, object keys, or commands from project source.
2. Collect successful artifacts only from the current source directory after the authoritative
   Stata status marker and project assertions pass. Reject reparse points,
   symlinks, missing files, extra manifest entries, invalid lengths, and path
   escapes. Recompute the binary SHA-256 and PE architecture. Reconstruct
   both JSON objects from a fixed field schema; never forward arbitrary JSON
   fields, free text, logs, machine identifiers, or license banners. On failure,
   return only a bounded failure-stage enum (for example build, clean_install,
   match_component, timeout, unknown) with source identity; never infer which
   stage ran from absent diagnostics. This preserves useful failure evidence
   without returning binaries from failed runs or raw compiler/Stata logs.
3. Bind the three fixed names, byte lengths, and SHA-256 values to the
   controller receipt's run ID, exact source revision, source archive hash,
   fixed target and profile. Store the artifacts only beneath the existing
   encrypted run-results prefix; do not grant any new permissions.
4. Extend the local wrapper to retrieve only these literal basenames after
   receipt identity validation and before existing cleanup. Enforce the size
   cap before writing, verify each file's size/hash/schema, independently
   validate PE32+ x86-64, and reconcile the binary hash with `windows-build.json`.
   Fail closed on any mismatch; still stop the machine and clean up. Record
   local file and receipt hashes only after complete collection.
5. Extend the project harness with locked Windows workspace/backend Rust
   tests, required PE import/export auditing, and the existing
   `test_rust_component_inference.do`, `test_rust_individual_inference.do`,
   and `test_rust_match_component_inference.do` suites.
   Record each explicit assertion/PASS gate in the bounded JSON. Keep the
   existing exact source smoke separate from any supplemental harness source
   identity. Do not change estimator or acceptance logic.
6. For the final assembled archive, add a fixed project-only input name and
   SHA-256 declaration to a separate source-bound harness, install its contents
   into empty PLUS, and run the registered installed q0/q1/help checks against
   those exact archived bytes. The source ZIP safety checks and maximum archive
   size remain in force. No public release or tag follows from these runs.

The exact source location of the machine-local controller and its deploy
mechanism are not available from the accepted wrapper. Its read-only inspector
has no maintenance option. Deployment must first recover and verify the
accepted controller source through owner-authorized maintenance; it must not
use a project Stata driver to self-modify the trusted SYSTEM controller or
switch to unrestricted remote PowerShell as a routine workaround.

## Reviewable local prototype and checks

The offline prototype is under
`.local/diagnostics/windows-inference-20260908/extension-proposal/`:
`validate_artifacts.py` and `test_validate_artifacts.py`. It performs no network
I/O and changes no machine configuration. It is a validation design, not a
replacement for the guarded wrapper or a claim of Windows success.

Command:

```text
./.venv/bin/python -m pytest -q .local/diagnostics/windows-inference-20260908/extension-proposal/test_validate_artifacts.py
```

Result: 20 tests passed. They cover valid fixed artifacts; source/archive
mismatch; failed runs; missing, duplicate and path-traversing entries; unknown
fields; oversized manifests; missing and symlinked files; truncation and hash
corruption; wrong PE architecture; failed gates; and unexpected free-text
receipt content; and valid/source-bound bounded failure diagnostics with
unknown fields and arbitrary stage strings rejected. An initial test helper accidentally used pytest's reserved
`setup` name; renaming it fixed the local test harness. Pytest also emitted
cleanup warnings about unrelated older temporary fixtures; those are not
artifact-validation failures.

These tests do not prove remote controller behavior, IAM scope, bounded
streaming downloads, malicious reparse races, MSVC imports/exports, or final
payload acceptance. The deployed implementation must retain those existing
or specifically required checks and receive independent review.

## Required maintenance authorization and acceptance

The Windows skill's `references/unattended-setup.md` explicitly says:
"These are infrastructure mutations and require explicit authorization."
It also requires reacceptance after any controller or wrapper change. The
owner's qualification request authorizes ordinary source transfer, tests,
collection, cleanup and stopping. Changing the shared trusted controller is
an additional maintenance action and remains unperformed.

The proposed authorization is limited to this fixed artifact-collection
extension and its reacceptance. It excludes authentication, IAM, network,
instance type, licensed software, general remote commands and safeguards.
If the existing permission or deployment surface cannot support it, stop and
report that concrete requirement before changing it.

Before using the extended backend for qualification, verify authentication
and out-of-scope denial, one clean smoke, one deliberate failure, rejection of
a concurrent request, exact source/artifact hashes, transient-object deletion,
lock release, and final stopped state. Test allowed artifact collection and
deliberate missing/corrupted outputs as part of those bounded runs. Update the
central AWS task only after actual shared-backend changes and acceptance.

## Subsequent owner approval and offline checkpoint

The owner approved this maintenance extension. Approval is no longer the
blocker. The existing attended maintenance login expired; successful owner
authentication is required before deployment. Reviewed-baseline recovery,
bounded candidate code and 90 helper tests are preserved with hashes under
`.local/diagnostics/windows-extension-20260908/`. The two project harness
tests also pass. See `README.md` there for exact identities and remaining
remote reacceptance. Neither shared controller nor local skill has changed.
Final-archive runner mode remains explicitly unimplemented/unaccepted.
