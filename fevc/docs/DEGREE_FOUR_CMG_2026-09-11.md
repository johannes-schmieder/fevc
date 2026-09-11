# Exact degree-four elimination in full CMG

## Implementation and scope

The full-CMG route now eliminates workers connected to four distinct firms by
their exact weighted Schur clique: each firm pair receives `w_i*w_j/sum(w)`.
Workers of degree two and three retain their prior elimination, and workers
of degree five or more retain an auxiliary star. The generic CMG constructor
and inference routes retain their degree-three threshold.

Only `rust/crates/vckss-core/src/cmg_impl.rs.in` and `full_cmg.rs` change
production behavior. The prebuild edge upper bound becomes the checked
`ceil(1.5*cells)`, since four cells can contribute six clique edges. The
existing constructed-memory reconciliation remains mandatory. This larger
conservative bound is not a prediction of process RSS. An omitted memory
budget still causes no memory-based planning or admission rejection.

Phase tolerances, the complete original-system residual certificate, Counter
draws, refinement schedule, requested batch semantics, worker count, solver
selection, typed failures, cancellation and native lifecycle are unchanged.
No new public option, ABI, dependency, upstream hierarchy or stopping rule is
introduced. Full-CMG identity remains `CMG_FULL_V2`.

## Selected SCC development evidence

Source is baseline `0010ec21f8c74acac9908af1352e50b26cf32beb` plus the frozen
degree4-pure patch in `rust/experiments/performance_20260911/stage2-payload/`.
All comparisons use literal accepted inputs, omitted tolerance and memory
options, and 28 workers. Estimators run sequentially on one host per graph;
the 1.6m cells rotate order across three repetitions. The 6.4m cells have one
repetition each and are scaling checks, not stable median estimates.

| Observations | Mobility | Baseline FEVC | Degree4-pure | KSS Matlab | Paired Matlab speedup |
| --- | --- | ---: | ---: | ---: | ---: |
| 1.6m | Well-mixed | 67.123 s | 14.564 s | 84.873 s | 5.83× |
| 1.6m | Segmented | 68.905 s | 15.166 s | 87.703 s | 5.77× |
| 6.4m | Well-mixed | 382.072 s | 130.494 s | 401.117 s | 3.07× |
| 6.4m | Segmented | 451.655 s | 142.971 s | 440.870 s | 3.08× |

The first two rows report command medians and the median of within-repetition
speedups; the last two report individual command times and paired ratios.
The 6.4m speedups versus baseline FEVC are 2.93× and 3.16×. This isolates exact
degree-four elimination, without queue or batch changes. It is evidence of a
substantial win in these selected mobility graphs, not a universal speedup or
replacement for the full paper figures.

All **24 calls** across accepted jobs **7522857.1–2** and **7523091.3–4** pass
source/input/file, scheduler, application, residual, sample, seed, resource and
target gates. Both arrays have complete `failed=0`, `exit_status=0` accounting.
The largest four-target FEVC common-draw gap is **1.276e-9**, below even the
registered deterministic `1e-8*max(1,abs(a),abs(b))` floor. Matlab retains its
independent RNG and descriptive-comparator semantics from the accepted grid.

At 6.4m, sampled whole-process-tree RSS is 13.59 / 14.01 GiB for the candidate,
14.68 / 15.35 GiB for baseline FEVC, and 99.73 / 102.31 GiB for Matlab.
Scheduler `maxvmem` is virtual memory, not physical RAM. The original 1.6m
attempts stopped at an incorrectly low Matlab harness RSS guard; they remain
archived. The owner-approved guard-only retry used 72 GiB for Matlab with the
same 28 × 3G request. Large checks used the original 28 × 6G request and
140 GiB guard, each with a 45-minute cap.

## Source compatibility and local checks

