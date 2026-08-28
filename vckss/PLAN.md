# Active package plan

## Objective

Produce a private `vckss` `0.4.0-alpha.1` release candidate that is a fast,
statistically equivalent Stata alternative to maintained MATLAB KSS on
compatible hard problems. Corrected-result equivalence and end-to-end speed
are the primary development criteria. Rust/Mata feature parity, pathwise
floating-point identity, and additional engineering evidence are secondary.
Windows and public distribution are deferred.

The active comparison rule is
[`docs/development_acceptance_v1.json`](docs/development_acceptance_v1.json).
It replaces bitwise identity and legacy fixed roundoff thresholds as promotion
gates while retaining estimator/sample/target meaning, finite outputs,
identification, accounting, complete residuals, direct memory, typed failure,
no hidden estimator fallback, and caller-state/lifecycle safety as hard gates.

## Baseline — 24 August 2026

Source `165a25cb3220a94c34656c2c22047ae0d60f26c8` was clean and matched
`origin/main`. The following baseline facts were established before alpha work:

- Python had one infrastructure failure because the mutable
  `.ci/stata/latest.json` pointer was incorrectly byte-locked by the rename
  inventory; archived per-SHA receipts were unaffected. Commit `0d63c29`
  repairs and tests that policy, after which all 375 tests pass.
- Generated CMG source is drift-free.
- Rust 1.81 formatting, Clippy, workspace tests, and Stata-backend tests pass.
- Public auto-exact is already implemented and qualified on macOS arm64 and
  Rosetta at its source-bound milestone.
- A protected cleanup removed 42,120 ignored disposable files and 3.98 GB of
  stale targets, logs, caches, traces, and obsolete local plugin artifacts;
  tracked evidence and development dependencies were retained.

The generated current feature ledger is
[`docs/RUST_MATA_PARITY.md`](docs/RUST_MATA_PARITY.md).

## Production full-CMG rollout — 26 August 2026

The owner selected the scalar private winner for productionization with a
faster-than-matched-MATLAB gate on both registered hard cases and at most 5%
regression from the source-bound private winner. Commit `ba83e53` vendors and
pins standalone CMG `dbefbc5` and Rust 1.85.1. Commit `93cd9e2` adds the
normal-build `CMG_FULL_V2` receipt and restricts it to the registered explicit
Rust no-control match-JLA cell. Commit `6ab23cb` adds checked whole-command
pre-RNG memory admission, retained-memory reconciliation, and deterministic
same-route residual refinement. Commit `6022b7b` adds cooperative caller-thread
UserBreak polling, worker-only atomic cancellation, and terminal-generation
lifecycle coverage. The current source removes the last private preparation
switch from the winning route: an additive V4 preparation ABI selects the
certified implicit worker-firm match key explicitly, charges its key and sort
workspace before RNG, and keeps older preparation callers unchanged. Disabled
fused, mixed-precision, and pass-fused sources live only under the historical
`rust/experiments/full_cmg_spike/` tree; normal workspaces and runtime sources
contain no private activation or logging hook. Runtime source `4b6874e` passes
the alpha production gate: the macOS headline median is 74.774 seconds versus
MATLAB R2024b Update 5 at 104.489 seconds and the 81.145-second private winner;
SCC's fixed CZ18 median is 19.097 seconds versus MATLAB R2024b Update 3 at
33.058 seconds and the 33.942-second private winner. Both pass statistical,
complete-residual, memory, state, wrapper, and process checks. Commit `61dba32`
admits the same effective cell through `backend(auto) rng(auto)` on qualified
macOS and Linux builds, preserving fail-closed behavior after selection and
all prior routes for unsupported cells. The private package identity is now
`0.4.0-alpha.1`. Windows, a public tag, and a public release remain deferred.
A final protected inventory/apply cleanup removed 26,310 regenerable target,
cache, CI-scratch, and local-plugin files totaling 4.187 GB without touching
tracked source, benchmark receipts, qualification evidence, or `.venv`.

## Comparative scaling study — 27 August 2026

