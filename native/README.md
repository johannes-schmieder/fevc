# Native package provenance

The repository installation includes macOS arm64, macOS x86-64, macOS
universal, and Linux x86-64 plugins. Windows binaries and testing are deferred.
Intel Mac execution is checked under Rosetta, not on native Intel hardware.

The input manifest and qualification receipts in this directory record each
binary's actual build source and SHA-256. Later packaging and evidence commits
do not change that build identity. Root `fevc.pkg` selects the native package;
`fevc/fevc.pkg` remains the portable source manifest.

Corresponding FEVC and CMG source, C shim, build scripts, and locked dependencies
are in `rust/` at the recorded build commit. See
[`rust/stata_backend/README.md`](../rust/stata_backend/README.md) for building.
[`dependencies.json`](dependencies.json) identifies every pinned registry crate,
its source download, license, and checksum. Runtime dependency license texts
are included in the installed [`THIRD_PARTY_NOTICES.txt`](../fevc/THIRD_PARTY_NOTICES.txt).

To check a staged root catalog through a loopback HTTP server in fresh,
isolated Stata directories:

```bash
./.venv/bin/python fevc/tools/verify_native_installers.py \
    --catalog /path/to/staged/package --test-root "$PWD" \
    --output /path/to/new/evidence-directory
```

Add `--public` to test both advertised public commands, including replacement
of an altered installed help file. The verifier checks every installed byte,
the current native reporting API, the README example and caller-data restoration,
help, decomposition, and the installed match q0/q1 regression. Stata execution
must use local or explicitly private licensed infrastructure. Detailed logs
remain outside the tracked checkout; only compact receipts are retained here.

When publishing a package update, keep `main` and the `master` compatibility
ref on the same package. The community `github` installer currently uses
`master` URLs. No release tag or release archive is implied by this prerelease
repository installation.
