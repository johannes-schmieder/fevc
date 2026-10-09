version 18.0
// The default no-control call selects CMG_FULL_V2, whose probe phase uses the
// registered 1e-6 tolerance and 1e-5 complete-residual gate. All-probe replay
// reuses that route, so its residual may exceed 10*tolerance(); the transport
// check must apply the route's gate rather than withhold the diagnostic.
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
quietly fevc_rust probe
assert r(numerical_api)==2
if "`c(os)'"=="Windows" {
    di as result "SKIP test_full_cmg_default_mcse.do: CMG_FULL_V2 is not a Windows automatic route"
    exit 0
}
set processors 4
quietly fevc, simulate_data(ex1) clear
quietly fevc log_wage, worker(worker_id) firm(firm_id) nolog nodisplay
assert "`e(cmg_backend)'"=="CMG_FULL_V2"
assert "`e(mcse_status)'"=="ok_local" & e(mcse_available)==1
assert e(residual_acceptance_tolerance)>10*e(tolerance)
assert e(mc_replay_max_residual)>10*e(tolerance)
assert e(mc_replay_max_residual)<=e(residual_acceptance_tolerance)
tempname auto fixed
matrix `auto'=e(results)
// An explicit batch selects the serial route at the 10*tolerance() gate.
quietly fevc log_wage, worker(worker_id) firm(firm_id) nolog nodisplay batch(16)
assert "`e(cmg_backend)'"==""
assert e(mc_replay_max_residual)<=max(1e-11,10*e(tolerance))
matrix `fixed'=e(results)
mata: assert(mreldif(st_matrix("`auto'")[3,.],st_matrix("`fixed'")[3,.])<1e-6)
mata: assert(mreldif(st_matrix("`auto'")[4,.],st_matrix("`fixed'")[4,.])<1e-4)
di as result "PASS test_full_cmg_default_mcse.do"
