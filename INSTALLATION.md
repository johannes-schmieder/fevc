# Installing fevc

`fevc` requires Stata 18 or 19. The public package is being prepared with
precompiled native plugins for macOS Apple Silicon and Intel, Linux x86-64,
and Windows x86-64.

## Public installation

These are the planned public commands. They are not yet operational: the
complete native payload, Windows qualification, and final installation checks
are unfinished, and the repository has not been published.

```stata
net install fevc, replace ///
    from("https://raw.githubusercontent.com/johannes-schmieder/fevc/main/")
```

With the community-contributed `github` installer already installed:

```stata
github install johannes-schmieder/fevc
```

Both routes are intended to install the same complete package. No compiler,
Rust installation, or separate plugin download will be required. Installer
compatibility with the repository's `main` branch must be verified before
publication. The native package preparation process is documented
[here](fevc/docs/RC_BINARY_PAYLOAD.md).

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
