version 15.0

// Check the first example on its actual data: printed truth and estimator
// describe the same retained population. The runner checks restoration below.
fevc, simulate_data(ex1) clear
quietly fevc log_wage productivity i.period, worker(worker_id) firm(firm_id)
assert e(N)==_N
assert e(sample)==1

clear
set obs 3
generate long sentinel = _n
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

foreach example in exact_controls jla_controls weights_targets {
    fevc_run `example' using fevc.sthlp
    assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
    assert rowsof(e(decomposition)) == 4
    // Realized observation-weighted truths from the help DGPs.
    if inlist("`example'", "exact_controls", "jla_controls") {
        tempname truth
        if "`example'" == "exact_controls" {
            matrix `truth' = (1.0506331, .99483822, .12537027, 2.29621186)
        }
        else {
            matrix `truth' = (1.0871205, .98406545, .16611605, 2.40341805)
        }
        assert el(e(plugin),1,3) < 0
        forvalues target = 1/4 {
            local true_value = el(`truth',1,`target')
            assert el(e(kss),1,`target') > 0
            assert abs(el(e(kss),1,`target')-`true_value') < .25*`true_value'
            assert abs(el(e(kss),1,`target')-`true_value') < ///
                abs(el(e(plugin),1,`target')-`true_value')
        }
        assert e(N) == cond("`example'" == "exact_controls", 1200, 60000)
    }
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`caller_signature'"'
}

// The example ends with inference(highrank), whose e(V) lincom uses; its
// inference(q1) call runs first. Both are explicit requests, so the point-only
// switch of the suite does not change them.
fevc_run component_inference using fevc.sthlp
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_SE"
assert "`e(inference_model)'" == "repeat_measurement"
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert rowsof(`compat_assert_V') == 4 & colsof(`compat_assert_V') == 4
matrix drop `compat_assert_V'
assert rowsof(e(component_inference)) == 4
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// Example 6: inference(highrank) gives normal intervals and posts e(V); it
// finds the leading directions too, so that the display shows their shares.
fevc_run highrank_inference using fevc.sthlp
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_SE"
assert "`e(inference_variance_fit)'" == "repeat_measurement"
assert "`e(inference_leading_request)'" == "highrank"
assert e(inference_joint_posted) == 1 & e(inference_q1_components) == 0
tempname help_V help_ci help_table
matrix `help_V' = e(V)
assert rowsof(`help_V') == 4 & colsof(`help_V') == 4
matrix `help_ci' = e(component_inference)
assert rowsof(`help_ci') == 4
matrix `help_table' = e(inference_intervals)
forvalues target = 1/4 {
    assert el(`help_ci',`target',2) > 0 & el(`help_ci',`target',2) < .
    assert el(`help_table',`target',7) == 0
    // The help: no direction carries more than a few per cent here.
    if `target' < 4 assert el(`help_table',`target',10) < .1
}
confirm matrix e(inference_leading)
matrix drop `help_V' `help_ci' `help_table'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
// The help says the default would give the same normal intervals here.
preserve
quietly fevc, simulate_data(ex2) clear
quietly fevc log_wage productivity i.period, worker(worker_id) ///
    firm(firm_id) inference(auto) nodisplay
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_SE"
assert e(inference_q1_components) == 0 & e(inference_joint_posted) == 1
restore

// Example 7: inference(q1) on the bottleneck of the ex4 network gives the
// q = 1 interval to all four components and withholds e(V).
fevc_run q1_inference using fevc.sthlp
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_Q1"
assert "`e(inference_leading_request)'" == "q1"
assert e(inference_q1_components) == 4
assert e(inference_joint_posted) == 0
capture confirm matrix e(V)
assert _rc
tempname help_intervals
matrix `help_intervals' = e(inference_intervals)
forvalues target = 1/4 {
    assert el(`help_intervals',`target',7) == 1
    assert el(`help_intervals',`target',8) == el(`help_intervals',`target',5)
    assert el(`help_intervals',`target',9) == el(`help_intervals',`target',6)
    assert el(`help_intervals',`target',8) < el(`help_intervals',`target',9)
}
matrix drop `help_intervals'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
// The help says the default gives q = 1 here only for the firm variance and
// the covariance, and then posts no e(V).
preserve
quietly fevc, simulate_data(ex4) clear
quietly fevc log_wage, worker(worker_id) firm(firm_id) inference(auto) nodisplay
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_Q1"
tempname help_auto
matrix `help_auto' = e(inference_intervals)
assert el(`help_auto',1,7) == 0 & el(`help_auto',2,7) == 1
assert el(`help_auto',3,7) == 1 & el(`help_auto',4,7) == 0
assert e(inference_q1_components) == 2 & e(inference_joint_posted) == 0
matrix drop `help_auto'
restore

// Example 8: the default on the data of Example 4 and the three detailed
// displays. A suite that switches the default off gets point estimates,
// and estat inference then says that there are no standard errors.
fevc_run postestimation using fevc.sthlp
if lower(strtrim(`"`: environment FEVC_DEFAULT_INFERENCE'"'))=="none" | ///
    lower(strtrim("$FEVC_DEFAULT_INFERENCE"))=="none" {
    assert "`e(inference_variance_fit)'" == ""
    capture confirm matrix e(inference_intervals)
    assert _rc
}
else {
    // As the help says: the worker variance keeps the normal interval
    // (lead share below 10%); the firm variance and the covariance, whose
    // leading direction carries most of their estimated variance (more
    // than all of it on these data), receive the q = 1 interval.
    assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_Q1"
    tempname help_details
    matrix `help_details' = e(inference_intervals)
    assert el(`help_details',1,7) == 0 & el(`help_details',1,10) < .1
    assert el(`help_details',2,7) == 1 & el(`help_details',2,10) > 1
    assert el(`help_details',3,7) == 1 & el(`help_details',3,10) > 1
    assert "`e(inference_leading_request)'" == "auto"
    matrix drop `help_details'
}
assert e(N) == 440
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

fevc_run projection_inference using fevc.sthlp
assert "`e(status)'" == "KSS_PROJECTION_INFERENCE"
assert "`e(deletion)'" == "match"
assert "`e(stayers)'" == "both"
assert e(N_stayers) == 0
assert e(N) == 440
assert "`e(projection_effect)'" == "firm"
assert "`e(projection_weight)'" == "frequency"
assert rowsof(e(projection_results)) == 2
assert colsof(e(projection_results)) == 7
assert abs(el(e(projection_results),2,1)-.5) < .05
assert el(e(projection_results),2,5) > 0
assert el(e(projection_results),2,5) < .5
assert el(e(projection_results),2,6) > .5
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

capture noisily fevc_run missing_example using fevc.sthlp
assert _rc == 111
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

capture noisily fevc_run exact_controls using missing_varcomp_help.sthlp
assert _rc == 601
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

di as result "PASS test_help_examples.do"
