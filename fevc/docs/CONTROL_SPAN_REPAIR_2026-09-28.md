# Control-span certification candidate — 2026-09-28

Status: tested candidate recommended for owner adoption on the qualified
platforms; adoption has not been performed. Baseline
source is `dc9b9d48beb04e83804527321934b269f4bf0a23`. Historical failed receipts
remain failed. No installed native artifact or frozen Separations package is
replaced by this work.

## Prospective numerical contract

The generic controlled JLA preparation retains its computed canonical columns,
anchor decisions, semantic ordering, sample, coefficient cells, deletion units,
target weights, nuisance treatment, and Counter domains. The candidate replaces
the three representation-error propagation gates with a full-fit and an
every-deletion **span perturbation** certificate. Existing structural rank,
original-system residual, information inverse, factor, deletion-maker, finite
output and accounting checks still execute.

Write `C` for the computed canonical columns, `X` for the exact stored original
controls, `T` for the stored square right transformation, `A` for the selected
original anchor matrix, and `W` for physical frequency weights. The posterior
check encloses `R=I-AT` and requires its induced norm to be below one. Therefore
`A` and `T` are nonsingular and `XT` spans exactly the requested controls.
The only control-span error is `F=C-XT`. Error in `T` relative to `A^-1` changes
coordinates; it must not be charged as a change in the control span. We bound
the **full** weighted `F`, including its FE-space component. Certifying only
`M_FE F` could miss a redistribution between fixed effects and nuisance controls
that leaves total fitted values unchanged but changes variance components.
An invertible right transformation of the controls preserves those components;
adding FE columns to the controls generally does not.

For a stored nonsingular right transform `B`, suppose the existing rank
certificate supplies `lambda > 0` below the least eigenvalue of
`B' C' W M C B`, where `M` is the weighted FE residual maker for the full or
deleted sample. Weighted orthogonal residualization and deletion are
contractions. An upper bound `f >= ||sqrt(W)F||_F` and
`b >= ||B||_2` imply

```
eta = f*b/sqrt(lambda)
delta = 2*eta + eta^2
relative_information_inverse_error <= delta/(1-delta),  delta < 1.
```

To see this, whiten the residualized `CB` by its exact Gram. The perturbation
has spectral norm at most `eta`; expand its perturbed Gram to obtain the two
cross terms and quadratic term. The Neumann inverse bound gives the last
expression. More precisely, for `H=B'C'WMCB` and perturbed `Hstar`,
this bounds `||H^(1/2) Hstar^-1 H^(1/2)-I||2`. It also bounds the
residualized-span projector distance by `eta/(1-eta)` when `eta<1`.
Neither expression is an elementwise coefficient-error guarantee. The original-information whitener and the certified generalized
minimum eigenvalue provide a conservative full-fit instance. The existing
within-cell whitener and minimum deleted-scatter gap provide the deletion
instance: within-cell residualization removes a superset of the FE span, so
its information lower bound also bounds FE-residualized information. Deleted
row restriction cannot enlarge `||sqrt(W)F||`.

The ceiling remains `1e-8` for this relative information-inverse certificate.
This is an internal numerical guarantee, **not** a universal absolute bound on
all variance components. Promotion separately requires the four-target policy
in `development_acceptance_v1.json`, independent literal-deletion oracles,
original-system residuals and the complete rank/deletion/accounting gates.
The diagonally scaled residualized-control rcond remains a rank/conditioning
diagnostic; it is not divided into an error measured in unrelated coordinates.
Its inverse and square-root computations retain their own residual checks.

There is no retry, random-dependent recovery, regularization, dropped column,
model change or alternate estimator. Preparation is deterministic and occurs
before estimator random draws. Exact and Mata paths retain their existing
propagation gates pending affected-surface qualification; shared Rust posterior
residual evaluation is sharpened without changing canonical values.

## Compensated residual evaluation

