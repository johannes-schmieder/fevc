# All-probe MCSE optimization — September 30, 2026

The requested optimization is implemented on `main`, preserving the existing
issue #7 implementation and dirty worktree. M0–M4 are complete within the local
development scope. No release, binary adoption, CI policy change or cluster
campaign is included. The [source-bound development receipt](all_probe_optimization_development_20260930.json)
records exact inventories, commands, seeds, tolerances, failures and limits.
Earlier receipts remain immutable.

The [final independent audit](all_probe_optimization_review_20260930.json)
reconciles every timing command, all 576 result comparisons, all 18 median
summaries, exact qualified binaries, both clean installs and runtime restoration.

## Implementation and interface

Native numerical preflight now loads the small `fevc_numerical.mata` module.
It avoids compiling the full Mata solver for a native command. Package and
benchmark manifests include the module; versioned runtime guards reject stale
loaded replay code. The public syntax and numerical V1 native layout remain
unchanged:

```stata
fevc y x1 x2, worker(worker_id) firm(firm_id) algorithm(jla) probes(200) numericalmcse(all)
estat numerical
```

Default `numericalmcse(conditional)` retains the legacy conditional diagnostic.
The normal display and `estat numerical` show approximate all-point-probe
numerical MCSE and its availability status. Four values are posted in
`e(numerical_mcse_all)`; the signed primitive covariance is separately available
as `e(numerical_mccov_all_raw)`. This measures randomized computation error;
it creates no econometric `e(V)`. Help, memory guidance, changelog and the current
checkpoint are updated.

Rust and Mata generic/compressed replay now batch at most four directions and
the admitted leverage width, reusing prepared solvers. Rust observation replay
reuses packed Counter-V1 blocks and combines RHS construction with copy
pullback contractions. Prediction, RHS, atom and fold scratch is reused;
match-only Rust state omits observation derivative arrays. Mata preallocates
R-by-3 replay receipts. Production storage remains bounded by rows/copies,
small batch width and probe metadata; no rows-by-probes cache is introduced.

Point draws, estimates, conditional MCSE, runtime RNG contracts, scientific and
residual gates, caller state and prior point/CMG interface meanings are preserved.
Replay receipts distinguish attempted directions, executor-accounted completed
work and the certified prefix. A failed batch accepts no partial prefix within
that batch and uses no replacement draw. Mata certificates now report each
direction's existing iterations and original-system residual.

## Complete-command performance

The frozen 192-command comparison uses both backends, generic observation,
generic match with combined stayers, and compressed mover-only match. It uses
R=T=200, literal point batch 7, diagonal preconditioning, seed 2026092911,
integer frequencies averaging two physical copies per stored row and nonuniform
target mass. Generic routes include two varying controls. Observation and
compressed match use `probeorder(probe)`; the existing hybrid match capability
omits it. Every command checks the selected backend/engine, counts, full caller
RNG/data/sort state and absence of `e(V)`.

Timing runs are sequential, with no concurrent calibration, compilation or
qualification workload. Previous and optimized versions are interleaved, and
each uses an immutable frontend snapshot and its exact privately qualified
signed arm64 artifact. Source/binary/runner identities, all raw rows, process
exits and Stata PASS markers are retained in
`.local/issue7-opt-20260930/performance-v2/`.

The tables give medians: three replications for the smaller input and two for
the larger input. Cold commands start in fresh Stata processes. Warm commands
follow one full all-probe command in the same process; their times exclude that
warmup. Added cost is optimized all minus optimized conditional. RSS is measured
for the complete process in decimal MB and includes Stata, loaded libraries and
data; it has a different scope from direct heap admission.

### Cold: 1,200 stored rows / 2,400 copies

Compressed match retains 1,140 rows / 2,280 copies.