The active evidence milestone is a new immutable three-way SCC benchmark of
VCkss--Mata, the qualified public `CMG_FULL_V2` Rust cell, and maintained
MATLAB `LeaveOutTwoWay`. It changes no estimator or public option behavior.
The registered grid contains four deterministic graph families, five stored-row
counts, five active-core counts, and three position-balanced seed repetitions:
300 same-host SCC tasks and 900 fresh-process estimator calls. Primary outcomes
are complete command time and estimator-phase summed process-tree RSS, with
process wall/RSS, setup/import, scheduler, and native memory receipts kept
separately. The compact ledger preserves VCkss/full-CMG phase timings and
maintained MATLAB's logged PCG convergence status; a nonconverged MATLAB call
retains timing evidence but makes the cell unrankable. The standalone report
pipeline has passed a warning-free, page-rendered 900-call synthetic schema
exercise. Those synthetic values authorize no performance claim.

The revised prototype protocol and harness live under
[`benchmarks/comparative_scaling/`](benchmarks/comparative_scaling/). Every task
uses one literal input, strict backend identities, 1/2/4/8/16 CPU affinity,
and complete source, binary, wrapper, qacct, node, state, residual, and MCSE
reconciliation. The SCC scheduler may choose any eligible host; the three
implementations run sequentially within one task on that host, order rotates,
and paired within-task ratios are primary. Exact CPU and affinity are retained,
while absolute timing and cross-core scaling across heterogeneous hosts remain
descriptive until confirmed on a homogeneous subset. There is no fixed
development RAM ceiling: the manifest records a scheduler-backed per-command
safety envelope and all forecasts and measured RSS. Only cells with three
scientifically accepted repetitions for all routes may be ranked. MATLAB
comparisons remain descriptive because its RNG and numerical policy differ. No
scaling result or applied recommendation is claimed until the 300-task
collection and standalone report are complete.

The owner registered the SCC license-aware design after jobs `7340247.7` and
`7340426` established that unrestricted 16-slot scheduling is available but
installed Stata/MP versions 15--19 expose only four licensed processors. Every
task still requests and binds 16 slots. Rust and MATLAB retain the registered
1/2/4/8/16 target-core grid, while Stata and Mata use
`min(target_cores,4)`. A source-hashed benchmark-only Ado adapter passes the
full-CMG native thread count through the existing internal boundary without
changing the installed public command: its fail-closed environment contract
requires the 1/2/4/8/16 target, the 16-slot allocation, the capped Stata
processor count, and matching requested/used native receipt fields. High-core
Rust/Mata results are therefore labeled capped-Mata comparisons, not equal-core
Mata scaling. The immutable manifest records target and role-effective counts;
reports limit Mata scaling to 1/2/4 but preserve Rust and MATLAB through 16.
The redesigned local gates, including a real four-processor Stata/eight-thread
native-CMG adapter run and macOS arm64/Rosetta/universal plugin qualification,
pass. No performance result is accepted until the SCC candidate qualification
and registered production collection complete.

Qualification generation `20260828T0434Z-cmgq-b168c98` (array `7342716`)
was rejected and cancelled when the largest pure-ring `weak_d3` tasks failed
the unchanged complete original-system residual gate in both frozen VCkss
checkpoints. The input, rather than the d9 integration, was numerically
pathological: its algebraic connectivity collapses quadratically as the ring
grows, and no valid paired result existed. The V3 input contract retains the
same four graph labels, row/core grids, degree-three weak family, default
tolerances, and target semantics. A first repair added exactly 32 deterministic
diameter chords, but immutable pilot `20260828T0601Z-cmgqpilot-87866ef`
(array `7342945`) was also rejected after task 55 returned complete residual
`2.0769920175e-5`. A second repair used two internally well-connected halves
joined by 32 cross-half bridges. Pilot
`20260828T0638Z-cmgqpilot-0e3dd26` (array `7343616`) passed its completed
eight-core residual, application, and wrapper gates, but both sources built a
155,395,972-byte full CMG plan with three planned batches. The cell was not the
required candidate-only connected-vector route, so the generation was rejected
and cancelled. Later ring, offset, hub-ring, and concentrated-hub repairs either
failed the unchanged residual gate or did not identify the vector route. The V6
shallow-hub-tree leaf-panel topology finally passed the largest 1/8/16-core
pilot (`7343969`), including candidate-only connected-vector routing at 8/16.