For each `value-X[row,.]*T[.,j]`, accumulate rounded products with TwoSum and
FMA product residuals. Sum the `2q` low parts ordinarily. If their computed
absolute sum is `a`, their summation error is at most
`gamma_(2q)*a/(1-gamma_(2q))`; include the final rounding and an absolute
`(4q+2)*eta_min/(1-gamma_(2q))` gradual-underflow allowance. Outward inflation
and the existing norm-reduction cushion enclose the aggregate norms.
Nonfinite products/intermediates make the posterior unavailable. This requires
IEEE binary64, gradual underflow, correctly rounded FMA, and no reassociation.
The span propagation calculation outwardly rounds the product `f*b` before
division by `sqrt(lambda)`, as well as the subsequent operations. This guards
against an underflowed intermediate product losing its enclosure after division
by a small information scale. A focused extreme-scale regression checks it.
The independent stored-input Decimal fixtures check the resulting enclosures.
The error-free primitives are TwoSum and TwoProductFMA (Algorithms 3.1 and 3.5
in Ogita, Rump and Oishi, [Accurate Sum and Dot Product](https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf));
the explicit low-part enclosure here includes its own underflow allowance.

Cost remains streamed `O(n q^2)` with small scratch; no production `n x n`
matrix or high-precision row computation is introduced.

## Development evidence

The logging-only baseline audit is SCC job `7766947`, isolated under
`/project/welfgr/separations/diagnostics/fevc_reliability_20260928/diagnostic`.
Eight prepared cases reproduce the expected baseline statuses. Raw row dumps
and anchor values stay in its mode-700 `private` directory on SCC; only
aggregate norms, logs, hashes and receipts are collected. Small matrices are
checked with 100/200-digit mpmath; streamed rows use 64-bit-significand Linux
long double, which is reported explicitly rather than called exact arithmetic.

New synthetic original-dummy SVD and 80/160-digit literal-deletion oracles
cover thirteen age/year controls, original match IDs distinct from coefficient
firms, pooled-firm movers, attached stayers, two explicit target mass systems,
and joint/fixed-offset nuisance. SVD and high precision agree within `1e-15`
on all four corrected targets. Native exact and Mata comparisons satisfy the registered `1e-8*scale`
four-target floor on all eight population/weight/nuisance combinations. JLA
comparisons use the registered common-draw and independent-randomized
allowances; they are not described as deterministic exact estimates.

## Baseline diagnosis

[Machine-readable audit](control_span_20260928_audit.json) records every
accumulated/direct contribution, norm, conditioning measure and independent
small-matrix check. The direct posterior is selected in all eight cases; its
absolute infinity-norm branch is active in each. The original diagonal gate
reproduces exactly these outcomes:

| Prepared design | Population | Selected E | Diagonal rcond | Old propagated error | Baseline |
| --- | --- | ---: | ---: | ---: | --- |
| CZ22 quarter | movers | 7.43894e-13 | 4.87884e-4 | 1.52473e-9 | pass |
| CZ22 quarter | combined | 7.02297e-13 | 3.86406e-4 | 1.81751e-9 | pass |
| CZ22 full | movers | 7.47190e-13 | 2.69759e-4 | 2.76985e-9 | pass |
| CZ22 full | combined | 4.80338e-12 | 1.62709e-4 | 2.95212e-8 | refuse |
| CZ25 full | movers | 3.01285e-12 | 1.68641e-4 | 1.78655e-8 | refuse |
| CZ25 full | combined | 3.91174e-13 | 9.62073e-4 | 4.06595e-10 | pass |
| CZ27 full | movers | 1.68352e-12 | 1.52637e-4 | 1.10295e-8 | refuse |
| CZ27 full | combined | 1.31072e-12 | 2.30933e-4 | 5.67577e-9 | pass |

