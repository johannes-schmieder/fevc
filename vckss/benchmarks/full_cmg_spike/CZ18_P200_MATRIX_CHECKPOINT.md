# Fixed CZ18 P200 alternating-matrix checkpoint

## Decision

The private direct full-CMG route clears the registered fixed-CZ18 development
target on SCC. Across five position-balanced warm runs on one host, median
complete command time is 33.942 seconds for VCkss and 70.147118 seconds for
maintained MATLAB R2025b. VCkss is 2.0667 times as fast and uses 58.7% of
MATLAB's median peak RSS.

This is a hard-case performance checkpoint, not alpha promotion or public
qualification. The separately registered alternating synthetic matrix remains
mandatory before hardening or exposing the route.

## Source and execution identity

- SCC job: `7317771`, `failed=0`, `exit_status=0`
- host: `scc-tb4.scc.bu.edu`
- benchmark source: `3daa465aa56b48d65457c626381f9600c55afdbc`
- baseline source: `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- standalone CMG source: `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- post-run validator repair: `5a6daaf7568e07def84cabf7dce14bdad9b2bfaf`
- run directory:
  `/projectnb/welfgr/vckss/runs/20260826T140330Z-alpha-cz18-p200-matrix-3daa465`
- input SHA-256:
  `1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`
- task SHA-256:
  `4cd7bc7479cb77d13dd70c208556a629b95b0198bd4ab47b88f9a8a949d20b0b`
- validation SHA-256:
  `97ade3ae790366ef8e8b883d2c6f0909b86897bbf12f31ff524c9ab238795d95`
- qacct SHA-256:
  `bb6b9dd2de27c0c547101655fb957e0952148aa4a3741613508d4347ca74484c`
- repaired validator SHA-256:
  `e8cc1dce9ab351ea9ef30be35d6b333540e6aed206627595d393d5d66d74787c`

The benchmark ran one cold round and five warm rounds. Every application used
a fresh process, four application threads or MATLAB workers, the same fixed
CZ18 sample, 200 probes, raw-match preparation, and the private `1e-9` probe
inner tolerance under the unchanged public `1e-6` effective probe tolerance.
The job reserved 14 SCC slots and 56 GiB for the whole sequential matrix.

The source-pinned validator originally stopped while parsing blank legacy
high-level phase fields. Those fields are optional and not emitted by this
route; all additive native phase fields and every hard gate were present. The
original failure is preserved. Commit `5a6daaf` removes only the inapplicable
phase fields from performance summarization, adds a validator self-hash, and
passes the full 453-test Python suite. That committed validator was copied to a
new receipt path and applied to the unchanged application, resource, task,
input, and qacct evidence.

## End-to-end timing

| implementation | cold seconds | warm seconds | warm median | relative to MATLAB |
| --- | ---: | --- | ---: | ---: |
| A: VCkss baseline | 269.149 | 255.124, 253.169, 254.351, 253.771, 253.210 | 253.771 | 3.618x |
| C: private direct full CMG | 36.467 | 40.281, 33.942, 35.814, 33.477, 33.848 | 33.942 | 0.4839x |
| maintained MATLAB KSS | 76.756578 | 76.677234, 74.592741, 68.980675, 68.152318, 70.147118 | 70.147118 | 1.0000x |

The matrix did not run the preconditioner-only B lane because the direct solve
was already the registered high-reward architecture and B was reserved for
diagnosis only.

## Candidate phases and repeated solves

The candidate warm-median native total is 24.758870 seconds. The official
full-CMG repeated solve remains the largest measured phase at 17.888371
seconds for 601 right-hand sides, or 0.0297643 seconds per RHS. Other median
native phases are 2.108681 seconds for compression, 1.627713 seconds for the
plan, 0.850820 seconds for graph construction, 1.920285 seconds for
canonicalization, and 0.407819 seconds for ingestion. The remaining difference
between complete command and native total is 9.183130 seconds and includes the
Stata/native boundary and non-native command work.

No further performance optimization is justified from this case alone. The
synthetic matrix decides whether the current route proceeds directly to
hardening or returns to profiling the measured end-to-end bottleneck.

## Numerical, state, and memory gates

- All complete original-system residual gates pass. The candidate maximum is
  stable at `1.19218677221e-6`, below the registered `1e-5` probe gate.
- All four common-probe corrected-target gates pass in all six rounds. The
  largest A/C absolute difference is `2.40082953518e-11`; its registered limit
  is `3.23087897272e-6`.
- Candidate and baseline corrected targets repeat exactly across rounds.
- All application, caller-state, MATLAB process-tree, wrapper, and qacct gates
  pass.
- Median peak RSS is 2,530,940 KiB for C, 2,732,984 KiB for A, and 4,310,024
  KiB for MATLAB. Maximum peak RSS is 2,537,304 KiB for C and 4,334,532 KiB
  for MATLAB.

The compact machine-readable summary is
[`cz18_p200_matrix_2026-08-26.json`](cz18_p200_matrix_2026-08-26.json). The
complete hashed evidence remains in the restricted SCC run directory.

## Commands

Submission from the clean benchmark source used:

```bash
env VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  ./vckss/benchmarks/full_cmg_spike/submit_scc_cz18_matrix.sh \
  20260826T140330Z-alpha-cz18-p200-matrix-3daa465
```

After successful qacct became available, the committed repair was applied to
the unchanged evidence with:

```bash
python3 receipts/validator-postrun-5a6daaf.py \
  --run-dir /projectnb/welfgr/vckss/runs/20260826T140330Z-alpha-cz18-p200-matrix-3daa465 \
  --source-commit 3daa465aa56b48d65457c626381f9600c55afdbc \
  --job-id 7317771 \
  --output receipts/validation.json
```