Exact source `0df7a481166735ffea3bc3f9cb4d194684910702` then passed complete
local qualification, licensed CI run `33156008043`, preparation job `7344203`,
and all 72 scientific, application, wrapper, memory, state, and numerical task
gates in array `7344263`. SCC never produced original `qacct` records for tasks
13 and 14. Source-identical retry array `7344638` rebuilt nothing and accepted
only those two task IDs after proving exact source, bundle, task row, binary,
and literal-input hashes; both retry records have `failed=0` and
`exit_status=0`. The completed generation nevertheless failed the frozen
performance gate: its 8/16-core paired geometric-mean command-time ratio was
`1.003057`, and the connected-vector eight-core median ratio was `1.000382`.
All cell medians, the connected-vector 16-core result, one-core time, and
one-core RSS gates passed. The generation is rejected, with no result promoted.
The diagnosed repair parallelizes independent Schur-RHS columns on the
solver-owned pool, preserves per-column arithmetic order and `CMG_FULL_V2`, and
extends checked pre-RNG admission for the concurrent worker-scaled temporaries.
Exact source `7e695bb419d6bc0b3141abcaee6ae2c463692d79` passed all local,
licensed CI, and complete 72-task SCC qualification gates. Its parallel-cell
paired geometric-mean command-time ratio is `0.934145`, its worst graph/core
median is `1.028582`, its one-core time and RSS ratios are `1.004790` and
`1.006678`, and its connected-vector medians are `0.888584` at eight cores and
`0.885402` at sixteen. It is promoted without changing any scientific gate or
falling back to the comparison checkpoint.

The first production preparation sequence then exposed a separate evidence-
workflow defect before any pilot measurement: independent Rust and MATLAB
builds differed bytewise because their binaries embed build paths, job IDs,
PIDs, and generated MEX bundle metadata. The unchanged production gate
correctly refused to treat those binaries as identical. The repaired workflow
builds one canonical exact-source artifact set in the preparation-only run and
requires every pilot and production preparation to verify its source,
accounting, manifests, and every artifact byte before importing it. This
workflow-only repair changes no estimator, native boundary, candidate-
qualification input, timing path, or promotion threshold; the exact binary-
identity gate is retained. Under the 28 August risk-based qualification policy,
that kind of source change carries forward unaffected scientific and
performance qualification through a recorded compatibility review rather than
triggering another large array merely because the commit changed.

Before that policy correction, exact workflow source
`35db5825c7ee3c20418d58cb005170e6f2d7e459` had already completed preparation
job `7349610` and all 72 tasks in array `7349704`. All wrapper, scientific,
application, memory, state, and accounting gates pass. The parallel paired
geometric-mean command-time ratio is `0.927447`, the maximum graph/core median
is `1.004666`, one-core time and phase-RSS ratios are `1.007074` and
`0.999905`, and connected vector-only medians are `0.925574` at eight cores and
`0.839869` at sixteen. The candidate remains promoted. The accepted packet is
under `benchmarks/cmg_candidate_qualification/evidence/scc/35db5825c7ee3c20418d58cb005170e6f2d7e459/`.

Pilot `20260828T0619Z-cmgqpilot-0deed11` (array `7343069`) was cancelled and
rejected before completion when a source audit found that its active benchmark
README still described the superseded diameter-chord topology. Its generator,
receipt, validators, protocol, tests, and qualification documentation already
used the two-block topology, but an internally inconsistent immutable source
cannot support accepted evidence. A documentation-corrected source and new run
identity are required; no timing from `7343069` is accepted.

## Public alpha contract

- Omitted `backend()` and `backend(auto)` prefer Rust when the complete
  effective request is supported and the native runtime passes transport and
  compositional capability checks.
