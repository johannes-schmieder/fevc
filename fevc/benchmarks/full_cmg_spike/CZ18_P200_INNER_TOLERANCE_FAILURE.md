# CZ18 P200 `1e-8` inner-tolerance failure

SCC job `7314843` is a preserved scientific-gate failure, not a performance
result. It ran source `c1ae402c47227ac0adda6c4e301382c6c25c98cb` with 200
probes, effective public probe tolerance `1e-6`, and the P20 private inner
tolerance `1e-8`.

The old VCkss baseline passed in 144.907 command seconds. The private full-CMG
candidate stopped during target solves before producing a result receipt:

```text
FULL_RESIDUAL_FAILED [jla_target]: phase Target, probe Some(56), side Firm:
zero-based RHS column 49: complete residual 0.000015628428262466848
exceeds tolerance 0.000009999999999999999
```

SGE recorded `failed=0`, `exit_status=1`; the wrapper recorded
`stage=candidate_application rc=1`. MATLAB was not launched and no performance
ratio is valid. The complete original worker-plus-firm residual gate remains
`1e-5` and will not be relaxed.

The bounded next attempt pre-registers a probe-count-specific private inner
tolerance: P20 stays at `1e-8`, while P200 uses `1e-9`. The effective public
probe tolerance remains `1e-6`. Task, node, and validator receipts must agree
on this choice before the run can pass. The compact failure receipt is
`cz18_p200_inner_1e8_failure_2026-08-25.json`.
