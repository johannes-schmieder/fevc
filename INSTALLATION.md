# Installing fevc

`fevc` requires Stata 18 or 19. The package includes precompiled native plugins
for macOS Apple Silicon and Intel, and Linux x86-64. No compiler, Rust
installation, or separate plugin download is needed. Windows binaries and
Windows testing are deferred; this distribution makes no Windows native claim.

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

Both routes install the same runtime, help, licenses, and four plugin files
(macOS arm64, Intel, universal, and Linux x86-64). The loader selects the
appropriate native backend. The `master` compatibility branch serves the
community installer; `main` remains the development branch.

After installation:

```stata
help fevc
fevc_run exact_controls using fevc.sthlp
```

The example uses simulated data and restores your data afterward. Restart
Stata after updating a loaded native plugin. This is a prerelease package;
see [inference support and limitations](fevc/docs/INFERENCE.md).

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