- A missing plugin or structurally unsupported effective request may fall back
  to Mata before native preparation and estimator RNG. Stale ABI/build ID,
  corrupt receipts, preparation, memory, rank, numerical, convergence,
  resource, and UserBreak failures fail closed without fallback.
- `backend(rust)` is strict; `backend(mata)` always selects Mata.
- Omitted `rng()` and `rng(auto)` select Counter-V1 on Rust and Stata RNG on
  Mata. Explicit `rng(counter_v1)` is strict Rust consent. Explicit
  `rng(stata)` selects Mata and conflicts with `backend(rust)`.
- The omitted algorithm is MATLAB-like `algorithm(jla)` with 200 probes.
  Explicit `algorithm(auto)` retains frozen exact-small/JLA-large selection.
- Other defaults remain match deletion, mover headline, joint nuisance, and
  automatic engine, preconditioner, and batch selection.
- Record requested/selected backend, fallback occurrence/reason, RNG,
  algorithm, engine, route, preconditioner, batch, memory, and selection
  reasons in `e()`. Alpha results post `e(status) == "ALPHA"` and never `e(V)`.

## Milestones

1. **M0 — baseline, parity ledger, and hygiene.** Keep source gates green,
   install protected dry-run/apply cleanup, and synchronize active docs.
   **Complete** at `967db1b`.
2. **M1 — default routing.** Route from effective options rather than supplied
   flags; add `rng(auto)`, preflight-only fallback, strict explicit routes, and
   clean-install routing/state tests. **Complete** at `daca3e2`; exact-SHA
   Python, Stata quick/full, and macOS arm64/universal/Rosetta plugin-build
   gates pass.
3. **M2 — broad public parity.** Admit automatic compressed/generic JLA,
   controls, deletion modes, weights, targets, deletion IDs, route/batch/wall
   options, and `probeorder()` wherever the native capability accepts them.
   **Complete** at `3bc6a89`; exact-SHA macOS arm64/universal/Rosetta
   `plugin-build` qualification covers automatic JLA, the ordinary
   effective-option surface, and semantic `probeorder()` tie breaking with
   direct memory and receipt reconciliation.
4. **M3 — exact stayer hybrid.** Implement the frozen mixed-deletion design
   with a versioned stayer-augmentation V1 lifecycle, mover headline, separate
   stayer correction, differential oracles, and lifecycle/resource tests.
   **Complete** at `c199bf0`; the exact-SHA macOS `plugin-build` receipt covers
   thin and universal arm64 and x86-64/Rosetta execution, including public
   `stayers(both)`, zero-RNG reconciliation, differential oracles, and clean
   installation.
5. **M4 — platforms and safety.** Requalify macOS arm64/Rosetta; build and
   qualify Linux x86-64 with Stata MP 19 on SCC; add Miri, fuzz, sanitizer,
   dependency, license, and SBOM evidence. Windows stays nonblocking.
   **Complete** at `6e1a7c3`: macOS source `c199bf0`, the
   [SCC source-bound receipt](../rust/qualification/evidence/M4-LINUX-SCC/),
   [bounded safety receipts](../rust/qualification/evidence/M4-SAFETY/), and
   [RustSec/license/SBOM evidence](../rust/qualification/evidence/M4-SUPPLY-CHAIN/)
   pass. Final human license/provenance approval remains a public-release gate.
