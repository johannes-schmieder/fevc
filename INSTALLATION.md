# Installing fevc

`fevc` requires Stata 18 or 19. The package includes precompiled native plugins
for macOS Apple Silicon and Intel, Linux x86-64, and Windows x86-64. No
compiler, Rust installation, or separate plugin download is needed. All five
plugins pass source-bound build and Stata runtime checks. Intel Mac execution
was tested through Rosetta; see [native provenance](native/README.md).

## Installation

Run this in Stata:

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/main/")
```

With the community-contributed `github` installer already installed:

```stata
github install johannes-schmieder/fevc
```

Both routes install the same runtime, help, licenses, and five plugin files
(macOS arm64, Intel, universal, Linux x86-64, and Windows x86-64). The loader selects the
appropriate native backend. `main` is the development and installation branch.

After installation:

```stata
help fevc
fevc_run exact_controls using fevc.sthlp
```

The example uses simulated data and restores your data afterward. Restart
Stata after updating a loaded native plugin. This is a prerelease package;
see [inference support and limitations](fevc/docs/INFERENCE.md).

## Windows installation check

Install with either command above, then restart Stata and run:

```stata
fevc_rust probe
assert r(progress_api) == 2
fevc_run exact_controls using fevc.sthlp
assert "`e(backend_selected)'" == "rust"
fevc_run jla_controls using fevc.sthlp
assert "`e(backend_selected)'" == "rust"
matrix list e(mcse)
estat diagnostics
```

The [Windows build](https://github.com/johannes-schmieder/fevc/actions/runs/36771390189)
also provides a standalone ZIP with a local installer, test do-file, and build
receipt. It uses the same Windows binary as the repository installer. The ZIP
is available as a GitHub Actions artifact for 14 days. This exact binary also
passed isolated installation and licensed Stata/MP 19 tests on the private
Windows test machine, including the current MCSE modes and projection/inference
attachments; [the qualification record](native/mcse-default-20260930/evidence/windows-qualification.json)
binds the hosted build, runtime harness, and artifact hash.

## Local source installation

For development, the portable source package can be installed from a checkout:

```stata
net install fevc, replace from("/absolute/path/to/fevc/fevc")
```

This source manifest installs Ado/Mata files and native loaders, but no plugin
binaries. Rust-only features require a matching native build. See the
[native build guide](rust/stata_backend/README.md).

Restart Stata after replacing a loaded native plugin. After a source update,
restart Stata or run `discard` to clear cached programs and Mata definitions.

## Outcome centering capability

The current repository source implements `centering(none|mean|corrected)` in
Mata and matching Rust builds. This repository contains qualified Mac
arm64, Rosetta x86-64, universal and Linux x86-64 centering plugins. The
Windows plugin is unchanged and lacks active centering. Its new hosted build
passes, but full runtime qualification and a focused diagnostic fail; see the
[October 7 attempt record](native/centering-20261007/manifest.json).

A current source installation can use `backend(mata)` on every platform.
For Rust, update to the matching qualified repository plugin, restart Stata,
and inspect its capability:

```stata
fevc_rust probe
return list
* Active native centering requires r(centering_api) == 1
```

A missing or zero `r(centering_api)` means the plugin does not support active
centering. Numerical API 2 for all-probe MCSE is independent of centering
API 1: a plugin supporting MCSE can still lack centering.
`backend(auto)` may use Mata before preparation/RNG if the centering
capability is absent, subject to strict native-consent rules.
`backend(rust)` or explicit `rng(counter_v1)` requires the matching build.

The [centering adoption record](native/centering-20261006/manifest.json) binds
the four updated plugins to full platform and explicit Rust centering
checks. The owner authorized source publication on October 7. The Windows hosted
build passes, but its candidate remains unqualified after two private failures.
Earlier binary records retain their original tested artifacts. See [centering](fevc/docs/CENTERING.md) and
[native provenance](native/README.md).

## Upgrading an older development installation

The September 18 prerelease cleanup renamed the 28 `_fevc*.ado` helper files
to `fevc__*.ado`. The public `fevc` command and plugin filenames are unchanged;
old helper aliases are not installed. Helpers are internal interfaces.

Before installing this source over an earlier development installation:

1. Run `ado uninstall fevc` while the old installation is still registered.
2. Install the current package from your chosen source using the commands above.
3. Restart Stata to clear cached ado/Mata programs and loaded native plugins.

Using `net install ..., replace` alone leaves the old helper files behind.
If files were copied manually or multiple installations exist, inspect `which
fevc` and `adopath` and remove only the identified obsolete package files;
the installer does not delete files by wildcard. Other packages can also
have underscore-prefixed filenames.

## Numerical MCSE interface

Current source defaults to `mcse(all)` and accepts `mcse(off)` to skip the
additional work. Native all-MCSE requires the numerical V2 interface. With an
older plugin, an implicit default preserves point estimation and reports MCSE
unavailable; an explicit `mcse(all)` requires a matching plugin. Restart Stata
when replacing loaded Mata/native runtimes. Platform qualification and binary
adoption are recorded in [native provenance](native/README.md).
Mean's MCSE holds the observed mean fixed; Corrected's also holds its extra
centering increment fixed. Exact enabled MCSE is zero; off is unavailable.
See [the centering contract](fevc/docs/CENTERING.md).
