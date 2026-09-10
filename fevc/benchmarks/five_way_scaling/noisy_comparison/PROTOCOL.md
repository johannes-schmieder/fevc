# Noisy-fixture comparison, 2026-09-09

The owner requested a slightly larger sample with economically meaningful bias
correction. This descriptive diagnostic is separate from the failed frozen
scaling preflight and does not promote or alter that evidence.

One fixed input: 3,840 rows, 1,280 workers, 32 firms, three unique matches per
worker, no controls/stayers/weights beyond unit mass. The original strong_d3
graph formula is retained. Independent PCG64 streams generate worker effects,
firm effects and Gaussian noise, with seeds 20260909201/20260909202/20260909203.
Latent effects are centered and scaled to finite-design variances 0.25 and
0.0625 before generating the outcome; independent noise has standard deviation
2.0. The final outcome is centered once for all packages. This deliberately
eliminates the previously identified difference in outcome centering inside
the correction score. One fixed draw, no seed search or resampling.

Before comparator execution, the independent dense reference must show a
worker correction of at least 25% of the plug-in value and a firm correction of
at least 20%. Record the realized corrections, identification and sample checks,
R-squared, known finite-design targets and both original/centered-score oracles.
The latent truth is context: a single corrected realization need not equal it.

Reuse the prior run's prepared environments and exact comparator versions.
Copy its hash-verified adapters into a new run; widen only their input guards
to admit N=3840. Do not fix comparator formulas or solver tolerances. Retain
population-N normalization, 100 ms process-tree RSS and primary timing phases.
FEVC uses exact_limit(1400) to admit the 1,311-coefficient design: the initial
attempt stopped at its default dimension cap of 500. A dense 1,311-square
double matrix is about 13 MiB, well within the reserved 32 GiB. This resource
admission change does not alter the estimator, input, or numerical tolerances.

At two active cores, run each role once in exact mode and three times in JLA
mode with 280 projections and seeds 202609091/202609193/202609299. JLA runs share
the same outcome; they measure projection randomness, not sampling uncertainty.
Rotate role order between JLA repetitions. Twenty total calls, one scalar SCC
job reserving four slots at 8 GiB/slot for at most one hour. The allocation
reflects the previously observed MATLAB process-tree memory plus headroom.

Execution acceptance requires all role statuses, samples, seeds, probes,
normalizations, finite targets, memory records, source/input hashes and SGE
accounting to validate. Preserve the original numerical comparison tolerance
as a descriptive column; numerical disagreement does not stop this diagnostic.
Report exact values, JLA median/full range, plug-in and oracle corrections,
absolute gaps and gaps relative to correction size. Performance is secondary,
single-size and non-isolated; no scaling claim.
