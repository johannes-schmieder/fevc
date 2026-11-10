# Automatic standard errors for the variance components: proposal

Status: **proposal with prototype evidence, November 10, 2026; since
implemented in part.** The owner accepted the recommendation and decided that
standard errors should be posted as `e(V)` by default with stated caveats,
that the prototype be extended to controls and other deletion units, and that
`centering(corrected)` report the mean-centred standard error. The
[technical supplement](standard_errors/standard_errors.tex) states the method
as implemented, the extended evidence, and what remains before it can be on
by default; it supersedes sections 5 and 6 below. Nothing in this note changes
the released package, its registered contracts, or any
existing evidence. The numbers below come from a Python prototype in
[`../benchmarks/auto_inference_prototype/`](../benchmarks/auto_inference_prototype/README.md),
not from Stata, Mata or the Rust plugin. They are development evidence for a
design decision, not qualification.

## 1. Recommendation in brief

Report a sampling standard error for the four components on every supported
`fevc` call, computed by three changes to the existing structured
calculation:

1. **Compute the noise terms by pushing pseudo-noise through the kernel**
   (a Hutchinson trace) instead of taking the sample variance of 1,000
   quadratic forms. One draw is then as accurate as several hundred of the
   current ones, and the accuracy improves with sample size.
2. **Estimate the dominant "signal" term from squared regression residuals**
   (an HC2-type variance), and remove its noise contribution by simulation.
   The variance model then matters only for second-order terms, so a wrong
   variance model no longer translates one-for-one into a wrong standard error.
3. **Fit the working variance model by regressing the leave-out products the
   point estimator already forms on a few outcome-free features.** This needs
   no linear solves and no residual Gram matrix.

The result needs about **35 extra linear solves** (3 influence solves plus
8 draws at 4 solves each) on top of the roughly 600–800 solves of a default
point estimate. A default `inference(highrank)
inferencemodel(structured_common)` call currently needs about **11,900**
extra solves. The variance estimate is positive by construction, so it never
has to be withheld, and a failure in this step need not withhold the point
estimate.

In the prototype, across five error-variance designs at 4,413 rows, the
proposed standard error is within −4% to +6% of the sampling standard
deviation and 95% intervals cover 93.5–96.5%. The current variance fit, given
an exact Gram matrix (its best case), is 15% too small when an important
variance driver is omitted, with 90% coverage. At 56,486 rows with
random-projection leverages the proposed standard error is within 0–3% of the
truth in three designs and −5% to +11% in the omitted-driver design. At
358,809 rows two draws (11 extra solves) already fix it to within half a
percent.

What this does **not** deliver: it is not the unrestricted Kline–Saggio–Sølvsten
(KSS) variance estimator, it keeps the independence assumptions of the point
estimator, it still treats an estimated control offset as fixed, and its
normal intervals are unreliable when one direction dominates the target. The
last case is detected by a new diagnostic that comes free with the draws.

Sections 2–4 give the diagnosis, the theory and the evidence. Section 5 is the
implementation plan. Section 6 lists the decisions that are the owner's.

## 2. What has gone wrong with inference so far

All statements here are taken from the repository's own records.

**Cost.** For the generic engine with 200 probes a point estimate needs 601
solves, or about 801 with the default `mcse(all)`. A default structured
highrank request adds 2,048 Gram solves, 1,000 covariance probes, 640
trace-square solves, 8,192 power-iteration solves (512 iterations × 16), 3
influence solves and 18 final solves: about 11,900, some 15–20 times the
point estimate. The spectral pass runs even when only a normal interval is
requested. The largest recorded inference run has 240,000 rows
([assessment](POOLED_COMPONENT_ASSESSMENT.md)); there is no evidence at one
million rows or above, and the documentation forbids such claims.

**Availability.** The exact Mata observation smoother returned intervals in
11.4% and 18.8% of attempts on two highrank designs, because fitted variances
were negative or the covariance was not positive semidefinite
([assessment](MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)). Structured
inference withholds weak targets (90–91% availability for weak highrank firm
and covariance targets, 44% for the q1 null), and the covariance interval was
withheld in the 240,000-row CMG run. A failed inference request also withholds
the point estimate, which is why the paper advises saving a point-only run
first.

**Dependence on the variance model.** With a severe omitted variance driver,
coverage fell to 0.77–0.82 in the observation confirmation and 0.81 in the
match campaign ([record](MATRIX_FREE_COMPONENT_INFERENCE.md)). Comparing the
two built-in models cannot detect a driver omitted from both.

