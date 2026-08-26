# SCC direct-versus-fused checkpoint

The source-bound SCC result keeps the independent direct full-CMG route and
disables the experimental fused-f64 executor. This remains private evidence:
the direct route is 1.669 times as fast as MATLAB end to end, but misses the
registered 2-times threshold by 25.079 seconds.

| Route | Command seconds | Native solve seconds | Peak RSS KiB |
|---|---:|---:|---:|
| A: baseline | 576.131 | 549.191 | 5,458,404 |
| C: direct | 151.351 | 123.476 | 4,063,416 |
| C: fused f64 | 239.610 | 211.250 | 4,145,168 |
| MATLAB | 252.543 | -- | 11,808,708 |

Fusing the independent columns was 1.583 times slower end to end and 1.711
times slower in the native solve than the direct route. Direct and fused
returned bit-identical corrected targets. Both kept the maximum complete
original-system residual at `6.971259e-6`, below the registered `1e-5` probe
gate, and passed accounting, caller-state, and memory checks.

The dominant remaining cost is the official full-CMG repeated solve, not RHS
generation, extraction, or Stata overhead. The next bounded route is therefore
to keep coarse independent-RHS concurrency, measure the contiguous-input
bridge, and profile official CMG preconditioner applications. The rejected
fused executor is retained as evidence but must stay disabled.

Exact submission command:

```bash
VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  vckss/benchmarks/full_cmg_spike/submit_scc_smoke.sh \
  20260826T002500Z-alpha-direct-fused-5ea043e
```

The compact machine-readable receipt, including source identities, scheduler
status, ratios, scientific gates, and archived evidence hashes, is
[`scc_direct_fused_2026-08-25.json`](scc_direct_fused_2026-08-25.json).
