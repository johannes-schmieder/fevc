# Direct full-CMG architectural decision

Status: **reject direct full CMG for promotion**. SCC job `7306628` completed
with `failed=0` and `exit_status=0`. This is a private performance-spike
report, not release qualification or a public-backend claim.

## Architecture implemented

`CMG_FULL_SPIKE_V1` directly solves the existing VCkss exact hybrid
Laplacian. For every transformed firm-Schur RHS, it places the firm values on
firm vertices, zeros the auxiliary worker vertices, solves with one shared
standalone-CMG `ParallelPcgSolver`, deterministically recenters the extracted
firm solution, recovers worker effects with existing VCkss logic, and applies
VCkss's independent complete original worker-plus-firm residual gate.

One graph, hierarchy, Rayon pool, parallel plan, and admitted workspace pool
are reused across outcome, leverage, and target batches. Parallelism is only
across independent RHSs; output ordering is fixed and there is no nested
parallelism. The spike is available only with private environment consent and
does not change the public capability or ABI. It links exact standalone CMG
source `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10` with Rust 1.85 while
ordinary VCkss continues to parse and build on Rust 1.81.

## Source and commands

Key commits, in implementation order:

- `0f8cb187`: preserve the superseded generic-batch scaffold;
- `1a8ea37`: private direct hybrid batch implementation;
- `8026922`: retained receipt-mismatch diagnostic;
- `1d320f8`: aggregate setup/batch/iteration/residual/memory diagnostics;
- `7d34df2`: registered local A/C/MATLAB harness;
- `e79f7ef` through `1e3a412`: batch-launch, log, isolated-MATLAB,
  planned-route, and source-binding repairs;
- `92b4038` and `fabae89`: bounded `1e-15` parity diagnosis and restoration of
  the measured `1e-14` private setting;
- `9a2dd30`, `61eb0de`, and `7d84536`: source-bound SCC smoke, locked crate
  resolution, and registered Linux plugin name;
- `e3e5bb8` and `bc92b96`: Stata test isolation and SCC deployment
  regressions.

The exact local decision command was:

```bash
./.venv/bin/python vckss/benchmarks/full_cmg_spike/run_local.py \
  --output-dir /private/tmp/vckss-full-cmg-local-1e3a412 \
  --baseline 4124b34f3ca216dcc3aae27e4b31bbac9e011f11 \
  --baseline-plugin /private/tmp/vckss-baseline-4124-exact/plugin/vckss_rust_macos_arm64.plugin \
  --candidate 7d34df221fdf1a13f868ab9606a1de4ab6bf940f \
  --candidate-plugin /private/tmp/vckss-full-cmg-7d34df2/candidate/vckss_rust_macos_arm64.plugin \
  --candidate-build-receipt /private/tmp/vckss-full-cmg-7d34df2/build-receipt.txt \
  --matlab-root /Users/johannes/Git/varcomp_hdfe/Monte_Carlo/LeaveOutTwoWay \
  --matlab /Applications/MATLAB_R2024b.app/bin/matlab
```

The accepted SCC submission command is:

```bash
VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  vckss/benchmarks/full_cmg_spike/submit_scc_smoke.sh \
  20260825T120500Z-1dd5fa56-fcmg4-smoke
```

It submitted scalar SGE job `7306628` with project `welfgr`, four `omp`
slots, 16 GiB per slot, and no GPU. Exact archived sources are VCkss
`1dd5fa5c28fb93a3e045f659fa03d9470a6be153`, baseline `4124b34`, and CMG
`dbefbc5`; all applications share one checksum-bound input on one compute
node. The scheduler recorded host `scc-h30`, 868 seconds of job wall clock,
four granted slots, and successful accounting.

## Timing and numerical evidence

