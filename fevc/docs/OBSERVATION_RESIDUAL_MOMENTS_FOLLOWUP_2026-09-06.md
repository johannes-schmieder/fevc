# Observation residual moments: JLA inputs, paired deletion and confirmation

## Outcome

All three owner-requested stages are complete. The residual-moment fitter
with actual JLA diagnostic inputs passes the predeclared independent
scientific confirmation. Its exact-input counterpart fails one gate:
firm-component coverage with joint controls is **96.60%**, above the
**96.50%** upper limit. That failure remains a failure. This is a mixed
internal result, not public-command qualification or permission to promote an
observation-inference option.

The original observation shortfall is no longer reproduced in the principal
weak-graph cases. The same-design comparison also shows why earlier success
with match deletion was not evidence that observation deletion had a wrong
formula: the graph, number of independent units and variance-model fit matter.
The public fixed-offset match interface and all previous confirmation receipts
are unchanged.

## What was held fixed

The new research harness is under
`rust/experiments/residual_moment_followup/`. It mechanically combines a
hash-checked, unedited copy of the independent observation oracle with a
separate adapter. The production fitter, point correction, q1 recenter,
interval formulas and solver were not changed in this follow-up.

The candidate still fits residual squares using

\[
\widehat\gamma=\widehat K^{-1}Z'e^2,\qquad
\widehat K=Z'\operatorname{diag}(1-2h)Z+
\tfrac12\widehat{\operatorname{Cov}}\{Z'(Pg)^2\}.
\]

There are 512 independent numerical projection probes, a 3-term leverage or
15-term common basis, no ridge or column dropping, and the previously
registered positivity floor. Exact moment identities do not make the
finite-probe inverse or floored predictions exactly unbiased.

Point kernels, covariance traces and q1 reference quadrature are exact
research-oracle calculations. JLA estimates only the leverage and target
diagonals used as variance-fit inputs here. Thus the experiment isolates
variance learning; it does not incorporate all sources of noise or failure in
the full native inference command.

JLA geometry is generated in an outcome-free prepass and frozen across
outcome draws. This distinction matters: current generic-JLA row addressing
can depend on outcome ordering. The candidate has not yet been attached to an
outcome-free public-command lifecycle. Approximate-leverage arms deliberately
stress the internal API beyond its documented exact-leverage contract; this
experiment does not silently expand that contract.

## Actual JLA inputs and numerical sensitivity

The completed development grid covers six correctly specified cases, 300
outcome draws per case, three numerical seeds and 200, 800 or 3,200 JLA
probes. It separates exact inputs, noisy leverage alone, noisy features alone,
and both noisy inputs. All arms and numerical budgets share the same outcome
draws; these are paired comparisons, not independent samples to pool.
There are 54 configurations, 216 successful preparations and 259,200 target
attempts. All final geometry captures and small-system preparations succeed.

Across fixtures, relative leverage error falls from approximately 8.5--10.5%
at 200 probes to 2.2--2.8% at 3,200. In the weak-graph leverage case, changing
the ranks used as variance features matters more than replacing leverage in
the moment correction alone. This is a useful warning against claiming that
any sketch budget is adequate. The covariance-target diagonal can be small
enough that its relative sketch error remains large even at 3,200 probes.

Two diagnostic-capture issues were retained and resolved before generating
confirmation outcomes:

- With only eight spectrum probes and 128 iterations, the reduced diagnostic
  attachment failed on nearly tied diffuse modes. Raising its fixed iteration
  count to 1,024 retained the same 0.002 certificate tolerance.
- Its artificial oracle variance of `1e-8` then put the diffuse primitive
  covariance near the dense eigencertificate's absolute roundoff envelope.
  Multiplying only the deterministic prepass response by 10,000 resolved
  capture. A paired check gives bitwise-identical leverage and target
  diagonals before and after this positive rescaling. Actual simulation
  outcomes are not rescaled.

Both initial 54-task grids and the superseded outcome-free preflight are
preserved. They are harness diagnostics, not adverse confirmation draws that
were removed. The protocol and its two pre-confirmation addenda record the
changes; no scientific acceptance threshold changed.

## Fresh independent confirmation

The final manifest was frozen before any confirmation outcomes. It binds an
immutable source bundle, executable, all 20 original cell/dimension entries,
geometry inputs, semantic Counter-V1 outcome keys, 2,500 replications per cell,
both arms and all four targets. The new outcome and numerical seeds differ
from development and the earlier failed confirmation. The 3,200-probe JLA
budget was selected prospectively, not by selecting a favorable result.

Manifest SHA-256:
`e0389f2ebc76f7973f1d00dd284ba628a0c47074b730371d35530571b8b06062`.
Source-bundle SHA-256:
`a258a96aed4ec2b1f82319aa597bf1857ce0c7d16022ccd79cc0e46f9cd4eae6`.

All **400,000** target attempts were audited. Each arm has 39 correctly
specified primary target cells. The original bias, coverage, standard-error
ratio, availability, spectral and misspecification gates are unchanged.
The q1 covariance target retains its original multi-mode coverage exclusion;
its availability is still checked. Null cells remain diagnostic.

Selected firm-component results, each based on 2,500 fresh draws:

| Case | Exact-input coverage | JLA-input coverage | JLA empirical SD / mean SE |
|---|---:|---:|---:|
| Weak graph, leverage model | 95.88% | 95.48% | 1.027 |
| Weak graph, common model | 95.72% | 95.96% | 1.008 |
| Weak graph, Student-t8 errors | 95.32% | 95.72% | 1.015 |
| Weak graph, two joint controls | **96.60% — FAIL** | 96.36% | 0.988 |
| Diffuse graph, common model, k=16 | 95.48% | 95.56% | 0.995 |

The exact-input arm's only gate failure is the marked coverage cell; its
standard-error ratio is 0.994. This is modest overcoverage, not a recurrence
of the original downward variance bias. It must nevertheless be explained or
bounded before making a broader claim. The JLA arm passes all registered
gates. Passing the JLA arm does not erase the separately declared exact-arm
failure, and one successful fresh numerical stream is not a theorem about
every stream or dataset.

There are 191,634 successful target intervals in the exact arm and 191,699
in the JLA arm. The remaining attempts are retained in the full failure
inventory, principally the deliberately difficult null cases. For the
dominant null firm target, q1 availability is 51.32% and 50.96%, respectively;
counting unavailable intervals as uncovered gives 50.20% and 49.84% coverage.
The new fitter therefore does **not** solve null/weak-signal availability.
Severe omitted variance drivers visibly expose misspecification in both arms.

## Same physical data, observation versus match deletion

The paired experiment uses 500 draws for each of six cell/size combinations,
five arms and four targets: 60,000 target attempts. It keeps the original
physical observations, outcomes, target population and true error variances
fixed. There are no controls and physical errors are independent in this
comparison. For a match with F observations it uses

\[
Y_g=\sum_{i\in g}Y_i/\sqrt F,\quad
X_g=\sqrt F X_i,\quad
\sigma_g^2=\sum_{i\in g}\sigma_i^2/F.
\]

An independent tiny physical-block deletion oracle verifies the collapsed
quadratic form, zero within-match blocks, point estimate and covariance trace.
The observation- and match-deletion corrected estimates need not coincide:
they delete different units. Their common target population is preserved.

For the leverage model on the weak graph:

| Physical observations / matches | Old observation | New observation | Old match | Oracle match |
|---|---:|---:|---:|---:|
| 536 / 144 | 93.6% | 95.4% | Unavailable | 96.8% |
| 1,195 / 312 | 95.4% | 96.4% | 94.0% | 95.8% |

These are descriptive firm-coverage comparisons, not a separately powered
match confirmation. At 144 matches the existing grouped routine fails its
nested-fold validity gate; it fits the common model as well as the selected
leverage model. At 312 matches it runs, but the old match SE ratio is 1.107,
compared with 1.013 for the new observation fitter and 1.014 for oracle match
variances. Match deletion is not automatically a cure for variance learning.

There is a further qualification: an observation-rank variance model does
not automatically remain correctly specified after collapse. The true
aggregate variances have relative projection errors of about 1.9--3.2% in
the tested common bases and 14.5--16.5% in the weak-graph leverage bases.
Consequently the poorer old-match result is not an isolated test of shrinkage
under a correctly specified aggregate model. This comparison explains why the
previous different-design match success cannot establish that an observation
formula is wrong; it does not invalidate the accepted match confirmation.

## Local scaling

The fitting benchmark uses the real full-firm zero-sum diagonal solver for
every projection probe, with complete original-system residual certificates.
It does not build an observation-by-observation projection matrix in that
path. The synthetic oracle fixture is constructed separately.

At 32,899 observations and 128 workers/firms, 512-probe preparation takes
1.60 seconds without controls and 1.66 seconds with two joint controls.
A subsequent variance fit takes approximately 1.3--1.4 milliseconds. All ten
fits per size/control configuration succeed. The admitted additional memory
envelope is about 14.1 MB, plus 4.7 MB of borrowed variance inputs; this is
not whole-process RSS and excludes retained solver/oracle state.

The complete diagnostic JLA prepass is more expensive: approximately
28.1--32.8 seconds at this size, using 800 JLA probes and the deliberately
long spectrum-capture budget. Neither timing is an end-to-end q0/q1 public
command benchmark. The result supports practical variance fitting at tens
of thousands of rows, not a demonstrated million-row inference claim.

## Checks, evidence and next boundary

The final harness passes 14 adversarial/pipeline tests and eight independent
oracle tests. The source gates pass: 726 Python tests, 481 Rust workspace
tests, CMG assembly checks and `git diff --check`. Tests cover malformed,
partial, duplicate, nonfinite, wrong-seed and scientifically failing outputs,
deliberate process failure, exact task inventory, and reverse/sharded draws.
Final revalidation checks every receipt/input/source hash and paired point
estimates. Raw outputs, sources, manifests, logs and a local archive identity
are recorded in `observation_residual_moments_followup_v1_result.json`.

No package release, commit, push, tag, Stata option or distributed package
binary was changed.
No external Windows/SCC job was needed or started. New public/native
qualification was not run: this turn changed the research harness and
documentation, not production execution.

The next bounded step is an **internal end-to-end integration**, not another
variance-model search: establish an outcome-free geometry lifecycle, attach
the fitter to that lifecycle, and test actual JLA point/covariance noise and
joint PSD failures with public-style q0/q1 calls. Preserve the exact-input
overcoverage failure as a targeted diagnostic and keep null/multi-mode cases
outside any new claim. Public observation promotion requires its own frozen
integration qualification and owner decision. The current match release
candidate need not wait for that work.
