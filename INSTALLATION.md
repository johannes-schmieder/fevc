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
An installation from an older development package with renamed runtime files
should first be removed with `ado uninstall fevc`.
