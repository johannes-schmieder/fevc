# VCkss CMG vendor record

- Upstream repository: `https://github.com/johannes-schmieder/CMG`
- Upstream commit: `98768722fa21800d2cb91cd2182406c9db3bf979`
- Selected-source archive SHA-256: `0ea043e9544da299f3a160a7aa6818167592f45c6823dcce95c09f41a4efdf65`
- License: `GPL-3.0-only`
- Imported on: 2026-08-27

The import contains the upstream library crate, tests, license, README, and
upstream provenance document. Upstream benchmark programs and generated
benchmark artifacts are intentionally excluded from the normal VCkss build.
`UPSTREAM_MANIFEST.sha256` records the exact imported bytes before VCkss
integration patches. The selected-source archive is the deterministic output
of `git archive --format=tar` at the pinned commit over the manifest paths.
The pinned source is a direct descendant of performance candidate `d9fef06`.
Its only intervening numerical-source change rewrites two LDL index loops as
ordered iterators so the registered strict Clippy gate passes without changing
the floating-point operation order. The later standalone change corrects the
performance workflow to compare identifiable Laplacian edge gradients while
retaining raw coordinate differences as diagnostics; none of the 42 selected
library, test, license, or provenance files changed from `88bf024`.

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
