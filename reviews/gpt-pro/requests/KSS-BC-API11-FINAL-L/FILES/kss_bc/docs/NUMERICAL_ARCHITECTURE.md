# Numerical architecture

## Sample pipeline

The ado layer freezes the requested variables, validates frequency and target
weights, expands factor-variable controls, and encodes string or numeric IDs
densely. Match deletion rejects incomplete requested rows because silently
changing one declared block would change the dependence contract.
Observation deletion uses a reproducible complete-case sample for outcomes
and regressors. Only factor-variable terms carrying Stata's explicit omitted
metadata are removed during expansion. An ordinary numeric column remains in
the requested design even when it is identically zero or collinear, so the
backend rank gates withhold the singular model rather than silently repairing
it. Before dispatch, the caller checks the API, version, and
semantic build token of any loaded Mata runtime. A stale same-level runtime is
withheld rather than silently used.

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

If two components tie on both registered ranking quantities at any selection
step, there is no ID-relabeling-invariant winner. The command withholds with
`AMBIGUOUS_LARGEST_COMPONENT` instead of using encoded identifiers as a
tie-breaker.

The command reports every stage through sample counts and graph diagnostics.
The exact block gate or deterministic joint-control certificate remains
authoritative for deletion estimability when controls add rank restrictions
not visible in the graph.

## Dense exact backend

The exact backend builds all worker indicators, all but one firm indicator,
and the requested control columns. It equilibrates the information matrix,
checks its spectrum, factors it once, and applies Woodbury block identities.
It relies on weighted information rank and deleted-information gates, not a
stored-row-count surrogate; literal copies can supply residual degrees of
freedom even when the stored-row count equals the parameter count.
It is deterministic and seed independent. `exact_limit()` and
`blocksize_limit()` prevent unregistered dense allocations. The backend
compares its inverse residual with reciprocal conditioning to obtain a
forward-error proxy. Whenever a Woodbury residual-maker eigenvalue lies
inside ten times that proxy, it also forms and factors the deleted information
matrix directly. A failed direct factor withholds the entire calculation.
The term exact distinguishes deterministic algebra from JLA; it does not mean
exact arithmetic.

## Matrix-free two-way solve

For the pure two-way part, the information matrix is

\[
H=\begin{pmatrix}
D'WD&D'WF\\
F'WD&F'WF
\end{pmatrix}.
\]

`D'WD` is diagonal. Eliminating worker coordinates produces the singular
firm-mobility Laplacian

\[
S_F=F'WF-F'WD(D'WD)^{-1}D'WF.
\]

Mata applies `S_F` with grouped sums and solves it by diagonally
preconditioned conjugate gradients on the zero-sum quotient. The right-hand
side includes all firm equations, every iterate and preconditioned residual is
projected onto the quotient, and no encoded firm is privileged during the
iterative path. Only after convergence does the implementation subtract the
last firm's coefficient to return the public all-worker/all-but-one-firm
coordinate representation. It then reconstructs the worker coordinates.
Every accepted solve recomputes every worker and every firm normal-equation
residual under the original full two-way operator. The reported solver
residual is the maximum across
the fit, control preparation, leverage probes, target probes, and every
right-hand-side column within a batch. No large or well-scaled neighboring
column can mask a failed column through a single Frobenius ratio. In match
mode the maximum also includes every small residual-maker inverse residual.

The algebraic JLA derivation refers to the exact orthogonal FE projection.
Matrix-free inverse actions are numerical approximations to that projection,
and each nonzero right-hand side is accepted only when its scale-relative
recomputed full-system residual is at most
`max(1e-11,10*tolerance())`. Solver error is reported separately
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

Before those solves, a weighted-orthonormal representation of the requested
control span is reduced to a canonical row-anchor basis. Anchor selection uses
only invariant row inner products and the already fixed conceptual-copy order;
the selected anchor rows become the identity. If `Z` is replaced by `ZT` for
invertible `T`, both inputs therefore produce the same canonical controls up
to the registered whitening and anchor residual gates. Adaptive per-column
PCG then sees the same right-hand sides instead of two coordinate-dependent
sets. A control span that cannot be canonicalized at the registered rank
margin is conservatively withheld.

Before any randomized calculation with controls is accepted, including
`nuisance(fixedoffset)`, a probe-independent sufficient rank certificate is
applied to the full joint fit and every declared deletion. Controls are
explicitly centered within worker--firm cells in a two-pass calculation and
then whitened by their weighted within-cell scatter. For each deletion
unit, the trace of the whitened scatter it can remove bounds the largest
generalized eigenvalue of that removal. If the maximum trace loss is below
one after subtracting both the measured whitening residual and a rounding
margin, every deleted control scatter remains positive definite at the
registered numerical margin. Each small deleted whitened scatter is also
formed and factored directly; its minimum eigenvalue and inverse residual are
independent acceptance gates. Combined with the FE graph certificate, this rules out
full-sample or deletion-induced loss of design rank without treating
approximate FE-solve error as genuine control variation or trusting an
underestimated JLA leverage. Designs that are valid but fail this conservative
certificate must use the exact backend or revise the control parameterization;
they are not randomly accepted.

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

With a fixed seed, the command orders conceptual physical copies by outcome
and per-copy target mass. It does not use encoded worker, firm, match,
frequency, stored-row labels, or raw control coordinates to assign signs.
Ties on the primary key are exchangeable only when the expanded controls also
agree within one worker--firm coordinate and, for match deletion, one deletion
block. If a tie spans differing controls or nonexchangeable coordinates, JLA
withholds as `AMBIGUOUS_PROBE_ORDER`; exact mode remains available. Because an
invertible control transformation preserves row equality, this fail-closed
rule makes the random sign stream pathwise invariant to control-basis
reparameterization, ID relabeling, harmless row sorting, and regrouping the
same physical copies. The quotient solve is permutation equivariant; accepted
numerical outputs under such transformations must agree within the registered
solver and roundoff tolerances, including when a deliberately loose allowed
PCG tolerance changes the stopping iteration. Sequential direction generation
also makes the logical stream invariant to `batch()`.

The regression design remains collapsed. Observation JLA retains two
cross-probe correlation sums per physical copy, forms every copy's nonlinear
finite-projection multiplier, and averages only the final multipliers within a
stored row. Match and target passes aggregate the same literal copy signs
before the solve. This is draw-for-draw equivalent to the canonical explicitly
expanded calculation.

`e(numerical_mcse)` is the Monte Carlo standard error of the second-pass
target-probe mean conditional on the realized leverage sketch. It excludes
leverage-sketch uncertainty, solver error, and econometric sampling
uncertainty.

## Memory and runtime boundary

The JLA path does not form an observation-by-parameter design, a parameter
inverse, or an observation-by-observation projection. Its leading storage is
linear in stored rows, physical copies, workers, firms, deletion units,
controls, and the chosen batch size, plus a dense matrix for one deletion
block. Physical-copy storage is required only for literal observation-JLA
semantics. `physical_limit()` supplies a typed pre-allocation boundary for
that state. The current preconditioner is the exact Schur diagonal. SCC scale
evidence determines the qualified range; performance evidence cannot relax
tolerances, probes, sample selection, or the estimator.
