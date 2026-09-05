# RC1 platform and packaging checkpoint

Candidate source: `4d5470f870f350121da7a1c8bf1a625e66e04a4c`, main,
clean at each build. This records engineering qualification, not a public
release or new statistical confirmation.

## Completed

- Public version `0.5.0-rc.1` is synchronized across Ado/Mata, both native
  result posters, help, catalog, Python metadata and tests. Stale native
  poster versions found by the first Stata run were corrected before the
  candidate was frozen; all public version posts now have a regression gate.
- The complete-payload builder requires all five Mac arm64/x86_64/universal,
  Linux x86_64 and Windows x86_64 plugins, exact source and evidence bindings,
  passing statuses and binary/evidence hashes. It rejects partial inventories,
  duplicate rows, wrong sources, unsafe paths and symlinks. It adds binary
  records to the archived install manifest while leaving the tracked portable
  manifest suitable for source-only CI. No complete native artifact is staged.
- `./.venv/bin/python fevc/tools/run_checks.py`: PASS, including 726 Python
  tests, 28 CMG Python checks, deterministic CMG assembly, public identity,
  historical-evidence, license and parity audits, Stata quick/full suites,
  isolated installation, installed help and bounded benchmark/preparation
  smokes. Python 3.13.0, pytest 7.4.4, Stata/MP 19 (19.0.115).
  Final focused checks after documentation edits: 50 PASS.
- Pinned Rust 1.85.1 standalone backend tests: 13 PASS; formatting and strict
  Clippy PASS. The Windows C build now selects `/MT` or `/MD` consistently
  with Rust's `crt-static` feature; its focused regression covers both choices.
- Exact-source `./ci/run_ci_profile.sh plugin-build`: PASS, 272.215 seconds.
  Native arm64, Rosetta x86_64, thin/universal binary audits, C ABI/error/
  interruption checks, public routes, match q0/q1 and isolated installation
  all pass. The generated common CI receipt validates against the exact SHA.
- SCC Linux job `7468587`: PASS on four cores, `failed=0`, `exit_status=0`,
  343 seconds, peak virtual memory 8.872 GiB within the 16 GiB reservation.
  The immutable source bundle hash is
  `374698ff800d5a74c74b6f4000c069aee6aa388dff3213f4ee295ae00fc4bf6f`.
  Pinned Rust/C/ELF/dependency/export gates, Stata/MP 19 full suite and clean
  installation pass. Both full-suite and clean-install logs contain the actual
  `PASS test_rust_match_component_inference.do` marker. All 20 collected
  receipt/evidence files rehash against SCC; directory-attribute transfer
  warnings did not change file bytes.
- Current supply-chain gate: PASS using cargo-audit 0.22.2 and cargo-cyclonedx
  0.5.9, security toolchain 1.97.1, RustSec snapshot
  `5a0ebedfe8bdd2e295b171f4162f8c977bcad9a5` (1,239 advisories).
  All three lockfiles have zero vulnerabilities and zero audit warnings.
  Four normalized CycloneDX 1.5 SBOMs contain 53 component records. The
  existing reviewed `version_check` license-spelling bridge is retained;
  its generator warning is not a newly waived dependency or license issue.
  The initial audit invocation rejected the machine's older 0.22.1 tool;
  the required pinned version was installed in ignored project tooling.

## Tested binary identities

| Artifact | SHA-256 |
| --- | --- |
| macOS arm64 | `c2ea3758253d6c74719b43f8bc8e2e0232699061fe89247679ffb12b2c83e1d9` |
| macOS x86_64 | `bc29b4ffdee2a7508284292d412a1da35cfe738843d08b1c1f2cb1c0e578d9ad` |
| macOS universal | `d9fa4a90c23fbc9eb53c594f24bc8553af58486720edab911b3c933c831efa69` |
| Linux x86_64 | `62459714ff9f53f51d2866a43936f7502d58d282c607f271b2b0386ce69719ba` |

Compact original receipts and SBOMs are preserved under
`rust/qualification/evidence/RC1-PLATFORMS/4d5470f870f350121da7a1c8bf1a625e66e04a4c/`.
Tested binaries and detailed sanitized logs remain in ignored
`.local/diagnostics/rc-platform-preflight/`; they are not committed binaries.

## Windows and final-artifact boundary

The accepted Windows inspection passed with the machine stopped. The first
sandboxed run stopped before starting the machine because the credential
helper could not access Keychain. Approved inspection with normal Keychain
access passed again; the same restricted identity and wrapper were used for
the private `stata-do` project smoke. Run `win-20260905T134429Z-8bc9fb43`
returned **FAIL**, code `STATA_DRIVER_FAILED`. Source archive SHA-256 is
`a5f819b1a6f93728899c49d70bdcbdc17b4ebce4f7174e8016f0365eaadfb9b4`;
receipt SHA-256 is
`d1ddd889ae39f3fe75732e12a4c56a6b054b920df246b97394fc1470b020ca16`.
Execution identity, source hash, archive paths, source manifest and shutdown
backstop checks pass; Stata/project-test checks do not. The receipt contains
no compiler diagnostic, Stata error, artifact hash or detailed failure stage.
It therefore does not establish that the plugin built, loaded or reached an
inference assertion. No Windows qualification or statistical failure is inferred.
The wrapper confirms `INSTANCE_STOPPED=yes`, `TRANSIENT_OBJECTS_DELETED=yes`
and `LOCK_RELEASED=yes`; no Windows task remains active.

The approved runner returns a source-bound application receipt, but not the
compiled binary or detailed sanitized project evidence. The owner has been
asked to approve bounded artifact collection through that existing private
channel. No unrestricted cloud command or alternate identity is used.

The next action is the separately approved runner collection extension,
followed by one bounded Windows diagnostic/retest. Do not repeat the same
opaque driver run, bypass the restricted runner, or change inference formulas
based on this receipt. Failure details must exclude startup/license banners.

Remaining gates: diagnose and complete Windows smoke, broader Windows route and PE
import/export checks, collect the tested Windows binary, audit corresponding
source/notices for the actual distribution, assemble the complete archive,
and install **that final archive** into empty PLUS directories on every
platform. Platform-local temporary install tests above are not a substitute
for this final assembled-artifact check. Public release/tagging still need
separate owner authorization.

## Scientific compatibility and limitations

`rust/crates/` is byte-for-byte unchanged from the public match interface
source `53f22a109effee87467b4ef0602b21d0b8ec1ca9`. The estimator, ABI,
scientific inputs, dependency locks, RNG contracts and acceptance thresholds
are unchanged by this RC preparation. Ado/Mata changes are version metadata;
the native build change affects only Windows CRT selection. Earlier exact
scientific records retain their original source identity and status.

Independent match q0 and eligible one-mode q1 confirmations remain PASS in
their declared regimes. The fixed-offset approximation still ignores
nuisance-control estimation uncertainty. Observation q1 retains its corrected
confirmation FAIL and calibration warning. No new coverage, scale, native
Intel hardware, Windows specialized full-CMG auto-route or public-release
claim is made. No Monte Carlo campaign was rerun and no cutoff was waived.
