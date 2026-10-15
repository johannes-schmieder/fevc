# Installing fevc 0.5.0

`fevc` requires Stata 18 or 19. The package includes precompiled native plugins
for macOS (Apple silicon and Intel), Linux x86-64, and Windows x86-64. No
compiler, Rust installation, or separate plugin download is needed. The macOS
and Linux plugins have been tested inside Stata (the Intel macOS plugin under
Rosetta emulation on Apple silicon); the Windows plugin passes build and unit
tests but has not yet been tested inside Stata on Windows.
See the [native plugin records](native/README.md) for details.

## Installation

Run this in Stata:

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/v0.5.0/")
```

Alternatively, use the community-contributed
[`github` installer by E. F. Haghish](https://github.com/haghish/github).
Install the `github` command once if needed:

```stata
net install github, from("https://haghish.github.io/github/")
```

Then install `fevc` directly from this repository:

```stata
github install johannes-schmieder/fevc
```

Both methods install the same programs, help, licenses, and five plugin files
(macOS Apple silicon, Intel and universal, Linux x86-64, and Windows x86-64).
`fevc` loads the plugin that matches your computer. The `net install` command
above installs the [0.5.0 release](https://github.com/johannes-schmieder/fevc/releases/tag/v0.5.0);
`github install` installs the current `main` branch, which may include later
changes.

After installation:

```stata
help fevc
fevc_run exact_controls using fevc.sthlp
```

The example uses simulated data and restores your data afterward. Restart
Stata after updating a loaded native plugin. See
[inference support and limitations](fevc/docs/INFERENCE.md).

## Checking the native plugin

After installing or updating the package and restarting Stata, you can
check which features the loaded plugin supports. With the plugins included
in version 0.5.0, all of the following assertions should hold, and the
example should run on the Rust backend. This has been verified on macOS and
Linux; the Windows plugin has not yet been tested inside Stata.

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

The probe values correspond to live progress reporting, outcome centering,
mean-centered projections, mean-centered component inference, and
confidence intervals that pool movers and stayers.

## Release ZIP and offline installation

Download `fevc-0.5.0.zip` from the
[release page](https://github.com/johannes-schmieder/fevc/releases/tag/v0.5.0).
The ZIP contains only the installation files, programs, help, license
notices, and five native plugins; it does not include tests, development
tools, or benchmark data. Extract it and run:

```stata
net install fevc, replace from("/path/to/extracted/fevc")
```

`SHA256SUMS` and `release-manifest.json` identify the exact downloads and
installed files. The release also attaches the source code used to build the
native plugins; you do not need it to use the package.

## Installing from a source checkout

For development, the package can be installed from a local checkout:

```stata
net install fevc, replace from("/absolute/path/to/fevc/fevc")
```

This installs the Stata and Mata programs and the plugin loaders, but no
plugin binaries, so `backend(mata)` is used unless you build a matching
native plugin. Features that only the Rust backend provides require such a
build; see the [native build guide](rust/stata_backend/README.md).

Restart Stata after replacing a loaded native plugin. After a source update,
restart Stata or run `discard` to clear cached programs and Mata definitions.

## Using a self-built or older native plugin

The Mata backend (`backend(mata)`) runs on every platform and supports most
features. In Mata, projections and component inference use exact
calculation and are therefore limited to small designs. Live progress,
`rng(counter_v1)` and some structured component-inference requests,
including all with observation deletion, require the Rust plugin; see the
[backend reference](fevc/docs/RUST_MATA_PARITY.md). A self-built or older
native plugin may lack some features. Check it with
`fevc_rust probe` and `return list`:

- `r(centering_api) == 1` is needed for outcome centering with the Rust
  backend;
- `r(projection_centering_api) == 1` for mean-centered projections;
- `r(component_centering_api) == 1` for mean-centered component inference;
- `r(component_mixed_api) == 1` for confidence intervals that pool movers and
  stayers; and
- `r(numerical_api) == 2` for the numerical MCSE.

A value of 0 means the feature is not available. When a feature is
missing, `backend(auto)` uses Mata if it can do so before estimation starts.
An explicit `backend(rust)` or `rng(counter_v1)` stops with an error instead.
For the numerical MCSE, an omitted `mcse()` keeps the native point estimates
and reports the MCSE as unavailable, while an explicit `mcse(all)` stops with
an error. Projections and component inference cannot be combined in one
native call; they can be combined only in exact Mata without a structured
variance model. See the
[centering guide](fevc/docs/CENTERING.md) and the
[native plugin records](native/README.md).

## Upgrading an older development installation

Some earlier development versions used helper files named `_fevc*.ado`;
these are now named `fevc__*.ado`. The public `fevc` command and plugin
filenames are unchanged, and the old helper names are not installed.

Before installing this version over an earlier development installation:

1. Run `ado uninstall fevc` while the old installation is still registered.
2. Install the current package using one of the commands above.
3. Restart Stata to clear cached programs and loaded native plugins.

Using `net install ..., replace` alone leaves the old helper files behind.
If files were copied manually or several installations exist, inspect
`which fevc` and `adopath` and remove only the identified old package files;
the installer does not delete files by wildcard, and other packages can also
have underscore-prefixed filenames.
