# Optimization parity and bounded paper refresh — 2026-09-13

This is the source directory for the owner-approved staged implementation.
The active package milestone remains `../../../fevc/PLAN.md`. Work is not
complete and this directory is not permission to launch paper timing.

## Immutable baseline and artifact boundary

Baseline: `../../../.local/optimization-parity-20260913/baseline/`, containing
618 allowlisted current dirty source files and the previously qualified Mac
binaries. Snapshot manifest SHA256:
`4bdba22c0e6da99b3fd597aabab8df4d01307d01704951f3d5d2c089885cd2ad`.
It includes the September 12 deletion-solver and stayer-option repairs above
`fbf8dcd`; the commit alone is not the comparison baseline. Private
`KSS_Veneto_replication/` was excluded. Evidence stays under the adjacent ignored
`.local/optimization-parity-20260913/` area, never the vault.

## Implemented first slice

- Generic no-control pooled match/stayer JLA can use the existing degree-four
  full-CMG solver, fixed k=2 widths and ordered scalar queue. Statistical
  mover-match and physical-observation stayer corrections remain distinct.
- The stayer-hybrid flag now reaches every direct-solver batch and phase memory
  forecast, including pre-pool admission and retained-memory reconciliation.
- Public default/explicit `stayers(both)` no longer excludes the no-control
  automatic full-CMG point route. Omitted `probeorder()` no longer excludes it
  either; existing hybrid probe-order capability restrictions are unchanged.
- Raw implicit-match ingestion remains separately restricted to its original
  mover/key cell. Three caller predicates were audited: the late preparation
  predicate now retains, rather than broadens, the early frozen selection.
- Result reconciliation follows the returned statistical family. A zero-stayer
  automatic call can remain compressed; a real pooled hybrid remains generic.
  The solver extension does not force one statistical engine onto both cases.
- No frozen ABI layout, exact kernel, inference method, installed PLUS package,
  upstream source, paper figure or private input was changed.

Core common-draw tests pass on zero-stayer and stayer-heavy unequal histories,
frequency/target weights, and 1/2/3/4/7/14/28/64 threads. A native V5 augmentation
regression passes. The focused isolated Stata hybrid test passes at 1/4 threads,
checking retained rows, corrected values, widths, residuals, state and release.
These are correctness checks, not performance or 64-core claims.

## Continuation: effective no-control joint point routing

The source candidate now admits the existing direct solver for explicit
`engine(generic)` and `preconditioner(cmg)`, literal frequency weights,
explicit target weights and declared match identifiers. Both deletion modes
and populations retain their statistical families. Effective supported defaults
select the same execution route whether their options were supplied or omitted;
explicit diagonal and explicit batches retain their existing routes. The
compressed core now also rejects a direct plan that tries to override an
explicit diagonal or exact solver selection.

An additive readiness bit (1024) protects the expanded point tuple from stale
plugins. Frozen V5 layouts and the meaning of `threads` when `full_cmg_v2=0`
are unchanged. No inference or control guard was removed. Raw implicit-match
ingestion remains separately restricted to unit-frequency/default-target data
without an explicit deletion ID, even when a probe-order key is supplied.

Focused results in `.local/optimization-parity-20260913/`:

- `point-routing-red-01.log`: the new native regression initially fails on
  the old explicit-generic/CMG exclusion; `point-routing-native-02.log` passes
  16 matched baseline/candidate configurations plus the frozen V5 diagonal
  semantics check. `point-routing-core-01.log` passes the explicit-route guard.
- `point-routing-stata-03.log`: public weighted/unweighted, both-deletion,
  auto/generic-engine and auto/CMG-solver comparisons pass, including genuine
  pooled stayers, omitted tolerance/default flags and explicit diagonal checks.
  These are diagnostic calls, not registered timing observations.
- `point-weighted-key-01.log`: keyed mover calls preserve 2,048 declared
  deletion units versus 1,024 coefficient cells under both statistical engines.
- `point-stale-stata-02.log`: old-runtime rejection passes before preparation
  and RNG, with clean state. Attempt 01 used the non-returning `capabilities`
  subcommand instead of `probe`; that fixture error is preserved.
- `point-routing-python-02.log`: all 805 source tests pass; the fresh dedicated
  pytest directory avoids the earlier nonfatal old-cache cleanup warnings.
  The focused source/benchmark checks also pass (73 tests).
- `point-workspace-tests-final.log` and `point-workspace-clippy-final.log`:
  the complete pinned Rust workspace/all-targets suite and strict Clippy pass,
  including the core/plugin tests beyond the standalone native qualifier.

Mac qualification attempts `native-*-point-01` and `native-*-point-02` stopped
at legacy small-system automatic-routing assertions, not numerical failures.
The two suites retain exact/diagonal automatic solver coverage through explicit
batches; the new point suite covers automatic-batch full CMG. Both focused
legacy-suite retests pass. Attempt `native-*-point-03` completes with process
exit 0: pinned Rust/Clippy/formatting/C-shim gates, arm64 and Rosetta thin and
universal checks, and isolated available/unavailable installation checks pass.
Receipt: `native-receipt-point-03.txt`; evidence: `native-artifacts-point-03/`.
All 190 source inputs rehash unchanged in `point-source-check-final.log`, with
manifest SHA256 `5f0a06c7b9d2b1aeec63c4b64688a2a45c8d89f83362f54206657b3f0a08683b`.
An earlier attempted rehash ran before the qualifier exported its manifest and
therefore had no input; the terminal audit is the authoritative check.
Staged ignored developer binaries (not installed PLUS) are arm64
`2e3b1c0c91a64c5854d52fdc3797cb56c5eff65ace508b7de1c6727bd7b1772f`,
x86-64 `bb7e3b2a2b252539481913e4b46e18f43f2cbea3d1e8d1aaffd30eda3fd07684`,
and universal `4ce4ebd8251a53a0171e5ff06774afec4dee2fb9f204b98a77c73ee6284f0c49`.
All validation processes are terminal. Linux and performance promotion are
still gated on completion of the remaining implementation.

## Continuation: internal controlled direct solver

Controlled point estimation now has a core-only direct path, including joint
and fixed-offset nuisance modes and the genuine pooled mover/stayer correction.
The public Ado/native control guards are deliberately unchanged. This is not a
public control capability, an inference extension or a measured speedup.

The preparation sequence is now explicit: build the shared FE hierarchy with
unallocated solve pools; select and freeze k=2 widths; admit the complete
control-preparation/command peak; allocate the bounded pools; perform the strict
control projections and rank certification; retain and reconcile the control
geometry. Explicit-budget reductions remain frozen after preparation; omission
does not impose memory planning. The existing public two-way constructor still
rejects controls. A separate crate-private borrowed FE sub-operator avoids
copying the compressed problem or pretending to solve its control equations.

The controlled adapter retains the strict FE control coefficients `U` and the
certified small Schur inverse. It computes `gamma=S^-1(b_X-U'b_FE)` and adjusts
the FE coefficients by `-U gamma`, then certifies every original W+F+Q equation.
The native layout is untouched. Internal model diagnostics distinguish explicit
strict-option RHSs, controlled RHSs and full-system control corrections from
ordinary direct FE solves. Control rank retains PCG 1e-13 and complete residual
1e-11; controlled estimation retains its prior solver tolerance, not the loose
default point-probe tolerance. FE solves retain the existing warm-start ladder.
Failing full-model columns alone enter a fixed three-step correction ladder.
Only the correction's numerical null-coordinate drift is projected onto the
existing FE quotient; the original RHS and final complete-system gate remain
unchanged. There is no ridge, rank repair or post-failure route change.

Focused checks and preserved diagnostics in `.local/optimization-parity-20260913/`:

- `controlled-red-01.log`: the new paired regression first demonstrates the
  old control exclusion. `controlled-green-01.log` passes both deletions,
  joint/fixed-offset, weights and 1/2/3/4/7/14/28/64 threads. These are small
  correctness tests, not claims about 64-core performance.
- `controlled-lib-01.log`: preparation initially omitted the empty retained
  control artifact required by the existing no-control generic path; the
  corrected preparation passes all 246 then-current core tests in
  `controlled-lib-02.log` without altering the no-control statistical kernels.
- `controlled-oracles-01.log`: a test initially assumed zero-column batches
  were supported, contrary to the legacy model API; the regression now preserves
  that rejection and separately tests actual zero RHSs. Deliberately perturbed
  control geometry also exposed correction-RHS null-coordinate roundoff.
  `controlled-oracles-02.log` passes independent grounded dense solves,
  control-only RHSs, odd/partial batches, forced correction, fail-closed
  nonconvergence, cancellation and subsequent solver reuse. No gate was relaxed.
- `controlled-pooled-01.log`: weighted, unequal-history pooled stayers pass
  common-draw corrected-target and sample/parameter checks at 1/7/28 threads.
- `controlled-memory-01.log`: separate-process Q=32 checks pass for both
  deletions at 1/7 threads, including more strict projection RHSs than the
  allocated batch capacity. Observed heap peaks are 769,424–782,915 bytes;
  forecasts are 1,691,166–1,692,664 bytes. These are requested heap payloads,
  not process RSS or large-data memory predictions. Exact strict boundaries,
  warn/off budget reductions, singular controls and cancellation are also tested.