**Numerical fragility.** Raising the point-probe budget from 200 to 800 moved
an interval endpoint by up to 23% of the interval width. The 2,048-probe Gram
matrix made standard errors 5.9% and 9.3% too small relative to an exact Gram
matrix in two cells that failed calibration. The corrected observation-q1
confirmation still fails its standard-error gate (1.101 against 1.10).

**Scope of the evidence.** Every coverage experiment uses a small synthetic
design (for example a 12×12 mover grid, or 400 matches). None resembles the
data sizes the command is built for.

**Interface.** Inference is opt-in, needs up to ten explicit options, cannot
share a call with `project()`, and exists in two statistically different
families (the exact Mata smoother and the structured fit).

Two of these have a common root. The trace term is estimated in the least
efficient available way, and the variance model carries the whole standard
error. Sections 3.3 and 3.4 address each.

## 3. Theory

### 3.1 Setup

Rows $a=1,\dots,n$ are the independent units of the existing structured
route: one row per mover deletion unit after the fixed-offset collapse, one
row per physical stayer observation. Write $y=X\beta+\varepsilon$ with
independent mean-zero errors and $V=\operatorname{diag}(\sigma_a^2)$,
$S=X'X$, $P=XS^{-1}X'$, $M=I-P$, $B_t=XS^{-1}A_tS^{-1}X'$ and
$d_{ta}=B_{t,aa}/M_{aa}$. The leave-out estimator of
$\theta_t=\beta'A_t\beta$ is the quadratic form

$$
\hat\theta_t=y'K_ty,\qquad
K_t=H\,C_t\,H,\qquad
C_t=B_t-\tfrac12\{\operatorname{diag}(d_t)M+M\operatorname{diag}(d_t)\},
$$

where $H$ removes the retained mean ($H=I$ under `centering(none)`). $C_t$ has
a zero diagonal. Using $K_t$ rather than $C_t$ in everything below removes the
"fixed observed mean" approximation from the variance calculation at no cost:
the same centering is applied to pseudo-noise as to the outcome. This differs
from the current rule, which leaves the noise draws uncentered. The small
centering bias of the point estimator itself is unaffected.

With $\mu=X\beta$ and $m_t=K_t\mu$, and ignoring the $O(1/n)$ diagonal of
$K_t$,

$$
\operatorname{Cov}(\hat\theta_t,\hat\theta_s)
=\underbrace{4\sum_a\sigma_a^2m_{ta}m_{sa}}_{\text{signal term }S_{ts}}
+\underbrace{2\operatorname{tr}(K_tVK_sV)}_{\text{noise term }2\tau_{ts}} .
$$

This is KSS's variance formula (their Section 5.2). In the prototype designs
the noise term is 8–46% of the variance for strongly identified targets and
59–75% for a firm variance close to zero.

When leverages and target diagonals come from random projections, $K_t$ is
the kernel built from those estimates. Its diagonal is then no longer exactly
zero, and the pseudo-noise calculation below includes the resulting terms
without further work. The variance is that of the estimator actually
reported, conditional on the probes; the numerical MCSE stays a separate
quantity.

### 3.2 Why KSS split the sample, and what a fitted variance assumes

The natural plug-in replaces $m_t$ by $g_t=K_ty$ and $\sigma_a^2$ by the
leave-out product $s_a=z_a\hat e_{a,-a}$. It fails for two reasons: products
$s_as_b$ are biased for $\sigma_a^2\sigma_b^2$, and $s_a$ is correlated with
$g_{ta}^2$ through the other observations' errors. In the main prototype
design this naive estimator returns a negative variance for the total in
every sample and for the worker variance in 91–100% of samples.

KSS remove both problems with two independent predictions of $x_a'\beta$ that
do not use $y_a$. In the worker–firm model these are edge-disjoint paths in the
mobility network; they exist if the design keeps full rank when any two
observations are dropped, and the conservative fallback can be avoided if it
keeps full rank when any three are dropped. That is the demanding requirement.

The KSS Matlab package and `fevc` instead replace $\sigma_a^2$ by a smooth
fitted value $\tilde v_a$ that depends on any single outcome only at order
$1/n$. Then

$$
\hat\Omega^{\text{model}}_{ts}
=4\sum_a\tilde v_ag_{ta}g_{sa}-2\tau_{ts}(\tilde V)
$$