| Backend / route | Previous all, s | Optimized conditional, s | Optimized all, s | Added MCSE cost, s | All speedup | Enabled RSS old / new, MB |
|---|---:|---:|---:|---:|---:|---:|
| Rust generic observation | 0.655 | 0.330 | 0.356 | 0.026 | 1.84× | 42.25 / 41.88 |
| Rust generic match | 0.655 | 0.335 | 0.362 | 0.027 | 1.81× | 42.60 / 42.35 |
| Rust compressed match | 0.494 | 0.174 | 0.200 | 0.026 | 2.47× | 40.73 / 41.45 |
| Mata generic observation | 1.412 | 0.827 | 1.235 | 0.408 | 1.14× | 42.96 / 43.30 |
| Mata generic match | 1.391 | 0.833 | 1.228 | 0.395 | 1.13× | 43.47 / 43.06 |
| Mata compressed match | 1.287 | 0.961 | 1.168 | 0.207 | 1.10× | 41.16 / 41.11 |

### Cold: 12,000 stored rows / 24,000 copies

Compressed match retains 11,400 rows / 22,800 copies.

| Backend / route | Previous all, s | Optimized conditional, s | Optimized all, s | Added MCSE cost, s | All speedup | Enabled RSS old / new, MB |
|---|---:|---:|---:|---:|---:|---:|
| Rust generic observation | 0.942 | 0.459 | 0.569 | 0.110 | 1.66× | 54.76 / 57.38 |
| Rust generic match | 0.864 | 0.485 | 0.558 | 0.074 | 1.55× | 55.85 / 59.20 |
| Rust compressed match | 0.704 | 0.305 | 0.370 | 0.065 | 1.90× | 50.26 / 51.47 |
| Mata generic observation | 5.668 | 1.863 | 5.212 | 3.349 | 1.09× | 87.32 / 88.24 |
| Mata generic match | 5.779 | 1.905 | 5.399 | 3.493 | 1.07× | 84.39 / 84.64 |
| Mata compressed match | 3.779 | 2.523 | 3.587 | 1.065 | 1.05× | 58.92 / 58.81 |

### Warm: smaller input

| Backend / route | Previous all, s | Optimized conditional, s | Optimized all, s | Added MCSE cost, s | All speedup |
|---|---:|---:|---:|---:|---:|
| Rust generic observation | 0.292 | 0.270 | 0.284 | 0.014 | 1.03× |
| Rust generic match | 0.294 | 0.280 | 0.292 | 0.012 | 1.01× |
| Rust compressed match | 0.135 | 0.117 | 0.129 | 0.012 | 1.05× |
| Mata generic observation | 0.881 | 0.268 | 0.669 | 0.401 | 1.32× |
| Mata generic match | 0.872 | 0.275 | 0.661 | 0.386 | 1.32× |
| Mata compressed match | 0.701 | 0.326 | 0.529 | 0.203 | 1.33× |

Cold Rust all-probe commands are 1.55–2.47× faster (35–60% less time).
Warm Rust gains are small, 1.01–1.05×, and three replications do not establish
a statistically significant gain. Mata all-probe speedups range from 1.05× to
1.33× (5–25% less time). The larger Mata generic routes still add about
3.35–3.49 seconds relative to their conditional commands. Some larger Rust
measurements use a few MB more process RSS; this is a bounded time/memory
tradeoff, not a uniform memory reduction. No measured whole-command median
regresses, but conditional cold Mata compilation is slightly slower.

All 192 commands complete and all cross-version/conditional-enabled checks
pass. Point estimates, conditional MCSE, raw all-probe covariance and usable
all-probe MCSE values compare exactly in these CSV outputs: maximum normalized
error is zero against the unchanged `1e-11` gate. No speed cutoff was used.
These are local fixed synthetic inputs; they do not support representative-scale
or universal overhead claims.

## Prescribed calibration and independent review

Fresh dense-PCG64 and production Counter-V1 complete/nested campaigns execute
239,616 attempts across 72 cells. Each RNG has 27 complete cells at K=4096,
64 blocks of 64, budgets (200,200), (400,200), (200,400), and ten contrasts.
All 540 screens pass the unchanged
`abs(relative discrepancy)+2.576*SE <= .15` gate. Maximum bounds are
0.1058532361 dense and 0.1035656334 native. There are zero observed point
failures and 100% usable/finite diagnostics, against the registered zero-failure
and 99%-usable gates. Every finite raw covariance is included.

