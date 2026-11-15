// Native match-deletion component inference with fixed nuisance offsets:
// repeat-measurement standard errors on the explicit generic Counter-V1
// tuple. Order, batch and solver invariance, literal copies, declared
// matches as units, the leading directions of inference(q1), the requests
// the model refuses before native work, and unit-receipt fault injection.
version 15.0
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
// this fixture supplies execution evidence, not a coverage claim.
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
generate double shock = (mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
generate double outcome = worker-.8*firm+shock
generate double control = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double controlled = outcome+1.4*control
sort order
local caller_rng `"`c(rngstate)'"'
quietly datasignature
local caller_data `"`r(datasignature)'"'

// The explicit generic tuple keeps the explicit batch() of these checks; the
// batch-invariance check widens it.
local point centering(none) worker(worker) firm(firm) deletion(match) deletionid(match) ///
    nuisance(fixedoffset) stayers(movers) backend(rust) rng(counter_v1) ///
    algorithm(jla) engine(generic) preconditioner(diagonal) probes(256) ///
    batch(8) tolerance(1e-11) targetweight(target) nodisplay
local infer inferencedraws(16)

quietly fevc outcome [fw=copies], `point' inference(none)
matrix baseline = e(results)
matrix baseline_b = e(b)
// Optional V2 fields left by a failed transport cannot contaminate V1 results.
scalar __vckss_comp_unit_movers = -17
scalar __vckss_comp_unit_stayers = 17
quietly fevc outcome [fw=copies], `point' `infer' inference(highrank)
assert mreldif(baseline,e(results))==0
tempname compat_assert_b
matrix `compat_assert_b' = e(b)
assert mreldif(baseline_b,`compat_assert_b')==0
matrix drop `compat_assert_b'
assert `"`e(status)'"'=="FEVC_REPEAT_MEASUREMENT_SE"
assert `"`e(inference_model)'"'=="repeat_measurement"
assert `"`e(inference_deletion_requested)'"'=="match"
assert `"`e(inference_deletion_selected)'"'=="match"
assert `"`e(inference_nuisance_requested)'"'=="fixedoffset"
assert `"`e(inference_nuisance_selected)'"'=="fixedoffset"
// No controls: no nuisance offset is estimated.
assert e(inference_nuisance_omitted)==1
assert strpos(`"`e(inference_offset_warning)'"',"no controls")>0
assert e(inference_independent_units)==400
assert e(inference_mover_units)==400
assert e(inference_stayer_units)==0
assert e(deletion_units)==400
assert e(N_physical)==1600
assert abs(e(inference_effective_matches)-400)<1e-10
assert abs(e(inference_largest_mass_share)-1/400)<1e-12
assert e(inference_largest_leverage)>0 & e(inference_largest_leverage)<1
assert e(inference_smallest_maker)>0
assert strpos(`"`e(inference_method)'"',"repeat-measurement standard errors")>0
assert inlist(el(e(component_unit_receipt),1,colnumb(e(component_unit_receipt),"schema")),1,2)
assert el(e(component_unit_receipt),1,colnumb(e(component_unit_receipt),"deletion"))==1
assert "`e(inference_variance_fit)'"=="repeat_measurement"
assert e(inference_draws)==16
// A mover's collapsed match has no repeat measurement: no sets, no pair term.
assert e(inference_repeat_sets)==0 & e(inference_pair_term)==0
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"gram_probes"))==0
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"ordering"))==0
assert e(inference_solver_columns)==4*16+3
local returned_matrices : e(matrices)
foreach legacy in structured_variance_summary structured_variance_folds ///
    structured_variance_cv residual_moment_diagnostics {
    assert !strpos(" `returned_matrices' "," `legacy' ")
}
matrix q0_V = e(V)
matrix q0_repeat = e(inference_repeat)
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
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(q0_V,`compat_assert_V')==0
matrix drop `compat_assert_V'
assert mreldif(q0_repeat,e(inference_repeat))==0
assert mreldif(q0_spectrum,e(component_spectrum))==0
sort order

// Literal copies increase regression mass, not the number of independent matches.
preserve
expand copies
replace copies = 1
replace target = target/2
quietly fevc outcome, `point' `infer' inference(highrank)
assert mreldif(baseline,e(results))<1e-10
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(q0_V,`compat_assert_V')<1e-10
matrix drop `compat_assert_V'
assert e(inference_independent_units)==400
restore

quietly fevc controlled control [fw=copies], `point' inference(none)
matrix controlled_baseline = e(results)
quietly fevc controlled control [fw=copies], `point' `infer' inference(q1)
assert mreldif(controlled_baseline,e(results))==0
// A control: its estimation enters to first order.
assert e(inference_nuisance_omitted)==0
assert strpos(`"`e(inference_method)'"',"included to first order")>0
assert "`e(inference_leading_request)'"=="q1"
assert e(inference_leading_unresolved)==0
assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
matrix controlled_table = e(inference_intervals)
assert rowsof(controlled_table)==4 & colsof(controlled_table)==16
forvalues row=1/4 {
    assert inlist(el(controlled_table,`row',colnumb(controlled_table,"regime")),0,1)
    assert el(controlled_table,`row',colnumb(controlled_table,"lb"))<el(controlled_table,`row',colnumb(controlled_table,"ub"))
}
local returned_matrices : e(matrices)
assert (strpos(" `returned_matrices' "," V ")>0)==(e(inference_q1_components)==0)

local cmg : subinstr local point "preconditioner(diagonal)" "preconditioner(cmg)"
quietly fevc controlled control [fw=copies], `cmg' `infer' inference(q1)
assert mreldif(controlled_baseline,e(results))<1e-8
// Compare the reported intervals, not near-tied eigenvector coordinates,
// which another solver can rotate.
matrix cmg_table = e(inference_intervals)
matrix controlled_intervals = controlled_table[1..4,1..9]
matrix cmg_intervals = cmg_table[1..4,1..9]
assert mreldif(controlled_intervals,cmg_intervals)<1e-8
forvalues row=1/4 {
    assert el(controlled_table,`row',colnumb(controlled_table,"regime"))==el(cmg_table,`row',colnumb(cmg_table,"regime"))
}
assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
assert `"`e(inference_solver_selected)'"'=="CMG"

// The model's name is a no-op alias of the one method.
quietly fevc outcome [fw=copies], `point' `infer' inference(q1)
assert mreldif(baseline,e(results))==0
matrix plain_table = e(inference_intervals)
quietly fevc outcome [fw=copies], `point' `infer' inference(q1) inferencemodel(repeat_measurement)
assert mreldif(baseline,e(results))==0
matrix alias_table = e(inference_intervals)
forvalues row=1/4 {
    forvalues column=1/16 {
        assert plain_table[`row',`column']==alias_table[`row',`column']
    }
}
assert `"`e(inference_model_selected)'"'=="repeat_measurement"
assert e(inference_independent_units)==400

// Two declared matches at one coefficient cell must remain distinct units.
replace match = 401 if order==2
quietly fevc outcome [fw=copies], `point' `infer' inference(highrank)
assert e(inference_independent_units)==401
assert e(deletion_units)==401
replace match = 1 if order==2

// Twenty original matches at one model firm still form a mover history.
// Both inference requests must retain that worker and all original units.
preserve
replace firm=0 if worker==0
replace outcome=worker-.8*firm+shock
quietly fevc outcome [fw=copies], `point' inference(none)
matrix pooled_point=e(results)
foreach reference in highrank q1 {
    quietly fevc outcome [fw=copies], `point' `infer' inference(`reference')
    assert e(sample)==1
    assert e(inference_independent_units)==400
    assert e(deletion_units)==400
    assert mreldif(pooled_point,e(results))==0
    assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
    if "`reference'"=="q1" confirm matrix e(inference_intervals)
    else {
        matrix pooled_V=e(V)
        assert rowsof(pooled_V)==4 & colsof(pooled_V)==4
    }
}
restore

// Requests the model refuses stop before native preparation and RNG:
// observation deletion with fixed offsets, native exact kernels, the removed
// variance models and their options, and project() beside the standard
// errors.
foreach change in observation exact model simulations project {
    local status REPEAT_MEASUREMENT_TUPLE_REQUIRED
    local code 498
    local request : copy local point
    local extra
    if "`change'"=="observation" local request : subinstr local point "deletion(match) deletionid(match)" "deletion(observation)"
    if "`change'"=="exact" local request : subinstr local point "algorithm(jla)" "algorithm(exact)"
    if "`change'"=="model" {
        local extra inferencemodel(structured_common)
        local status INFERENCE_MODEL_REMOVED
        local code 198
    }
    if "`change'"=="simulations" {
        local extra inferencesimulations(512)
        local status INFERENCE_OPTION_REMOVED
        local code 198
    }
    if "`change'"=="project" {
        local extra project(control) projecteffect(firm)
        local status COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED
    }
    capture noisily fevc outcome control [fw=copies], `request' `infer' `extra' inference(highrank)
    assert _rc==`code'
    assert `"`e(status)'"'=="WITHHELD"
    assert `"`e(withholding_status)'"'=="`status'"
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly fevc_rust snapshot
    assert r(state)==0
}
// Default/explicit both requires the mixed-unit capability, even with zero stayers.
quietly fevc_rust probe
local mixed_api = r(component_mixed_api)
foreach population in both default {
    local replacement "stayers(both)"
    if "`population'"=="default" local replacement ""
    local pooled : subinstr local point "stayers(movers)" "`replacement'"
    capture noisily fevc outcome [fw=copies], `pooled' `infer' inference(highrank)
    if `mixed_api'==1 {
        assert _rc==0
        assert e(inference_stayer_units)==0
        assert mreldif(baseline,e(results))==0
    }
    else {
        assert _rc==498
        assert "`e(withholding_status)'"=="REPEAT_MEASUREMENT_TUPLE_REQUIRED"
    }
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly fevc_rust snapshot
    assert r(state)==0
}
// A valid-looking but inconsistent unit receipt must not reach e(V).
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 15.0
    gettoken command rest : 0
    fevc_rust `command' `rest'
    return add
    if "`command'"=="componentresultv5" {
        if "$FEVC_TEST_UNIT_FAULT"=="count" return scalar independent_units = r(independent_units)+1
        if "$FEVC_TEST_UNIT_FAULT"=="deletion" return scalar unit_deletion = 2
        if "$FEVC_TEST_UNIT_FAULT"=="omission" return scalar nuisance_uncertainty_omitted = 0
        if "$FEVC_TEST_UNIT_FAULT"=="schema" return scalar unit_schema = 99
        if "$FEVC_TEST_UNIT_FAULT"=="ordering" return scalar ordering = 2
    }
end
foreach fault in count deletion omission schema ordering {
    global FEVC_TEST_UNIT_FAULT "`fault'"
    capture noisily fevc outcome [fw=copies], `point' `infer' inference(highrank)
    assert _rc==498
    capture matrix list e(V)
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
