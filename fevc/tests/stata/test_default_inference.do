* The command's own default: without any inference option it reports
* repeat-measurement standard errors and one interval per component, and the
* switch FEVC_DEFAULT_INFERENCE=none keeps the earlier point-only default for
* suites written before it. A process started with the switch in its
* environment cannot see the default; the final line says which was checked.
version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
* A suite may have set the switch as a global; it gets it back.
local caller_setting "$FEVC_DEFAULT_INFERENCE"

* Does the installed plugin carry the method?
local native 0
capture quietly fevc_rust probe
if !_rc {
    if r(component_repeat_api)==1 & r(component_mixed_api)==1 local native 1
}

fevc, simulate_data(ex2) clear
generate double x1 = sin(_n*.37)+.2*firm_id/61

* The switch as a global macro: point estimates only, nothing of the pass.
global FEVC_DEFAULT_INFERENCE "none"
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) nodisplay
assert "`e(status)'"=="KSS_POINT_ESTIMATES_ONLY"
capture confirm matrix e(V)
assert _rc
capture confirm matrix e(inference_intervals)
assert _rc
global FEVC_DEFAULT_INFERENCE ""

* The default's model is the only one: inference(highrank) and inference(q1)
* without inferencemodel() give exactly the numbers of the request that names
* it, element by element, whatever the switch says.
program define _same_save
    foreach name in b V kss results component_inference inference_intervals ///
        inference_leading inference_repeat component_inference_receipt {
        capture matrix drop same_`name'
        // A missing e() matrix would otherwise be copied as one missing value.
        capture confirm matrix e(`name')
        if !_rc matrix same_`name' = e(`name')
    }
    global DI_SAME_STATUS "`e(status)'"
end
program define _same_check
    assert "`e(status)'"=="$DI_SAME_STATUS" & "`e(inference_model)'"=="repeat_measurement"
    tempname A
    foreach name in b V kss results component_inference inference_intervals ///
        inference_leading inference_repeat component_inference_receipt {
        capture confirm matrix same_`name'
        local saved = (_rc==0)
        capture confirm matrix e(`name')
        assert `saved'==(_rc==0)
        if !`saved' continue
        matrix `A' = e(`name')
        assert rowsof(`A')==rowsof(same_`name') & colsof(`A')==colsof(same_`name')
        forvalues i = 1/`=rowsof(`A')' {
            forvalues j = 1/`=colsof(`A')' {
                assert `A'[`i',`j']==same_`name'[`i',`j']
            }
        }
    }
end
foreach backend in mata rust {
    if "`backend'"=="rust" & !`native' continue
    foreach inference in highrank q1 {
        quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(`backend') ///
            deletion(observation) inference(`inference') ///
            inferencemodel(repeat_measurement) nodisplay
        _same_save
        quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(`backend') ///
            deletion(observation) inference(`inference') nodisplay
        _same_check
        assert "`e(backend_selected)'"=="`backend'"
        capture confirm scalar e(inference_auto)
        assert _rc
    }
}
macro drop DI_SAME_STATUS

if lower(strtrim(`"`: environment FEVC_DEFAULT_INFERENCE'"'))=="none" {
    global FEVC_DEFAULT_INFERENCE "`caller_setting'"
    display "PASS test_default_inference.do default=switched_off native=`native'"
    exit
}

program define _default_check
    args backend
    assert "`e(backend_selected)'"=="`backend'"
    assert e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
    assert "`e(inference_model)'"=="repeat_measurement"
    assert "`e(inference_leading_request)'"=="auto"
    assert e(inference_share_threshold)==.1
    * One control: its estimation enters the standard errors.
    assert e(inference_nuisance_omitted)==0
    tempname table b
    matrix `table' = e(inference_intervals)
    matrix `b' = e(b)
    assert rowsof(`table')==4 & colsof(`table')==16
    forvalues j = 1/4 {
        assert inlist(`table'[`j',7],0,1)
        assert `table'[`j',8]<`b'[1,`j'] & `b'[1,`j']<`table'[`j',9]
    }
    * The joint covariance is posted exactly when every interval is normal.
    capture confirm matrix e(V)
    assert (_rc==0)==(e(inference_q1_components)==0)
    assert e(inference_joint_posted)==(e(inference_q1_components)==0)
end

foreach backend in mata rust {
    if "`backend'"=="rust" & !`native' continue
    foreach deletion in match observation {
        quietly fevc log_wage x1, worker(worker_id) firm(firm_id) ///
            backend(`backend') deletion(`deletion') nodisplay
        _default_check `backend'
    }
}
* No backend named: the command chooses one and either reports the
* intervals or says why it could not.
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) nodisplay
assert e(inference_auto)==1
if "`e(inference_auto_reason)'"=="" _default_check `e(backend_selected)'
else assert "`e(status)'"=="KSS_POINT_ESTIMATES_ONLY"
* inference(none) by name is the point-only request whatever the default.
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) inference(none) nodisplay
assert "`e(status)'"=="KSS_POINT_ESTIMATES_ONLY"
capture confirm matrix e(inference_intervals)
assert _rc
capture confirm scalar e(inference_auto)
assert _rc

global FEVC_DEFAULT_INFERENCE "`caller_setting'"
display "PASS test_default_inference.do default=checked native=`native'"