For CZ22 combined, the accumulated alternative is `1.47515e-9`, versus
posterior relative/weighted error `4.37289e-13` and absolute error
`4.80338e-12`. Its infinity-norm contribution includes
`||C||inf=10.9505` times the anchor transformation residual enclosure
`4.29987e-13`. This is principally right-coordinate error. The streamed
independent measurement of `||C-XT||F` is `9.00993e-14`, while the old ordinary
residual enclosure was `7.79169e-12`. The high-precision anchor inverse agrees
at 100 and 200 digits to about `8e-106` relative. The stored small Schur inverse
has independent Frobenius residual `2.95998e-13`; the corresponding CZ25 and
CZ27 mover values are `3.59445e-13` and `6.08900e-13`. These are measurements
of the specified small matrices, not a high-precision refit of the entire FE
system or a literal-deletion oracle for the full private data.

Adding attached-stayer observations adds positive-semidefinite information in
the original coordinates. It can change the canonical anchors and the
coordinate-dependent diagonally normalized ratio; neither change establishes
loss of identification. Here the generalized margins remain about 0.03. The
repair does not substitute those ratios into the old scalar formula: it maps
the absolute span residual through the same stored whitener and a certified
least eigenvalue before deriving a Gram perturbation bound.

## Downstream coordination

The Separations parser must distinguish Stata missing values from finite
numbers and reject missing **required** outputs. A valid primary maximum-support
fit, an unavailable supplementary mover fit, and complete campaign success are
separate statuses. The FEVC native failure code, exact phase and
`FEVC_CONTROL_CERT_V1` fields are available for that stage receipt.

Publish successful scientific stages and fitted-person restart inputs through
manifests and atomic checkpoint publication before optional report validation.
Do not omit CZ22 from pooling or substitute mover-only estimates. The canceled
CZ18 jobs and pools remain canceled. Their censored whole-pipeline durations
are not FEVC timings; preparation, legacy comparison, reghdfe and PPML need
separate downstream profiling. No downstream wrapper changes are made here.


## Qualified surfaces and evidence accounting

The intended data scope is the prepared 2002–2009 BW SCC-person population,
with the frozen worker-history assignment, not the national IAB population.
No coefficient firm, deletion ID, eligibility mask, outcome, nuisance setting
or target mass is altered. The existing deletion-ID mover and `if`/`in`
regressions pass with the candidate native package.

The A8 candidate snapshot is SHA256
`a8fb7ebd1b97b5969b4425d4917be959d9a4eb0c34b7df3ff5dd42a162eedab3`.
The frozen final B snapshot, with an additional outward-rounding guard, is
`7f770ddafd96b3223a18d1314779e96873cfed6e712e28e1890bd178e66f8bb7`.
Final B passed all thirteen retained-design calls in SCC job `7767513`,
including all three prior refusals, their paired populations, quarter CZ22,
CZ24 and coherent combined weights in CZ22/CZ25/CZ27. Its 208 comparisons
(four targets times plugin/correction/corrected/MCSE times thirteen calls)
agree exactly with A8 as recorded in the emitted binary64 CSV values. Its
four-thread runs on `scc-tb4` are verification runs, not the matched-host
4/28 benchmark. Both are dirty-source bundles based on the baseline commit; a commit name
alone does not identify these candidates. Final artifact identities and the
A8-to-B compatibility review are recorded in the candidate manifest.

SCC job 7767238 passed ten native-population calls, then encountered a **harness
failure** (`r(111)`, absent `sb_reference`) before the first coherent FEVC call.
It remains a failed job (11 attempted, ten successful, one harness failure,
39 unattempted); its original fail-fast receipt is not relabeled successful.
Prepared designs predate construction of the reference graph. The one narrow
fix in job 7767296 reconstructed that graph with frozen `sc_candidate3` and
applied the original coherent-weight formula. Every reference/coherent mask
and target mass matched retained CZ25/CZ27 `new_effects.dta` (weight tolerance
`1e-15`). For CZ22, where that downstream file does not exist, construction
uses the same frozen source and the successful prior finding-stage invariant
that `sc_logf` is available on all common AKM rows. No PPML refit or branch search
was run. Its exact inputs, assertions and hashes are retained separately.

