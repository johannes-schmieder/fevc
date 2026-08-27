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
reported separately. Primary memory is peak summed process-tree RSS during the
estimator phase; complete-process RSS, GNU time, scheduler `maxvmem`, and
VCkss's receipted memory forecast are retained as distinct diagnostics.

Cross-backend Rust--Mata corrected results use the registered independent-probe
MCSE envelope. MATLAB's different RNG, tolerance, and legacy finite-projection
formula make its corrected targets descriptive; a MATLAB timing can guide use
only when its finite-target, identity, retained-row, and numerical-status gates
pass. Failures and timeouts remain results and are never silently dropped.

Build a manifest only after freezing the exact source bundle:

```bash
./.venv/bin/python vckss/benchmarks/comparative_scaling/build_manifest.py \
  --output /path/to/run/input/tasks.tsv \
  --source-commit "$SOURCE_SHA" --bundle "$BUNDLE_SHA256" \
  --mem-per-core-gib 4 --command-memory-gib 56
```

The SCC workflow uses run-scoped storage under
`/projectnb/welfgr/vckss/runs/`, Stata/MP 19, MATLAB R2024b, pinned Rust
1.85.1, and job-local `$TMPDIR` for generated input rows and details. The
production matrix is submitted only after small and worst-case scheduled
pilots freeze the 4/56 or 6/88 GiB policy.

The immutable measurement and scientific policy is in [PROTOCOL.md](PROTOCOL.md).
The harness is deliberately separate from historical evidence:

- `build_run.py` creates a clean-commit source archive, source/SPI manifests,
  and the 300-task manifest;
- `deploy_scc.sh` creates a new run-scoped SCC directory and verifies the
  extracted exact source;
- `prepare_artifacts.sge` builds and hashes the normal Rust 1.85.1 plugin and
  maintained MATLAB R2024b MEX set once;
- `run_task.sge` executes three fresh, CPU-restricted processes in registered
  order and records command/full-process time and phase/full-process RSS;
- `validate_task.py` reconciles scheduler, wrapper, source, binary, route,
  state, residual, numerical, and memory evidence;
- `aggregate.py` emits the compact 900-call ledger, cell summaries, scheduler
  index, and collection receipt.
