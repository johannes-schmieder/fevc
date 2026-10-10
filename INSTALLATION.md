# Installing fevc

`fevc` requires Stata 18 or 19. The package includes precompiled native plugins
for macOS Apple Silicon and Intel, Linux x86-64, and Windows x86-64. No
compiler, Rust installation, or separate plugin download is needed. Native
qualification is specific to the exact artifact and platform. Intel Mac
qualification uses Rosetta. The current Windows artifact passes hosted build,
unit and PE/export checks; Windows Stata runtime testing is pending with the
owner, who explicitly requested publication before that test.
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
These URLs serve the published package. The [current checkpoint](fevc/PLAN.md)
tracks publication and installation verification; the
[adoption manifest](native/pooled-component-20261010/manifest.json) binds the
included payloads to their sources and tested routes.

After installation:

```stata
help fevc
fevc_run exact_controls using fevc.sthlp
```

The example uses simulated data and restores your data afterward. Restart
Stata after updating a loaded native plugin. This is a prerelease package;
see [inference support and limitations](fevc/docs/INFERENCE.md).

## Native capability check

After updating the package and restarting Stata, inspect the selected plugin:

```stata
fevc_rust probe
assert r(progress_api) == 2
assert r(centering_api) == 1
assert r(projection_centering_api) == 1
assert r(component_centering_api) == 1
assert r(component_mixed_api) == 1
fevc_run exact_controls using fevc.sthlp
assert "`e(backend_selected)'" == "rust"
```

All five included binaries are rebuilt from `002205f2` and provide the centering
capabilities plus pooled component inference and bounded diagonal residual
refinement. Mac arm64/Rosetta thin/universal and Linux full/installed qualification
pass. Windows runtime qualification remains pending. See the
[adoption manifest](native/pooled-component-20261010/manifest.json).

The package contains 61 installed files, including five plugins. Installation
checks verify every file hash and exercise pooled components on both backends
and with both variance models. The [current checkpoint](fevc/PLAN.md) and
[installation receipts](native/pooled-component-20261010/evidence/packaging/)
record local HTTP and public `net`/`github` fresh/replacement results on Mac arm64.
These installation checks do not establish Windows runtime qualification.

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
uncertainty. The five included native payloads support point, projection
and component centering API 1. Exact sources and platform qualification limits
are in the [adoption manifest](native/pooled-component-20261010/manifest.json).
The [completed assessment](fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
records substantial exact-Mata interval unavailability and native numerical
sensitivity; installation or successful computation is not a coverage claim.

A current source installation can use `backend(mata)` on every platform.
For Rust, use a matching qualified build, restart Stata, and inspect its
capabilities:

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
The included five-payload set exposes this capability. Earlier
point-centering payloads may lack it.
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
checks. The owner authorized source publication on October 7. At that checkpoint,
the Windows hosted build passed and the owner subsequently authorized adoption for manual
testing while runtime qualification remained pending.
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