The frozen continuation (job 7767323) passed all 41 calls, including a repeated
CZ22 bridge cell on the timing host. Combining it with the ten earlier
successes covers all 50 originally planned logical cells. All 60 four-target
comparisons across control permutations, invertible combinations/rescaling,
reversed row order, thread counts and profiling/plain builds pass; the largest
absolute difference is `1.588e-14`, below even the deterministic floor.
The private full data do not have a literal-deletion/high-precision
four-target oracle: their independent checks are the streamed span residual,
high-precision small matrices and original-system residuals, alongside these
four-target invariance checks. The separate synthetic oracle checks the
statistical formula and nuisance semantics independently of production code.

Synthetic tests include the alternative polynomial coordinates
`x^2(1+x), x^2(1-x)` within each education group; these are an invertible
combination of the requested quadratic/cubic pair. Simply recentering a
quadratic/cubic polynomial without its missing lower-order terms would not
necessarily preserve the requested nuisance span.

Exact/Mata and generic JLA negative tests retain explicit failures for strict
rank loss/duplicate columns, indefinite or ill-identified deletion blocks,
large complete original-system residuals, nonfinite arithmetic and insufficient
span accuracy. Duplicate controls remain rejected under the current strict
rank policy; they are not silently dropped. Failure-channel tests cover stale
results, caller data/RNG preservation, and normal, quiet and verbose error
retrieval. Native thread tests cover exact, automatic/compressed and generic
JLA, diagonal and CMG solvers, joint and fixed-offset nuisance, movers and
combined samples, projection and component attachments, and invalid budgets.
The 28-thread tests ran inside a 28-slot allocation with Stata at four cores;
Apple Silicon tests use four/eight native threads.

Source checks: 831 Python tests; generated CMG check; full local check runner
(including Stata quick/full and isolated portable install); Rust workspace
and standalone backend tests, feature-enabled profiling tests, format and
strict Clippy. The broad local Stata runner uses the previously installed
native artifact; candidate native qualification is supplied by the separate
explicit-package tests and focused earlier-repair regressions. Final B repeats
focused arithmetic, independent-target/interface tests and retained cases;
it does not pretend that every broad gate executed every candidate binary.

## Retained-design results and resource use

The following are complete FEVC command times at 200 probes on `scc-tq3`
(Xeon E5-2680 v4, 28 physical cores, no SMT), using A8 and 28 native threads.
Peak resident memory includes the Stata process and its loaded design. All
rows use thirteen controls and joint nuisance; the seeds are 9252026 for
movers and 9252027 for combined samples. Four corrected targets, finite
outputs, sample/deletion identities, accepted draw counts and caller state
are validated per call. These are qualification runs, not published
Separations results.

| Design | Population / weights | Rows | Deletion units | Seconds | Peak RSS MiB | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| quarter | movers / native | 38,057 | 1,758 | 2.546 | 206.2 | pass |
| quarter | combined / native | 41,731 | 5,432 | 3.211 | 230.3 | pass |
| cz22 | movers / native | 332,340 | 15,214 | 19.190 | 873.0 | pass |
| cz22 | combined / native | 359,402 | 42,276 | 25.327 | 1095.8 | pass |
| cz25 | movers / native | 262,401 | 11,381 | 15.520 | 703.3 | pass |
| cz25 | combined / native | 281,639 | 30,619 | 19.271 | 881.8 | pass |
| cz27 | movers / native | 373,166 | 16,355 | 21.678 | 974.5 | pass |
| cz27 | combined / native | 404,536 | 47,725 | 28.851 | 1225.0 | pass |
| cz24 | movers / native | 161,265 | 7,484 | 9.008 | 465.0 | pass |
| cz24 | combined / native | 175,515 | 21,734 | 11.888 | 582.6 | pass |
| cz22 | combined / coherent | 359,402 | 42,276 | 25.198 | 1102.3 | pass |
| cz25 | combined / coherent | 281,639 | 30,619 | 19.038 | 882.7 | pass |
| cz27 | combined / coherent | 404,536 | 47,725 | 28.370 | 1221.5 | pass |

