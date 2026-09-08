# Match deletion: separate individual intervals from joint covariance reporting

## Result and decision

The shared joint-covariance check withholds many computable match-deletion
intervals. It should not be the sole availability check for an explicitly
individual-interval result. However, removing that shared rejection would not
make every individual interval usable: target-specific failures must remain.

The existing match variance fitter was kept unchanged. The audit replayed all
5,189 joint-PSD-rejected calls from the two original match confirmations, plus
16 prospectively selected successful comparisons. All 5,205 calls and 20,820
target attempts reconcile. Every original rejection still occurs; every
successful comparison reproduces its original output. No production source,
public reporting rule, point estimate, tolerance, or old confirmation changed.

For the **5,000 rejected match-q1 calls**, the existing target finalizers give:

| Target | Computable individual q1 intervals | Still unavailable |
|---|---:|---:|
| Worker variance | 4,918 / 5,000 (98.36%) | 82 |
| Firm variance | 4,909 / 5,000 (98.18%) | 91 |
| Total variance | 4,994 / 5,000 (99.88%) | 6 |

All three intervals are computable together in **4,828 / 5,000 calls
(96.56%)**. The remaining 172 calls have at least one genuinely nonpositive
target-specific remainder variance. The valid pairs are not near the singularity
threshold: their smallest standardized determinants are 0.9783, 0.9725 and
0.9329, respectively, versus the unchanged `1e-8` threshold.

The covariance-component q1 interval is computable in 2,879 / 5,000 calls.
Its 2,121 failures comprise 2,118 nonpositive-variance cases and three singular
pairs. It remains outside the primary one-mode calibration claim; computing
it does not establish a diffuse remainder.

For the **189 rejected match-q0 calls**, marginal variances remain positive for
120 worker, 126 firm, 189 covariance-component and 182 total estimates.
All four are positive in 58 calls. Thus individual q0 reporting also has a
useful role, but some scalar variances genuinely require withholding.

These are **computability results, not new coverage qualification**. The q1
rejections are 2,500 weak-signal and 2,500 zero-signal draws. The q0 rejections
are 144 zero-signal, 44 weak-signal and one multi-mode draw. Those cases remain
outside the original primary calibration claims. Neither original match PASS
nor the observation confirmation FAIL is relabeled.

## Why the joint and individual checks differ

The joint matrix describes the worker-variance, firm-variance and covariance
estimates together. A valid joint matrix is needed for joint inference and
arbitrary linear combinations of those estimates. An individual q0 interval
instead needs its own positive scalar variance. An individual q1 interval
uses a different two-coordinate covariance matrix: its leading-mode score and
rank-one-subtracted remainder.

The current implementation checks the shared matrix before returning either
kind of individual result. The audit reads already prepared quantities before
that gate and then lets the original gate run unchanged. It does not clip
eigenvalues, replace a negative variance, switch q, or fabricate a joint matrix.

The match result also differs from the preceding observation t8 diagnosis.
On the fixed dense slice of **128 weak/null match-q1 draws**, the full joint
matrix remains invalid in every case with exact traces, both with fitted and
with true aggregate error variances. Thus the shared failures here cannot be
attributed solely to the variance learner or random trace probes. The
realized covariance estimator subtracts a trace term from a random influence
term; under weak signal that subtraction need not yield a positive matrix,
even with true variance inputs. This is not a claim that the population
sampling covariance is indefinite.

For q0, exact traces with fitted variances produce positive joint matrices in
57 of the 189 rejected draws; true variances and exact traces do so in 63.
Probe noise therefore explains some q0 rejections, but not all. These are
diagnostic changes to the inputs of the same realized covariance estimator,
not repaired production results.

## Independent checks and scope

The [prospective protocol](match_target_interval_audit_v1.json) fixes every
rejected call and selects the first four fully successful calls in each q0
rejected design, plus four regular-signal q1 comparisons with the same match
mass/dependence regime. There are 400 independent matches per call. Original
outcome seeds, outcome-free fold seeds, 256 JLA probes, 512 covariance probes,
128 spectral probes, 256 spectral iterations and 4,000 native critical draws
are retained. The low critical-draw count reproduces the old campaign; it
does not change the public minimum. Both old campaigns used deterministic
quadrature for their recorded scientific interval endpoints.

