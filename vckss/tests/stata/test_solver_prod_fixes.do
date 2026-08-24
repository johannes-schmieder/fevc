version 18.0
clear all
set more off
set varabbrev off

capture confirm file "vckss/vckss.mata"
if _rc {
    di as error "run the solver production test from the repository root"
    exit 601
}
adopath ++ "`c(pwd)'/vckss"

quietly do "vckss/vckss.mata"
quietly do "vckss/vckss_cmg.mata"
quietly do "vckss/vckss_solver.mata"

mata:
void test_solver_prod_fixes()
{
real scalar dwork, workers, firms, degree, repetition
real colvector worker, link, firm, frequency, worker_key, firm_key
real matrix rhs
struct vckss_solve_result scalar pilot
struct vckss_fe_design scalar design
struct vckss_cmg__cells scalar cells
struct vckss_cmg__options scalar options
struct vckss_cmg__preflight_result scalar preflight
struct vckss_cmg__hierarchy scalar hierarchy, hierarchy_reused
struct vckss_solver_cmg_context scalar context
struct vckss_cmg__apply_result scalar ordinary, reused, cells_applied

assert(vckss_solver__cmg_runtime_ok())

// Routing work is a pure function of registered dimensions and iteration
// diagnostics.  No timer or wall-clock result is an argument.
dwork = vckss_solver__diagonal_work(601,12,256472,4063,1285)
assert(dwork == vckss_solver__diagonal_work(601,12,256472,4063,1285))
assert(dwork > vckss_solver__diagonal_work(601,4,256472,4063,1285))
assert(vckss_solver__cmg_work(
    601,1,256472,4063,1285,1796,5948,1,1,8*1796^2/2) > 0)
assert(vckss_solver__cmg_work_wins(0.80))
assert(!vckss_solver__cmg_work_wins(0.8000001))

// A 601-RHS, 95-step B1 projection is never a permitted automatic fallback.
assert(!vckss_solver__fallback_work_ok(
    vckss_solver__diagonal_work(601,95,7000000,100000,20000)))

pilot.status = "CONVERGED"
pilot.rhs_status = J(1,4,"CONVERGED")
pilot.rhs_iterations = (95,94,93,92)
pilot.rhs_relres = J(1,4,1e-12)
assert(!vckss_solver__pilots_valid(pilot,32,1e-10))
pilot.rhs_iterations = J(1,4,2)
pilot.rhs_relres[4] = 2e-9
assert(!vckss_solver__pilots_valid(pilot,32,1e-10))
pilot.rhs_relres[4] = 1e-12
assert(vckss_solver__pilots_valid(pilot,32,1e-10))

// The reusable API preserves the ordinary KSS pullback action to numerical
// precision even though measured scale results keep it off in production.
workers = 1200
firms = 300
degree = 3
worker = floor(((1::(degree*workers)):-1)/degree):+1
link = mod((1::(degree*workers)):-1,degree)
firm = mod(worker:-1:+(link:==2):*17:+(link:!=2):*link,firms):+1
frequency = J(rows(worker),1,1)
design = vckss__fe_prepare(worker,firm,frequency,1e-10)
assert(design.status == "CONVERGED")
worker_key = vckss_solver__canonical_keys(worker,workers)
firm_key = vckss_solver__canonical_keys(firm,firms)
cells = vckss_cmg__cells_prepare(
    design.worker,design.firm,design.frequency,worker_key,firm_key)
assert(cells.status == "CONVERGED")
options = vckss_cmg__options_resource(0.65*4*1024^3,firms,121)
preflight = vckss_cmg__preflight(
    cells,121,1,0.65*4*1024^3,options)
assert(preflight.status == "CONVERGED")
timer_clear(82)
timer_clear(83)
timer_on(82)
hierarchy_reused = vckss_solver__hierarchy_cells(
    cells,0.65*4*1024^3,121)
timer_off(82)
timer_on(83)
hierarchy = vckss_solver__hierarchy(
    design,worker_key,firm_key,0.65*4*1024^3,121)
timer_off(83)
st_numscalar("solver_cells_reused",vckss__timer_seconds(82))
st_numscalar("solver_cells_wrapper",vckss__timer_seconds(83))
assert(hierarchy.status == "CONVERGED")
assert(hierarchy_reused.status == "CONVERGED")
assert(hierarchy_reused.n_level == hierarchy.n_level)
assert(hierarchy_reused.edge_complexity == hierarchy.edge_complexity)
assert(hierarchy_reused.vertex_complexity == hierarchy.vertex_complexity)
assert(hierarchy_reused.structural_bytes == hierarchy.structural_bytes)
assert(hierarchy_reused.dense_factor_bytes == hierarchy.dense_factor_bytes)
assert(mreldif(hierarchy_reused.attempted_level_table,
    hierarchy.attempted_level_table) <= 1e-15)
context = vckss_solver__cmg_context(&hierarchy,8,0.65*4*1024^3,1)
assert(context.use_workspace)
rhs = runiform(firms,4):-0.5
rhs = rhs:-mean(rhs)
ordinary = vckss_cmg__apply_kss(hierarchy,rhs)
cells_applied = vckss_cmg__apply_kss(hierarchy_reused,rhs)
reused = vckss_solver__cmg_apply_ws(context,rhs)
assert(ordinary.status == "CONVERGED")
assert(cells_applied.status == "CONVERGED")
assert(reused.status == "CONVERGED")
assert(mreldif(ordinary.value,cells_applied.value) <= 1e-15)
assert(mreldif(ordinary.value,reused.value) <= 1e-12)
timer_clear(80)
timer_clear(81)
timer_on(80)
for (repetition=1; repetition<=40; repetition++) {
    ordinary = vckss_cmg__apply_kss(hierarchy,rhs)
}
timer_off(80)
timer_on(81)
for (repetition=1; repetition<=40; repetition++) {
    reused = vckss_solver__cmg_apply_ws(context,rhs)
}
timer_off(81)
st_numscalar("solver_ws_ordinary",vckss__timer_seconds(80))
st_numscalar("solver_ws_reused",vckss__timer_seconds(81))
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
vckss outcome, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) preconditioner(cmg) memory_gib(4) ///
    engine(generic) ///
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
mata:
solver_rhs_values = st_matrix("solver_rhs")
solver_rhs_max = max(solver_rhs_values[.,5])
assert(solver_rhs_max <= 1e-9)
assert(abs(solver_rhs_max-st_numscalar("e(complete_residual_max)")) <=
    16*epsilon(max((1,solver_rhs_max))))
