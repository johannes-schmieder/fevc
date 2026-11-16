* End-to-end smoke of the package with the plugin under test: a small simulated panel, both deletion modes, the
* automatic interval of the repeat-measurement model, and a request that must fail.
* Usage: do smoke.do <pkgroot> <threads>
version 18.0
clear all
set more off
args pkgroot threads
adopath ++ "`pkgroot'"
set seed 20261113
set obs 3000
generate long worker = _n
generate double a = .3*rnormal()
expand 5
bysort worker: generate int year = _n
generate int firm = 1+floor(60*runiform()) if year==1
bysort worker (year): replace firm = cond(runiform()<.15,1+floor(60*runiform()),firm[_n-1]) if year>1
generate double psi = .15*sin(firm*1.7)
generate double y = a+psi+.2*rnormal()
foreach deletion in match observation {
    * The plain default command.
    fevc y, worker(worker) firm(firm) deletion(`deletion') nativethreads(`threads')
    assert "`e(inference_model)'"=="repeat_measurement" & "`e(backend_selected)'"=="rust"
    assert e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
    assert inlist("`e(status)'","FEVC_REPEAT_MEASUREMENT_SE","FEVC_REPEAT_MEASUREMENT_Q1")
    matrix T = e(inference_intervals)
    assert rowsof(T)==4 & colsof(T)==16
    forvalues j = 1/4 {
        assert T[`j',8]<T[`j',1] & T[`j',1]<T[`j',9]
    }
    display "SMOKE deletion `deletion' status `e(status)' retained " e(N_retained) " q1 components " e(inference_q1_components)
}
capture noisily fevc y, worker(worker) firm(firm) nativethreads(`threads') inferencemodel(no_such_model)
assert _rc!=0
display "E2E SMOKE PASS"
