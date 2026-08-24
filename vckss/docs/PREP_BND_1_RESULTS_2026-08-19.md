# PREP-BND-1 implementation and results

Date: 2026-08-19

Status: implemented and regression-qualified; retained as a safe cumulative
preparation-boundary simplification, not as a large-data preparation speedup
or a MATLAB-parity claim

Measurement baseline: `72179fb3627ac9699fb892ca721105930d560fc1`

PREP-MAP-1 candidate: `a83f902e71a77b205bb63976ce6df584f01480fc`

Cumulative PREP-MAP-1 + PREP-SEM-1 runtime candidate:
`f06e29e3a5bbb27cfddf3ac48e9596d60b95dbfa`

## Outcome

PREP-BND-1 removes two retained-sample worker/firm grouping passes, one
semantic grouping pass, and one Stata sort from the eligible compressed JLA
path. The graph selector now returns the final dense worker and firm maps, and
the compressed constructor computes exact per-copy target (`target/frequency`)
semantic ranks and lexicographic order in Mata. The public command, sample, graph, target,
random-atom, route, result, residual, failure, data-state, and RNG contracts do
not change. Runtime remains entirely Stata/Mata.

The small archive-isolated local fixture improves complete-command time by
`1.23%` and `1.39%` under source-order reversal. The semantic-order stage
improves by `30.77%`, and observed preparation improves by `1.63%` and
`2.43%`. These are small descriptive within-job differences. Each role has
only two warm repetitions, so the timing evidence does not support a precision
claim.

The fixed 8,201,888-row CZ18 holdout does not carry the preparation gain to
large data. The cumulative candidate's observed preparation is `1.90%` and
`1.85%` slower in AB and BA order; semantic ordering is `18.57%` and `26.69%`
slower. Complete-command time is nevertheless `0.22%` and `0.86%` faster,
because numerical-work timings happened to be lower in both jobs. That later
work is outside PREP-BND-1 and is not attributed to this optimization. The
MAP-only candidate is command-neutral on CZ18 (`+0.71%` and `-0.09%`) and
preparation-neutral (`+0.44%` and `-0.59%`).

MAP-only is consistent with the intended synthetic scaling mechanism: the returned-map
stage is about 34% faster at F4096 and 25% faster at F8192, while observed
preparation improves by median 1.43% and 1.93%. Complete-command timing is
noisy (`-2.47%` at F4096 and `+1.70%` at F8192), with the F8192 source orders
individually at `+8.63%` and `-5.24%`. Those reversals are why the stage
counter/timing evidence, rather than a single total, supports retaining MAP.

The code is retained because it removes redundant operations, has complete
semantic and regression coverage, improves the small local path, and
establishes a useful boundary for the next optimization. It does not meet the
advisory 35% preparation or 1.25x complete-command targets.

## Implementation

### Exclusive diagnostics

`PREP-BND-PERF-V1` records eleven exclusive stages: mark/validation, initial
grouping, runtime setup, graph setup/I/O, graph pruning, retained mapping,
semantic ordering, compression preparation, lifecycle transition, lifecycle
restore, and observed total. `PREP-BND-COUNTS-V1` records grouping, sorting,
import, and returned-map exposure. Timers and counters are diagnostic only;
they never affect admission, route, batch, convergence, or scientific
acceptance.

The instrumentation also fixes a prior diagnostic defect that reset the
semantic timer before posting `e(prep_profile)`. No numerical behavior changed
in the measurement commit.

### PREP-MAP-1

The graph selector accepts optional output variables and returns retained
worker and firm maps derived only from Stata's already generated complete-case
numeric group codes. It redensifies retained levels in ascending initial-code
order, which is exactly the retained-sample `egen group()` oracle for numeric
and string identifiers. The command consumes those maps and returned level
counts directly, eliminating the two post-prune `egen group()` calls and two
level summaries.

Graph API 21 and build identity
`varcomp-kss-graph-api21-prep-map1-retained` fail closed against stale loaded
code. Direct legacy graph callers remain valid through optional arguments.

### PREP-SEM-1

The eligible boundary is deliberately narrow: JLA, match deletion, no
controls, retained physical mass below `2^53`, and a non-generic engine
request. Mata receives the graph-returned dense maps plus deletion, frequency,
outcome, target, and optional probe order; it constructs exact stored-double
target/frequency ranks and a lexicographic semantic order. The compressed
constructor consumes that order directly.

