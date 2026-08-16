version 18.0
clear all
set more off
set varabbrev off

capture confirm file "kss_bc/kss_bc.mata"
if _rc {
    di as error "run the solver production test from the repository root"
    exit 601
}
adopath ++ "`c(pwd)'/kss_bc"

quietly do "kss_bc/kss_bc.mata"
quietly do "kss_bc/kss_bc_cmg.mata"
quietly do "kss_bc/kss_bc_solver.mata"

mata:
void test_solver_prod_fixes()
{
real scalar dwork, workers, firms, degree, repetition
real colvector worker, link, firm, frequency, worker_key, firm_key
real matrix rhs
struct kssbc_solve_result scalar pilot
struct kssbc_fe_design scalar design
struct kssbc_cmg__cells scalar cells
struct kssbc_cmg__options scalar options
struct kssbc_cmg__preflight_result scalar preflight
struct kssbc_cmg__hierarchy scalar hierarchy, hierarchy_reused
struct kssbc_solver_cmg_context scalar context
struct kssbc_cmg__apply_result scalar ordinary, reused, cells_applied

assert(kssbc_solver__cmg_runtime_ok())

// Routing work is a pure function of registered dimensions and iteration
// diagnostics.  No timer or wall-clock result is an argument.
dwork = kssbc_solver__diagonal_work(601,12,256472,4063,1285)
assert(dwork == kssbc_solver__diagonal_work(601,12,256472,4063,1285))
assert(dwork > kssbc_solver__diagonal_work(601,4,256472,4063,1285))
assert(kssbc_solver__cmg_work(
    601,1,256472,4063,1285,1796,5948,1,1,8*1796^2/2) > 0)
assert(kssbc_solver__cmg_work_wins(0.80))
assert(!kssbc_solver__cmg_work_wins(0.8000001))

// A 601-RHS, 95-step B1 projection is never a permitted automatic fallback.
assert(!kssbc_solver__fallback_work_ok(
    kssbc_solver__diagonal_work(601,95,7000000,100000,20000)))

pilot.status = "CONVERGED"
pilot.rhs_status = J(1,4,"CONVERGED")
pilot.rhs_iterations = (95,94,93,92)
pilot.rhs_relres = J(1,4,1e-12)
assert(!kssbc_solver__pilots_valid(pilot,32,1e-10))
pilot.rhs_iterations = J(1,4,2)
pilot.rhs_relres[4] = 2e-9
assert(!kssbc_solver__pilots_valid(pilot,32,1e-10))
pilot.rhs_relres[4] = 1e-12
assert(kssbc_solver__pilots_valid(pilot,32,1e-10))

// The reusable API preserves the ordinary KSS pullback action to numerical
// precision even though measured scale results keep it off in production.
workers = 1200
firms = 300
degree = 3
worker = floor(((1::(degree*workers)):-1)/degree):+1
link = mod((1::(degree*workers)):-1,degree)
firm = mod(worker:-1:+(link:==2):*17:+(link:!=2):*link,firms):+1
frequency = J(rows(worker),1,1)
design = kssbc__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")
worker_key = kssbc_solver__canonical_keys(worker,workers)
firm_key = kssbc_solver__canonical_keys(firm,firms)
cells = kssbc_cmg__cells_prepare(
    design.worker,design.firm,design.frequency,worker_key,firm_key)
assert(cells.status == "CONVERGED")
options = kssbc_cmg__options_resource(0.65*4*1024^3,firms,121)
preflight = kssbc_cmg__preflight(
    cells,121,1,0.65*4*1024^3,options)
assert(preflight.status == "CONVERGED")
timer_clear(82)
timer_clear(83)
timer_on(82)
hierarchy_reused = kssbc_solver__hierarchy_cells(
    cells,0.65*4*1024^3,121)
timer_off(82)
timer_on(83)
hierarchy = kssbc_solver__hierarchy(
    design,worker_key,firm_key,0.65*4*1024^3,121)
timer_off(83)
st_numscalar("solver_cells_reused",kssbc__timer_seconds(82))
st_numscalar("solver_cells_wrapper",kssbc__timer_seconds(83))
assert(hierarchy.status == "CONVERGED")
assert(hierarchy_reused.status == "CONVERGED")
assert(hierarchy_reused.n_level == hierarchy.n_level)
assert(hierarchy_reused.edge_complexity == hierarchy.edge_complexity)
assert(hierarchy_reused.vertex_complexity == hierarchy.vertex_complexity)
assert(hierarchy_reused.structural_bytes == hierarchy.structural_bytes)
assert(hierarchy_reused.dense_factor_bytes == hierarchy.dense_factor_bytes)
assert(mreldif(hierarchy_reused.attempted_level_table,
    hierarchy.attempted_level_table) <= 1e-15)
context = kssbc_solver__cmg_context(&hierarchy,8,0.65*4*1024^3,1)
assert(context.use_workspace)
rhs = runiform(firms,4):-0.5
rhs = rhs:-mean(rhs)
ordinary = kssbc_cmg__apply_kss(hierarchy,rhs)
cells_applied = kssbc_cmg__apply_kss(hierarchy_reused,rhs)
reused = kssbc_solver__cmg_apply_ws(context,rhs)
assert(ordinary.status == "CONVERGED")
assert(cells_applied.status == "CONVERGED")
assert(reused.status == "CONVERGED")
assert(mreldif(ordinary.value,cells_applied.value) <= 1e-15)
assert(mreldif(ordinary.value,reused.value) <= 1e-12)
timer_clear(80)
timer_clear(81)
timer_on(80)
for (repetition=1; repetition<=40; repetition++) {
    ordinary = kssbc_cmg__apply_kss(hierarchy,rhs)
}
timer_off(80)
timer_on(81)
for (repetition=1; repetition<=40; repetition++) {
    reused = kssbc_solver__cmg_apply_ws(context,rhs)
}
timer_off(81)
st_numscalar("solver_ws_ordinary",kssbc__timer_seconds(80))
st_numscalar("solver_ws_reused",kssbc__timer_seconds(81))
}
test_solver_prod_fixes()
end

