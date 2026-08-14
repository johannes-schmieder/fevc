# Numerical architecture

## Sample pipeline

The ado layer freezes the requested variables, validates frequency and target
weights, expands factor-variable controls, and encodes string or numeric IDs
densely. Match deletion rejects incomplete requested rows because silently
changing one declared block would change the dependence contract.
Observation deletion uses a reproducible complete-case sample for outcomes
and regressors.

The graph stage follows the maintained MATLAB compatibility convention:

1. select the largest connected worker--firm component, ranked first by firm
   count and then by physical mass;
2. in match mode, restrict every headline target and the fitted model to
   workers observed at more than one fitted firm;
3. find worker articulation vertices with an iterative, nonrecursive Tarjan
   traversal;
4. remove those workers, select the largest resulting component, and repeat;
5. remove insufficient physical histories; and
6. encode the final quotient again.

The command reports every stage through sample counts and graph diagnostics.
The exact block gate or deterministic joint-control certificate remains
authoritative for deletion estimability when controls add rank restrictions
not visible in the graph.

## Dense exact backend

The exact backend builds all worker indicators, all but one firm indicator,
and the requested control columns. It equilibrates the information matrix,
checks its spectrum, factors it once, and applies Woodbury block identities.
It is deterministic and seed independent. `exact_limit()` and
`blocksize_limit()` prevent unregistered dense allocations.

## Matrix-free two-way solve

For the pure two-way part, the information matrix is

\[
H=\begin{pmatrix}
D'WD&D'WF\\
F'WD&F'WF
\end{pmatrix}.
\]

`D'WD` is diagonal. Eliminating worker coordinates produces the grounded
firm mobility Schur system

\[
S_F=F'WF-F'WD(D'WD)^{-1}D'WF.
\]

Mata applies `S_F` with grouped sums and solves it by diagonally
preconditioned conjugate gradients. It then reconstructs the worker
coordinates. Every accepted solve recomputes the residual under the original
full two-way operator. The reported solver residual is the maximum across
the fit, control preparation, leverage probes, and target probes.

The algebraic JLA derivation refers to the exact orthogonal FE projection.
Matrix-free inverse actions are numerical approximations to that projection,
and each is accepted only when its recomputed full-system relative residual
is at most `max(1e-11,10*tolerance())`. Solver error is reported separately
from probe MCSE. The implementation does not infer a forward-error or
uniform-theorem bound from a residual alone; dense overlap tests and SCC
qualification are numerical evidence at the registered tolerance.

The command profiles graph selection, fit, Schur-diagonal preconditioner
construction, leverage probes, target probes, and the combined correction.
Preconditioner construction is a measured subset of fit time. Exact mode
reports zero preconditioner time because it uses a dense inverse.

Joint controls use an exact low-dimensional FWL Schur complement. The
matrix-free service accepts scalar or multiple right-hand sides. JLA
directions are staged in batches of at most `batch()` columns; each PCG right
hand side retains its own convergence and full-residual gate.

Before a randomized joint-control deletion calculation is accepted, a
probe-independent sufficient rank certificate is applied. Controls are
whitened by their weighted within-worker--firm-cell scatter. For each deletion
unit, the trace of the whitened scatter it can remove bounds the largest
generalized eigenvalue of that removal. If the maximum trace loss is below
one by the registered numerical gap, every deleted control scatter remains
positive definite. Combined with the FE graph certificate, this rules out
deletion-induced loss of design rank without trusting an underestimated JLA
leverage. Designs that are valid but fail this conservative certificate must
use the exact backend or revise the control parameterization; they are not
randomly accepted.

## Improved JLA passes

The first randomized pass estimates FE projection and residual shares and
their raw fourth moments. Projection and residual estimates are divided by
their common sum so they remain complementary. The coefficient-one
finite-projection correction is then applied. Exact residualized-control
leverage is added separately under `nuisance(joint)`.

The second randomized pass factors each target centering matrix through
Rademacher directions. Worker, firm, and total contractions use common
directions. Covariance is computed probe by probe as

\[
(\text{total}-\text{worker}-\text{firm})/2,
\]

so both plug-in and corrected accounting identities hold at machine
precision.

The second pass consumes new directions that are disjoint from the first-pass
leverage directions. Conditional on the realized leverage sketch, the fitted
outcome, residual, exact control projection, and adjusted deleted residual are
fixed when the target-probe mean is formed. No additional stochastic
order-one multiplier from the same leverage probes enters that mean.

With a fixed seed, canonical row ordering and sequential direction generation
make the logical probe stream invariant to `batch()`. Physical frequency
copies are not expanded in production. Their Rademacher sums are generated
from the corresponding binomial distribution, which preserves the literal
expanded distribution and fourth moments.

`e(numerical_mcse)` is the Monte Carlo standard error of the second-pass
target-probe mean conditional on the realized leverage sketch. It excludes
leverage-sketch uncertainty, solver error, and econometric sampling
uncertainty.

## Memory and runtime boundary

The JLA path does not form an observation-by-parameter design, a parameter
inverse, or an observation-by-observation projection. Its leading storage is
linear in stored rows, workers, firms, deletion units, controls, and the
chosen batch size, plus a dense matrix for one deletion block. The current
preconditioner is the exact Schur diagonal. SCC scale evidence determines the
qualified range; performance evidence cannot relax tolerances, probes, sample
selection, or the estimator.