Exact does not require semantic ordering; controlled, observation-deletion,
and forced-generic JLA paths retain the Stata semantic oracle. If automatic
compressed preparation fails and falls back to generic, the command builds the
unchanged Stata rank/order before the fallback. Scale API 6 and build identity
`varcomp-kss-scale-api6-prep-sem1-mata` enforce the new private boundary.

The eligible operation counts change causally as intended:

| Operation | Baseline | Candidate |
|---|---:|---:|
| Semantic grouping calls | 1 | 0 |
| Stata sorts | 2 | 1 |
| Post-prune retained worker/firm grouping calls | 2 | 0 |

All other import and preparation counts remain exact.

## Archive-isolated local paired comparison

The local harness archives baseline and candidate separately, uses one
hash-bound driver, and launches four fresh Stata processes. The fixture has
15,360 stored rows, 10,240 workers, 256 firms, P40, and four processors. Each
role has one cold and two warm rows; the table reports warm medians.

| Stage | Baseline first | Candidate first |
|---|---:|---:|
| Complete command | -1.233% | -1.392% |
| Observed preparation | -1.631% | -2.427% |
| Selection | -1.307% | -2.376% |
| Semantic order | -30.769% | -30.769% |
| Compression | +0.000% | -0.719% |
| Numerical work | -1.063% | -0.767% |

Every run passes the exact structural, sample, route, operation-count, data,
sort, and RNG comparison. Scientific results pass the registered `2e-12`
absolute/relative tolerance.

## Same-host Stata SCC scale ladder

Every AB or BA scalar job runs the measurement baseline and cumulative
candidate sequentially on one host, with one cold and two warm repetitions.
Synthetic cells use P256 and four actual Stata processors. CZ18 uses three
fresh-process repetitions per role, P20, seed 8675309, compressed CMG, four
actual processors, and the frozen retained input with SHA-256
`1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575`.

Changes are `100 × (candidate / baseline − 1)`, so negative values favor the
candidate. Each order-specific cell compares warm-repetition medians within
one sequential same-host job. Because each role has two warm repetitions, that
median is their midpoint. Across-order columns likewise summarize only the AB
and BA job-level changes; they are descriptive midpoints, not pooled estimates
or uncertainty measures.

| Case | Stored rows | AB command | BA command | Median command | Median prep |
|---|---:|---:|---:|---:|---:|
| F256, P256 | 15,360 | +2.40% | +1.15% | +1.77% | -2.00% |
| F1024, P256 | 61,440 | -1.57% | -0.65% | -1.11% | -0.07% |
| F4096, P256 | 245,760 | +0.81% | +2.02% | +1.42% | -1.01% |
| F8192, P256 | 491,520 | +6.76% | +1.00% | +3.88% | +1.24% |
| CZ18, P20 | 8,201,888 | -0.22% | -0.86% | -0.54% | +1.87% |

Replacement job 7237620 is the sole accepted F8192 AB timing source. It
completed on four slots with qacct `failed=0`, `exit_status=0`, and a
5,714-second wall time. At the 18:46 collection cutoff, original job 7236971
had not produced a complete candidate receipt or terminal pair marker.
Replacement 7237620 was submitted at 18:49, before 7236971 completed, and
therefore became the frozen ledger's AB source. Job 7236971 later completed
successfully at 19:09 but remains excluded under that pre-outcome supersession
decision; it is not evidence of a candidate defect. The accepted F8192 BA
source remains job 7236969.

The cumulative F8192 results do not show a large-synthetic preparation
speedup. The candidate is `6.76%` slower in AB order and `1.00%` slower in BA
order, for an across-order midpoint of `+3.88%`. Observed preparation is
`2.60%` faster in AB order and `5.08%` slower in BA order, for an across-order
midpoint of `+1.24%`. The command slowdown is concentrated in numerical-
work/Schur timing, outside the stages changed by PREP-BND-1. It is reported
but not attributed to the preparation-boundary change.

Timing thresholds are advisory. AB and BA are separate jobs and can share node
load with other jobs, so the order-specific receipts remain the primary
evidence. All accepted jobs have `failed=0`, `exit_status=0`, source and
driver hashes, application and wrapper markers, exact structural/sample/RNG
comparisons, and scientific differences below `2e-12`.

On CZ18, the cumulative candidate spends median `165.6` seconds in graph
pruning, `11.6` seconds returning retained maps, `10.4` seconds in semantic
ordering, and `60.4` seconds in compression preparation. Observed preparation
is `278.2` seconds, about 84% of the `333.0`-second command. Graph pruning alone
is about half the complete command. This profile, not the aspirational target,
sets the next priority.