The working-source implementation differs from the frozen SCC degree4-pure
source only by formatting of the checked edge-bound closure, an updated
scope comment, and one additional unit test. Production arithmetic and
control flow are unchanged. The test checks edge-bound rounding and typed
overflow; the frozen independent weighted Schur test covers degrees 2–5.
SCC input, binaries, manifests, tolerances and acceptance receipts are not
rewritten. Performance claims remain bound to those SCC binaries; local Mac
checks qualify their newly built local artifacts separately.

The real-checkout Python suite passes **800 tests** (with pre-existing pytest
temporary-directory cleanup warnings). The complete Rust workspace suite
passes, with its one registered diagnostic still ignored; the additional
degree-four edge-memory regression passes separately. Strict Clippy,
formatting and CMG generated-source checks pass. Source-local native and Stata
qualification passes on arm64 and Rosetta, including signed thin/universal
artifacts, C/ABI checks, lifecycle, public routes and clean installation.
The tested three Mac plugins are staged in the local package; earlier binaries
are backed up. All 156 qualifier source-file hashes and staged candidate
hashes were independently rechecked after completion. The integrated package
check also passes: static/source audits, 800 Python tests, CMG component gates,
Stata quick/full suites, clean installation and the small benchmark/bridge
smokes. Its supervised process exits zero without timeout (1,044.441 seconds).
These checks were run on an uncommitted local checkpoint, not an exact-commit
qualification or release. A subsequent source commit does not retroactively
change the tested-source identity or manufacture a clean-SHA receipt.

Native source manifest:
`a658fc250b4f938aad5643541f944aa0ba95e581b65ce053c49378705d36910c`.
Native receipt:
`f2f2ec7f835ee139fe329141189848ffb6fee65a1efbf9bcd54a46d0c0d9f50b`.
The receipt and sanitized transcripts are preserved under experiment
`qualification/`, together with `source-compatibility.json` and prior plugins.
The consolidated `qualification/final-checkpoint.json` has status
`PASS_LOCAL_DEVELOPMENT` and SHA-256
`356ad2858795566d2e55784da9751e8d7b3525dd6a635263765730f2db485dc5`.
No performance jobs, local tests or monitor remain pending.

## Other candidates and evidence locations

The separate 96-call, 400k scheduling screen covers 4/7/14/28 threads.
Queue-only gains about 1–7%; smaller aligned batches regress in seven of eight
graph/thread cells and are not integrated. Arbitrary-thread scheduling tests
also cover odd counts, tails and counts through 64. Existing batching remains
in the ordinary runtime until composition with degree-four reduction is shown
to help.

Captured-system upstream replays find main CMG nearly flat, optional scalar
kernels about 2–5% faster, and fused four-RHS lanes about 2.1–2.9× faster in
the tested solver-only workload. The cancellable fused-kernel port remains
experimental and is not wired into native FEVC. These gains cannot be assumed
additive after graph reduction; no wholesale upstream update is promoted.

SCC root: `/projectnb/welfgr/vckss/runs/performance-20260911-v1`.
Durable development artifacts: `rust/experiments/performance_20260911/`, with
protocol, handoff and reports in `local-study/`, exact source in
`stage2-payload/`, and audit/receipt/log evidence in `scc-evidence/`.
Accepted audit hashes:

- `guard72-audit.json`: `fb2dc89aa59e9dbd16ad0c6eba6329c3edbeed87ea8b992d8fbc8a6efa387f25`.
- `large-audit.json`: `fe826a5040fa72e3e7b3775ef4080e77793401413be022ad3f9ea4b5021b835a`.
- Frozen stage-two manifest: `5f73e91396cbc677028e8a0051631f660cec8adfa29f24a8e0b810da86941ad9`.

The paper, upstream CMG checkout and unrelated SCC jobs are unchanged. After
completion, the owner authorized committing and pushing the degree-four
implementation and documentation on September 11. Experimental sources and
machine-local evidence remain outside that commit. No full publication
campaign, Windows gate, tag or release is part of this development pass.
