# Mata all-probe MCSE optimization — September 30, 2026

The requested Mata optimization is implemented in the existing `main` worktree.
On the measured 12,000-row generic inputs, complete all-probe commands improve
2.11–2.12× cold and 2.46× warm. Compressed Mata's cold percentage overhead is
comparable to Rust. Generic Mata still has higher percentage overhead, especially
for small warm commands. These are local synthetic measurements, not a scale
qualification or a universal speed guarantee.

The [source-bound receipt](mata_all_mcse_optimization_development_20260930.json)
binds source, packages, inputs, runners, logs and comparisons. The earlier
[optimization report](ALL_PROBE_OPTIMIZATION_2026-09-30.md) and receipts remain
immutable. No commit, push, release, native binary adoption, CI policy change or
cluster campaign is included.

## Implementation and preserved behavior

The original bottleneck was scalar replay scoring and repeated group gathers.
An instrumented 12,000-row development profile measured observation scoring at
2.593 seconds, generic match target folding at 1.576 seconds and match replay
scoring at 1.415 seconds. Intermediate vector implementations measured these at
.097, .129 and .111 seconds respectively. These profiles identified the
bottleneck; the complete-command tables below are the final performance evidence.

- Apply analytic derivatives to bounded physical-copy tiles. Each copy's
  nonlinear inverse remains separate; no moments are averaged before inversion.
  Any diagnostic boundary refuses the fast preparation path and uses the
  original scalar status/margin ordering.
- Prepare weights, group frequencies and short-panel gather indices once.
  Short-panel plans contain at most twice the source-row count and width 16.
  Skewed panels use a compensated tiled reduction or stable quad group sums.
- Use quad reductions and crossproducts for fixed target folds and whole-probe
  scores. Compact observation arrays to the observation/stayer portion.
  Six fixed fold weights per physical observation copy replace repeated gathers;
  there is no copies-by-probes cache.
- Replay at most eight directions and the admitted point leverage width. Batch
  compressed cursor state capture/restoration while retaining each original
  logical probe's RNG call shape and order. Rust's four-direction cap is unchanged.
- Explicitly reject overflowing observation influences before `quadcross`, which
  otherwise omits missing rows. Nonfinite folds and scores withhold the diagnostic.

Point estimates, conditional MCSE, scientific/residual gates, production RNG
contracts, caller state, literal-copy semantics and mover/stayer interactions
are preserved. Copy/group contributions still combine before covariance products.
The private numerical Mata module advances from API 2 to API 4, with build ID
`vckss-numerical-api4-vector-replay8`; all consumers reject stale loaded code.
The mathematical/numerical native schema remains V1. Rust production code,
build inputs, reference binary and plugin artifacts are unchanged by this work.

Public syntax, display and returns are unchanged:

```stata
fevc y x1 x2, worker(worker_id) firm(firm_id) algorithm(jla) probes(200) numericalmcse(all)
estat numerical
```

Default `numericalmcse(conditional)` remains available. All-probe MCSE is still a
local approximation for randomized computation error, with availability status
and separate signed raw covariance; it does not create econometric `e(V)`.
Existing help covers this interface. Memory guidance, changelog, implementation
status and the current checkpoint document the new internals and evidence.

## Final complete-command measurements

The frozen 180-command manifest interleaves the task-start Mata implementation,
final Mata implementation and unchanged private Rust artifact. It uses R=T=200,
point batch 7, diagonal preconditioning, seed 2026092911, integer frequencies
averaging two copies per stored row, and nonuniform target mass. Generic routes
have two varying controls; generic match includes combined stayers. Compressed
match has no controls and retains movers: 1,140 or 11,400 rows, with twice as many
physical copies. Observation/compressed routes use `probeorder(probe)`; generic
hybrid match uses its existing capability without that option.

Three smaller-input and two larger-input repetitions give the reported medians.
Cold commands start in fresh processes; warm commands follow one all-probe
command in the same process, excluding warmup time. Overhead is relative to
that version's conditional command: `100*(all/conditional-1)`. It is not a
comparison with an unimplemented MCSE-off option. Rust Counter-V1 and Mata
Stata streams differ; equal numeric seeds do not imply shared atoms.