## MATLAB comparison

### Contracts

The hard numerical gate uses the repository's independent dense MATLAB oracle
against Stata `algorithm(exact)`. On SCC MATLAB R2025b, plug-in, correction,
and corrected targets differ by at most `3.61e-15`, far below the registered
`2e-10`/`2e-9` tolerances. The source-bound SCC job and its post-job validator
both pass.

The scaled timing track uses the checksum-bound maintained `LeaveOutTwoWay`
tree at upstream identity `8b957ffe...`, tree SHA-256
`7d7581e77bcea131d0041cf7bab2d7a462fd5d535ca22110d080da51ded4f192`,
and core SHA-256
`7ab72bcf1f9e1a0091a6a423b1ef5cbd23688f7c64d753cf9adcc6243989a120`.
Each of 30 SCC jobs runs one Stata and one MATLAB process sequentially on the
same host in both process orders, with four Stata processors, four distinct
MATLAB pool workers, single-threaded numerical libraries, identical literal
stored rows, uniform stored-row targets, dimensions, requested probes, and
seed labels. Three seeds are used for every cell.

Maintained MATLAB is a descriptive comparator, not the equality oracle. Its
legacy finite-projection expression, RNG schedule, and solver tolerance differ
from `varcomp_kss`. Corrected estimates are therefore admitted only when
MATLAB's own PCG converges and are reported as gaps, never as an equality
test. All 30 maintained-MATLAB solves converged.

### Synthetic timing and numerical similarity

| Firms | Rows | Probes | Stata command | MATLAB command | Median paired Stata/MATLAB | Scaled corrected gap |
|---:|---:|---:|---:|---:|---:|---:|
| 64 | 7,680 | 20 | 1.991 s | 3.260 s | 0.62x | 1.21e-4 |
| 256 | 30,720 | 20 | 6.142 s | 5.720 s | 1.08x | 2.86e-5 |
| 256 | 30,720 | 200 | 26.277 s | 6.987 s | 3.69x | 2.70e-5 |
| 1,024 | 122,880 | 20 | 23.501 s | 14.112 s | 1.67x | 8.33e-6 |
| 1,024 | 122,880 | 200 | 122.043 s | 23.140 s | 5.27x | 6.69e-6 |

The comparison shows two different gaps. At P20, Stata crosses from faster at
F64 to roughly tied at F256 and 1.67x slower at F1024. At P200, the Stata
command is 3.69x to 5.27x slower. The probe-count slope therefore remains a
major parity target after real-data graph preparation.

Command columns are marginal medians across the six jobs in a cell. The ratio
column is the median of six within-job Stata/MATLAB ratios, so it need not equal
the quotient of the displayed command medians. The scaled corrected gap is the
cell median of `||Stata targets − MATLAB targets||₂ / (1 + ||Stata targets||₂)`.

Numerical similarity improves with scale and probes, but the maintained and
Stata corrected targets are not pathwise identical. Median
absolute total-target gaps fall from `2.69e-3` at F64/P20 to `1.69e-4` at
F1024/P200. The gap remains larger than within-method seed dispersion in some
cells, as expected from the different correction formulas. On the registered
dense-oracle fixture, the independent exact MATLAB gate has a maximum gap of
`3.61e-15`; this descriptive JLA comparison is not an equality gate.

MATLAB process startup is material: median pool startup ranges from 43 to 89
seconds, while MEX setup is about 5.5--5.8 seconds. The command table therefore
compares estimator calls, not scheduler wall. Median observed MATLAB process-
tree RSS is about 7.8--8.2 GiB; Stata GNU-time RSS rises from about 0.21 GiB at
F64/P20 to 1.12 GiB at F1024/P200. Scheduler `maxvmem` is around 49 GiB for the
combined job and is not interpreted as either process's resident set.

### Fixed CZ18 comparison

Three fresh maintained-MATLAB R2025b jobs use the exact frozen CZ18 input,
P20, seed 8675309, four pool workers, and the verified maintained source. All
three pass source, process-tree, dimension, target-identity, qacct, and
terminal-marker gates. Their MATLAB command times are `32.21`, `36.06`, and
`33.23` seconds, with a `33.23`-second median. The current Stata candidate's
AB/BA median is `333.02` seconds, about `10.0x` the maintained MATLAB command.

