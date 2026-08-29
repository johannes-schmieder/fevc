# VCkss three-way comparative scaling benchmark

This active benchmark compares the current `vckss` Mata and qualified
`CMG_FULL_V2` Rust routes with maintained MATLAB `LeaveOutTwoWay`. It is a new
source-bound study and does not modify or reinterpret the frozen historical
Stata--MATLAB scaling matrix.

The registered matrix has four deterministic graph families, five stored-row
counts, five target-core counts, and three seed/order replicates. One SCC task
runs all three implementations in a position-balanced order on the same host:

```text
4 graphs x 5 sizes x 5 core counts x 3 replicates = 300 SCC tasks
300 tasks x 3 implementations = 900 estimator calls
```

The frozen 300-task manifest produced 297 complete tasks and 891 accepted
calls. In the largest `strong_d2` one-core cell, MATLAB reached the registered
10,800-second limit in all three repetitions. Those attempts remain explicit
right-censored lower bounds, while the whole cell is excluded from rankings so
partial sequential-task results cannot create an unbalanced comparison. The
same three timeouts occurred in both immutable generations. Future studies
must apply `future_exclusions.json` and must not submit historical task IDs
61--63 (experiment IDs `scale_strong_d2_n1966080_c1_r1` through `r3`).

The graphs and sizes are inherited from the prior public synthetic design:

| ID | Description | Matches per worker |
| --- | --- | ---: |
| `strong_d2` | multi-offset | 2 |
| `strong_d3` | long-range | 3 |
| `strong_d6` | long-range | 6 |
| `weak_d3` | shallow hub-tree leaf-panel bottleneck | 3 |

Stored rows are 7,680; 30,720; 122,880; 491,520; and 1,966,080. At a fixed
row count, graph degree and topology change the numbers of workers and firms,
so cross-family differences are descriptive rather than pure degree effects.
The versioned weak topology partitions workers into five equal panels sharing
one leaf firm. Their five hub pairs are spokes around a branch hub in a
depth-two tree with one root and 39 grandchildren per branch. The smallest
7,680-row cell uses 20 branches; larger cells use 40. This adaptive count keeps
every hub incident to at least two leaf-panel paths and prevents worker
articulations or bridge matches from shrinking the registered sample. A
stride-five pattern covers every hub; seven patterns include the root. Each
worker appears at its branch, one distinct outer hub, and its leaf, so every
leaf touches six hubs. At the
largest row count the graph has 655,360 workers, 131,072 leaf firms, 132,673
firms in total, and 788,032 canonical edges. Its diameter-four heavy forest
contracts completely in the first hierarchy level, leaving zero retained
operators and plan bytes. It clears both frozen candidate-route floors:
350,000 edges and 131,072 vertices. The V7 input receipt records the topology,
panel and branch constants, firm counts, and canonical-edge decomposition.

This topology supersedes rejected pure-cycle and chorded-ring repairs that
failed the unchanged complete-residual gate. A two-block repair passed the
residual gate but created a full CMG plan in both sources, and the final
eight-offset ring passed its residual gate at eight cores but retained an
864,260-byte plan and ran serially in both sources (rejected array `7343654`).
The subsequent hub-ring source `3a236025` was operator-free and large enough to
route, but comparison task `7343701.61` failed the unchanged complete-residual
gate at `1.7926322539575545e-4`; the array was cancelled and no timing is
accepted. That failure also exposed the separate frozen 131,072-vertex vector
floor that earlier low-firm-count repairs could never reach. Exact six-hub
source `c0d9345` then passed local qualification and CI `33153647170`, but
comparison task `7343749.55` failed at reduced residual `5.3625967261654e-5`
and complete residual `2.3704048296633303e-5`; the remaining pilot tasks were
cancelled. The shallow hub-tree design passed the actual local one-core,
200-probe estimator gate at maximum complete residual `6.50839354464e-6` while
remaining operator-free. The first production generation then revealed that
its fixed 40-branch smallest cell had 720 bridge matches and worker
articulations: MATLAB and Mata retained only 5,040 of 7,680 rows, and Rust
correctly rejected the implicit-match shortcut. V2 uses 20 branches only in
that smallest cell, eliminating those graph defects while preserving the
scientific sample and route gates. It replaces the rejected V1 cell without
relaxing a scientific or routing gate.

