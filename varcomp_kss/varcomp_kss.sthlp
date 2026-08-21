{smcl}
{* *! version 0.3.0-dev 21aug2026}{...}
{.-}
help for {cmd:varcomp_kss} {right:(Johannes F. Schmieder)}
{.-}

{title:Title}

{p 4 4 2}
{cmd:varcomp_kss} {hline 2} KSS leave-out bias-corrected variance
decompositions for linear two-way fixed-effect models

{marker quickstart}
{title:Quick start}

{pstd}
{cmd:varcomp_kss} estimates the variance of worker effects, the variance of
firm effects, their covariance, and the variance of their sum.  The labels
{cmd:worker()} and {cmd:firm()} follow the classic AKM application, but the
two dimensions can instead be patients and physicians, products and stores,
authors and institutions, or any other linked pair.

{pstd}
A typical match-deletion call is

{phang2}{cmd:. varcomp_kss log_wage i.year, worker(worker_id) firm(firm_id) ///}{p_end}
{phang3}{cmd:deletion(match) deletionid(match_id) nuisance(joint)}{p_end}

{pstd}
The default output first reports the four KSS targets.  It then writes the
additive identity

{p 8 12 2}
worker variance + firm variance + 2 x worker-firm covariance
= total worker-firm variance.

{pstd}
Shares of outcome variance use the same retained target mass as the KSS
targets.  A separate descriptive full-model fit summary uses regression
frequency weights and includes supplied controls.  These totals coincide in
weighting only when {cmd:targetweight()} is not supplied.

{marker syntax}
{title:Syntax}

{p 8 16 2}
{cmd:varcomp_kss} {it:depvar} [{it:controls}]
[{cmd:[fw=}{it:frequency}{cmd:]}] [{help if}] [{help in}],
{cmd:worker(}{it:varname}{cmd:)} {cmd:firm(}{it:varname}{cmd:)}
[{it:options}]

  {it:option}{col 36}description
  {hline 76}
  {ul:Required identifiers}
    {cmd:worker(}{it:varname}{cmd:)}{col 36}first fixed-effect dimension
    {cmd:firm(}{it:varname}{cmd:)}{col 36}second fixed-effect dimension

  {ul:Deletion and target population}
    {cmd:deletion(match|observation)}{col 36}delete a declared match or one physical observation
    {cmd:deletionid(}{it:varname}{cmd:)}{col 36}dependence-block ID for match deletion
    {cmd:stayers(movers|both)}{col 36}target convention; only movers is currently implemented
    {cmd:targetweight(}{it:varname}{cmd:)}{col 36}stored-row target mass, separate from regression weight

  {ul:Controls and numerical method}
    {cmd:nuisance(joint|fixedoffset)}{col 36}re-estimate controls after deletion or hold their index fixed
    {cmd:algorithm(auto|exact|jla)}{col 36}automatic, dense deterministic, or randomized calculation
    {cmd:backend(auto|mata|rust)}{col 36}public estimator backend routing
    {cmd:rng(stata|counter_v1)}{col 36}explicit RNG contract; Counter-V1 is Rust-only
    {cmd:engine(auto|compressed|generic)}{col 36}automatic or forced JLA representation
    {cmd:preconditioner(auto|diagonal|cmg)}{col 36}automatic or forced iterative-solver route

  {ul:JLA reproducibility and work}
    {cmd:probes(}{it:#}{cmd:)}{col 36}number of random projections; default 200
    {cmd:batch(auto|}{it:#}{cmd:)}{col 36}simultaneous right-hand-side width
    {cmd:seed(}{it:#}{cmd:)}{col 36}registered master seed; default 8675309
    {cmd:probeorder(}{it:varname}{cmd:)}{col 36}optional semantic tie-breaker
    {cmd:tolerance(}{it:#}{cmd:)}{col 36}PCG tolerance; default 1e-10
    {cmd:maxiter(}{it:#}{cmd:)}{col 36}maximum PCG iterations; default 10,000

  {ul:Safety and resource envelopes}
    {cmd:memory_gib(}{it:#}{cmd:)}{col 36}direct-allocation envelope; default 4 GiB
    {cmd:wallseconds(}{it:#}{cmd:)}{col 36}optional advisory wall-time envelope
    {cmd:exact_limit(}{it:#}{cmd:)}{col 36}maximum exact identified dimension; default 500
    {cmd:rank_tolerance(}{it:#}{cmd:)}{col 36}rank gate; default 1e-10
    {cmd:block_tolerance(}{it:#}{cmd:)}{col 36}deleted-block gate; default 1e-10
    {cmd:blocksize_limit(}{it:#}{cmd:)}{col 36}stored match-block limit; default 5,000
    {cmd:physical_limit(}{it:#}{cmd:)}{col 36}generic JLA literal-copy limit; default 50,000,000
    {cmd:nodisplay}{col 36}suppress successful output; stored results are unchanged
  {hline 76}

{marker backend}
{title:Backend routing}

{pstd}
Omitting {cmd:backend()} permanently selects the established Mata estimator;
it is not an alias for {cmd:backend(auto)}.  Explicit {cmd:backend(mata)} and
{cmd:backend(auto)} also select Mata.  These routes use the historical Stata
RNG contract; {cmd:rng(stata)} may be explicit or omitted.

{pstd}
The strict source-local Rust route requires explicit
{cmd:backend(rust) rng(counter_v1) algorithm(jla)}
{cmd:preconditioner(diagonal) batch(}{it:#}{cmd:)}.  It supports match
deletion, joint nuisance handling, movers, {cmd:if}/{cmd:in}, frequency and
target weights, deletion IDs, {cmd:engine(auto|compressed)}, and ordinary
seed, probe, tolerance, iteration, and memory options.  Controls, observation
deletion, fixed-offset nuisance, stayers, {cmd:probeorder()},
{cmd:wallseconds()}, automatic batching, exact, generic engine, CMG, and
nondefault unforwarded structural limits are rejected before native
preparation.  There is no native-to-Mata fallback.

{pstd}
{cmd:backend(rust)} without explicit {cmd:rng(counter_v1)}, and
{cmd:rng(counter_v1)} with omitted, Mata, or auto backend, are typed errors.
The Rust route uses a stateless canonical Counter-V1 contract and never
silently changes the caller's Stata RNG.  All successful routes remain point
estimates plus numerical diagnostics; the command does not post {cmd:e(V)}.

{marker description}
{title:What the command estimates}

{pstd}
On the retained sample, the fitted model is

{p 8 12 2}
{it:y} = worker effect + firm effect + nuisance controls + error.

{pstd}
For each target, the plug-in estimate is the corresponding quadratic form in
the full-sample least-squares coefficients.  Plug-in variance components are
upward biased when many worker and firm effects are estimated imprecisely.
The Kline--Saggio--Sølvsten correction uses outcomes from each declared
deletion block together with residuals evaluated against a fit that excludes
that block.  The reported estimate is

{p 8 12 2}
KSS corrected = plug-in - bias correction.

{pstd}
The four stored targets are worker variance, firm variance, raw worker-firm
covariance, and total worker-firm variance.  The additive output uses twice
the covariance as the sorting contribution so that its components add to the
total.

{pstd}
Controls are nuisance coefficients and have zero weight in the four KSS
target matrices.  The corrected worker-firm total is therefore not a
KSS-corrected decomposition of the controls.  The separate full-model
explained variance is descriptive: it equals frequency-weighted
{cmd:Var(Y)} minus {cmd:e(weighted_rss)/e(N_physical)} and includes controls.

{marker output}
{title:Reading the output}

{pstd}
The header reports the retained stored rows, literal physical observations,
worker and firm levels, deletion units, target population, numerical method,
engine, and preconditioner.

{pstd}
{ul:Quadratic-form targets} reports plug-in levels, the estimated bias
correction, and the corrected KSS levels.  Its covariance row is the raw
covariance stored in {cmd:e(results)}.

{pstd}
{ul:Additive worker-firm decomposition} replaces that covariance row with
{cmd:2 x covariance}.  {ul:Shares} reports both plug-in and corrected
components as percentages of target-weighted outcome variance and of their
corresponding worker-firm totals.  Negative sorting contributions and shares
above 100 percent can be economically meaningful.  Shares are missing when
their denominator is nonpositive.

{pstd}
{ul:Variance and fit summary} deliberately distinguishes:

{p 8 12 2}
1. target-weighted {cmd:Var(Y)} and the KSS-corrected worker-firm total; and

{p 8 12 2}
2. frequency-weighted {cmd:Var(Y)} and descriptive full-model explained
variance.

{pstd}
When {cmd:targetweight()} differs from the frequency weight, these are
different populations and should not be combined into one accounting
identity.

{pstd}
JLA also reports a numerical MCSE for the target-probe mean conditional on
the realized leverage sketch.  It is not a sampling standard error, excludes
first-pass sketch uncertainty, and is not econometric inference.  The command
does not post {cmd:e(V)}.

{marker sample}
{title:Sample construction and deletion assumptions}

{pstd}
{cmd:deletion(match)} is the default.  A deletion unit contains every retained
copy with the same {cmd:deletionid()}.  One ID must not cross worker-firm
coordinates.  If {cmd:deletionid()} is omitted, the worker-firm coordinate is
the match.  Match deletion allows arbitrary covariance within a declared
match and assumes independence across declared matches; it does not allow
arbitrary dependence across all matches belonging to one worker.

{pstd}
The match headline is a mover target.  The command selects a largest connected
component, removes stayers from the target, removes insufficient histories
and worker articulation vertices, and repeatedly removes deletion-unit
bridges until reaching a fixed point.  Parallel deletion IDs at one
worker-firm coordinate remain distinct multigraph edges.  A successful match
sample has a final zero-bridge certificate.  A tied component ranking is
withheld rather than broken using arbitrary encoded IDs.

{pstd}
{cmd:deletion(observation)} deletes one literal physical observation and uses
the retained-observation target.  With frequency weights, one stored row
represents several physical observations; observation deletion removes one
copy, while match deletion removes every copy in the block.

{marker controls}
{title:Nuisance controls}

{pstd}
{cmd:nuisance(joint)} is the default.  Controls are part of every deleted fit,
so their coefficients can move when a match or observation is removed.  This
is the primary joint leave-out convention.

{pstd}
{cmd:nuisance(fixedoffset)} first estimates the full model, subtracts the
full-sample control index, and holds that index fixed while correcting the
two-way effects.  This is a conditional convention and can differ from joint
deletion in finite samples.

{pstd}
Submitted numeric controls must have an identified, numerically stable span.
The command does not silently drop ordinary zero or collinear variables.
Only factor-variable terms explicitly marked omitted by Stata are removed.
JLA supports at most 32 joint controls and applies a fail-closed deleted-rank
certificate.

{marker weights}
{title:Regression weights and target weights}

{pstd}
Frequency weights are positive integer physical-copy counts.  They affect the
least-squares fit, graph mass, deletion blocks, and residual sum of squares.
Their exact retained total must be representable as a binary64 integer.

{pstd}
{cmd:targetweight()} instead defines how retained rows are weighted in the
variance targets.  It is stored-row mass and is not multiplied by the
frequency weight.  Without the option, target mass equals frequency mass.
Changing target weights changes the estimand, not merely its efficiency.

{marker algorithms}
{title:Exact and randomized calculations}

{pstd}
{cmd:algorithm(exact)} uses deterministic dense linear algebra and is intended
for smaller designs and validation.  "Exact" means deterministic numerical
linear algebra, not exact arithmetic.  It is limited by {cmd:exact_limit()}
and {cmd:blocksize_limit()}.

{pstd}
{cmd:algorithm(jla)} uses reproducible randomized inverse actions.  It solves
the full worker-firm normal equations, certifies the original worker and firm
residuals for every accepted right-hand side, and applies the coefficient-one
finite-projection correction.  A graph-only or reduced-system residual is not
sufficient.

{pstd}
{cmd:algorithm(auto)} chooses exact when the identified dimension is within
{cmd:exact_limit()} and JLA otherwise.  Automatic JLA routing uses the
compressed engine only for an exactly representable no-control match design;
other supported designs use the generic engine.  {cmd:preconditioner(auto)}
chooses diagonal or package-owned CMG from structural preflight before the
production random stream begins.

{pstd}
The complete direct-peak forecast is the memory admission gate; percentage and processor rules are
only automatic batch-width heuristics.  Structural preflight, not routing trial solves, selects the
automatic preconditioner.  Reported setup and fit are disjoint timings, so setup is not counted twice.

{pstd}
The fixed seed is tied to the runtime contract and canonical semantic atom
order.  Supported batching and solver routes do not change those atoms.  The
command restores the caller's RNG algorithm, stream, complete state, sort
jumbler, data, and estimation sample on every supported exit.

{marker troubleshooting}
{title:Troubleshooting withheld calculations}

{pstd}
A recognized failure stores {cmd:e(status)="WITHHELD"}, a technical
{cmd:e(withholding_status)}, the detailed condition, a plain-language reason,
and a suggested next step.  The command withholds the whole decomposition; it
does not drop a failed block, add a hidden ridge, change the sample, change
the deletion unit, reduce probes, or loosen tolerances silently.

  {it:problem}{col 34}what to check
  {hline 76}
  {ul:Input or deletion definition}
    Invalid weights or IDs{col 34}check types, missing values, integer frequency, and target mass
    Cross-coordinate match{col 34}each deletion ID must stay within one worker-firm coordinate
    Incomplete match input{col 34}all frozen outcome, ID, control, and weight inputs must be complete

  {ul:Graph and target sample}
    No mover/leave-out sample{col 34}inspect mover histories, match IDs, and requested restrictions
    Ambiguous component{col 34}the command will not break an exact ranking tie using encoded IDs

  {ul:Identification and controls}
    Singular information{col 34}remove substantively redundant controls or repair the design
    Unverified deletion rank{col 34}try exact on a feasible design or revise weakly supported controls
    Nonestimable deletion{col 34}inspect thin matches and whether every declared block can be removed

  {ul:Computation and resources}
    Exact size limit{col 34}use auto/JLA for a large identified design
    PCG nonconvergence{col 34}check scaling/connectivity, maxiter(), and solver route
    Memory admission{col 34}reduce batch width or declare only actually available memory
    Forced compressed failure{col 34}use engine(auto) or generic for unsupported structures

  {ul:Installation and runtime}
    Stale Mata runtime{col 34}run discard or restart Stata, then reinstall one complete build
    Unregistered JLA runtime{col 34}use supported Stata 18/19 or exact when feasible
    Unavailable Rust artifact{col 34}run the source-local macOS qualifier or use backend(mata)
    Unsupported Rust options{col 34}use the documented strict JLA/match/diagonal subset or backend(mata)
  {hline 76}

{pstd}
Do not treat a conservative rejection as proof that the economic estimand does
not exist.  It means this implementation did not certify the requested finite
calculation under its registered gates.  When asking for support, report the
command line, Stata version, {cmd:e(withholding_status)},
{cmd:e(withholding_detail)}, and a small reproducible design when possible.

{marker stored}
{title:Stored results}

{pstd}
The existing scientific return contract is unchanged.  {cmd:e(results)} has
rows {cmd:plugin}, {cmd:bias_correction}, {cmd:corrected}, and
{cmd:numerical_mcse}; columns are {cmd:worker_variance},
{cmd:firm_variance}, {cmd:worker_firm_covariance}, and
{cmd:total_variance}.  The corrected row is also stored in {cmd:e(b)} and
{cmd:e(kss)}.  Separate matrices are {cmd:e(plugin)},
{cmd:e(correction)}, and {cmd:e(numerical_mcse)}.

{pstd}
{cmd:e(decomposition)} is the additive applied-user view.  Its rows are
{cmd:worker_variance}, {cmd:firm_variance},
{cmd:sorting_2covariance}, and {cmd:total_worker_firm}.  Its columns contain
plug-in, bias correction, corrected levels, plug-in/corrected shares of
target-weighted outcome variance, and plug-in/corrected shares of the
worker-firm total.  Stored shares are proportions; the display multiplies
them by 100.

{pstd}
Outcome and fit scalars are {cmd:e(target_outcome_variance)},
{cmd:e(regression_outcome_variance)}, {cmd:e(residual_variance)},
{cmd:e(full_model_explained_variance)}, and
{cmd:e(full_model_explained_share)}.

{pstd}
Sample and design scalars include {cmd:e(N_requested)},
{cmd:e(N_complete)}, {cmd:e(N_initial_component)},
{cmd:e(N_mover_input)}, {cmd:e(N_retained)}, {cmd:e(N_physical)},
{cmd:e(worker_levels)}, {cmd:e(firm_levels)},
{cmd:e(deletion_units)}, {cmd:e(target_weight_sum)},
{cmd:e(weighted_rss)}, {cmd:e(max_leverage)}, and graph-pruning counts.

{pstd}
JLA additionally stores the selected engine, preconditioner, routing reason,
batch, probes, complete residual, per-RHS convergence diagnostics, resource
forecasts, timing diagnostics, RNG contract, and restoration metadata.  Type
{cmd:ereturn list} after a successful call for the complete diagnostic set.

{pstd}
Backend routing is recorded in {cmd:e(backend_requested)},
{cmd:e(backend_selected)}, {cmd:e(backend_routing_reason)}, and
{cmd:e(backend_option_supplied)}.  The last is zero only when
{cmd:backend()} was omitted.  RNG routing is recorded analogously in
{cmd:e(rng_requested)}, {cmd:e(rng_selected)}, and
{cmd:e(rng_option_supplied)}.  Strict Rust results additionally include
{cmd:e(rust_preparation_receipt)}, {cmd:e(rust_graph_receipt)},
{cmd:e(rust_memory_receipt)}, {cmd:e(rust_rhs_receipts)}, capability masks,
topology checksum halves, and the Counter-V1 contract.  On strict Rust
failure, {cmd:e(backend_selected)} is empty and the routing fields accompany
the typed withholding result.

{pstd}
On a recognized failure, the principal strings are
{cmd:e(withholding_status)}, {cmd:e(withholding_detail)},
{cmd:e(withholding_reason)}, and {cmd:e(withholding_suggestion)}.

{marker examples}
{title:Examples}

{pstd}
Each example creates its own connected AKM-style worker-firm graph and nuisance
controls.  The visible {cmd:preserve}/{cmd:restore} lines make the block safe
to copy into a do-file.  The clickable link executes the marked inner block
through {cmd:varcomp_kss_run}, which also restores the caller's data.  Before
estimation, each block displays the population worker and firm variances,
worker-firm covariance, and total implied by its DGP.

{space 4}{hline 10} {it:Example 1 - Small exact calculation with joint controls} {hline 10}
{cmd}{...}
          preserve
{* example_start - exact_controls}{...}
          clear
          set seed 20260820
          local workers 80
          local firms 20
          local spells 3
          local periods 2
          set obs `=`workers'*`spells'*`periods''
          generate long worker_id = ceil(_n/(`spells'*`periods'))
          bysort worker_id: generate byte within_worker = _n
          generate byte spell = ceil(within_worker/`periods')
          generate byte period = mod(within_worker-1,`periods')+1
          generate long firm_id = mod(worker_id-1+(spell-1)*7,`firms')+1
          generate long match_id = worker_id*10+spell
          generate double worker_fe = rnormal() if within_worker==1
          bysort worker_id: replace worker_fe = worker_fe[1]
          bysort firm_id: generate double firm_fe = rnormal() if _n==1
          bysort firm_id: replace firm_fe = firm_fe[1]
          generate double productivity = rnormal()
          generate double log_wage = 2+worker_fe+firm_fe+.30*productivity+.15*(period==2)+.50*rnormal()
          display as text _newline "True DGP worker-firm components (population):"
          display as text "  Var(worker effect)       = " as result %6.2f 1
          display as text "  Var(firm effect)         = " as result %6.2f 1
          display as text "  Cov(worker, firm)        = " as result %6.2f 0
          display as text "  Var(worker + firm)       = " as result %6.2f 2
          varcomp_kss log_wage productivity i.period, worker(worker_id) firm(firm_id) ///
              deletion(match) deletionid(match_id) nuisance(joint) algorithm(exact)
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata varcomp_kss_run exact_controls using varcomp_kss.sthlp:click to run})}

{space 4}{hline 10} {it:Example 2 - Larger controlled graph with JLA} {hline 10}
{cmd}{...}
          preserve
{* example_start - jla_controls}{...}
          clear
          set seed 20260821
          local workers 180
          local firms 45
          local spells 4
          local periods 2
          set obs `=`workers'*`spells'*`periods''
          generate long worker_id = ceil(_n/(`spells'*`periods'))
          bysort worker_id: generate byte within_worker = _n
          generate byte spell = ceil(within_worker/`periods')
          generate byte period = mod(within_worker-1,`periods')+1
          generate long firm_id = mod(worker_id-1+(spell-1)*11,`firms')+1
          generate long match_id = worker_id*10+spell
          generate double worker_fe = rnormal() if within_worker==1
          bysort worker_id: replace worker_fe = worker_fe[1]
          bysort firm_id: generate double firm_fe = .7*rnormal() if _n==1
          bysort firm_id: replace firm_fe = firm_fe[1]
          generate double productivity = rnormal()
          generate double log_wage = 2+worker_fe+firm_fe+.25*productivity+.10*(period==2)+.60*rnormal()
          display as text _newline "True DGP worker-firm components (population):"
          display as text "  Var(worker effect)       = " as result %6.2f 1
          display as text "  Var(firm effect)         = " as result %6.2f .49
          display as text "  Cov(worker, firm)        = " as result %6.2f 0
          display as text "  Var(worker + firm)       = " as result %6.2f 1.49
          varcomp_kss log_wage productivity i.period, worker(worker_id) firm(firm_id) ///
              deletion(match) deletionid(match_id) nuisance(joint) algorithm(jla) ///
              probes(40) batch(8) seed(8675309) engine(auto) preconditioner(auto)
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata varcomp_kss_run jla_controls using varcomp_kss.sthlp:click to run})}

{space 4}{hline 10} {it:Example 3 - Frequency weights, target mass, and fixed controls} {hline 10}
{cmd}{...}
          preserve
{* example_start - weights_targets}{...}
          clear
          set seed 20260822
          local workers 60
          local firms 15
          local spells 3
          set obs `=`workers'*`spells''
          generate long worker_id = ceil(_n/`spells')
          bysort worker_id: generate byte spell = _n
          generate long firm_id = mod(worker_id-1+cond(spell==1,0,cond(spell==2,1,7)),`firms')+1
          generate long match_id = worker_id*10+spell
          generate int frequency = 1+mod(worker_id+spell,3)
          generate double target_mass = 1+spell/2
          generate double worker_fe = rnormal() if spell==1
          bysort worker_id: replace worker_fe = worker_fe[1]
          bysort firm_id: generate double firm_fe = .6*rnormal() if _n==1
          bysort firm_id: replace firm_fe = firm_fe[1]
          generate double productivity = rnormal()
          generate double log_wage = 2+worker_fe+firm_fe+.35*productivity+.45*rnormal()
          display as text _newline "True DGP worker-firm components (population):"
          display as text "  Var(worker effect)       = " as result %6.2f 1
          display as text "  Var(firm effect)         = " as result %6.2f .36
          display as text "  Cov(worker, firm)        = " as result %6.2f 0
          display as text "  Var(worker + firm)       = " as result %6.2f 1.36
          varcomp_kss log_wage productivity [fw=frequency], worker(worker_id) firm(firm_id) ///
              deletion(match) deletionid(match_id) nuisance(fixedoffset) ///
              targetweight(target_mass) algorithm(exact)
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata varcomp_kss_run weights_targets using varcomp_kss.sthlp:click to run})}

{marker reference}
{title:Reference}

{p 4 4 2}
Kline, Patrick, Raffaele Saggio, and Mikkel Sølvsten. 2020.
"Leave-Out Estimation of Variance Components." {it:Econometrica}
88(5): 1859-1898.

{marker author}
{title:Author}

{p 4 4 2}
Johannes F. Schmieder, Boston University, USA

{p 4 4 2}
Email: {browse "mailto:johannes@bu.edu":johannes@bu.edu}

{marker status}
{title:Development status}

{pstd}
Version 0.3.0-dev is internal candidate software.  Covered implementation
source is GPL-3.0-only, but public release remains disabled pending the
documented human license and provenance review.  The command provides point
estimates and numerical diagnostics; it is not a substitute for an
application-specific econometric inference procedure.

{marker also}
{title:Also see}

{p 0 24}
Online: {help regress}, {help xtreg}, {help areg}, {help fvvarlist},
{help weights}
{p_end}
{.-}
