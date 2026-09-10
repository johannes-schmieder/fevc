version 18.0
clear all
set more off
set seed 12345
// Reproduce the owner's uncentered simple_AKM.do without reghdfe. Keep its
// generated storage types, seed, dimensions and default estimator request.
set obs 10000
generate workerid=_n
expand 5
bys workerid: generate year=2000+_n-1
bys workerid(year): generate firmid=ceil(runiform()*500) if _n==1
bys workerid(year): replace firmid=cond(runiform()<.15,ceil(runiform()*500),firmid[_n-1]) if _n>1
bys workerid: generate worker_fe=rnormal(0,.4) if _n==1
bys workerid: replace worker_fe=worker_fe[1]
bys firmid: generate firm_fe=rnormal(0,.2) if _n==1
bys firmid: replace firm_fe=firm_fe[1]
generate experience=year-2000+runiform()*5
generate x=rnormal()
generate lnwage=2+worker_fe+firm_fe+.03*experience-.0005*experience^2+.10*x+rnormal(0,.2)
order workerid firmid year lnwage experience x
sort workerid year
generate exp2=experience^2
quietly summarize experience
scalar c_exp=r(mean)
generate double centered_exp=experience-c_exp
generate double centered_exp2=exp2-2*c_exp*experience+c_exp^2
quietly summarize centered_exp2
replace centered_exp2=centered_exp2-r(mean)
local caller_rng `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'
capture quietly fevc_rust probe
local has_rust=(_rc==0)
local backends mata
if `has_rust' local backends mata rust auto
foreach selected_backend of local backends {
    fevc lnwage experience exp2, worker(workerid) firm(firmid) ///
        backend(`selected_backend') nodisplay
    assert e(N)==50000 & e(N_retained)==50000 & e(N_physical)==50000
    count if e(sample)
    assert r(N)==50000
    assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
    mata: assert(!hasmissing(st_matrix("e(results)")))
    matrix akm_`selected_backend'=e(results)
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    local current_sortedby : sortedby
    assert `"`current_sortedby'"'==`"`caller_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"'==`"`caller_signature'"'
    if "`selected_backend'"!="mata" {
        assert "`e(backend_selected)'"=="rust"
        // Registered independent-draw corrected-result equivalence gate.
        mata: a=st_matrix("akm_mata"); b=st_matrix("akm_`selected_backend'")
        mata: assert(all(abs(a[3,.]-b[3,.]):<=rowmax((J(4,1,1e-8),6*sqrt(a[4,.]':^2+b[4,.]':^2)))'))
        assert e(rust_full_fit_complete_residual)<=e(residual_acceptance_tolerance)
        quietly fevc_rust clear
    }
}
fevc lnwage centered_exp centered_exp2, worker(workerid) firm(firmid) ///
    backend(mata) nodisplay
assert e(N)==50000
// Same draws and full FE span: deterministic policy is a tighter diagnostic.
mata: a=st_matrix("akm_mata"); b=st_matrix("e(results)")
mata: assert(max(abs(a[3,.]-b[3,.]))<=1e-8)
if !`has_rust' display "SKIP AKM native routes: no local plugin"
display "PASS test_control_akm.do"