All calls retain native marginal moments and q1 statuses. The independently
specified dense slice contains all 201 selected q0 calls, the first and last
32 original keys in each rejected q1 design, and the four q1 comparisons:
333 calls total. The dense calculation reconstructs mass-scaled scalar match
rows from the original physical rows, the regression projector, target
quadratic forms, corrected kernels, and rank-one remainder kernels. It checks
influence actions and covariance quantities using the captured stochastic
trace terms, then separately substitutes exact traces. It does **not** claim
an independent reconstruction of all 512 native Gaussian draws or a new
independent validation of the unchanged variance fitter.

Independent Gaussian quadrature and quartic stationary points verify the q1
interval construction. The largest scale-relative reconstruction difference
is `3.49e-9`, below the registered `1e-8` gate; it occurs in a covariance-target
q1 quantity. The worker/firm/total maxima are at most `7.38e-11`.
Collapsed-outcome, q0 influence and q0 covariance differences are at most
`4.23e-16`, `6.92e-15` and `6.80e-17`. All successful baselines reproduce the
old points, covariance diagonals, fold fingerprints and quadrature intervals.

Original variance-fit positivity failures are not rescued or omitted from
their campaigns: 64 q0 and 57 q1 calls are separately enumerated in the replay
manifest and excluded from this PSD-specific audit. The original q1 campaign's
208 already target-local unavailable attempts likewise remain unchanged.

## Recommended next implementation

Implement one **internal individual-interval reporting contract** shared by
observation and match deletion, retaining the appropriate existing fitter for
each route. Keep strict joint-request behavior unchanged.

1. Give the joint matrix an explicit availability status. In an individual-only
   result, withhold an invalid joint matrix and any postestimation requiring it.
2. Give each q0 interval its own positive-variance check; retain each q1
   leading-mode certification, pair-positivity, determinant and interval check.
   Preserve the raw leave-out recenter and target-keyed RNG streams.
3. Regress unchanged successful outputs, partial availability, withheld joint
   results, resource/Counter accounting, and typed failure behavior on both
   deletion routes. Do not automatically switch q or replace the match fitter.
4. Register a broader existing-draw development matrix before interpreting
   availability/calibration under the new contract. A separately authorized
   fresh confirmation and versioned public integration would follow; neither
   is supplied by this audit.

This is a common reporting improvement, not a solution for estimated-offset
uncertainty, unrestricted aggregate heteroskedasticity, arbitrary within-worker
dependence, null coverage, or multi-mode inference. The existing match-focused
release candidate and its limitations remain intact.

## Reproducibility and checks

The q0 source is `bb580fe69085d1f98c9151cea2de038aec0a8ba6`; the q1 source is
`4a68ea2ae8b77f7b134a827c74b56f5d3e92c912`. Their production/build/RNG sources
and q1 harness are byte-identical. The sole instrumented original path is
`rust/crates/vckss-core/src/generic_jla.rs` in a disposable source copy.
Generated diagnostic executables are not qualified plugins. The live
observation candidate's previously frozen source files are also unchanged.

The frozen replay manifest SHA-256 is
`daed6753d4e0d8673bdf08877ff0da7bf021d32fa5e94c86dc6fb59ce964e6a8`.
The local four-worker replay took 482.7 seconds. Complete raw captures,
receipts, selected baselines, original source/input identities, exact-trace
results and test logs are retained in the archive indexed by
[the machine-readable result](match_target_interval_audit_v1_result.json).
The runner and independent audit are under
`rust/experiments/match_target_interval_audit/`.

Checks: 20 new focused tests; 45 combined observation/match audit tests using
`--import-mode=importlib`; 726 repository Python tests; CMG generated-source
check; integrated Stata quick/full/install checks, ending in
`FEVC LOCAL QUALIFICATION PASS`. The invalid replication-key test exits 101
before generating an outcome. No full plugin, platform, SCC or Windows
qualification was run for this diagnostic-only change.

Two development-harness issues remain documented rather than hidden: the first
tiny validator did not allow matching inapplicable q1 fields in q0 output;
the fixed parser now has a regression. A combined pytest invocation initially
hit identical test-module basenames; importlib collection passes all 45 tests.
A premature smoke invocation stopped on the absent build receipt without
running a draw. Existing pytest temporary-directory cleanup warnings are
nonfatal. None changes the protocol, estimator, acceptance thresholds or an
old scientific result.
