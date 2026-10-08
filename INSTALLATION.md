# Installing fevc

`fevc` requires Stata 18 or 19. The package includes precompiled native plugins
for macOS Apple Silicon and Intel, Linux x86-64, and Windows x86-64. No
compiler, Rust installation, or separate plugin download is needed. Native
qualification is specific to the exact artifact and platform. Intel Mac
execution uses Rosetta; Windows runtime qualification remains pending.
See [native provenance](native/README.md).

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
These URLs serve the published package; the October 8 local prerelease
candidate has not been published.

After installation:

```stata
help fevc
fevc_run exact_controls using fevc.sthlp
```

The example uses simulated data and restores your data afterward. Restart
Stata after updating a loaded native plugin. This is a prerelease package;
see [inference support and limitations](fevc/docs/INFERENCE.md).

## Existing Windows manual-test payload

Install with either command above, then restart Stata and run:

```stata
fevc_rust probe
assert r(progress_api) == 2
assert r(centering_api) == 1
fevc_run exact_controls using fevc.sthlp
assert "`e(backend_selected)'" == "rust"
fevc_run jla_controls using fevc.sthlp
assert "`e(backend_selected)'" == "rust"
matrix list e(mcse)
estat diagnostics
```

The [October 7 Windows build](https://github.com/johannes-schmieder/fevc/actions/runs/37571812326)
produced a standalone ZIP with a local installer, test do-file and build
receipt. It supplied the Windows binary retained in the repository installer.
Its hosted build and binary audit pass; automated private runtime checks fail.
The owner requested this exact binary for manual testing. The
[manual-test adoption record](native/centering-windows-manual-20261007.json)
binds its source and hash; earlier passing Windows records apply to earlier
bytes.

The October 8 bounded smoke `win-20261008T153726Z-59a2355d` also returned
`STATA_DRIVER_FAILED`. The accepted collector supplies only an aggregate
receipt, so the failing assertion is unknown. A reviewed proposal to collect
bounded failure diagnostics and validated candidate artifacts awaits owner
approval; it has not been deployed. Current Windows qualification remains
pending. See the [current candidate record](native/prerelease-20261008/manifest.json).

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
Mata and matching Rust builds. Omitted `centering()` selects `mean` for both
exact and JLA. Use explicit `centering(none)` to reproduce the former default
or request supported component inference. Projection supports Mean and None;
Corrected remains unsupported. The local Mac candidates at source `24754269`
pass clean build/runtime qualification and 24 isolated-install capability,
point-centering and Mean-projection checks. Linux x86-64 also passes full
qualification, installed point centering and
Mean projection at the same source (SCC job `7962808`). The existing Windows
manual-test payload has centering API 1;
its runtime qualification is pending and it lacks projection-centering API 1.
See the [current candidate record](native/prerelease-20261008/manifest.json).

A current source installation can use `backend(mata)` on every platform.
For Rust, update to the matching repository plugin, restart Stata,
and inspect its capability:

```stata
fevc_rust probe
return list
* Active native centering requires r(centering_api) == 1
* Mean projection also requires r(projection_centering_api) == 1
```

A missing or zero `r(centering_api)` means the plugin does not support active
centering. Numerical API 2 for all-probe MCSE is independent of centering
API 1: a plugin supporting MCSE can still lack centering. Mean projection
requires the separate additive `r(projection_centering_api) == 1` capability.
The local Mac candidates expose this capability. Earlier point-centering
payloads, including the retained Windows manual-test binary, lack it.
Use current source with `backend(mata)` or a matching qualified native build
and inspect the probe result. None projection retains its existing capability contract.
`backend(auto)` may use Mata before preparation/RNG if the centering
capability is absent, subject to strict native-consent rules.
`backend(rust)` or explicit `rng(counter_v1)` requires the matching build.

The historical [centering adoption record](native/centering-20261006/manifest.json) binds
the four October 6 plugins to full platform and explicit Rust centering
checks. The owner authorized source publication on October 7. The Windows hosted
build passes; the owner subsequently authorized its adoption for manual
testing while runtime qualification remains pending.
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
