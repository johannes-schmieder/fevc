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
inference, positive integer frequency weights, explicit diagonal PCG or forced
generic CMG, and the generic engine (explicitly or by automatic selection).
Frequency- and target-mass projections and worker- and firm-effect projections
are supported. Automatic solver routing remains outside the qualified tuple.
This narrow gate avoids changing any previous `project()` request silently.

Frequency weights are literal physical copies. Each copy enters the normal
equations, leverage proxy, physical projection mean, and KSS/naive covariance.
Explicit target mass remains stored-row mass and is not frequency multiplied.

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

### Frequency-weight and 6,000-row preflight

The weighted 1,002-row oracle uses nonconstant frequency weights and both
firm/frequency and worker/target projections. Compressed exact and literal
expansion agree to `1.78e-15` maximum absolute error. The independent dense
MATLAB same-formula calculation agrees within `4.39e-8` absolute, and official
maintained `lincom_KSS` coefficients and standard errors agree within
`7.95e-8` and `1.17e-6` relative, respectively. With 4,000 Counter-V1 probes,
Rust agrees with exact within `1.52e-12` for coefficients and `0.2994%` for
the full covariance.

The deterministic 6,000-row local gate also passes. Rust coefficients agree
with exact Mata to `5.51e-13` maximum absolute error, its full covariance is
within `0.185%` on the maximum-matrix scale, and its `z1`/`z2` standard errors
are within `0.162%` of maintained MATLAB. Local estimator-phase times were
10.85 seconds for exact Mata, 23.38 seconds for Rust/JLA/diagonal, and 10.01
seconds for maintained MATLAB JLA plus `lincom_KSS`. These are preflight
receipts, not a performance claim. The registered paired SCC result is recorded
below.

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
- Cross-implementation coefficients must satisfy the registered `1e-8`
  unit-scaled tolerance; projection covariance diagonals and standard errors
  must agree within 1%, conditional on matched projection counts. Maintained
  `lincom_KSS` does not return off-diagonal covariance, so the complete Rust
  covariance, both implementations' directly comparable diagonals, and seed
  dispersion must be reported without implying a MATLAB off-diagonal oracle.
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

## SCC result: diagonal PCG is not the large-data route

Exact runtime source
`96e7a666c0a2dcc2c89183c656edd72e04b8ec0e` passed preparation and the
6,000-row exact/Rust/maintained-MATLAB gate. Array `7368481` then completed all
nine registered paired tasks with complete accounting: tasks 1--6 had
`failed=0`, `exit_status=0`, and all application gates passed; tasks 7--9 had
`failed=0`, `exit_status=1`. The accepted 6,000- and 24,000-row repetitions
show that the new projection formulas remain statistically aligned while
diagonal PCG does not deliver MATLAB-like solve scaling.

| Rows | VCkss command median | MATLAB command median | VCkss whole-process median | MATLAB whole-process median | Maximum SE difference | Maximum covariance-diagonal difference |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 6,000 | 32.494 s | 78.743 s | 33.260 s | 159.705 s | 0.2395% | 0.4785% |
| 24,000 | 1,466.706 s | 77.336 s | 1,468.176 s | 172.261 s | 0.2204% | 0.4403% |
| 96,000 | not accepted | not accepted | not accepted | not accepted | not evaluated | not evaluated |

At 6,000 rows VCkss is 2.42 times faster on command time and 4.80 times faster
on complete-process wall time. At 24,000 rows it is 18.97 times slower on
command time and 8.52 times slower on complete-process wall time. Maximum
unit-scaled coefficient differences are `1.85e-10` and `1.21e-10`,
respectively. The Rust model solve needed 161 iterations at 6,000 rows and 625
at 24,000 rows; its complete-system projection residuals were `6.71e-11` and
`9.24e-11`, both below the registered `1e-9` tolerance.

All three 96,000-row tasks failed their first role in the registered sequential
order. In two MATLAB-first repetitions, the independently reconstructed
grounded coefficient fit reached the 1,000-iteration PCG limit, returned best
iterate 982, and had relative residual `1.7e-7`, above the harness's strict
`1e-10` fit tolerance. This check is stricter than maintained MATLAB's warning-
and-continue behavior and is not a claim that `lincom_KSS` itself failed. In
the Rust-first repetition, VCkss diagonal model PCG reached 20,000 iterations
with reduced residual `0.034749`; MATLAB was consequently not run in that
task. The rotated order therefore gives typed convergence evidence without
manufacturing an unpaired 96,000-row speed or covariance comparison.

