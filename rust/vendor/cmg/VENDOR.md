# VCkss CMG vendor record

- Upstream repository: `https://github.com/johannes-schmieder/CMG`
- Upstream commit: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- Imported archive SHA-256: `b8a1b9a61dffc52ab914f1bcdafab221635aeaaec7fe5c5eb35ad24b705cb626`
- License: `GPL-3.0-only`
- Imported on: 2026-08-26

The import contains the upstream library crate, tests, license, README, and
upstream provenance document. Upstream benchmark programs and generated
benchmark artifacts are intentionally excluded from the normal VCkss build.
`UPSTREAM_MANIFEST.sha256` records the exact imported bytes before VCkss
integration patches.

VCkss modifications are kept narrow and source-visible:

1. remove upstream benchmark binary declarations from the vendored manifest;
2. align only the two mechanical Clippy allowances already used by the VCkss
   workspace, without changing CMG arithmetic;
3. add the contiguous, fixed-order independent-RHS bridge used by
   `CMG_FULL_V2`;
4. add cooperative cancellation and memory-forecast interfaces required by
   the VCkss Stata lifecycle.

The standalone `/Users/johannes/Git/CMG` checkout is not a build dependency
and is never modified by VCkss builds.
