// Individual component results of native repeat-measurement standard errors
// on both deletion modes: per-component status, the joint covariance only
// when it is admissible, pseudo-noise draws as their own numerical domain,
// the removed Gram option, and transport faults injected into a real solved
// result.
version 18.0
clear all
set more off
set varabbrev off
args pkgroot plugindir
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
if `"`plugindir'"'!="" adopath ++ `"`plugindir'"'
quietly run `"`pkgroot'/fevc.ado"'
noisily fevc_rust componentversion
assert r(interface_version)==4

set obs 800
generate long worker = floor((_n-1)/40)
generate long firm = floor(mod(_n-1,40)/2)
generate long match = floor((_n-1)/2)+1
generate long rowid = _n
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
generate double control = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double shock = (mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
generate double outcome = worker-.8*firm+1.4*control+shock
// The explicit generic tuple keeps an explicit batch(): with an automatic
// component batch the native receipt check refuses the residual pass.
local common worker(worker) firm(firm) stayers(movers) backend(rust) rng(counter_v1) ///
    algorithm(jla) engine(generic) preconditioner(diagonal) batch(16) ///
    targetweight(target)
quietly datasignature
local data_before `"`r(datasignature)'"'
local rng_before `"`c(rngstate)'"'

foreach deletion in observation match {
    local options deletion(`deletion')
    if "`deletion'"=="match" local options `options' deletionid(match) nuisance(fixedoffset)
    foreach reference in highrank q1 {
        noisily fevc outcome control, `common' `options' centering(none) inference(`reference')
        assert e(probes)==200
        assert e(inference_draws)==8 & e(inference_simulations)==4*8
        assert e(component_inference_receipt)[1,"schema"]==5
        assert e(component_inference_receipt)[1,"model"]==4
        assert e(component_inference_receipt)[1,"ordering"]==0
        assert inlist(e(inference_joint_status),0,1,2,3)
        assert e(inference_computed_targets)>=0 & e(inference_computed_targets)<=4
        assert rowsof(e(q0_status))==4
        local returned_matrices : e(matrices)
        assert "`e(inference_variance_fit)'"=="repeat_measurement"
        assert "`e(inference_gram_method)'"=="not applicable"
        assert !strpos(" `returned_matrices' "," structured_variance_cv ")
        assert !strpos(" `returned_matrices' "," residual_moment_diagnostics ")
        assert e(variance_floor_share)>=0 & e(variance_floor_share)<1
        assert e(inference_spectrum_iterations)==0
        // A control: its estimation enters to first order on both deletion modes.
        assert e(inference_nuisance_omitted)==0
        if "`deletion'"=="match" {
            assert strpos("`e(inference_variance_response)'", "sample variance")>0
            assert e(component_unit_receipt)[1,"deletion"]==1
        }
        else assert e(component_unit_receipt)[1,"deletion"]==2
        // The pass is the high-rank one; no critical-value simulation.
        assert e(inference_critical_draws)==0
        if "`reference'"=="q1" {
            assert "`e(inference_leading_request)'"=="q1"
            assert rowsof(e(inference_intervals))==4 & colsof(e(inference_intervals))==16
            assert (e(inference_joint_status)==3)==(e(inference_q1_components)>0)
            assert (strpos(" `returned_matrices' "," V ")>0)==(e(inference_q1_components)==0)
            assert e(inference_joint_posted)==(e(inference_q1_components)==0)
        }
        else {
            assert inlist(e(inference_joint_status),0,1,2)
            capture confirm matrix e(inference_intervals)
            assert _rc
        }
        if e(inference_joint_available)==0 {
            assert !strpos(" `returned_matrices' "," V_primitive ")
        }
        noisily estat diagnostics
        quietly datasignature
        assert `"`r(datasignature)'"'==`"`data_before'"'
        assert `"`c(rngstate)'"'==`"`rng_before'"'
        quietly fevc_rust snapshot
        assert r(state)==0
    }
}
// The number of pseudo-noise draws changes only its own numerical domain;
// omitted and explicit defaults agree, points and caller state are invariant
// at every tested count, and every draw costs four linear solves.
foreach deletion in observation match {
    local options deletion(`deletion')
    if "`deletion'"=="match" local options `options' deletionid(match) nuisance(fixedoffset)
    quietly fevc outcome control, `common' `options' centering(none) inference(highrank)
    matrix default_b = e(b)
    matrix default_ci = e(component_inference)
    scalar default_columns = e(inference_solver_columns)
    foreach count in 8 4 16 {
        quietly fevc outcome control, `common' `options' centering(none) inference(highrank) inferencedraws(`count')
        assert e(inference_draws)==`count' & e(inference_simulations)==4*`count'
        assert mreldif(e(b),default_b)==0
        assert e(inference_solver_columns)==default_columns+4*(`count'-8)
        if `count'==8 assert mreldif(e(component_inference),default_ci)==0
        assert `"`c(rngstate)'"'==`"`rng_before'"'
    }
}
foreach count in 1 4097 {
    capture noisily fevc outcome control, `common' deletion(observation) centering(none) inference(highrank) inferencedraws(`count')
    assert _rc==198
    assert "`e(withholding_status)'"=="INVALID_INFERENCE_TUNING"
    assert `"`c(rngstate)'"'==`"`rng_before'"'
}
// inferencegramprobes() tuned only the removed structured models: any value,
// on any request, stops before other work.
foreach count in 2048 0 511 1.5 2147483648 . nope {
    capture noisily fevc outcome control, `common' deletion(observation) centering(none) inference(highrank) inferencegramprobes(`count')
    assert _rc==198
    assert "`e(withholding_status)'"=="INFERENCE_OPTION_REMOVED"
    assert `"`e(withholding_detail)'"'=="inferencegramprobes() applied only to the variance models removed in fevc 0.6.0"
    assert `"`c(rngstate)'"'==`"`rng_before'"'
}
foreach options in "" "inference(highrank) algorithm(exact)" "project(control) projecteffect(firm)" {
    capture noisily fevc outcome control, worker(worker) firm(firm) deletion(observation) centering(none) `options' inferencegramprobes(2048)
    assert _rc==198
    assert "`e(withholding_status)'"=="INFERENCE_OPTION_REMOVED"
    assert `"`c(rngstate)'"'==`"`rng_before'"'
}

// Inject only transport changes into a real solved result. This is a boundary
// regression, not a claim that this fixture has an indefinite joint matrix.
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 18.0
    gettoken command rest : 0
    tempname target spectrum
    fevc_rust `command' `rest'
    if "`command'"=="componentresultv5" & "$FEVC_TEST_INDIVIDUAL_FAULT"=="target" {
        matrix `target' = r(target_variance_status)
    }
    if "`command'"=="componentresultv5" & "$FEVC_TEST_INDIVIDUAL_FAULT"=="spectrum" {
        matrix `spectrum' = r(spectrum)
    }
    return add
    if "`command'"=="componentversion" & "$FEVC_TEST_INDIVIDUAL_FAULT"=="version" return scalar interface_version = 3
    if "`command'"=="componentresultv5" {
        if "$FEVC_TEST_INDIVIDUAL_FAULT"=="joint" {
            tempname primitive covariance
            matrix `primitive' = J(3,3,.)
            matrix `covariance' = J(4,4,.)
            return matrix primitive_covariance = `primitive'
            return matrix covariance = `covariance'
            return scalar joint_status = 2
        }
        if "$FEVC_TEST_INDIVIDUAL_FAULT"=="count" return scalar computed_targets = 9
        // No Gram matrix is fitted: a Gram condition or count is a fault.
        if "$FEVC_TEST_INDIVIDUAL_FAULT"=="gram" return scalar gram_rcond = 0
        if "$FEVC_TEST_INDIVIDUAL_FAULT"=="gramcount" return scalar gram_probes = 512
        if "$FEVC_TEST_INDIVIDUAL_FAULT"=="target" {
            matrix `target'[1,1] = -1
            matrix `target'[1,2] = 0
            return matrix target_variance_status = `target'
        }
        // No spectral iteration is run.
        if "$FEVC_TEST_INDIVIDUAL_FAULT"=="spectrum" {
            matrix `spectrum'[1,14] = 128
            return matrix spectrum = `spectrum'
        }
    }
end
global FEVC_TEST_INDIVIDUAL_FAULT joint
foreach reference in highrank q1 {
    quietly fevc outcome control, `common' deletion(observation) centering(none) inference(`reference')
    assert e(inference_joint_available)==0
    assert e(inference_joint_posted)==0
    assert e(inference_computed_targets)==4
    local returned_matrices : e(matrices)
    assert !strpos(" `returned_matrices' "," V ")
    assert !strpos(" `returned_matrices' "," V_primitive ")
    if "`reference'"=="highrank" assert e(component_inference)[1,"se"]>0
    else assert e(inference_intervals)[1,"lb"]<e(inference_intervals)[1,"ub"]
}
foreach fault in count gram gramcount target spectrum version {
    display "INDIVIDUAL RECEIPT FAULT: `fault'"
    global FEVC_TEST_INDIVIDUAL_FAULT `fault'
    capture noisily fevc outcome control, `common' deletion(observation) centering(none) inference(q1)
    assert _rc==498
    local returned_matrices : e(matrices)
    assert !strpos(" `returned_matrices' "," V ")
    assert `"`c(rngstate)'"'==`"`rng_before'"'
    quietly fevc_rust snapshot
    assert r(state)==0
}
macro drop FEVC_TEST_INDIVIDUAL_FAULT
program drop fevc__rust_public_call
quietly datasignature
assert `"`r(datasignature)'"'==`"`data_before'"'

// An outcome-free equal-match diffuse geometry: every component is computed
// on both solvers.
clear
set obs 1200
generate long worker = floor((_n-1)/60)+1
generate long firm = floor(mod(_n-1,60)/3)+1
generate long match = floor((_n-1)/3)+1
generate double target = (.85+mod((worker-1)*19+(firm-1)*11+3,31)/100)/3
generate double outcome = .12*worker-.09*firm + ///
    (mod((_n-1)*37+11,101)/50-1)*(.5+.01*worker)
foreach solver in diagonal cmg {
    quietly fevc outcome, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) nuisance(fixedoffset) stayers(movers) ///
        backend(rust) rng(counter_v1) algorithm(jla) engine(generic) ///
        preconditioner(`solver') batch(16) targetweight(target) ///
        centering(none) inference(highrank) inferenceseed(8675309)
    assert e(probes)==200
    assert "`e(inference_variance_fit)'"=="repeat_measurement"
    assert e(inference_computed_targets)==4
    forvalues target_index=1/4 {
        assert e(q0_status)[`target_index',1]==0
    }
    quietly fevc_rust snapshot
    assert r(state)==0
}
display "FEVC INDIVIDUAL INFERENCE PASS"
