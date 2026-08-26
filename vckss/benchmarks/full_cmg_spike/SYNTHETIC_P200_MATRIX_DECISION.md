# Synthetic P200 alternating-matrix decision

## Decision

The private direct full-CMG route is scientifically acceptable and materially
faster than both the frozen VCkss baseline and maintained MATLAB on the
registered synthetic hard case, but it does not clear the alpha performance
or memory gates. Across five position-balanced warm runs on one SCC host,
median complete command time is 123.633 seconds for VCkss and 183.017703
seconds for maintained MATLAB R2025b. VCkss is 1.4803 times as fast, whereas
the registered two-times-MATLAB target requires no more than 91.5088515
seconds. Candidate maximum peak RSS is 4,134,164 KiB versus MATLAB's
3,847,076 KiB.

The route therefore remains private and is not hardened, vendored, exposed
through `backend(auto)`, tagged, or promoted to an alpha candidate. The
fixed-CZ18 matrix remains a valid 2.0667-times-MATLAB checkpoint, but both
registered hard cases had to pass. This source-bound bottleneck report is the
required outcome when one case misses; a benchmark PDF is deliberately not
created from a non-promoted route.

## Implemented experimental architecture

`CMG_FULL_SPIKE_V1` directly solves VCkss's existing exact hybrid Laplacian.
For every transformed firm-Schur RHS, it places firm values on the firm
vertices, zeros auxiliary worker vertices, solves with the shared standalone-
CMG `ParallelPcgSolver`, deterministically extracts and recenters firm effects,
recovers worker effects with VCkss's existing logic, and independently checks
the complete original worker-plus-firm residual. One graph, official CMG
hierarchy, plan, Rayon pool, and admitted four-workspace pool serve the fit,
leverage, and target batches. Parallelism is across independent RHSs without
nested parallelism, and fixed output order is preserved.

The renewed route also uses packed Counter-V1 generation, direct contiguous
RHS construction, parallel recovery/certification, raw-match fast preparation,
and linear-time planning. Fused-f64, mixed-precision, and scalar pass-fusion
variants were separately measured, preserved, and disabled because they did
not meet their registered speed/memory gates. The route remains behind private
experimental consent and does not advertise a public backend capability.

## Source and execution identity

- SCC job: `7318114`, `failed=0`, `exit_status=0`
- host: `scc-gr4.scc.bu.edu`
- benchmark and candidate source:
  `787327f61eb35c2f61b400ed6d288c131bd0c919`
- frozen baseline source:
  `4124b34f3ca216dcc3aae27e4b31bbac9e011f11`
- standalone CMG source:
  `dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10`
- run directory:
  `/projectnb/welfgr/vckss/runs/20260826T153000Z-alpha-synthetic-p200-matrix-787327f`
- input SHA-256:
  `19744b8418ffff82461527d7426cdc976dce18bf36597555d19bf847dec5afbe`
- task SHA-256:
  `0bf2b1de2aa5c9d6177fde822879ac325abcc1344fb2d95204140aa08aabde0e`
- validation SHA-256:
  `b08c3e923d54c6ddd680e2aef16a051763a13200db611973f9d33bb9e8eee2ad`
- validator SHA-256:
  `229e346a6bfcc53ae41d02115fe131acfc6f06596d5f4a547cd9e00b2a0ebd5b`
- qacct SHA-256:
  `d0b03171ce1b134912f56235e334f05766d2113601408e5888e36405e13dc32c`
- maintained-MATLAB source-identity receipt SHA-256:
  `a5c5be72245f9e2269f86a7065650c40c85c4de5705666ac93ea263e965f67a6`

The case has 1,966,080 literal match rows, 327,680 workers, 8,192 firms,
degree six, and 200 probes. Every application used a fresh process, four
application threads or MATLAB pool workers, the same generated input, and the
same position-balanced cold-plus-five-warm protocol. Candidate C used the
private raw-match fast-preparation path, public fit/probe tolerances `1e-10`
and `1e-6`, and private inner tolerances `1e-12` and `1e-6`. The job reserved
14 SCC slots and a 56-GiB whole-job envelope for the sequential matrix.

## End-to-end timing

| implementation | cold seconds | warm seconds | warm median | relative to MATLAB |
| --- | ---: | --- | ---: | ---: |
| A: VCkss baseline | 487.362 | 490.823, 489.970, 489.852, 490.986, 490.209 | 490.209 | 2.6785x |
| C: private direct full CMG | 123.081 | 121.264, 129.251, 123.590, 123.633, 124.020 | 123.633 | 0.6755x |
| maintained MATLAB KSS | 182.981924 | 182.573367, 188.633390, 185.062202, 182.870494, 183.017703 | 183.017703 | 1.0000x |

C is 3.9650 times as fast as A and 1.4803 times as fast as MATLAB. This is a
substantial end-to-end result, but it is 32.1241485 seconds above the
registered two-times-MATLAB ceiling. It requires a 25.98% reduction in the
current complete command, not a rounding or tolerance-policy change.

The matrix did not run the preconditioner-only B lane. The direct prepared
hybrid solve was the registered high-reward route, and B remained a bounded
diagnostic option only.

## Numerical and state gates

All scientific and state gates pass in all six rounds:

- the maximum candidate complete original-system residual is
  `6.97125921276e-6`, below the unchanged `1e-5` probe threshold;
