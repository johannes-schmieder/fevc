# Numerical architecture

## Optimization III scale representation

The compressed production modules compile with `matalnum off` and keep
`matastrict on`; explicit quad operations remain deliberate. Canonical
worker, firm, cell, deletion-unit, and target-stratum identities are exact
numeric dense ranks. No persistent string key or unverified hash enters the
RNG contract. The runtime stores the coefficient-cell payload once, plus
worker-major and firm-major order/panel vectors. A compact FE view dispatches
transpose, prediction, Schur, diagonal, and reconstruction operations to that
payload instead of building the generic cell design again.

Cancellation-sensitive preprocessing uses a deterministic tiled Neumaier
reducer with bounded `65,536 x active-RHS` scratch. Exact nonnegative integer
mass totals use native segmented sums only after checking their registered
sum is below `2^53`. Repeated FE operator traversal uses native segmented
aggregation and relies on the unchanged complete original-system residual as
the acceptance certificate. The ado reuses canonical graph maps through
pruning and deletion construction, then releases row mappings at the existing
disk-backed lifecycle boundary.

The explicit persistent-cell inventory is `8*(9*C + 3*W + 4*F)` bytes:
seven canonical payload columns, two cell-order vectors, and the worker/firm
panels and masses. Deletion units and target strata each use four numeric
columns (`32*G` and `32*S` bytes). Routed quotient work vectors add
`8*5*(W+F)` bytes to the external operator view. Phase scratch, hierarchy,
certificates, transition high-water, restoration, and raw Stata data remain
separate model families; a compressed-engine forecast is not an end-to-end
raw-input forecast.

The Optimization III scale ladder identifies a route discontinuity at four
cells per worker. At 625,000 workers, the three-cell case builds a 15,625-
vertex hybrid hierarchy in 35.478 seconds, while the four-cell case retains
640,625 hybrid vertices and spends 8,755.873 seconds in setup. The latter
still converges in 15 iterations with a passing complete residual, so the
dominant problem is hierarchy construction rather than repeated-RHS PCG.
The maintained MATLAB implementation completes the paired four-cell task in
84.951 seconds and converges, providing an independent algorithmic benchmark
for a future firm-Schur/degree-four CMG setup specialization. Optimization III
does not change routing semantics or introduce that new hierarchy algorithm;
its target model treats this measured discontinuity explicitly and the final
report records the resulting feasibility boundary.

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

The match graph stage uses the following deterministic fixed point:

1. select the largest connected worker--firm component, ranked first by firm
   count and then by physical mass;
2. in match mode, restrict every headline target and the fitted model to
   workers observed at more than one fitted firm;
3. find worker articulation vertices with an iterative, nonrecursive Tarjan
   traversal;
4. remove those workers and insufficient histories, select the largest
   resulting component, and repeat;
5. encode each distinct deletion ID as one multigraph edge, preserving
   parallel deletion IDs at a common worker--firm coordinate;
6. find deletion-unit bridges with an iterative Tarjan traversal, remove the
   complete bridge set simultaneously, and repeat all earlier stages; and
7. require a final zero-bridge certificate before encoding the quotient.

Observation deletion retains the API 17 graph path unchanged. Its deletion
unit is still one literal physical copy.

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

API 18 factors every match projection as \(UU'\). It checks and solves the
positive-definite observation-space maker \(I-UU'\) when stored rows are the
smaller dimension. Otherwise it checks the reduced maker \(I-U'U\) and applies

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
the combined correction. API 18 measures setup and fit with disjoint timers
and posts setup as `e(setup_seconds)`; `e(preconditioner_seconds)` remains its
compatibility alias. Exact mode reports zero for iterative-solver fields. JLA posts per-RHS
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

With a fixed seed, the command constructs probe atoms from the registered
runtime contract and a canonical order within the observed dense worker,
firm, deletion-unit, and target structure. The order also uses outcome,
per-copy target mass, controls, and an optional `probeorder()` tie-breaker.
It is invariant to harmless row sorting, regrouping the same physical copies,
batch partitioning, solver route, processor count, and scheduling. Tied rows
are valid and are not withheld merely because a pathwise labeling is not
unique. Arbitrarily relabeling observed IDs may produce another valid
randomized draw; it must not change the estimand, identification,
deterministic exact result, or numerical acceptance rules. Sequential
direction generation keeps the logical stream invariant to `batch()`.

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
inverse, or an observation-by-observation projection. API 18 represents each
match residual projection with at most one common direction plus the control
directions, solves the reduced Woodbury system, and checks the resulting
observation-space actions. Its leading storage is linear in stored rows,
physical copies, workers, firms, deletion units, controls, and the chosen
batch size, plus a small matrix quadratic in the number of control directions.
Physical-copy storage is used by observation leverage probes, match
leverage probes, and target probes under literal-frequency semantics.
`physical_limit()` supplies a typed pre-allocation boundary for every selected
JLA route before any of that state is allocated. The package installs the
exact Schur-diagonal and source-informed GPL CMG preconditioners behind one lockstep
solver contract. Routing is structural and finishes before estimator RNG.
Explicit diagonal uses B1. Explicit CMG builds the hierarchy and fails closed
if setup or execution fails. Automatic mode uses diagonal for structurally
small inputs or when CMG setup is unavailable; otherwise it uses the
successfully constructed hierarchy. It records the requested route, selected
route, reason, and any typed pre-RNG fallback cause. It does not run trial
right-hand sides or reject a route using projected work or wall time. Every
production solve still has to converge and pass the complete original-system
residual gate.

`memory_gib()` declares any positive direct allocation envelope. The maximum
simultaneous predicted allocation must fit that value before estimator RNG.
The additional 30-percent headroom calculation is advisory, as are wall-time
forecasts and `wallseconds()`. Concrete scheduler memory, wall, and temporary
disk limits remain hard for a submitted job.
`batch(auto)` resolves after deterministic sample construction and before
solver routing or random probes. It chooses the largest evidence-backed width
in 8, 16, 32, 64 that fits the probe count, processor heuristic, and a
conservative scratch heuristic. Those percentages select a practical width;
they are not separate rejection gates. Explicit positive integer batches,
including 128, retain the sequential direction stream when the complete
direct-peak forecast fits. Performance evidence cannot relax tolerances,
probes, sample selection, or the estimator.

The reusable CMG workspace remains an equality-tested API but is not the
production application path. At 32,768 hybrid vertices it was 34 percent
slower for four-column applications and 74 percent slower for sixteen-column
applications; earlier 100,000-vertex tests were about 63 percent slower. The
production adapter therefore uses ordinary batched CMG applications and
reuses the immutable hierarchy and terminal factors without reserving the
slower workspace matrices.
