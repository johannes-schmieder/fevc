# Rust numerical source provenance

The Rust backend is distributed under `GPL-3.0-only`. Public binary release remains subject to the repository's required human license and provenance review.

## Improved CMG baseline

The Rust CMG implementation is a source-informed port of the `varcomp_kss` Mata CMG component. The initial hybrid graph implementation ports only the numerical construction needed to represent the worker-eliminated firm Laplacian exactly:

- repository: `johannes-schmieder/varcomp_kss`;
- baseline repository commit: `396b529f5e8c18afed0e5b87145082b0f683341f`;
- source path: `varcomp_kss/cmg/src/cmg_core.mata.in`;
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

## Standalone Stata SDK boundary

The standalone plugin build does not bundle the licensed Stata Plugin SDK.
An authorized build must set `VCKSS_STATA_SDK_DIR` explicitly to a directory
containing both `stplugin.c` and `stplugin.h`. The build stops before compiling
the C shim when that variable is unset, is not a directory, or lacks either
file.

This preflight checks only the configured directory and required filenames. It
does not establish the SDK's authenticity, version, licensing, provenance, or
content hashes. Those facts must be verified by the controlled environment
that supplies the SDK before producing or distributing a plugin artifact.
