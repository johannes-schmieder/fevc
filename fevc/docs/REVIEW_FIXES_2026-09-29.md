# Review fixes — 2026-09-29

This repair starts from `08c51adc`. It addresses the confirmed defects in the
owner-supplied review. The owner selected fixes before performance/redesign
work and explicitly selected continued estimation with missing diagnostics
when caller timers leave no profiling capacity.

## Changes

- The root native install catalog is rendered from the portable inventory and
  checked against it, including mandatory notices and native file entries.
- Rust matrix jobs explicitly select and report their toolchain. Local source
  instructions select 1.85.1. Stable Clippy fixes use equivalent saturating
  arithmetic and test-only iterator expressions supported by the pinned MSRV.
- Command, lifecycle, graph, solver and estimator profiling share private
  logical slots backed only by unused caller timers. Owned timers are released
  on success, error and UserBreak. Missing timing capacity propagates as missing
  diagnostics, including totals; phases not executed retain zero. Timings do
  not affect routing or scientific acceptance. Two setup-time presence checks
  were removed; all numerical backend checks remain. Runtime build identifiers
  reject cached pre-repair Mata modules. The standalone RNG comparison benchmark
  retains its independent loading contract and is outside public estimation.
- The C marked-column transport rejects an excess marked row before accessing
  its numeric columns. The final count check still rejects too few rows.
- The deployer test owns a disposable main-branch fixture. Documentation
  inventory assertions use minimal recorded expectations. Frozen-evidence
  tests explicitly distinguish unavailable shallow history from bad evidence;
  full-history CI requires their execution.
- The diagonal queue module comment reflects its production routing.
- Standalone Mata tests and benchmarks register their package directory before
  loading runtime files, so the private timing dependency resolves in a fresh
  Stata process as well as inside the integrated suite.

The timing profile assembly moved into the private timing helper because the
additional ownership calls exceeded Stata's compiled-program size limit.
The profile columns and accounting meanings are preserved.

## Validation

Development logs are retained under ignored `.local/review-fixes/`. They are
diagnostic worktree runs, not qualification of distributed binary bytes.

The root-manifest regression failed before repair. The C regression produced
an AddressSanitizer heap-buffer-overflow before the bounds guard and passes
with it. The timing regression exercises Mata exact, generic and compressed
JLA plus native exact/generic routes, mixed running/accumulated caller timers,
partial and complete exhaustion, nested scopes, failure, interruption and reuse.
It compares the result matrices at `1e-8`, uses seed 1731 and 24 probes, and
checks caller data, sort, RNG and timer preservation.

The production repair is `70516881340adf12959870e0afb833514031ec69`.
Follow-up `5387db8d` changes only the search-path setup in thirteen standalone
test/benchmark entrypoints. No estimator, binary, fixture, seed or acceptance
threshold changes in that follow-up.

| Gate | Result |
| --- | --- |
| `./.venv/bin/python -m pytest -q` | 835 passed, Python 3.13.0. The final run emitted only temporary-directory cleanup warnings after its successful test result. |
| `./.venv/bin/python fevc/cmg/tools/assemble.py --all --check` | Pass; generated CMG sources unchanged. |
| Rust workspace and standalone backend | Formatting, strict Clippy, and locked all-target tests pass with Rust 1.85.1 and installed stable 1.97.1; the existing ignored test remains ignored. |
| C transport | AddressSanitizer/UndefinedBehaviorSanitizer regression, interrupt harness and ABI-header compilation pass. |
| `./.venv/bin/python fevc/tools/run_checks.py` | Stata/MP 19 quick/full suites, CMG core/scale gates, portable install, both helper migrations, and synthetic/paired/Separations harness gates pass. The final standalone bridge-audit smoke initially failed to find the timing helper; its repaired entrypoint and all other affected standalone entrypoints pass separately. |
| `DEVELOPER_DIR=/Library/Developer/CommandLineTools ./ci/run_ci_profile.sh plugin-build` | Exact-source success on `70516881`, clean at both boundaries, 1,563 seconds. Thin/universal arm64 and Rosetta x86-64 routes, timer regressions and isolated installs pass. |
| Root-catalog `verify_native_installers.py` | Fresh and replacement `net install` pass through loopback HTTP; all 57 installed-file hashes and installed regressions pass. |
| Standalone follow-up | Ten tests pass in separate fresh Stata processes. Three small benchmark entrypoints also pass, including a converged forced-CMG solve and the eight-row retained-sample smoke. |