The registered case has 1,966,080 literal match rows, 327,680 workers, 8,192
firms, degree six, 200 Counter-V1 probes, and input SHA-256
`19744b8418ffff82461527d7426cdc976dce18bf36597555d19bf847dec5afbe`.
The local run is one cold architectural diagnostic on an Apple M2 Ultra Mac
Studio with 192 GB RAM, Stata/MP 18, and four requested processors. The SCC
run is the accepted one-node A/C/MATLAB decision experiment on `scc-h30` with
four requested and used workers. Neither table is a warm-median claim.

| Lane | Complete command | Native solve | Peak RSS | Iterations | Complete residual | Scientific status |
|---|---:|---:|---:|---:|---:|---|
| A: VCkss `4124b34` | 230.000 s | 218.795 s | 7.30 GB | 12 max | `4.64e-11` | individual gates pass |
| B: full CMG inside outer PCG | not run | not run | not run | not run | not run | diagnostic lane was unnecessary |
| C: direct full-CMG hybrid | 201.592 s | 190.671 s | 5.03 GB | 21 max | `6.61e-14` | A/C gate fails in two fields |
| Maintained MATLAB R2024b | no timing | no timing | 0.38 GB before stop | n/a | n/a | startup infrastructure failure |

C/A is `0.876487`, a 12.351% cold end-to-end improvement and a 1.141x
speedup. The maintained MATLAB 184-file runtime passes its exact source/hash
contract, but the local MATLAB process did not create process identity,
scratch, pool workers, or output during 952 seconds and was stopped. This is
an infrastructure failure, not a MATLAB timing observation. The registered
warm matrix was not launched after the scientific gate failed.

The accepted same-node SCC result is the architectural decision:

| Lane | Complete command | Native solve | Peak RSS | Iterations | Complete residual | Decision status |
|---|---:|---:|---:|---:|---:|---|
| A: VCkss `4124b34` | 329.260 s | 311.446 s | 5.61 GB | 12 max | `4.64e-11` | individual gates pass |
| B: full CMG inside outer PCG | not run | not run | not run | not run | not run | unnecessary diagnostic lane |
| C: direct full-CMG hybrid | 223.232 s | 204.518 s | 4.90 GB | 21 max | `6.61e-14` | A/C gate fails in two fields |
| Maintained MATLAB R2025b | 171.733 s | not separately reported | 12.09 GB process tree | n/a | n/a | comparator identity passes |

C/A is `0.677981`: C is 32.202% faster than A, a 1.475x speedup. C/MATLAB
is `1.299880`: C is 29.988% slower than MATLAB. Equivalently, MATLAB is
23.070% faster than C. MATLAB's four-worker process identity, exact source
contract, input checksum, sample structure, and run marker all pass. Its
scientific values remain descriptive because its RNG, solver tolerance, and
finite-projection correction are not the VCkss contracts.

Candidate C emits exactly 15 batch receipts covering 601 RHSs. Aggregate
private diagnostics are:

| Quantity | Value |
|---|---:|
| Graph construction | 0.418 s |
| Solver/hierarchy construction | 0.133 s |
| RHS construction/layout | 1.720 s |
| Direct CMG solve | 145.485 s |
| Extraction/recentering/recovery | 16.826 s |
| Total iterations / preconditioner applications | 12,621 / 12,621 |
| Total operator applications | 13,222 |
| Maximum concurrency | 4 |
| Graph / hierarchy / plan bytes | 34,144,256 / 175,616,208 / 102,618,492 |
| Workspace each / pool bytes | 21,882,320 / 87,529,280 |
| Private admitted peak | 1,007,900,328 bytes |

On SCC, graph and solver construction took 0.658 and 0.270 seconds; RHS
construction/layout took 2.353 seconds, the direct CMG solves took 153.432
seconds, and extraction/recentering/recovery took 11.474 seconds. The same 601
RHSs required 12,621 iterations and preconditioner applications and 13,222
operator applications in 15 batches, with maximum concurrency four. The
direct solve alone was 68.7% of C's complete command time.