is unbiased if $\tilde v=\sigma^2$. If the model is wrong, the signal term is
off by $4\sum_a(\sigma_a^2-\tilde v_a)m_{ta}m_{sa}$: a first-order error
whenever the part of the variance the features miss is correlated with the
squared influence. Because the signal term is most of the variance, the
standard error inherits the model's error almost one for one.

### 3.3 Noise terms: push pseudo-noise through the kernel

Let $\zeta=\tilde V^{1/2}\xi$ with $\xi\sim N(0,I)$. The current code solves
once per draw, forms the scalar $q=\zeta'K_t\zeta$, and uses
$\operatorname{Var}(q)=2\operatorname{tr}(K_t\tilde VK_t\tilde V)$. A sample
variance from $R$ draws has relative standard error $\sqrt{2/(R-1)}$: 4.5%
at $R=1{,}000$, whatever the design.

Compute the vector $K_t\zeta$ instead. It costs three more solves per draw
(one per target), using exactly the construction that already produces
$g_t=K_ty$. Then

$$
\hat\tau_{ts}=\frac1R\sum_{r=1}^R\sum_a\tilde v_a\,(K_t\zeta^{(r)})_a(K_s\zeta^{(r)})_a,
\qquad \mathrm E\,\hat\tau_{ts}=\operatorname{tr}(K_t\tilde VK_s\tilde V).
$$

With $W_t=\tilde V^{1/2}K_t\tilde V^{1/2}$ and eigenvalues $\lambda_j$, one
draw has relative variance $2/r_{\text{eff}}$, where

$$
r_{\text{eff}}=\frac{(\sum_j\lambda_j^2)^2}{\sum_j\lambda_j^4}
$$

is the effective rank of the squared kernel. Per linear solve this is
$r_{\text{eff}}/4$ times more efficient than the current estimator. In the
prototype $r_{\text{eff}}$ is 200–1,200 at 4,413 rows, 2,000–15,000 at
56,486 rows and above 4,000 at 358,809 rows. It grows with the sample, so the
calculation gets relatively cheaper as data get larger.

