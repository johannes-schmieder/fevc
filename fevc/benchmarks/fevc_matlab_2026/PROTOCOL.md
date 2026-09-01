# FEVC--MATLAB R2026a benchmark protocol

Status: registered before the first SCC preparation submission.  Earlier
benchmark receipts are immutable and are not pooled with this campaign.

## Scientific roles and exclusions

- The production comparison is source-bound FEVC Rust versus maintained
  LeaveOutTwoWay under MATLAB R2026a.
- Mata is restricted to the registered manual-style slice and small numerical
  oracles.  It is not run across the production matrix.
- No CZ18 or other restricted row-level data enter the campaign.  Public
  Veneto is the only real-data case.
- The three historical largest strong-degree-two, one-core MATLAB calls remain
  right-censored at 10,800 seconds and are `DO_NOT_SUBMIT`.
- Build, compilation, and MEX preparation are disclosed but excluded from
  estimator command time.

## Primary platform and main matrix

Primary jobs request one 28-core E5-2680v4 Broadwell node under `-P welfgr`,
eight GiB per slot, and at most eight hours.  Runtime receipts must establish
the host model, 28 granted slots, binding, socket topology, and node occupancy.
The registered application-core ladder is 1, 2, 4, 8, 14, and 28; no process
or child pool may exceed the selected count.

The main structures are `strong_d2`, `strong_d3`, `strong_d6`, and `weak_d3`.
At 28 cores the row ladder is 7,680; 30,720; 122,880; 491,520; and 1,966,080.
At 491,520 rows the full core ladder is used.  The overlapping 491,520/28 cell
is run once.  Six fixed seeds are used; three repetitions execute Rust then
MATLAB and three execute MATLAB then Rust.

The 240 cells are partitioned into 24 topology-by-repetition scheduler bundles.
Each bundle runs its ten cells sequentially on one host.  Rust and MATLAB use
the same literal input bytes and each role starts in a fresh process.
The inherited comparative-scaling sample contract is explicit match deletion,
joint nuisance handling, movers, observation-key ordering, no controls or
weights, and uniform stored-row targets.  All generated main-matrix workers
are movers, so spelling out `stayers(movers)` admits the frozen Rust route
without changing the retained sample or target population.

## Timing and memory

Primary timing is invocation-to-return estimator command time.  Secondary
fields include fresh-process wall, CPU, import, MATLAB pool/MEX setup, teardown,
plugin load, and internal FEVC phases.  Internal time from one implementation
is never divided by end-to-end time from the other.

The full role process tree is sampled every 0.10 seconds.  After a one-second
stabilization at each marker, the monitor records:

- `R_empty`: median RSS after the runtime and required worker pool start but
  before analysis data are loaded;
- `R_data`: median RSS after the identical literal input is loaded and before
  the estimator starts; and
- `R_peak`: peak RSS during the estimator phase.

Absolute runtime baseline, `R_data-R_empty`, `R_data`, `R_peak-R_data`, and
`R_peak` are primary memory outcomes.  `R_peak/R_data` is secondary.  Linux PSS
is recorded at the empty, loaded-data, estimator-start, and estimator-end phase
boundaries as a shared-page sensitivity.  SGE `maxvmem` is diagnostic only.

## Additional registered modules

1. Bottleneck continuum at 491,520 rows with bridge counts 4, 8, 16, 32, 64,
   and 128, holding graph dimensions and approximate degree fixed.
2. Fixed-graph observation expansion from 122,880 rows by factors 1, 2, 4, 8,
   and 16, with deterministic within-match outcome variation.
3. Fixed mover graph with stayer:mover ratios 0, 1/4, 1/2, and 1, preceded by
   an exact retained-sample and target-population audit.
4. Package-default versus harmonized JLA sensitivity for representative strong
   and bottleneck cells.
5. Mata/manual panels for strong d2, strong d6, and bottleneck: row scaling at
   four cores over 30,720, 122,880, and 491,520; core scaling at 122,880 over
   1, 2, and 4; three cyclic-order repetitions.
6. Projection: exact 1,002-row gate, row slice near 6k/24k/96k at 14 cores, and
   core slice 4/14/28 near 24k.  The historical 480k boundary is not resubmitted.
7. Public Veneto with at least ten balanced four-core repetitions and separate
   cold, warm, and memory-phase reporting.
8. Gold-6242 architecture sensitivity for representative strong d3 and
   bottleneck 491,520-row cells at 14 and 28 active cores, three repetitions.

Each module has an immutable manifest and is submitted only after the main
small and worst-case gates establish the resource envelope.  Semantic mismatch
with MATLAB prevents a performance ranking; it is retained as a scientific
result.

## Acceptance and recovery

Rankable calls require identical input and retained-sample hashes; registered
worker, firm, cell, deletion-unit, stayer, and target-population counts; finite
outputs; convergence; complete original-system residual acceptance; requested
route and thread receipts; complete phase markers; and valid memory monitoring.
Small cross-language oracles use a scaled `1e-8` tolerance.  Randomized target
differences are descriptive unless a registered independent MCSE gate applies.

Scheduler acceptance requires complete `qacct` for every expected bundle with
`failed=0` and `exit_status=0`, a wrapper success marker, all expected cell
receipts, and successful schema/hash validation.  Timeout, OOM, nonconvergence,
semantic rejection, route failure, and node contamination remain typed outcomes
and are never imputed.

Operational faults receive a new immutable run ID.  A scientific or protocol
change requires an explicit amendment before another submission.  Every
accepted `qsub` is followed by a same-thread `$monitor` heartbeat at 30-minute
intervals; disappearance from `qstat` is not success, and the heartbeat waits
for structurally complete `qacct` before validating or continuing.
