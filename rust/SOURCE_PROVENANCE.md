# Rust numerical source provenance

The Rust backend is distributed under `GPL-3.0-only`. Public binary release remains subject to the repository's required human license and provenance review.

## Improved CMG baseline

The Rust CMG implementation is a source-informed port of the `vckss` Mata CMG component. The initial hybrid graph implementation ports only the numerical construction needed to represent the worker-eliminated firm Laplacian exactly:

- repository: `johannes-schmieder/vckss`;
- baseline repository commit: `396b529f5e8c18afed0e5b87145082b0f683341f`;
- source path: `vckss/cmg/src/cmg_core.mata.in`;
- source Git blob: `5b5acdde93c3e154317c82f99d506c42006986e6`;
- relevant Mata routines: prepared worker-firm cells, hybrid graph construction, graph finalization, graph action, preflight forecasting;
- Rust destination: `rust/crates/vckss-core/src/cmg.rs`;
- modification: Rust data structures, zero-based checked identifiers, deterministic `BTreeMap` edge collapse, explicit CSR incidence, fallible resource accounting, and Rust unit tests.

The exact hybrid construction is:

- degree two: one edge with conductance `w1*w2/sum(w)`;
- degree three: the three clique edges with conductance `wi*wj/sum(w)`;
- degree four or greater: one auxiliary worker vertex with incident conductances `wi`.

Eliminating each auxiliary vertex recovers `diag(w) - w*w'/sum(w)`, so the hybrid graph is algebraically identical to the worker-eliminated firm Schur contribution while remaining linear in high worker degree.

No MEX interface, compiled upstream binary, or imported runtime is used.

## Vendored full CMG backend

The production `CMG_FULL_V2` route vendors the deterministic Rust CMG library
from `https://github.com/johannes-schmieder/CMG` at exact commit
`761a0f022f20d1114d9f20589b60563eab6fcb84`. The imported library source,
tests, license, README, and upstream provenance document live under
`rust/vendor/cmg/`; upstream benchmark programs and generated benchmark
artifacts are excluded. `rust/vendor/cmg/VENDOR.md` records the archive hash,
file-level upstream hashes, and the narrow VCkss integration patch classes.

The vendored crate and VCkss are `GPL-3.0-only`. Normal builds use the locked
path dependency and Rust 1.85.1; they do not read or modify a standalone CMG
checkout. Public distribution remains subject to the repository's human
mathematical and license/provenance review.

## Stata plugin interface boundary

The ordinary third-party Stata plugin build uses the public SPI 3.0 files that
StataCorp instructs plugin authors to download from
`https://www.stata.com/plugins/`. They are not repository-authored GPL source
and remain untracked. The files were reviewed and retrieved on 2026-08-21;
`stplugin.c` identifies SPI 3.0 and is 198 bytes, while `stplugin.h` identifies
version 3.0.0 and is 6,215 bytes. Run
`rust/stata_backend/fetch_stata_spi.sh` to download them into the ignored local
build directory. The helper and build script both enforce the tracked
`rust/stata_backend/stata-spi.sha256` manifest, whose reviewed values are
`ab694f53e30a404bbfbe59d301a81b8bc59eeecf84bc5427eb65cbf0c5020d6d`
and `0d32086bfb7a621e30ed7fefa41b351b6733bb4561da28a4c581580d62c64e8b`.

`VCKSS_STATA_SPI_DIR` remains an optional override for builders who keep those
two SPI files elsewhere; overrides must match the same reviewed hashes. Public
download availability does not itself determine redistribution rights. A
future public source or binary distribution must respect StataCorp's notices
and then-current terms for these third-party files.
