version 18.0
clear all
set more off
set varabbrev off
args pkgroot plugindir
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
if `"`plugindir'"'!="" adopath ++ `"`plugindir'"'
quietly run `"`pkgroot'/fevc.ado"'

set obs 800
generate long worker = floor((_n-1)/40)
generate long firm = floor(mod(_n-1,40)/2)
generate long match = floor((_n-1)/2)+1
generate long order = _n
generate byte copies = 2
// Nonuniform target masses separate the leading modes for this boundary test;
// this fixture supplies execution evidence, not a q0/q1 coverage claim.
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
generate double shock = (mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
generate double outcome = worker-.8*firm+shock
generate double control = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double controlled = outcome+1.4*control
sort order
local caller_rng `"`c(rngstate)'"'
quietly datasignature
local caller_data `"`r(datasignature)'"'

local point worker(worker) firm(firm) deletion(match) deletionid(match) ///
    nuisance(fixedoffset) stayers(movers) backend(rust) rng(counter_v1) ///
    algorithm(jla) engine(generic) preconditioner(diagonal) probes(256) ///
    batch(8) tolerance(1e-11) targetweight(target) nodisplay
local infer inferencemodel(structured_common) inferencesimulations(512)

quietly fevc outcome [fw=copies], `point'
matrix baseline = e(results)
matrix baseline_b = e(b)
quietly fevc outcome [fw=copies], `point' `infer' inference(highrank)
assert mreldif(baseline,e(results))==0
assert mreldif(baseline_b,e(b))==0
assert `"`e(inference_deletion_requested)'"'=="match"
assert `"`e(inference_deletion_selected)'"'=="match"
assert `"`e(inference_nuisance_requested)'"'=="fixedoffset"
assert `"`e(inference_nuisance_selected)'"'=="fixedoffset"
assert e(inference_nuisance_omitted)==1
assert e(inference_independent_units)==400
assert e(deletion_units)==400
assert e(N_physical)==1600
assert abs(e(inference_effective_matches)-400)<1e-10
assert abs(e(inference_largest_mass_share)-1/400)<1e-12
assert e(inference_largest_leverage)>0 & e(inference_largest_leverage)<1
assert e(inference_smallest_maker)>0
assert strpos(`"`e(inference_method)'"',"Fixed-offset approximate match inference")>0
assert strpos(`"`e(inference_offset_warning)'"',"few controls")>0
assert e(component_unit_receipt)[1,"schema"]==1
assert e(component_unit_receipt)[1,"deletion"]==1
assert "`e(inference_variance_fit)'"=="residual_moments"
assert e(inference_gram_probes)==2048
assert e(component_inference_receipt)[1,"ordering"]==3
local returned_matrices : e(matrices)
foreach legacy in structured_variance_summary structured_variance_folds structured_variance_cv {
    assert !strpos(" `returned_matrices' "," `legacy' ")
}
matrix q0_V = e(V)
matrix q0_fit = e(residual_moment_diagnostics)
matrix q0_spectrum = e(component_spectrum)
quietly estat diagnostics
assert `"`c(rngstate)'"'==`"`caller_rng'"'
quietly datasignature
assert `"`r(datasignature)'"'==`"`caller_data'"'
quietly fevc_rust snapshot
assert r(state)==0

local wide : subinstr local point "batch(8)" "batch(16)"
gsort -order
quietly fevc outcome [fw=copies], `wide' `infer' inference(highrank)
assert mreldif(baseline,e(results))==0
assert mreldif(q0_V,e(V))==0
assert mreldif(q0_fit,e(residual_moment_diagnostics))==0
assert mreldif(q0_spectrum,e(component_spectrum))==0
sort order

// Literal copies increase regression mass, not the number of independent matches.
preserve
expand copies
replace copies = 1
replace target = target/2
quietly fevc outcome, `point' `infer' inference(highrank)
assert mreldif(baseline,e(results))<1e-10
assert mreldif(q0_V,e(V))<1e-10
assert e(inference_independent_units)==400
restore

quietly fevc controlled control [fw=copies], `point'
matrix controlled_baseline = e(results)
quietly fevc controlled control [fw=copies], `point' `infer' inference(q1)
assert mreldif(controlled_baseline,e(results))==0
assert e(inference_nuisance_omitted)==1
assert e(q1_computed_targets)>0
assert colsof(e(component_q1_diagnostics))==20
assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
matrix controlled_q1 = e(q1_inference)
matrix controlled_status = e(q1_status)
local returned_matrices : e(matrices)
assert !strpos(" `returned_matrices' "," V ")

local cmg : subinstr local point "preconditioner(diagonal)" "preconditioner(cmg)"
quietly fevc controlled control [fw=copies], `cmg' `infer' inference(q1)
assert mreldif(controlled_baseline,e(results))<1e-8
// Compare the reported intervals, not near-tied eigenvector coordinates.
// Rosetta can rotate those coordinates across solvers while the endpoints
// agree at the original 1e-8 gate. Numerical mode certificates remain required.
matrix cmg_q1 = e(q1_inference)
matrix controlled_intervals = controlled_q1[1..4,1..6]
matrix cmg_intervals = cmg_q1[1..4,1..6]
assert mreldif(controlled_intervals,cmg_intervals)<1e-8
assert mreldif(controlled_status,e(q1_status))==0
assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
assert `"`e(inference_solver_selected)'"'=="CMG"

local leverage : subinstr local infer "structured_common" "structured_leverage"
quietly fevc outcome [fw=copies], `point' `leverage' inference(q1)
assert mreldif(baseline,e(results))==0
assert `"`e(inference_model_selected)'"'=="structured_leverage"
assert e(inference_independent_units)==400

// Two declared matches at one coefficient cell must remain distinct units.
replace match = 401 if order==2
quietly fevc outcome [fw=copies], `point' `infer' inference(highrank)
assert e(inference_independent_units)==401
assert e(deletion_units)==401
replace match = 1 if order==2

// Twenty original matches at one model firm still form a mover history.
// Both inference references must retain that worker and all original units.
preserve
replace firm=0 if worker==0
replace outcome=worker-.8*firm+shock
quietly fevc outcome [fw=copies], `point'
matrix pooled_point=e(results)
foreach reference in highrank q1 {
    quietly fevc outcome [fw=copies], `point' `infer' inference(`reference')
    assert e(sample)==1
    assert e(inference_independent_units)==400
    assert e(deletion_units)==400
    assert mreldif(pooled_point,e(results))==0
    assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
    if "`reference'"=="q1" assert e(q1_computed_targets)>0
    else {
        matrix pooled_V=e(V)
        assert rowsof(pooled_V)==4 & colsof(pooled_V)==4
    }
}
restore

// Unsupported combinations stop in capability preflight, before native work.
foreach change in joint both autoengine omittedengine omitteddeletion omittednuisance omittedstayers {
    if "`change'"=="joint" local invalid : subinstr local point "nuisance(fixedoffset)" "nuisance(joint)"
    if "`change'"=="both" local invalid : subinstr local point "stayers(movers)" "stayers(both)"
    if "`change'"=="autoengine" local invalid : subinstr local point "engine(generic)" "engine(auto)"
    if "`change'"=="omittedengine" local invalid : subinstr local point "engine(generic)" ""
    if "`change'"=="omitteddeletion" local invalid : subinstr local point "deletion(match)" ""
    if "`change'"=="omittednuisance" local invalid : subinstr local point "nuisance(fixedoffset)" ""
    if "`change'"=="omittedstayers" local invalid : subinstr local point "stayers(movers)" ""
    capture noisily fevc outcome [fw=copies], `invalid' `infer' inference(highrank)
    assert _rc==498
    assert `"`e(status)'"'=="WITHHELD"
    assert `"`e(withholding_status)'"'=="STRUCTURED_INFERENCE_TUPLE_REQUIRED"
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly fevc_rust snapshot
    assert r(state)==0
}
// A valid-looking but inconsistent unit receipt must not reach e(V).
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 18.0
    gettoken command rest : 0
    fevc_rust `command' `rest'
    return add
    if "`command'"=="componentresultv5" {
        if "$FEVC_TEST_UNIT_FAULT"=="count" return scalar independent_units = r(independent_units)+1
        if "$FEVC_TEST_UNIT_FAULT"=="deletion" return scalar unit_deletion = 2
        if "$FEVC_TEST_UNIT_FAULT"=="omission" return scalar nuisance_uncertainty_omitted = 0
        if "$FEVC_TEST_UNIT_FAULT"=="schema" return scalar unit_schema = 2
        if "$FEVC_TEST_UNIT_FAULT"=="ordering" return scalar ordering = 2
    }
end
foreach fault in count deletion omission schema ordering {
    global FEVC_TEST_UNIT_FAULT "`fault'"
    capture noisily fevc outcome [fw=copies], `point' `infer' inference(highrank)
    assert _rc==498
    capture matrix bad_V = e(V)
    assert _rc!=0
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly fevc_rust snapshot
    assert r(state)==0
}
macro drop FEVC_TEST_UNIT_FAULT
program drop fevc__rust_public_call
quietly datasignature
assert `"`r(datasignature)'"'==`"`caller_data'"'
di as result "PASS test_rust_match_component_inference.do"
exit 0
