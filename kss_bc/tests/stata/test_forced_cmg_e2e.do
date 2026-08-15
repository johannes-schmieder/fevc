version 18.0

clear
set more off
set varabbrev off

capture confirm file "shared/cmg/generated/kssbc_cmg_core.mata"
if _rc {
    di as error "run the forced-CMG estimator test from the repository root"
    exit 601
}

// A deterministic moderate repeated-RHS graph that survives the public
// mover and leave-worker-out sample gates without dropping observations.
local workers = 1200
local firms = 300
local degree = 3
set obs `=`degree'*`workers''
generate long worker = floor((_n-1)/`degree') + 1
generate byte link = mod(_n-1,`degree')
generate long firm = mod(worker-1+cond(link==2,17,link),`firms') + 1
generate double outcome = sin(worker/37) + cos(firm/19) + link/101

kss_bc outcome, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probes(40) batch(8) seed(8675309) ///
    tolerance(1e-10) maxiter(10000) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
matrix b1_results = e(results)
matrix b1_rhs = e(solver_rhs_diagnostics)
scalar b1_N = e(N_retained)
scalar b1_setup = e(setup_seconds)
scalar b1_schur = e(schur_seconds)
scalar b1_apply = e(preconditioner_apply_seconds)
scalar b1_pcg = e(pcg_seconds)
scalar b1_leverage = e(leverage_seconds)
scalar b1_target = e(target_seconds)
generate byte b1_sample = e(sample)
local b1_rngstate `"`c(rngstate)'"'

capture mata: kssbc_cmg__api_level()
if _rc quietly do "shared/cmg/generated/kssbc_cmg_core.mata"
capture mata: kssbc__planned_rhs(40,0,"joint")
if _rc quietly do "kss_bc/tests/support/kss_cmg_adapter.mata"
mata: mata drop kssbc__stata_jla()
quietly do "kss_bc/tests/support/kss_cmg_bridge_override.mata"
global KSSBC_CMG_MEMORY_GIB 4

kss_bc outcome, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probes(40) batch(8) seed(8675309) ///
    tolerance(1e-10) maxiter(10000) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "$KSSBC_CMG_STATUS" == "CONVERGED"
assert e(N_retained) == b1_N
assert b1_sample == e(sample)
assert mreldif(b1_results,e(results)) <= 2e-9
assert `"`c(rngstate)'"' == `"`b1_rngstate'"'
assert e(setup_seconds) > 0
assert e(schur_seconds) > 0
assert e(preconditioner_apply_seconds) > 0
assert e(pcg_seconds) > 0
assert e(leverage_seconds) > 0
assert e(target_seconds) > 0
assert rowsof(e(solver_rhs_diagnostics)) == rowsof(b1_rhs)
assert colsof(e(solver_rhs_diagnostics)) == 6

matrix cmg_rhs = e(solver_rhs_diagnostics)
mata:
rhs = st_matrix("cmg_rhs")
assert(rows(rhs) == 121)
assert(max(rhs[.,5]) <= 1e-9)
assert(min(rhs[.,6]) == 1)
assert(sum(rhs[.,1]:==4) == 40)
assert(sum(rhs[.,1]:==5) == 80)
route = st_matrix("KSSBC_CMG_ROUTE_DIAGNOSTICS")
assert(route[1] == 121)
assert(route[2] == 4*1024^3)
assert(route[3] > 0)
assert(route[4] >= 1)
end

macro drop KSSBC_CMG_MEMORY_GIB KSSBC_CMG_STATUS KSSBC_CMG_MESSAGE
di as result "PASS test_forced_cmg_e2e.do"
