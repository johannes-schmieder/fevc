# Registered protocol: VCkss three-way scaling

## Scope and decision rule

This source-bound study measures the installed, public VCkss command without
changing estimator behavior. It compares VCkss--Mata, the qualified VCkss--Rust
`CMG_FULL_V2` cell, and maintained MATLAB `LeaveOutTwoWay`. Conclusions are
restricted to the registered synthetic grid. Fixed stored-row counts imply
different worker and firm counts across graph degrees; the study is not a
universal language ranking.

A graph--row--core cell is rankable only when all three repetitions complete
scientifically for all three implementations and each paired Rust--Mata result
passes the registered independent-probe MCSE envelope. Successful calls in an
incomplete cell remain in the evidence but the cell receives no performance
winner. MATLAB results are descriptive cross-package comparisons because its
RNG, tolerance, and finite-projection implementation differ.

## Frozen design

- Graphs: `strong_d2`, `strong_d3`, `strong_d6`, and bottlenecked `weak_d3`.
- Stored rows: 7,680; 30,720; 122,880; 491,520; and 1,966,080.
- Active cores: 1, 2, 4, 8, and 16.
- Repetitions/seeds: `(1, 104729)`, `(2, 8675309)`, and `(3, 20260819)`.
- Order rotation: Mata--Rust--MATLAB; Rust--MATLAB--Mata; MATLAB--Mata--Rust.
- JLA projections: 200.
- Sample: match deletion, joint nuisance handling, movers, observation-key
  ordering, and no controls, observation weights, target weights, or custom
  deletion identifiers.
- Target population: uniform stored rows.

The Stata input generator creates the literal CSV once in job-local `$TMPDIR`.
The same SHA-256-identical file is imported by all three fresh processes.
Project storage retains only the input receipt and hash, not duplicate row data.

## Estimator requests

VCkss--Rust is:

```stata
vckss y, worker(worker) firm(firm) deletion(match)              ///
    probeorder(observation_key) backend(rust) rng(counter_v1)    ///
    algorithm(jla) engine(auto) preconditioner(auto) batch(auto) ///
    probes(200) seed(SEED) maxiter(20000) nodisplay
```

The driver adds the registered memory and wall limits and requires
`e(cmg_backend)=="CMG_FULL_V2"`, the vendored CMG source identity, and exact
requested/used thread receipts. VCkss--Mata changes only the explicit backend
and RNG consent to `backend(mata) rng(stata)`. Neither route supplies
`tolerance()`, so production defaults remain fit `1e-10` and probes `1e-6`.

MATLAB calls pinned maintained `leave_out_KSS` with match deletion, JLA P200,
and no controls. Its source tree, core file, runtime tree, MEX binaries, version,
RNG setup, and numerical status are receipted separately. No claim of identical
random draws or solver tolerances is made.
The application log's single maintained-PCG termination is parsed. Converged
iteration/residual evidence is accepted; a reported nonconvergence becomes a
scientific numerical rejection while retaining the measured time and memory.

## SCC scheduling and resources

Each prototype task requests project `welfgr`, 16 bound OpenMP slots, and 8 GiB
per slot, but no fixed queue, host, CPU model/architecture, exclusive node, or
buy-in resource. Production is submitted as `qsub -t 1-300` without `-tc`, so
all instances are eligible immediately and SCC controls actual concurrency by
available resources and fair share. The SCC scheduler may select any eligible
host. Both job scripts clear inherited request-file defaults before declaring
their registered resources. SCC's mandatory global JSV subsequently injects a
soft `buyin=TRUE` preference into every batch job; the harness does not request
it, and it does not restrict queue or host eligibility. Submissions are held,
their effective `qstat` specifications are captured and validated, and the
whole array is released only after the hard-resource contract passes. All
three implementations run sequentially within that one task
and host, and order rotates across repetitions. Each fresh application is
restricted with `taskset` to the first registered 1/2/4/8/16 assigned CPUs.
Stata verifies `c(processors)`; Rust verifies requested and used CMG threads;
MATLAB verifies its local pool size; and the monitor observes the client plus
every worker PID. Hostname, CPU model, assigned affinity, and scheduler
accounting are required evidence.

The default prototype request is 8 GiB per slot, or 128 GiB of scheduler-backed
memory, with `memory_gib(112)` as VCkss's direct-allocation safety envelope.
The remaining allocation covers the Stata process and non-native overhead.
This envelope is neither a target nor a performance acceptance ceiling. A
source-bound run may request more when forecast or observed usage warrants it;
the manifest requires only that the command envelope fit the scheduler request.
Forecast, admission, retained memory, estimator-phase RSS, full-process RSS,
and scheduler memory remain recorded outcomes.

The small pilot is task 7 (`strong_d2`, 7,680 rows, four cores, repetition 1).
The worst-case pilot is task 298 (`weak_d3`, 1,966,080 rows, 16 cores,
repetition 1). They validate mechanics and resource adequacy, not a universal
RAM ceiling. Preparation-only, small-pilot, worst-case-pilot, and production
use distinct immutable run directories. Each measurement directory has an
exact-source preparation receipt. Production submission requires both pilot
pass receipts to match its source commit, bundle, source manifest, task
manifest, Stata SPI manifest, memory policy, and binary manifest.

