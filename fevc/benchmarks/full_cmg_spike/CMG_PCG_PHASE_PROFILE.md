# Scalar CMG PCG phase diagnostic

This diagnostic uses the existing profiler from exact standalone CMG commit
`dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`. It is not a VCkss estimator or
MATLAB comparison. Its purpose is to choose the next bounded optimization
after the end-to-end SCC direct route identified repeated solves as dominant.

The generated connected worker-firm graph has 200,000 vertices and 299,996
canonical edges. Three one-thread repetitions take a production median of
156.491 milliseconds per 20-iteration solve. The profiler adds 0.73% overhead
and returns a bitwise-identical solution.

| Phase | Median ms | Share |
|---|---:|---:|
| CMG preconditioner | 80.795 | 51.26% |
| quotient centering | 21.288 | 13.51% |
| solution/residual norms | 19.692 | 12.49% |
| finest matvec | 14.982 | 9.50% |
| dot products | 13.758 | 8.73% |
| vector updates | 2.970 | 1.88% |
| setup, recompute, certification | 4.044 | 2.57% |

This supports one narrow next experiment: keep independent scalar PCGs and
the official hierarchy, but fuse deterministic vector passes and defer
solution null-space centering on a certified connected graph. The final
solution must still be deterministically centered and certified against the
complete original system. A nonconnected graph must fail closed to the
ordinary direct route. No performance claim follows until this is measured
inside the actual hybrid estimator path.

Exact command shape (the temporary path is freshly created):

```bash
profile_root=$(mktemp -d /private/tmp/vckss-cmg-pcg-profile.XXXXXX)
git -C /Users/johannes/Git/CMG archive dbefbc5 | tar -x -C "$profile_root"
CARGO_TARGET_DIR="$profile_root/target" \
  /Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin/cargo \
  run --manifest-path "$profile_root/benchmarks/Cargo.toml" \
  --release --locked --offline --bin pcg-phase-profile -- \
  worker-firm 100000 3 1
```

The exact values and scientific scope are in
[`cmg_pcg_phase_profile_2026-08-25.json`](cmg_pcg_phase_profile_2026-08-25.json).