The complete pinned workspace/all-targets suite (`controlled-workspace-01.log`),
strict Clippy (`controlled-clippy-02.log`) and 805 Python tests
(`controlled-python-01.log`, 78.70 seconds) pass with terminal exit 0.
Fresh Mac native regression qualification passes with terminal exit 0:
`native-receipt-controlled-01.txt` and `native-artifacts-controlled-01/` cover
arm64/Rosetta thin/universal, C/ABI, and isolated available/unavailable installs.
All 190 native sources rehash unchanged in `controlled-source-check-final.log`;
manifest SHA256 is
`a350e81e47eb16dd9036485145f58b1a9e62982718f94e44d6c1918e9aac480d`.
Ignored developer binaries (not installed PLUS) are arm64
`25ddae576405b0ef223d5e3187d7df2518461ec4f1c78978e28d80221236f4f3`,
x86-64 `900f928841e7d28e5dfd10b0bf0478d61032f33442687ffa38720102d6faaab5`,
and universal `301d04c3a871e352549be2ae5f27d8639fdbe36277f0ad122447a825c1be32c3`.
The 13 graph/campaign-definition checks pass again; that separate pytest run
reports nonfatal cleanup warnings from a pre-existing temporary cache.
All local processes are terminal. Native qualification protects existing public
routes; it does not qualify the new internal controlled path for public use. The candidate still
needs additive control execution/receipt integration and focused native/Stata
control tests, inference/diagonal optimization, Linux and performance gates.

## Five-class campaign sources

`generate.py` derives from the preserved mixed-degree study generator. It adds
fixed degree five, keeps balanced mixed degrees 2–8 at exact mean five, and adds
one degree-four bottleneck (not an extra degree-four well-mixed class). Exact
firm degree is 100. Two-market classes have exactly 1% crossing workers; the
fraction does not shrink with sample size. Degree four uses N/4 workers, while
the other four use N/5, so it is an elimination-threshold diagnostic rather than
a worker-count-controlled degree contrast.

`test_optimization_campaign.py` has 13 passing checks, including all five classes at 8k and
100k. `graph_smoke.do` has ten passing local 8k/P200 calls (both deletion modes,
all five classes). This does not replace the real SCC-launcher smoke.

`campaign.py` writes a **gated definition**, not an executable submission:

| Stage | Configurations | Timed calls | Separate warm-ups |
|---|---:|---:|---:|
| Eight development profiles at 1/7 threads | 16 | 96 | 32 |
| Five synthetic match size sweeps | 15 | 90 | 30 |
| Mixed-degree additional core sweep | 4 | 24 | 8 |
| Five observation checks | 5 | 30 | 10 |
| Private full pooled Veneto at 1/8 threads | 2 | 12 | 4 |
| Paper total | 26 | 156 | 52 |

The generated definition deliberately has no candidate/input hashes and has
status `GATED_NOT_SUBMITTED`. Sources, inputs, commands, expected outputs and
acceptance must be frozen before any timing stage. Use fresh processes,
startup-inclusive post-import primary timing, separate estimator/pool/process
clocks, same-host sequential rotated pairs, actual Matlab worker RNG records,
and physical process-tree RSS distinct from scheduler virtual memory. Matlab
remains pinned to `8b957ffe`. Do not infer equal statistical accuracy from P=200
or three timing repetitions. Preserve the finite-projection formula difference
and the historical private Veneto numerical failure.

## Preserved development failures

- The deliberately new red core regression initially rejected every hybrid at
  `generic_full_cmg`; the implemented routing/memory repair passes it.
- An initial local Stata attempt exposed a remaining late raw-ingestion
  predicate. It was repaired without changing deletion IDs or populations;
  the original failed package/log are preserved in `hybrid-01/`.
- The initial new native fixture accidentally gave a stayer only one physical
  observation; it correctly failed eligibility. The eligible three-observation
  fixture passes; eligibility was not relaxed.
- A new Stata assertion incorrectly expected zero-stayer automatic calls to be
  generic. The test now checks the existing compressed/generic family choice,
  comparing against the corresponding legacy route. Diagnostics are preserved
  under `hybrid-02/`.
- The Python source suite initially found the old manual benchmark insertion
  anchor; updating that anchor restores all 805 tests.
- A combined harness invocation found two test modules named `test_campaign`;
  the new module has a unique name. The fresh combined run passes all 39 checks.
- Mac qualification attempt 01 stopped at the old expectation that omitting
  `probeorder()` must withhold the full-CMG identity. The numerical tests passed
  through that point. The updated route expectation passes its focused retest;
  attempt 02 is recorded separately and passes.

Mac qualification attempt 02 is terminal with process exit 0 and all native,
thin/universal arm64/Rosetta and isolated-install checks passing. Receipt:
`.local/optimization-parity-20260913/native-receipt-02.txt`; sanitized evidence
and exact binaries: `native-artifacts-02/`. Its 189-file source manifest is
`cedaf27b4c9737e8dc3fc9e00bf6bfc37ff65e674d65278c20a39b741d94f8a3`.
The qualifier stages ignored source-local developer plugins, not installed
PLUS. First-attempt artifacts and all failed local logs remain preserved.
The current benchmark adapter additionally admits seven native threads; its
allowlist change is harness-only and is outside that native source inventory.
The final audit rehashes all 189 native-qualified files successfully after the
harness and documentation changes. All local validation processes are terminal.

The bounded operational-repair rule governs released campaign stages. These
development regressions/fixture repairs must not be mislabelled as campaign
estimator calls, nor may they be erased from the engineering record.

## Continuation: public controlled/fixed-offset point integration

The development package now connects the controlled direct solver to public
auto/generic, auto/CMG, automatic-batch point requests for both deletion modes,
both populations and joint/fixed-offset nuisance handling. Fixed offset with
zero controls is included. Explicit diagonal/batches and inference retain
their previous execution. Raw implicit-match ingestion remains joint-only,
no-control, mover-only and subject to its existing key/weight/identifier limits.

Additive readiness bit 11 and `VckssFullCmgModelReceiptV1` protect the new path.
The new model receipt is 56 bytes; frozen V5 and 400-byte/46-column receipts
are unchanged. Native and Stata checks reconcile logical model RHSs
`1+Q+3P+I(Q>0 && fixedoffset)` separately from extra full-system control
corrections. Controlled RHS count is `1+2P` for joint controls, one for fixed
offset controls, and zero without controls. Strict-option RHS count is Q.
Rank exports retain their independent complete gate instead of being
overwritten by the probe-phase summary. Controlled probe tolerances stay
strict even when `tolerance()` is omitted. The point-specific accounting
contract must not be silently reused for future inference work.

Evidence under `.local/optimization-parity-20260913/`:

- `public-controlled-ffi-01.log` preserves an initially incorrect test
  expectation for a short output buffer (ABI_MISMATCH, not INVALID_INPUT).
  The corrected focused test passes in `public-controlled-ffi-02.log`;
  `public-controlled-ffi-all-01.log` passes all 54 FFI tests, including legacy
  comparisons, both deletions/nuisance modes, controls/no controls, 1/4/7
  threads, exact logical/refinement counts, strict rank gates and stale handles.
- `public-controlled-workspace-01.log` passes the pinned complete Rust
  workspace/all-targets suite. Strict Clippy passes in
  `public-controlled-clippy-01.log`; formatting checks pass separately.
- `public-controlled-local-01/package.log` passes the new isolated Stata
  controlled regression: weighted pooled/mover-only joint/fixed-offset models,
  both deletion modes, declared IDs, auto/CMG, omitted/explicit tolerances,
  empty controls, original residuals, sample/RNG/sort state and clean release.
  The native profile additionally runs the strict-budget failure/reuse checks.
- `stale-model-local-01/package.log` passes four pre-RNG rejection cases using
  the prior `native-artifacts-controlled-01` binary, which has bit 10 but not
  bit 11. This does not modify an installed package.
- `public-controlled-python-01.log` retains six failures: old additive helper
  inventory and point-only count assertions, and the unsorted added allowlist
  entry. After updating those checks and sorting the inventory,
  `public-controlled-python-02.log` passes all 805 configured tests.
- `native-qualification-public-controls-01.log` stopped at a legacy diagonal
  bitwise assertion after auto now correctly selected direct CMG. That legacy
  oracle is now explicitly diagonal, retaining its original assertions, and
  its focused retest passes in `native-artifacts-public-controls-01/package.log`.
  The separate direct controlled equivalence test passes. No numerical gate
  or production solver behavior was changed to repair the legacy test.

Fresh source-bound Mac qualification passes with process exit 0 in
`native-qualification-public-controls-02.log` and receipt
`native-receipt-public-controls-02.txt`. It includes the new controlled test on
arm64/Rosetta thin/universal binaries and isolated installations, C error and
interrupt transport, ABI fences and native build checks. All 192 native inputs
rehash in `public-controls-source-check-final.log`; the source manifest is
`35a8d1c85c7675ee7e52581d35b8bf7802168ccf307a6a86eb69895cf00cc484`.
The qualifier SHA256 is
`398be5d53039a186e02bfdb8754699cf52a4056649a48f832f4134cf67233b6f`.
Qualified artifacts under `native-artifacts-public-controls-02/candidates/`
match the ignored source-local staged plugins byte-for-byte:

