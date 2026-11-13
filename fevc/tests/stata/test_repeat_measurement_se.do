* Repeat-measurement standard errors on the Mata routes: the sample variance
* of every repeat-measurement set (an eligible stayer's observations; under
* observation deletion also a mover's match of two or more observations),
* the two-step model for the remaining units, and the high-rank covariance
* with the pair term of the sets. Expected values come from an independent
* dense calculation in Python on the same deterministic panels (exact trace
* term); the command simulates that term, so standard errors agree to its
* Monte Carlo error. Every fevc call names inference().
version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

* 200 movers with three jobs of two observations (ex2), and 150 stayers with
* four observations each at firms of the movers' network.
fevc, simulate_data(ex2) clear
keep log_wage worker_id firm_id
local N = _N
quietly set obs `=_N+600'
quietly replace worker_id = 200+ceil((_n-`N')/4) if _n>`N'
quietly replace firm_id = 1+mod(worker_id,61) if _n>`N'
quietly replace log_wage = sin(worker_id*1.7)+0.01*firm_id+ ///
    (0.2+0.3*mod(worker_id,3))*cos(_n*2.3+worker_id) if _n>`N'
generate double x1 = sin(_n*.37)+.2*firm_id/61

* Independent dense calculation: estimates, pair term, standard errors.
matrix expected_b = (2.193981373967747, 1.014791107858934, -.2004904720705475)
scalar expected_pair = 1.478986773550021e-06
matrix expected_se = (.2595212617894884, .2433340968591373, ///
    .1263905520856996, .2676009330004356)

program define _rm_check
    args draws sets
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_model)'"=="repeat_measurement"
    assert "`e(inference_variance_fit)'"=="repeat_measurement"
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_SE"
    assert e(inference_draws)==`draws'
    assert e(inference_repeat_sets)==`sets'
    * The estimated offset is held fixed, with or without controls.
    assert e(inference_nuisance_omitted)==1
    assert strpos("`e(inference_variance_warning)'","model for movers")>0
    confirm matrix e(V)
    tempname V Vt b ci shape
    matrix `V' = e(V)
    matrix `Vt' = `V''
    matrix `b' = e(b)
    matrix `ci' = e(component_inference)
    matrix `shape' = e(inference_repeat)
    assert colsof(`shape')==14 & `shape'[1,5]==`sets'
    assert inrange(`shape'[1,1],1,5) & inrange(`shape'[1,2],1,6)
    assert `shape'[1,3]>0 & `shape'[1,8]>=0
    assert mreldif(`V',`Vt')<1e-12
    forvalues j = 1/4 {
        assert `V'[`j',`j']>0
        assert reldif(`ci'[`j',1],`b'[1,`j'])<1e-12
        assert reldif(`ci'[`j',2],sqrt(`V'[`j',`j']))<1e-12
    }
    local total = `V'[1,1]+`V'[2,2]+4*`V'[3,3]+2*`V'[1,2]+4*`V'[1,3]+4*`V'[2,3]
    assert reldif(`total',`V'[4,4])<1e-10
end

* --- point estimates are not touched -----------------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(none) nodisplay
matrix point = e(kss)
forvalues j = 1/3 {
    assert reldif(point[1,`j'],expected_b[1,`j'])<1e-10
}

* --- exact kernels: the independent calculation ------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(512) nodisplay
_rm_check 512 150
assert mreldif(point,e(kss))<1e-12
assert e(inference_independent_units)==1200
assert reldif(e(inference_pair_term),expected_pair)<1e-8
matrix shape = e(inference_repeat)
* Every match has two observations here, so the two mass terms of the scale
* are spanned by the intercept: three active scale terms, six profile terms.
assert shape[1,1]==3 & shape[1,2]==6 & shape[1,6]==600 & shape[1,7]==0
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * 512 draws: the simulated trace term is within a fraction of a per cent.
    assert reldif(ci[`j',2],expected_se[1,`j'])<0.01
}
* More draws move only the simulated part.
matrix b512 = e(b)
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(64) nodisplay
_rm_check 64 150
assert mreldif(b512,e(b))<1e-12
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],expected_se[1,`j'])<0.05
}

* --- linear solves and projected leverages -----------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 150
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * Projection noise in leverages and target diagonals on top of the draws.
    assert reldif(ci[`j',2],expected_se[1,`j'])<0.15
}

* --- controls, the mover-only population, centering --------------------------
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) nodisplay
_rm_check 8 150
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) nuisance(fixedoffset) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
_rm_check 8 150
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) stayers(movers) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
_rm_check 8 0
assert e(inference_pair_term)==0
foreach centering in none corrected {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
        algorithm(exact) centering(`centering') inference(highrank) ///
        inferencemodel(repeat_measurement) nodisplay
    _rm_check 8 150
}

* --- the native route, when the loaded plugin carries the method -------------
local native 0
capture quietly fevc_rust probe
if !_rc {
    if r(component_repeat_api)==1 & r(component_mixed_api)==1 local native 1
}
if `native' {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) inference(highrank) inferencemodel(repeat_measurement) ///
        inferencedraws(64) nodisplay
    _rm_check 64 150
    assert "`e(backend_selected)'"=="rust"
    assert colsof(e(component_inference_receipt))==78
    assert e(inference_independent_units)==1200
    * Projected target diagonals: the pair term agrees to projection noise.
    assert reldif(e(inference_pair_term),expected_pair)<0.2
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        assert reldif(ci[`j',2],expected_se[1,`j'])<0.15
    }
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(highrank) inferencemodel(repeat_measurement) nodisplay
    _rm_check 8 150
    * The default on the same backend is still the leave-out residual method.
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(auto) nodisplay
    assert "`e(inference_variance_fit)'"=="leaveout_residual"
    assert "`e(backend_selected)'"=="rust"
}
else {
    * A plugin without the capability refuses a strict native request.
    capture quietly fevc_rust probe
    if !_rc {
        capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
            inference(highrank) inferencemodel(repeat_measurement) nodisplay
        assert _rc==498 & "`e(withholding_status)'"=="REPEAT_MEASUREMENT_TUPLE_REQUIRED"
    }
}

* --- observation deletion ----------------------------------------------------
* Every physical observation is a unit. The panel gets matches of one, two
* and three observations: a match of two or more is a repeat-measurement set
* like a stayer, and the 66 single observations of movers receive the model,
* fitted on all observations of movers because there are fewer than a hundred.
preserve
bysort worker_id (firm_id log_wage): generate long seq = _n
quietly drop if worker_id<=200 & mod(worker_id,3)==0 & seq==2
quietly expand 2 if worker_id<=200 & mod(worker_id,3)==1 & seq==1, generate(extra)
quietly replace log_wage = log_wage+0.1*cos(worker_id) if extra==1
drop seq extra
assert _N==1801
* Independent dense calculation on this panel; with x1 the estimated control
* index of the full regression is held fixed.
matrix observation_b = (2.154005266392971, 1.171084753616503, -.2944140523794802)
scalar observation_pair = 1.434113200955210e-04
matrix observation_se = (.2661415525311538, .2647469861814654, ///
    .1714761737879706, .2121325734000382)
matrix observation_se_x1 = (.2660014404422780, .2644807941651816, ///
    .1712614183010554, .2121175684510716)

quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(none) nodisplay
matrix observation_point = e(kss)
forvalues j = 1/3 {
    assert reldif(observation_point[1,`j'],observation_b[1,`j'])<1e-10
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684
assert mreldif(observation_point,e(kss))<1e-12
assert e(inference_independent_units)==1801
assert reldif(e(inference_pair_term),observation_pair)<1e-8
matrix shape = e(inference_repeat)
* Unit mass is one throughout, so the two mass terms of the scale are
* spanned by the intercept; 1,735 observations lie in the 684 sets.
assert shape[1,1]==3 & shape[1,2]==6 & shape[1,6]==1735 & shape[1,7]==0
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],observation_se[1,`j'])<0.01
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 684
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],observation_se[1,`j'])<0.15
}
* Jointly estimated controls: the point estimate keeps them in the model,
* the standard error holds their estimated index fixed.
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],observation_se_x1[1,`j'])<0.01
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) stayers(movers) ///
    inference(highrank) inferencemodel(repeat_measurement) nodisplay
_rm_check 8 534
if `native' {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(highrank) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_check 64 684
    assert "`e(backend_selected)'"=="rust"
    assert e(inference_independent_units)==1801
    assert reldif(e(inference_pair_term),observation_pair)<0.2
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        assert reldif(ci[`j',2],observation_se[1,`j'])<0.15
    }
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(highrank) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_check 64 684
    assert "`e(backend_selected)'"=="rust"
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        assert reldif(ci[`j',2],observation_se_x1[1,`j'])<0.15
    }
}
restore

* --- what the model does not accept ------------------------------------------
capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    inference(q1) inferencemodel(repeat_measurement) nodisplay
assert _rc==498 & "`e(withholding_status)'"=="REPEAT_MEASUREMENT_Q1_UNSUPPORTED"
capture quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    deletion(observation) nuisance(fixedoffset) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
assert _rc==498 & "`e(withholding_status)'"=="REPEAT_MEASUREMENT_TUPLE_REQUIRED"
capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    inference(auto) inferencemodel(repeat_measurement) nodisplay
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_WITHOUT_INFERENCE"

* --- the default is unchanged ------------------------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) nodisplay
assert "`e(inference_variance_fit)'"=="leaveout_residual"
assert "`e(status)'"=="FEVC_LEAVEOUT_RESIDUAL_SE"
capture confirm matrix e(inference_repeat)
assert _rc

display "PASS test_repeat_measurement_se.do"
