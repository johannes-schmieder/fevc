# Scalable projection qualification

## Runtime architecture

The scalable `project()` route is an additive capability on the planned Rust
generic-JLA lifecycle. It does not change the exact Mata projection oracle or
component-inference surface.

At the operation level, the route now has the following ownership boundary:

1. Stata validates the strict public tuple and preflights the synchronous
   projection-copy and coefficient-space preparation peak.
2. The C boundary and Rust temporarily hold validated copies of the retained
   project columns. Rust streams them once to form the small weighted
   projection Gram and coefficient-space fixed-effect loadings. Both
   observation-by-project copies are released before solve; retained
   projection state is `O(pq+q^2)`.
3. The existing generic-JLA solve supplies the full-model coefficients,
   leverages, and deleted-residual adjustment. The projection variance proxy
   is `(working_y - mean(working_y)) * deleted_adjusted`.
4. The live sparse solver performs one inverse action for each projection
   column, including the automatic constant. No full inverse is requested.
5. Rust streams retained observations and accumulates KSS and residual-squared
   score outer products directly into two `q`-by-`q` matrices. It never stores
   a dense `n`-by-parameter design or an `n`-by-`q` score matrix.
6. Rust, the C/SPI boundary, and Stata independently reconcile projection
   column counts, loading residuals, complete-system residuals, conditioning,
   covariance symmetry/PSD, result bytes, augmentation memory, solve memory,
   and whole-command memory admission before posting results.

The route is available only for explicit
`backend(rust) rng(counter_v1) algorithm(jla) deletion(observation)`, mover-only
inference, unit frequency weights, explicit diagonal PCG, and the generic
engine (explicitly or by automatic selection). Frequency- and target-mass
projections and worker- and firm-effect projections are supported. CMG and
automatic solver routing remain outside this first qualification. This narrow
gate avoids changing any previous `project()` request silently.

The established result matrices remain:

- `e(projection_b)`;
- `e(projection_V)`;
- `e(projection_V_naive)`; and
- `e(projection_results)`.

The sparse route adds `e(projection_diagnostics)`,
`e(projection_augmentation_receipt)`, and
`e(projection_solver_diagnostics)`.

## Focused qualification

`tests/stata/test_rust_projection.do` covers the public lifecycle on a
240-observation connected graph. It compares firm/frequency and worker/target
projection coefficients with Mata exact, checks the complete covariance on the
firm case, verifies phase-6 RHS receipts, and enforces residual, PSD-schema,
and memory gates.

`vckss_scalable_projection.do` reuses the immutable 1,002-observation input in
`evidence/input.csv`. It does not rewrite the accepted exact/MATLAB evidence.
With 2,000 deterministic Counter-V1 probes it requires:

- projection coefficients within `1e-10` relative of the committed exact
  oracle;
- the full three-by-three covariance within `5e-4` relative of exact;
- the maintained MATLAB `lincom_KSS` `z1` and `z2` standard errors within a
  0.5% Monte Carlo band;
- complete-system projection residuals within the registered solver tolerance;
- a nonnegative covariance eigenvalue up to registered roundoff; and
- exact RHS-count and memory-forecast reconciliation.

The focused local run on 29 August 2026 passed. Projection coefficients were
within `3.24e-14` relative of exact and the full covariance was within
`1.33e-5` relative. The final Rust command took 3.40 seconds on the local four-core
test setting. This timing is a smoke-test observation, not a MATLAB comparison
or a performance claim.

### Qualification record

Runtime source commit `9c62619` passed the focused 29 August 2026 gate on
macOS arm64 with Stata 18 and Rust 1.85.1:

- locked Rust workspace formatting, Clippy with warnings denied, and all
  workspace/all-target tests;
- the retained-session projection memory-boundary test and the C interrupt,
  error-transport, and ABI-header fixtures;
- `test_rust_projection.do`, `test_inference.do`,
  `test_rust_planned_v4.do`, `test_rust_generic_jla.do`,
  `test_rust_plugin.do`, and `test_backend_routing.do` against a plugin rebuilt
  from that source;
- `vckss_scalable_projection.do` against the immutable 1,002-row exact/MATLAB
  fixture; and
- 41 focused Python layout, formula, dense-oracle, and Rust/Mata parity tests,
  plus the generated-CMG assembly check.

The complete Python collection reported 452 passes, four failures, and three
errors because the unchanged scale-bundle allowlist omits `LICENSE`,
`THIRD_PARTY_NOTICES.txt`, and `vckss_inference.mata`; all three files and that
omission predate this route. This is recorded as a baseline workflow defect,
not accepted as projection evidence and not expanded here under the focused
qualification policy. No SCC run, platform matrix, or paper claim was changed.

## Focused MATLAB scaling comparison

No large SCC run is authorized merely by this source change. When the route is
ready for performance qualification, use the following registered comparison
rather than the earlier broad component benchmark matrix.

### Cells

- one deterministic all-mover connected design;
- `n = 6,000` for the dense-oracle cell and `n = 24,000` and `96,000` for the
  scaling cells;
- worker spell length six, three adjacent firms per worker, no controls;
- firm-effect projection on an automatic constant plus two fixed covariates;
- observation deletion, unit frequency mass, four application threads; and
- MATLAB JLL `epsilon = 0.01`, with VCkss probes set to the same
  `ceil(log2(n)/epsilon)` count used by maintained `eff_res.m`.

### Compared paths

The VCkss process runs one public command with generic JLA, diagonal PCG, and
`project()`.
The MATLAB process calls maintained `leave_out_COMPLETE` with observation
deletion, mover restriction, three component outputs, `do_SE=0`, and `JLL`,
then constructs the KSS proxy from the saved `eta_h` and calls maintained
`lincom_KSS` once. Thus the comparison includes MATLAB's graph/sample work,
JLA leverage work, coefficient solve, and one inverse action per projection
column. Timing `lincom_KSS` alone is not an admissible comparator.

Run each implementation/size as a separate cold process. Collect total wall
time and process peak RSS with `/usr/bin/time -v`, and collect native phase
times inside each application. Use three measured repetitions per scaling
cell; report all repetitions and the median. Different MATLAB and Counter-V1
random streams are not seed-coupled.

### Gates

- The 6,000-row cell must pass the exact coefficient, covariance, residual,
  PSD, and schema gates before larger cells start.
- Every VCkss cell must reconcile its admitted memory receipt and all complete
  original-system residuals.
- Cross-implementation coefficients must agree within `1e-8` relative and
  projection standard errors within 1%, conditional on the matched projection
  counts. The raw covariance and seed dispersion must also be reported.
- Incremental VCkss peak RSS, net of an empty licensed-Stata process, may not
  grow by more than 5.5 times when `n` grows fourfold from 24,000 to 96,000.
  This is a focused guard against reintroducing dense quadratic storage, not a
  claim that all memory is exactly linear.
- Any allocation failure, nonconvergence, conditioning failure, material PSD
  cleanup, missing accounting record, or source/hash mismatch fails the cell.

The first run should be a single focused SCC array containing only these six
implementation-by-size cells and their three repetitions. A broader platform
matrix, large production sample, or paper performance edit requires a
separate risk decision after these receipts are reviewed.