6. **M5 — performance.** Stop optimizing the simplified embedded hierarchy.
   Evaluate standalone full CMG first as a private direct prepared solve of
   the existing hybrid Laplacian, sharing its graph, hierarchy, plan, thread
   pool, and admitted workspace pool across all estimator RHSs. Retain only
   statistically equivalent, independently reversible wins. **Complete:** the
   scalar direct hybrid route is the normal-build `CMG_FULL_V2` implementation
   at runtime source `4b6874e`; it passes the selected macOS and SCC alpha
   gates above. Fused, mixed-precision, and pass-fused experiments remain
   disabled historical evidence. The following decision log is retained to
   explain the route's earlier rejection and later promotion. The
   `CMG_FULL_SPIKE_V1` route is source-bound to standalone CMG `dbefbc5` and
   isolated behind private environment consent. On the registered local cold
   8,192-firm/200-probe/four-thread case, direct full CMG takes 201.592 seconds
   versus 230.000 seconds for source `4124b34`, a 12.35% end-to-end gain, while
   lowering peak RSS from 7.30 to 5.03 GB. On accepted SCC job `7306628`, it
   takes 223.232 seconds versus 329.260 seconds for A, but maintained MATLAB
   R2025b takes 171.733 seconds on the same node: C is 29.99% slower than
   MATLAB. The 153.432-second SCC direct solve is the dominant candidate
   phase. Its A/C covariance and corrected covariance differ by about
   `2.22e-12`; that failed the legacy `2e-12` pathwise gate but easily passes
   the current development-equivalence rule and is not a scientific blocker.
   The original route was rejected because it was 29.99% slower than MATLAB,
   not because of the numerical difference. The owner subsequently authorized
   a bounded renewed architecture wave on the direct full-CMG route while
   keeping it private. Source `598a08d` adds packed and ordered-parallel
   Counter-V1 generation, direct RHS construction, independent target
   preparation, parallel moment accumulation, and parallel recovery and
   complete-residual certification. Its first clean source-bound macOS
   headline completes in 81.145 seconds versus the registered 171.733-second
   MATLAB comparator: 0.473 times MATLAB, or 2.12 times as fast. All corrected
   targets are bit-identical to the preceding source-bound run and the maximum
   complete residual is `6.97e-6` under the `1e-5` probe gate. This is a
   single-run checkpoint, not promotion evidence. SCC synthetic job `7311964`
   takes 156.455 seconds against MATLAB's 260.928 seconds and misses the 2x
   gate by 25.991 seconds. Accepted same-host job `7312041` keeps the direct
   route at 151.351 seconds and disables the fused-f64 route after its 239.610-
   second regression; the official full-CMG repeated solve is the dominant
   cost. Commit `83d2284` repairs the private fit receipt reconciliation
   without weakening reduced or complete residual gates. SCC P20 job `7314745`
   then passes the native result boundary, all 61 solves, all corrected-target
   gates, caller-state restoration, wrapper, qacct, and the final validator at
   maximum complete residual `7.993e-6`. It is implementation smoke only:
   candidate command time is 91.596 seconds versus MATLAB's 47.154 seconds.
   The scalar pass-fusion experiment at `08565be` preserves the official CMG
   hierarchy, recurrence, and certification but improves the best adjacent
   local command/solve observations by only about 2% while raising private
   admitted memory 2.6%; it is preserved and disabled without an SCC matrix.
   The first checksum-bound P200 attempt, SCC job `7314843` at source
   `c1ae402`, correctly fails the unchanged `1e-5` complete residual gate on
   target probe 56 (`1.563e-5`) before MATLAB. The next source-bound attempt
   keeps effective probe tolerance `1e-6` but pre-registers private inner
   tolerance `1e-9` for P200 (`1e-8` remains the P20 setting). Fixed-CZ18 job
   `7317771` at source `3daa465` now passes one cold and five position-balanced
   warm A/C/MATLAB rounds. Warm medians are 253.771 seconds for A, 33.942
   seconds for C, and 70.147118 seconds for MATLAB: C is 2.0667x MATLAB and
   uses 58.7% of MATLAB's median peak RSS. All complete residual, common-probe
   corrected-target, repeatability, application/state, process-tree, wrapper,
   and qacct gates pass. The pinned validator's first post-job invocation
   stopped on blank optional legacy phase diagnostics; commit `5a6daaf`
   repairs only that evidence parser, self-hashes it, and validates the
   unchanged run. This remains a fixed-CZ18 checkpoint, not alpha promotion.
   Alternating synthetic medians remain mandatory before vendoring or
   hardening. Do not return to isolated optimization or qualification of the
   simplified hierarchy.
   The first post-reboot macOS synthetic-matrix attempt at source `787327f`
   stopped before comparator identity because MATLAB R2024b was signed out.
   Its two completed cold Stata cells are preserved but cannot enter any
   timing claim; rerun the full matrix in a new directory after authentication.
   The registered SCC synthetic matrix is complete at job `7318114`, source
   `787327f`. Five warm medians are 490.209 seconds for A, 123.633 seconds for
   C, and 183.017703 seconds for maintained MATLAB R2025b. C is 3.9650x A and
   1.4803x MATLAB, while all corrected-target, complete-residual, caller-state,
   process-tree, wrapper, and qacct gates pass. It nevertheless misses the
   registered 2x gate by 32.124 seconds and exceeds MATLAB's maximum process
   RSS by 287,088 KiB. The official full-CMG repeated solve is the dominant
   phase at 98.664913 seconds for 601 RHSs. At that source this result directed
   the team to keep the route private and defer hardening; the later scalar
   production wave and the exact alpha matrices above supersede that decision
   without rewriting its preserved evidence.