assert(st_numscalar("e(solver_max_residual)")+
    16*epsilon(max((1,solver_rhs_max))) >= solver_rhs_max)
end
di as result "CMG workspace benchmark, 40 x 4 RHS: ordinary=" ///
    %9.6f solver_ws_ordinary "s reusable=" %9.6f solver_ws_reused "s"
di as result "CMG hierarchy setup: reused cells=" ///
    %9.6f solver_cells_reused "s wrapper=" %9.6f solver_cells_wrapper "s"

// Call the routed Stata bridge with exactly its 32 required arguments.  The
// retired pilot-receipt arguments are optional compatibility slots; a legacy
// caller must neither dereference nor write them.
generate double legacy_frequency = 1
generate double legacy_target = 1
egen long legacy_match = group(worker firm)
generate byte legacy_sample = 1
generate long legacy_rank = _n
tempname legacy_results legacy_diagnostics legacy_rhs legacy_route
capture noisily mata: vckss__stata_jla_routed(                    ///
    "outcome", "worker", "firm", "",                          ///
    "legacy_frequency", "legacy_target", "legacy_match",       ///
    "legacy_sample", "legacy_rank", 0, "match", "joint",      ///
    2, 2, 8675309, 1e-10, 10000, 1e-10, 1e-10, 500,              ///
    "diagonal", 4*1024^3, "`legacy_results'",                  ///
    "legacy_status", "legacy_message",                         ///
    "`legacy_diagnostics'", "`legacy_rhs'", "`legacy_route'", ///
    "legacy_selected", "legacy_reason",                        ///
    "legacy_fallback_status", "legacy_fallback_message")
assert _rc == 0
assert "`legacy_status'" == "CONVERGED"
assert "`legacy_selected'" == "DIAGONAL"
assert rowsof(`legacy_results') == 4 & colsof(`legacy_results') == 4
assert rowsof(`legacy_diagnostics') == 1 &                        ///
    colsof(`legacy_diagnostics') == 32
assert rowsof(`legacy_rhs') == 7 & colsof(`legacy_rhs') == 6
assert rowsof(`legacy_route') == 1 & colsof(`legacy_route') == 26

// Explicit diagonal B1 bypasses CMG preflight and its memory forecast.
vckss outcome, worker(worker) firm(firm) deletion(match) ///
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

// A graph just above the dense-terminal cap constructs a multilevel CMG
// hierarchy. maxiter(1) then makes the estimator's convergence failure
// deterministic; the outer command must restore the caller's RNG state.
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
capture noisily vckss outcome, worker(worker) firm(firm) deletion(match) ///
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
forvalues reserved = 18/21 {
    assert missing(failed_route[1,`reserved'])
}
assert missing(failed_route[1,23])
assert missing(failed_route[1,24])
assert failed_route[1,25]+e(batch_scratch_forecast_bytes) <= 4*1024^3
assert e(route_terminal_vertices) > 0 & e(route_terminal_vertices) <= 6144
assert strpos(lower(`"`e(routing_reason)'"'),"converged") == 0

di as result "PASS test_solver_prod_fixes.do"
exit 0
