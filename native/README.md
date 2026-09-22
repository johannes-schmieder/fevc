# Native package provenance

The repository installation includes macOS arm64, macOS x86-64, macOS
universal, Linux x86-64, and a Windows x86-64 candidate. Windows Stata testing
is pending the owner's check. Intel Mac execution uses Rosetta, not native
Intel hardware.

The [current Windows build](https://github.com/johannes-schmieder/fevc/actions/runs/35759636558)
uses source `991597f7`, including the shortened help and simulation helper.
It passed compilation, 14 Rust unit tests, 129-export PE inspection, system-only
dependency checks, and all package hashes. Its
[build receipt](refresh-20260922-windows-build.json) records the source and
payload. The same binary is installed from the repository and included in the
workflow's standalone ZIP with a smoke-test do-file. Licensed Windows Stata
has not been run. No AWS machine was used. GitHub artifacts last 14 days;
the committed plugin remains available through the normal installer. The
[earlier Windows candidate record](windows-test-20260922.json) remains historical.

The current package was retested with the existing Mac binaries: arm64 and
Rosetta x86-64 thin plugins, plus the universal binary under both architectures.
All four isolated installs passed the exact/JLA help examples, progress
regression, installed-file hashes, and idle-registry check. The compiled native
sources and locked dependencies are unchanged since their qualified build;
the new simulation helper and help are covered by the current package tests.
The [refresh receipt](refresh-20260922.json) records compatibility, current
binary hashes, and public installer checks. Linux remains blocked as described
below; this is not a completed all-platform qualification.

The Mac plugins were rebuilt from `0614534c` on September 22 to suppress
repeated progress summaries. Their [qualification receipt](progress-20260922-macos.txt)
records the exact source, binary hashes, and passing arm64, Rosetta x86-64,
and universal-plugin checks. The [source inventory](progress-20260922-macos.sha256)
binds the tested files. The exact help example was also checked to print one
method summary and one allocation summary, followed by elapsed-time updates.
[Validation and public installer checks](progress-20260922-validation.json)
record the passing source, Rust, and Stata gates and both public installation
commands on Mac, including replacement installs and all 50 installed-file hashes.

The Linux plugin remains the September 21 build from `f2a15dea`; it does not
yet contain this progress-display fix. Its rebuild is awaiting SCC login access.
The original [input manifest](manifest.json) and qualification receipts preserve
the September 21 binaries' actual build sources and SHA-256 hashes.
[Installation verification](installation.json) records fresh and replacement
installs of that initial package with both public commands on Mac and Linux under Stata/MP 19. Later packaging and evidence commits
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
