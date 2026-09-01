# FEVC--MATLAB R2026a benchmark protocol

Status: registered before the first SCC preparation submission.  Earlier
benchmark receipts are immutable and are not pooled with this campaign.

### Amendment 1: convergence-boundary gate

Campaign `20260901T145443Z-847bbf87-submit` exposed a deterministic harness
boundary: all six largest `strong_d2` cells stopped at the harness's
undocumented 1,000-iteration PCG cap.  That cap was lower than the package
default of 10,000 and was not part of the registered scientific design.  The
owner authorized an explicit, receipted 10,000-iteration cap and a stronger
pilot gate in one replacement campaign.  The replacement pilot remains one
28-core job, but runs the largest `strong_d2` convergence boundary and largest
`weak_d3` resource boundary sequentially.  Both must pass before production.
Estimator tolerances, algorithms, inputs, and the 240-cell matrix are unchanged;
no result from the failed campaign is reused.

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

Before those primary jobs, one diagnostic smoke uses 7,680 strong-degree-two
rows, four application workers, four unrestricted SCC slots, eight GiB per
slot, and a 20-minute wall request.  It builds the campaign artifacts and runs
the exact production launch, measurement, and validation path.  It is an
operational gate, not a main-matrix observation.

After the smoke, one 28-core pilot job runs two cells sequentially: the largest
`strong_d2` cell at 1,966,080 rows and 28 application cores, then the largest
`weak_d3` cell at the same size and core count.  The first is the registered
convergence boundary and the second is the registered resource boundary.  Both
must be scientifically rankable and satisfy the memory envelope.  Pilot timing
is a gate only and does not enter the production evidence set.

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

The full role process tree is sampled nominally every 0.10 seconds.  At the
empty-runtime and loaded-data markers, the application waits for the monitor
to acknowledge at least two complete RSS/PSS samples before advancing.  The
wait is bounded at 30 seconds.  The monitor records:

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

Each publication module has an immutable manifest and is submitted only after
the smoke and worst-case gates establish the resource envelope.  Semantic
mismatch with MATLAB prevents a performance ranking; it is retained as a
scientific result.

## Acceptance and recovery

Rankable calls require identical input and retained-sample hashes; registered
worker, firm, cell, deletion-unit, stayer, and target-population counts; finite
outputs; convergence; complete original-system residual acceptance; requested
route and thread receipts; complete phase markers; and valid memory monitoring.
Every FEVC estimator call explicitly receipts `maxiter(10000)`; the registered
residual and cross-language tolerances are unchanged.
Small cross-language oracles use a scaled `1e-8` tolerance.  Randomized target
differences are descriptive unless a registered independent MCSE gate applies.

Scheduler acceptance requires complete `qacct` for every expected bundle with
`failed=0` and `exit_status=0`, a wrapper success marker, all expected cell
receipts, and successful schema/hash validation.  Timeout, OOM, nonconvergence,
semantic rejection, route failure, and node contamination remain typed outcomes
and are never imputed.

One exact-source campaign owns artifact preparation, smoke, worst-case pilot,
production attempts, and their receipts.  SGE dependencies order the stages;
each downstream wrapper checks the upstream pass receipt because dependency
completion alone is not success.  Monitoring is on demand for short work and,
when useful, campaign-level for long production work.  It never releases
stages.

An identical job may be retried once for a proven scheduler, transport, or
execution-host fault.  An environment, path, or wrapper defect receives one
narrow fix and one focused smoke retest.  Another operational failure stops
the campaign for review.  Scientific or protocol changes require an explicit
amendment.  Final acceptance still requires structurally complete `qacct`;
disappearance from `qstat` is not success.
