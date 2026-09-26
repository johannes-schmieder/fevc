{smcl}
{* *! version 0.5.0-rc.1 22sep2026}{...}
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
{cmd:deletionid()} is needed only when your match units differ from the
default worker-firm pairs, for example when distinct employment spells at
the same firm are treated as independent matches. Each ID must belong to
one worker-firm pair. It applies to match deletion. A match-mode mover has
more than one distinct original deletion ID, even if those IDs share a model
firm (for example, pooled employers). Without this option, movers still have
more than one firm. Different IDs assume independent error blocks; repeated
spells with the same ID remain one block.

{phang}
{cmd:stayers(both)}, the default, includes movers and eligible stayers.
Match-mode stayers have one original deletion unit; observation-mode stayers
have one model firm. With match deletion, stayer contributions use a separate
observation-level correction and are {it:not} robust to within-match error
correlation. Use {cmd:stayers(movers)} for a decomposition restricted to
movers. This changes the estimation sample and the population described by
the estimates. With observation deletion, all retained observations use the
same observation-level correction. All retained deletions must preserve model
and control identification; counting two blocks alone does not guarantee this.

{phang}
Mata and current Rust plugins support parallel declared blocks within a
worker-firm cell. Older plugins use Mata under {cmd:backend(auto)}; strict
{cmd:backend(rust)} or {cmd:rng(counter_v1)} requests require a plugin update.
The usual inference restrictions continue to apply.

{phang}
{cmd:probes()} controls the precision of the default randomized calculation
(JLA). More probes reduce numerical approximation error but take longer.
Start with the default 200; increase it if the reported numerical MCSE is
large relative to the components you wish to interpret. Numerical MCSE is
{bf:not a sampling standard error}; it also excludes uncertainty from the
initial leverage approximation.

{phang}
{cmd:seed()} makes the randomized calculation reproducible within the
selected runtime. The default is 8675309. Different backends need not give
identical randomized estimates with the same seed. Estimation restores the
caller's random-number state.

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
mover sample, then includes eligible original stayers at retained firms
with at least two observations (counting frequency weights). Removed movers are not reclassified as
stayers. Controls must also remain identified under the requested deletions;
if that cannot be established, the command reports a failure.

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
Positive integer {cmd:fweight}s represent repeated physical observations
and affect the regression. {cmd:targetweight()} instead determines how
retained rows contribute to the variance components. By default, target
weights equal frequency weights (or one without weights). Explicit target
weights are stored-row masses and are {bf:not multiplied by frequency weights}. They change the population described by the decomposition.

{pstd}
{cmd:nuisance(joint)} allows control coefficients to change in the leave-out
calculation. {cmd:nuisance(fixedoffset)} first subtracts the fitted control
index and then holds it fixed. This is a different, conditional convention.
Controls must be identified; ordinary collinear controls are not silently
dropped. See the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/ESTIMATOR_CONTRACT.md":estimator reference}.

