# Requesting a stricter residual without changing CMG defaults

PCG's existing acceptance rule is

```text
atol + rtol * (||b||₂ + operator_norm_bound * ||x||₂).
```

This is a backward-error certificate. On ill-conditioned graphs, the solution
norm can make the accepted absolute residual large. `rtol = 1e-8` does not
mean `||b-Lx||₂ <= 1e-8 * ||b||₂`, and neither criterion alone guarantees a
relative coefficient error of `1e-6`.

The 64-vertex dyadic bridge regression has endpoint unit injections and a
bridge resistance of `2^24`. Its reference voltage is the centered cumulative
resistance, computed exactly in binary64 independently of the solver. The
original default-policy diagnostic failed the stronger gates identically on
upstream `90d06d5` and the ownership candidate: hierarchical relative maximum
coefficient error was approximately `2.51e-6`. That evidence remains valid;
the test had asked the default solver to meet a contract it does not promise.

For that fixture, explicitly request the independently frozen residual limit:

```rust
let options = cmg::PcgOptions {
    relative_tolerance: 0.0,
    absolute_tolerance: 1e-8 * std::f64::consts::SQRT_2,
    ..Default::default()
};
```

The enabled regression retains both the `1e-6` coefficient gate and the
`sqrt(2)*1e-8` independent current-divergence residual gate. A minimal-feature
serial test also checks the direct and multilevel paths; the component test
checks ordinary/component construction and one/four workers. The historical
`bridge-oracle` diagnostic intentionally continues testing the default policy.
Do not overwrite its failed evidence or treat diagnostic completion as an
accuracy pass. New test expectations are not evidence of a pass until executed.

No default, summation order, restart interval, certification check, or automatic
fallback policy is changed. Tight requests can cost iterations or return an
error at an arithmetic floor. Callers must still validate coefficients and
their downstream estimands against independent references. For an RHS-relative
request, compute a robust norm of the actual unprojected RHS and set a finite,
positive absolute limit accordingly; setting both tolerances to zero is invalid.
