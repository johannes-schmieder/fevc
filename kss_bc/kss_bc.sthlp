{smcl}
{* *! version 0.2.0-dev API 19 16aug2026}{...}
{title:Title}

{phang}
{bf:kss_bc} {hline 2} KSS leave-out bias-corrected point estimates for a
linear worker--firm model

{title:Syntax}

{p 8 16 2}
{cmd:kss_bc} {it:depvar} [{it:controls}] [{cmd:[fw=}{it:frequency}{cmd:]}]
[{it:if}] [{it:in}],
{cmd:worker(}{it:varname}{cmd:)} {cmd:firm(}{it:varname}{cmd:)}
[{cmd:deletion(match|observation)} {cmd:deletionid(}{it:varname}{cmd:)}
{cmd:algorithm(auto|exact|jla)} {cmd:nuisance(joint|fixedoffset)}
{cmd:targetweight(}{it:varname}{cmd:)} {cmd:stayers(movers|both)}
{cmd:probeorder(}{it:varname}{cmd:)}
{cmd:probes(}{it:#}{cmd:)} {cmd:batch(auto|}{it:#}{cmd:)}
{cmd:engine(auto|compressed|generic)}
{cmd:preconditioner(auto|diagonal|cmg)}
{cmd:memory_gib(}{it:#}{cmd:)}
{cmd:wallseconds(}{it:#}{cmd:)}
{cmd:seed(}{it:#}{cmd:)} {cmd:tolerance(}{it:#}{cmd:)}
{cmd:maxiter(}{it:#}{cmd:)} {cmd:exact_limit(}{it:#}{cmd:)}
{cmd:rank_tolerance(}{it:#}{cmd:)} {cmd:block_tolerance(}{it:#}{cmd:)}
{cmd:blocksize_limit(}{it:#}{cmd:)} {cmd:physical_limit(}{it:#}{cmd:)}
{cmd:nodisplay}]

{title:Options}

{phang}
{cmd:worker()} and {cmd:firm()} identify the two fixed-effect dimensions.
Numeric and string identifiers are accepted.

{phang}
{cmd:deletion(match)} is the default and removes the complete declared match.
{cmd:deletion(observation)} removes one physical observation.  With match
deletion, {cmd:deletionid()} may distinguish actual matches sharing one fitted
worker--firm coordinate.  A supplied match ID cannot cross coordinates.

{phang}
{cmd:algorithm(auto)} selects exact calculation when the identified dimension
does not exceed {cmd:exact_limit()}, and JLA otherwise.  {cmd:algorithm(exact)}
is deterministic.  {cmd:algorithm(jla)} uses reproducible randomized inverse
actions.

{phang}
{cmd:nuisance(joint)} is the default.  Controls move under deletion and enter
the information inverse.  {cmd:nuisance(fixedoffset)} conditions on their
full-sample fitted index.

{phang}
{cmd:probes()}, {cmd:batch()}, and {cmd:seed()} control the JLA stream.
{cmd:batch(auto)} is the default and deterministically selects among 8, 16,
32, and 64 after sample construction. The selection is bounded by the
retained rows, parameter count, probe count, active processors, and 35 percent
of {cmd:memory_gib()}; its processor cap is 32 through four processors and 64
with eight or more, and samples below 10,000 retained rows use batch 8.
Positive integer batches, including 128 when its memory forecast fits, are
also accepted.
{cmd:probeorder()} supplies a complete, unique physical-observation key only
when discrete outcomes and per-copy target mass leave otherwise
nonexchangeable rows tied. It is never inferred from worker, firm, match, or
stored-row order. Existing calls retain the original stream. The explicit key
becomes part of the registered fixed-seed semantics and is stored in
{cmd:e(probe_order)}.
{cmd:tolerance()} and {cmd:maxiter()} govern PCG.  {cmd:rank_tolerance()},
{cmd:block_tolerance()}, {cmd:exact_limit()}, {cmd:blocksize_limit()}, and
{cmd:physical_limit()}
are explicit safety gates.  Their defaults are documented by {cmd:help
kss_bc} and stored where applicable in {cmd:e()}.  Defaults are 200 probes,
automatic batching (with batch 8 as the small-sample floor), seed 8675309,
solver tolerance 1e-10, 10,000 iterations, exact dimension limit 500, rank
and block tolerances 1e-10, and stored block-size limit 5,000. The largest
solver tolerance is 1e-4. The all-JLA
physical-copy limit defaults to 50,000,000.

{phang}
{cmd:preconditioner(auto)} is the default. It selects between the exact
Schur-diagonal and installed clean-room CMG preconditioners using deterministic
preflight and pilot actions before the production random stream is initialized.
{cmd:preconditioner(diagonal)} forces diagonal PCG.
{cmd:preconditioner(cmg)} forces CMG and fails closed when CMG is unavailable;
it never falls back. {cmd:memory_gib()} declares the CMG allocation envelope
from 1 through 56 GiB and defaults to 4.

{phang}
{cmd:engine(auto)} is the default. Under API 19 it selects the experimental
compressed engine only for an eligible no-control JLA match design. Eligibility
requires every deletion unit to lie within one worker--firm coefficient cell,
an exact target-scale partition within cells, a physical total below 2^53, at
most 16,383 probes, a registered runtime RNG contract, and a passing pre-probe
resource forecast. Multiple deletion IDs may share one coefficient cell.
{cmd:engine(generic)} forces the existing general calculation.
{cmd:engine(compressed)} fails closed with the exact fast-path eligibility
status instead of silently changing the design. An automatic generic fallback
also receives an independent memory and wall-time forecast; a doomed fallback
returns {cmd:GENERIC_RESOURCE_ADMISSION_FAILED} before RNG.

{phang}
{cmd:wallseconds()} declares the command wall-time envelope used for pre-RNG
resource admission. It must lie between 300 and 43,200 seconds and defaults to
43,200. The forecast adds 25--30 percent memory headroom and 50 percent
wall-time headroom and must fit both the declared envelope and the hard
56-GiB/12-hour scale limits. Forecasts are conservative admission evidence,
not measured performance claims.

{title:Description}

{pstd}
{cmd:kss_bc} estimates worker-effect variance, firm-effect variance,
worker--firm covariance, and their total in a linear two-way fixed-effect
model.  It reports plug-in values, the KSS leave-out bias correction, and the
corrected point estimates.  It does not implement econometric inference and
does not post {cmd:e(V)}.

{pstd}
Match deletion is the default.  {cmd:deletionid()} identifies the independent
block and may differ from the worker--firm coefficient cell.  Every supplied
match ID must remain within one worker--firm coordinate.  Dependence across a
worker's distinct matches is not covered by match deletion.

{pstd}
The match headline uses movers: workers observed at more than one firm.
{cmd:stayers(both)} is currently withheld pending a separately labeled
all-worker hybrid; it is never substituted silently.

{pstd}
Match sample construction chooses a largest connected component, enforces the
mover target, and removes insufficient histories and worker articulation
vertices. Distinct deletion IDs are distinct multigraph edges, including
parallel edges at one worker--firm coordinate. Every deletion-unit bridge in
one pass is removed simultaneously, and all stages repeat to a fixed point.
The retained multigraph must pass a final zero-bridge certificate. Observation
deletion keeps the prior selector unchanged. Counts for every stage are returned. A tie
on the registered firm-count and physical-mass ranking is withheld because an
encoded-ID tie-break would not be invariant to ID relabeling.

{pstd}
{cmd:nuisance(joint)} includes controls in every deleted-system inverse.
{cmd:nuisance(fixedoffset)} estimates their full-sample coefficients, removes
that fitted index, and conditions the two-way correction on it.

{pstd}
Frequency weights are positive integer counts of literal physical copies.
Their exact total may not exceed 2^53; larger totals are withheld as
{cmd:PHYSICAL_TOTAL_LIMIT} before graph ranking, so component masses and
{cmd:e(N_physical)} remain exact integers.
Observation deletion removes one copy; match deletion removes every copy in
the match.  An explicit {cmd:targetweight()} is total stored-row target mass
and is not multiplied by the frequency weight.  Without it, target mass is
the frequency count.  Observation JLA applies its finite-probe nonlinear
adjustment separately to every physical copy and only then aggregates final
multipliers back to stored rows.
Every selected generic JLA route withholds as {cmd:PHYSICAL_COPY_LIMIT} before
allocating probe state when the retained literal-copy count exceeds
{cmd:physical_limit()}. The compressed match engine does not materialize
literal-copy signs and therefore does not use this allocation gate; its exact
binomial trial and total-integer gates apply instead.
For observation deletion, the batch-memory forecast includes the literal
physical-copy-by-batch sign matrix. Both an explicit batch and the automatic
batch floor are withheld as {cmd:BATCH_MEMORY_LIMIT} before routing or random
probe generation when projected scratch exceeds 35 percent of
{cmd:memory_gib()}.

{pstd}
API 19 defines a logical probe by a versioned RNG contract, the master seed,
the {cmd:leverage} or {cmd:target} domain, the probe index, and canonical
semantic atom identity/order. Batch width, tiling, solver route, convergence
history, processor count, and phase scheduling cannot change its random
atoms. The two domains use separate registered {cmd:mt64s} streams. Local K1
evidence selects one fixed-order stateful stream per domain over repeated
per-probe resets. The command restores the caller's RNG algorithm, selected
stream, and complete state on every exit. Each Stata runtime needs registered
golden vectors and an unregistered runtime returns
{cmd:RNG_RUNTIME_UNREGISTERED}. Stata 18 is registered locally; Stata 19
qualification remains pending.

{pstd}
Canonical atom order never uses arbitrary dense ID encodings or raw row order.
Generic JLA orders conceptual copies by outcomes and per-copy target mass;
the compressed path uses unique canonical deletion-unit and exact target-
stratum keys. An explicit {cmd:probeorder()} key may refine exact ties without
changing non-tied order. If the resulting key ties while controls differ or
rows span different worker--firm coordinates or match blocks, the command
withholds with {cmd:AMBIGUOUS_PROBE_ORDER}. Exact mode without controls remains
available; controlled exact applies the same semantic-order check and may
withhold as {cmd:AMBIGUOUS_CONTROL_BASIS}. Probe atoms are invariant; estimator
results after different reductions need only satisfy registered numerical
tolerances, not bitwise equality.

{pstd}
The exact backend is deterministic numerical linear algebra, not exact
arithmetic, and is limited by {cmd:exact_limit()}.  Near a Woodbury rank
boundary it directly factors the deleted information matrix as an additional
fail-closed gate.
The JLA backend eliminates worker coordinates exactly, solves the full
firm-mobility Laplacian by PCG on its zero-sum quotient, and grounds the public
coefficient representation only after convergence. It treats low-dimensional
controls through an exact Schur complement. Before either controlled backend,
it maps at most 32 controls to an ID-free canonical basis, so an invertible
user reparameterization produces the same certified numerical right-hand
sides. Its dimensioned envelope covers Gram, inverse/Cholesky, score/cutoff,
anchor-projector, final span, and full/deletion propagation error, with a
registered propagated ceiling of 1e-8. If that envelope cannot certify a
pivot or downstream system, the command withholds as
{cmd:AMBIGUOUS_CONTROL_BASIS}. It uses the coefficient-one
finite-projection correction. Ordinary zero or collinear controls are retained
and rejected by the rank gates; only factor terms that Stata marks omitted are
removed during expansion.
Its reported numerical MCSE describes target-probe variation conditional on
the leverage sketch; it is not econometric inference.

{pstd}
The experimental compressed engine keeps coefficient cells, deletion units,
and exact target-scale strata as separate indices. Target scales are grouped
by exact equality only; cancellation-sensitive grouped sums use compensated
accumulation. In a no-control match block, let {it:E_g} be its frequency-
weighted residual mass, {it:m_g} its constrained residual share, and
{it:B_g} and {it:V_g} its registered coefficient-one finite-projection bias
and variance moments. The exact specialization is

{p 8 12 2}
{it:D_g = E_g (m_g^-1 + B_g m_g^-2 - V_g m_g^-3)},
{it:K_c = sum_(g->c) Y_g D_g},
and one target correction draw is {it:sum_c K_c z_c^2}.

{pstd}
These formulas retain every existing definition, conditioning gate,
reciprocal-residual gate, and typed failure. They avoid per-match generic
eigendecompositions and a row-sized deleted-adjusted vector. Exact algebra
does not imply bitwise equality after regrouping.

{pstd}
Direct compression uses {it:2*Binomial(F,1/2)-F} only when the kernel consumes
the sum of {it:F} independent signs. Leverage meets this condition at deletion
units. Target probes meet it only within an exact per-copy target-scale stratum
at a coefficient cell; unequal scales remain separate strata, and frequency
one is a degenerate fast case. Observation deletion and cross-cell blocks do
not satisfy the match fast-path contract. Exact binary64 integer
representation alone is not a sufficient RNG-call certificate.

{pstd}
The compressed lifecycle marks the estimation sample, constructs its
canonical state, invokes native disk-backed Stata {cmd:preserve}, clears the
raw dataset before peak Mata scratch, frees large Mata state after numerical
work, and restores the caller data before returning. The command verifies the
restored sample signature and exact {cmd:e(sample)} membership. It reports
separate transition, work, restoration, and memory diagnostics. There is no
destructive scale-only mode.

{pstd}
Multiple inverse-action right-hand sides run in lockstep with one matrix Schur
action per iteration. Each column keeps its own recurrence, stopping rule,
iteration count, and freshly recomputed full worker-plus-firm residual. A
failed column withholds the calculation; it cannot be masked by other columns.

{pstd}
For each fit, leverage, and target RHS, the complete residual uses the
original frequency-weighted normal equations:

{p 8 12 2}
{it:r_w = b_w - [d_w alpha_w + sum_(c:w_c=w) F_c gamma_(f_c)]},

{p 8 12 2}
{it:r_f = b_f - [e_f gamma_f + sum_(c:f_c=f) F_c alpha_(w_c)]}.

{pstd}
Here {it:F_c} is the coefficient-cell physical mass, {it:d_w} and {it:e_f}
are its worker and firm mass sums, and {it:b_w,b_f} are the original RHS
blocks.

{pstd}
The solve uses the full-firm zero-sum quotient, then displays the last firm at
zero and still checks that grounded firm's original equation. The norm is the
combined worker/firm Euclidean residual divided by the original RHS Euclidean
norm, or the absolute residual for a zero RHS. Acceptance requires no more
than {cmd:max(1e-11,10*tolerance())}. A graph-only residual is insufficient.

{pstd}
Both backends separately require the plug-in row, correction row, and final
plug-in-minus-correction row to be finite. Overflow in the final subtraction
is withheld as {cmd:NONFINITE_CORRECTED_TARGET}; no partial row is posted.

{title:Stored results}

{pstd}
{cmd:e(b)} and {cmd:e(kss)} contain the corrected values.  {cmd:e(plugin)},
{cmd:e(correction)}, and {cmd:e(numerical_mcse)} are 1 by 4 matrices.
{cmd:e(results)} has rows {cmd:plugin}, {cmd:bias_correction},
{cmd:corrected}, and {cmd:numerical_mcse}; its columns are the four targets.
Exact calculations store zero numerical MCSE.  This is not a sampling
standard error.

{pstd}
Key scalars include {cmd:e(N_requested)}, {cmd:e(N_complete)},
{cmd:e(N_initial_component)}, {cmd:e(N_mover_input)}, {cmd:e(N_retained)},
{cmd:e(N_physical)}, {cmd:e(worker_levels)}, {cmd:e(firm_levels)},
{cmd:e(deletion_units)}, {cmd:e(target_weight_sum)},
{cmd:e(max_leverage)}, {cmd:e(weighted_rss)},
{cmd:e(solver_iterations)}, {cmd:e(solver_max_residual)}, and
{cmd:e(inverse_relres)}.  Dense exact mode reports
{cmd:e(information_rcond)}. Under exact {cmd:nuisance(fixedoffset)}, this is
the minimum reciprocal conditioning across the preliminary full joint fit and
the pure-FE working fit. {cmd:e(full_parameters)} counts the preliminary full
design and {cmd:e(correction_parameters)} counts the working leave-out design.
{cmd:e(parameters)} is a compatibility alias for the latter, so fixed offset
excludes the already-fitted controls from that count. JLA reports
{cmd:e(preconditioner_ratio)}, and
its matrix-free joint-control preparation reports
{cmd:e(control_schur_rcond)}.  Graph,
algorithm, weight, target, and sample-selection metadata are also stored.
Match diagnostics include {cmd:e(graph_retained_edges)},
{cmd:e(graph_bridge_units_removed)}, {cmd:e(graph_bridge_rows_removed)},
{cmd:e(graph_bridge_iterations)}, {cmd:e(graph_fixedpoint_iterations)}, and
the required zero certificate {cmd:e(graph_final_bridge_units)}.
Every accepted JLA calculation with controls, including fixed offset, reports
the positive deterministic lower bound {cmd:e(deletion_rank_gap)} after
full-fit, trace, direct deleted-scatter, whitening-error, and rounding gates.
A design that does not satisfy this sufficient certificate is withheld for
exact verification.
Timing scalars include {cmd:e(graph_seconds)}, {cmd:e(fit_seconds)},
{cmd:e(setup_seconds)}, {cmd:e(preconditioner_seconds)},
{cmd:e(schur_seconds)}, {cmd:e(preconditioner_apply_seconds)},
{cmd:e(pcg_seconds)}, {cmd:e(solver_backend_seconds)},
{cmd:e(leverage_seconds)}, {cmd:e(target_seconds)},
{cmd:e(correction_seconds)}, and, for the compressed engine,
{cmd:e(rng_seconds)}. Correction and RNG are nested substage attributions;
they are not additional terms to add to command wall time.
{cmd:e(preconditioner_seconds)} is the
compatibility alias for setup time. Under API 18, setup and fit are disjoint
timers; exact mode records zero setup time. {cmd:e(solver_rhs_diagnostics)} stores stage, batch start,
global logical RHS index, iterations, complete relative residual, and
convergence indicator. Stage-4 indices cover 1 through P and stage-5 indices
cover 1 through 2P even when a run uses several numerical batches.
RHS-equivalent and physical-batch Schur/preconditioner counts are stored
separately.

{pstd}
JLA routing returns {cmd:e(preconditioner_requested)},
{cmd:e(preconditioner_selected)}, {cmd:e(routing_reason)},
{cmd:e(fallback_status)}, {cmd:e(fallback_message)},
{cmd:e(route_diagnostics)}, {cmd:e(memory_gib)},
{cmd:e(batch_requested)}, the selected numeric {cmd:e(batch)},
{cmd:e(batch_routing_reason)}, and batch scratch/budget forecasts. Automatic
CMG routes also return {cmd:e(route_hybrid_vertices)},
{cmd:e(route_hybrid_edges)}, {cmd:e(route_hierarchy_levels)}, and the bounded
{cmd:e(route_terminal_vertices)}. Automatic
fallback is limited to registered CMG preflight, construction, or pilot
boundaries before production RNG. Forced CMG never falls back.

{pstd}
API 19 JLA calls also return {cmd:e(engine_requested)},
{cmd:e(engine_selected)}, {cmd:e(fastpath_status)},
{cmd:e(fastpath_message)}, {cmd:e(resource_status)},
{cmd:e(resource_peak_phase)}, {cmd:e(resource_components)}, and
{cmd:e(resource_forecasts)}.  Compressed calls report
{cmd:e(coefficient_cells)}, {cmd:e(deletion_units)},
{cmd:e(target_strata)}, {cmd:e(row_cell_compression)}, separate
{cmd:e(leverage_batch)} and {cmd:e(target_batch)}, and
{cmd:e(scale_receipt)}. The receipt includes compressed numerical and RNG
substage times. Resource scalars distinguish selection, transition,
numerical, and restoration peak forecasts; memory and wall-time admission;
and their hard limits.

{pstd}
Compressed lifecycle returns include {cmd:e(life_method)}, transition, work,
and restoration seconds; memory before clearing, while cleared, during work,
and after restoration; and {cmd:e(life_sample_restored)}. RNG metadata record
the versioned contract, implementation, runtime, master seed, separate
leverage/target domains, and each domain's inclusive probe range. Residual
metadata record the full-firm zero-sum quotient, last-firm displayed grounding,
original-equation normalization, the acceptance tolerance, the maximum
complete residual, reciprocal correction residual, and target identity
residual.

{pstd}
A recognized invalid calculation returns {cmd:e(status)="WITHHELD"} and a
typed {cmd:e(withholding_status)} before exiting.  The command does not use a
hidden ridge, change the deletion unit, or loosen numerical tolerances.
Finite-probe gates can conservatively withhold a design that is exactly
estimable.  Passing those gates does not imply conditional unbiasedness.

{title:Example}

{phang2}{cmd:. kss_bc log_wage age2 age3 i.year [fw=freq],}{p_end}
{phang3}{cmd:worker(person_id) firm(analysis_firm_id)}{p_end}
{phang3}{cmd:deletion(match) deletionid(actual_match_id)}{p_end}
{phang3}{cmd:algorithm(jla) nuisance(joint) targetweight(target_mass)}{p_end}
{phang3}{cmd:probes(200) batch(auto) engine(auto)}{p_end}
{phang3}{cmd:preconditioner(auto) memory_gib(56)}{p_end}
{phang3}{cmd:wallseconds(43200) seed(8675309)}{p_end}

{title:Status}

{pstd}
Version 0.2.0-dev is internal candidate software and is not production-
qualified.  The source-bound KSS-PROD-1 run passes CZ24, CZ25, and full CZ18,
but three larger-stress calibrations withhold at the typed automatic-route gate
before RNG; no full stress run was admitted. API 19's single-process compressed
engine has local algebra, RNG, lifecycle, resource, fixture, and command tests,
but its Stata 19, CZ24/CZ25, CZ18, and 2x/4x SCC qualification remains pending.
It is not production-qualified or a public release. The repository has no
selected public software license, so public redistribution is not authorized.
