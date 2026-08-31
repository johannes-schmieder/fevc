# Registered protocol: VCkss three-way scaling

## Scope and decision rule

This source-bound study measures the public VCkss estimator semantics with a
source-bound benchmark-only thread adapter at the Ado/native boundary. The
adapter changes no public option or ordinary installation behavior. It compares
VCkss--Mata, the qualified VCkss--Rust
`CMG_FULL_V2` cell, and maintained MATLAB `LeaveOutTwoWay`. Conclusions are
restricted to the registered synthetic grid. Fixed stored-row counts imply
different worker and firm counts across graph degrees and topologies; the study
is not a universal language ranking.

This protocol governs an intentionally requested performance study; that is a
compelling reason to run its pilots and production matrix once. It does not
make the 300-task array a routine source gate. Later commits reuse accepted
measurements through the repository compatibility-review policy unless they can
affect estimator/build bytes, inputs, execution or timing, resource measurement,
validation semantics, or the reported conclusion. Reporting-, documentation-,
and unrelated workflow-only changes reprocess retained evidence instead of
rerunning cluster work.

A graph--row--core cell is rankable only when all three repetitions complete
scientifically for all three implementations and each paired Rust--Mata result
passes the registered independent-probe MCSE envelope. Successful calls in an
incomplete cell remain in the evidence but the cell receives no performance
winner. MATLAB results are descriptive cross-package comparisons because its
RNG, tolerance, and finite-projection implementation differ.

## Frozen design

- Graphs: `strong_d2`, `strong_d3`, `strong_d6`, and bottlenecked `weak_d3`.
  The weak graph is the versioned
  `adaptive_shallow_hub_tree_leaf_panel_vector_v2` topology. Five worker
  panels share one leaf and use five spokes around a branch hub in a depth-two
  tree with one root and 39 grandchildren per branch. The 7,680-row cell uses
  20 branches and every larger cell uses 40. This preserves the shallow
  bottleneck while ensuring that every hub has at least two independent
  leaf-panel incidences, so no worker articulation or bridge match changes the
  fixed sample. A stride-five pattern exposes every hub, and each leaf touches
  six hubs. At the largest row count, 132,673
  vertices and 788,032 canonical edges clear CMG's frozen 131,072-vertex and
  350,000-edge connected-vector floors while the diameter-four heavy forest
  contracts completely in the first hierarchy level with zero plan bytes. The
  superseded V1 smallest cell used 40 branches with only 512 leaves, creating
  exactly 720 degree-one outer hubs, 720 bridge matches, and 720 worker
  articulations. Its production evidence is rejected; the uniform-row gate is
  unchanged.
- Stored rows: 7,680; 30,720; 122,880; 491,520; and 1,966,080.
- Target cores: 1, 2, 4, 8, and 16.
- Effective role cores: Rust and MATLAB 1/2/4/8/16; Stata and Mata 1/2/4/4/4.
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
fevc y, worker(worker) firm(firm) deletion(match)              ///
    probeorder(observation_key) backend(rust) rng(counter_v1)    ///
    algorithm(jla) engine(auto) preconditioner(auto) batch(auto) ///
    probes(200) seed(SEED) maxiter(20000) nodisplay
```

The driver adds the registered memory and wall limits and requires
`e(cmg_backend)=="CMG_FULL_V2"`, the vendored CMG source identity, and exact
requested/used thread receipts. The benchmark artifact's deterministic adapter
requires contract `FEVC-BENCHMARK-THREADS-V1`, validates the target and Stata
processor counts before estimator RNG, and supplies the registered Rust thread
count through the existing internal Ado boundary. Ordinary installations retain
the public behavior of deriving native threads from `c(processors)`.
VCkss--Mata changes only the explicit backend
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
whole array is released only after the hard-resource contract passes. SCC may
resolve the literal `omp 16` request to its equivalent `omp16` site alias and
omit the binding line from held-job `qstat`. In that case the audit records the
alias, hashes the immutable script's single `#$ -binding linear:16` directive,
and requires runtime evidence for a 16-core harness-enforced block. SCC's OGS
configuration can expose the full physical host rather than the 16 requested
slots, including all 32 cores on newer nodes. Each task atomically claims one
deterministic 16-core block with a host-local `flock`, holds the claim for its
lifetime, and records the host affinity, assigned block, block index, and block
capacity. This prevents two VCkss jobs owned by this user from selecting the
same half of a 32-core host without adding a scheduler restriction. Rust and
MATLAB use the same first 1/2/4/8/16 CPUs of the claimed block; Mata uses the
same subset through four and its first four CPUs at targets eight and 16.
All three
implementations run sequentially within that one task
and host, and order rotates across repetitions. Each fresh application is
restricted with `taskset` to its registered role-specific assigned CPUs.
Stata verifies `c(processors)==min(target,4)`; Rust verifies requested and used
CMG threads equal the full target;
MATLAB verifies its local pool size; and the monitor observes the client plus
every worker PID. Hostname, CPU model, assigned affinity, and scheduler
accounting are required evidence.

