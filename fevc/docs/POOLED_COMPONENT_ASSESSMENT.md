# Pooled component development assessment

The registered bounded assessment completed 16,000 calls and 64,000 target
results on the corrected source. All 320 tasks passed execution and inventory
validation. Every primary interval was available and every registered primary
descriptive screen passed. This is evidence for the specified synthetic designs,
not general coverage qualification.

The [machine-readable report](pooled_component_inference_assessment_v1.json)
contains every cell summary, paired-arm comparison and failed target, together
with exact frozen manifest and output hashes. Raw inputs, outputs, interpreter
logs and task receipts remain in the recorded local directories. The
[registration](pooled_component_inference_validation_v1.json),
[amendment](pooled_component_inference_validation_v1_amendment1.json) and
[numerical registration](pooled_component_inference_numerical_v1.json) retain
their pre-outcome definitions. Native distribution status is separate in
[the checkpoint](../PLAN.md).

## Primary results

Each row below represents 1,000 outcomes with both observed-Mean and
fixed-population-mean calls. The table reports the range of all-attempt coverage
over the primary targets in the observed-Mean arm, for nominal 95% intervals.
All four highrank targets are primary. For q1, worker, firm and total are
primary; covariance is a predeclared two-mode diagnostic.

| Backend | Reference | Stayer target mass | Primary availability | Coverage range |
|---|---|---:|---:|---:|
| Exact Mata | Highrank | 25% | 100% | 93.9–95.3% |
| Exact Mata | Highrank | 75% | 100% | 93.3–94.9% |
| Exact Mata | q1 | 25% | 100% | 94.8–96.7% |
| Exact Mata | q1 | 75% | 100% | 96.0–97.1% |
| Rust/JLA | Highrank | 25% | 100% | 96.0–96.7% |
| Rust/JLA | Highrank | 75% | 100% | 93.6–95.7% |
| Rust/JLA | q1 | 25% | 100% | 94.2–97.0% |
| Rust/JLA | q1 | 75% | 100% | 95.8–98.7% |

The designs use unequal mover-unit masses, attached stayer histories of 2/4/8
observations, and separate positive quadratic variance functions by unit type.
Their realized spectral regimes, identification, support and positivity were
verified before outcome draws. Rust uses 200 point probes, 2,048 residual-Gram
probes and 1,000 covariance simulations; exact Mata uses the exact Gram and
1,000 covariance simulations. Both use the existing q1 critical-draw floor.
Outcome draws differ by backend's semantic cell key, so coverage differences
are not paired comparisons of the two implementations.

Across both arms, 32 q1 covariance results were withheld: eight in Mata at
25% stayer mass, six in Mata at 75%, and eighteen in Rust at 75%. They remain
in all-attempt denominators and in the complete failure inventory. There were
no atomic call failures. The largest observed coverage drop from using the
sample mean was 0.1 percentage point. The exact mean-approximation discrepancy
was at most 0.000421 sampling standard deviations in these designs; this does
not justify ignoring mean estimation uncertainty in arbitrary applications.

## Stress and numerical limitations

Nine separate 100-outcome stress cells completed 1,800 calls and 7,200 target
results. There were no execution failures, but 446 target-arm results were
withheld. These cells are diagnostic and have no primary coverage acceptance
gate. In the observed-Mean arm:

- The q1 null worker/total targets had 44% availability and 42% all-attempt
  coverage; covariance had 52% availability and 51% coverage.
- The deliberately multiple-mode q1 worker target had 77% availability and
  74% all-attempt coverage.
- Weak highrank firm/covariance targets had 91%/90% availability and
  86%/83% all-attempt coverage.
- Severe variance-model misspecification gave 93% availability and 82%
  all-attempt coverage for the q1 covariance diagnostic.

The separate fixed-outcome grid completed all 96 calls without withheld
targets. Across the primary targets, increasing point probes from 200 to 800
changed the more affected interval endpoint by a median 2.35% and maximum
23.00% of the baseline interval width. Increasing Gram probes to 8,192 gave
1.40% and 9.15%; increasing covariance simulations to 4,000 gave 0.058% and
0.276%. These paired budget comparisons use four fixed outcomes and three
numerical seeds. They describe sensitivity, not convergence certification or
an acceptance cutoff. No budgets or scientific screens were retuned afterward.

## Preserved engineering repair

The first primary attempt was stopped after an independent solver-invariance
regression detected that roundoff in structurally zero stayer firm/covariance
features created spurious variance regressors. The repair sets those two
basis features to their mathematical zero before ranking. It does not replace
point estimates, target diagonals, variances or displayed numerical errors.
The stopped attempt and its outputs remain preserved. Amendment 1 registered
the repair and a fresh outcome domain before the replacement assessment;
designs, budgets, estimands and cutoffs were unchanged.

## Complete-command timing and memory

The [performance record](pooled_component_inference_performance_v1.json) binds
the drivers, runtime, plugin, seeds and raw measurement receipts. These are
single serial runs on an Apple M2 Ultra with 192 GiB RAM, macOS 26.6.2 and
Stata/MP 19, using two Stata processors and two native threads. Each size has
equal counts of mover and stayer workers, 100 firms, one estimated control held
fixed for inference, and nonuniform target weights. The commands include default
all-probe point MCSE, 200 point probes, 2,048 Gram probes and 1,000 covariance
simulations. RSS is the process maximum, including Stata.

| Stored rows | Solver | Reference | Command seconds | Peak RSS, MiB | Available intervals |
|---:|---|---|---:|---:|---:|
| 2,400 | Diagonal | Highrank | 2.41 | 51.1 | 4/4 |
| 2,400 | Diagonal | q1 | 1.60 | 50.5 | 1/4 |
| 24,000 | Diagonal | Highrank | 13.04 | 106.9 | 4/4 |
| 24,000 | Diagonal | q1 | 8.32 | 103.7 | 3/4 |
| 240,000 | Diagonal | Highrank | Rejected | 525.9 | None |
| 240,000 | Diagonal | q1 | Rejected | 542.4 | None |
| 240,000 | CMG | Highrank | 57.03 | 605.4 | 3/4 |
| 240,000 | CMG | q1 | 41.98 | 582.0 | 4/4 |

The registered large diagonal calls both failed `FULL_RESIDUAL_FAILED`, with
complete original-system residual 1.79e-9 against the 1e-9 gate, after roughly
39 seconds of process time. These are preserved numerical rejections, not
successful timings. The two CMG runs were declared afterward as engineering
diagnostics on the same fixed data, seeds and budgets, using the unchanged
acceptance rule. They do not replace the failed diagonal cases. The covariance
interval was withheld in the large CMG highrank call; all q1 intervals were
available. These measurements neither certify the reference-distribution
conditions nor promise representative-data performance. The first profiling
attempt also preserves a sandbox-denied macOS RSS query; the estimator itself
completed, and profiling was rerun with host-statistics access.
