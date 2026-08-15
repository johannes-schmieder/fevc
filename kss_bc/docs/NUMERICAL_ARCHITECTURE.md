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
It also accumulates the literal frequency total with an exact remaining-
capacity test and withholds `PHYSICAL_TOTAL_LIMIT` above `2^53`. Consequently,
all later nonnegative integer component and observation counts are exactly
represented in binary64.

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

For a match block with more stored rows than identified coefficients, API16
factors its projection as (UU'), checks the smaller (U'U) spectrum, and
applies

\[
(I-UU')^{-1}R=R+U(I-U'U)^{-1}U'R.
\]

The action is then substituted into the unchanged exact correction. The
implementation recomputes the complete observation-space residual for every
right-hand side and includes its maximum in the posted inverse residual. It
therefore avoids a large observation-block eigendecomposition without using a
weaker acceptance gate.

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

The command profiles graph selection, fit, Schur-diagonal setup, Schur
actions, preconditioner applications, PCG, leverage probes, target probes, and
the combined correction. Setup is a measured subset of fit time and is posted
as `e(setup_seconds)`; `e(preconditioner_seconds)` remains its compatibility
alias. Exact mode reports zero for iterative-solver fields. JLA posts per-RHS
stage/batch/iteration/complete-residual diagnostics plus RHS-equivalent action
counts and physical matrix-batch counts.

Joint controls use an exact low-dimensional FWL Schur complement. The
matrix-free service accepts scalar or multiple right-hand sides. Multiple
columns use independent scalar PCG recurrences in lockstep: a single matrix
Schur action serves all active columns at each iteration, inactive columns are
exactly zero, and every column retains its own curvature, stopping, iteration,
and full-residual gate. Every 100 iterations an explicit quotient residual is
recomputed. It replaces and restarts a recurrence only when it has converged
or measured drift is material at the registered tolerance. JLA directions are
staged in batches of at most `batch()` columns.

Before either controlled backend runs, rows are put in one control-coordinate-
and ID-free semantic order based on outcome and per-copy target mass. A tie
across nonexchangeable controls, model coordinates, or match blocks is
withheld. A weighted-orthonormal representation of the requested control span
is then reduced to a canonical row-anchor basis. The selected anchor rows
become the identity. The certified range is `k<=32` controls.

The implementation uses the dimensioned envelope stated in
`ESTIMATOR_CONTRACT.md`. It covers Gram accumulation, inverse and Cholesky
conditioning, the computed span, every score/cutoff decision, every later
anchor projector, and final anchor/span reconstruction. Maximum-column inverse
residuals are multiplied by `sqrt(k)` and divided by a positive reciprocal-
conditioning remainder. Measured projector and selected-row residuals enter
the envelope directly. The result is then propagated through the full design
and observation/match deletion margins; JLA additionally uses its deterministic
deleted-control rank gap. A propagated value above `1e-8`, a nonpositive
denominator, or an ambiguous pivot returns `AMBIGUOUS_CONTROL_BASIS`.
Adaptive per-column PCG therefore receives the same certified canonical
right-hand sides rather than two raw-coordinate-dependent systems.

After each backend has separately checked the plug-in and correction rows, it
forms their difference and applies another finite-value gate. This last gate
is necessary because subtracting two finite, opposite-signed doubles can
overflow even when both inputs passed their component checks.

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
withholds as `AMBIGUOUS_PROBE_ORDER`. Exact mode without controls remains
available; controlled exact uses the same semantic-order check and may return
`AMBIGUOUS_CONTROL_BASIS`. Because an
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
inverse, or an observation-by-observation projection. API16 represents each
match residual projection with at most one common direction plus the control
directions, solves the reduced Woodbury system, and checks the resulting
observation-space actions. Its leading storage is linear in stored rows,
physical copies, workers, firms, deletion units, controls, and the chosen
batch size, plus a small matrix quadratic in the number of control directions.
Physical-copy storage is used by observation leverage probes, match
leverage probes, and target probes under literal-frequency semantics.
`physical_limit()` supplies a typed pre-allocation boundary for every selected
JLA route before any of that state is allocated. The installed preconditioner
is the exact Schur diagonal. A shared CMG core is integrated only through a
forced test adapter and cannot be selected by the public command. Local
promotion gates retain diagonal as the sole route. SCC scale
evidence determines the qualified range; performance evidence cannot relax
tolerances, probes, sample selection, or the estimator.