Before any measurement array is eligible, each immutable preparation job runs
the checked-in Stata capability driver under the pinned Stata/MP 19 module and
records `c(processors_lic)`. The capability receipt, preparation receipt, and
preparation `qacct` receipt must agree that at least four processors are
licensed. This is distinct from SGE's 16-slot reservation and from the Rust and
MATLAB 16-core targets. Preparation also hashes the adapter source and generated
Ado receipt; any missing, malformed, or mismatched contract blocks pilots,
production, retry, and collection.

The preparation-only run builds the canonical normal Rust plugin and maintained
MATLAB MEX set once. Rust and MATLAB builds can embed absolute build paths,
job IDs, PIDs, and generated bundle metadata, so independent compilations are
not assumed byte-reproducible. Each pilot and production preparation instead
verifies the canonical run's exact source, complete preparation accounting,
shared manifests, and every binary byte, imports that immutable artifact set,
regenerates and compares the deterministic benchmark Ado adapter, and records
the canonical artifact-source receipt. No pilot or production task is eligible
unless its imported binary manifest is byte-identical to the canonical one.

The default prototype request is 8 GiB per slot, or 128 GiB of scheduler-backed
memory, with `memory_gib(112)` as VCkss's direct-allocation safety envelope.
The remaining allocation covers the Stata process and non-native overhead.
This envelope is neither a target nor a performance acceptance ceiling. A
source-bound run may request more when forecast or observed usage warrants it;
the manifest requires only that the command envelope fit the scheduler request.
Forecast, admission, retained memory, estimator-phase RSS, full-process RSS,
and scheduler memory remain recorded outcomes. Pilot resource adequacy is gated
by the largest role-specific observed physical RSS because the three estimators
run sequentially. SGE `qacct maxvmem` records virtual address space rather than
physical RSS; it remains a diagnostic and does not by itself reject a pilot.

The small pilot is task 7 (`strong_d2`, 7,680 rows, four cores, repetition 1).
The worst-case pilot is task 298 (`weak_d3`, 1,966,080 rows, 16 cores,
repetition 1). They validate mechanics and resource adequacy, not a universal
RAM ceiling. Preparation-only, small-pilot, worst-case-pilot, and production
use distinct immutable run directories. Each measurement directory has an
exact-source preparation receipt and names the same canonical preparation-only
run. Production submission requires both pilot pass receipts to match its
canonical artifact-source receipt, source commit, bundle, source manifest, task
manifest, Stata SPI manifest, memory policy, and binary manifest.
The preparation, pilot, and production receipts also bind the same required
and licensed Stata processor counts, 16-thread Rust ceiling, adapter identity,
and capability-receipt hash.

Each estimator has a 10,800-second timeout; the task has a 43,200-second hard
wall. Timeouts and scientific rejections are retained as outcomes. Infrastructure
acceptance requires `qacct failed=0`, `exit_status=0`, the exact source and
binary manifests, wrapper/node receipts, a complete scheduler-assigned host/CPU
identity, and valid role status schemas.

The completed production evidence establishes one registered exception. In
`strong_d2` at 1,966,080 rows and one active core, maintained MATLAB reached
the 10,800-second estimator timeout in all three repetitions in both the base
and replacement generations. Historical task IDs 61--63 are therefore
right-censored at 10,800 seconds. Their entire graph--size--core cell is
excluded from paired rankings, pooled ratios, and one-to-many-core scaling;
partial route results from the sequential tasks are not accepted as complete
triplets. The timeout evidence remains in `censored_matlab_3.tsv`. Future
benchmark generations must not submit these three experiments. The
machine-readable rule is `future_exclusions.json`.

## Timing and memory

Primary time is estimator invocation-to-return inside the fresh Stata or MATLAB
process. Secondary time is process launch through validated output. Import,
MATLAB pool startup/teardown, and one-time plugin/MEX compilation are reported
separately. There is no unreported warm-process estimator timing.
The 891-call accepted ledger retains Mata's VCkss selection, graph, compression, setup,
fit, leverage, target, correction, RNG, Schur, and PCG timers; Rust's documented
native ingest, canonicalize, graph, compress, plan, stayer-augmentation, solve,
and total timers; and full-CMG graph, hierarchy/plan, RHS, solve, and extraction
timers. Legacy VCkss scalar phases are inapplicable to the full-CMG early-return
route and are never relabeled or fabricated.

Primary memory is peak summed process-tree RSS between estimator phase markers.
The full-process peak, GNU time peak RSS, scheduler `maxvmem`, and VCkss's
forecast/admitted/retained native memory are separate diagnostics. In
particular, scheduler `maxvmem` is virtual memory and is not compared with the
physical scheduler request as a hard acceptance gate. The monitor samples at
250 ms and supports every registered MATLAB worker count.

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
requires diagnosis. A correction that can affect estimator results, route,
timing, binaries, inputs, or validation meaning invalidates the affected cells
and requires compatible new measurement. Unaffected successful cells may be
carried into a composite study only through an explicit machine-readable
compatibility review proving that estimator/build bytes and each carried
task's input, execution, validation, and reported meaning are unchanged. Each
cell retains its immutable generation and source identity; generations are
never relabeled as attempts. A correction confined to collection or reporting
revalidates retained receipts; it does not automatically trigger another
300-task array.