- arm64: `409b2d3c8840ea0eb26400fdb9b0f05a502e8df7f260c630c596ba9b09de5250`
- x86-64: `cedb276233109d5af646e7df02fdb553d62dea5e8f84d31fbe5ee2e4a186e7c2`
- universal: `da8cbc37f30f22f97a64a9a77745ef5af2b5ad32c888c78152b62ac096998167`

Strict workspace Clippy passes again at the final Rust source in
`public-controlled-clippy-final.log`. All local validation processes are
terminal; installed PLUS is unchanged. This completes the local Mac public
controlled/fixed-offset point integration checkpoint, not optimization parity,
Linux qualification, a performance/paper campaign, private-data work or release.

## Isolated diagonal queue continuation

`model_solver::diagonal_queue` now provides an isolated executor for an
already-selected diagonal model solver. It never substitutes CMG. Its owned
Rayon pool is bounded by permitted threads and frozen maximum RHS capacity;
one scalar RHS at a time is assigned to each worker. This retains the existing
PCG arithmetic, phase-specific options, complete W+F+Q residual certification,
and input-order outputs/errors. Strict options are passed through, not derived
from the looser point-probe tolerance. Host callbacks stay on the caller;
workers use cancellation tokens, are joined before return, and remain reusable
after numerical errors, cancellation and caught worker panic.

Admission includes bounded concurrent scalar scratch, coefficient/residual
outputs, queue slots and duplicate collection headers, a conservative runtime
heap allowance, and explicitly identified thread-stack reservations. Queue and
output capacities are reconciled before work starts. The caller must separately
include its prepared model, borrowed RHSs, earlier retained outputs and other
live command data. Calls on one executor are serialized to avoid multiplying
scratch/queue lifetimes. PCG scratch is still fallibly allocated per RHS; only
the thread pool is reused. The direct Rayon dependency uses the already-locked
1.12.0 version; both lockfiles add only that dependency edge. No vendor edits.

Focused numerical tests use weighted/repeated degree-2--8 graphs with zero/two
controls, independent row-level A'WA predictions, 17 RHSs including zero, and
1/2/3/4/7/14/28/64 requested threads at 1e-10 and 1e-13 PCG tolerances. They match
the legacy diagonal coefficients and iterations exactly. Tests also cover
ordered incompatible-RHS/maximum-iteration failures, cancellation/reuse,
pre-pool cancellation, budget boundaries, omitted/warn/off behavior, overflow,
capacity and wrong-solver rejection. A separate coordinated test proves four
workers overlap, and a 64-worker allocation solves 65 zero RHSs. These are
functional tests, not performance observations or a new 64-core claim.

Preserved development evidence in `.local/optimization-parity-20260913/`:

- `diagonal-queue-tests-01.log`: initial five focused tests pass.
- `diagonal-queue-tests-02.log`: expanded focused and allocation checks pass.
  Incremental measured heap peaks were 132,655 bytes at one thread and 203,789
  at seven, versus heap forecasts 164,880 and 1,308,176; retained heap was
  119,656 and 177,056. The seven-thread stack reservation of 16,777,216 bytes
  is separate. Prepared model/input allocations were outside this measurement;
  it is neither whole-command memory nor physical RSS.
- `diagonal-queue-workspace-01.log`: one scheduling-sensitive test assertion
  failed under full-suite load (it required every tiny solve batch to overlap).
  Numerical checks passed. Replaced only that timing assertion with a bounded,
  coordinated concurrency test; the failed log remains unchanged.
- `diagonal-queue-stata-check-01.log`: standalone Stata crate compiles with
  the updated dependency edge. This is not a native/runtime qualification.
- `diagonal-queue-python-01.log`: all 805 Python checks pass in 73.80 seconds.
- `diagonal-queue-workspace-02.log`: pinned full workspace/all-target tests
  pass, including all 255 core unit tests, eight generic memory tests and
  54 engine FFI tests. Existing inference/exact/point regression gates pass.