The three original refusals (CZ22 combined, CZ25 movers, CZ27 movers) now
pass, as do their paired passing populations, both CZ22 quarter cases and
both CZ24 repair-witness populations. Coherent combined weights are separately
qualified for CZ22, CZ25 and CZ27.

For CZ22 combined/native at 200 probes, three plain-build command repetitions
on the same host had medians **28.494 seconds at four native threads** and
**25.283 seconds at 28** (1.127x speedup, 11.3% less wall time). At this budget,
28 slots consume about 6.2x the allocated core-seconds. Use 28 when latency is
the priority; the unchanged four-thread default remains economical. The
comparison is not a benchmark against the KSS Matlab package.

One profiling-build call per thread count decomposes the native work below.
Rows are inclusive phase times; nested solver timers are not added again.

| Phase | 4 threads, seconds | 28 threads, seconds |
| --- | ---: | ---: |
| Control canonicalization / posterior certificate | 8.917 | 8.997 |
| Control solver preparation | 0.936 | 0.915 |
| Full fit | 0.038 | 0.039 |
| Control geometry / deletion validation | 4.905 | 4.892 |
| Leverage probes | 0.825 | 0.342 |
| Target probes | 3.736 | 0.803 |
| Other native command work, exclusive | 3.209 | 3.197 |
| Native estimator command total | 22.567 | 19.185 |

At 28 threads, native ingest/semantic canonicalization/graph/compression/
stayer augmentation add about 1.146 seconds outside the estimator command;
the full native boundary takes 20.331 seconds. Remaining Ado/lifecycle work
accounts for the gap to the complete FEVC command. This profile does not
separately identify every Ado validation substep. No dense large-sample
fallback or retry is introduced. The 6h33m48s and 3h54m05s canceled CZ18
pipeline observations are censored whole-pipeline times and are not compared
with these FEVC measurements.

## Probe precision and recommendation

The retained CZ22 combined fit was run with both target masses at budgets
200, 512 and 2,048 and seeds 9252027, 930010, 930011 and 930012. The reported
numerical MCSE conditions on the leverage sketch. Across-seed variation also
includes changing that sketch and its interaction with the target probes.
The two must not be added as if they were independent standard errors.
The machine-readable summary records all four targets, means, ranges and the
signed diagnostic `across_seed_variance - mean(conditional_MCSE^2)`; with only
four seeds this difference is noisy and is not a reliable leverage-variance
estimator or coverage test.

| Weights | Probes | Worker conditional MCSE, RMS | Worker across-seed SD | Covariance conditional MCSE, RMS | Covariance across-seed SD | Median FEVC seconds |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| native | 200 | 0.001146 | 0.002204 | 0.001015 | 0.002088 | 25.381 |
| native | 512 | 0.000713 | 0.001490 | 0.000628 | 0.001196 | 27.895 |
| native | 2,048 | 0.000362 | 0.000746 | 0.000316 | 0.000725 | 40.809 |
| coherent | 200 | 0.000873 | 0.001533 | 0.000698 | 0.000854 | 25.252 |
| coherent | 512 | 0.000551 | 0.001271 | 0.000448 | 0.000709 | 27.654 |
| coherent | 2,048 | 0.000278 | 0.000342 | 0.000228 | 0.000247 | 40.177 |