The report will show command time by rows, parallel speedup and efficiency,
Rust/MATLAB and Rust/Mata time ratios, estimator/full-process RSS, memory ratios,
time--memory Pareto frontiers, and failure maps. Applied guidance is restricted
to the measured grid: fastest accepted route, the smallest effective core count
within 10% of each role's measured maximum (four for Mata, 16 otherwise), and
the largest measured case fitting 8/16/32/64 GiB
with 25% headroom over the observed full-process RSS.
Every table and figure retains both target and effective core counts. High-core
Rust--Mata ratios are labeled `CAPPED_MATA`; they are useful paired availability
comparisons but are not interpreted as equal-core Mata scaling.

## Reproduction sequence

From a clean committed checkout, first build and deploy the canonical
preparation-only stage:

```bash
./.venv/bin/python fevc/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/PREPARATION_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind preparation
fevc/benchmarks/comparative_scaling/deploy_scc.sh \
  /private/tmp/PREPARATION_RUN
```

Submit and collect that preparation before staging measurement runs:

```bash
bash PREPARATION_RUN/source/fevc/benchmarks/comparative_scaling/submit_scc.sh \
  PREPARATION_RUN prepare
bash PREPARATION_RUN/source/fevc/benchmarks/comparative_scaling/collect_preparation_qacct.sh \
  PREPARATION_RUN PREPARATION_JOB
```

Then build and deploy the three measurement stages from the same clean commit,
memory policy, and canonical artifact-source run:

```bash
./.venv/bin/python fevc/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/SMALL_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind pilot-small \
  --artifact-source-run-id PREPARATION_RUN
./.venv/bin/python fevc/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/WORST_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind pilot-worst \
  --artifact-source-run-id PREPARATION_RUN
./.venv/bin/python fevc/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/PRODUCTION_RUN \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 8 --command-memory-gib 112 --run-kind production \
  --artifact-source-run-id PREPARATION_RUN \
  --pilot-small-run-id SMALL_RUN --pilot-worst-run-id WORST_RUN
for run in SMALL_RUN WORST_RUN PRODUCTION_RUN; do
  fevc/benchmarks/comparative_scaling/deploy_scc.sh "/private/tmp/$run"
done
```

On SCC, prepare and execute each pilot and collect complete accounting before
moving on:

```bash
bash SMALL_RUN/source/fevc/benchmarks/comparative_scaling/submit_scc.sh \
  SMALL_RUN prepare
bash SMALL_RUN/source/fevc/benchmarks/comparative_scaling/collect_preparation_qacct.sh \
  SMALL_RUN SMALL_PREPARATION_JOB
bash SMALL_RUN/source/fevc/benchmarks/comparative_scaling/submit_scc.sh \
  SMALL_RUN pilot-small first
bash SMALL_RUN/source/fevc/benchmarks/comparative_scaling/collect_qacct.sh \
  SMALL_RUN first SMALL_ARRAY_JOB 7
bash WORST_RUN/source/fevc/benchmarks/comparative_scaling/submit_scc.sh \
  WORST_RUN prepare
bash WORST_RUN/source/fevc/benchmarks/comparative_scaling/collect_preparation_qacct.sh \
  WORST_RUN WORST_PREPARATION_JOB
bash WORST_RUN/source/fevc/benchmarks/comparative_scaling/submit_scc.sh \
  WORST_RUN pilot-worst first
bash WORST_RUN/source/fevc/benchmarks/comparative_scaling/collect_qacct.sh \
  WORST_RUN first WORST_ARRAY_JOB 298
```

After pilot acceptance, run production preparation and submit `production`.
The submission wrapper verifies the two source/binary-bound pilot receipts and
then issues the unthrottled `-t 1-300` request. Continue monitoring until every
task has a complete `qacct` record. For infrastructure-only omissions, submit
an exact-ID retry such as `retry 14,88-90 retry-1` and collect that attempt
separately. `aggregate.py` refuses anything other than 300 unique validated
task identities and 900 accepted estimator calls.

If a diagnosed application or scientific correction is isolated to a strict
subset, first collect the terminal generation inventory and build a new
production run with `--replaces-run-id`. The replacement must import the exact
canonical estimator binaries and pass the machine-readable safe-source-delta
review. Submit tasks 1 and 226 as the registered repair pilot: these exercise
the one-worker monitoring representation and the smallest weak graph together.
Only after both validate may the exact other 70 affected task IDs be submitted.
Sparse retry and replacement sets are submitted as one dense scheduler array
because SCC's SGE accepts only one `-t` range. An immutable TSV maps each dense
scheduler task ID to exactly one authorized manifest task ID; node and
validation receipts preserve both IDs and its SHA-256, and collection resolves
raw accounting through the inverse map.
`aggregate_composite.py` then requires a disjoint partition of 228 accepted base
tasks and 72 accepted replacement tasks; it rejects duplicate successes,
unapproved task IDs, binary drift, or incomplete generation provenance.
