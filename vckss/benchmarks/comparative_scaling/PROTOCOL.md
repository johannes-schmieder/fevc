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

## SCC isolation and resources

Each task requests project `welfgr`, queue `econ`, 16 bound OpenMP slots,
`cpu_type=Gold-6242`, and `exclusive=TRUE`. This selects the registered Cascade
Lake host class (`scc-gd4`/`scc-gr4`) and is an explicit benchmark-isolation
exception. Each fresh application is additionally restricted with `taskset` to
the first registered 1/2/4/8/16 CPUs. Stata verifies `c(processors)`; Rust
verifies requested and used CMG threads; MATLAB verifies its local pool size and
the monitor observes the client plus every worker PID.

Pilot policy is 4 GiB per requested slot and `memory_gib(56)`. The small pilot
is task 7 (`strong_d2`, 7,680 rows, four cores, repetition 1). The worst-case
pilot is task 298 (`weak_d3`, 1,966,080 rows, 16 cores, repetition 1). If any
pilot exceeds 48 GiB RSS or admission, the entire production run is restaged at
6 GiB per slot and `memory_gib(88)`. If that envelope is insufficient, the
benchmark stops.

Each estimator has a 10,800-second timeout; the task has a 43,200-second hard
wall. Timeouts and scientific rejections are retained as outcomes. Infrastructure
acceptance requires `qacct failed=0`, `exit_status=0`, the exact source and
binary manifests, wrapper/node receipts, the Gold-6242 identity, and valid role
status schemas.

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
and a collection receipt with hashes. Cell reports use medians and full
three-run ranges; three observations are not presented as confidence intervals.

The report will show command time by rows, parallel speedup and efficiency,
Rust/MATLAB and Rust/Mata time ratios, estimator/full-process RSS, memory ratios,
time--memory Pareto frontiers, and failure maps. Applied guidance is restricted
to the measured grid: fastest accepted route, the smallest core count within
10% of the 16-core median, and the largest measured case fitting 8/16/32/64 GiB
with 25% headroom over the observed full-process RSS.

## Reproduction sequence

From a clean committed checkout, build a local immutable stage:

```bash
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_run.py \
  --repo "$PWD" --output /private/tmp/RUN_ID \
  --stata-spi rust/stata_backend/stata-spi \
  --mem-per-core-gib 4 --command-memory-gib 56
vckss/benchmarks/comparative_scaling/deploy_scc.sh /private/tmp/RUN_ID
```

On SCC, submit preparation and then the two pilots:

```bash
bash RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh RUN prepare
bash RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh RUN pilot-small
bash RUN/source/vckss/benchmarks/comparative_scaling/submit_scc.sh RUN pilot-worst
```

After pilot acceptance, stage a fresh production run using the frozen memory
policy, run preparation, and submit `production`. Post-completion collection
requires per-task `qacct` receipts and `validate_task.py`; `aggregate.py` refuses
anything other than exactly 300 validated tasks and 900 estimator calls.