VCkss median incremental RSS rose from 175,452 KiB at 6,000 rows to 271,792
KiB at 24,000 rows, a 1.55-fold increase for four times as many rows. The
failed 96,000-row Rust attempt peaked at 401,272 KiB above its Stata baseline,
which is consistent with sparse rather than quadratic retained state, but it
cannot pass the registered 24,000-to-96,000 memory gate because the scientific
solve failed. MATLAB's reported peaks sum MATLAB and four worker processes and
are comparable across its own cells, not directly to the single VCkss process.

The qualification decision is therefore narrow and decisive: keep the exact
oracle and the implemented sparse projection machinery, but do not describe
explicit diagonal PCG as having comparable large-data reach. The smallest next
implementation step is to route the same model and projection inverse actions
through a stronger qualified preconditioner, preferably the existing full-CMG
infrastructure, while preserving complete-system residual, conditioning, PSD,
memory, and public-schema gates. No replacement array, broad platform matrix,
or paper performance edit is warranted before that source change exists.

Compact receipts are under
`benchmarks/projection_scaling/evidence/scc/96e7a666c0a2dcc2c89183c656edd72e04b8ec0e/`.

## Forced-CMG implementation checkpoint

The subsequent source audit established that the generic-JLA runtime already
owns the sound CMG composition needed by `project()`: it prepares one
source-informed FE hierarchy, shares it with the full W+F+Q block
preconditioner, reuses it for the FE-only solver, charges hierarchy and
workspace memory before estimator RNG, and certifies every inverse action in
the complete original system. The public projection gate, rather than the
native solver, was the missing link.

This generic CMG route is not `CMG_FULL_V2`. The latter is a direct compressed
hybrid solver qualified only for the no-control match-deletion point-estimation
cell and cannot accept the observation-deletion generic-JLA projection
lifecycle or its control block without a new architecture. The smallest sound
implementation therefore admits explicit `preconditioner(cmg)` to the
existing generic projection route, keeps automatic projection routing
withheld, and preserves fail-closed behavior after selection.

A local four-core prototype on the unchanged deterministic inputs produced:

| Rows | Command seconds | Maximum projection iterations | Maximum complete residual | Projection memory forecast |
|---:|---:|---:|---:|---:|
| 6,000 | 6.322 | 17 | `7.14e-11` | 2,753,302 bytes |
| 24,000 | 42.078 | 38 | `8.97e-11` | 10,135,024 bytes |
| 96,000 | 338.682 | 79 | `8.85e-11` | 39,560,968 bytes |

All three commands pass their `1e-9` complete-residual gate, covariance PSD
gate with zero cleanup, and direct-memory admission. On the 6,000-row common
Counter-V1 request, forced CMG and the accepted diagonal route differ by at
most `2.28e-11` absolute across the three projection coefficients and the full
3-by-3 covariance. The prototype therefore removes the observed diagonal
nonconvergence on this 96,000-row graph without changing the statistical
formula path.

After freezing source `33ede86`, the same 6,000-, 24,000-, and 96,000-row
commands returned identical coefficients, full KSS and naive covariance
matrices, residuals, iteration counts, PSD cleanup, and memory forecasts.  The
exact-source command times were 6.627, 42.609, and 344.04 seconds.  Measured
complete-command peak RSS was 186,368,000, 303,464,448, and 451,444,736 bytes.
The 96,000-row command was repeated and reproduced every numerical field.
Compact source, binary, input, harness, result, and resource receipts are under
`evidence/cmg_projection_local/33ede864111c319185949ede4ef6d2bcc44b1383/`.

These local timings are not comparable to the SCC diagonal/MATLAB timings and
do not establish a MATLAB-relative speed or cross-platform reach claim. Such a
claim requires a same-host paired CMG/MATLAB comparison. The accepted diagonal
evidence above remains immutable.

The affected-surface gates pass: pinned Rust formatting, strict Clippy, and
workspace/all-target tests; generated-CMG checks and tests; focused Python
formula, weighting, package, and parity tests; C shim and ABI checks; the public
Stata projection, routing, inference, and planned-generic tests; an isolated
package-install CMG command with controls and nonunit frequency weights; and
the 1,002-row exact/MATLAB oracle. The root Python suite still exposes an
unchanged scale-bundle allowlist defect (three already-installed runtime files
are absent), and Stata 19 still rejects an unchanged closing brace in
`test_rust_public_generic.do`; both fail before reaching this projection
change. They are recorded as baseline harness defects rather than silently
reported as green or expanded into unrelated repairs.