Every implementation receives the same literal CSV, 200 probes, match
deletion, and no controls or weights. The target grid is 1/2/4/8/16 cores.
Rust and MATLAB use that full target; Stata and Mata use 1/2/4/4/4 processors.
At target 8 and 16, Mata is restricted to the first four CPUs of the same bound
subset, so Rust--Mata ratios are capped-Mata comparisons rather than equal-core
scaling results. VCkss omits
`tolerance()` so its documented phase defaults apply: `1e-10` for fit solves
and `1e-6` for randomized probe solves. Rust is strict
`backend(rust) rng(counter_v1) algorithm(jla) engine(auto)
preconditioner(auto) batch(auto)` and must return `CMG_FULL_V2`; Mata is strict
`backend(mata) rng(stata)` with the otherwise matching request. MATLAB uses
its pinned implementation-specific numerical policy and RNG.

Primary time is fresh-process estimator invocation-to-return. Process launch,
input import, MATLAB pool setup, one-time native compilation, and teardown are
reported separately. Mata's VCkss selection/graph/setup/solve phases, Rust's
documented native ingest/canonicalize/graph/compress/plan/stayer/solve profile,
and the full-CMG graph/hierarchy/RHS/solve/extraction phases remain secondary
diagnostics. Legacy VCkss scalar phases are not relabeled as Rust phases on the
full-CMG early-return route.
Primary memory is peak summed process-tree RSS during the estimator phase;
complete-process RSS, GNU time, scheduler `maxvmem`, and VCkss's receipted
forecast/admitted/retained memory are retained as distinct diagnostics. Pilot
resource acceptance uses observed process-tree physical RSS; SGE `maxvmem` is
virtual address space and remains diagnostic rather than a physical-allocation
gate.

Cross-backend Rust--Mata corrected results use the registered independent-probe
MCSE envelope. MATLAB's different RNG, tolerance, and legacy finite-projection
formula make its corrected targets descriptive; a MATLAB timing can guide use
only when its finite-target, identity, retained-row, and numerical-status gates
pass. Failures and timeouts remain results and are never silently dropped.
Registered right-censored calls live in a separate machine-readable ledger and
do not enter complete-cell performance summaries.
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
The preparation-only run builds one canonical exact-source Rust/MATLAB artifact
set. Each measurement run independently rechecks source, Stata capability,
MATLAB source, canonical preparation accounting, and every artifact byte before
importing that same set. Production submission is blocked unless both pilot
receipts and production match the canonical artifact-source receipt as well as
their source, task, SPI, and binary identities. The default prototype
request reserves 16 slots at 8 GiB per slot and gives VCkss a 112 GiB direct-
allocation envelope, leaving room for the host process. This is not a memory-
efficiency acceptance ceiling and can be raised in a new source-bound run.
Every preparation also runs a source-bound Stata/MP capability probe before
building or importing artifacts. It hashes `c(processors_lic)` and requires an
entitlement of four. The public package continues to derive native
threads from `c(processors)`. For this study only, preparation applies the
checked-in, hash-bound Ado adapter to the benchmark artifact. Its fail-closed
environment contract passes the registered 1/2/4/8/16 Rust thread count through
the existing native boundary while confirming that Stata remains at
`min(target,4)`. The adapter is applied only after source archiving, to both
qualification candidates identically, and is never installed as public VCkss.