The unchanged `2e-12` absolute-or-relative A/C gate fails only for plug-in
covariance (`-2.22150e-12`) and corrected covariance (`-2.22189e-12`). An
independent candidate built at private inner tolerance `1e-15` reduced its
maximum complete residual to `4.46e-15` but returned the same covariance gap
and slowed the command to 220.975 seconds. This diagnoses a stabilized direct
solution versus the looser baseline solution, not inadequate C convergence;
the gate is not weakened.

The local compact receipt hashes are:

- baseline row: `3b6d83a0b0469cb74c3aa3987f3c932f5024006b7a4b252f3d7850c020c41bd5`;
- candidate row: `695aea0226c9c56688dbafc8d214923223a9d7a853831d91695b9fb03f59d620`;
- baseline resources: `b1bc8b1c5fddd96dd115a5365668e86fd1eff8587b6cda6637142895965bf14e`;
- candidate resources: `0473ac4f23cd76cef95db2b73f258b8e50fef00bc6b71345ea41b228a0750b3a`;
- MATLAB source identity: `a5c5be72245f9e2269f86a7065650c40c85c4de5705666ac93ea263e965f67a6`;
- `1e-15` diagnostic row: `7b3a7ac98a5c8107b5babba31b8251f013674e18295b9d282889c8924d9b0fa4`.

The accepted SCC compact evidence hashes are:

- baseline and candidate result rows: `600396f08cd261e715f0f87f53d19d3b08f07f8c7f609fcf8d9199c6646dfcab`
  and `af74a6a92e4fe3246f7200e1591d6a105c140855d6dee72eeacbc13991991952`;
- MATLAB aggregate and process tree: `cbd1ff30fa5e071c5789ada7e2cdc9d673c5baac968768799fa0805a25b62743`
  and `78ab561473543bcff2fd01343352c68f6793db974124b601703aa3fe953f23f3`;
- node and scheduler accounting receipts: `eb84fbe6248aaa341aaff27e58f2e8408a43dfd2da13cb23da87aacadbd1d99e`
  and `3e94cde3226fe0073b397391b7d8a2304176c29cafb4152336f563208aaa2425`;
- wrapper pass and combined SGE log: `d6c0161c8d013e94757c6189882af1782971d6f3c9ad7c4e3612493729d5b226`
  and `5b0c5b2ef44d4412e07a4485c9eeb8ce26822db933e3dd04662cd4178692ddb7`.

## Decision and bottleneck

The dominant candidate bottleneck is the direct full-CMG repeated solve:
145.485 seconds locally and 153.432 seconds on SCC. Setup is immaterial. RHS
layout plus extraction/recovery is secondary, but even eliminating all of it
would leave SCC C at about 209.4 seconds, still slower than MATLAB's complete
171.7-second command and nowhere near the roughly 2x development target.

The current implementation is therefore rejected for promotion. It is faster
than matched A and more memory-efficient, but it neither clears strict A/C
parity nor beats maintained MATLAB end-to-end. The fixed CZ18 experiment and
warm qualification matrix were deliberately not launched because the
registered synthetic gate failed both prerequisites. The simplified embedded
hierarchy remains frozen; no public ABI, vendoring, cancellation, release
qualification, or benchmark-PDF work begins from this route.

## Shortest hardening path if a new architecture changes the decision

1. Explain and close the two-field A/C gap against an independent dense/direct
   reference without changing the `2e-12` gate or baseline scientific
   contract.
2. Profile the 153.432-second SCC direct CMG solve on the same full estimator;
   require a materially different repeated-solve architecture rather than
   hierarchy-setup tuning, then repeat the single A/C/MATLAB gate.
3. Only after a decisive accepted MATLAB win, pin and vendor CMG with license
   provenance, adopt Rust 1.85, and define versioned `CMG_FULL_V1` receipts.
4. Add conservative whole-process pre-RNG admission, bounded caller-thread
   UserBreak polling with worker cancellation, fixed scheduling/output order,
   exact thread/plan/workspace/iteration receipts, and existing exactly-once
   lifecycle/no-post-RNG-fallback gates.
5. Run macOS/SCC qualification and produce the registered compact benchmark
   data, figures, and visually verified PDF. Windows remains deferred.
