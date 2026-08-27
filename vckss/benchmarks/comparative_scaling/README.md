# VCkss three-way comparative scaling benchmark

This active benchmark compares the current `vckss` Mata and qualified
`CMG_FULL_V2` Rust routes with maintained MATLAB `LeaveOutTwoWay`. It is a new
source-bound study and does not modify or reinterpret the frozen historical
Stata--MATLAB scaling matrix.

The registered matrix has four deterministic graph families, five stored-row
counts, five active-core counts, and three seed/order replicates. One SCC task
runs all three implementations in a position-balanced order on the same host:

```text
4 graphs x 5 sizes x 5 core counts x 3 replicates = 300 SCC tasks
300 tasks x 3 implementations = 900 estimator calls
```

The graphs and sizes are inherited from the prior public synthetic design:

| ID | Description | Matches per worker |
| --- | --- | ---: |
| `strong_d2` | multi-offset | 2 |
| `strong_d3` | long-range | 3 |
| `strong_d6` | long-range | 6 |
| `weak_d3` | local ring / bottleneck | 3 |

Stored rows are 7,680; 30,720; 122,880; 491,520; and 1,966,080. At a fixed
row count, graph degree changes the numbers of workers and firms, so
cross-family differences are descriptive rather than pure degree effects.

Every implementation receives the same literal CSV, 200 probes, match
deletion, no controls or weights, and the same active-core count. VCkss omits
`tolerance()` so its documented phase defaults apply: `1e-10` for fit solves
and `1e-6` for randomized probe solves. Rust is strict
`backend(rust) rng(counter_v1) algorithm(jla) engine(auto)
preconditioner(auto) batch(auto)` and must return `CMG_FULL_V2`; Mata is strict
`backend(mata) rng(stata)` with the otherwise matching request. MATLAB uses
its pinned implementation-specific numerical policy and RNG.

Primary time is fresh-process estimator invocation-to-return. Process launch,
input import, MATLAB pool setup, one-time native compilation, and teardown are
reported separately. VCkss selection/graph/setup/solve phases and the full-CMG
graph/hierarchy/RHS/solve/extraction phases remain secondary diagnostics.
Primary memory is peak summed process-tree RSS during the estimator phase;
complete-process RSS, GNU time, scheduler `maxvmem`, and VCkss's receipted
forecast/admitted/retained memory are retained as distinct diagnostics.

Cross-backend Rust--Mata corrected results use the registered independent-probe
MCSE envelope. MATLAB's different RNG, tolerance, and legacy finite-projection
formula make its corrected targets descriptive; a MATLAB timing can guide use
only when its finite-target, identity, retained-row, and numerical-status gates
pass. Failures and timeouts remain results and are never silently dropped.
The validator parses maintained MATLAB's own logged PCG termination. A
nonconverged PCG keeps its time and memory evidence but marks the call
`NUMERICAL_REJECTED`, preventing its cell from being ranked.

Build a manifest only after freezing the exact source bundle:

```bash
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_manifest.py \
  --output /path/to/run/input/tasks.tsv \
  --source-commit "$SOURCE_SHA" --bundle "$BUNDLE_SHA256" \
  --mem-per-core-gib 8 --command-memory-gib 112
```

The SCC workflow uses four separate immutable run directories under
`/projectnb/welfgr/vckss/runs/`, Stata/MP 19, MATLAB R2024b, pinned Rust
1.85.1, and job-local `$TMPDIR` for generated input rows and details. The
stages are preparation-only, small pilot, worst-case pilot, and production.
Each measurement run prepares its own exact-source artifacts, and production
submission is blocked unless the two independently collected pilot receipts
match its source, task, SPI, and binary identities. The default prototype
request reserves 16 slots at 8 GiB per slot and gives VCkss a 112 GiB direct-
allocation envelope, leaving room for the host process. This is not a memory-
efficiency acceptance ceiling and can be raised in a new source-bound run.

Prototype tasks do not request a fixed queue, host, CPU model/architecture,
exclusive node, or buy-in resource. Production uses `qsub -t 1-300` with no
`-tc`, making all 300 tasks scheduler-eligible immediately; SCC controls actual
concurrency through available resources and fair share. Each task retains its
start/end interval, and aggregation reports overlap with other accepted tasks
on the same host as a contention-sensitivity diagnostic.
All SCC-side Python entry points explicitly load and verify
`python3/3.12.4`; they never depend on SCC's default Python 3.6.
The scheduler chooses any eligible host, and each task runs Mata, Rust, and
MATLAB sequentially on that same host with rotated order. Exact CPU, hostname,
affinity, and scheduler receipts are retained. Paired within-task time and
memory ratios are the primary prototype comparison; absolute curves and
cross-core speedups across different hosts are descriptive. CPU-model strata
and own-array host-overlap sensitivity are reported explicitly.

Infrastructure-only recovery creates a new immutable attempt directory and
resubmits the exact missing or infrastructure-failed task IDs. Successful
duplicates are rejected. Aggregation can combine attempts only when source,
bundle, source manifest, task row, binaries, and literal input hashes match.
An application or scientific failure invalidates the generation and requires
corrected source under a new run identity.

The immutable measurement and scientific policy is in [PROTOCOL.md](PROTOCOL.md).
The harness is deliberately separate from historical evidence:

- `build_run.py` creates a clean-commit source archive, source/SPI manifests,
  and the 300-task manifest;
- `deploy_scc.sh` creates a new run-scoped SCC directory and verifies the
  extracted exact source;
- `prepare_artifacts.sge` builds and hashes the normal Rust 1.85.1 plugin and
  maintained MATLAB R2024b MEX set once;
- `run_task.sge` executes three fresh, CPU-restricted processes in registered
  order and records command/full-process time, phase/full-process RSS, and task
  start/end timestamps;
- `validate_task.py` reconciles scheduler, wrapper, source, binary, route,
  state, residual, numerical, and memory evidence;
- `validate_pilot.py` and `verify_pilots.py` bind both pilot passes to the
  production source, manifests, memory policy, and binaries;
- `aggregate.py` emits the compact 900-call ledger, cell summaries, scheduler
  index, 20 deterministic graph/size input hashes, pinned preparation
  identities, source/binary manifests, CPU-model strata, overlap sensitivity,
  and collection receipt.
- `report/` consumes only an accepted 300-task collection and emits vector
  figures, LaTeX tables, machine-readable applied guidance, Markdown, and the
  standalone PDF. Its memory-budget guide uses full-process RSS plus 25%
  headroom.
