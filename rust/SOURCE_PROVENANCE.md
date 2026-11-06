# Rust numerical source provenance

The Rust backend is distributed under `MIT`. Public binary release remains subject to the repository's required human license and provenance review.

## Improved CMG baseline

The Rust CMG implementation is a source-informed port of the predecessor Mata
CMG component. The initial hybrid graph implementation ports only the
numerical construction needed to represent the worker-eliminated firm
Laplacian exactly:

- repository: `johannes-schmieder/fevc`;
- baseline repository commit: `396b529f5e8c18afed0e5b87145082b0f683341f`;
- source path at that commit: `vckss/cmg/src/cmg_core.mata.in`;
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

The production `CMG_FULL_V2` route uses unmodified CMG 0.2.0 from
`https://github.com/johannes-schmieder/CMG`, pinned to the full revision in
`rust/vendor/CMG-SOURCE.json`. That external record inventories every vendored
file, including the unchanged standalone Cargo manifest. The vendor directory
is excluded from FEVC's workspace; all integration code lives in
`rust/crates/vckss-core/src/full_cmg_host.rs`.

Refresh and verify from a local upstream checkout with:

```sh
./.venv/bin/python fevc/tools/cmg_vendor.py refresh --source /path/to/CMG --rev FULL_SHA --destination rust/vendor/cmg --record rust/vendor/CMG-SOURCE.json
./.venv/bin/python fevc/tools/cmg_vendor.py check --source /path/to/CMG --rev FULL_SHA --destination rust/vendor/cmg --record rust/vendor/CMG-SOURCE.json
```

The offline check omits `--source` and `--rev`; it compares the entire inventory
to the committed provenance. Release validation also runs the upstream-backed
check. Update the Rust and Stata source-identity constants with each refresh.
The former private vendor patches and their provenance remain in Git history.

FEVC owns one joined Rayon pool and uses it both for CMG teams and statistical
work. The public CMG contexts carry only the atomic cancellation query; Stata
callbacks stay on the caller. A fixed, admitted batch-workspace bank services
the dynamically claimed RHS queue. Input ordering, warm starts, and complete
original-system certification remain FEVC contracts. `max_levels = 128` is
explicit, preserving FEVC's established option.

Memory admission counts CMG workspace payload and descriptors, FEVC RHS/output
and queue buffers, and host worker handles. Requested 2 MiB worker stacks and
128 KiB per-worker Rayon heap allowances are separate runtime planning charges,
not measured allocation bounds or process-RSS guarantees. Allocator-only
accuracy checks exclude requested OS stack reservations while retaining the
registered tolerance; command admission includes them.

The vendored crate and FEVC are `MIT`. Normal builds use the locked
path dependency and Rust 1.85.1; they do not read or modify a standalone CMG
checkout. Public distribution remains subject to the repository's human
mathematical and license/provenance review.

## Stata plugin interface boundary

The ordinary third-party Stata plugin build uses the public SPI 3.0 files that
StataCorp instructs plugin authors to download from
`https://www.stata.com/plugins/`. They are not repository-authored MIT source
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
