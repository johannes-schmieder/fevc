# Native binary payload

## Current Mean-component status and provenance

The [Mean-component manifest](../../native/mean-component-20261008/manifest.json)
records the five qualified payloads adopted locally, exact hashes, build
sources, source compatibility and installation evidence. Local installation
checks pass at `240ad74d`; publication and final public checks of the updated
package bytes remain pending. The
[active plan](../PLAN.md) records the completed gates and remaining work,
including the later owner-authorized Windows repair continuation. Preserve
all earlier failed attempts and their original receipts.

The Mean refresh retains the original Mac candidates built at `b9f80ce9` and
Linux candidate built at `63757839`, with their corresponding-source archives.
The Windows candidate is built at `240ad74d`, which repairs serial exact
Corrected accounting. The retained Mac/Linux binaries do not contain that
lower-level repair. Their reuse is limited to unchanged public routes under
an explicit affected-route compatibility review; it does not establish that
all native source is unchanged or that all five payloads contain the repair.
The manifest binds each actual build source, tested route and final package
identity separately.

Platform qualification is an engineering claim. The
[bounded assessment](MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md) retains
22 exact-Mata availability-screen failures and material numerical sensitivity;
there is no general coverage guarantee. Earlier statistical limitations remain.
For this task, native packaging uses `--check` and `--repository-dir`; a native
release archive, tag or hosted release remains outside the authorized scope.

The dated sections below preserve their original source and approval cutoffs.
Their pending approvals and then-current artifacts are historical; they do
not replace the current plan or exact-artifact manifest.

## Historical distribution request — September 5, 2026

The owner's 2026-09-05 request selects `0.5.0-rc.1` with all five plugin
files: macOS arm64, macOS x86_64, macOS universal, Linux x86_64 and Windows
x86_64. This authorizes local candidate preparation and private platform
tests, not publication or tagging. No current platform success is implied
by this preparation document.

## Historical October 8 point/Mean-projection candidate

Clean build source `24754269` supplied the then-current candidate. Mac arm64,
Rosetta x86-64 and universal plugins pass full clean qualification plus
24 isolated-install capability, point-centering and Mean-projection checks.
They expose centering API 1 and projection-centering API 1.
Linux x86-64 also passes full qualification and installed point-centering
and Mean-projection checks at the same source (SCC job `7962808`).
The [compact candidate manifest](../../native/prerelease-20261008/manifest.json)
records the four qualified Mac/Linux files and the separate pending Windows
status; it must not represent an incomplete set as a qualified `complete`
profile. Native Intel hardware is not claimed.

Windows smoke `win-20261008T153726Z-59a2355d` fails with the aggregate
`STATA_DRIVER_FAILED` status. No failing assertion is available from the
accepted collector. The proposal for bounded diagnostics and validated
artifact collection is reviewed locally but still awaits owner approval;
no shared update is deployed. The existing Windows manual-test bytes retain
their original status. No new publication, tag or release is authorized.

## October 7 Mean-projection development checkpoint

Mean projection requires additive `projection_centering_api=1`. The rebuilt
local Mac development candidates expose it and pass arm64/Rosetta separate/universal
runtime and isolated-install checks. They are preserved outside the package
while the tracked payload retains its previously adopted bytes; no new
distributed-payload adoption is recorded. The qualifier completed with
`LOCAL_CHECKPOINT_DIRTY_TREE`, so its outer CI clean-checkout gate remains
failed. Linux, Windows and native Intel hardware were not requalified for this
extension. The earlier adoption records below retain their original bytes
and scope. See [CENTERING.md](CENTERING.md) and the [checkpoint](../PLAN.md).

## October 6 centering refresh

The owner requested updated plugins for the smaller None/Mean/Corrected
implementation. Local macOS and private Linux/Windows candidate preparation
and tests are in scope. The October 7 instruction authorizes committing and
pushing the reviewed implementation, documentation and qualified payloads,
providing the source for the hosted Windows build, which passes after a
behavior-preserving C identifier rename. Full private qualification and one
focused centering diagnostic fail. The owner then explicitly requests adoption
of the exact Windows build for manual testing; runtime qualification remains
pending. No tag or release is authorized. The four local Mac/Linux payloads have now passed full
platform and explicit Rust centering checks and are adopted in the local
checkout and are now published. [The adoption record](../../native/centering-20261006/manifest.json)
binds tested bytes and source identity. The [Windows manual-test record](../../native/centering-windows-manual-20261007.json)
binds the newly adopted Windows bytes without claiming runtime qualification;
the [October 7 attempt record](../../native/centering-20261007/manifest.json)
preserves its failed attempts and cleanup.
Historical payload records below keep their original scope.

Active native centering requires additive centering API 1 independently of
numerical API 2. In addition to ordinary platform gates, run the four
`test_centering_mean.do`, `test_centering_exact.do`,
`test_centering_jla.do` and `test_centering_options.do` files with explicit
`rust` against each exact candidate/architecture. The ordinary Stata suite
invokes centering in Mata and does not establish native centering by itself.
The independent three-pool map unit test remains a separate Rust gate.
See [CENTERING.md](CENTERING.md) for methods and MCSE assumptions.

## Previously authorized distribution scope

On September 26 the owner authorized deletion-unit mover integration, all five
plugins, and source then package publication to `main`. Use the `complete`
profile for this update. Qualification is tracked in
[the integration record](DELETION_UNIT_MOVERS_2026-09-26.md); a tag or release
is still a separate decision.