Every command passes backend/engine, counts, data/sort/full caller RNG state and
absence-of-`e(V)` checks. All 540 within-backend result-family comparisons pass
at `1e-11`, with zero differences at recorded precision. Every attempted timing
is retained. CPU checks found no competing compute job before/after the commands;
they cannot exclude transient contention between checks. Stata/MP 19 uses the
existing eight-processor licensed configuration.

### Cold commands

| Rows / route | Previous Mata all, s | New conditional, s | New all, s | Added cost, s | Mata overhead old → new | Rust overhead | All speedup | Mata enabled RSS old / new, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,200 / generic observation | 1.231 | .840 | .977 | .137 | 47.2 → 16.3% | 9.4% | 1.26× | 43.29 / 43.11 |
| 1,200 / generic match | 1.227 | .846 | .979 | .133 | 46.8 → 15.7% | 8.9% | 1.25× | 42.83 / 43.61 |
| 1,200 / compressed match | 1.169 | .961 | 1.071 | .110 | 20.4 → 11.4% | 19.4% | 1.09× | 41.73 / 41.21 |
| 12,000 / generic observation | 5.210 | 1.883 | 2.469 | .586 | 176.7 → 31.1% | 22.8% | 2.11× | 94.85 / 88.76 |
| 12,000 / generic match | 5.410 | 1.918 | 2.552 | .634 | 184.9 → 33.1% | 14.9% | 2.12× | 89.36 / 88.82 |
| 12,000 / compressed match | 3.594 | 2.541 | 3.018 | .476 | 42.3 → 18.8% | 21.1% | 1.19× | 58.41 / 59.41 |

### Warm commands

| Rows / route | Previous Mata all, s | New conditional, s | New all, s | Added cost, s | Mata overhead old → new | Rust overhead | All speedup | Mata enabled RSS old / new, MB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1,200 / generic observation | .672 | .269 | .409 | .140 | 149.8 → 52.0% | 4.8% | 1.64× | 44.74 / 45.48 |
| 1,200 / generic match | .656 | .274 | .404 | .130 | 136.8 → 47.4% | 3.2% | 1.62× | 44.73 / 45.01 |
| 1,200 / compressed match | .528 | .325 | .425 | .100 | 64.0 → 30.8% | 9.3% | 1.24× | 43.38 / 43.12 |
| 12,000 / generic observation | 4.626 | 1.296 | 1.883 | .587 | 257.6 → 45.3% | 23.6% | 2.46× | 98.45 / 102.52 |
| 12,000 / generic match | 4.824 | 1.331 | 1.964 | .633 | 262.0 → 47.6% | 13.9% | 2.46× | 97.86 / 98.18 |
| 12,000 / compressed match | 2.934 | 1.878 | 2.359 | .482 | 53.8 → 25.6% | 19.7% | 1.24× | 63.90 / 62.69 |

The larger generic added cost drops approximately 82% cold and warm. Final
replay timers are about .47 seconds generic and .43 seconds compressed at the
larger size. Regenerating original atoms and extra fixed-effect solves now
dominate replay. Widening the admitted replay cap from four to eight improved
complete commands a further 2–9% in a separate interleaved development experiment.
Changing the point solver/RNG contract or caching every probe would require a
different scope and validation. Percentage parity with Rust is not achieved
for generic Mata, and the small warm percentages remain sensitive to a short
conditional baseline.

RSS includes Stata, libraries, loaded source and data; it is separate from payload
admission. Generic Mata reserves `192*physical_copies + 640*stored_rows + 512*R +
4096` bytes; compressed Mata reserves `576*deletion_units + 640*coefficient_cells
+ 512*R + 4096`. The measured larger generic bound is 12,394,496 bytes; compressed
is 3,572,096 bytes. These conservative bounds include fixed copy-fold weights
and eight-direction scratch. Memory admission can consequently refuse a tighter
Mata envelope than before. No direct Mata heap tracking or representative-scale
qualification is claimed; the Rust allocator evidence remains scoped to Rust.

