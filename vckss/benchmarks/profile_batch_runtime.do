version 18.0
clear all
set more off
set varabbrev off

args workers_arg firms_arg probes_arg seed_arg
local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")
if missing(`workers') local workers = 2500
if missing(`firms') local firms = 250
if missing(`probes') local probes = 40
if missing(`benchmark_seed') local benchmark_seed = 8675309
if `workers' != floor(`workers') | `firms' != floor(`firms') | ///
    `probes' != floor(`probes') | `benchmark_seed' != floor(`benchmark_seed') | ///
    `workers' < 2*`firms' | `firms' < 3 | `probes' < 2 {
    di as error "invalid profile dimensions, probes, or seed"
    exit 198
}

capture confirm file "vckss/vckss.ado"
if _rc {
    di as error "run profile_batch_runtime.do from the repository root"
    exit 601
}
adopath ++ "`c(pwd)'/vckss"
capture set processors 8
local active_processors = c(processors)

set obs `=4*`workers''
generate long worker = floor((_n-1)/4)+1
generate byte period = mod(_n-1,4)+1
generate long home_firm = mod(worker-1,`firms')+1
generate long firm = cond(period<=2,home_firm,mod(home_firm,`firms')+1)
generate long match = 2*(worker-1)+(period>2)+1
generate long observation_key = _n
generate byte frequency = 1+mod(match,2)
generate double target = .5+mod(_n-1,17)/17
generate double c1 = cond(mod(period,2),-.5,.5)
generate double c2 = cond(period==1,-.8,cond(period==2,.3, ///
    cond(period==3,.6,-.2)))
generate double y = sin(worker/97)+cos(firm/31)+.35*c1-.18*c2+ ///
    sin(observation_key/113)

local batches 1 4 8 16
matrix profile = J(4,14,.)
matrix colnames profile = batch command setup fit leverage target ///
    correction pcg schur precond_apply schur_batches precond_batches ///
    max_residual rel_to_batch1
local profile_row = 0
foreach selected_batch of local batches {
    local ++profile_row
    timer clear 80
    timer on 80
    vckss y c1 c2 [fw=frequency], worker(worker) firm(firm) ///
        deletion(match) deletionid(match) targetweight(target) ///
        probeorder(observation_key) algorithm(jla) nuisance(joint) ///
        preconditioner(diagonal) memory_gib(4) ///
        probes(`probes') batch(`selected_batch') seed(`benchmark_seed') ///
        tolerance(1e-10) maxiter(20000) nodisplay
    timer off 80
    quietly timer list 80
    local command_seconds = r(t80)
    if `profile_row' == 1 {
        matrix reference = e(results)
        local rng_reference `"`c(rngstate)'"'
        local batch1_seconds = `command_seconds'
        local estimator_difference = 0
    }
    else {
        assert `"`c(rngstate)'"' == `"`rng_reference'"'
        local estimator_difference = mreldif(reference,e(results))
        assert `estimator_difference' < 2e-11
    }
    assert e(solver_max_residual) <= 1e-10
    assert e(setup_seconds)+e(fit_seconds)+e(leverage_seconds)+ ///
        e(target_seconds) <= `command_seconds'+0.05
    matrix profile[`profile_row',.] = (`selected_batch', ///
        `command_seconds',e(setup_seconds),e(fit_seconds), ///
        e(leverage_seconds),e(target_seconds),e(correction_seconds), ///
        e(pcg_seconds),e(schur_seconds),e(preconditioner_apply_seconds), ///
        e(solver_schur_batches),e(solver_precond_batches), ///
        e(solver_max_residual),`batch1_seconds'/`command_seconds')
}

di as txt "KSS batch profile: Stata `c(stata_version)' `c(flavor)', " ///
    "processors=`active_processors', workers=`workers', firms=`firms', " ///
    "probes=`probes', seed=`benchmark_seed'"
matrix list profile, format(%12.5g)
di as result "VCKSS PROFILE PASS: batch runtime"
exit 0
