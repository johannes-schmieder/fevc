{smcl}
{* *! version 0.1.0-dev 14aug2026}{...}
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
{cmd:probes(}{it:#}{cmd:)} {cmd:batch(}{it:#}{cmd:)}
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
{cmd:tolerance()} and {cmd:maxiter()} govern PCG.  {cmd:rank_tolerance()},
{cmd:block_tolerance()}, {cmd:exact_limit()}, {cmd:blocksize_limit()}, and
{cmd:physical_limit()}
are explicit safety gates.  Their defaults are documented by {cmd:help
kss_bc} and stored where applicable in {cmd:e()}.  Defaults are 200 probes,
batch size 8, seed 8675309, solver tolerance 1e-10, 10,000 iterations, exact
dimension limit 500, rank and block tolerances 1e-10, and stored block-size
limit 5,000. The largest solver tolerance is 1e-4. The observation-JLA
physical-copy limit defaults to 50,000,000.

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
Sample construction follows the maintained MATLAB compatibility rule.  The
command chooses a largest connected component, removes worker articulation
vertices iteratively, and retains a largest resulting component.  Match mode
also enforces a mover-only fit.  Counts for every stage are returned.  A tie
on the registered firm-count and physical-mass ranking is withheld because an
encoded-ID tie-break would not be invariant to ID relabeling.

{pstd}
{cmd:nuisance(joint)} includes controls in every deleted-system inverse.
{cmd:nuisance(fixedoffset)} estimates their full-sample coefficients, removes
that fitted index, and conditions the two-way correction on it.

{pstd}
Frequency weights are positive integer counts of literal physical copies.
Observation deletion removes one copy; match deletion removes every copy in
the match.  An explicit {cmd:targetweight()} is total stored-row target mass
and is not multiplied by the frequency weight.  Without it, target mass is
the frequency count.  Observation JLA applies its finite-probe nonlinear
adjustment separately to every physical copy and only then aggregates final
multipliers back to stored rows.
Observation JLA withholds as {cmd:PHYSICAL_COPY_LIMIT} before allocating probe
state when the retained literal-copy count exceeds {cmd:physical_limit()}.

{pstd}
For fixed-seed reproducibility, JLA orders conceptual copies by outcomes and
per-copy target mass, never by encoded IDs, stored-row fields, or raw control
coordinates.  If that primary key ties while controls differ or rows span
different worker--firm coordinates or match blocks, the command withholds with
{cmd:AMBIGUOUS_PROBE_ORDER}; {cmd:algorithm(exact)} remains available.

{pstd}
The exact backend is deterministic numerical linear algebra, not exact
arithmetic, and is limited by {cmd:exact_limit()}.  Near a Woodbury rank
boundary it directly factors the deleted information matrix as an additional
fail-closed gate.
The JLA backend eliminates worker coordinates exactly, solves the full
firm-mobility Laplacian by PCG on its zero-sum quotient, and grounds the public
coefficient representation only after convergence. It treats low-dimensional
controls through an exact Schur complement. Before iterative control solves,
it maps the requested control span to an ID-free canonical basis, so an
invertible user reparameterization produces the same numerical right-hand
sides. It uses the coefficient-one
finite-projection correction. Ordinary zero or collinear controls are retained
and rejected by the rank gates; only factor terms that Stata marks omitted are
removed during expansion.
Its reported numerical MCSE describes target-probe variation conditional on
the leverage sketch; it is not econometric inference.

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
Every accepted JLA calculation with controls, including fixed offset, reports
the positive deterministic lower bound {cmd:e(deletion_rank_gap)} after
full-fit, trace, direct deleted-scatter, whitening-error, and rounding gates.
A design that does not satisfy this sufficient certificate is withheld for
exact verification.
Timing scalars include {cmd:e(graph_seconds)}, {cmd:e(fit_seconds)},
{cmd:e(preconditioner_seconds)}, {cmd:e(leverage_seconds)},
{cmd:e(target_seconds)}, and {cmd:e(correction_seconds)}.  Preconditioner setup
is included within fit time; exact mode records zero for that field.

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
{phang3}{cmd:probes(200) batch(8) seed(8675309)}{p_end}

{title:Status}

{pstd}
Version 0.1.0-dev is internal development software.  The repository has no
selected public software license, so public redistribution is not authorized.
