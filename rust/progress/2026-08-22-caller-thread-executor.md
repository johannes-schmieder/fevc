# Caller-thread deterministic executor qualification

Status date: 2026-08-22

The deterministic fixed-partition executor now evaluates the first partition on
the calling thread and spawns only the remaining `N - 1` scoped workers. Result
collection remains in ascending partition order, so the registered merge order
is unchanged. The change removes one thread creation and join from every
multi-partition invocation without changing the public API or numerical order.

The source was copied byte-for-byte from
`codex/rust-backend-completion` to the public `playground` compiler controller.
The exact-tree verifier matched every tracked Rust file to the source-bound
private Rust tree. The resulting GitHub Actions matrix passed on:

- Ubuntu, macOS, and Windows;
- Rust 1.81.0 and current stable; and
- all required commands: strict rustfmt, strict all-target Clippy, debug tests,
  release tests, workspace/all-target release build, and root release build.

The allocation-bound deterministic-reduction integration test and the bitwise
compensated-summation test also remained green. Workflow jobs and decoded logs
were inspected; no failed steps or GitHub error annotations were found.

No Rust code was compiled locally and Stata was not executed. This checkpoint
qualifies the Rust source and tests only; it does not qualify the plugin inside
licensed Stata.

Next recovery point:

1. add explicit deterministic-executor invariance tests through 32 requested
   threads, including stable partition/result/error ordering;
2. reconcile the generic batch direct-memory forecast with the already
   qualified reuse of the Krylov action buffer during final residual
   verification; and
3. resume the unfinished V4 solve/V7 receipt boundary in
   `varcomp_kss_rust.ado`.
