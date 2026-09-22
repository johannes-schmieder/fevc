version 18.0

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

fevc_run component_inference using fevc.sthlp
assert "`e(status)'" == "KSS_HIGHRANK_INFERENCE"
assert rowsof(e(V)) == 4 & colsof(e(V)) == 4
assert rowsof(e(component_inference)) == 4
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
