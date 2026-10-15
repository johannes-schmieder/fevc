{smcl}
{* *! version 0.5.0 14oct2026}{...}
{.-}
help for {cmd:fevc} {right:(Johannes F. Schmieder)}
{.-}

{title:Title}

{pstd}
{cmd:fevc} {hline 2} Leave-out bias-corrected variance decompositions
for two-way fixed-effect models

{pstd}
{cmd:fevc} estimates the variance of worker effects, the variance of firm
effects, their covariance, and the variance of their sum. It corrects the
bias that arises because individual fixed effects are estimated with error,
using the method of Kline, Saggio, and Sølvsten (2020). The two dimensions
can also represent other linked groups, such as patients and physicians.
For methodological details, see the {help fevc##reference:companion fevc paper}.

{pstd}
{help fevc##quickstart:Example} | {help fevc##syntax:Syntax} |
{help fevc##options:Main options} | {help fevc##sample:Leave-out sample} |
{help fevc##centering:Outcome centering} |
{help fevc##advanced:Advanced options} | {help fevc##examples:More examples}

{marker quickstart}
{title:Start with an example}

{pstd}
This example creates 60,000 observations on 10,000 workers and 3,001 firms,
with positive sorting between worker and firm effects. The helper prints
the true variance components before estimation. With the default seeds,
the correction brings the estimates closer to those true values.

{cmd}{...}
        preserve
{* example_start - jla_controls}{...}
        fevc, simulate_data(ex1) clear
        fevc log_wage productivity i.period, ///
            worker(worker_id) firm(firm_id)
        estat decomposition, full
        matrix list e(mcse)
        estat diagnostics
{* example_end}{...}
        restore
{txt}{...}
{pstd}
{stata fevc_run jla_controls using fevc.sthlp:Click to run this example}
(your data are restored afterward).
{stata viewsource fevc__simulate_data.ado:View the data-generation code}.

{pstd}
With your own data, the basic command is
{cmd:fevc log_wage, worker(worker_id) firm(firm_id)}.
Put additional controls after the outcome, as in the example; Stata
factor-variable notation such as {cmd:i.year} is supported. Controls enter
the regression, but the corrected components concern the worker and firm
effects. Computational settings are selected automatically.

{marker syntax}
{title:Syntax}

{p 8 16 2}
{cmd:fevc} {it:depvar} [{it:controls}]
[{help if}] [{help in}] [{it:weight}],
{cmd:worker(}{it:varname}{cmd:)} {cmd:firm(}{it:varname}{cmd:)}
[{it:options}]

{pstd}
Only frequency weights ({cmd:[fw=}{it:frequency}{cmd:]}) are supported.

  {it:Main options}{col 40}Description
  {hline 76}
    {cmd:worker(}{it:varname}{cmd:)}{col 40}worker identifier (required)
    {cmd:firm(}{it:varname}{cmd:)}{col 40}firm identifier (required)
    {cmd:deletion(match|observation)}{col 40}unit left out; default match
    {cmd:deletionid(}{it:varname}{cmd:)}{col 40}optional match identifier
    {cmd:stayers(both|movers)}{col 40}include eligible stayers or movers only
    {cmd:probes(}{it:#}{cmd:)}{col 40}approximation precision; default 200
    {cmd:centering(none|mean|corrected)}{col 40}outcome centering; default mean
    {cmd:mcse(all|off)}{col 40}numerical MCSE; default all
    {cmd:seed(}{it:#}{cmd:)}{col 40}estimation seed; default 8675309
    {cmd:nolog}{col 40}suppress progress messages
  {hline 76}

{marker options}
{title:Main options}

{phang}
{cmd:worker()} and {cmd:firm()} identify the two fixed-effect dimensions.
Repeated observations of a worker at a firm are allowed; a separate
identifier for each observation is not required.

{phang}
{cmd:deletion(match)}, the default, leaves out an entire worker-firm match
when correcting mover contributions. It allows errors to be correlated
within a match and assumes independence across matches, including different
matches of the same worker. It is not worker-level clustering.
{cmd:deletion(observation)} instead leaves out one observation at a time
and assumes independent errors across observations. Choose this option
according to the dependence in your data.

{phang}
{cmd:deletionid()} is needed only when the blocks of observations whose
errors may be correlated differ from the default worker-firm pairs, for
example when distinct employment spells at the same firm are treated as
independent. Each ID must belong to one worker-firm pair, and the option
applies to match deletion. Errors are assumed independent across IDs;
repeated spells with the same ID remain one block. With {cmd:deletionid()},
a worker counts as a mover when the worker has more than one ID, even if
the IDs share a firm in the model (for example, when small employers are pooled
into one firm group). Without the option, a mover is a worker observed at
more than one firm. Having two IDs does not by itself guarantee that each
deletion leaves the model identified.

{phang}
{cmd:stayers(both)}, the default, includes movers and eligible stayers.
Under match deletion, a stayer has a single match (one {cmd:deletionid()}
value, or one firm by default), so stayer contributions are corrected one
observation at a time. This correction assumes
independent errors across a stayer's observations and is {it:not} robust to
correlation within the stayer's match. Use {cmd:stayers(movers)} for a
decomposition restricted to movers; this changes the estimation sample and
the population described by the estimates. Under observation deletion, all
retained observations are corrected one at a time. In either case, every
deletion must leave the model, including the controls, identified.

{phang}
{cmd:probes()} controls the precision of the default randomized calculation
(the Johnson-Lindenstrauss approximation, JLA). More probes reduce numerical
approximation error but take longer. Start with the default 200; increase it
if the reported numerical MCSE is large relative to the components you wish
to interpret.

{phang}
{cmd:mcse(all)}, the default, reports a numerical Monte Carlo standard error
(MCSE) for each of the four main estimates. Holding the data fixed, it
approximates how much the estimates would change if the random directions
used by JLA were drawn again. The numerical MCSE is {bf:not a sampling standard error}:
it excludes the bias that remains with a finite number of probes,
solver and rounding error, and sampling uncertainty. Under
{cmd:centering(mean)} it treats the observed outcome mean as fixed; under
{cmd:centering(corrected)} it also treats the added centering adjustment as
fixed and so excludes the numerical uncertainty in that adjustment (see
{help fevc##centering:Outcome centering}). Exact calculation reports an MCSE
of zero. If the diagnostic cannot be computed reliably, the MCSE is withheld
(missing) while the point estimates remain valid; the raw numerical
covariance is kept when it was computed.

{phang}
{cmd:mcse(off)} skips this calculation; the MCSE results are then missing,
not zero. {cmd:numericalmcse()} is accepted as a synonym for {cmd:mcse()};
specifying both is an error. The developer option {cmd:mcse(conditional)}
reports a narrower diagnostic that holds the leverage directions fixed.

{phang}
The numerical MCSE is also reported when {cmd:project()} or an inference
option is requested, but it describes only the four main point estimates.
It is not computed for projection coefficients, component standard errors
or confidence intervals, and it is never posted as {cmd:e(V)}. The main display
and {cmd:estat diagnostics} show whether it is available.

{phang}
{cmd:seed()} makes the randomized calculation reproducible within the
selected backend. The default is 8675309. The Mata and Rust backends use
different random-number generators, so the same seed need not give identical
estimates in both. Estimation restores the caller's random-number state.

{phang}
{cmd:nolog} hides progress messages while retaining the final results.

{marker sample}
{title:Which observations are used? The leave-out sample}

{pstd}
The correction requires enough connections between workers and firms for
the model to remain identified when a match or observation is removed.
{cmd:fevc} starts from complete cases satisfying your {cmd:if}/{cmd:in}
restrictions, selects a largest connected component, and removes observations
that do not meet its leave-out requirements. Merely restricting the data to
a connected component beforehand need not be sufficient.

{pstd}
Under default match deletion, the command first constructs the retained
mover sample and then adds eligible stayers: workers with a single match
in your data (one {cmd:deletionid()} value, or one firm by default),
employed at a retained firm, and with at least two observations (counting
frequency weights). Movers removed during sample
construction are not reclassified as stayers. All eligibility and
observation counts use the requested {cmd:if}/{cmd:in} sample; observations
outside it never enter {cmd:e(sample)}. Controls must also remain identified
under the requested deletions; if that cannot be established, the command
reports a failure.

{pstd}
The decomposition describes the {bf:retained sample}, which can differ from
your original data. The output reports sample counts. Use {cmd:estat sample}
to inspect exclusions and {cmd:e(sample)} to identify retained observations:

{phang2}{cmd:. estat sample}{p_end}
{phang2}{cmd:. generate byte fevc_sample = e(sample)}{p_end}

{marker output}
{title:Reading the results}

{pstd}
The main table shows the uncorrected (plug-in) estimate, estimated bias,
corrected estimate, and corrected share of outcome variance:

{p 8 12 2}
KSS corrected = plug-in - estimated bias

{pstd}
The sorting row is {bf:twice the worker-firm covariance}, so the worker
variance, firm variance, and sorting contribution add to the total
worker-firm variance. {cmd:estat decomposition, full} also shows the raw
covariance. Negative corrected variances can occur in finite samples;
covariance and sorting may be negative. Shares need not lie between 0 and
100 percent.

{pstd}
With controls, this total concerns the two fixed effects; it is not a
bias-corrected decomposition of the controls or the full model's R-squared.
Point estimation remains the default. Standard errors and confidence
intervals require an explicit {help fevc##inference:inference request}.

{marker centering}
{title:Outcome centering}

{pstd}
{cmd:centering()} controls how the outcome enters the bias correction. The
fitted coefficients, residuals, plug-in components, retained sample, weights
and deletion units are the same under all three settings. In point
estimation, only the estimated bias, and hence the corrected estimate,
changes; with {cmd:project()} or an inference request, centering also
changes the standard errors. The {cmd:corrected} row of {cmd:e(results)}
always holds the final leave-out estimate, whatever the setting. The
default is {cmd:mean} for both exact and JLA calculation.

{phang}
{cmd:centering(mean)}, the default, subtracts the mean outcome of the
retained sample before computing the bias correction. Centering can make
the correction considerably more precise, particularly when the outcome
mean is far from zero, although it is not better in every case. Because the
sample mean depends on the same errors as the correction, it can introduce
a bias when error variances differ across observations. The mean counts frequency
weights but {bf:not} {cmd:targetweight()} or {cmd:projectweight()}. With
{cmd:nuisance(fixedoffset)}, it is taken after subtracting the fitted
controls. Centering adds only the calculation of one mean.

{phang}
{cmd:centering(none)} uses the uncentered estimator of the published KSS
article. Because the KSS Matlab package centers the outcome, {cmd:none} does
not reproduce that package.

{phang}
{cmd:centering(corrected)} keeps the centering and adds an adjustment for
the bias from estimating the mean. With exact calculation and jointly
estimated controls, this restores exact unbiasedness under the model's error
assumptions; the adjustment need not reduce mean squared error. It applies
to point estimates only. With exact calculation it
solves one additional linear system. With JLA it splits the existing probes
into two halves, so {cmd:probes()} must be even and at least 4; no extra
probes are drawn. For a small problem where the adjustment matters, use
{cmd:algorithm(exact) centering(corrected)}.

{pstd}
With {cmd:centering(corrected)} and JLA, the reported MCSE and numerical
covariance are those of {cmd:centering(mean)} for the same call and probes.
{bf:They exclude numerical uncertainty in the centering adjustment.}
The developer diagnostic {cmd:mcse(conditional)} follows the same convention.
With {cmd:mcse(off)}, the diagnostics are missing even in exact mode, but
the corrected point estimate is still computed.

{pstd}
Projections and component {cmd:inference(highrank|q1)} accept {cmd:mean}
(the default) and {cmd:none}, but not {cmd:corrected}; requesting
{cmd:corrected} with either stops the command before estimation. For
projections, centering changes the standard errors but not the coefficients.
For component inference, mean centering treats the observed mean as a known
constant and ignores the uncertainty from estimating it. This is an
approximation, not inference conditional on the observed mean. Component
inference and {cmd:project()} can be requested in the same call only with
exact Mata and without a structured model. A failed centering calculation
returns an error and never switches to another setting.

{pstd}
{cmd:e(centering)} and {cmd:e(mcse_centering)} record the choice and the
MCSE convention. With component inference, {cmd:e(inference_centering)} and
{cmd:e(inference_mean_omitted)} also record the inference convention. The
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/CENTERING.md":centering guide}
({cmd:fevc/docs/CENTERING.md} in the source repository) gives the formulas,
timing evidence and complete restrictions.

{marker postestimation}
{title:Postestimation display}

  {it:Command}{col 40}What it shows
  {hline 76}
    {cmd:estat decomposition}{col 40}main decomposition table
    {cmd:estat decomposition, full}{col 40}raw covariance, shares, and model fit
    {cmd:estat sample}{col 40}retained sample and exclusions
    {cmd:estat computation}{col 40}selected computation settings
    {cmd:estat diagnostics}{col 40}numerical and resource diagnostics
  {hline 76}

{marker advanced}
{title:Advanced options}

{pstd}
{bf:Most applications can leave these at their defaults.} The options below
support alternative estimands, inference, or specialized computation.
Change statistical options for a substantive reason; changing a numerical
setting does not resolve an identification or dependence problem.

{dlgtab:Alternative weighting and control treatment}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:targetweight(}{it:varname}{cmd:)}{col 40}weights for the variance components
    {cmd:nuisance(joint|fixedoffset)}{col 40}control treatment; default joint
  {hline 76}

{pstd}
Positive integer {cmd:fweight}s represent repeated observations and affect
the regression. {cmd:targetweight()} instead determines how retained rows
contribute to the variance components, and so the population the
decomposition describes. By default, target weights equal frequency weights
(or one without weights). Explicit target weights are the total weight of
each stored row and are {bf:not multiplied by frequency weights}.

{pstd}
{cmd:nuisance(joint)} re-estimates the control coefficients in each
leave-out calculation. {cmd:nuisance(fixedoffset)} estimates the controls
once, subtracts their fitted contribution, and then treats it as known.
This is a conditional convention for the point estimates, not joint
leave-out estimation. Controls must be identified; ordinary collinear
controls are not silently dropped. See the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/ESTIMATOR_CONTRACT.md":estimator reference}.

{marker inference}
{dlgtab:Component inference}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:inference(none|highrank|q1)}{col 40}component intervals; default none
    {cmd:inferencemodel(}{it:mode}{cmd:)}{col 40}optional structured variance model
    {cmd:level(}{it:#}{cmd:)}{col 40}confidence level; default 95
    {cmd:inferencesimulations(}{it:#}{cmd:)}{col 40}variance simulations; default 1,000
    {cmd:inferencegramprobes(}{it:#}{cmd:)}{col 40}Rust variance-model probes; default 2,048
    {cmd:inferenceseed(}{it:#}{cmd:)}{col 40}inference seed; default 8675309
    {cmd:inferencebins(}{it:#}{cmd:)}{col 40}target_lowess bins; default 1,000
  {hline 76}

{pstd}
{cmd:inference(highrank)} requests
normal-approximation (Gaussian) intervals for the four components. They
require that no single direction in the data dominate a component and that
individual observations have limited influence. {cmd:inference(q1)} allows
one dominant, weakly identified direction with a diffuse remainder; it does
not cover several such directions (multi-mode weakness). There is no
automatic choice between them, and successful computation does not
establish valid coverage. Both rely on a model for the error variances and
on assumptions beyond those needed for point estimates. Under the default
{cmd:centering(mean)}, the observed mean is treated as fixed, which requires
the uncertainty in estimating it to be negligible; use {cmd:centering(none)}
for the uncentered calculation.

{pstd}
Without {cmd:inferencemodel()}, component inference uses exact Mata
calculation, observation deletion, and unit frequency weights.
Observation-q1 calibration retains a documented limitation: in the
package's confirmation runs, its standard errors narrowly failed one
calibration check (see the inference guide). The explicit
structured models {cmd:structured_common} and {cmd:structured_leverage}
impose additional assumptions on the error variances; they are not
unrestricted heteroskedasticity-robust inference. {cmd:structured_common}
lets the variance depend on a richer set of design features;
{cmd:structured_leverage} is a simpler sensitivity specification. Neither is
chosen automatically.

{pstd}
{bf:Movers and stayers together (pooled match inference).} Use
{cmd:deletion(match)}, {cmd:nuisance(fixedoffset)}, a structured model and
the default {cmd:stayers(both)}, or {cmd:stayers(movers)} for movers only.
Errors may be correlated within a mover's match, but mover matches and the
individual observations of each stayer are assumed independent, including
repeated observations of one stayer and frequency copies of a stayer row.
The variance model has separate coefficients for movers and stayers.
With {cmd:deletionid()}, a worker with several IDs at one firm counts as a
mover. Fixed-offset match inference is approximate, ignoring
nuisance-control estimation uncertainty, and under the default centering it
also ignores the uncertainty in the observed mean.

{pstd}
{bf:Small and large datasets use different calculations.} For small designs
within the exact-size and memory limits, select
{cmd:backend(mata) algorithm(exact)}. For large datasets, select
{cmd:backend(rust) rng(counter_v1) algorithm(jla) engine(generic)} with
{cmd:preconditioner(diagonal)} or {cmd:preconditioner(cmg)}. Both accept the
two structured models, {cmd:deletionid()}, {cmd:targetweight()} and positive
integer frequency weights. Mata JLA does not support structured inference,
and structured inference cannot be combined with {cmd:project()} in the same
call. For example, with outcome {cmd:y}, control {cmd:x}, worker identifier
{cmd:id} and firm identifier {cmd:firmid}, a small-sample request is

{phang2}{cmd:fevc y x, worker(id) firm(firmid) deletion(match) nuisance(fixedoffset)}
{cmd:stayers(both) backend(mata) algorithm(exact) inference(highrank) inferencemodel(structured_common)}{p_end}

{pstd}
and a large-sample request is

{phang2}{cmd:fevc y x, worker(id) firm(firmid) deletion(match) nuisance(fixedoffset)}
{cmd:stayers(both) backend(rust) rng(counter_v1) algorithm(jla) engine(generic)}
{cmd:preconditioner(diagonal) inference(highrank) inferencemodel(structured_common)}{p_end}

{pstd}
Replace {cmd:inference(highrank)} by {cmd:inference(q1)} only when the
one-dominant-direction assumptions are appropriate. See the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/POOLED_COMPONENT_INFERENCE.md":pooled inference guide}.

{pstd}
The inference calculations have their own numerical settings.
{cmd:inferencesimulations()} sets the number of simulation draws used for the
covariance on both backends; {cmd:inferencegramprobes()} sets the random
probes Rust uses to fit the structured variance model; {cmd:inferencebins()}
applies to the observation-level smoothing method ({cmd:target_lowess}) used
without {cmd:inferencemodel()}. These are separate from the point-estimation
probes set by {cmd:probes()}, and the numerical MCSE does not cover them.

{pstd}
Gaussian results are stored in {cmd:e(component_inference)} and q1 results
in {cmd:e(q1_inference)}. A successful command can still withhold individual
intervals: {cmd:e(q0_status)} and {cmd:e(q1_status)} record availability,
with zero meaning computed. A withheld q1 interval is not replaced by a
Gaussian one. Structured highrank posts {cmd:e(V)} only when its joint
covariance and all individual checks pass; structured q1 does not post
{cmd:e(V)}. Thus {cmd:lincom} requires an available highrank covariance.
Failures that affect the whole calculation, such as a failed solver
accuracy check, stop the command with an error.

{pstd}
For pooled inference, {cmd:e(inference_mover_units)},
{cmd:e(inference_stayer_units)} and {cmd:e(inference_independent_units)}
count the independent units. {cmd:e(residual_moment_diagnostics)} and
{cmd:e(component_spectrum)} describe the variance-model fit and how
concentrated each component is. These checks describe the realized design;
they do not establish coverage.

{pstd}
In the package's simulation assessment, the exact Mata method produced
intervals too rarely in three of its four primary designs (22 failed
availability checks), while the native calculation produced intervals in
all eight primary designs and passed the broad descriptive checks but was
materially sensitive to the number of probes. These finite designs provide no general
coverage guarantee, and no conservative-coverage guarantee is implied.
Read the {browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/INFERENCE.md":inference guide},
the {browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md":assessment}
and the separate
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/POOLED_COMPONENT_ASSESSMENT.md":pooled mover-stayer assessment}
before reporting intervals; evidence for observation or mover-only inference
does not establish coverage for pooled inference. An
{help fevc##component_example:illustrative example} appears below.

{dlgtab:Fixed-effect projection inference}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:project(}{it:varlist}{cmd:)}{col 40}project fixed effects on covariates
    {cmd:projecteffect(worker|firm)}{col 40}dimension to project (required)
    {cmd:projectweight(frequency|target)}{col 40}projection weights; default frequency
  {hline 76}

{pstd}
These options estimate relationships between fixed effects and observed
characteristics, such as firm wage premiums and firm size, with leave-out
standard errors that account for the estimation error in the effects. An
intercept is included automatically. Slopes do not depend on the
normalization of the fixed effects; the intercept does. The default
calculation is exact and suited to small designs; see the
{help fevc##projection_example:example} and the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/INFERENCE.md":inference guide} for the large-sample
alternative. Projection results are stored separately; Stata's {cmd:lincom}
does not operate directly on them.

{pstd}
Projections use {cmd:centering(mean)} by default and accept
{cmd:centering(none)}; {cmd:centering(corrected)} is not supported. The mean
is the same retained-sample mean as for point estimates, equal to
{cmd:summarize u if e(sample) [fw=f]}, where {cmd:u} is the outcome (net of
the fitted controls under {cmd:nuisance(fixedoffset)}). Only the leave-out
covariance changes; the coefficients, residuals and the conventional
covariance based on full-sample residuals are unchanged. When a match contains
several observations, {cmd:fevc} centers the match's average outcome at the
overall mean. The KSS Matlab package centers the transformed match outcomes
instead; on the same mover-only sample, the two conventions agree when all
matches have the same number of observations and can differ otherwise. The
KSS Matlab package also treats eligible stayers separately. The {cmd:fevc} convention treats
frequency copies as separate observations and makes the standard errors
invariant to adding a constant to the outcome; neither convention is
uniformly better. Because the mean is estimated, the covariance can have a
finite-sample bias under heteroskedasticity; mean-centered projection does
not correct for it.
See {help fevc##centering:Outcome centering}.

{dlgtab:Computation and reproducibility}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:algorithm(jla|exact|auto)}{col 40}calculation method; default jla
    {cmd:backend(auto|mata|rust)}{col 40}implementation; default auto
    {cmd:rng(auto|stata|counter_v1)}{col 40}random-number method; default auto
    {cmd:engine(auto|compressed|generic)}{col 40}internal representation; default auto
    {cmd:preconditioner(auto|diagonal|cmg)}{col 40}solver method; default auto
    {cmd:nativethreads(}{it:#}{cmd:)}{col 40}native concurrency cap; default c(processors)
    {cmd:batch(auto|}{it:#}{cmd:)}{col 40}simultaneous calculations; default auto
    {cmd:probeorder(}{it:varname}{cmd:)}{col 40}stable ordering key for random draws
  {hline 76}

{pstd}
The default {cmd:algorithm(jla)} uses a randomized approximation.
{cmd:algorithm(exact)} uses deterministic linear algebra for small designs.
{cmd:algorithm(auto)} chooses exact when the number of identified parameters
is within {cmd:exact_limit()}, and JLA otherwise.

{pstd}
Automatic backend selection uses the Rust plugin when it supports the
request and otherwise uses Mata. {cmd:backend(mata)} selects the portable
implementation; {cmd:backend(rust)} and {cmd:rng(counter_v1)} require the
native one. The package includes native plugins for macOS, Linux (x86-64)
and Windows (x86-64). The Windows plugin passes build and unit tests but has
not yet been tested inside Stata on Windows. A self-built or outdated plugin
may lack some features. In that case {cmd:backend(auto)} uses Mata if it can
decide before estimation starts, while {cmd:backend(rust)} or
{cmd:rng(counter_v1)} stop with an error; an omitted {cmd:mcse()} keeps the
native point estimates and reports the MCSE as unavailable, while an
explicit {cmd:mcse(all)} stops with an error. Run {cmd:fevc_rust probe} to
check the loaded plugin; the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/INSTALLATION.md":installation notes} list the values to
expect.
Leave the remaining settings automatic unless investigating a specific
computational issue. The
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/RUST_MATA_PARITY.md":backend reference}
lists which features each backend supports, and the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/RC_BINARY_PAYLOAD.md":native payload guide}
records the exact plugins and their testing.

{pstd}
{cmd:nativethreads()} limits how many threads the Rust backend uses. It is
independent of the Stata/MP license and does not change Stata's processor
setting; Mata does not use it. Specify an integer from 1 to 64, no greater
than the number of processors available. On a cluster using the SGE
scheduler, both the native threads and Stata's processors must fit within
the job's {cmd:NSLOTS}. For example, with a four-core Stata/MP license in a
28-slot job, use {cmd:set processors 4} and {cmd:nativethreads(28)}. A
request beyond the allocation stops before estimation. Some steps run with
fewer threads. {cmd:e(native_threads_requested)},
{cmd:e(native_threads_selected)} and {cmd:e(native_threads_capacity)} record
the request, the cap actually applied and the available capacity;
{cmd:e(native_threads_active)} records the concurrency actually measured,
where missing means unavailable, not zero.

{dlgtab:Memory and numerical settings}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:memory_gib(}{it:#}{cmd:)}{col 40}optional memory planning budget in GiB
    {cmd:memorycheck(warn|error|off)}{col 40}budget policy; default warn
    {cmd:wallseconds(}{it:#}{cmd:)}{col 40}optional advisory time budget
    {cmd:tolerance(}{it:#}{cmd:)}{col 40}override solver tolerances
    {cmd:maxiter(}{it:#}{cmd:)}{col 40}iteration limit; default 10,000
    {cmd:exact_limit(}{it:#}{cmd:)}{col 40}exact dimension limit; default 500
    {cmd:rank_tolerance(}{it:#}{cmd:)}{col 40}rank threshold; default 1e-10
    {cmd:block_tolerance(}{it:#}{cmd:)}{col 40}deletion threshold; default 1e-10
    {cmd:blocksize_limit(}{it:#}{cmd:)}{col 40}stored match-block limit; default 5,000
    {cmd:physical_limit(}{it:#}{cmd:)}{col 40}generic JLA copies; default 50 million
  {hline 76}

{pstd}
Without {cmd:memory_gib()}, the command assumes no memory budget. An
explicit budget guides automatic batching. {cmd:memorycheck(warn)}, the default,
warns and continues when the forecast exceeds that budget;
{cmd:memorycheck(error)} stops and {cmd:memorycheck(off)} suppresses the
warning. The forecast covers the command's own allocations, not total Stata
memory, and the budget is not an operating-system memory cap.
See the {browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/MEMORY.md":memory guide}.

{pstd}
Numerical defaults normally need no adjustment. If estimation fails,
inspect the reported reason before changing a tolerance or limit.

{dlgtab:Additional output}

{pstd}
{cmd:verbose} adds computation details. {cmd:nolog} takes precedence over
it. {cmd:nodisplay} suppresses progress and successful final output;
{cmd:quietly} is also supported. These options do not change estimates or
stored results. Memory warnings and errors follow their own policies.

{marker examples}
{title:More runnable examples}

{pstd}
Each link runs the displayed commands and restores your data. The example
data retain the true {cmd:worker_fe}, {cmd:firm_fe}, and {cmd:error}.
{stata viewsource fevc__simulate_data.ado:View all data-generation code}.

{dlgtab:Example 2: Exact calculation on a small sample}

{pstd}
The same positive-sorting design as Example 1, with 200 workers and 61 firms.

{cmd}{...}
        preserve
{* example_start - exact_controls}{...}
        fevc, simulate_data(ex2) clear
        fevc log_wage productivity i.period, ///
            worker(worker_id) firm(firm_id) algorithm(exact)
        estat decomposition, full
{* example_end}{...}
        restore
{txt}{...}
{pstd}{stata fevc_run exact_controls using fevc.sthlp:Click to run}{p_end}

{dlgtab:Example 3: Frequency weights and target weights}

{pstd}
Frequency weights enter the regression; target weights determine how the
variance components are weighted. This example also holds the control
contribution fixed with {cmd:nuisance(fixedoffset)}.

{cmd}{...}
        preserve
{* example_start - weights_targets}{...}
        fevc, simulate_data(ex3) clear
        fevc log_wage productivity [fw=frequency], ///
            worker(worker_id) firm(firm_id) ///
            deletionid(match_id) nuisance(fixedoffset) ///
            targetweight(target_mass) algorithm(exact)
{* example_end}{...}
        restore
{txt}{...}
{pstd}{stata fevc_run weights_targets using fevc.sthlp:Click to run}{p_end}

{marker projection_example}
{dlgtab:Example 4: Project firm effects on firm size}

{pstd}
Firm size is average annual employment. The true firm effect is
{cmd:0.5*ln(firm_size)}, so the estimated projection slope should be near 0.5.
The projection weights firms by worker-year observations and uses the
default mean centering.

{cmd}{...}
        preserve
{* example_start - projection_inference}{...}
        fevc, simulate_data(ex4) clear
        fevc log_wage, worker(worker_id) firm(firm_id) algorithm(exact) ///
            backend(mata) project(log_firm_size) projecteffect(firm)
{* example_end}{...}
        restore
{txt}{...}
{pstd}{stata fevc_run projection_inference using fevc.sthlp:Click to run}{p_end}

{marker component_example}
{dlgtab:Example 5: Component inference and lincom}

{pstd}
A small fixed dataset illustrates syntax; it is not evidence of interval
coverage. This example uses observation deletion and exact inference.
{cmd:lincom} requires an available joint component covariance {cmd:e(V)}.

{cmd}{...}
        preserve
{* example_start - component_inference}{...}
        fevc, simulate_data(ex5) clear
        fevc log_wage productivity policy, ///
            worker(worker_id) firm(firm_id) ///
            deletion(observation) inference(highrank) ///
            inferencesimulations(100) inferenceseed(42) inferencebins(16)
        lincom worker_variance+firm_variance+2*worker_firm_covariance
{* example_end}{...}
        restore
{txt}{...}
{pstd}{stata fevc_run component_inference using fevc.sthlp:Click to run}{p_end}

{marker simulation}
{title:Generate example data separately}

{phang2}{cmd:. fevc, simulate_data(ex1) clear}{p_end}
{phang2}{cmd:. fevc, simulate_data(ex2) clear seed(12345)}{p_end}

{pstd}
{cmd:simulate_data(ex1)} through {cmd:simulate_data(ex5)} create the datasets
above and leave them in memory. Specify {cmd:clear} to replace existing data.
Generation restores the caller's random-number state and restores the old
data if it fails. Each random example has a fixed default seed;
{cmd:seed()} here controls the data, separately from the estimation seed.
Example 5 is fixed and does not accept a seed.

{pstd}
The printed true components are population moments of the realized effects
in the generated sample, using the example's target weights (not sample
variances with an N-1 denominator). Examples 1 and 2 retain the full generated
sample under their displayed specifications. If you change the sample or
weights, recompute the truth for that population. The helper returns
{cmd:r(truth)}, {cmd:r(N)}, {cmd:r(workers)}, {cmd:r(firms)}, {cmd:r(seed)},
{cmd:r(example)}, and {cmd:r(weighting)}. It does not estimate a model.

{marker stored}
{title:Key stored results}

{pstd}
{bf:Estimates.} {cmd:e(b)} and {cmd:e(kss)} contain the corrected worker
variance, firm variance, worker-firm covariance, and variance of the sum, in
that order. {cmd:e(plugin)} and {cmd:e(correction)} hold the uncorrected
estimates and the bias corrections; {cmd:e(kss)} is their difference.
{cmd:e(decomposition)} adds the sorting row and shares; stored shares are
proportions, while the display reports percentages. Centering leaves
{cmd:e(plugin)} unchanged and changes {cmd:e(correction)}.

{pstd}
{bf:Numerical MCSE.} {cmd:e(mcse)} contains the four numerical MCSEs, and
{cmd:e(results)} has rows {cmd:plugin}, {cmd:bias_correction},
{cmd:corrected} and {cmd:mcse}; its last row matches {cmd:e(mcse)}.
{cmd:e(mcse_cov_raw)} and {cmd:e(mcse_cov)} are the raw and usable 4 by 4
numerical covariances in the same order; the total is the linear
combination worker variance + firm variance + 2 x covariance. When the raw
covariance has materially negative eigenvalues, it is kept but the usable
covariance and MCSEs are withheld. {cmd:e(mcse_mode)} records {cmd:all},
{cmd:off} or {cmd:conditional}; {cmd:e(mcse_method)} is
{cmd:crossfit_if_v1} for JLA, {cmd:exact} for exact calculation,
{cmd:conditional_target_v1} for the developer diagnostic, or {cmd:none} when
off. {cmd:e(mcse_status)} and {cmd:e(mcse_available)} record the status and
whether the MCSE is usable. With {cmd:mcse(off)}, {cmd:e(mcse)} and both
covariance matrices are missing, {cmd:e(mcse_available)} is zero and the
status is {cmd:off}. In conditional mode, or when the native plugin does
not provide the MCSE, the covariance matrices are missing; conditional mode
supplies only the conditional standard errors. Exact
calculation returns zero covariance and zero MCSE with availability one.
These diagnostics never create econometric {cmd:e(V)}. When exporting MCSEs,
also save the mode, method, status, availability, {cmd:e(mcse_cov_raw)},
{cmd:e(centering)} and {cmd:e(mcse_centering)}, so that a withheld MCSE is
not mistaken for zero.

{pstd}
{bf:Centering.} {cmd:e(centering)} records {cmd:none}, {cmd:mean} or
{cmd:corrected}, and {cmd:e(mcse_centering)} records the matching MCSE
convention: {cmd:uncentered}, {cmd:fixed observed mean}, or
{cmd:fixed observed mean and fixed centering increment}. Both are recorded
in exact mode and with MCSE off. With component inference,
{cmd:e(inference_centering)} is {cmd:uncentered} or {cmd:fixed observed mean}
and {cmd:e(inference_mean_omitted)} is 0 or 1; these are absent for
point-only or projection-only requests and cleared on failure. The
inference convention is separate from the MCSE convention and from the
omitted uncertainty in fitted controls.

{pstd}
{bf:Additional numerical diagnostics.} {cmd:e(numerical_mcse)} and
{cmd:e(numerical_mcse_available)} repeat the selected diagnostic;
{cmd:e(numerical_mcse_all)} holds the all-probe vector when calculated; and
{cmd:e(numerical_mccov_cond)}, {cmd:e(numerical_mccov_leverage)},
{cmd:e(numerical_mccov_all_raw)} and {cmd:e(numerical_mccov_all)} hold 3 by 3
covariances for the worker variance, firm variance and covariance. The
conditional vector is {cmd:e(mcse_conditional)}, with
{cmd:e(mcse_conditional_available)}, when all or conditional mode is
selected. {cmd:e(mc_replay_rhs)},
{cmd:e(mc_replay_attempted_rhs)}, {cmd:e(mc_replay_executed_rhs)},
{cmd:e(mc_replay_generator_work)} and {cmd:e(mc_allocation_bound_bytes)}
record the extra work and memory used to compute the MCSE, which reuses the
original random directions and the prepared solver.
{cmd:e(numerical_replay_rhs)} holds the corresponding solver residuals. If
this calculation fails, the MCSE is withheld and
{cmd:e(mc_failed_replay_probe)} identifies the first direction that failed.

{pstd}
{bf:Sample and other results.} {cmd:e(sample)} identifies retained rows.
Sample counts include {cmd:e(N)}, {cmd:e(worker_levels)}, and
{cmd:e(firm_levels)}. Component covariance is posted in {cmd:e(V)} only for
supported successful inference requests; q1 intervals have separate returns.
{cmd:e(projection_b)}, {cmd:e(projection_V)}, and
{cmd:e(projection_results)} contain projection results. Timing diagnostics
use only Stata timers that are not already in use and leave the caller's
timers unchanged. Phases that did not run have zero timings; timings that
could not be measured are missing. Use
{cmd:ereturn list} and the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/FAILURES_AND_RETURNS.md":returned-results reference}
for the complete record.

{marker troubleshooting}
{title:If estimation fails}

{pstd}
Read the reported reason and suggested next step. Common causes are too few
connections for leave-out estimation, collinear controls, unsupported option
combinations, or a numerical or memory limit. The command does not silently
substitute another estimator after a failed calculation.
For support, include your command, Stata version, and a small reproducible
example, together with {cmd:e(withholding_status)} and
{cmd:e(withholding_detail)} when available.

{marker reference}
{title:References and further reading}

{pstd}
Schmieder, Johannes. 2026. "fevc: Leave-out bias-corrected variance
decompositions in Stata." Working paper. The companion paper and its
technical supplement explain the estimator, sample construction, and
computational methods.

{pstd}
Kline, Patrick, Raffaele Saggio, and Mikkel Sølvsten. 2020.
"Leave-Out Estimation of Variance Components." {it:Econometrica}
88(5): 1859-1898.

{pstd}
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/README.md":Technical documentation}
provides implementation details and capability restrictions.

{marker author}
{title:Author}

{pstd}
Johannes F. Schmieder, Boston University.
{browse "mailto:johannes@bu.edu":johannes@bu.edu}

{pstd}
Version 0.5.0. Code is GPL-3.0-only.

{title:Also see}

{pstd}
{help estat}, {help lincom}, {help fvvarlist}, {help weights}
