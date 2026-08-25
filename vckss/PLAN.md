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
   statistically equivalent, independently reversible wins. **Decision spike
   complete; route rejected.** The
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
   single-run checkpoint, not promotion evidence. The route remains private
   and must clear alternating warm medians and the checksum-bound fixed CZ18
   case before vendoring or hardening. SCC synthetic job `7311964` and fixed-
   CZ18 P20 route smoke `7311979` are the current submitted Linux gates. Do not
   return to isolated optimization or qualification of the simplified
   hierarchy.
7. **M6 — benchmark report.** After a new architecture clears the synthetic
   decision gate, run the registered A/B/C/maintained-MATLAB
   synthetic and checksum-bound CZ18 matrix on macOS and SCC; publish compact
   data, phase/RHS/thread/memory/residual tables, figures, a validation receipt,
   and a visually verified benchmark PDF in the style of the standalone CMG
   benchmark document, without placeholders.
8. **M7 — alpha packet.** Freeze one source commit, build all artifacts from
   it, collect exact-SHA receipts, tag `v0.4.0-alpha.1`, and create a private
   prerelease with no public or Windows claim.

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
