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
    args expect_controls draws
    if "`draws'"=="" local draws 8
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_model)'"=="leaveout_residual"
    assert "`e(inference_variance_fit)'"=="leaveout_residual"
    assert e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
    assert e(inference_draws)==`draws'
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
    * That combination has zero variance by construction, so its standard
    * error is zero up to rounding (and missing if rounding goes negative).
    quietly lincom worker_variance+firm_variance+2*worker_firm_covariance-total_variance
    assert abs(r(estimate))<1e-10*max(1,abs(`b'[1,4]))
    assert r(se)<1e-6*sqrt(`V'[4,4]) | missing(r(se))
    matrix `rank' = e(inference_effective_rank)
    assert rowsof(`rank')==4 & `rank'[1,1]>=1
    assert e(inference_max_influence_share)>0 & e(inference_max_influence_share)<=1
    assert e(inference_mean_omitted)==0
    * No offset is omitted with controls, nor for observation units.
    assert e(inference_nuisance_omitted)==(`expect_controls'==0 & "`e(deletion)'"=="match")
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
    * Standard errors do not change the estimator. On the native backend
    * they select another executor (and, for a mover-only request without
    * controls, the generic engine), so agreement is to solver tolerance.
    assert mreldif(point,e(kss))<1e-5
    if "`e(backend_selected)'"=="mata" | `native' {
        _se_check `q'
        if `q'>0 & "`e(backend_selected)'"=="rust" {
            * Joint nuisance with controls spends one target pass on the
            * fixed-offset geometry: two solves per point probe.
            assert e(inference_offset_pass_solves)==2*e(probes)
        }
        if "`e(backend_selected)'"=="rust" {
            * The native route also reports the working-variance fit and the
            * covariance that takes the fitted working variances as true.
            assert colsof(e(component_inference_receipt))==64
            assert e(component_inference_receipt)[1,"offset_pass_solves"]==e(inference_offset_pass_solves)
            assert e(inference_controls)==`q'
            assert e(inference_signal_truncation)>=0
            matrix terms = e(inference_working_terms)
            assert inrange(terms[1,1],1,5) & inrange(terms[1,2],0,3)
            matrix Vm = e(V_modelbased)
            matrix sem = e(se_modelbased)
            assert rowsof(Vm)==4 & colsof(Vm)==4 & colsof(sem)==4
            forvalues j = 1/4 {
                assert Vm[`j',`j']>0 & reldif(sem[1,`j'],sqrt(Vm[`j',`j']))<1e-12
            }
            assert mreldif(Vm,Vm')<1e-10
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
* More draws change only the simulated part of the variance; the estimates
* themselves are untouched.
matrix exact_b = e(b)
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) inferencedraws(64) nodisplay
_se_check 1 64
assert mreldif(exact_b,e(b))<1e-12
* Without controls the exact unit kernel must reproduce the posted estimate;
* a mismatch stops the command with TARGET_IDENTITY_FAILED.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) nodisplay
_se_check 0
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) nuisance(fixedoffset) inference(auto) nodisplay
_se_check 1

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
