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

Final gate results and exact-source native evidence are recorded after the
source and native qualification runs finish. No old receipt is rewritten or
treated as qualification of this repair.

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