7. **M6 — benchmark report.** After a winning route clears the synthetic
   decision gate, run the registered A/B/C/maintained-MATLAB
   synthetic and checksum-bound CZ18 matrix on macOS and SCC; publish compact
   data, phase/RHS/thread/memory/residual tables, figures, a validation receipt,
   and a visually verified benchmark PDF in the style of the standalone CMG
   benchmark document, without placeholders. **Complete** for runtime source
   `4b6874e`; exact compact evidence and the rendered report are maintained
   under `benchmarks/full_cmg_production/`.
8. **M7 — alpha packet.** Freeze one runtime source commit, collect exact-SHA
   macOS, SCC Linux, supply-chain, and benchmark receipts, and maintain the
   private `0.4.0-alpha.1` candidate. **Complete without a tag or prerelease:**
   Windows, human public-release review, tagging, and public distribution are
   intentionally deferred.

Use focused local red/green commits. Push after each completed milestone, run
the exact-SHA quick lane for every push, and require `plugin-build` after any
native boundary change. Receipt-only CI commits do not change the tested source.

## Performance acceptance

The primary performance comparison is maintained MATLAB, not Mata. A candidate
is competitive when its registered median complete-command time is no slower
than MATLAB on compatible hard problems; the development target remains about
twice as fast. Rust/Mata timing is retained as a secondary regression and may
not justify promotion by itself.

The four corrected targets must pass the registered statistical-equivalence
rule. Bitwise/ULP identity, equal iterations, internal reduction order, and a
legacy fixed `2e-12` cross-backend threshold are diagnostic only. Finite
outputs, estimator/sample/target semantics, identification, accounting,
complete residuals, direct memory, typed failure/UserBreak, no hidden
estimator change or post-RNG fallback, and state/lifecycle restoration remain
hard correctness gates independent of speed.

The benchmark report compares public Rust and Mata with identical `vckss`
requests and registers a same-host maintained-MATLAB lane for compatible
no-control match-JLA cells. Because MATLAB uses a different probe stream and
legacy finite-projection expression, cross-language equivalence uses registered
repeated-seed distributions rather than one pathwise numerical comparison.
Performance claims require identical samples, graph structure, probe count,
compatible tolerances, hardware, and equivalent worker counts.

## Completion gates

The alpha candidate requires:

1. every alpha-required public feature works on its claimed platform and its
   corrected targets pass the registered statistical-equivalence policy;
2. Python, generated CMG, Rust fmt/Clippy/workspace/backend, Stata quick/full,
   integrated, clean-install, and source-local plugin gates passing;
3. macOS arm64/Rosetta and SCC Linux x86-64 exact-source receipts;
4. lifecycle, malformed-receipt, interruption, memory, safety, and benchmark
   gates passing without hidden estimator or target changes;
5. a rendered and visually inspected benchmark PDF plus reproducible compact
   inputs and summaries;
6. clean local/remote `main`, no `.ci/codex/` transport, no disposable logs,
   and current documentation/Vault status; and
7. completed human mathematical and license/provenance review before any later
   public release. The private alpha does not make that public-release claim.
