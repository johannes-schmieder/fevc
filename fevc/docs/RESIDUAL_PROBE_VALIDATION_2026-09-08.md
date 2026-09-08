# Residual-probe candidate: bounded validation, 2026-09-08

Status: **FAIL — numerical-seed and saved-outcome screens**. Do not integrate
or promote the 512-Gram-probe candidate. Production source/binaries, point
estimation defaults, the manuscript and all previous evidence are unchanged.

The owner approved continuing the bounded checks proposed in the
[isolated candidate report](VENETO_RESIDUAL_PROBE_CANDIDATE_2026-09-08.md).
The prospective scope and screens are in
[`residual_probe_validation_v1.json`](residual_probe_validation_v1.json).
No outcome draws, model changes, ridge, threshold relaxations or alternative
fitters were introduced. The successful original Veneto replay remains a
successful computation, not a qualification result.

## Veneto numerical-seed check

Five additional inference seeds, 8675310–8675314, were run under both q0 and
q1. Together with the preserved original seed 8675309, this gives six seeds
per method. The point seed and 200 JLA probes remain fixed. Each inference
call uses 512 Gram probes, 1,000 covariance probes, and the existing spectrum,
critical-value and admission settings. Changing the inference seed changes
all those numerical domains; this is not a Gram-only variance attribution.

All ten new calls complete and return all four intervals. Points, corrections,
numerical point MCSEs and retained sample signatures are unchanged. All
conditioning and complete-system solve checks pass. The predictor Gram is
identical across the new seeds. Nevertheless, the registered stability screen
fails:

| Target | SE coefficient of variation | q0 width CV | q1 width CV |
| --- | ---: | ---: | ---: |
| Worker variance | 12.50% | 12.50% | 9.81% |
| Firm variance | 11.82% | 11.82% | 16.41% |
| Worker–firm covariance | 17.76% | 17.76% | 17.88% |
| Variance of the sum | 2.29% | 2.29% | 2.57% |

The predeclared CV ceiling was 10%. q1 firm and covariance endpoint ranges
also exceed 25% of median interval width, at 26.09% and 28.51%. The original
seed floors 72/12,828 predictions. The five new seeds floor respectively
131, 876, 33, 469 and zero predictions. The maximum share, 6.8288%, exceeds
the registered 5% screen. These are numerical-seed comparisons, not statements
about repeated-sample coverage or a universal acceptable floor percentage.

The candidate Gram reciprocal condition is at least 2.4134e-5 across the
six seeds. New-seed fit residuals range from 2.33e-13 to 6.39e-13. The solves
are accurate for their estimated matrices, while the fitted variance vectors
change materially with numerical probing. Positive definiteness is therefore
not enough. This identifies a precision concern but does not quantify how
much SE variability comes from Gram estimation versus downstream probes.

## Saved-outcome comparison

The comparison uses the existing byte-verified outcome generators and original
semantic outcome seeds from `public-development-1`. Both arms use the current
unified public-ABI adapter; only the candidate core uses the direct residual
sample covariance. The comparator is the current unified subtractive fitter,
not the older cross-fitted match smoother or a historical paper binary.

The complete paired tiny pipeline passes 28 native calls across all 14 selected
cells, along with malformed-CLI rejection and missing, duplicate and corrupted
record checks. Thirty-four existing harness tests pass. Pytest emits cleanup
warnings for unrelated old temporary directories; no test fails and those
directories were not manually modified.

The main comparison completes **3,680 calls and 14,720 target attempts**:
200 saved outcomes in each of eight primary cells and 40 in each of six
diagnostic cells, under both arms. Each task has a separate directory and
hash-bound receipt. All attempted calls are accounted for. There are zero
shared fit failures in either arm, and no saved-outcome seed, design, point,
point-MCSE or paired inferential-unit regressions. All four targets return in
every primary-cell call. Original q1 covariance exclusions remain excluded
from primary coverage claims, not from accounting.

The registered screens fail on five eligible target/cell combinations:

| Cell and target | Current coverage | Candidate coverage | Candidate SD/RMS-SE | Failed screens |
| --- | ---: | ---: | ---: | --- |
| Match q1, equal independent matches: firm | 97.0% | 89.5% | 1.151 | Coverage floor; paired drop |
| Observation q0, diffuse common: total | 93.5% | 93.0% | 1.119 | SD/RMS-SE ceiling |
| Observation q0, diffuse common with controls: firm | 92.0% | 88.0% | 1.229 | Coverage floor; SD/RMS-SE ceiling; paired drop |
| Observation q0, diffuse common with controls: total | 92.5% | 92.0% | 1.117 | SD/RMS-SE ceiling |
| Observation q1, dominant common: firm | 93.0% | 91.0% | 1.134 | SD/RMS-SE ceiling |

All table entries use 200 attempted and successful intervals, so coverage is
not conditioned on a selected subset of successes here. For the matched q1
firm case, the candidate loses coverage on 15 previously covered draws and
gains none; the paired difference is -7.5 percentage points with MCSE 1.87
points. For the observation-controls firm case it loses eight and gains none;
the difference is -4 points with MCSE 1.39 points. These are descriptive paired
Monte Carlo comparisons, not multiplicity-adjusted hypothesis tests or fresh
confirmation.