{marker inference}
{dlgtab:Component inference}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:inference(none|highrank|q1)}{col 40}component intervals; default none
    {cmd:inferencemodel(}{it:mode}{cmd:)}{col 40}optional structured variance model
    {cmd:level(}{it:#}{cmd:)}{col 40}confidence level; default 95
    {cmd:inferencesimulations(}{it:#}{cmd:)}{col 40}variance simulations; default 1,000
    {cmd:inferencegramprobes(}{it:#}{cmd:)}{col 40}Gram precision; default 2,048
    {cmd:inferenceseed(}{it:#}{cmd:)}{col 40}inference seed; default 8675309
    {cmd:inferencebins(}{it:#}{cmd:)}{col 40}exact-route smoothing; default 1,000
  {hline 76}

{pstd}
{cmd:inference(highrank)} requests Gaussian component intervals;
{cmd:inference(q1)} requests intervals allowing one dominant weakly
identified mode. These require different identification conditions; q1
does not cover arbitrary multi-mode weakness. Successful computation does
not establish valid coverage, and there is no automatic choice between them.

{pstd}
Without {cmd:inferencemodel()}, component inference uses exact Mata
calculation, observation deletion, and unit frequency weights.
The explicit structured models {cmd:structured_common} and
{cmd:structured_leverage} impose additional variance assumptions; they are
not unrestricted heteroskedasticity-robust inference. Observation-q1
calibration retains a documented limitation. Fixed-offset match inference
is approximate, ignoring nuisance-control estimation uncertainty.
Read the {browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/INFERENCE.md":inference guide}
for supported requests, assumptions, and limitations before reporting intervals.
An {help fevc##component_example:illustrative example} appears below.

{dlgtab:Fixed-effect projection inference}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:project(}{it:varlist}{cmd:)}{col 40}project fixed effects on covariates
    {cmd:projecteffect(worker|firm)}{col 40}dimension to project (required)
    {cmd:projectweight(frequency|target)}{col 40}projection weights; default frequency
  {hline 76}

{pstd}
These options estimate relationships between fixed effects and observed
characteristics, such as firm wage premiums and firm size. An intercept is
included automatically. Slopes are invariant to the fixed-effect
normalization; the intercept is not. The default calculation is exact and
suited to small designs. See the {help fevc##projection_example:example}
and the {browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/INFERENCE.md":inference guide}
for the explicit scalable alternative. Projection results are stored
separately; Stata's {cmd:lincom} does not operate directly on these rows.

{dlgtab:Computation and reproducibility}

  {it:Option}{col 40}Description
  {hline 76}
    {cmd:algorithm(jla|exact|auto)}{col 40}calculation method; default jla
    {cmd:backend(auto|mata|rust)}{col 40}implementation; default auto
    {cmd:rng(auto|stata|counter_v1)}{col 40}random-number method; default auto
    {cmd:engine(auto|compressed|generic)}{col 40}internal representation; default auto
    {cmd:preconditioner(auto|diagonal|cmg)}{col 40}solver method; default auto
    {cmd:batch(auto|}{it:#}{cmd:)}{col 40}simultaneous calculations; default auto
    {cmd:probeorder(}{it:varname}{cmd:)}{col 40}stable ordering key for random draws
  {hline 76}

{pstd}
The default {cmd:algorithm(jla)} uses a randomized approximation.
{cmd:algorithm(exact)} uses deterministic linear algebra for small designs.
Explicit {cmd:algorithm(auto)} chooses exact when the identified dimension
is within {cmd:exact_limit()}, and JLA otherwise.

{pstd}
Automatic backend selection prefers a compatible Rust plugin and otherwise
uses Mata. {cmd:backend(mata)} selects the portable implementation;
{cmd:backend(rust)} requires the native implementation. Leave the remaining
settings automatic unless investigating a specific computational issue.
The {browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/RUST_MATA_PARITY.md":backend reference}
describes specialized combinations and restrictions.

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
warning. The forecast covers command allocations, not total Stata memory,
and the budget is not an operating-system memory cap.
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
Frequency weights enter the regression; target mass weights the variance
components. This example also illustrates holding the control index fixed.

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
The projection weights firms by worker-year observations.

{cmd}{...}
        preserve
{* example_start - projection_inference}{...}
        fevc, simulate_data(ex4) clear
        fevc log_wage, worker(worker_id) firm(firm_id) algorithm(exact) ///
            project(log_firm_size) projecteffect(firm)
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
{cmd:e(b)} and {cmd:e(kss)} contain corrected worker variance, firm variance,
raw worker-firm covariance, and total variance, in that order.
{cmd:e(plugin)}, {cmd:e(correction)}, and {cmd:e(numerical_mcse)} hold their
uncorrected estimates, bias corrections, and numerical MCSEs.
{cmd:e(decomposition)} adds the sorting row and shares; stored shares are
proportions, while the display reports percentages.

{pstd}
{cmd:e(sample)} identifies retained rows. Sample counts include
{cmd:e(N)}, {cmd:e(worker_levels)}, and {cmd:e(firm_levels)}.
Component covariance is posted in {cmd:e(V)} only for supported successful
inference requests; q1 intervals have separate returns.
{cmd:e(projection_b)}, {cmd:e(projection_V)}, and
{cmd:e(projection_results)} contain projection results.
Use {cmd:ereturn list} and the
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/FAILURES_AND_RETURNS.md":returned-results reference}
for the complete diagnostic record.

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
decompositions in Stata." Working paper, September. The companion
fevc paper and its appendix explain the estimator, sample construction,
and computational methods.

{pstd}
Kline, Patrick, Raffaele Saggio, and Mikkel Sølvsten. 2020.
"Leave-Out Estimation of Variance Components." {it:Econometrica}
88(5): 1859-1898.

{pstd}
{browse "https://github.com/johannes-schmieder/fevc/blob/main/fevc/docs/README.md":Technical documentation}
provides implementation details and current capability restrictions.

{marker author}
{title:Author}

{pstd}
Johannes F. Schmieder, Boston University.
{browse "mailto:johannes@bu.edu":johannes@bu.edu}

{pstd}
Version 0.5.0-rc.1 is prerelease software. Code is GPL-3.0-only.

{title:Also see}

{pstd}
{help estat}, {help lincom}, {help fvvarlist}, {help weights}