The September 21 Mac/Linux-only checkpoint used `--profile macos-linux` and
`"profile": "macos-linux"` in its input manifest. This profile requires all
four Mac/Linux plugin files and rejects missing, duplicate, unexpected,
unqualified, or mismatched inputs. The default `complete` profile retains the
original five-binary requirement. Never fill a deferred platform with a
placeholder or call an untested build qualified.

The existing repository may be published with its retained historical GitHub
PR refs: the owner explicitly accepted those deleted reviews remaining there.

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
3. For the complete profile,
   run the private Windows skill's accepted `stata-do` profile against
   repository-root `windows-ci.do`, starting with the bounded smoke before
   the full profile. Both profiles require point and Mean-projection capability
   checks; the full profile preserves the earlier runtime assertions.
   A supplied exact hosted candidate retains
   its CI build/Rust-test provenance; the private driver audits x86-64 PE format
   and loads it from an isolated PLUS installation. The fallback build uses
   pinned Rust 1.85.1, authenticated SPI and static MSVC CRT.
   It exercises lifecycle and the public fixed-offset match q0/q1 regression.
   The September 26 driver also checks observation/individual inference, the
   deletion-unit oracle, canonical-header exports and system-only imports.
   The Mac/Linux specialized full-CMG auto route remains out of scope
   on Windows; generic diagonal/CMG match inference is the intended RC route.
4. Collect tested bytes and sanitized evidence. The approved Windows runner
   returns only a receipt. Build the Windows candidate in hosted CI, download
   it, and transfer those exact bytes with a hash-bound input manifest through
   the private runner. Its project receipt checks the installed candidate hash;
   this path does not require binary retrieval or a runner extension. A shared
   collector change requires separate owner approval and runner reacceptance.
   Record the transfer as a snapshot, distinct from the clean source identity.
   Never bypass the runner,
   upload license material or collect raw Stata startup logs.
5. Create an input manifest with schema `FEVC-BINARY-INPUTS-V1`, the exact
   `source_commit`, and the selected profile’s `binaries` rows. Each row names `name`, `sha256`,
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
   Current authorization covers local preparation and private tests. Publishing
   this candidate, a tag or a GitHub release requires a separate owner decision.

The tracked `fevc/fevc.pkg` remains a portable development/source manifest.
The native builder generates the complete manifest only after validating every binary in the selected profile. The public installation will use root `fevc.pkg` and
`stata.toc` files that reference the runtime and binaries under `fevc/`.
Both `net install` and `github install` must deliver that same payload.
Generated native manifests use `F` entries for the license and notices so
Stata installs them with the runtime instead of treating them as ancillary files.

For a packaging or documentation change that leaves the tested native
source unchanged, retain each binary's original `source_commit`. An explicit
compatibility record may bind it to the new package source under the registered
acceptance policy. That unchanged-source case does not describe the complete
Mean refresh: `240ad74d` changes serial exact Corrected accounting. Reuse of
the older Mac/Linux binaries requires the separately reviewed affected-route
compatibility and its explicit limitations described above. The binary row
must provide `compatibility_evidence` and
`compatibility_sha256`. The referenced JSON uses schema
`FEVC-BINARY-COMPATIBILITY-V1`, records `build_source_commit`,
`package_source_commit`, `status: PASS`, a `binaries` name-to-SHA-256 mapping,
`unchanged_source_manifest_sha256`, `changed_paths`, `checks`, and `limitations`.
For an unchanged-source compatibility claim, verify the unchanged production,
build, input and acceptance identities before writing that record. For the
current affected-route reuse, identify the changed source and excluded
lower-level route explicitly; a packaging field must not be presented as proof
that all native production source is unchanged. The packager verifies its
bindings; it does not establish the truth of its qualification claims. Final
installation checks still apply to the new package bytes.

To prepare the repository layout in a new directory, use `--repository-dir`
instead of `--output-dir`:

```bash
./.venv/bin/python fevc/tools/build_native_release.py \
    --binary-dir DIR --manifest MANIFEST --profile complete \
    --repository-dir /private/tmp/fevc-install-repository
```

This requires a clean committed source and the same profile-specific, source-bound
binary evidence as the archive builder. It writes root installation metadata,
the `fevc/` payload, and a hash receipt, and refuses an existing destination.
It does not copy anything into the live checkout or publish it. Once qualified
and approved, the installation metadata and tested binaries can be committed
together with their matching source. Release attachments alone do not supply
the raw GitHub installation endpoint.

Verify the exact advertised `github install` command against the publication
layout on `main`; a hardcoded URL in the upstream installer alone does not
establish that a separate branch is required. Verify clean installs and upgrades
using both commands on the supported platforms, including actual native
execution and binary hashes.

The final receipt binds the package source, native inputs, archive digest and
every installed file. A later documentation/evidence commit must not be
reported as the tested binary source. Preserve all accepted earlier records.

A complete native prerelease needs root `fevc.pkg` and `stata.toc`, all five
qualified `fevc/*.plugin` files, and a compact binary manifest linking exact
build sources and sanitized qualification receipts. Keep the nested portable
manifest unchanged. Publish package updates to `main` only after explicit owner
authorization. Verify actual HTTP installs after publication; building a
staging directory alone is not an installer verification.
