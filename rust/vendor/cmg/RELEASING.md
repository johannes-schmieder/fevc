# Releasing CMG

This document is the authoritative process for versioning, creating GitHub
Releases, and distributing CMG. It describes policy, not release history; the
chronological user-facing record belongs in [`CHANGELOG.md`](CHANGELOG.md).

CMG is currently a Rust crate. The repository does not yet contain a Stata
package and has not been published on SSC. The SSC sections below define the
required process once the planned Stata integration exists; they must not be
read as claiming that an SSC package is available today.

## Release channels

- `main` is the normal development branch. It answers “What are we developing
  now?” and may contain changes intended for the next version.
- `vX.Y.Z-rcN` tags and their GitHub prereleases are optional test snapshots.
  They are not stable releases and must never be submitted to SSC.
- A final `vX.Y.Z` tag and the matching GitHub Release are the immutable source
  snapshot of version `X.Y.Z`. Creating the final tag is an explicit release
  decision; an RC does not become final merely because no commits followed it.
- When a Stata package is available, SSC is the stable Stata distribution
  channel. Its files must come from a final release commit, never from the
  current tip of `main`.

Development normally continues on `main` immediately after a release. For
example, `main` may be work toward `0.3.0` while `v0.2.0` remains the frozen
stable source and SSC continues to distribute `0.2.0`.

## Version numbers and immutable tags

Use semantic-style versions: increment the patch number for compatible fixes,
the minor number for compatible features, and the major number for incompatible
API changes. Before `1.0.0`, a minor increment may include compatibility changes
appropriate to a pre-1.0 API. Tags have a `v` prefix, such as `v0.1.0`,
`v0.1.1`, and `v1.0.0`. Release candidates append `-rcN`, such as
`v0.2.0-rc1`.

Published final tags are immutable. Never force-move, rewrite, delete and
recreate, or otherwise replace one. If a released version is defective, fix the
problem on `main` and create a new patch release such as `v0.2.1`. Preserve the
same traceability if SSC review exposes a source problem: commit the correction
and release a new version rather than silently changing `v0.2.0`.

## Metadata that must agree

For every final release, synchronize:

- the `vX.Y.Z` tag and GitHub Release title;
- `package.version` in `Cargo.toml` and the root `cmg` entry in `Cargo.lock`;
- the dated `X.Y.Z` section in `CHANGELOG.md`;
- once Stata packaging exists, the primary `.ado` version/date header, the
  `.pkg` distribution date and any explicit package version;
- the version submitted to SSC.

Do not add a standalone `VERSION` file. `Cargo.toml` is the current source of
the package version. A conventional Stata header should look like
`*! cmg 0.2.0 14oct2026`, and `.pkg` metadata should contain
`d Distribution-Date: 20261014`. Use the same calendar date for the changelog,
the ado header, and the distribution date.

Run `python3 scripts/check_release_metadata.py` during development. On an exact
version tag, the checker also requires the tag, Cargo version, changelog entry,
and any detected Stata metadata to agree. CI runs the same check.

## Preparing and validating a release

1. Choose the intended version and update `Cargo.toml`; run `cargo check` or
   another Cargo command to update `Cargo.lock`.
2. Move the relevant entries from `Unreleased` into a dated `X.Y.Z` section in
   `CHANGELOG.md`. Leave a fresh `Unreleased` section at the top.
3. Update user documentation and, when present, all Stata metadata described
   above.
4. Run the metadata checker and the complete commands listed under “Build and
   test” in `README.md`. Review CI on the exact commit being considered.
5. If distribution artifacts or compiled Stata plugins exist, build them from
   that commit in CI and retain enough provenance to identify the source commit.

The release flow is:

```text
Development on main
        ↓
Update changelog, documentation, and version metadata
        ↓
Run complete tests
        ↓
Create and test vX.Y.Z-rc1 if an RC is warranted
        ↓
Fix issues on main; create another RC if needed
        ↓
Explicitly choose the final tested commit
        ↓
Create immutable vX.Y.Z tag and final GitHub Release
        ↓
Submit that exact Stata package to SSC, when applicable
        ↓
Continue development on main
```

## Release candidates

Create an RC only when external or packaging validation is useful. From the
prepared, tested commit:

```bash
VERSION=0.2.0
COMMIT=0123456789abcdef0123456789abcdef01234567
git tag -s "v${VERSION}-rc1" "$COMMIT"
git push origin "v${VERSION}-rc1"
gh release create "v${VERSION}-rc1" --prerelease --verify-tag \
    --notes-file path/to/release-notes.md
```

Use a signed tag when the maintainer's signing setup permits it; otherwise use
an annotated tag (`git tag -a`). Mark alpha, beta, and RC GitHub Releases as
prereleases. Test the tag itself, including any generated artifacts. Apply fixes
on `main` and create `rc2`, `rc3`, and so on as needed. Never publish an RC to
SSC.

## Final GitHub release

After deciding explicitly that a tested commit is final, verify it again and
create a new final tag that points to that commit:

```bash
VERSION=0.2.0
COMMIT=0123456789abcdef0123456789abcdef01234567
git tag -s "v${VERSION}" "$COMMIT"
git push origin "v${VERSION}"
gh release create "v${VERSION}" --verify-tag --title "v${VERSION}" \
    --notes-file path/to/release-notes.md
```

Release notes should be concise and derived from that version's changelog
section. Mark the release as a normal release and as latest when appropriate.
Do not tag a different commit merely to avoid making an explicit final-release
decision. Any attached archives or platform-specific binaries must be produced
from the tested commit, preferably by tag-triggered CI.

## SSC publication

This section applies only after the repository contains the Stata package.
Submit the package corresponding to the final GitHub Release. Never assemble an
SSC submission from a working tree on `main`.

Resolve and archive the final tag, then work only from the extracted archive:

```bash
git fetch --tags origin
git rev-parse vX.Y.Z^{commit}
git archive --format=tar.gz --prefix=cmg-X.Y.Z/ \
    --output=cmg-X.Y.Z.tar.gz vX.Y.Z
```

Record the resolved commit and archive checksum with the submission notes.
Extract the archive into a clean directory, run the metadata checker and Stata
tests there, and inspect the SSC-bound file list. Confirm that every submitted
file is byte-for-byte identical to the file at the tag, for example with
`git show vX.Y.Z:path/to/file | cmp - path/to/staging/file`. This prevents later
work on `main` from leaking into the stable package.

After SSC accepts the package, verify that `ssc install cmg` installs the
intended version. If submission uncovers a problem that requires source changes,
commit and test the fix on `main`, then make an appropriate new patch release.
Do not alter the published final tag.

At that point, document these three Stata channels distinctly. The first is the
ordinary supported installation, the second is explicitly a development build,
and the third reproduces a historical final release:

```stata
* Stable version distributed by SSC
ssc install cmg

* Development version from main
net install cmg, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/CMG/main/")

* Exact immutable historical release
net install cmg, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/CMG/v0.2.0/")
```

## After a release

Keep the released changelog section unchanged except for corrections that do not
misrepresent the released artifact. Add subsequent work under `Unreleased`, and
advance the Cargo version when development requires it. The final tag remains
the answer to “What exactly was version X.Y.Z?” even as `main` moves ahead.