- `diagonal-queue-clippy-01.log`: strict workspace/all-target Clippy passes.
  Final `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passes.
- `diagonal-queue-assembly-01.log`: generated CMG assembly check passes without
  edits. No vendored CMG source or dependency version changed.

All checks are terminal. `diagonal-queue-source-v1.sha256` binds the changed
core/dependency inputs to these development checks; it is not a complete native
source/binary receipt. The prior 192-file public-controls native receipt remains
historical and its installed/staged binaries were not replaced by this work.

An initial formatting invocation without a workspace package selected failed
before formatting/testing; reran with `-p vckss-core`. No source or scientific
gate was altered by that command repair.

### Inference audit and next hookup constraints

The main generic attachment already presents several independent RHS groups:
projection columns; three component influence targets; spectrum trace probes
followed by four target transforms per probe; the two spectrum-start vectors;
four q=1 influence targets; and covariance pseudo-outcome probes. Queue only
within these groups, leaving each transform dependent on its preceding base
solve. In `component_apply_target_pair`, the two RHSs within a spectrum step
are independent, but normalization, Ritz rotation and the next iteration are
dependent and must remain ordered.

There is another explicit serial loop in
`generic_jla/residual_moment_attachment.rs`: its projection callback receives
multiple Gaussian columns but calls `solve_with_interrupt` once per column.
That callback needs batched RHS construction, per-column projection receipts
and its own revised live-memory forecast; changing only the main target loop
would leave it serial. Existing counter addressing, basis/fold construction,
strict solver options and residual certificates must remain unchanged.

The legacy controlled CMG block preconditioner holds a synchronized workspace,
so an outer queue there would still serialize FE applications. Its supported
inference hookup needs the separately admitted shared direct solver, strict
phase options, extra logical/refinement accounting and unchanged capability
guards. The current point-only model receipt cannot simply count these extra
inference solves as point probes.

**Not integrated yet:** no estimator, FFI, Ado, installed PLUS or paper route
selects the diagonal executor. Whole-command pre-RNG admission, budget-guided
automatic widths, parallel strict rank setup, diagonal/inference routing and
additive native execution diagnostics remain to be implemented and tested.
Pool/coordinator overhead and per-RHS allocations remain performance questions,
especially for two-column spectral steps. No campaign or performance claim.

## Internal diagonal estimator and projection continuation

The new internal `run_generic_jla_with_diagonal_queue_interrupt` entrypoint
connects the queue to explicit-diagonal generic point/projection estimation.
Existing entrypoints still select their original execution; no FFI or Ado flag
has been reinterpreted. Full and FE-only solvers share one runtime by ownership,
not a borrowed/self-referential solver. Independent RHSs are queued in bounded
physical chunks; scalar fit remains scalar. Strict control-rank projections
use the same admitted FE queue before estimator RNG, retaining 1e-13 PCG and
1e-11 complete-residual gates. A logical Q-column rank/projection group may
exceed physical queue capacity without allocating more active workspaces.

Automatic widths use the fixed k=2 formulas for permitted threads. Omitted
memory chooses those widths without memory-based planning. Explicit widths
remain explicit (capped only by available probes). With an explicit budget,
the fixed powers-of-two-plus-cap ladder is searched jointly: maximize the
product of leverage/target widths, breaking ties toward leverage. A joint
search matters because the shared capacity is `max(L, 2*Tgt)`; independently
admitted widths could otherwise exceed the combined command bound. Strict
budgets reject if no requested/automatic combination fits; warn/off retain
their non-rejecting semantics. This does not add a public batch policy option.

The complete existing generic statistical-phase forecast conservatively
coexists with one queue increment. This includes full/FE preparation, strict
rank, returned logical columns across physical chunks, hybrid/projection
lifetimes, metadata, outputs, runtime allowance and identified stack reserve.
It intentionally does not subtract the old serial scratch estimate. The
runtime validates the frozen forecast again before pool allocation. Separate
internal diagnostics reconcile queue work `Q + 3P + projection_columns`,
permitted/owned/observed workers and the admitted command bound. Fits are not
miscounted as queued work. Frozen native result and execution layouts remain
unchanged; component inference is not selected by this entrypoint.

Development evidence under `.local/optimization-parity-20260913/`:

- `diagonal-estimator-tests-01.log`: the refactored queue passes its existing
  focused regression and allocation test.
- `diagonal-estimator-tests-02.log`: two fixture assertions fail. Projection
  preparation includes an intercept, so two supplied columns produce three
  projection RHSs; use the prepared count. The initial 32-sinusoid control
  fixture is rejected by the baseline's unchanged `AmbiguousControlBasis`
  gate before candidate execution. Replaced that capacity fixture with
  within-cell orthogonal DCT controls, retaining the failure and all gates.
- `diagonal-estimator-tests-03.log`: four integrated suites pass: weighted
  degree-2--8 graphs, both deletion/nuisance modes, all requested thread counts,
  33 probes, explicit/automatic batches, omitted/advisory/strict budgets,
  exact and one-byte-short explicit-batch boundaries, budget reduction,
  pre-RNG/solve cancellation and successful reuse. Pooled unequal-history
  stayers and projection coefficients/covariances agree with the existing
  diagonal path under the unchanged 1e-8 corrected-result rule. A Q=32/P=2
  case certifies all 32 controls through physical RHS capacity four.
- `diagonal-estimator-memory-01.log`: separate-process Q=32/P=2 whole-estimator
  heap checks pass for observation and match deletion at permitted T=1/7.
  All observed heap peaks are 769,424 bytes. Heap bounds are 1,367,104 at T=1
  and 1,923,648 at T=7; the latter owns four workers because capacity is four,
  with 10,485,760 bytes separately reserved for worker/coordinator stacks.
  The T=7 command bound is 12,409,408 bytes. Input preparation is outside the
  tracking epoch; these are heap-payload checks, not process RSS observations.
- `diagonal-estimator-clippy-01.log`: strict pinned workspace/all-target
  Clippy passes.

- `diagonal-estimator-workspace-01.log`: the full pinned Rust workspace and
  all targets pass, including 259 core unit tests, nine generic-memory tests
  and 54 FFI tests. The pre-existing ignored diagnostic remains ignored.
- `diagonal-estimator-python-01.log`: all 805 Python checks pass;
  `diagonal-estimator-assembly-01.log`: generated CMG source check passes.
- `native-qualification-diagonal-internal-01.log`: complete Mac regression
  passes on arm64 and Rosetta x86_64, thin and universal binaries, including
  isolated clean installs. The receipt classifies this as
  `LOCAL_CHECKPOINT_DIRTY_TREE`, not a release. All 196 native source inputs
  rehash successfully; the source manifest SHA256 is
  `b996a3319fdd5729b38404f0cb83cb74b6e00aea59b2649b9bc63ab30b0f2420`.
  The qualifier SHA256 remains
  `398be5d53039a186e02bfdb8754699cf52a4056649a48f832f4134cf67233b6f`.
  Exact tested artifacts in `native-artifacts-diagonal-internal-01/` match the
  staged ignored source-local plugins: arm64
  `ca2a52b6c1301806e824acc8c3b7d694eed37f10690382a81a226aa076884572`,
  x86_64 `3fd79679dc2fbfdf09e1084af65062855ddff3afaa51d21a88370398cca72d33`,
  universal `f75994867b764faf8297d10f3bdfd07e04859b04f7b9c690ff375854f504cbb0`.

All checks are terminal. Native regression covers existing public routes;
the new diagonal entrypoint is tested in Rust but is not selected by FFI/Ado.
This is internal point/projection integration, not a public diagonal speedup,
component-inference completion, Linux qualification or authority to start
paper timing. Installed PLUS, paper files and private Veneto are unchanged.

## Internal diagonal component-inference continuation

The additional internal attachments entrypoint now selects the same diagonal
queue beneath eligible observation/joint and match/fixed-offset component
inference. No FFI/Ado selector, public option, frozen layout, statistical
capability, variance formula, RNG address or phase tolerance changes. The
existing public/legacy entrypoints retain the scalar residual-moment callback.

Independent influence, covariance, trace-transform, spectrum-start and paired
spectrum-iteration groups use the shared executor. The two successive target
applications, normalization, Ritz rotation and next spectral iteration remain
dependent and ordered. Residual-moment projection callbacks now pack fallibly
allocated coefficient RHSs in bounded physical chunks when the internal queue
is selected. Prediction, certificates and probe receipts are consumed in input
order. The declared inference/Gram batch widths remain unchanged, while
physical chunks cannot exceed the point planner's admitted queue capacity.
This is not an automatic inference-width tuning policy.

The whole attachment, retained outputs and additional callback packing memory
enter the joint point-batch search before pool creation and strict rank setup.
The final command bound must equal that pre-pool bound. Omitted memory still
performs no budget planning; explicit widths remain explicit. Actual successful
inference solve receipts and residual projection receipts extend the queue's
point/control count, preserving target-local spectral unavailability without
pretending every maximum-possible iteration executed.

Evidence in `.local/optimization-parity-20260913/`:

- `diagonal-inference-point-regression-01.log`: four existing internal
  point/projection suites pass.
- `diagonal-inference-tests-01.log`: test compilation stopped at an incorrect
  counter-receipt field name and unused import; no estimator executed.
  Corrected the test reference without changing any gate.
- `diagonal-inference-tests-02.log`: initial three inference suites pass.
  `diagonal-inference-tests-03.log`: expanded four suites pass, including both
  q1 deletion modes, computed and unavailable targets, both legacy structured
  fits, all requested thread settings, odd/partial groups, 513 direct Gram
  probes, exact budgets, rejection before queue setup, automatic reduction,
  cancellation and reuse. Corrected targets and covariance/interval outputs
  agree at the 1e-8 scale-relative engineering check; all existing stricter
  baseline scientific/residual tests remain intact.
- `diagonal-inference-memory-01.log`: separate-process whole-estimator heap
  checks pass with two controls, 200 point probes, 513 Gram probes and physical
  capacity four. Observation T=1/7 measured 270,216/305,104 bytes versus heap
  bounds 707,536/1,241,040; match T=1/7 measured 328,624/363,512 versus
  690,176/1,223,680. T=7 owns four workers and reserves 10,485,760 stack bytes
  separately. These are heap-payload checks, not process RSS observations;
  input/attachment preparation is outside the tracking epoch.
- `diagonal-inference-clippy-01.log`: strict pinned workspace/all-target
  Clippy passes.
- `native-qualification-diagonal-inference-01.log`: launcher preflight stopped
  because its required empty artifacts directory had not been created. No
  compilation or Stata call ran. Created a fresh empty `...-02` directory and
  restarted into separate `native-*-diagonal-inference-02` evidence paths.

- `diagonal-inference-workspace-01.log`: full pinned workspace/all-target
  regression passes, including 259 core unit tests, 52 generic integration
  tests, ten generic-memory tests and 54 FFI tests. The one pre-existing ignored
  diagnostic remains ignored.
- `diagonal-inference-python-01.log`: all 805 Python checks pass;
  `diagonal-inference-assembly-01.log`: generated-source check passes.
- `native-qualification-diagonal-inference-02.log`: Mac arm64/Rosetta,
  thin/universal, existing public inference and isolated-install regression
  passes. All 197 native source inputs rehash at manifest SHA256
  `de6dc32cba6711634f63a857b403a8fdbe198df98a8076b345a319ee0f372ea0`.
  Qualifier SHA256 remains
  `398be5d53039a186e02bfdb8754699cf52a4056649a48f832f4134cf67233b6f`.
  The staged ignored source-local plugins match the receipt's tested artifacts:
  arm64 `816706122cafaaff5ad1fc333c44463a27264973e0bfb928358cbdebcf19fc25`,
  x86_64 `cf7394e835780917d04dc25fe0265829030830f6004c9467bd70c29bcc5b5324`,
  universal `d80a85e594340345811b8f80d379b8279ad38fdcda5eb2716ec547f8024ec5d4`.

All checks are terminal. The new internal queue entrypoint is qualified by
Rust tests, not selected by the native tests; native regression covers existing
public execution. Public diagonal/inference routing, direct-CMG inference,
Linux qualification and any performance claim remain gated. No installed PLUS,
paper, private-data, upstream, commit or push action occurred.

The public inference batch handoff needs particular care in the next boundary
slice: `fevc.ado` materializes automatic `batch()` to a numeric width before
`_fevc_rust_component_attach`, and the frozen native augmentation carries that
positive width rather than omission intent. Current residual constructors also
cap Gram width at 16. Do not reinterpret a literal eight (or any explicit width)
as automatic. Any new thread-aware default must preserve explicit intent and
legacy request meanings through an additive execution boundary; the current
internal tests deliberately preserve those declared widths.

## Internal direct-CMG attachment continuation

The additive Rust-only `run_generic_jla_with_direct_attachments_interrupt`
connects explicit CMG to existing projection and component-inference tuples.
The ordinary direct entrypoint remains point-only; no FFI/Ado selector,
capability bit, public option, installed PLUS package or frozen receipt layout
changes. Fusion and upstream CMG remain untouched. Automatic inference-width
intent is still a separate boundary issue, not inferred from numeric widths.

The attachment planner prices both point-batch axes jointly on the fixed k=2
ladder. Candidate capacities are forecast without allocation or mutation; the
selected capacity includes the shared scalar pool and all declared attachment,
logical output, control-preparation, queue and packed Gram callback lifetimes.
Admission precedes solve-workspace allocation and strict control-rank solves.
Omitted memory skips budget-driven planning; explicit advisory budgets can
select minimum capacity, while strict budgets reject below the complete bound.
Declared inference widths are preserved and physically chunked as necessary.
The original model equations, strict phase options (including Q=0 attachments),
fixed refinement ladders and dependent spectral steps remain enforced.
Separate internal work records distinguish fit, rank, point probes, projection,
component and Gram RHSs; their sum plus full-model corrections reconciles the
existing solver counters, without repurposing the point-only native receipt.

This work exposed a pre-existing numerical defect in the shared final 2-by-2
Ritz rotation. Its choice between equivalent eigenvector formulas preferred
the cancellation-prone formula near a diagonal matrix. In the observation,
controlled, one-thread oracle fixture, point/trace results agreed but the
candidate worker eigen-residual was about 0.135 and its target became
`ModeNotCertified`; the existing route's residual was about 4.3e-15.
The new deterministic signed/near-diagonal matrix regression also failed
before repair (a 1e-10 residual for a matrix with 1e-10 off-diagonal entries).
Selecting the larger of the two equivalent vectors repairs cancellation;
the recurrence, fixed iteration count and acceptance thresholds are unchanged.
This shared arithmetic repair affects existing component routes as well as the
internal candidate, so native regression is required. Historical confirmations
and their calibration limitations remain unchanged; this is not a new coverage
claim or permission to relax target-availability gates.

Evidence below is in `.local/optimization-parity-20260913/`:

- `direct-inference-existing-01.log`: four existing diagonal-component suites
  pass before the shared Ritz repair.
- `direct-inference-tests-01.log`: three tests stopped at a test-only tolerance
  expectation (1e-11 versus the fixture's requested 1e-12); production already
  preserved 1e-12. Test checkpoint names were also corrected before use.
- `direct-inference-tests-02.log`: two suites pass; the oracle matrix exposes
  the real target-availability failure above. Its focused reproduction is
  `direct-inference-status-diagnostic-01.log`.
- `direct-inference-ritz-before-01.log`: independent 2-by-2 residual regression
  fails. `direct-inference-ritz-after-01.log`: repaired arithmetic passes all
  36 signed/diagonal/near-diagonal matrix cases.
- `direct-inference-tests-03.log`: all three new suites pass. They cover both
  deletion/nuisance modes, controls/no controls, applicable frequency/target
  weights, 1/2/3/4/7/14/28/64 threads, oracle and both structured models,
  q0/q1 computed/unavailable targets, 513 Gram probes, capacity-two partial
  groups, strict/advisory/omitted budgets, pre-workspace rejection, cancellation
  and reuse. The old direct entrypoint still rejects attachments. Unit coverage
  at 64 threads is not a 64-core performance claim.
- `direct-inference-memory-01.log`: separate-process Q=32 whole-estimator heap
  checks pass at one/seven threads. Observation measured 802,688/813,427 bytes
  against 2,920,974/2,922,593-byte command bounds; match measured
  802,744/813,747 against 2,921,030/2,922,649. Capacity two splits 32 strict rank
  RHSs and seven-column Gram callbacks. Retained CMG storage is 1,192/1,608
  bytes, with one/two workspaces of 416 bytes each. This tiny dense-terminal
  case measures heap, not physical RSS or a general memory-accuracy ratio.
- `direct-inference-clippy-01.log` and `direct-inference-workspace-01.log`:
  test-only compilation failure from using nonexistent `PcgOptions::default`.
  `direct-inference-clippy-02.log` preserves the subsequent test-field spelling
  mistake. `direct-inference-clippy-03.log` passes strict pinned checks after
  explicit test settings were corrected.
- `direct-inference-workspace-02.log`: full pinned workspace/all-target
  regression passes: 261 core unit tests, 55 generic tests (one pre-existing
  ignored diagnostic), 11 generic memory tests, 54 FFI tests and all other
  workspace suites. Capacity-forecast purity/realized-pool checks and weighted
  mixed-degree pooled projection comparisons are included.
- `direct-inference-python-01.log`: all 805 Python checks pass (76.76 seconds).
  Formatting and `direct-inference-assembly-01.log` generated-source checks pass.
- `native-qualification-direct-inference-01.log`: source-local Mac qualification
  passes on arm64, Rosetta x86_64, both universal aliases and isolated installs.
  Existing observation/match/individual component suites exercise the shared
  Ritz repair, not the internal attachment entrypoint. All 199 inputs rehash at
  manifest `a9714b2f31d99753ea79bb1914eaa09853db15d8b80e838d2efb2b6572a8e8a4`.
  Receipt classification is `LOCAL_CHECKPOINT_DIRTY_TREE`, with unchanged
  qualifier hash `398be5d53039a186e02bfdb8754699cf52a4056649a48f832f4134cf67233b6f`.
  The staged ignored source-local binaries match the tested artifacts:
  arm64 `51ce7cdb21f94bba5480abe1e8d038f4ffb37e2aab8cc8cfc400c1cf40a406dc`,
  x86_64 `75bdc476108d36e190d21d9599635b3c13b127349bfa961c9fba8ee6e0688ad0`,
  universal `0acd8fef42cd2f602d9faf9c1e5a4278d0ebf23978e7d91e6fd8d57f855a3685`.

- `direct-inference-package-01.log`: final integrated package audits,
  805 Python checks, CMG qualification, Stata quick/full suites, clean install
  and small synthetic/separations harness smokes pass, ending in
  `FEVC LOCAL QUALIFICATION PASS`. No Matlab comparison or paper run was added.
  The one-second `direct-inference-package-stata-sample-01.txt` diagnostic
  showed active preparation/solve work during the lengthy quick suite, not an
  input/launcher wait. It is not benchmark or representative-RSS evidence.

All checks are terminal. The final post-package source verification rehashes
all 199 inputs; source-local plugin hashes still match the native receipt.
No performance/paper gate has passed. Public execution/accounting, Linux,
the bounded development screen and the guarded paper runner remain unfinished.

## Additive native execution continuation

`generic_execution_api.rs` connects the tested diagonal queue and direct-CMG
attachment runners through a separate V6 native request (304 bytes, exact V4
prefix) and interrupt request (328 bytes). Readiness bit 12 identifies the
interface on Mac/Linux builds without extending statistical capabilities.
Explicit generic JLA, Counter-V1 and no fallback are required. Diagonal keeps
explicit/automatic point batch intent; direct attachments require explicit CMG,
automatic point batches and an actual projection/component attachment.
The existing augmentation API retains literal inference/Gram widths, including
eight. This slice does **not** infer omission from eight or add a public Ado
option/dispatch. That remaining transport and omission-intent work precedes
the development timing screen.

The separate 160-byte generic execution receipt reconciles fit, strict rank,
point, projection, component and Gram RHSs. Diagonal queue counts exclude scalar
fits; CMG counts include fits and extra full-model refinement. It reports
permitted threads, planned workers, capacity/point widths and complete allocation
forecast. Diagonal maximum active workers are measured; CMG selected concurrency
is exported in a distinct field, with measured activity zero/unavailable. The
complete residual maximum includes Gram callbacks. The 56-byte point-only model
receipt explicitly rejects direct attachment execution; V1--V5 layouts and
the old V5 flag-zero/ignored-thread meaning are unchanged. No upstream or
scientific algorithm changes were made in this slice.

Evidence under `.local/optimization-parity-20260913/`:

- `native-execution-check-01.log`: initial compiler checks caught two incorrect
  field names in the new receipt builder; corrected to existing peak/capacity
  fields. No run occurred.
- `native-execution-tests-01.log`: test-only output-initialization and type
  annotation compilation errors, corrected without changing production logic.
- `native-execution-tests-02.log`: four initial suites pass (24.56 seconds),
  including both-mode/controlled point comparison, 1/2/3/4/7/14/28/64 thread
  settings, V5 compatibility, structured component/513 Gram counts, literal
  inference widths seven/eight, stale buffers/generations and cancellation/reuse.
- `native-execution-tests-03.log`: the new generic-dense projection fixture
  failed the unchanged **legacy** covariance PSD gate. The other failures in
  that process were shared-test-lock poisoning, not independent solver failures.
  `native-execution-clippy-01.log` passes strict pinned checks.
- `native-execution-tests-04.log`: two suites pass, then a test expected
  `UnsupportedFeature` where the existing core correctly reports `InvalidInput`
  for absent attachments; subsequent failures are test-lock poisoning. Corrected
  only the expected error category.
- `native-execution-projection-01.log`: the positive fixture copied from
  `test_rust_projection.do` accidentally supplied an extra intercept; native
  preparation already adds one and correctly rejected the singular Gram.
  `native-execution-projection-02.log`: after removing that duplicate, the
  fixture's match/joint covariance is still legitimately indefinite. Its other
  three deletion/nuisance cases succeed. These are test-fixture findings, not
  evidence of changed inference calibration or grounds to weaken a gate.
- `native-execution-memory-01.log`: explicit-width native memory-policy test
  passes at the exact allocation bound, rejects one byte less, and preserves
  widths with omitted/advisory/off budgets.
- `native-execution-tests-05.log`: all seven expanded suites pass (27.91 seconds).
  All 20 projection attempts are printed: old, queue T1/T7 and direct T1/T7
  agree on three successful cells and the match/joint PSD rejection. Positive
  coefficients/covariances and negative typed withholding are checked separately.
  Tests also verify foreign callbacks stay on the caller thread, work counts,
  strict residuals, legacy semantics, unsupported tuples and terminal reuse.
  Thread-64 tests establish API/correctness coverage, not 64-core performance.

- `native-execution-workspace-01.log`: complete pinned workspace/all-target
  regression passes, including 261 core unit tests, 55 generic integrations
  (one pre-existing ignored diagnostic), 11 memory tests and **61 FFI tests**.
  `native-execution-clippy-02.log` and final formatting checks pass.
- `native-execution-smoke-build-01.log`: C harness compilation caught a
  nonexistent cancellation-status macro; the harness now checks the existing
  `ErrorCode::UserBreak` value. `native-execution-smoke-build-02.log` passes
  strict warnings for both architectures. Harness source SHA256 is
  `ac11fbb86cd64e1a17d89418e027956acee27be211bb46ee0b31106bcdaea7bd`,
  executable SHA256
  `319476c221c262cc4ea99cd17982d6112e37a18349f57341b4758804612c6acb`.
- `native-execution-library-{arm64,x86,universal-arm64,universal-x86}-01.log`:
  all four compiled-library profiles pass actual C-ABI V6 q0 inference execution,
  both deletion units, caller-only callbacks, deliberate cancellation and
  generation-safe reuse. Each profile has four successful calls plus four
  cancelled calls, not timing observations. Every successful call reconciles
  2,277 logical RHSs; complete residuals are at most 2.77e-15. Corrected point
  results agree between queue and CMG. This extends the Rust-level FFI checks
  to the exact shared binaries, not public Stata routing or calibration.
- `native-qualification-execution-01.log`: complete Mac arm64/Rosetta
  thin/universal native and isolated-install qualification passes, classified
  `LOCAL_CHECKPOINT_DIRTY_TREE`. All **201** source inputs rehash at
  `738e36db39c4eb7da4ac2ed5d3839742fea3d056b148786b497ec4cd9a56f803`;
  `native-execution-source-verify-01.log` records the check. The unchanged
  qualifier SHA256 is `398be5d53039a186e02bfdb8754699cf52a4056649a48f832f4134cf67233b6f`.
  The source-local staged ignored binaries are identical to the library smokes:
  arm64 `ca15d326936f3c4a206b8ab9ab28b2b7bfb87c3b491706c62f899a1c7217bc8e`,
  x86_64 `13ce7c8839bb5858fa41e51babca8e7c596fc1cf03c5ab86ea2a476ad2512afb`,
  universal `1925aa00f2f9fa142b70dd96f185ec768abfa11c972ffeee36fddda7d0eaa0a4`.

- `native-execution-package-01.log`: final integrated source/history/license/
  provenance/parity/package audits, all 805 Python tests (75.40 seconds), CMG
  qualification, Stata quick/full, clean installation, tiny synthetic and
  B1/CMG/separations fixture smokes pass. Terminal marker is
  `FEVC LOCAL QUALIFICATION PASS`; this is not a Matlab timing comparison.
  `native-execution-final-source-verify-01.log` rehashes all 201 native inputs
  after the package run; staged ignored plugin hashes still match the receipt.

All checks are terminal and passing. The public Stata dispatch, inference
omission intent, Linux qualification and all timing/paper gates remain pending.
Installed PLUS, private data and old evidence are unchanged.

## Continuation: public V6 execution transport

Separate C selectors `solveexecution` and `executionreceipt` connect the V6
executor without repurposing V4/V5. The 160-byte receipt is checked before any
scalar export, including exact integer range, checked logical sums, queue/CMG
identities, workspace capacity, generation and finite residuals. Failed exports
release the generation; Ado clears all transport scalars on success or error.
Public dispatch now selects explicit generic/JLA/diagonal (automatic or literal
point batches) and explicit generic/JLA/CMG attachments (automatic point batches).
The V5 full-CMG point path and separate 56-byte point-model receipt are preserved.
Mode-aware joint admission is reconciled exactly, not as a second post-plan
component allocation. Logical component RHSs exclude the separately counted
Gram work; the old component receipt continues to include it.

The focused C transport regression passes parsing/default/header/interrupt,
both modes, malformed arguments, 14 corrupt receipts per mode and failed scalar
exports (`execution-cshim-test-01`). Source-local arm64 build passes in
`execution-stata-build-01.log`. All isolated packages are under
`public-execution-NN/`; none replaces installed PLUS. The Stata attempt ledger:

- 01: automatic point execution passed, but literal batches still selected
  the legacy wrapper. Corrected selection only for already-supported tuples.
  Diagnostic/trace outputs are retained in this attempt directory.
- 02--05: match/pooled work reconciliation exposed local-macro decimal rounding
  of the complete residual; the final diagnostic isolated the execution gate.
  Comparison now retains the original binary64 scalar. Other gates passed.
- 06: 72 point cells and projection passed; the test requested 33 simulations,
  below the existing public minimum. Corrected the fixture to 129, not the gate.
- 07: an altered uniform-target inference fixture failed FULL_RESIDUAL_FAILED.
  All eight candidate combinations and the same eight calls against immutable
  baseline (`execution-inference-baseline-01/diagnostic.log`) fail that gate.
  This is an unresolved baseline limitation, not positive evidence. Restored
  the established heterogeneous target masses for the positive regression.
- 08: positive inference exposed the old post-plan memory increment assumption.
  Added execution-aware reconciliation of V6's jointly admitted command peak.
- 09: all 23 corrupt-work cases passed; a test-only attempt to reload an already
  defined command failed. Restore only the deliberately overridden call helper.
- 10--11: inference succeeded but public logical accounting counted Gram work
  twice. Preserve the legacy component total and subtract Gram only when
  reconciling the new disjoint work receipt. Diagnostic matrices are retained.
- 12: the complete new public execution suite passes, including all 72 point
  cells, projection, both component deletion/nuisance modes, all corrupt work
  fields, interruption/reuse, exact explicit widths and memory policy.
- 13: the legacy generic suite exposed misclassification of a valid Q33
  capability decline. Unsupported V3 capabilities now retain their typed reason
  and fail before preparation; this does not increase the controls limit.
- 14: preserved first/second/final logs show obsolete expected solve phase and
  singular-status names, followed by corrupted-capability preparation. The
  planned path now independently reproduces the V3 fixed-domain FNV request
  signature, including little-endian binary64 wall bytes, before preparation.
  Updated only route/schema-specific legacy assertions; numerical tests remain.
- 15: `generic-test.log` and `package.log` both pass. The former includes
  missing/corrupt capability rejection before preparation and all existing
  negative-result/caller-state tests. The latter additionally passes six
  independent Python-packed V3 signature vectors (zero, fractional, tiny and
  large wall values and a >32-bit physical limit).

`stale-execution-01/package.log` passes six public requests against a bit-11-only
runtime; all reject before preparation and preserve caller state. The first
805-test Python run has 803 passes and two source-assertion failures: one old
full-CMG-active expression and the manual adapter's exact thread-line anchor.
Both focused repairs pass all 43 relevant tests in
`public-execution-python-focused-02.log`; the adapter semantics are unchanged.
Generated-source and formatting checks pass. The fresh full Python run
`public-execution-python-02.log` passes all 805 tests in 78.93 seconds with a
dedicated empty pytest directory. Full integrated checks are pending.

Mac qualification attempt `native-*-public-execution-01` passed pinned Rust,
strict Clippy, formatting, C/ABI, all binary builds and the new arm64 suite, then
stopped on the legacy generic suite's V2-only schema assertion. That test now
expects planned V3 only when bit 12 is present, retaining the private old-scalar
reference. The repaired focused suites pass at attempt 15. Fresh qualification
`native-*-public-execution-02` passed all arm64 scientific suites, including
component and individual inference, then stopped in the backend-routing test:
the capability-fault mock still advertised 511 readiness flags, so V6 preflight
correctly rejected it before that test's intended capability query. Only the
capability-fault mock now advertises 8191; the separate stale-runtime mock
retains 255. Focused `public-execution-16/package.log` passes. Fresh
`native-*-public-execution-03` passes the complete Mac arm64/Rosetta
thin/universal and isolated-install qualifier. All 202 source inputs rehash in
`public-execution-source-verify-03.log` at manifest
`e4c806bcb114059345a563c4fa7f2d69eaf41b16ac4253e7ecb2dc4e60085fc6`.
Its source-local candidate hashes are arm64
`b61312cce3bd6576ecc13da17d74cacf89197b22a9711b4d9efb9228c3c7661e`,
x86_64 `edb5a087a2e7290c8c66b217bedc956282f56f6446acd3d6b593793e71f7b643`,
and universal `40e4512e1a9cbe93c7ff13cbf9a3f59c3a2b906dfc54a76c7c819691408eeb7b`.
The pinned qualifier is `c8e1c37b52bdcf288d5ece3b4ba100accbc30b9687e0b0e8416786fad446e15c`.

An additional old-binary check then exposed a distinct preflight gap:
`stale-transport-01/package.log` pairs the earlier FFI-only bit-12 binary with
new Ado. It reports INVALID_INPUT at solve_jla, with one prepared/released
generation; the process and RNG were clean, but rejection was too late and
misclassified. Native readiness alone cannot certify Stata command transport.
The C probe now separately exports `execution_api=1`; new Ado clears any old
scalar, defaults absence to zero and requires both native bit 12 and transport
API 1 before preparation. No native ABI layout, statistical capability or
signature domain is changed. The C test checks successful and failed metadata
export without taking ownership of a generation (`execution-cshim-test-02`).

`stale-transport-02/package 1.log` and `stale-execution-02/package 0.log`
each pass all six rejection cells with last_released=0 and caller-state
restoration, including a deliberately cached readiness scalar. This covers
both the FFI-only bit-12 binary and the older bit-11-only runtime.
`public-execution-17/routing-test.log` and `package.log` pass the repaired
routing proxy and complete new execution suite. The source-local build passes
in `execution-stata-build-02.log`; the second full-source retest passes all
805 tests in 74.09 seconds (`public-execution-python-03.log`). Fresh Mac
qualification `native-*-public-execution-04` passes with terminal process exit 0:
arm64 and Rosetta thin/universal public execution, legacy regression, available
and unavailable isolated-install checks, pinned Rust/strict Clippy/formatting,
C transport and ABI tests all pass. All 202 source inputs rehash in
`public-execution-source-verify-04.log`; the manifest is
`900714be700c6cf266f56dc7cb43e04fe51bbbf1f30f9be6598f9c5a08349630`.
Source-local ignored binaries match the receipt exactly: arm64
`0b53795f8dead183ae3d180335cd9dddba4da015c8f4a6f126b3bb94b3c388fa`,
x86_64 `a19fb4587dcc2ae1af41ba057d42fa2bbde80313516e9f52fbb8831fa3cc9786`,
and universal `1f6c2c7f6bdafbc318cfe03aa1f86672605431df6eac8e7cc9862582fa2ce11b`.
The qualifier hash is unchanged from attempt 03. All local test processes are
terminal; this is a dirty-tree Mac checkpoint, not Linux, performance, paper or
public-release qualification. Installed PLUS is unchanged.

`public-execution-source-audits-{01,02}.log` pass historical-name/identity/history,
license/provenance, parity, portable release-artifact and deterministic CMG
checks; attempt 02 covers the final transport guard and verifies the 45-file
portable artifact at `d3e9b2bca537c1f6a145a908a1274d8cc74b67eca2b6c08104e80b14965fd11b`.
The final whole-package run is deferred until remaining automatic
route and inference-width work is complete. This slice uses full affected
native/public/clean-install checks plus all Python source tests; the earlier
`native-execution-package-01.log` remains historical evidence for unchanged
Mata-only algorithms, not a fresh integrated PASS for the current source.

## Remaining implementation gate

The full approved plan remains incomplete. In particular:

1. The diagonal queue now has internal point/projection admission and batch
   planning; V6 native and explicit public dispatch are implemented. Complete
   effective automatic routing without changing solver choice. Internal diagonal
   and direct-CMG component inference have separate admission and solve
   accounting. Fixed-offset point execution uses the direct route where
   eligible. Explicit batches keep legacy behavior and cannot request full CMG.
2. Preserve the controlled point route's separate additive accounting and
   strict rank gates; V6 attachments have their own work receipt.
3. Preserve the qualified explicit public V6 routing, strict tolerances, explicit
   batch intent and dependent recurrences. Carry inference omission intent explicitly before
   attachment admission/RNG; do not reinterpret the legacy numeric eight.
4. New private execution capability, if needed, must be additive. Do not reuse
   a reserved/frozen field or claim thread use without an actual worker plan.
5. All affected-surface native/install checks and source-bound Linux checks,
   then the frozen 96-call development screen, must pass before paper timing.
6. The guarded execution/collection/timing harness and paper promotion remain
   to be implemented; neither the definition nor local smoke is a launch.

## Internal automatic inference-batch continuation

The internal `run_generic_jla_with_automatic_component_batches_interrupt`
entrypoint now carries explicit automatic inference intent to either the
diagonal queue or direct-CMG attachment executor. Existing entrypoints remain
literal, including numeric eight and the older Gram cap of 16. This is not
yet selected by Stata or any frozen native solve/augmentation interface.

A borrowed execution view overrides only covariance/spectrum and Gram widths;
it neither clones retained variance arrays nor mutates the prepared generation.
The fixed k=2 policy caps the common automatic inference ladder at
`min(max(covariance probes, spectrum probes, Gram probes), max(32, 8*T))`,
then caps each phase by its own probe count. Covariance and Gram probes each
produce one initial RHS; spectrum target columns retain their existing four
actions per probe and split through the admitted physical pool. Dependent
spectral iterations, RNG domains, strict solves and numerical gates are unchanged.

Explicit budgets now jointly price leverage, target and the automatic inference
axis before pool allocation/RNG. Selection maximizes the product on the fixed
ladders, breaking ties by leverage then target width. The pool capacity is
`max(L, 2*Tgt, inference width)`. Omitted memory selects the caps without budget
planning; advisory/off over-budget requests use minimum automatic widths.
Literal point and inference choices are not reduced. Full CMG still rejects
explicit point batches. Separate internal diagnostics retain original widths,
selected component/Gram widths, policy, selection reason, thread count and cap;
no native receipt layout changes.

Focused evidence under `.local/optimization-parity-20260913/`:

- `automatic-component-check-01.log`: pinned core/test compilation passes.
- `automatic-component-tests-01.log`: two new tests stop at an old test-helper
  assumption that trace receipts have the same storage order across widths.
  Trace batches store base solves followed by four target groups, so the order
  changes. Numerical comparisons passed up to that assertion. The new
  width-changing comparator checks the complete `(phase, logical probe)`
  multiset including duplicate keys and every original residual gate; existing
  unchanged-width tests retain their ordered check. Production receipt ordering
  and numerical code were not changed to repair the test.
- `automatic-component-tests-02.log`: both new suites pass (31.45 seconds).
- `automatic-component-tests-03.log`: all six focused suites pass (30.86 seconds),
  including both deletion modes and solvers, 1/2/3/4/7/14/28/64 threads, literal
  eight, oracle/structured q0/q1 with computed and unavailable targets, 513 Gram
  probes, exact/one-byte-short budgets, pre-pool rejection, cancellation/reuse,
  invalid/overflow requests and preserved explicit point widths.
- `automatic-component-memory-01.log`: all eight separate-process Q=32 heap
  checks pass at one/seven threads, both deletion modes and both solvers.
  Observed heap is 0.775--0.992 MB against conservative 2.688--4.895 MB heap
  bounds. Diagonal seven-thread stack reservation is separately 16 MiB; these
  are allocator payload observations, not physical RSS or performance evidence.
- `automatic-component-clippy-{01,02}.log`: strict pinned workspace Clippy passes.
- `automatic-component-python-01.log`: 805 tests pass (80.13 seconds).
- `automatic-component-assembly-01.log`: generated-source check passes.
- `automatic-component-workspace-01.log`: full pinned workspace/all-target
  regression passes, including 262 core unit tests, 58 generic integrations
  with one existing ignored diagnostic, all 12 memory suites and 61 FFI tests.
  This also passes the final intermediate-budget and selection-reason checks.
- `automatic-component-standalone-01.log`: 10 standalone Stata crate tests and
  six build-boundary checks pass; no plugin artifact was built/staged for use.
- `automatic-component-format-01.log`: pinned format check passes.
- `automatic-component-source-01.sha256`: all 203 source inputs are bound by
  manifest SHA256 `99b1ec7ddcc6c9136cc6f9593d2b2dccca6c8956fee31927741907b43bf97ae2`.
  `automatic-component-source-verify-01.log` rehashes all 203 successfully.
  The source-only snapshot `automatic-component-source-01.tar.gz` contains
  exactly those files, SHA256
  `5a949f127819b68e9ac9d84309d5bd249eda3d716b8c668286bef67c895d7f4a`;
  private data and binaries are excluded.

All local checks are terminal and passing. No
plugin was rebuilt/staged, installed PLUS changed, SCC job submitted, timing
campaign released, paper updated, commit/push made or Telegram sent. The previous
202-file native qualification remains historical and does not qualify this
new internal path. Native/public integration and its qualification are still
required before the development screen.

### Remaining boundary audit

The new internal automatic-width entrypoint is ready for an additive native
handoff. Preserve omission intent through **augmentation admission as well as
solve**: carrying a flag only at solve is insufficient if legacy augmentation
has already forecast/rejected a literal width of eight. Original augmentation
requests, V6 meanings and native work-receipt layouts remain frozen. Account
for the larger inference-owned capacity in new work/width reconciliation;
never merely relax a legacy capacity equality. Add matching native/C transport
readiness and reject older binaries before preparation/RNG. Explicit solver
restrictions on inference remain unchanged.

`rust_execution_eligible` and
`_fevc_rust_generic_planned` currently require explicit `algorithm(jla)` and
`engine(generic)` for V6. `generic_execution_api::validate` independently
enforces that same tuple, so simply rewriting the Ado request to look explicit
would corrupt capability/request provenance. Automatic effective selection must
be carried through native planning before RNG, including exact/compressed
non-applicability and pre-RNG automatic CMG fallback. Separately,
`_fevc_rust_component_attach` currently forwards the normalized numeric public
batch (eight for automatic) into legacy augmentation V4; its native options
validate a positive literal width. Omission intent must cross an additive
boundary, not be reconstructed from eight. Projection and dependent spectral
recurrences retain their separate scheduling/strict-option constraints.

Native routing entrypoint for the next slice:
`solve_engine_v4_with_generic_execution` first validates the original V3
capability signature against prepared controls, then calls
`resolve_estimator_plan` on retained dimensions and compressed semantic/RNG
eligibility, and only then branches to exact, compressed or generic execution.
The generic branch currently consumes the preselected V6 executor. Preserve
this ordering and the original request: exact/compressed outcomes must not be
forced through a generic-work receipt. Automatic solver setup/fallback and
actual workspace admission also have to resolve before estimator RNG. This is
an implementation pointer, not a frozen design for a new ABI or a completed
automatic-routing feature.

Do not start the paper campaign while any applicable path still unintentionally
runs serially. Do not compensate with more repetitions, resource increases,
stayer exclusions or fresh 6.4m cells. No commit/push, fusion promotion, upstream
edit, Windows work, installed-package replacement or private-data transfer.

## Native/public automatic component-batch checkpoint

The next additive slice carries omitted inference-batch intent through V5
component augmentation and V7 execution. V5 augmentation prices one initial
RHS rather than the legacy literal width eight; the V7 executor selects fixed
k=2 component/Gram widths jointly with point widths before its owned pool and
RNG. Literal eight, legacy Gram width, V1--V6 solve meaning and frozen work
receipt layouts remain unchanged. Native readiness bit 13 and independent C
transport API 2 reject stale runtimes before preparation. The 88-byte separate
receipt reports original/selected widths, capacity, threads and command peak;
the public Stata route reconciles it with work, output and memory receipts.

The source-local Mac qualifier passes at
`.local/optimization-parity-20260913/native-receipt-automatic-component-07.txt`.
All 205 inputs rehash at manifest
`a7acf41c55bb501e9b64d7c79bd822424f0de55cd0f1531c8152d74656b51bab`.
Arm64/Rosetta thin and universal candidates, isolated clean installs, C
transport, ABI, full pinned Rust and strict Clippy all pass. Native FFI tests
cover both deletion modes, diagonal/direct CMG and 1/7 threads; public Stata
tests cover the explicit inference tuple. A separate low-point/high-inference
test confirms point width eight versus selected inference width 32/capacity 32
at four threads with unchanged corrected results. Full `fevc/tests/python`
passes 687/687 after updating stale helper-inventory/comment expectations and
sorting the bundle allowlist. Those test/allowlist edits are outside the 205
qualifier inputs. Earlier attempts 01--06 preserve tooling, C-stub and Stata
loader/test failures and focused repairs; no numerical or scientific gate was
weakened. Installed PLUS and private Veneto remain untouched.

The audit after this checkpoint confirms `algorithm(auto)` and `engine(auto)`
do **not** select the new V6/V7 executor: both APIs require explicit JLA and
generic engine. Rewriting request strings in Stata would falsify the V3
capability signature. The native V4 engine resolver already selects exact,
compressed or generic from original retained dimensions before RNG, so an
additive automatic executor must attach only after this selection, preserve
exact/compressed non-applicability, and retain pre-RNG automatic CMG fallback.
The existing SCC Linux deployer is clean-commit-only and cannot represent the
approved dirty candidate; an isolated source-bound bundle is required. No SCC
job or benchmark timing has been launched at this checkpoint.

## Source-bound SCC Linux continuation

The existing clean-commit builder is again byte-identical to its registered
SHA256 `2c1280e12b8ac7974fba35afa050a5ed264d80f00f331302cdb69a6f9bd59935`.
Dirty-source bundling lives in the isolated `build_dirty_linux_bundle.py`;
its tests cover deterministic bundles, private Veneto exclusion and unexpected
untracked-source rejection. The source-bound bundle used for qualification has
SHA256 `ebe248555d411449b4d5ca7783f7f037e51aaa6e5c1626d3fe7b18ec0288e587`.

The first four-slot job `7564881` in SCC run
`20260913T230540Z-optparity-dirtylinux-01` passed C, ABI, Rust, Clippy and build,
but the Stata full suite stopped when `test_rust_execution_paths.do` requested
seven processors on a four-slot licensed job (`r(198)`). Its failed receipts and
sanitized logs are preserved. A narrow test-only guard skips only thread cells
that Stata refuses; local licensed execution still tests seven threads. The
focused Mac Stata retest passes. The frozen clean builder was restored after a
registered source-hash check caught an accidental change; the first diagnostic
bundle is not promoted.

The single repaired SCC retest `7564893` in run
`20260913T231445Z-optparity-dirtylinux-02` is accepted for that exact snapshot:
SGE `failed=0`, `exit_status=0`, four slots on `scc-gd4`, 1,642 seconds wall,
`maxvmem=9.412G` (scheduler virtual memory), and `ru_maxrss=881028` (not a
whole-process-tree physical-RSS measurement). The wrapper and application
receipts pass. The Linux qualifier reports full Stata, isolated clean install,
public/lifecycle/diagnostic/shared-atom checks, C/ABI, pinned Rust, strict
Clippy, formatting and release build as PASS. Its classification is
`DIRTY_SCC_LINUX_X86_64_CANDIDATE_QUALIFICATION`, 2,472 qualifier source
files at manifest `f945eb784736d081ff36abb67577d4d0437a1e508951cfc2a7e486ae6b61ae47`.
The candidate ELF SHA256 is
`e5de46a21321c5d54ec1f91890da0b3b540a086b6a7f5e90170c5ed60e7aea24`;
the collected copy rehashes identically. Sanitized receipts, logs and binary
are in `.local/optimization-parity-20260913/linux-artifacts-dirty-02/`. Private
Veneto is absent from the SCC source snapshot. The initial `rsync -a` collection
hit a local metadata-permission error; a content-only `rsync -r` completed and
the collected candidate hash and PASS markers were verified.

The local integrated `fevc/tools/run_checks.py` finishes with
`FEVC LOCAL QUALIFICATION PASS`: source audits, 807 Python tests, Stata quick/full,
clean installation and small comparison checks. An earlier diagnostic attempt
exposed the frozen-builder hash change (one Python failure) and is not accepted;
the restored-builder rerun passes. The test-only processor guard was added
before the Stata phases of the passing integrated run. The older Mac 205-file
manifest still matches 204/205 paths, with only that test file changed; it is
historical Mac native evidence, not a byte-identical receipt for the Linux
snapshot. No benchmark timing, paper change, installed PLUS replacement,
private-data transfer, commit, push or Telegram occurred. Effective
`algorithm(auto)`/`engine(auto)` execution routing is still pending.

## Native-only resolver-aware V8 slice

The source-only additive V8 solve request retains V7/V6/V4 prefixes and the
original signed V3 algorithm/engine/route fields. After V3 signature
reconciliation and the existing pre-RNG estimator resolver, exact and
compressed calls keep their established executors and have no generic-work
receipt. Generic JLA uses the ordered diagonal queue only when an explicit
diagonal or the registered automatic firm/RHS rule selects diagonal. A
structural automatic CMG selection stays on the legacy routed setup, including
its existing pre-RNG fallback permission; V8 does not optimize CMG or claim a
new fallback test. No old request layout or public selector was changed.

The focused FFI comparison covers automatic exact, automatic compressed,
automatic generic and explicit generic decisions with unchanged signatures
and corrected targets. At seven threads and 200 probes, generic V8 reports
56 leverage probes per batch, 32 target probes per batch and 64 RHS capacity.
A 256-firm automatic-CMG case matches the legacy result and exports no V8
generic-work receipt. The first signature-failure test incorrectly expected a
failed generation to remain prepared; the native lifecycle correctly marks it
failed, and the corrected test verifies failure, release and fresh preparation.
The caller-thread cancellation test then verifies interruption and successful
reuse through the new V8 interrupt entrypoint.

The complete pinned Rust workspace passed with 64 FFI tests before the final
test-only interruption addition; all 65 FFI tests pass at the final source.
Strict workspace Clippy, formatting and a standalone C11 header layout check
also pass. The focused package-layout/Linux-qualifier Python pair passes 38/38;
pytest emitted only cleanup warnings from unrelated pre-existing temporary
symlink fixtures. This is native-only developer evidence, not Mac/Linux source-bound
qualification of V8, Stata transport support or measured performance. The
next slice needs a C transport selector and public reconciliation that fetches
generic work only after native selection, plus a separate treatment of
structurally selected automatic CMG before the 96-call development gate.
Installed PLUS, private Veneto and the paper remain untouched; no further SCC
job, commit, push or Telegram was requested or performed.

## Public resolver and frozen development harness

The additive V8 route now crosses the C/Stata boundary without rewriting the
signed V3 request. `solveexecutionv8` carries resolved-execution mode 3 and the
tolerance-supplied bit; public readiness independently requires native bit 14
and `execution_api=3` before preparation. After native resolution, Ado fetches
a work receipt only for generic JLA. Exact and compressed remain receipt-
inapplicable. Generic automatic CMG now uses the optimized direct solver and
retains the typed pre-RNG setup/resource fallback to the ordered diagonal queue.
Legacy V5 `full_cmg_v2` remains forced and is not re-resolved by V8 thresholds.

The C11 transport test exercises the 36-field V8 selector and unchanged struct
sizes. The focused public test now requires a truthful diagonal-queue receipt;
the routing suite rejects either missing bit 14 or `execution_api<3` before
prepare. A full Rust run initially caught the V5 compatibility error described
above; its corrected serial 65-test engine FFI suite passes. The focused Python
gate passes 51 tests, and the new campaign sources pass 17 generator/launcher
checks. Current-source Mac and Linux qualification still precede timing.

The write-once development harness in this directory generates eight
deterministic mixed-degree public-path inputs (100k point rows and 8k
projection/inference rows), prepares immutable baseline/candidate Linux
packages, requires a real four-slot smoke plus deliberate failure propagation,
then schedules 16 same-host paired tasks. Each task has two separately recorded
warm-ups and six rotated timed calls: exactly 32 warm-ups and 96 timed calls.
The array requests 14 slots x 3 GiB for 45 minutes; applications use only one
or seven threads. Inputs, sources, binaries, routes, physical RSS, residuals
and all result cells are frozen or recorded. The validator enforces numerical
parity, a three-percent point-command geometric-mean gain and the five-percent
per-profile median-regression ceiling. No paper job is released by this harness.
