# VCkss CMG vendor record

- Upstream repository: `https://github.com/johannes-schmieder/CMG`
- Upstream commit: `761a0f022f20d1114d9f20589b60563eab6fcb84`
- Selected-source archive SHA-256: `0669be90011452dad231e43642be592f2cae34cfd32328f1e2c26a35ef2b8af0`
- License: `GPL-3.0-only`
- Imported on: 2026-08-27

The import contains the upstream library crate, tests, license, README, and
upstream provenance document. Upstream benchmark programs and generated
benchmark artifacts are intentionally excluded from the normal VCkss build.
`UPSTREAM_MANIFEST.sha256` records the exact imported bytes before VCkss
integration patches. The selected-source archive is the deterministic output
of `git archive --format=tar` at the pinned commit over the manifest paths.

VCkss modifications are kept narrow and source-visible:

1. remove upstream benchmark binary declarations from the vendored manifest;
2. align only the two mechanical Clippy allowances already used by the VCkss
   workspace, without changing CMG arithmetic;
3. add the contiguous, fixed-order independent-RHS bridge used by
   `CMG_FULL_V2`;
4. add cooperative atomic cancellation at hierarchy, parallel-plan, PCG,
   V-cycle, and fixed-order multi-RHS boundaries. The VCkss caller thread owns
   the Stata callback; CMG and Rayon workers observe only the shared atomic
   flag. This patch adds `src/cancel.rs` and cancellable counterparts in
   `hierarchy.rs`, `preconditioner.rs`, `pcg.rs`, `parallel_solver.rs`, and
   `vckss_bridge.rs` without changing the original entry points. The optional
   profiling module calls the same prevalidated non-cancelling form through
   that additive interface; and
5. add checked memory-forecast interfaces required by the VCkss pre-RNG
   admission and lifecycle contract; and
6. add certified caller-supplied initial guesses to the scalar/planned PCG and
   contiguous-column bridge so same-route VCkss residual refinement can reuse
   an already certified solution without changing the frozen tolerance ladder.

The standalone `/Users/johannes/Git/CMG` checkout is not a build dependency
and is never modified by VCkss builds.