The phase boundary matters. MATLAB additionally spends median `54.43` seconds
loading/preparing/exporting the native row input, `7.48` seconds validating
it, `34.12` seconds starting the pool, `5.22` seconds on run-local MEX setup,
and `6.49` seconds tearing down the pool. Median whole-job qacct wall is 281
seconds and median observed process-tree peak is 9.93 GiB. Stata preparation
inside the estimator alone is about 278 seconds, dominated by graph pruning.

The corrected CZ18 targets are close descriptively but not equal:

| Target | Stata | Maintained MATLAB | Absolute gap | Relative gap |
|---|---:|---:|---:|---:|
| Worker variance | 0.0909240 | 0.0899593 | 0.0009647 | 1.06% |
| Firm variance | 0.0240707 | 0.0242248 | 0.0001541 | 0.64% |
| Worker-firm covariance | 0.0148040 | 0.0146081 | 0.0001959 | 1.32% |
| Total variance | 0.1446028 | 0.1434003 | 0.0012025 | 0.83% |

All three MATLAB repetitions return the same values at the fixed seed. These
gaps are expected under—and not interpretable apart from—the nonidentical
correction, RNG, and solver contracts; they are not failures of the dense
numerical oracle.

## Regression qualification

- Full local Python, generated-CMG, CMG Mata, KSS quick/full, clean-install,
  namespace/stale-runtime, benchmark, separations, and MATLAB-bridge gates
  pass after the runtime change, ending in `VARCOMP_KSS LOCAL QUALIFICATION
  PASS`.
- The full integrated runtime run reports 373 passing Python tests; after the
  benchmark analyzer edge-case test was added, the complete Python suite
  reports 374 passes.
- Direct semantic oracles cover numeric, string, labeled, and gapped IDs;
  exact ties; adjacent binary64 per-copy targets; frequency/target ranks;
  probe order; row permutations; and raw-versus-compressed atoms.
- Graph tests compare the returned mask and dense maps directly with retained-
  sample Stata `egen group()` output, including a string-ID pruning fixture.
- Exact, generic, controlled, observation-deletion, compressed, automatic-
  fallback, early graph/resource, and post-RNG paths retain caller data,
  labels, value-label definitions, sort state, `e(sample)`, and full RNG state.
- The clean-room dense MATLAB oracle passes at machine-precision gaps.
- All 30 scaled maintained-MATLAB pairs and all three CZ18 MATLAB jobs pass
  their fail-closed source/application/qacct validators.
- Ruff, deterministic bundle closure, the frozen legacy-name audit, and `git
  diff --check` pass.

## Interpretation and next optimization

PREP-MAP-1 and PREP-SEM-1 establish useful ownership boundaries and remove
four redundant Stata operations, but they do not reduce preparation time on the
fixed CZ18 benchmark. In that benchmark, graph pruning is the dominant measured
preparation stage at roughly 166 seconds, compared with about 12 seconds for
retained mapping and 10 seconds for semantic ordering. This profile identifies
the next stage to investigate; it does not establish that a graph redesign will
be faster.

Decision: proceed to `GRAPH-FP-1` as the next independently measured
milestone, but admit it in two gates. First instrument graph-pruning substages
and allocation counts. Only then prototype a command-local persistent
workspace, retaining it only if it passes exact graph-oracle and fallback
comparisons and has bounded wall-time and RSS extrapolations. The F8192 result
does not change this decision: its preparation changes have opposite signs
across orders, while its AB command result includes a numerical-work/Schur
spike outside PREP-BND-1. No state survives the command.

Implementation should be independently killable:

1. split graph-prune timing into component, mover-degree, articulation,
   bridge construction/search, deletion-panel sort, and active-mask update;
2. add operation/allocation counters and a resource charge for the persistent
   workspace;
3. implement the persistent edge/deletion representation without changing
   removal order or tie handling;
4. compare masks, dense maps, diagnostics, failures, and iteration counts
   exactly against the current graph oracle on exhaustive small and randomized
   multigraphs;
5. run local string/gapped/adversarial fixtures, then F256/F1024 AB/BA;
6. promote to F4096/F8192 only after measured wall/RSS extrapolation; and
7. rerun CZ18 in both source orders, retaining any safe cumulative gain even
   if it misses the advisory target.

After graph preparation, the P200 MATLAB ratios identify the second priority:
reduce per-probe RNG, Schur, and correction work with a separately measured
batched numerical candidate. The two targets should not be mixed in one
commit: graph work targets CZ18 P20, while numerical batching addresses the
probe-count slope. That separation supports causal attribution and keeps every
regression bisectable.