Prototype tasks do not request a fixed queue, host, CPU model/architecture,
exclusive node, or buy-in resource. Production uses `qsub -t 1-300` with no
`-tc`, making all 300 tasks scheduler-eligible immediately; SCC controls actual
concurrency through available resources and fair share. Each task retains its
start/end interval, and aggregation reports overlap with other accepted tasks
on the same host as a contention-sensitivity diagnostic.
All SCC-side Python entry points explicitly load and verify
`python3/3.12.4`; they never depend on SCC's default Python 3.6.
The submitted source remains literal `-pe omp 16` plus
`-binding linear:16`. SCC may display the effective parallel environment as
its slot-specific `omp16` alias and may omit binding from held-job `qstat`
output. The held-submission audit therefore records that alias resolution,
hashes the one exact binding directive in the immutable job script, and requires
the task to enforce one receipted 16-core block at runtime. SCC's OGS setup can
expose the whole physical host rather than the 16 requested slots, including all
32 cores on newer nodes. The harness therefore atomically claims a non-overlapping
16-core block with a host-local `flock`, holds it for the task lifetime, and
applies the registered role subsets with `taskset`. This permits two array tasks
to use opposite halves of a 32-core node without narrowing scheduler eligibility.
Rust and MATLAB use the same full target
subset; Mata uses that subset through target four and its first four CPUs at
targets eight and 16.
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
An application or scientific failure requires diagnosis. Corrections that can
affect estimator results, route, timing, binaries, inputs, or validation meaning
require a new immutable generation for the affected cells. Unaffected cells may
be carried forward only through a machine-readable compatibility review proving
their estimator/build bytes, inputs, execution, validation, and meaning are
unchanged. Carried tasks retain their original source identity, and a composite
collection records every contributing generation; it never relabels one
generation as an attempt of another. Collection-, reporting-, documentation-,
or unrelated workflow-only corrections revalidate and reuse retained evidence
under the repository compatibility-review policy; they do not automatically
trigger another 300-task array.

The immutable measurement and scientific policy is in [PROTOCOL.md](PROTOCOL.md).
The harness is deliberately separate from historical evidence:

- `build_run.py` creates a clean-commit source archive, source/SPI manifests,
  and the 300-task manifest;
- `deploy_scc.sh` creates a new run-scoped SCC directory and verifies the
  extracted exact source;
- `prepare_artifacts.sge` proves the four-processor Stata entitlement, builds
  and hashes the canonical normal Rust 1.85.1 plugin and maintained MATLAB
  R2024b MEX set once, or verifies and imports those exact bytes into a
  measurement run, then independently receipts the benchmark-only Ado adapter;
- `verify_artifact_source.py` rejects any canonical source, accounting,
  manifest, or artifact-byte drift before an import;
- `collect_preparation_qacct.sh` requires complete source-bound preparation
  accounting before any measurement array can be submitted;
- `run_task.sge` executes three fresh, CPU-restricted processes in registered
  order and records command/full-process time, phase/full-process RSS, and task
  start/end timestamps;
- `validate_task.py` reconciles scheduler, wrapper, source, binary, route,
  state, residual, numerical, and memory evidence;
- `validate_pilot.py` and `verify_pilots.py` bind both pilot passes to the
  production source, manifests, memory policy, and binaries;
- `collect_generation.py` inventories terminal accounting, validates every
  successful task, and classifies each rejected task from exact signatures;
- `authorize_replacement.py` permits only the registered two-task repair pilot
  (tasks 1 and 226), then the exact unresolved affected-task remainder, while
  binding both stages to the old terminal inventory and unchanged estimator
  binaries;
- `task_map.py` maps an authorized sparse retry or replacement set onto one
  dense scheduler array, with a bijective map hash carried through node,
  accounting, validation, and composite provenance;
- `aggregate.py` emits the compact 900-call ledger, cell summaries, scheduler
  index, 20 deterministic graph/size input hashes, pinned preparation
  identities, source/binary manifests, CPU-model strata, overlap sensitivity,
  and collection receipt.
- `aggregate_composite.py` accepts a disjoint 228-task base and 72-task
  replacement partition only after all generation and compatibility receipts
  agree, retaining the source identity of every task.
- `report/` consumes only an accepted 300-task collection and emits vector
  figures, LaTeX tables, machine-readable applied guidance, Markdown, and the
  standalone PDF. Its memory-budget guide uses full-process RSS plus 25%
  headroom.