Each RNG also has nine K=128, L=8 nested cells. Every attempt is retained;
signed `B-W/L`, `B+(1-1/L)W`, paired differences and delete-one-sketch uncertainty
are reported with no invented nested cutoff. Dense master seeds are
9274401/9284401; native master seeds are 9324401/9334401. Inputs, seeds,
dimensions, budgets and acceptance thresholds match the prior registered
campaigns and were frozen before execution.

An independent reviewer regenerated all 72 audits byte-identically and verified
attempt keys, seeds, inventory, source archives and all failure classifications.
Across 119,808 native attempts, point estimates and conditional matrices equal
the prior implementation; maximum normalized raw covariance difference is
`2.1726675157853779e-16`, with zero diagnostic-status differences. Independent
generic and compressed original-stream checks verify each of 33 per-RHS replay
receipts exactly; 24 rows per fixture distinguish the direction's certificate
from the aggregate batch maximum. Caller full RNG restoration passes.

Calibration binds source-v2's immutable 198-file inventory. Final source-v3
changes only two Mata per-RHS receipt selections and the shared-atom test's
explicit full-solver dependency. The recorded compatibility review preserves
Rust production/build/binary, dense generator/auditor, input and acceptance
identities, and adds final focused/native checks. It carries calibration only
within that unchanged scope. Compressed transport, Mata RNG and caller state
have separate source/native/reference tests; this calibration does not qualify
sampling inference, near-singular designs or every runtime/platform.

## Validation, failures and limits

The full integrated source driver passes 866 Python tests, CMG generation,
Stata quick/full suites and portable/downstream installs. Final affected checks
pass 99 Python tests, independent dense Mata math/generic/compressed replay,
stale-runtime preflight, the explicit cached-response complex-step oracle,
packed-word/batch-boundary regressions and strict Rust Clippy. The Rust workspace
passes 689 tests with two explicitly ignored; the required Python-dependent
cache oracle is executed separately and passes.

The q32 tracking allocator measures the same 785,248-byte peak with and without
the attachment for observation and match. Retained payload changes from 7,008
to 7,824 bytes. The conservative reserved increment is 363,000 bytes; generic
stored-row scratch reserve rises to 640 bytes and compressed reserve to 240
bytes per group to cover the bounded batched buffers. Process RSS is separately
reported above.

Final Mac qualification binds 252 source files and passes C ABI/transport,
backend tests/Clippy, thin and universal arm64 and Rosetta, numerical V1,
shared atoms and both clean installs. Its classification remains
`LOCAL_CHECKPOINT_DIRTY_TREE`. All five original local plugin bytes and modes
are restored afterward; candidates remain private test artifacts.

The failed dense-v2 registration carries a historical complete-validator hash.
It preserves 57,344 raw attempts in 14 cells, 13 execution records and no accepted
audits. Fresh dense-v3 reruns all 27 complete cells with the same scientific
protocol and correct validator identity. Nine nested dense manifests retain an
unused historical complete-validator field; their actual generator and nested
auditor are independently bound and verified. The raw dense environment label
`development_dense_small` is descriptive metadata; scientific profiles come
from each frozen confirmation manifest and were not relabeled.

Native qualification v3 fails because a comparison test relied on preflight
loading the full Mata solver. The test now loads its own solver dependency;
an isolated install and fresh v4 profile pass. Earlier artifact-directory/network
setup failures and unused superseded registrations are preserved separately.
No estimator, DGP, seed, exclusion or scientific cutoff was tuned after outcomes.

Remaining qualification includes clean exact-commit promotion, owner binary
adoption, native Linux/Windows and Intel hardware, representative scale,
broader runtime-specific/boundary calibration, and owner release/provenance
decisions. Projection and component/sampling-inference intersections remain
explicitly unsupported. No public release, distributed-binary change, CI policy
change, external issue mutation or cluster campaign is made.
