{smcl}
{* *! version 0.5.0-alpha.1 02sep2026}{...}
{.-}
help for {cmd:fevc} {right:(Johannes F. Schmieder)}
{.-}

{title:Title}

{p 4 4 2}
{cmd:fevc} {hline 2} KSS leave-out variance decompositions and
fixed-effect projection inference for linear two-way fixed-effect models

{marker quickstart}
{title:Quick start}

{pstd}
{cmd:fevc} estimates the variance of worker effects, the variance of
firm effects, their covariance, and the variance of their sum.  The labels
{cmd:worker()} and {cmd:firm()} follow the classic AKM application, but the
two dimensions can instead be patients and physicians, products and stores,
authors and institutions, or any other linked pair.

{pstd}
A minimal call is

{phang2}{cmd:. fevc log_wage, worker(worker_id) firm(firm_id)}{p_end}

{pstd}
This uses match deletion, joint nuisance handling, the combined
mover-plus-eligible-stayer target, 200 JLA probes, and automatic backend,
engine, preconditioner, and batch selection.  A call with controls and an
explicit dependence-block identifier is

{phang2}{cmd:. fevc log_wage i.year, worker(worker_id) firm(firm_id) ///}{p_end}
{phang3}{cmd:deletion(match) deletionid(match_id) nuisance(joint)}{p_end}

{pstd}
The default output reports sample retention, the estimand, the selected
computation route, and one additive table built around the identity

{p 8 12 2}
worker variance + firm variance + 2 x worker-firm covariance
= total worker-firm variance.

{pstd}
The table shows plug-in values, estimated bias, corrected KSS values, and
corrected shares of target-weighted outcome variance.  Type
{cmd:estat decomposition, full} for raw covariance, all share denominators,
and descriptive full-model fit accounting.

{marker syntax}
{title:Syntax}

{p 8 16 2}
{cmd:fevc} {it:depvar} [{it:controls}]
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
    {cmd:stayers(both|movers)}{col 36}combined MATLAB population (match default) or mover-only opt-out
    {cmd:targetweight(}{it:varname}{cmd:)}{col 36}stored-row target mass, separate from regression weight

  {ul:Controls and numerical method}
    {cmd:nuisance(joint|fixedoffset)}{col 36}re-estimate controls after deletion or hold their index fixed
    {cmd:algorithm(auto|exact|jla)}{col 36}automatic, dense deterministic, or randomized calculation
    {cmd:backend(auto|mata|rust)}{col 36}public estimator backend routing
    {cmd:rng(auto|stata|counter_v1)}{col 36}automatic or explicit RNG contract; Counter-V1 is Rust-only
    {cmd:engine(auto|compressed|generic)}{col 36}automatic or forced JLA representation
    {cmd:preconditioner(auto|diagonal|cmg)}{col 36}automatic or forced iterative-solver route

  {ul:JLA reproducibility and work}
    {cmd:probes(}{it:#}{cmd:)}{col 36}number of random projections; default 200
    {cmd:batch(auto|}{it:#}{cmd:)}{col 36}simultaneous right-hand-side width
    {cmd:seed(}{it:#}{cmd:)}{col 36}registered master seed; default 8675309
    {cmd:probeorder(}{it:varname}{cmd:)}{col 36}optional semantic tie-breaker
    {cmd:tolerance(}{it:#}{cmd:)}{col 36}PCG tolerance override; phase defaults are documented below
    {cmd:maxiter(}{it:#}{cmd:)}{col 36}maximum PCG iterations; default 10,000

  {ul:Component inference and fixed-effect projections}
    {cmd:inference(none|highrank|q1)}{col 36}component covariance and intervals; default none
    {cmd:level(}{it:#}{cmd:)}{col 36}component/projection confidence level; default 95
    {cmd:inferencesimulations(}{it:#}{cmd:)}{col 36}component variance simulations; default 1,000
    {cmd:inferenceseed(}{it:#}{cmd:)}{col 36}component-inference seed; default 8675309
    {cmd:inferencebins(}{it:#}{cmd:)}{col 36}component smoothing resolution; default 1,000
    {cmd:project(}{it:varlist}{cmd:)}{col 36}covariates plus an automatic constant
    {cmd:projecteffect(worker|firm)}{col 36}dimension to project; required with project()
    {cmd:projectweight(frequency|target)}{col 36}projection weighting; default frequency

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
Omitting {cmd:backend()} is equivalent to {cmd:backend(auto)}.  Both prefer
Rust when the complete effective request passes native preflight.  A missing
plugin or structurally unsupported request may fall back to Mata only before
native preparation and estimator RNG.  The fallback is recorded in
{cmd:e(backend_fallback)}, {cmd:e(backend_fallback_reason)}, and
{cmd:e(backend_fallback_phase)}.

{pstd}
Omitted {cmd:rng()} means {cmd:rng(auto)}: Counter-V1 is selected on Rust and
the Stata RNG contract is selected on Mata.  Explicit {cmd:rng(counter_v1)}
pins a strict Rust request and disables Mata fallback.  Explicit
{cmd:rng(stata)} selects Mata and conflicts with {cmd:backend(rust)}.
{cmd:backend(mata)} always selects Mata; {cmd:backend(rust)} is strict.

{pstd}
The Rust backend exposes three result families:

{phang}
{cmd:backend(rust) algorithm(exact)} runs deterministic dense exact
estimation.  It accepts {cmd:engine(auto|generic)} and the ordinary control,
factor-variable, match/observation deletion, joint/fixed-offset, frequency
weight, stored target weight, {cmd:if}/{cmd:in}, and deletion-ID inputs.
Counter-V1 consent is not required: exact consumes no estimator RNG and
records the selected RNG contract as not applicable.

{phang}
The frozen compressed JLA form requires explicit
{cmd:backend(rust) rng(counter_v1) algorithm(jla)}
{cmd:preconditioner(diagonal) batch(}{it:#}{cmd:)} and
{cmd:engine(auto|compressed)}.  It remains limited to match deletion, joint
nuisance handling, movers, and no controls.  It supports {cmd:if}/{cmd:in},
frequency and stored target weights, deletion IDs, and the ordinary seed,
probe, tolerance, iteration, and memory options.

{phang}
The generic JLA form requires the fully explicit tuple
{cmd:backend(rust) rng(counter_v1) algorithm(jla) engine(generic)}
{cmd:preconditioner(diagonal) batch(}{it:#}{cmd:)}.  It supports up to 32
materialized nonomitted controls, including factor-variable columns; match or
observation deletion; joint or fixed-offset nuisance handling; frequency and
stored target weights; {cmd:if}/{cmd:in}; and deletion IDs for match deletion.
For match deletion it also supports {cmd:stayers(both)} through the combined
mover-match/stayer-observation correction.  The compressed JLA specialization
remains mover-only.

{pstd}
Planned Rust JLA supports automatic compressed/generic representation,
diagonal/CMG preconditioning, and automatic batching for admitted effective
tuples.  Explicit {cmd:algorithm(auto) engine(auto)} may select the exact
result family before estimator RNG.  {cmd:probeorder()} is a supported
semantic tie-breaker for mover-only Rust JLA.  Exact and generic JLA
{cmd:stayers(both)} routes use the versioned native augmentation lifecycle.
On qualified macOS and Linux builds, the no-control
match/joint/movers JLA cell with {cmd:engine(auto)},
{cmd:preconditioner(auto)}, {cmd:batch(auto)}, and an explicit
{cmd:probeorder()} selects {cmd:CMG_FULL_V2} through either strict
{cmd:backend(rust) rng(counter_v1)} or automatic
{cmd:backend(auto) rng(auto)} routing.  Other requests retain their existing
routes.  Counter-V1 JLA never changes the caller's Stata RNG.  The default
full-CMG fit and probe tolerances are {cmd:1e-10} and {cmd:1e-6}; an explicit
{cmd:tolerance()} overrides both.  Failed columns are deterministically
re-solved only on the frozen full-CMG route.  These alpha routes make no
Windows, license, or public-release claim.  Component-inference requests select
the capability-gated Mata exact runtime described below.  Projection requests
also select Mata exact unless they use the explicit scalable tuple documented
below.  Point-only calls do not post {cmd:e(V)}.

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
With match deletion, {cmd:stayers(both)} is the default, matching the current
MATLAB package.  It uses one pooled mover-stayer fit and target normalization,
deletes retained mover matches as blocks, and deletes eligible stayer
observations one literal physical copy at a time.  Thus the mover part uses
the declared match-dependence convention, while the stayer part is explicitly
{it:not} match-robust.  Specify {cmd:stayers(movers)} to recover the
mover-only fit, target, correction, and estimation sample.

{pstd}
Controls are nuisance coefficients and have zero weight in the four KSS
target matrices.  The corrected worker-firm total is therefore not a
KSS-corrected decomposition of the controls.  The separate full-model
explained variance is descriptive: it equals frequency-weighted
{cmd:Var(Y)} minus {cmd:e(weighted_rss)/e(N_physical)} and includes controls.

{marker output}
{title:Reading the output}

{pstd}
The header reports retained versus requested rows, literal physical
observations when frequency weights make them differ from stored rows,
worker and firm levels, deletion units, the target population, and the
selected algorithm and backend.  JLA calls additionally show the engine,
preconditioner, probes, and seed.  Sample pruning, automatic backend fallback,
solver fallback, and mixed mover/stayer deletion generate visible notes only
when relevant.

{pstd}
{ul:Additive worker-firm decomposition} is the primary applied-user table.
Its rows are worker variance, firm variance, sorting
({cmd:2 x worker-firm covariance}), and their total.  Its columns report the
plug-in value, estimated bias, KSS-corrected value, and corrected percentage
of target-weighted outcome variance.  The displayed identity is
{cmd:corrected = plug-in - estimated bias}.

{pstd}
Negative corrected components, negative sorting, and shares outside zero to
100 percent are possible and can be economically meaningful.  A share is
missing when target-weighted outcome variance is nonpositive.

{pstd}
JLA calls show numerical MCSE for worker variance, firm variance, sorting,
and the total.  Sorting MCSE is twice the raw covariance MCSE.  These values
describe randomized numerical error conditional on the realized leverage
sketch.  They are not sampling standard errors and exclude first-pass sketch
uncertainty.

{pstd}
When component inference or projection inference is requested, its estimate,
KSS standard error, p-value, and confidence interval remain in the default
output.  Rank-one requests also report the weak-identification intervals and
diagnostics.  Point-only and projection-only calls do not post component
{cmd:e(V)}; projection covariance is stored separately.

{marker postestimation}
{title:Postestimation display}

{pstd}
The compact output is backed by four read-only {cmd:estat} views.  They do not
change the estimates or stored results.

{phang}
{cmd:estat decomposition} redisplays the compact additive table.
{cmd:estat decomposition, full} adds the raw covariance targets, additive
bias accounting, plug-in and corrected shares relative to outcome variance
and the worker-firm total, and descriptive full-model fit.

{phang}
{cmd:estat sample} reports requested, complete-case, component, mover, and
retained rows; physical mass; stayer inclusion; retained dimensions; target
mass; and the match-graph pruning certificate when applicable.

{phang}
{cmd:estat computation} reports backend and RNG routing, the selected
algorithm, engine, preconditioner, JLA work and solver settings, fallback,
processor count, and memory envelope.

{phang}
{cmd:estat diagnostics} reports leverage, conditioning and residual
certificates when applicable, outcome and residual variance, memory forecast,
stage timings when available, and JLA numerical MCSE.  Use
{cmd:ereturn list} for the complete machine-readable record.

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
The match graph is constructed from movers.  The command selects a largest connected
component, removes stayers from that graph, removes insufficient histories
and worker articulation vertices, and repeatedly removes deletion-unit
bridges until reaching a fixed point.  Parallel deletion IDs at one
worker-firm coordinate remain distinct multigraph edges.  A successful match
sample has a final zero-bridge certificate.  A tied component ranking is
withheld rather than broken using arbitrary encoded IDs.

{pstd}
For {cmd:stayers(both)}, let M be exactly those final mover rows.  The
estimation sample is M plus workers who were one-firm stayers in
the original frozen complete-case sample, whose firm is represented in M,
and whose frequency-weighted physical history has at least two observations.
An original mover removed by graph or component selection is never
reclassified as a stayer.  A stayer on an unretained firm and a one-copy
stayer are excluded.  The combined model is refit on this combined sample;
its target shares use the combined pooled target mass for all four targets.
The main matrices and {cmd:e(b)} report this combined target, and
{cmd:e(sample)} marks both M and the eligible attached stayers.  Graph-pruning
diagnostics continue to describe the mover graph.  If no stayer is eligible,
the combined convention reduces exactly to the mover result.

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
Submitted numeric or factor-variable controls must have an identified,
numerically stable materialized span.
The command does not silently drop ordinary zero or collinear variables.
Only factor-variable terms explicitly marked omitted by Stata are removed.
The explicit generic Rust JLA route supports at most 32 materialized controls
and applies a fail-closed control and deleted-rank certificate.

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
Omitting {cmd:algorithm()} selects MATLAB-like {cmd:algorithm(jla)} with 200
probes.  Explicit {cmd:algorithm(auto)} chooses exact when the identified
dimension is within {cmd:exact_limit()} and JLA otherwise.  Automatic JLA
routing uses the compressed engine only for an exactly representable
no-control match design; other supported designs use the generic engine.
{cmd:preconditioner(auto)} chooses diagonal or package-owned CMG from
structural preflight before the estimator random stream begins.  Rust resolves
the same frozen plan through versioned capability and plan receipts.

{pstd}
{cmd:stayers(both)} requires {cmd:deletion(match)}.  It is implemented by
Mata and Rust for exact and generic JLA calculations; JLA automatically
bypasses the mover-only compressed engine.  {cmd:probeorder()} and
{cmd:wallseconds()} are not supported on the current mixed Rust route.  If the
combined design or any required mover-match or stayer-observation deletion
fails a rank, convergence, or numerical gate, the complete request is
withheld.

{pstd}
The complete direct-peak forecast is the memory admission gate; percentage and processor rules are
only automatic batch-width heuristics.  Structural preflight, not routing trial solves, selects the
automatic preconditioner.  Reported setup and fit are disjoint timings, so setup is not counted twice.

{pstd}
The fixed seed is tied to the runtime contract and canonical semantic atom
order.  Supported batching and solver routes do not change those atoms.  The
command restores the caller's RNG algorithm, stream, complete state, sort
jumbler, data, and estimation sample on every supported exit.

{marker inference}
{title:Component inference and fixed-effect projections}

{dlgtab:Component inference}

{pstd}
Inference is opt-in.  {cmd:inference(highrank)} posts a joint econometric
covariance for the four established targets and ordinary Wald intervals.
{cmd:inference(q1)} additionally reports rank-one weak-identification
diagnostics and Anderson--Rubin-style interval endpoints.  Point-only calls
retain their previous behavior and do not post {cmd:e(V)}.

{pstd}
Component inference requires the Mata exact route,
{cmd:deletion(observation)}, {cmd:stayers(movers)}, and unit frequency
weights.  Omitted or automatic algorithm selection resolves to exact for an
inference request.  Rust/JLA component inference, match-cluster inference,
frequency-weight inference, and stayer-hybrid inference are rejected with
typed statuses.  The outer command restores the caller's complete RNG state.

{pstd}
The high-rank procedure forms the KSS observation-level variance proxy,
smooths it over leverage and target diagonal separately for movers and
stayers, and simulates the variance of the corrected quadratic.  Pairwise
polarization produces a three-target covariance, which is mapped to the total
through {cmd:total=worker+firm+2*covariance}.  Materially negative fitted
variances or indefinite covariances withhold the entire request.  Only tiny
roundoff values may be set to zero, and these adjustments are stored in
{cmd:e(inference_diagnostics)}.

{pstd}
Accepted component inference supports Stata's standard {cmd:lincom} because
the four coefficient names and their joint covariance are posted in
{cmd:e(b)} and {cmd:e(V)}.  For example:

{phang2}{cmd:. lincom worker_variance + firm_variance + 2*worker_firm_covariance}{p_end}

{pstd}
This component combination is distinct from MATLAB's {cmd:lincom_KSS}, which
computes fixed-effect projection inference.  The FEVC counterpart to that
MATLAB function is {cmd:project()}.

{dlgtab:Fixed-effect projection inference}

{pstd}
{cmd:project()} projects the worker or firm effects selected by
{cmd:projecteffect()} on an automatic constant and numeric covariates.
{cmd:projectweight(frequency)} is the default; {cmd:projectweight(target)}
uses target mass.  Projection coefficients and KSS/naive covariances are
stored under {cmd:e(projection_*)}.  Projection alone does not populate the
component {cmd:e(V)}, and Stata's standard {cmd:lincom} therefore does not
operate on projection rows directly.  The naive covariance is a descriptive
residual-squared plug-in benchmark; use the KSS covariance for reported
projection inference.

{pstd}
A projection-only call with no explicit scalable Rust tuple selects the
deterministic Mata exact route.  It can use the default match-deletion,
combined mover/stayer population or explicit observation deletion.  Component
inference and projection inference can be requested together only on their
common supported surface: Mata exact, observation deletion, movers, and unit
frequency weights.

{pstd}
Projection inference inherits the point estimator's deletion and population
contract.  With omitted {cmd:deletion()}, declared mover matches are
independent blocks with unrestricted within-match covariance, while eligible
attached stayers remain physical-observation deletion units under default
{cmd:stayers(both)}.  The covariance uses symmetrized block cross-fit
products.  Explicit observation deletion reduces to the valid uncentered
{cmd:y_i*e_i,-i} product; the old sample-mean-centered product is not used.

{pstd}
The automatic constant uses last-retained-firm-zero grounding and is
normalization-dependent.  Projection slopes are invariant to equivalent
worker/firm location shifts.

{pstd}
The scalable projection route is deliberately explicit.  It requires
{cmd:backend(rust) rng(counter_v1) algorithm(jla)}, the generic engine
(explicitly or by automatic selection), observation or match deletion,
explicit {cmd:preconditioner(diagonal)} or forced
{cmd:preconditioner(cmg)}, and positive integer frequency
weights interpreted as literal physical copies.  Automatic solver routing is
not admitted for {cmd:project()}.  Forced projection CMG shares the planned
generic hierarchy between the full and fixed-effect solvers and fails closed;
it is distinct from the specialized match-deletion {cmd:CMG_FULL_V2} route.
For match deletion the explicit generic route also supports the default
{cmd:stayers(both)} mixed deletion partition.
Target mass remains stored-row mass and is not multiplied by frequency.  The
native runtime obtains the requested block variance proxy from the same JLA solve,
solves the fixed-effect projection loadings without a full inverse, and
streams the score covariance without retaining an observation-by-coefficient
design.  Complete-system residual, projection-Gram conditioning, PSD, and
memory gates are fail closed.

{phang2}{cmd:. fevc wage i.year, worker(id) firm(fid) ///}{p_end}
{phang3}{cmd:deletion(observation) inference(highrank)}{p_end}

{phang2}{cmd:. fevc wage i.year, worker(id) firm(fid) ///}{p_end}
{phang3}{cmd:deletion(observation) inference(q1) level(95)}{p_end}

{phang2}{cmd:. fevc wage i.year, worker(id) firm(fid) ///}{p_end}
{phang3}{cmd:project(education experience) ///}{p_end}
{phang3}{cmd:projecteffect(firm) projectweight(frequency)}{p_end}

{phang2}{cmd:. fevc wage i.year, worker(id) firm(fid) ///}{p_end}
{phang3}{cmd:deletion(observation) inference(highrank) ///}{p_end}
{phang3}{cmd:project(education experience) projecteffect(firm)}{p_end}

{phang2}{cmd:. fevc wage i.year, worker(id) firm(fid) ///}{p_end}
{phang3}{cmd:project(education experience) ///}{p_end}
{phang3}{cmd:projecteffect(firm) backend(rust) rng(counter_v1) ///}{p_end}
{phang3}{cmd:algorithm(jla) engine(generic) preconditioner(cmg)}{p_end}

{pstd}
These procedures follow the published KSS formulas and maintained MATLAB
behavior, but are independently authored GPL-3.0-only source.  No MATLAB
source or critical-value table is distributed.  The binned local-linear
calculation is a high-rank approximation, not a fully unbiased leave-three-out
variance estimator.

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
    Unsupported stayer convention{col 34}use match deletion and exact or generic JLA, or request stayers(movers)
    Unsupported component route{col 34}use Mata exact observation deletion with unit frequency weights
    Invalid inference covariance{col 34}inspect leverage, support, smoothing fit, and projection rank
    PCG nonconvergence{col 34}check scaling/connectivity, maxiter(), and solver route
    Memory admission{col 34}reduce batch width or declare only actually available memory
    Forced compressed failure{col 34}use the full explicit generic tuple or backend(mata)

  {ul:Installation and runtime}
    Stale Mata runtime{col 34}run discard or restart Stata, then reinstall one complete build
    Unregistered JLA runtime{col 34}use supported Stata 18/19 or exact when feasible
    Unavailable Rust artifact{col 34}build and test a local developer artifact or use backend(mata)
    Unsupported Rust options{col 34}use exact, frozen compressed JLA, explicit generic JLA, or backend(mata)
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
Accepted {cmd:inference(highrank|q1)} calls post {cmd:e(V)} for the same four
targets.  {cmd:e(V_primitive)} contains the worker, firm, and covariance
block; {cmd:e(component_inference)} contains estimates, standard errors, and
Wald endpoints.  {cmd:inference(q1)} also stores
{cmd:e(q1_inference)} with weak-identification endpoints, eigen diagnostics,
rank-one covariance terms, F statistic, curvature, and critical value.

{pstd}
A projection request stores {cmd:e(projection_b)},
{cmd:e(projection_V)}, {cmd:e(projection_V_naive)}, and
{cmd:e(projection_results)}.  The columns of {cmd:e(projection_results)} are
the estimate, KSS standard error, z statistic, p-value, lower and upper
confidence endpoints, and naive standard error.  The scalable Rust route
additionally stores
{cmd:e(projection_diagnostics)},
{cmd:e(projection_augmentation_receipt)}, and
{cmd:e(projection_solver_diagnostics)}.  These bind the projection Gram,
coefficient solves, complete-system residuals, covariance PSD cleanup, proxy
range, and admitted memory forecast.  For Mata exact projection,
{cmd:e(inference_diagnostics)} records applicable component-inference
settings, confidence level, covariance cleanup magnitudes, variance-proxy
range, mover/stayer row counts, and tiny fitted-variance floor count.

{pstd}
{cmd:e(inference_deletion)}, {cmd:e(inference_method)},
{cmd:e(projection_constant)}, and {cmd:e(grounding_convention)} identify the
effective deletion partition, exact or JLA block route, and normalization.

{pstd}
{cmd:e(decomposition)} is the additive applied-user view.  Its rows are
{cmd:worker_variance}, {cmd:firm_variance},
{cmd:sorting_2covariance}, and {cmd:total_worker_firm}.  Its columns contain
plug-in, bias correction, corrected levels, plug-in/corrected shares of
target-weighted outcome variance, and plug-in/corrected shares of the
worker-firm total.  Stored shares are proportions; the display multiplies
them by 100.

{pstd}
With {cmd:stayers(both)}, the ordinary headline returns have the combined
mover-stayer meaning.  Compatibility aliases are
{cmd:e(stayer_hybrid_results)}, {cmd:e(stayer_hybrid_plugin)},
{cmd:e(stayer_hybrid_correction)}, and {cmd:e(stayer_hybrid_kss)}.
{cmd:e(stayer_hybrid_decomposition)} is the corresponding additive/share
view.  {cmd:e(stayer_hybrid_correction_source)} has rows
{cmd:mover_match} and {cmd:stayer_observation}, making the mixed correction
accounting explicit for exact calculations; JLA reports that source split as
missing because its finite-projection correction is estimated jointly.
{cmd:e(stayer_hybrid_sample_accounting)} has mover,
stayer, and total rows and reports stored rows, physical observations, worker
levels, target mass, and deletion units.

{pstd}
The associated scalars report the combined dimensions and numerical
diagnostics, the number of included stayers and their stored/physical mass,
excluded singleton and unattached stayer counts, and mover/stayer target
mass.  The labels {cmd:e(stayer_hybrid_target_population)},
{cmd:e(stayer_hybrid_deletion)}, {cmd:e(stayer_hybrid_assumption)}, and
{cmd:e(stayer_hybrid_esample)} record the population, mixed-deletion
convention, lack of match robustness for stayers, and the combined meaning
of {cmd:e(sample)}.  Exact calculations additionally preserve the mover-only
intermediate under {cmd:e(mover_results)}, {cmd:e(mover_plugin)},
{cmd:e(mover_correction)}, and {cmd:e(mover_kss)}.

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
Successful estimates store {cmd:e(estat_cmd)="fevc_estat"}, which makes the
four postestimation display commands documented above available through
Stata's standard {cmd:estat} dispatcher.

{pstd}
Backend routing is recorded in {cmd:e(backend_requested)},
{cmd:e(backend_selected)}, {cmd:e(backend_routing_reason)}, and
{cmd:e(backend_option_supplied)}.  The last is zero only when
{cmd:backend()} was omitted.  Automatic fallback additionally records
{cmd:e(backend_fallback)}, {cmd:e(backend_fallback_reason)}, and
{cmd:e(backend_fallback_phase)}.  RNG routing is recorded analogously in
{cmd:e(rng_requested)}, {cmd:e(rng_selected)}, and
{cmd:e(rng_option_supplied)}.  Rust results additionally include a
request-capability receipt and route-appropriate preparation, graph, memory,
residual, rank, and accounting diagnostics.  JLA results also include
per-right-hand-side receipts, topology checksum halves, and the Counter-V1
contract; exact results record RNG as not applicable.  On explicit Rust
failure, {cmd:e(backend_selected)} is empty and the routing fields accompany
the typed withholding result.

{pstd}
Planned Rust results additionally store {cmd:e(rust_phase_profile)} with
columns {cmd:ingest}, {cmd:canonicalize}, {cmd:graph}, {cmd:compress},
{cmd:plan}, {cmd:stayer_augmentation}, {cmd:solve}, and {cmd:native_total}.
Units are seconds and the schema is recorded in
{cmd:e(rust_phase_profile_schema)}.  These are diagnostic wall-clock values;
they never affect estimator selection, memory admission, RNG, or results.

{pstd}
On a recognized failure, the principal strings are
{cmd:e(withholding_status)}, {cmd:e(withholding_detail)},
{cmd:e(withholding_reason)}, and {cmd:e(withholding_suggestion)}.

{marker examples}
{title:Examples}

{pstd}
Each example creates its own connected AKM-style worker-firm graph.  The
visible {cmd:preserve}/{cmd:restore} lines make the block safe to copy into a
do-file.  The clickable link executes the marked inner block through
{cmd:fevc_run}, which also restores the caller's data.  The first three
examples display the population worker and firm variances, worker-firm
covariance, and total implied by their DGP before estimation.

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
          fevc log_wage productivity i.period, worker(worker_id) firm(firm_id) ///
              deletion(match) deletionid(match_id) nuisance(joint) algorithm(exact)
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata fevc_run exact_controls using fevc.sthlp:click to run})}

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
          fevc log_wage productivity i.period, worker(worker_id) firm(firm_id) ///
              deletion(match) deletionid(match_id) nuisance(joint) algorithm(jla) ///
              probes(40) batch(8) seed(8675309) engine(auto) preconditioner(auto)
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata fevc_run jla_controls using fevc.sthlp:click to run})}

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
          fevc log_wage productivity [fw=frequency], worker(worker_id) firm(firm_id) ///
              deletion(match) deletionid(match_id) nuisance(fixedoffset) ///
              targetweight(target_mass) algorithm(exact)
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata fevc_run weights_targets using fevc.sthlp:click to run})}

{space 4}{hline 10} {it:Example 4 - Component inference and lincom} {hline 10}
{cmd}{...}
          preserve
{* example_start - component_inference}{...}
          clear
          set obs 24
          generate long worker_id = floor((_n-1)/4)
          generate byte period = mod(_n-1,4)
          generate double productivity = period-1.5
          generate double policy = period==2
          generate byte firm_id = .
          generate double noise = .
          local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
          local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
          forvalues row = 1/24 {
              local value : word `row' of `firms'
              quietly replace firm_id = `value' in `row'
              local value : word `row' of `noises'
              quietly replace noise = `value' in `row'
          }
          generate double log_wage = 1.5+.3*worker_id-.2*firm_id+.4*productivity-.15*policy+noise
          fevc log_wage productivity policy, worker(worker_id) firm(firm_id) ///
              deletion(observation) inference(highrank) ///
              inferencesimulations(100) inferenceseed(42) inferencebins(16)
          lincom worker_variance+firm_variance+2*worker_firm_covariance
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata fevc_run component_inference using fevc.sthlp:click to run})}

{pstd}
For rank-one weak-identification intervals, replace
{cmd:inference(highrank)} with {cmd:inference(q1)}.  The command then reports
both the ordinary component table and the rank-one interval table.

{space 4}{hline 10} {it:Example 5 - Firm-effect projection with movers and stayers} {hline 10}
{cmd}{...}
          preserve
{* example_start - projection_inference}{...}
          clear
          set obs 28
          generate long worker_id = .
          generate byte firm_id = .
          generate long match_id = .
          generate double productivity = .
          generate double policy = .
          local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
          local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
          forvalues row = 1/24 {
              quietly replace worker_id = floor((`row'-1)/4) in `row'
              local value : word `row' of `firms'
              quietly replace firm_id = `value' in `row'
              local value : word `row' of `matches'
              quietly replace match_id = `value' in `row'
              quietly replace productivity = mod(`row'-1,4)-1.5 in `row'
              quietly replace policy = mod(`row',3)==0 in `row'
          }
          quietly replace worker_id = 100 in 25/26
          quietly replace firm_id = 0 in 25/26
          quietly replace worker_id = 101 in 27/28
          quietly replace firm_id = 2 in 27/28
          quietly replace match_id = 1000+_n in 25/28
          quietly replace productivity = -.4+.25*(_n-25) in 25/28
          quietly replace policy = mod(_n,2) in 25/28
          generate double target_mass = 1+mod(_n,5)/7
          generate double projection_z = sin(_n/3)
          generate double log_wage = -48.8+.07*worker_id-.11*firm_id ///
              +.35*productivity-.2*policy+sin(_n)/20
          fevc log_wage productivity policy, worker(worker_id) firm(firm_id) ///
              deletionid(match_id) targetweight(target_mass) algorithm(exact) ///
              project(projection_z) projecteffect(firm) projectweight(target)
          estat sample
{* example_end}{...}
          restore
{txt}{...}
{space 4}{hline 76}
{space 4}{it:({stata fevc_run projection_inference using fevc.sthlp:click to run})}

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
Version 0.5.0-alpha.1 is public-source prerelease software.  Covered
implementation source is GPL-3.0-only, and the documented human
package-boundary and provenance review is complete.  No public package release,
tag, or native binary distribution has yet been issued.
Point estimates remain the default.  Component inference remains limited to
the exact observation-deletion assumptions documented above; the explicit
scalable projection route uses the same corrected observation-or-match block
estimand as exact Mata on its qualified generic-JLA surface.  Neither is a substitute for an
application-specific assessment of dependence and identification.

{marker also}
{title:Also see}

{p 0 24}
Online: {help estat}, {help lincom}, {help regress}, {help xtreg},
{help areg}, {help fvvarlist}, {help weights}
{p_end}
{.-}