Use **200 for diagnostics, 512 for exploratory iterations, and 2,048 as the
starting budget for reported decompositions** on this class of retained fit.
Here 2,048 costs about 15 additional seconds at 28 threads versus 200, while
substantially reducing random variation. It is not a universal precision
threshold: at 2,048, native worker across-seed SD is about 1.0% of its mean,
and native covariance about 3.9%. Coherent values are about 0.45% and 1.9%.
Repeat independent seeds and raise a fixed, predeclared budget if the desired
scientific contrast is smaller. Four seeds do not establish tail probabilities
or adequate precision for every region. No automatic stopping/controller or
RNG contract change is introduced, and `1e-8` deterministic numerical accuracy
must not be advertised as Monte Carlo accuracy at these budgets.

## Remaining limits

The repair addresses generic JLA propagation of canonical representation
error. It retains earlier canonical-anchor/whitening checks and the sufficient
within-cell deletion-rank certificate. Deliberately difficult synthetic age
spreads still produce explicit inconclusive canonical-construction refusals;
some independently identified small designs can fail the sufficient deletion
certificate. Those are not proof of structural nonidentification. A future
remedy would need a separately certified QR/SVD construction or a sharper
per-deletion information bound. No fixture-specific exception is added here.

Qualification covers Stata/MP 19 on Apple Silicon macOS 26.6.2 and SCC
x86-64 Linux, using Rust 1.85.1 (LLVM 19.1.7). Intel/Rosetta,
Mac universal artifacts and Windows are untested for this candidate. Exact
Rust retains the earlier propagation gates; its shared posterior arithmetic
and relevant Mata parity are tested. This work does not requalify the separate
`fereg` executable or claim faster downstream legacy/reghdfe/PPML stages.
No campaign restart, pooling, installed-package replacement, publication,
tag or release is performed.


## Reproduction commands and private artifacts

The local evidence root is `.local/control-reliability-20260928/`; the SCC
root is `/project/welfgr/separations/diagnostics/fevc_reliability_20260928/`.
Only aggregate evidence and synthetic data are local. Private row streams and
anchor values remain on SCC. The frozen `qualification`, `qualification-v2`,
`coherent-retest` and `final-b` payloads contain their exact manifests, commands,
validators and hash inventories. They are diagnostic evidence, not release
archives. Reproduction must use a new output directory rather than overwrite
any frozen receipt. The SCC launchers enforce `NSLOTS`, require compute-node
scratch, hydrate/hash-check archived inputs and use `-P welfgr`.

From the repository root, the principal local commands are:

```bash
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
./.venv/bin/python fevc/tools/run_checks.py
./.venv/bin/python fevc/tests/oracles/control_span.py NEW_ORACLE_DIRECTORY
cargo +1.85.1 test --manifest-path rust/Cargo.toml --workspace --locked
cargo +1.85.1 test --manifest-path rust/Cargo.toml --workspace --features pipeline-profile --locked
cargo +1.85.1 test --manifest-path rust/stata_backend/Cargo.toml --locked
cargo +1.85.1 clippy --manifest-path rust/Cargo.toml --workspace --all-targets --features pipeline-profile -- -D warnings
cargo +1.85.1 clippy --manifest-path rust/stata_backend/Cargo.toml --all-targets -- -D warnings
```

Native interface tests run in fresh Stata processes with arguments, in order:
`test_control_span.do PACKAGE ORACLE_DIRECTORY OUTPUT_LOG UPPER_THREADS`,
`test_control_failure.do PACKAGE OUTPUT_LOG`, and
`test_native_threads.do PACKAGE UPPER_THREADS OUTPUT_LOG`.
The final Mac polynomial extension is test-only and executes against the
unchanged final B native artifact; Linux's frozen test includes the mixed,
rescaled and reversed-coordinate cases. Existing anchor/posterior, pooled
firm/deletion, subsample, native progress and controlled-CMG tests were also
run against the private candidate.

See [the aggregate qualification receipt](control_span_20260928_qualification.json)
and the private [adoption instructions](../../.local/control-reliability-20260928/ADOPTION.md).
The latter locate the matched platform folders, exact source, artifact/receipt
manifest and fresh-session instructions. Owner adoption and public release
remain separate decisions.
