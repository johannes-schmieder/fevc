# SCC AKM projection continuation evidence

This directory contains compact evidence for the staged AKM projection timing
continuation at exact source
`30d49fda876597782cca22adf36094993f7a6212`.

- SCC run: `/projectnb/welfgr/vckss/runs/20260830T2006Z-projection-akm-censor-30d49fd`
- preparation job: `7373800` (clean accounting and passing preparation receipt)
- 6,000-row gate job: `7374823` (clean accounting and passing exact/Rust/MATLAB gate)
- 480,000-row array: `7374830`, tasks 1 and 2
- input SHA-256: `fd4aa39f8b02bba9f9700e57663993d23cb784c0e2bcb0eb6fd0f3c5e68aafc1`

## Scheduler accounting

| Task | Application cores | Order | `failed` | `exit_status` | SGE wall | SGE `maxvmem` |
|---:|---:|---|---:|---:|---:|---:|
| 1 | 4 | Rust, MATLAB | 0 | 0 | 10,238 s | 47.333 GiB |
| 2 | 16 | MATLAB, Rust | 0 | 0 | 10,021 s | 167.066 GiB |

The full-array accounting collector was run once after both tasks left
`qstat`. Its receipt reports complete scheduler accounting and
`COMPLETE_NONPASS` with two task statuses of `FAIL`.

## Scientific outcome

| Cores | VCkss outcome | VCkss diagnostic wall | MATLAB outcome | MATLAB diagnostic wall |
|---:|---|---:|---|---:|
| 4 | `FAIL/PCG_MAXITER` | 9,733.567 s | `RIGHT_CENSORED/MATLAB_FIT_NONCONVERGENCE` | 492.664 s |
| 16 | `FAIL/PCG_MAXITER` | 9,719.729 s | `RIGHT_CENSORED/MATLAB_FIT_NONCONVERGENCE` | 297.322 s |

Both VCkss cells exhausted the registered 40,000 model-PCG iterations at the
same reduced residual, `4.412307557090175e-10`, above the unchanged `1e-10`
gate. The recorded VCkss walls are failed-run diagnostics, not accepted
completion times. The MATLAB failure receipts bind the source, input, rows,
and cores to `vckss:projectionAkm:Fit` with `Grounded fit did not converge.`
Their observed walls are likewise diagnostic: no MATLAB completion time is
imputed and no paired ratio is computed.

The continuation therefore supplies no accepted 480,000-row timing or parity
result. The 1,920,000- and 7,680,000-row stages were not submitted, and no
paper performance claim is changed.

## Files

- [`collection/receipts/feasibility-480000.json`](collection/receipts/feasibility-480000.json): terminal stage receipt
- [`collection/receipts/feasibility-480000-qacct.txt`](collection/receipts/feasibility-480000-qacct.txt): full-array scheduler accounting
- [`collection/receipts/summary.json`](collection/receipts/summary.json): failure-preserving aggregate
- `collection/tasks/feasibility-480000/task-{1,2}/validation.json`: task identities and role outcomes
- `collection/tasks/feasibility-480000/task-{1,2}/{rust,matlab}/`: application, failure, timing, memory, and process receipts
- [`collection/gate/validation.json`](collection/gate/validation.json): passing 6,000-row gate
- [`collection/source/SOURCE_COMMIT.txt`](collection/source/SOURCE_COMMIT.txt): tested source identity
