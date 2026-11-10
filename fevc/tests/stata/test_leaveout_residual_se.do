* Leave-out residual standard errors: the default request and its routes.
* Written without access to Stata; it has not been executed. Every fevc call
* names inference() so that FEVC_DEFAULT_INFERENCE cannot change its meaning.
version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

* Does the installed plugin carry the method? Without it the native calls
* fall back to point estimates and the Mata calls still report them.
local native 0
capture quietly fevc_rust probe
if !_rc {
    if r(component_residual_api)==1 local native 1
}

fevc, simulate_data(ex2) clear
generate double x1 = sin(_n*.37)+.2*firm_id/61
generate double x2 = cos(_n*.11)

program define _se_check
    args expect_controls
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_model)'"=="leaveout_residual"
    assert "`e(inference_variance_fit)'"=="leaveout_residual"
    assert e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
    assert e(inference_draws)==8
    confirm matrix e(V)
    tempname V b ci rank
    matrix `V' = e(V)
    matrix `b' = e(b)
    matrix `ci' = e(component_inference)
    assert rowsof(`V')==4 & colsof(`V')==4
    assert mreldif(`V',`V'')<1e-12
    forvalues j = 1/4 {
        assert `V'[`j',`j']>0
        assert reldif(`ci'[`j',1],`b'[1,`j'])<1e-12
        assert reldif(`ci'[`j',2],sqrt(`V'[`j',`j']))<1e-12
        assert `ci'[`j',3]<`ci'[`j',1] & `ci'[`j',1]<`ci'[`j',4]
    }
    * The total is worker + firm + 2 x covariance, so its variance follows.
    local total = `V'[1,1]+`V'[2,2]+4*`V'[3,3]+2*`V'[1,2]+4*`V'[1,3]+4*`V'[2,3]
    assert reldif(`total',`V'[4,4])<1e-10
    quietly lincom worker_variance+firm_variance+2*worker_firm_covariance-total_variance
    assert abs(r(estimate))<1e-10*max(1,abs(`b'[1,4])) & r(se)<1e-6*sqrt(`V'[4,4])
    matrix `rank' = e(inference_effective_rank)
    assert rowsof(`rank')==4 & `rank'[1,1]>=1
    assert e(inference_max_influence_share)>0 & e(inference_max_influence_share)<=1
    assert e(inference_mean_omitted)==0
    assert e(inference_nuisance_omitted)==(`expect_controls'==0)
    assert strpos("`e(inference_variance_warning)'","heteroskedasticity")>0
end

* --- inference(none) is point estimation only, as before -------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) inference(none) nodisplay
capture confirm matrix e(V)
assert _rc
assert "`e(inference_variance_fit)'"==""
matrix point = e(kss)
capture confirm scalar e(inference_auto)
assert _rc

* --- the default request, without and with controls --------------------------
foreach controls in "" "x1 x2" {
    local q : word count `controls'
    quietly fevc log_wage `controls', worker(worker_id) firm(firm_id) inference(none) nodisplay
    matrix point = e(kss)
    quietly fevc log_wage `controls', worker(worker_id) firm(firm_id) inference(auto) nodisplay
    * Standard errors never change the point estimate beyond the engine's
    * numerical tolerance (a mover-only request without controls moves from
    * the compressed to the generic engine).
    assert mreldif(point,e(kss))<1e-6
    if "`e(backend_selected)'"=="mata" | `native' {
        _se_check `q'
        if `q'>0 & "`e(backend_selected)'"=="rust" {
            * Joint nuisance with controls spends one target pass on the
            * fixed-offset geometry: two solves per point probe.
            assert e(inference_offset_pass_solves)==2*e(probes)
        }
    }
    else {
        capture confirm matrix e(V)
        assert _rc
        assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""
    }
}

* --- other units and options the method accepts ------------------------------
foreach options in "deletion(observation)" "stayers(movers)"                 ///
    "nuisance(fixedoffset)" "centering(corrected)" "centering(none)" {
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) `options' ///
        inference(auto) nodisplay
    if "`e(backend_selected)'"=="mata" | `native' _se_check 1
}

* --- the Mata runtime: linear solves and exact kernels ------------------------
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    inference(auto) nodisplay
_se_check 1
assert "`e(backend_selected)'"=="mata"
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) nodisplay
_se_check 1
matrix exact_V = e(V)
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) inferencedraws(64) nodisplay
* More draws change only the simulated part of the variance.
forvalues j = 1/4 {
    assert reldif(sqrt(exact_V[`j',`j']),sqrt(el(e(V),`j',`j')))<.2
}

* --- explicit strict request and its limits ----------------------------------
capture quietly fevc log_wage, worker(worker_id) firm(firm_id) ///
    inference(q1) inferencemodel(leaveout_residual) nodisplay
assert _rc==498 & "`e(withholding_status)'"=="LEAVEOUT_RESIDUAL_Q1_UNSUPPORTED"
* project() does not carry them: the default returns point estimates.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) project(x1) projecteffect(firm) inference(auto) nodisplay
capture confirm matrix e(V)
assert _rc
assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""

display "PASS test_leaveout_residual_se.do"