The native receipt is retained at
`.ci/stata/results/70516881340adf12959870e0afb833514031ec69.json`, with source
manifest, candidate binaries and sanitized qualification transcripts under
`.ci/stata/run/`. The root install record is
`.local/review-fixes/root-install-final/verification.json`. Source, Rust,
sanitizer, timing and standalone logs remain under `.local/review-fixes/`.
The exact native commands, toolchain identity and PASS inventory are in
`.ci/stata/run/plugin-qualification.txt`.

The integrated command was not repeated after the final harness-only fix:
every preceding gate had passed, and the failed final gate plus all affected
standalone entrypoints were retested directly. The later source compatibility
review records the two commits and changed paths without relabeling the
native receipt. Earlier failed diagnostic attempts remain separate, including
the lifecycle return-value failure that was fixed before exact-source
qualification.

All five distributed plugin files retain their original bytes and modes.
The qualifier's staged Mac candidates were preserved as ignored evidence, and
its staging copies were restored to the original distribution files. This is
source/candidate qualification, not binary adoption. Linux, Windows, native
Intel hardware, production scale and public release were not requalified.
No old receipt is rewritten or treated as qualification of this repair.

The complete-command timing screen compares baseline `08c51adc` with the
repaired Mata package, using 24 probes, seed 1731, four firms and eight rows per
worker. Two processes per version run in before/after/after/before order, with
one warm-up and eight measured calls per route in each process. Other task
validation processes had finished. Median seconds across sixteen warm calls:

| Stored rows | Route | Before | After |
| --- | --- | ---: | ---: |
| 960 | Exact, one control | 0.0360 | 0.0360 |
| 960 | Generic JLA, one control | 0.0410 | 0.0440 |
| 960 | Compressed JLA, no controls | 0.0445 | 0.0470 |
| 9,600 | Generic JLA, one control | 0.3180 | 0.3220 |
| 9,600 | Compressed JLA, no controls | 0.3505 | 0.3530 |

Timer ownership adds a few milliseconds in this screen. This is a bounded
overhead check, not a speedup or production-scale performance claim; the
review's proposed kernel optimizations remain deferred.

## Remaining review proposals

| Proposal | Disposition |
| --- | --- |
| Retire ABI generations; generate the C header | Deferred compatibility project; inventory actual consumers and stale-runtime behavior first. |
| Broad ado split, routing decision table, narrower receipts | Deferred design work; retain scientific and memory reconciliation checks. |
| Merge JLA engines or CMG implementations; reduce options | Deferred owner decisions about compatibility, RNG paths and supported requests. |
| Interleaved Schur batches, chunk-loop checks, control layout | Deferred performance experiments requiring independent oracles, cancellation/memory checks and complete-command timings. `checkpoint_chunk` already polls callbacks only every 4096 iterations; the proposed optimization removes per-element wrapper/branch overhead. |
| Remove input double buffering | Deferred ownership and memory-accounting change at the native boundary. |
| Vectorize Mata loops; repair the degree-four setup cliff | Deferred algorithm/performance work with the registered numerical gates. |
| Remove the universal Mac payload | Deferred distribution decision. The default loader selects thin binaries; native qualification explicitly exercises universal payloads. |
| Remove the diagnostic executor | Deferred cleanup; self-test use is still a consumer. |
| Separate research, reduce prose assertions, enforce all Ruff rules, consolidate versions | Deferred maintenance work. No blanket rewrites or version/ABI identity changes are part of this repair. |

No pushes, tags, releases, remote campaigns or binary adoption are included.
