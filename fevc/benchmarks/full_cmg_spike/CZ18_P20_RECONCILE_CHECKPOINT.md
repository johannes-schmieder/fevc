# CZ18 P20 reconciliation checkpoint

Status: **green implementation smoke; not a performance promotion result**.

The fixed CZ18 P20 job verified the private direct full-CMG path after the
compressed V7 reconciliation repair. The scheduler, wrapper, Stata baseline,
Stata candidate, maintained MATLAB comparator, process-tree monitor, and final
qacct-aware validator all passed. The restricted retained sample stayed on SCC.

## Source and command

- VCkss source: `eee4acc76902bcde0f7c1f1141e860ded7244fef`
- Reconciliation repair: `83d22846e9c835590fa293b48058ba7991ccf446`
- Baseline: `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- Standalone CMG: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- SCC job: `7314745`, `scc-tb4.scc.bu.edu`, project `welfgr`, 14 slots,
  four application threads
- Submission command:

```bash
env VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  fevc/benchmarks/full_cmg_spike/submit_scc_cz18_smoke.sh \
  20260826T013000Z-alpha-cz18-reconcile-eee4acc 20
```

The candidate used effective probe tolerance `1e-6`, the pre-registered private
inner tolerance `1e-8`, and the unchanged complete original-system residual
limit `1e-5`.

## Result

| Route | Command seconds | Native solve seconds | Peak RSS KiB |
|---|---:|---:|---:|
| VCkss baseline | 108.570 | 33.733 | 2,737,448 |
| VCkss direct full-CMG | 91.596 | 15.443 | 2,742,984 |
| Maintained MATLAB KSS | 47.154 | — | 4,277,308 |

The candidate was 15.6% faster than the old VCkss baseline but 1.94 times the
MATLAB command time. Its maximum complete original-system residual was
`7.99266646976e-6`; the target identity residual was
`1.38777878078e-17`. All four common-probe corrected-target gates passed by a
wide margin (maximum used share `0.000205`), and data, RNG, and sort-RNG state
were restored.

This is only a P20 implementation smoke. The MATLAB corrected-target comparison
is explicitly descriptive for this single seed, so neither statistical
promotion nor a performance claim follows from it. The registered P200
single-run decision is the next bounded step.

The compact machine-readable receipt is
`cz18_p20_reconcile_2026-08-25.json`. The final remote validator receipt is at
`/projectnb/welfgr/fevc/runs/20260826T013000Z-alpha-cz18-reconcile-eee4acc/receipts/validation.json`
with SHA-256
`78cc714fa66199f2b6e5b08500051de43ed49e454530e5cc8260f60cd83c1c04`.
