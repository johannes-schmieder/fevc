# CZ18 P200 MATLAB-harness failure

The first source-bound P200 retry at private probe inner tolerance `1e-9`
proved that the VCkss candidate clears the unchanged public `1e-5` complete
residual gate.  It did not produce an A/C/MATLAB comparison because the shared
MATLAB driver retained a stale P20-only assertion and stopped before calling
the maintained estimator.

## Bound run

- VCkss source: `97e89130f8ecf2d24a94eb2248da9aa81bb13abd`
- baseline: `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- standalone CMG: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- SCC job: `7314873`, `failed=0`, `exit_status=1`
- run: `/projectnb/welfgr/fevc/runs/20260826T024500Z-alpha-cz18-p200-inner1e9-97e8913`
- task SHA-256: `31e17267191097591ce2c497b22548cc9e96502da0adac5fc9ad1f1468bacea9`

## Useful result retained

| Route | Complete command (s) | Native solve (s) | Native total (s) | Peak RSS (KiB) |
|---|---:|---:|---:|---:|
| A, baseline | 178.491 | 118.6685 | 161.2613 | 2,735,132 |
| C, direct full-CMG | 74.851 | 17.6221 | 57.7673 | 2,734,304 |

The candidate completed 21 iterations, reported maximum complete original-
system residual `1.19218677221e-6 < 1e-5`, zero target-identity residual, and
restored data, RNG, and sort RNG state.  These measurements remain a useful
candidate checkpoint, but there is no MATLAB timing and therefore no
performance-promotion decision from this run.

## Failure and repair

MATLAB stopped with
`kss:cmgMata1Cz18Matlab:Randomness`: `CZ18 comparison requires P20 and seed
8675309.`  The private wrapper and validator already admitted only P20 or P200;
the shared MATLAB driver did not.  The narrow repair is to admit exactly those
two probe counts while retaining the fixed seed and all sample, source,
process-tree, four-worker, accounting, and output checks.  No scientific gate
is relaxed.

The machine-readable evidence is
`cz18_p200_matlab_harness_failure_2026-08-25.json`.