The registered nominal-based coverage floor is about 90.38% at 200 attempts.
The SD/RMS-SE ceilings are the original family limits: 1.10 for observation
and 1.18 for match. The maximum paired coverage drop is three points. The
candidate's match-q0 primary cells pass these screens; eligible coverage spans
92–96%. The match-q1 serial-dependence cell also passes, with eligible coverage
93.5–96%. Those successes do not override the failures elsewhere.

The original two-sided, mean-SE-based flags are separately retained. The
current comparator already flags observation-controls firm SE calibration;
the candidate flags that row plus diffuse-observation total, controls total
and dominant-observation firm SE calibration, as well as controls-firm
two-sided coverage. The match-q1 firm result fails the newly registered
nominal-based lower screen and paired comparison even though it is not flagged
by the original observed-coverage-MCSE two-sided rule. No rule was weakened or
relabelled after seeing results.

## Diagnostic cases and reporting safeguards

There are no shared failures in diagnostic cells either, but target-local
withholding remains. Among 40 candidate draws, weak-signal match q1 returns
firm/covariance intervals in 39/28 draws; null-signal observation q1 returns
firm/covariance intervals in 38/35 draws. The other diagnostic target counts
are complete. The comparator's weak-match counts are also 39/28; its null
observation worker/firm/covariance/total counts are 37/37/34/39. All missing
intervals remain in the accounting and all-attempt coverage denominators.

Known limitations remain visible. Candidate total-variance coverage is 60%
in the varying-controls fixed-offset match diagnostic, 67.5% in the severe
omitted-driver match diagnostic, and 80% in the severe omitted-driver observation
diagnostic (40 draws each). These are not newly qualifying regimes. All
112 arm/cell/target summaries and 28 fit-diagnostic summaries are preserved.

The first main reporter reused complete-campaign historical gate helpers that
require cells outside this deliberately bounded subset. It consequently
records six missing-cell reporting errors and `ACCOUNTING_FAIL`; that original
receipt remains intact. The main calls themselves all validate. A separate
raw-output audit checks all 3,680 calls and independently reconstructs the
registered screens for the selected cases. It distinguishes the registered
RMS-SE denominator from the legacy helper's mean-SE denominator and applies
the original observation-q1 covariance exclusion explicitly. Both the legacy
mean-SE summaries and the registered RMS-SE results are retained. The reporting
adapter issue neither causes nor removes the numerical/scientific failures.

## Evidence and commands

Evidence root: `.local/diagnostics/residual-probe-validation-20260908/`.
Source and input hashes are frozen before the main run and rechecked after it.
The candidate reuses the earlier isolated source without changing its arithmetic.
Builds use Rust 1.85.1 with `--release --locked --offline`. Generated adapters
are hash-checked copies of the earlier unified adapter, linked to the isolated
candidate libraries. The current-arm binaries retain their archived hashes.

The executed repository-Python scripts are `build_candidate.py`,
`run_seeds.py`, `replay.py tiny`, `replay.py main`, `audit_seeds.py`, and
`independent_audit.py` under that evidence root. Native replay tasks use four
concurrent single-threaded workers; Veneto uses two concurrent Stata-MP
processes with eight processors each. These are not performance benchmarks.
The registered input/source/build identities, exact CLI commands, every task
receipt, raw output and error log remain local. No cluster, AWS, fresh coverage
campaign, production rebuild or paper edit was performed.

SHA-256 identities:

- Registration: `db0280f074243c65f3c07f43c3fe7b76a9dc2990395fc3f5b21d2e40bc17e1f7`.
- Main manifest: `dcf5dbf46d904d18d5de75625412b6a892c4f3d965628fd2f7cf796ce48c4f2d`.
- Original main report: `99e7b63118764de89e1b5d8a673073183aca79652fc2e2ac176a4ecffecd8a26`.
- Independent raw-output audit: `aef96e06338ceb3de1b4e6450ba168c2ec6b2c3961194481ad93ec3aa9f867e6`.
- Seed audit: `9ccaee10e4ebb174fda0d2755f637ab1916bdd736187eeba58203d80dde7e1f6`.

## Interpretation and next owner decision

The direct residual identity is still correct at the population level. The
experiment shows that the current finite-probe implementation is not ready
for adoption: removing indefiniteness did not ensure a sufficiently accurate
matrix for variance fitting and subsequent inference. One fixed numerical
seed can affect all outcome draws in a design, so increasing outcome
replications alone would not cure that approximation error.

The next bounded candidate should improve numerical precision, not introduce
a second scientific fitter. A reasonable proposed test is **2,048 Gram probes**,
keeping the user-selected/default **200 point-estimation JLA probes** and all
other settings unchanged. Pair it with the existing 512-probe results on the
same six Veneto seeds and the same saved primary outcomes, and compare the
small-fixture Gram estimates to an independent exact matrix. This would test
whether reduced probing noise improves stability and the flagged calibration
rows; it is not a guarantee that it will. Fix the subset-reporting adapter
before starting that comparison. Do not select a favourable seed, clip
eigenvalues, add ridge, or revise the screens. If this bounded precision test
also fails, report the failure before another methodological change. This
report proposes that next step but does not execute it.