## Numerical validation and calibration compatibility

The final integrated local driver passes 866 Python tests, CMG assembly,
Stata quick/full suites, clean portable/helper migration installs and downstream
smokes. Focused final-source tests additionally exercise replay width eight with
point batch 11 in both generic and compressed routes, including a compressed
target batch of five. Dense expanded-copy finite-difference oracles cover both
deletion/nuisance modes, combined stayers, odd folds and a 42,042-copy stress.
New reductions tests cover 70,000 panels across a 65,536-panel tile boundary,
`(1e16,1,-1e16)` cancellation, unequal/permuted panels, wide-panel fallback,
missing propagation, batched derivatives and diagnostic boundaries.

An original-stream equivalence check runs nine prescribed calibration input
regimes, R=T of 33/200/400, three fixed seeds and point batches 1/7: 162 fits per
version, 324 total. Every point fit converges; point and conditional families
match, all diagnostic statuses/missing patterns match, post-point stream states
are byte-identical and caller restoration is asserted. Maximum normalized
family error is `3.3306690738754696e-16`, against `1e-11`. These fits are numerical
equivalence evidence, not an empirical calibration campaign. Native frontend,
stale-runtime and shared injected Counter-V1 atom tests also pass with the
unchanged privately qualified arm64 artifact.

The existing prescribed dense/Counter-V1 complete and nested calibration raw
records are re-audited using the unchanged current validators. All 72 audit
outputs reproduce byte-for-byte: 239,616 existing attempts, all 540 complete
contrast screens passing, with maximum screens .10585323610286138 dense and
.10356563343093189 Counter-V1 against .15. These are reused draws, not fresh
calibration attempts. Source compatibility explicitly verifies unchanged
dense/reference/core/build/binary/input/seed/acceptance identities. The existing
calibration claim remains limited to those calibrated streams and routes; it
does not become broad empirical Mata RNG qualification through this rewrite.

## Source, failures and remaining qualification

HEAD remains the issue's reviewed `ecd62544a132906aad1ffa7b8e4a3aa0a03ee9c3`,
with the original issue implementation preserved. Final production and integrated
checks bind the 399-file source-v3 inventory
`eaf1848b437e282841615fff0dfb80860e509e36be11c557f7622140837a8327`.
Two later test-only width-eight regressions produce source-v4 inventory
`3e8b86ea050eeb285f5f66a4a7319b9ed599961920aa6fb70c717e091331e518`;
production/package/build identities are identical and the focused suite passes
on v4. The receipt records this compatibility and additive report/checkpoint
documentation separately. Classification remains `LOCAL_CHECKPOINT_DIRTY_TREE`.

Early local equivalence harness parse/setup failures produced no accepted fits;
they were repaired before v6. A native frontend staging attempt lacked LICENSE
and failed before tests; final staging includes the package license. A prototype
process-memory measurement hit the sandbox's sysctl restriction; the authorized
final runner uses successful process measurements. Long Mata helper-name and
reshape compile mistakes were repaired by focused regressions. An initially
missing new reduction helper failed before implementation, then passed.
One batch experiment overlapped other project compute work and supports no
final speed claim. Prior cap-four measurements are retained as superseded
development evidence. No final scientific gate failed, no timing was dropped,
and no cutoff, input, seed or estimator was tuned after the final registration.

M0–M4's earlier local development conclusions are preserved within their existing
scope. Current Mata correctness, original-stream equivalence, bounded storage
and local performance checks pass. Remaining qualification includes broad Mata
RNG calibration, representative scale/memory pressure, other platforms, clean
exact-commit candidate promotion, owner adoption and release. Previous native
qualification is carried only for unchanged Rust production/build/artifacts;
the new Mata frontend is locally tested on Apple Silicon. Installed plugin hashes
remain exactly the earlier restored runtimes; qualification artifacts stay private.
