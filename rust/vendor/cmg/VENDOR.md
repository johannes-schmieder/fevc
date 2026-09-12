# FEVC CMG vendor record

- Upstream repository: `https://github.com/johannes-schmieder/CMG`
- Upstream commit: `92a12f2d572ca56b30a035220953f9dd4bced999`
- Selected-source archive SHA-256: `708e5d1eac60fe472246ce2989c6d9753667b4ceb9f5286b48c192d16b870b34`
- License: `GPL-3.0-only`
- Imported on: 2026-08-27

The import contains the upstream library crate, tests, license, README, and
upstream provenance document. Upstream benchmark programs and generated
benchmark artifacts are intentionally excluded from the normal FEVC build.
`UPSTREAM_MANIFEST.sha256` records the exact imported bytes before FEVC
integration patches. The selected-source archive is the deterministic output
of `git archive --format=tar` at the pinned commit over the manifest paths.
The pinned source is a direct descendant of performance candidate `d9fef06`.
Its only intervening numerical-source change rewrites two LDL index loops as
ordered iterators so the registered strict Clippy gate passes without changing
the floating-point operation order. The later standalone change corrects the
performance workflow to compare identifiable Laplacian edge gradients while
retaining raw coordinate differences as diagnostics, followed by its
benchmark-workspace formatting correction; none of the 42 selected library,
test, license, or provenance files changed from `88bf024`.

FEVC modifications are kept narrow and source-visible:

1. remove upstream benchmark binary declarations from the vendored manifest;
2. align only the two mechanical Clippy allowances already used by the FEVC
   workspace, without changing CMG arithmetic;
3. add the contiguous, fixed-order independent-RHS bridge used by
   `CMG_FULL_V2`;
4. add cooperative atomic cancellation at hierarchy, parallel-plan, PCG,
   V-cycle, and fixed-order multi-RHS boundaries. The FEVC caller thread owns
   the Stata callback; CMG and Rayon workers observe only the shared atomic
   flag. This patch adds `src/cancel.rs` and cancellable counterparts in
   `hierarchy.rs`, `preconditioner.rs`, `pcg.rs`, `parallel_solver.rs`, and
   `vckss_bridge.rs` without changing the original entry points. The optional
   profiling module calls the same prevalidated non-cancelling form through
   that additive interface; and
5. add checked memory-forecast interfaces required by the FEVC pre-RNG
   admission and lifecycle contract; and
6. add certified caller-supplied initial guesses to the scalar/planned PCG and
   contiguous-column bridge so same-route FEVC residual refinement can reuse
   an already certified solution without changing the frozen tolerance ladder;
7. replace barrier-separated contiguous RHS waves with an ordered work queue
   on September 12, 2026. Each admitted scalar workspace independently claims
   the next RHS; output and error order remain input order. Add fallible pool
   construction and checked queue-metadata accounting. Changes are confined
   to `vckss_bridge.rs`, `pcg.rs`, `workspace.rs`, `components.rs`,
   `preconditioner.rs` and `error.rs`; scalar arithmetic is unchanged. This is
   a FEVC integration patch, not an upstream CMG update or fused-RHS port.

The standalone `/Users/johannes/Git/CMG` checkout is not a build dependency
and is never modified by FEVC builds.