The same draws give the **spectral diagnostic that now costs 8,850 solves**.
For two independent draws $r\neq r'$,
$\mathrm E[\{\sum_a\tilde v_a(K_t\zeta^{(r)})_a(K_t\zeta^{(r')})_a\}^2]
=\sum_j\lambda_j^4$, so $r_{\text{eff}}$ is estimated from the off-diagonal
entries of an $R\times R$ Gram matrix. It brackets the leading spectral share
that KSS's normal approximation requires to vanish:

$$
\frac1{r_{\text{eff}}}\ \le\ \frac{\lambda_1^2}{\sum_j\lambda_j^2}\ \le\ \frac1{\sqrt{r_{\text{eff}}}} .
$$

A large $r_{\text{eff}}$ certifies a diffuse spectrum without a power
iteration. These are eigenvalues of the variance-weighted kernel, which is the
object the central limit theorem for $\varepsilon'K_t\varepsilon$ concerns;
with bounded variances they are of the same order as KSS's design eigenvalues.

### 3.4 Signal term: squared residuals, with simulated noise calibration

Let $e=My$ and $\hat h_a=e_a^2/M_{aa}$, the HC2 variance estimate, with the
existing finite-probe maker inverse in place of $1/M_{aa}$. Because $e$
contains no signal,

$$
\mathrm E\Big[\sum_a\hat h_a\,g_{ta}g_{sa}\Big]
=\sum_a\bar\sigma_a^2\,m_{ta}m_{sa}+N_{ts}(V)+\text{third- and fourth-cumulant terms},
$$

$$
\bar\sigma_a^2=\sigma_a^2+\sum_{b\ne a}\frac{P_{ab}^2}{M_{aa}}(\sigma_b^2-\sigma_a^2),
\qquad
N_{ts}(V)=\mathrm E_{\zeta\sim N(0,V)}\Big[\sum_a\frac{(M\zeta)_a^2}{M_{aa}}(K_t\zeta)_a(K_s\zeta)_a\Big].
$$

$N_{ts}$ is the expectation of the same statistic under pure noise. It is
estimated by evaluating that statistic on the pseudo-noise draws already
made, since $M\zeta$ is a by-product of the first solve of each draw:

$$
\hat S^{\text{robust}}_{ts}
=4\Big[\sum_a\hat h_ag_{ta}g_{sa}
-\frac1R\sum_r\sum_a\frac{(M\zeta^{(r)})_a^2}{M_{aa}}(K_t\zeta^{(r)})_a(K_s\zeta^{(r)})_a\Big],
\qquad
\hat\Omega^{\text{robust}}=[\hat S^{\text{robust}}]_+ + 2\hat\tau .
$$

The principle is one rule applied twice: *compute a statistic on the data,
then subtract what the fitted noise alone would produce, by running noise
through the same code.* The model-based estimator of Section 3.2 is the same
rule with $\tilde v_a$ in place of $\hat h_a$.

Three properties follow.

- **The variance model enters only second-order terms** ($N$ and $\tau$). An
  error of 20% in those terms moves the standard error by about 1–3% when the
  noise share is 10–30%.
- **The signal term is exact under local homogeneity.** It targets
  $\bar\sigma_a^2$, a leverage-weighted average of a row's own variance
  (weight $M_{aa}$) and its neighbours' (the rows sharing its worker or firm).
  It equals $S_{ts}$ when error variances are equal within such
  neighbourhoods, or when their differences are uncorrelated with the squared
  influence. Heteroskedasticity across firms or across workers, whatever its
  cause, is therefore captured without being modelled.
- **In the low-leverage limit this is the textbook delta method with White's
  covariance:** $4\hat\beta'A_tS^{-1}(\sum_ax_ax_a'e_a^2)S^{-1}A_t\hat\beta$,
  using the leave-out influence and corrected for quadratic noise.

It is not assumption-free. With leverage near one half, squared residuals
smooth over neighbours, and an HC2 variance is not consistent under arbitrary
heteroskedasticity; that is the reason KSS use leave-out products and sample
splitting. Local homogeneity is the assumption that replaces the split sample.

Truncating the signal part at zero (for the matrix, projecting it on the
positive semidefinite cone) uses only the fact that $S\succeq0$. It gives
$\hat\Omega\succeq2\hat\tau\succ0$, so a variance can always be reported.

**Heavy tails.** Gaussian pseudo-noise omits a non-negative fourth-cumulant
term, $\kappa_4\sum_a M_{aa}^{-1}\sum_bK_{tab}K_{sab}M_{ab}^2\sigma_b^4$, which
is largest for stayer rows. The robust standard error is then conservative.
Matching the kurtosis of the pseudo-noise removes the excess on average but
makes the estimate noisier (Section 4.3), so the proposal keeps Gaussian
draws.

### 3.5 Working variance model without a Gram matrix

The current fit solves $Z'(M\circ M)Z\,\eta=Z'(e\circ e)$ and needs 2,048
solves to approximate the Gram matrix, whose sampling error biased standard
errors by 6–9% in two recorded cells. The leave-out product is already
unbiased row by row, $\mathrm E\,s_a=\sigma_a^2$, so ordinary least squares of
$s$ on $Z$ is unbiased for the same coefficients and needs only $Z'Z$:

$$
\hat\eta=(Z'Z)^{-1}Z's,\qquad \tilde v=\max(Z\hat\eta,\ \text{floor}).
$$

The normal equations give $\sum_aZ_a(\tilde v_a-s_a)=0$: fitted variances
reproduce the leave-out totals along every feature. $s_a$ is noisier than
$e_a^2$, which is why the squared-residual moment was preferred on designs
with a few hundred rows. With thousands of rows and a handful of
coefficients the noise is immaterial, and the estimator has no simulation
error at all.

The recommended default is deliberately small: for each unit type an
intercept, the leverage midrank and its square, and for mover units the
regression-mass midrank and its square (5 + 3 coefficients). The existing
21 + 15-term `structured_common` basis remains available. Because the model
now carries only second-order terms, this choice has little effect
(Section 4.1); a large basis on a small sample is slightly harmful because
noise in $\tilde v$ biases the products $\tilde v_a\tilde v_b$.

### 3.6 Assumptions

1. Independent units, as the point estimator already assumes: independence
   across mover deletion units and across physical stayer observations,
   unrestricted dependence within a mover unit.
2. Local variance homogeneity (Section 3.4) for the signal term.
3. A working variance function adequate for the noise terms, which are a
   minority of the variance unless the target is close to zero.
4. For the normal interval: a diffuse spectrum and limited individual
   influence (KSS Theorem 2). Both are reported:
   $r_{\text{eff}}$ and $\max_a\hat h_ag_{ta}^2/\sum_a\hat h_ag_{ta}^2$.
5. An estimated control offset is treated as fixed, as now.

## 4. Prototype evidence

Designs are synthetic worker–firm panels in the pooled match/stayer row
convention, pruned to the leave-one-match-out connected set, with sorting,
unequal firm sizes and spell lengths of one to six periods. "SE ratio" is the
root-mean-square standard error divided by the sampling standard deviation of
the estimator, taken from the Monte Carlo samples unless stated. Coverage is
for nominal 95% normal intervals. Targets are worker
variance, firm variance, covariance and their total, in that order.

Five error-variance designs are used. `homo` is homoskedastic. `mass` has a
match random effect, so the variance rises with spell length. `leverage` is
a smooth function of leverage. `firm_omit` makes the variance depend on the
squared firm effect, which no feature measures and which also drives the
firm-variance influence. `worker_omit` makes it depend on the worker's type.

### 4.1 Exact kernels, 4,413 rows, 1,000 samples per design

Logs: `results/dense_main.txt`, `results/dense_stayers.txt`,
`results/dense_limited.txt`.

Dense exact kernels, so the only approximation is the variance estimator.
1,804 workers, 415 firms, mean leverage 0.50, KSS correction 10–21% of the
component. "True variances" uses the true $\sigma_a^2$ in the formula and
shows the Monte Carlo noise in the ratios. "Current" is the residual-moment
fit with the full `structured_common` basis and an **exact** Gram matrix,
which is better than the production 2,048-probe approximation. The proposed
estimators use the small leverage-and-mass working model of Section 3.5 and
exact traces.

| Design | Estimator | SE ratio (worker, firm, cov, total) | Coverage |
|---|---|---|---|
| homo | true variances | 0.989, 0.973, 1.010, 0.982 | 0.939, 0.944, 0.953, 0.946 |
|  | current | 0.989, 0.974, 1.011, 0.984 | 0.941, 0.945, 0.950, 0.944 |
|  | proposed model | 0.987, 0.972, 1.009, 0.980 | 0.939, 0.946, 0.953, 0.947 |
|  | **proposed robust** | 0.985, 0.970, 1.005, 0.979 | 0.935, 0.941, 0.951, 0.940 |
| mass | true variances | 0.990, 0.976, 1.015, 0.989 | 0.942, 0.946, 0.955, 0.952 |
|  | current | 0.990, 0.976, 1.014, 0.989 | 0.945, 0.946, 0.954, 0.948 |
|  | proposed model | 0.988, 0.975, 1.013, 0.986 | 0.944, 0.944, 0.953, 0.950 |
|  | **proposed robust** | 0.983, 0.966, 0.989, 0.972 | 0.946, 0.947, 0.949, 0.947 |
| leverage | true variances | 0.992, 0.978, 1.019, 0.987 | 0.943, 0.944, 0.950, 0.954 |
|  | current | 0.993, 0.980, 1.020, 0.989 | 0.944, 0.945, 0.954, 0.948 |
|  | proposed model | 0.990, 0.977, 1.019, 0.985 | 0.943, 0.945, 0.952, 0.948 |
|  | **proposed robust** | 0.986, 0.968, 0.985, 0.965 | 0.942, 0.945, 0.944, 0.942 |
| firm_omit | true variances | 0.990, 0.992, 1.001, 0.992 | 0.948, 0.948, 0.950, 0.952 |
|  | current | 1.000, 0.862, 0.906, 0.851 | 0.953, 0.913, 0.919, 0.901 |
|  | proposed model | 0.990, 0.853, 0.898, 0.840 | 0.948, 0.908, 0.917, 0.900 |
|  | **proposed robust** | 1.056, 0.982, 0.975, 0.956 | 0.965, 0.944, 0.942, 0.940 |
| worker_omit | true variances | 1.005, 0.986, 1.005, 0.987 | 0.954, 0.949, 0.948, 0.945 |
|  | current | 0.958, 0.984, 1.006, 0.927 | 0.942, 0.948, 0.951, 0.929 |
|  | proposed model | 0.953, 0.980, 0.999, 0.918 | 0.942, 0.947, 0.948, 0.929 |
|  | **proposed robust** | 1.020, 0.998, 1.034, 0.995 | 0.954, 0.951, 0.954, 0.945 |

Readings:

- The proposed model fit (no solves) matches the current fit (2,048 solves
  in production) in every design.
- The robust estimator matches both when the model is right and repairs most
  of the shortfall when it is wrong.
- With a homoskedastic working model the robust estimator gives SE ratios of
  0.96–1.06 across the same five designs, which shows how little it leans on
  the model. That working model does fail for a weak target (ratio 0.91
  before truncation in Section 4.3), so the small leverage-and-mass model is
  the recommended default.

A second design in which 57% of rows are stayer observations (4,650 rows,
800 samples) gives robust SE ratios of 0.96–1.07 and coverage of
0.927–0.963 across `homo`, `mass`, `firm_omit` and `worker_omit`; the
model-based estimators give 0.83–0.87 and coverage of 0.90–0.91 for the firm
and total targets under `firm_omit`.

A third design has movers only and less mobility (4,148 rows, 638 firms, 6.5
mover matches per firm, mean leverage 0.59, KSS correction 13–30% of the
component, 800 samples). Robust SE ratios are 0.96–1.10 and coverage is
0.940–0.968 across the same four error designs, where the estimator using
the true variances gives 0.97–1.06. The model-based estimators give 0.87–0.88
for the total under `firm_omit` and 0.89–0.90 under `worker_omit`, with coverage
of 0.91–0.92.

### 4.2 Matrix-free pipeline with random-projection leverages

Logs: `results/sparse_mc_4k.txt`, `results/sparse_mc_56k.txt`,
`results/scale_359k.txt`, `results/sparse_draws.txt`.

Leverages and target diagonals from 200 Rademacher probes, 8 pseudo-noise
draws, 36 solves in the second pass (including the fit that the point
estimate already performs). The working model here is the larger
`structured_common` basis.

| Rows | Design | Model SE ratio | Robust SE ratio | Robust coverage |
|---|---|---|---|---|
| 4,413 | mass | 1.001, 1.000, 1.008, 0.997 | 0.990, 0.983, 0.973, 0.977 | 0.958, 0.930, 0.952, 0.958 |
| 4,413 | firm_omit | 1.007, 0.869, 0.913, 0.857 | 1.063, 0.982, 0.973, 0.957 | 0.976, 0.938, 0.954, 0.948 |
| 56,486 | homo | 1.000, 1.000, 1.005, 1.000 | 1.002, 1.002, 0.998, 1.001 | 0.925, 0.967, 0.925, 0.950 |
| 56,486 | mass | 1.000, 0.999, 1.004, 0.999 | 1.002, 0.996, 0.973, 0.992 | 0.942, 0.958, 0.917, 0.950 |
| 56,486 | firm_omit | 1.045, 0.944, 0.880, 0.817 | 1.108, 0.989, 1.010, 0.945 | 0.950, 0.967, 0.917, 0.950 |
| 56,486 | worker_omit | 0.983, 1.002, 0.965, 0.933 | 1.021, 1.007, 1.020, 1.014 | 0.942, 0.967, 0.958, 0.967 |

SE ratios in this table are measured against the analytic standard deviation
of the estimator, computed from its kernel and the true variances, so they
are precise even with few samples. The 4,413-row rows use 500 samples and the
56,486-row rows 120; coverage figures therefore have Monte Carlo standard
errors of about 0.01 and 0.02. At 56,486
rows the first pass took 4.5–5.0 seconds and the second 0.37–0.38 seconds, an
overhead of about 8%. All variances were positive after truncation; before
truncation the robust covariance-target variance was negative in 0.2–0.4% of
samples at 4,413 rows.

One larger design has 358,809 rows (139,005 workers, 9,575 firms). For a
single outcome per design and six pseudo-noise seeds, measured against the
analytic standard deviation:

| Design | Draws | Second-pass solves | Model SE ratio | Robust SE ratio | Largest CV of robust SE across seeds |
|---|---:|---:|---|---|---:|
| mass | 2 | 12 | 1.006, 0.998, 1.007, 1.000 | 1.006, 1.001, 0.973, 0.996 | 0.35% |
| mass | 8 | 36 | 1.006, 0.998, 1.007, 1.000 | 1.006, 1.001, 0.972, 0.996 | 0.07% |
| firm_omit | 2 | 12 | 1.044, 0.937, 0.848, 0.808 | 1.108, 0.989, 1.004, 0.957 | 0.28% |
| firm_omit | 8 | 36 | 1.044, 0.937, 0.848, 0.808 | 1.107, 0.989, 1.003, 0.957 | 0.04% |

At this size two draws already fix the standard error to a fraction of a
percent. The first pass took 26 seconds and the eight-draw second pass 2.3
seconds in the prototype.

**How many draws.** For one fixed outcome at 4,413 rows, the coefficient of
variation of the standard error across pseudo-noise seeds:

| Draws | Second-pass solves | Model SE | Robust SE |
|---:|---:|---|---|
| 2 | 12 | 0.1–0.9% | 1.5–5.0% |
| 8 | 36 | 0.1–0.4% | 0.8–3.5% |
| 16 | 68 | 0.05–0.3% | 0.5–1.7% |

The larger figure in each range is the covariance target. A single
four-solve draw estimates the trace with a coefficient of variation of
4–10%, against 19–21% for 50 solves of the current estimator. Eight draws
recover $r_{\text{eff}}$ as 1,184, 392 and 230 against exact values of 1,209,
346 and 198.

### 4.3 Stress cases (4,413 rows, 1,000 samples)

Logs: `results/dense_weak.txt`, `results/dense_very_weak.txt`,
`results/dense_skew.txt`, `results/dense_t5.txt`,
`results/bottleneck_2.txt`, `results/bottleneck_12.txt`.

| Case | Robust SE ratio | Robust coverage |
|---|---|---|
| Weak firm variance (SD 39% of the component), `mass` | 0.979, 0.981, 1.009, 0.970 | 0.942, 0.942, 0.953, 0.948 |
| Weak firm variance (SD 33% of the component), `firm_omit` | 1.055, 1.063, 1.005, 0.998 | 0.961, 0.958, 0.952, 0.951 |
| Very weak firm variance (SD 70% of the component), `mass` | 0.978, 0.983, 1.005, 0.971 | 0.941, 0.944, 0.951, 0.948 |
| Skewed errors (centred chi-square, 3 d.f.), `firm_omit` | 1.130, 0.974, 1.048, 1.005 | 0.971, 0.936, 0.955, 0.955 |
| Heavy tails ($t_5$), `mass` | 1.136, 1.031, 1.049, 1.115 | 0.962, 0.945, 0.952, 0.960 |
| Heavy tails ($t_5$), `firm_omit` | 1.273, 1.030, 1.087, 1.118 | 0.976, 0.949, 0.951, 0.953 |

Every variance was available. For the weak firm target, truncation raised
coverage from 0.92–0.93 to 0.94. Under $t_5$ errors the median
standard-error ratio is 0.99–1.14; the root-mean-square figures above are
inflated by the estimate's own heavy tail. Adding the exact kurtosis term
lowered the root-mean-square ratio to 1.00–1.20 but the median to 0.93–1.06
and coverage to 0.932–0.962, so it is not recommended.

**Concentrated spectrum.** Two firm clusters joined by 12 movers give exact
leading shares of 0.79–0.90. Normal intervals then under-cover for every
estimator, including the one using the true variances (0.89 for firm, 0.85
for covariance). Sixteen draws estimate $r_{\text{eff}}$ as 1.6, 1.3 and 1.2,
equal to the exact values; eight draws give 3.4, 1.9 and 1.8. Either flags
the design unambiguously, where the main designs give values in the hundreds
or thousands. With two bridging movers the estimate is 2.2–2.5 against an
exact 2.0, and the largest single-unit influence share is 0.31–0.58.

### 4.4 Limits of this evidence

The prototype is Python with a direct sparse factorisation; it does not
exercise the Rust solver, its finite-probe maker formula, frequency weights,
controls or the Stata interface. Designs are synthetic and Gaussian unless
stated. Nothing here was pre-registered. The largest Monte Carlo design has
56,486 rows and the largest single run 358,809.
The claims about cost follow from solve counts, which carry over; the claims
about calibration need the registered campaign in Section 5.4.

## 5. Implementation plan

### 5.1 Algorithm

After the existing point pass has produced the fit, residuals, leverages,
maker inverses and target diagonals:

1. Form $s_a$ (already `working_y * deleted_adjusted`), build the small
   outcome-free basis with the existing midrank code, solve the
   $p\times p$ least-squares system, floor.
2. Compute $g_t=K_ty$ with the existing three-column influence solve.
3. For each of $R$ pseudo-outcomes (default 8, one batch):
   solve once for the pseudo-fit; build three right-hand sides per draw with
   the same code that builds the influence right-hand sides, using the
   pseudo-coefficients and pseudo-outcome; solve; finish with
   `finish_influence`; centre as for the outcome.
4. Accumulate $\hat\tau$, the noise statistic, and within the batch the
   $R\times R$ Gram matrices for $r_{\text{eff}}$.
5. Assemble both covariance matrices, truncate the signal part, map three
   targets to four.

Extra solves: $3+4R$. Extra memory: the vectors of one batch, which the
solver already holds for multi-column solves. No observation-by-probe matrix
is retained.

### 5.2 Rust

- `generic_jla.rs`: a new attachment policy beside the residual-moment one.
  It reuses `target_ratios`, `primitive_target_rhs`,
  `component_transpose_rhs`, `finish_influence`,
  `fill_gaussian_pseudo_outcome` and `solver.solve_batch_with_interrupt`. The
  covariance loop gains a second batched solve per batch and new
  accumulators. `run_component_spectrum` is skipped on this policy.
- `structured_variance.rs`: an ordinary least-squares fit of the leave-out
  product on the reduced basis. The file already regresses this response in
  its legacy cross-fitted ridge path, so only the fold and penalty logic is
  bypassed.
- A new Counter-V1 domain for these draws, so existing addresses and results
  do not move.
- An additive result block: both $3\times3$ matrices, $\hat\tau$,
  $r_{\text{eff}}$, influence shares, draw count, floor share and solve
  receipts, with a new capability bit.
- Dense-oracle unit tests of the three expectations in Sections 3.3–3.4, of
  the truncation, and of invariance to row order and batch width.

### 5.3 Stata and Mata

- A separate request, for example `se(robust|model|off)` and `sedraws(#)`,
  so that `inference(highrank|q1)` and `inferencemodel()` keep their meaning
  and their registered behaviour.
- Results stay non-atomic: if this step fails, post the point estimate and a
  typed status, with missing standard errors.
- Returns: standard errors and intervals in `e(results)`, the two covariance
  matrices, `e(se_reff)`, `e(se_influence)`, `e(se_draws)`, `e(se_status)`.
  The numerical MCSE stays a separate row; a combined
  $\sqrt{\text{SE}^2+\text{MCSE}^2}$ could be shown as a third line but must
  not replace either.
- A warning when $r_{\text{eff}}$ is small or the influence share is large,
  pointing to `inference(q1)`.
- Exact Mata backend: the same formulas with exact inverse actions. The Mata
  JLA engine needs the influence and pseudo-outcome solves added; it has the
  solver and the diagonals already.

### 5.4 Validation before any default changes

1. Dense oracles and a Rust/Mata/prototype three-way comparison on small
   fixtures.
2. A registered campaign that includes realistic sizes ($10^5$–$10^6$ rows),
   frequency weights, estimated control offsets, the five variance designs
   above, heavy tails and concentrated spectra, with the true variance
   computed analytically from the kernel so that few replications suffice.
3. The public Veneto illustration and the private Veneto panels, compared
   with the existing structured results.
4. Timing and memory at the benchmark sizes.

### 5.5 Suggested order

1. Add the route as an explicit opt-in on the tuples that structured
   inference already supports, and run the comparison.
2. Make it non-atomic and default-on for those tuples.
3. Extend to the default joint-control route and the Mata engines.
4. Revisit q1 with the same draws.

## 6. Decisions for the owner

1. **Default behaviour.** The package contract says point estimation is the
   default and no sampling covariance is posted without an explicit request.
   Automatic standard errors change that. Options: on by default with
   `se(off)`; or opt-in with a single option.
2. **Whether to post `e(V)`.** Posting it lets `lincom` and `test` work but
   changes what a default call returns.
3. **Headline estimator.** Recommended: robust as the reported standard
   error, model-based stored beside it. A gap between them is itself a
   diagnostic of variance-model misspecification.
4. **Controls under the default `nuisance(joint)`.** With controls and match
   deletion the kernel is not the collapsed scalar one. Options: report the
   fixed-offset standard error with a flag; or report nothing until a
   joint-control version exists.
5. **The older routes.** Whether to keep the exact Mata observation smoother,
   given its 11–19% availability.
6. **Stayer serial correlation.** Both the point correction and any standard
   error here assume independent stayer observations. A worker-clustered
   signal term is a natural extension but is not part of this proposal.

## 7. What the paper could then say

That `fevc` reports standard errors for the components by default; that they
combine KSS's variance formula with a residual-based estimate of its leading
term and a simulated correction for its noise terms; that they assume local
variance homogeneity in place of KSS's split-sample predictions and are not
robust to arbitrary heteroskedasticity; what they cost; and what the
diagnostics mean. References to check before citing: Hutchinson (1990) for
the trace estimator, MacKinnon and White (1985) for HC2, and Cattaneo,
Jansson and Newey (2018) on why residual-based variances fail with many
regressors.
