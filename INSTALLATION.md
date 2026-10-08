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

The older October 8 smoke `win-20261008T153726Z-59a2355d` remains a
`STATA_DRIVER_FAILED` attempt with no identified assertion. The diagnostic
collector has since been deployed and its infrastructure checks pass. For the
Mean-component extension, the first private source-build smoke failed at
`build_toolchain` with return code 601 and the second at `build_native` with
a null return code. Cleanup completed and the guarded machine is stopped.
The prepared hosted-build alternative passes 81 offline tests and independent
review; source publication before complete qualification and a bounded retry
await owner approval. No new full Windows qualification or payload adoption
has occurred. See [the current candidate record](native/mean-component-20261008/checkpoint.json).

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
exact and JLA. Use explicit `centering(none)` to reproduce the former default.
Projection and component `inference(highrank|q1)` support Mean or None on their
existing supported tuples; Corrected remains unsupported with either. Mean
component inference holds the observed retained physical-frequency
working-outcome mean fixed and omits its estimation uncertainty. This
approximation is separate from numerical MCSE and from omitted nuisance-offset
uncertainty. New Mac candidates at `b9f80ce9` and Linux at `63757839`
pass source-bound qualification and installed Mean-component checks. They
remain preserved outside the installation payloads while Windows blocks the
five-payload update. The existing Windows manual-test binary has point
centering API 1 and lacks both attachment-centering capabilities. See
[the current candidate record](native/mean-component-20261008/checkpoint.json).
The [completed assessment](fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
records substantial exact-Mata interval unavailability and native numerical
sensitivity; installation or successful computation is not a coverage claim.

A current source installation can use `backend(mata)` on every platform.
For Rust, use a matching qualified build, restart Stata, and inspect its
capability. The retained repository payloads do not yet provide Mean component
inference:

```stata
fevc_rust probe
return list
* Active native centering requires r(centering_api) == 1
* Mean projection also requires r(projection_centering_api) == 1
* Mean component inference also requires r(component_centering_api) == 1
```

A missing or zero `r(centering_api)` means the plugin does not support active
centering. Numerical API 2 for all-probe MCSE is independent of centering
API 1: a plugin supporting MCSE can still lack centering. Mean projection
requires the separate additive `r(projection_centering_api) == 1` capability.
The local Mac candidates expose this capability. Earlier point-centering
payloads, including the retained Windows manual-test binary, lack it.
Use current source with `backend(mata)` or a matching qualified native build
and inspect the probe result. Mean component inference additionally requires
`r(component_centering_api) == 1`, which identifies matching C transport and
Rust core readiness bit 16. It uses the existing capabilities struct/export;
there is no new DLL export or layout. Existing combined exact-Mata requests
support Mean; native combined component/projection requests remain unsupported
for both Mean and None. Missing metadata is zero; None retains its
existing capability contract.
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