- all four common-Counter-V1 corrected targets pass the active statistical
  gate; the largest A/C difference is `1.78450587640e-11`, and the largest
  difference-to-limit ratio is `0.0008081`;
- baseline and candidate corrected targets repeat exactly across rounds;
- all application, caller data, RNG, sort-state, MATLAB process-tree,
  wrapper, and scheduler-accounting checks pass; and
- the independent target-accounting identities pass.

No numerical tolerance, estimator definition, residual requirement, RNG
semantics, or lifecycle check was weakened to obtain the timing.

## Memory decision

| implementation | median peak RSS | maximum peak RSS |
| --- | ---: | ---: |
| A: VCkss baseline | 5,393,976 KiB | 5,408,408 KiB |
| C: private direct full CMG | 3,951,356 KiB | 4,134,164 KiB |
| maintained MATLAB KSS | 3,773,636 KiB | 3,847,076 KiB |

C uses much less memory than A, but its median peak is 177,720 KiB (4.71%)
above MATLAB and its maximum is 287,088 KiB (7.46%) above MATLAB. It
therefore also misses the registered `candidate peak RSS <= MATLAB peak RSS`
gate. The representative private setup admits 1,007,900,328 bytes, including
175,616,208 hierarchy bytes, 102,618,492 plan bytes, and an 87,529,280-byte
four-workspace pool. Scheduler `maxvmem=53.121G` is the peak for the entire
sequential 14-slot job and is not a per-application RSS observation.

## Dominant bottleneck

The official full-CMG repeated solve is unequivocally dominant. Its warm
median is 98.664913 seconds across 601 right-hand sides, or 0.164168 seconds
per RHS and 79.80% of the complete command. The 600 probe solves each take
eight PCG iterations; the deterministic fit takes 17, for 4,817 total
iterations, 4,817 preconditioner applications, and 5,418 operator
applications. Median RHS construction/layout is 3.865007 seconds and median
extraction/recentering/recovery is 5.497490 seconds. Candidate native total is
121.831095 seconds, so Stata and boundary overhead is no longer the decision
bottleneck.

Even deleting every phase outside the official repeated solve would leave
98.665 seconds, still above the 91.509-second two-times-MATLAB ceiling. If all
other time stays fixed, the repeated solve must fall to about 66.541 seconds,
a 32.56% reduction. The next work must therefore change the cost or
convergence of the official full-CMG solves; another small preparation,
Counter-V1, extraction, or simplified-hierarchy optimization cannot close the
gap.

Beating MATLAB remains plausible: the current route already wins end to end
by 1.4803x and the fixed-CZ18 matrix clears 2x. A decisive synthetic win is not
plausible from the remaining non-solver phases, however. It requires either a
materially better official hierarchy for this graph family or a large reduction
in the cost of each official CMG/PCG application.

## Shortest route to a performance winner

1. Pre-register a small deterministic tuning matrix for standalone CMG's
   `direct_threshold`, `max_hierarchy_nnz_factor`, and
   `low_effective_degree_threshold` on this synthetic development case.
   Report hierarchy size, setup, applications, iterations, solve time,
   complete residuals, corrected targets, and process RSS; freeze the choice
   before returning to CZ18.
2. If tuning cannot remove roughly one third of repeated-solve time, profile
   the official `ParallelPcgSolver` inside the full estimator and optimize
   only its measured preconditioner, quotient-centering, norm, dot-product,
   or matvec costs. Preserve independent per-column PCG, fixed reduction
   order, four-way coarse RHS concurrency, and no nested parallelism. Do not
   revive the simplified hierarchy or the disabled fused-f64, mixed-precision,
   and scalar pass-fusion experiments without new evidence.
3. Couple the speed experiment to a memory plan that reduces maximum
   application RSS by at least 287,088 KiB, starting with hierarchy/plan size
   and workspace-pool lifetime rather than weakening whole-process admission.
4. Repeat this same six-round synthetic matrix first. Only after both the
   `<=0.5` timing ratio and MATLAB peak-RSS gates pass should the winning
   tuning be rerun on CZ18 and enter vendoring, Rust 1.85 qualification,
   interruption/memory hardening, public parity, default-auto, and benchmark
   PDF work.

The compact machine-readable decision is
[`synthetic_p200_matrix_2026-08-26.json`](synthetic_p200_matrix_2026-08-26.json).
The complete hashed evidence remains in the restricted SCC run directory.

## Commands

Submission from clean source used:

```bash
env VCKSS_CMG_ROOT=/Users/johannes/Git/CMG \
  ./vckss/benchmarks/full_cmg_spike/submit_scc_synthetic_matrix.sh \
  20260826T153000Z-alpha-synthetic-p200-matrix-787327f
```

After successful scheduler accounting became available:

```bash
python3 /projectnb/welfgr/vckss/runs/20260826T153000Z-alpha-synthetic-p200-matrix-787327f/source/candidate/vckss/benchmarks/full_cmg_spike/validate_scc_synthetic_matrix.py \
  --run-dir /projectnb/welfgr/vckss/runs/20260826T153000Z-alpha-synthetic-p200-matrix-787327f \
  --source-commit 787327f61eb35c2f61b400ed6d288c131bd0c919 \
  --job-id 7318114 \
  --output /projectnb/welfgr/vckss/runs/20260826T153000Z-alpha-synthetic-p200-matrix-787327f/receipts/validation.json
```