// Exercise the public estimator with the production ordinary-batch adapter.
// Complete original-system RHS diagnostics remain the acceptance evidence.
clear
local workers = 1200
local firms = 300
local degree = 3
set obs `=`degree'*`workers''
generate long worker = floor((_n-1)/`degree') + 1
generate byte link = mod(_n-1,`degree')
generate long firm = mod(worker-1+cond(link==2,17,link),`firms') + 1
generate double outcome = sin(worker/37) + cos(firm/19) + link/101
kss_bc outcome, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) preconditioner(cmg) memory_gib(4) ///
    probes(4) batch(4) seed(8675309) tolerance(1e-10) ///
    maxiter(10000) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "`e(preconditioner_selected)'" == "CMG"
assert e(memory_forecast_bytes) <= 4*1024^3
matrix solver_rhs = e(solver_rhs_diagnostics)
matrix cmg_route = e(route_diagnostics)
scalar solver_base_bytes = 8*(8*e(N_retained)+ ///
    5*(e(worker_levels)+e(firm_levels)))
assert cmg_route[1,25] >= solver_base_bytes+ ///
    cmg_route[1,7]+cmg_route[1,8]
assert cmg_route[1,25]+e(batch_scratch_forecast_bytes) <= 4*1024^3
mata: assert(max(st_matrix("solver_rhs")[.,5]) <= 1e-9)
di as result "CMG workspace benchmark, 40 x 4 RHS: ordinary=" ///
    %9.6f solver_ws_ordinary "s reusable=" %9.6f solver_ws_reused "s"
di as result "CMG hierarchy setup: reused cells=" ///
    %9.6f solver_cells_reused "s wrapper=" %9.6f solver_cells_wrapper "s"

// Explicit diagonal B1 bypasses CMG preflight and its memory forecast.
kss_bc outcome, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) preconditioner(diagonal) memory_gib(4) ///
    probes(2) batch(2) seed(8675309) tolerance(1e-10) ///
    maxiter(10000) nodisplay
assert "`e(preconditioner_selected)'" == "DIAGONAL"
matrix diagonal_route = e(route_diagnostics)
assert diagonal_route[1,14] == 0
assert diagonal_route[1,15] == 0
assert diagonal_route[1,16] == 0
assert diagonal_route[1,17] == 0
assert diagonal_route[1,25]+e(batch_scratch_forecast_bytes) <= 4*1024^3

// A graph just above the dense-terminal cap forces a multilevel CMG pilot.
// maxiter(1) makes failure deterministic and occurs before the probe stream.
clear
local firms = 6200
local workers = 24800
local degree = 3
set obs `=`degree'*`workers''
generate long worker = floor((_n-1)/`degree') + 1
generate byte link = mod(_n-1,`degree')
generate long firm = mod(worker-1+cond(link==2,17,link),`firms') + 1
generate double outcome = sin(worker/37) + cos(firm/19) + link/101
local rng_before_failure `"`c(rngstate)'"'
capture noisily kss_bc outcome, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) preconditioner(cmg) memory_gib(4) ///
    probes(2) batch(2) seed(8675309) tolerance(1e-10) ///
    maxiter(1) nodisplay
assert _rc == 498
assert "`e(status)'" == "WITHHELD"
assert `"`c(rngstate)'"' == `"`rng_before_failure'"'
matrix failed_route = e(route_diagnostics)
assert rowsof(failed_route) == 1
assert colsof(failed_route) == 26
assert failed_route[1,1] == 7
assert failed_route[1,11] > 0
assert failed_route[1,12] > 0
assert failed_route[1,18] == 128
assert failed_route[1,25]+e(batch_scratch_forecast_bytes) <= 4*1024^3
assert e(route_terminal_vertices) > 0 & e(route_terminal_vertices) <= 6144
assert strpos(lower(`"`e(routing_reason)'"'),"converged") == 0

di as result "PASS test_solver_prod_fixes.do"
exit 0
