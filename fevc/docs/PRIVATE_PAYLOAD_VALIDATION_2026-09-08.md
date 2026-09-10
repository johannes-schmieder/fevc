# Private payload validation checkpoint, 2026-09-08

Candidate source: `f3098bc1369992fccbc1d276aac5fc65ceb3f404` on main.
**Incomplete: the complete private native payload cannot yet be validated.**
This work applies the five-binary installation contract in
[RC_BINARY_PAYLOAD.md](RC_BINARY_PAYLOAD.md); it does not concern restricted
row-level data. The owner requested this work on 2026-09-08.

## Completed gates

- macOS compatibility review: all 150 source/build/ABI/runtime/test/package
  files in the successful morning qualifier's source manifest match the
  committed candidate exactly. The three thin/universal binaries also match
  the original tested hashes. The acceptance policy is unchanged. This
  explicitly carries the recorded Mac route and isolated-install evidence
  forward; it is not a fresh clean-commit Mac build. The historical receipt
  remains classified `LOCAL_CHECKPOINT_DIRTY_TREE` against base commit
  `1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`. The actual tested manifest,
  rather than that older clean base tree, is the relevant identity.
- Native/portable packaging regressions: **15 passed in 1.00 seconds**, using
  `./.venv/bin/python -m pytest -q fevc/tests/python/test_native_release.py
  fevc/tests/python/test_release_artifact.py`. Tests cover malformed inventories,
  source/hash mismatches and unsafe paths as well as deterministic packaging.
- `./.venv/bin/python fevc/tools/build_release_artifact.py --check`: **PASS**,
  41 portable files, SHA-256
  `df7d82948d9ed4894b5da1d452617a58ea9ffc32cf6f822c7017a14f6b5a4d3a`.
- `./.venv/bin/python fevc/tools/license_audit.py`: **PASS**. This is the
  automated source notice check, not the final human approval of an artifact.
- Fresh exact-source supply-chain gate: **PASS**, cargo-audit 0.22.2,
  cargo-cyclonedx 0.5.9, security toolchain 1.97.1. RustSec commit
  `bf25f6575a93a35f30796c65c0ed91bee7fa19fd` contains 1,242 advisories.
  All three lockfiles have zero vulnerabilities and zero audit warnings;
  four CycloneDX 1.5 SBOMs contain 53 component records. The existing reviewed
  `MIT/Apache-2.0` SPDX spelling equivalence remains recorded.

The established supply-chain invocation used repository-local pinned tools:

```bash
VCKSS_CARGO_AUDIT="$PWD/.local/tools/cargo-audit-0.22.2/bin/cargo-audit" \
VCKSS_CARGO_CYCLONEDX="$PWD/.local/tools/cyclonedx-0.5.9/bin/cargo-cyclonedx" \
rust/tools/run_supply_chain_checks.sh \
  --receipt "$PWD/.local/diagnostics/private-payload-20260908/supply-chain.txt" \
  --sbom-dir "$PWD/.local/diagnostics/private-payload-20260908/sboms"
```

The initial sandboxed fetch failed DNS resolution before obtaining advisory
information. The same audit passed with approved network access. Pytest also
reported teardown permission warnings for unrelated pre-existing garbage
folders; all 15 selected tests passed.

## Remaining dependency and final-artifact gates

Linux qualification runs separately in this task. The accepted Windows
wrapper currently retrieves only its generic receipt and deletes transient
remote objects; it cannot return the tested plugin or detailed build evidence.
Its extension is a separate shared-infrastructure decision. No substitute
binary, missing-platform PASS, raw AWS command, or relaxed gate is used here.

After source-bound Linux and Windows binaries and their reviewed passing
receipts are available, assemble exactly five `FEVC-BINARY-INPUTS-V1` rows.
Each must bind the exact candidate commit, binary hash, relative evidence
path and evidence hash. The prepared record deliberately has a different
schema and lists missing inputs; it is not a valid complete build manifest.
The native builder validates those bindings, not the truth of the underlying
qualification. The explicit Mac compatibility record is the reviewed evidence
for its three rows; the historical receipt must not be relabeled.

Then, from a clean candidate checkout, run:

```bash
./.venv/bin/python fevc/tools/build_native_release.py \
  --binary-dir DIR --manifest MANIFEST --check
```

The established next private-artifact step replaces `--check` with
`--output-dir NEW_DIRECTORY`. Freeze the corresponding-source archive and
actual notices/dependency records, audit every archive member/hash, and install
those final native archive bytes into empty PLUS directories on all three
platforms. Exercise installed help and q0/q1, and retain the separate
portable-only missing-native gate. The existing Mac test entrypoint is
`fevc/tests/stata/test_rust_public_install.do` with archive-extracted package,
empty PLUS root, `qualified`, and source route-test directory arguments;
run both arm64 and Rosetta. Linux and Windows require their authorized
execution channels and the same final artifact identity.

No complete native archive was staged here. No final-archive installation,
Windows full qualification, distribution, tagging, human review or coverage
claim is made. Do not replace this incomplete result with PASS based only on
successful source-local platform smoke tests.

## Evidence

[Machine-readable result](private_payload_validation_20260908_result.json)
binds all retained sanitized evidence under
[`rust/qualification/evidence/INFERENCE-PRIVATE-PAYLOAD/f3098bc1369992fccbc1d276aac5fc65ceb3f404/`](../../rust/qualification/evidence/INFERENCE-PRIVATE-PAYLOAD/f3098bc1369992fccbc1d276aac5fc65ceb3f404/).
That packet includes the full compatibility review, original Mac receipt and
manifest, fresh supply-chain receipt, four SBOMs, license and portable checks,
and the incomplete input preparation record. Tested Mac binaries remain in
ignored `.local/diagnostics/private-payload-20260908/binary-inputs/`.
