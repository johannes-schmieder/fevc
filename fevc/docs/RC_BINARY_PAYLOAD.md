# Complete RC binary payload

The owner's 2026-09-05 request selects `0.5.0-rc.1` with all five plugin
files: macOS arm64, macOS x86_64, macOS universal, Linux x86_64 and Windows
x86_64. This authorizes local candidate preparation and private platform
tests, not publication or tagging. No current platform success is implied
by this preparation document.

## Build and acceptance sequence

1. Freeze a clean source on main after local source, build-boundary and Stata
   checks. Keep statistical code, scientific inputs, thresholds and defaults
   unchanged; retain the observation-q1 calibration warning.
2. Qualify macOS using `ci/run_ci_profile.sh plugin-build` and Linux using
   the existing four-core `rust/stata_backend/scc/deploy_linux_bundle.sh`
   and `submit_linux_qualifier.sh` entrypoints. Their lifecycle smokes precede
   the broader public and isolated-install gates. Linux acceptance requires
   scheduler `failed=0`, `exit_status=0`, all explicit application markers,
   and source/binary hashes. No scaling or Monte Carlo campaign is requested.
3. Run the private Windows skill's accepted `stata-do` profile against
   repository-root `windows-ci.do`. The first bounded gate builds with pinned
   Rust 1.85.1, authenticated SPI and static MSVC CRT, checks x86-64 PE format,
   then loads the plugin for the first time from an isolated PLUS installation.
   It exercises lifecycle and the public fixed-offset match q0/q1 regression.
   A later full Windows claim needs broader route tests and binary import/export
   audit. The Mac/Linux specialized full-CMG auto route remains out of scope
   on Windows; generic diagonal/CMG match inference is the intended RC route.
4. Collect tested bytes and sanitized evidence. The currently approved Windows
   runner returns only a receipt, so tested-binary/evidence collection requires
   a separately authorized bounded runner extension. Never bypass that runner,
   upload license material or collect raw Stata startup logs.
5. Create an input manifest with schema `FEVC-BINARY-INPUTS-V1`, the exact
   `source_commit`, and five `binaries` rows. Each row names `name`, `sha256`,
   `source_commit`, `status: PASS`, a relative `evidence` file and its
   `evidence_sha256`. Review those receipts against the actual test results;
   the packaging tool validates bindings, not the scientific truth of a PASS.
6. Run `./.venv/bin/python fevc/tools/build_native_release.py --binary-dir DIR
   --manifest MANIFEST --check`, then `--output-dir NEW_DIRECTORY` in place
   of `--check`. It rejects missing platforms, duplicate entries, wrong-source
   binaries, hash mismatches, unsafe paths and symlinks. It never publishes.
7. Freeze and audit a corresponding-source archive, dependency/SBOM records
   and actual notices alongside the binary artifact. Install the **final
   archive bytes** into empty PLUS directories on each platform and exercise
   q0/q1 and help; separately retain the portable-only missing-native test.
   Obtain the owner's exact-artifact approval before distribution.

The tracked `fevc/fevc.pkg` remains a portable development/source manifest.
The native builder generates the complete manifest only after validating all
five binary inputs. The public installation will use root `fevc.pkg` and
`stata.toc` files that reference the runtime and binaries under `fevc/`.
Both `net install` and `github install` must deliver that same payload.
Generated native manifests use `F` entries for the license and notices so
Stata installs them with the runtime instead of treating them as ancillary files.

When a later packaging or documentation commit leaves the tested native
source unchanged, retain each binary's original `source_commit`. An explicit
compatibility record may bind it to the new package source under the registered
acceptance policy. The binary row must provide `compatibility_evidence` and
`compatibility_sha256`. The referenced JSON uses schema
`FEVC-BINARY-COMPATIBILITY-V1`, records `build_source_commit`,
`package_source_commit`, `status: PASS`, a `binaries` name-to-SHA-256 mapping,
`unchanged_source_manifest_sha256`, `changed_paths`, `checks`, and `limitations`.
Verify the unchanged production, build, input, and acceptance identities before
writing that record. The packager verifies its bindings; it does not establish
the truth of its qualification claims. Final installation checks still apply
to the new package bytes.

To prepare the repository layout in a new directory, use `--repository-dir`
instead of `--output-dir`:

```bash
./.venv/bin/python fevc/tools/build_native_release.py \
    --binary-dir DIR --manifest MANIFEST \
    --repository-dir /private/tmp/fevc-install-repository
```

This requires a clean committed source and the same complete, source-bound
binary evidence as the archive builder. It writes root installation metadata,
the `fevc/` payload, and a hash receipt, and refuses an existing destination.
It does not copy anything into the live checkout or publish it. Once qualified
and approved, the installation metadata and tested binaries can be committed
together with their matching source. Release attachments alone do not supply
the raw GitHub installation endpoint.

The upstream `github` installer inspected during preparation uses `master`
URLs. Verify the exact advertised command against the publication layout and
provide an equivalent compatibility ref if needed. No such ref is created by
the builder. Verify clean installs and upgrades using both commands on the
supported platforms, including actual native execution and binary hashes.

The final receipt binds the package source, native inputs, archive digest and
every installed file. A later documentation/evidence commit must not be
reported as the tested binary source. Preserve all accepted earlier records.