Each estimator has a 10,800-second timeout; the task has a 43,200-second hard
wall. Timeouts and scientific rejections are retained as outcomes. Infrastructure
acceptance requires `qacct failed=0`, `exit_status=0`, the exact source and
binary manifests, wrapper/node receipts, a complete scheduler-assigned host/CPU
identity, and valid role status schemas.

## Timing and memory

Primary time is estimator invocation-to-return inside the fresh Stata or MATLAB
process. Secondary time is process launch through validated output. Import,
MATLAB pool startup/teardown, and one-time plugin/MEX compilation are reported
separately. There is no unreported warm-process estimator timing.
The 900-call ledger also retains VCkss selection, graph, compression, setup,
fit, leverage, target, correction, RNG, Schur, and PCG timers, together with
full-CMG graph, hierarchy/plan, RHS, solve, and extraction timers.

Primary memory is peak summed process-tree RSS between estimator phase markers.
The full-process peak, GNU time peak RSS, scheduler `maxvmem`, and VCkss's
forecast/admitted/retained native memory are separate diagnostics. The monitor
samples at 250 ms and supports every registered MATLAB worker count.

## Scientific acceptance

Every successful VCkss call must pass its complete original-system residual,
accounting, target-identity, `e(sample)`, data, RNG, sort-state, route, memory,
and lifecycle gates. For each corrected Rust--Mata target, independent-probe
acceptance is

```text
abs(rust - mata) <= max(1e-8 * max(1, abs(rust), abs(mata)),
                        6 * hypot(rust_mcse, mata_mcse)).
```

MATLAB must return finite corrected targets, the target identity, the registered
retained-row count, and an accepted numerical status before its performance can
enter guidance. Its target gaps are reported descriptively, not used as an
equality gate.

## Aggregation and reporting

The compact evidence contains 300 immutable task receipts, 300 validations, a
900-row estimator-call ledger, a 300-row role/cell summary, a scheduler index,
20 deterministic graph/size input hashes, preparation qacct and wrapper
receipts, pinned toolchain/MATLAB identities, source and binary manifests, and
a collection receipt with hashes. Cell reports use medians and full three-run
ranges; three observations are not presented as confidence intervals. Route
ratios are calculated within each same-host task and then summarized across
repetitions. Absolute timing and cross-core scaling may mix SCC CPU models and
are descriptive unless reported within a homogeneous CPU stratum or confirmed
in a later controlled subset. Every task records start/end UTC and epoch times.
The scheduler index reports the number, duration, and maximum concurrency of
overlapping accepted tasks from the same array generation on the same host;
separate tables stratify by CPU model and summarize paired ratios by CPU model
and overlap class.

Only missing tasks or tasks with infrastructure-level scheduler/wrapper failure
may be retried, using their exact task IDs and a new attempt ID. Attempt output
paths are immutable. A validated task ID may occur only once; duplicate
successful results are rejected. Cross-attempt aggregation additionally
requires identical source commit, source bundle, source manifest, task row,
binary manifest, and literal input hash. Application or scientific failure
requires corrected source and a new complete run generation.

The report will show command time by rows, parallel speedup and efficiency,
Rust/MATLAB and Rust/Mata time ratios, estimator/full-process RSS, memory ratios,
time--memory Pareto frontiers, and failure maps. Applied guidance is restricted
to the measured grid: fastest accepted route, the smallest core count within
10% of the 16-core median, and the largest measured case fitting 8/16/32/64 GiB
with 25% headroom over the observed full-process RSS.

## Reproduction sequence

From a clean committed checkout, build and deploy four local immutable stages.
The same source commit and memory policy are used in each command:

```bash
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/PREPARATION_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind preparation
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/SMALL_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind pilot-small
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/WORST_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind pilot-worst
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/PRODUCTION_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind production \
  --pilot-small-run-id SMALL_RUN --pilot-worst-run-id WORST_RUN
for run in PREPARATION_RUN SMALL_RUN WORST_RUN PRODUCTION_RUN; do
  vckss/benchmarks/comparative_scaling/deploy_scc.sh "/private/tmp/$run"
done
```

On SCC, first pass the preparation-only run, then prepare and execute each pilot
and collect complete accounting before moving on:

```bash
bash PREPARATION_RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh \
  PREPARATION_RUN prepare
bash SMALL_RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh \
  SMALL_RUN prepare
bash SMALL_RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh \
  SMALL_RUN pilot-small first
bash SMALL_RUN/source/vckss/benchmarks/comparative_scaling/collect_qacct.sh \
  SMALL_RUN first SMALL_ARRAY_JOB 7
bash WORST_RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh \
  WORST_RUN prepare
bash WORST_RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh \
  WORST_RUN pilot-worst first
bash WORST_RUN/source/vckss/benchmarks/comparative_scaling/collect_qacct.sh \
  WORST_RUN first WORST_ARRAY_JOB 298
```

After pilot acceptance, run production preparation and submit `production`.
The submission wrapper verifies the two source/binary-bound pilot receipts and
then issues the unthrottled `-t 1-300` request. Continue monitoring until every
task has a complete `qacct` record. For infrastructure-only omissions, submit
an exact-ID retry such as `retry 14,88-90 retry-1` and collect that attempt
separately. `aggregate.py` refuses anything other than 300 unique validated
task identities and 900 accepted estimator calls.
