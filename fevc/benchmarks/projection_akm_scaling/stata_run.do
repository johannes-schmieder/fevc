version 18.0
clear all
set more off
set varabbrev off
set linesize 255

args package_root input_csv input_sha output_csv phase_start phase_end role source_commit rows_arg probes_arg seed_arg cores_arg stata_processors_arg memory_arg wall_arg maxiter_arg
local rows = real("`rows_arg'")
local probes = real("`probes_arg'")
local seed = real("`seed_arg'")
local cores = real("`cores_arg'")
local stata_processors = real("`stata_processors_arg'")
local memory = real("`memory_arg'")
local wall = real("`wall_arg'")
local maxiter = real("`maxiter_arg'")
local registered_rows = inlist(`rows',6000,480000,1920000,7680000)
local registered_probes = (`rows'==6000 & `probes'==1256) |              ///
    (`rows'==480000 & `probes'==1888) | (`rows'==1920000 & `probes'==2088) | ///
    (`rows'==7680000 & `probes'==2288)
if !inlist("`role'","exact","rust") | !`registered_rows' | !`registered_probes' | ///
   !inlist(`cores',4,16) | `stata_processors'!=min(4,`cores') |       ///
   !inlist(`memory',16,64,128,256) | `wall'<1 |                      ///
   (`rows'==6000 & `maxiter'!=20000) | (`rows'>6000 & `maxiter'!=40000) | ///
   !ustrregexm("`source_commit'","^[0-9a-f]{40}$") |                 ///
   !ustrregexm("`input_sha'","^[0-9a-f]{64}$") | `seed'<1 |         ///
   ("`role'"=="exact" & (`rows'!=6000 | `cores'!=4)) {
    di as error "invalid projection-AKM Stata arguments"
    exit 198
}

adopath ++ `"`package_root'/fevc"'
confirm file `"`package_root'/fevc/fevc.ado"'
if "`role'"=="rust" {
    if strpos(c(machine_type),"Mac") {
        confirm file `"`package_root'/fevc/fevc_rust_macos_arm64.plugin"'
    }
    else {
        confirm file `"`package_root'/fevc/fevc_rust_linux_x64.plugin"'
    }
}
set processors `stata_processors'
import delimited using `"`input_csv'"', clear varnames(1) asdouble bindquote(strict)
assert _N==`rows'
isid observation_key
isid worker period
assert worker==floor(worker) & firm==floor(firm)
quietly count if missing(observation_key,worker,firm,period,y,z1,z2)
assert r(N)==0
quietly bysort worker (period): assert _N==6
quietly bysort worker firm: generate byte first_match = _n==1
quietly bysort worker: egen byte firm_count = total(first_match)
assert firm_count==3
drop first_match firm_count
sort observation_key

quietly _datasignature
local data_signature `"`r(datasignature)'"'
set rng default
set seed 20260830
set sortseed 20260830
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'

tempname marker
file open `marker' using `"`phase_start'"', write text replace
file write `marker' "START `role'" _n
file close `marker'
timer clear 80
timer on 80
if "`role'"=="exact" {
    capture noisily fevc y, worker(worker) firm(firm)                   ///
        deletion(observation) algorithm(exact) backend(mata)            ///
        exact_limit(2000) project(z1 z2) projecteffect(firm)            ///
        projectweight(frequency) nodisplay
}
else {
    capture noisily fevc y, worker(worker) firm(firm)                   ///
        deletion(observation) algorithm(jla) engine(generic)            ///
        backend(rust) rng(counter_v1) preconditioner(cmg)                ///
        batch(16) probes(`probes') seed(`seed') maxiter(`maxiter')       ///
        memory_gib(`memory') wallseconds(`wall') project(z1 z2)          ///
        projecteffect(firm) projectweight(frequency) nodisplay
}
local command_rc = _rc
timer off 80
tempname endmarker
file open `endmarker' using `"`phase_end'"', write text replace
file write `endmarker' "END `role' rc=`command_rc'" _n
file close `endmarker'
if `command_rc' exit `command_rc'
timer list 80
local command_seconds = r(t80)

matrix b = e(projection_b)
matrix V = e(projection_V)
matrix V_naive = e(projection_V_naive)
assert rowsof(b)==1 & colsof(b)==3 & rowsof(V)==3 & colsof(V)==3
assert e(N_stored)==`rows' & e(N_physical)==`rows' & e(N_retained)==`rows'
tempname covariance_minimum covariance_maximum
mata: st_numscalar("`covariance_minimum'",min(symeigenvalues(st_matrix("V"))))
mata: st_numscalar("`covariance_maximum'",max(symeigenvalues(st_matrix("V"))))
local covariance_min = scalar(`covariance_minimum')
local covariance_max = scalar(`covariance_maximum')
assert `covariance_min'>=-1e-8*max(1,`covariance_max')

local psd_cleanup = 0
local projection_residual = .
local residual_tolerance = .
local memory_forecast = .
local memory_limit = .
local projection_iterations = .
local solver_iterations = .
local solver_complete_residual = .
local gram_rcond = .
local gram_relres = .
local gram_original_relres = .
local route_levels = .
local route_vertices = .
local route_edges = .
local route_terminal = .
local route_code = .
local rust_requested_route = .
local rust_selected_route = .
local rust_solver_fallback = .
local rust_solver_fallback_error = .
local setup_seconds = .
local leverage_seconds = .
local projection_peak = .
if "`role'"=="rust" {
    assert abs(e(projection_covariance_min)-`covariance_min')<=1e-12*max(1,abs(`covariance_min'))
    assert abs(e(projection_covariance_max)-`covariance_max')<=1e-12*max(1,abs(`covariance_max'))
    assert e(projection_psd_cleanup)<=1e-8*max(1,e(projection_covariance_max))
    assert "`e(backend_selected)'"=="rust" & "`e(algorithm)'"=="jla"
    assert "`e(engine_selected)'"=="generic"
    assert "`e(preconditioner_selected)'"=="CMG"
    assert "`e(fallback_status)'"=="NOT_NEEDED"
    assert "`e(rng_selected)'"=="counter_v1"
    assert e(route_code)==3 & e(rust_requested_route)==3
    assert e(rust_selected_route)==3
    assert e(rust_solver_fallback)==0 & e(rust_solver_fallback_error)==0
    assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
    assert e(projection_solver_max_complete)<=e(residual_acceptance_tolerance)
    assert e(projection_peak_forecast_bytes)<=e(batch_memory_budget_bytes)
    matrix PD = e(projection_diagnostics)
    assert PD[1,5]>0 & PD[1,5]<=1
    assert PD[1,6]<=e(residual_acceptance_tolerance)
    assert PD[1,7]<=e(residual_acceptance_tolerance)
    local projection_residual = e(projection_solver_max_complete)
    local residual_tolerance = e(residual_acceptance_tolerance)
    local memory_forecast = e(memory_forecast_bytes)
    local memory_limit = `memory'*1024^3
    assert `memory_forecast'<=`memory_limit'
    local projection_iterations = e(projection_solver_iterations)
    local solver_iterations = e(solver_iterations)
    local solver_complete_residual = e(complete_residual_max)
    local psd_cleanup = e(projection_psd_cleanup)
    local gram_rcond = PD[1,5]
    local gram_relres = PD[1,6]
    local gram_original_relres = PD[1,7]
    local route_levels = e(route_hierarchy_levels)
    local route_vertices = e(route_hybrid_vertices)
    local route_edges = e(route_hybrid_edges)
    local route_terminal = e(route_terminal_vertices)
    local route_code = e(route_code)
    local rust_requested_route = e(rust_requested_route)
    local rust_selected_route = e(rust_selected_route)
    local rust_solver_fallback = e(rust_solver_fallback)
    local rust_solver_fallback_error = e(rust_solver_fallback_error)
    local setup_seconds = e(setup_seconds)
    local leverage_seconds = e(leverage_seconds)
    local projection_peak = e(projection_peak_forecast_bytes)
}

quietly _datasignature
assert `"`r(datasignature)'"'==`"`data_signature'"'
assert `"`c(rngstate)'"'==`"`rng_before'"'
assert `"`c(sortrngstate)'"'==`"`sort_rng_before'"'

clear
set obs 1
generate str44 schema = "FEVC-PROJECTION-AKM-STATA-V2"
generate str8 role = "`role'"
generate str4 status = "PASS"
generate str40 source_commit = "`source_commit'"
generate str64 input_sha256 = "`input_sha'"
generate long rows = `rows'
generate long workers = `rows'/6
generate long firms = `rows'/12
generate int probes = `probes'
generate long seed = `seed'
generate byte active_cores = `cores'
generate byte stata_processors = `stata_processors'
generate long maxiter_budget = `maxiter'
generate double command_seconds = `command_seconds'
generate double probe_throughput = `probes'/`command_seconds'
generate double b_cons = b[1,1]
generate double b_z1 = b[1,2]
generate double b_z2 = b[1,3]
forvalues left = 1/3 {
    forvalues right = 1/3 {
        generate double V_`left'_`right' = V[`left',`right']
        generate double V_naive_`left'_`right' = V_naive[`left',`right']
    }
}
generate double covariance_min = `covariance_min'
generate double covariance_max = `covariance_max'
generate double psd_cleanup = `psd_cleanup'
generate double projection_complete_residual = `projection_residual'
generate double solver_complete_residual = `solver_complete_residual'
generate double residual_tolerance = `residual_tolerance'
generate double memory_forecast_bytes = `memory_forecast'
generate double memory_limit_bytes = `memory_limit'
generate double projection_peak_forecast_bytes = `projection_peak'
generate double projection_solver_iterations = `projection_iterations'
generate double solver_iterations = `solver_iterations'
generate double projection_gram_rcond = `gram_rcond'
generate double projection_gram_relres = `gram_relres'
generate double projection_gram_original_relres = `gram_original_relres'
generate double route_hierarchy_levels = `route_levels'
generate double route_hybrid_vertices = `route_vertices'
generate double route_hybrid_edges = `route_edges'
generate double route_terminal_vertices = `route_terminal'
generate double route_code = `route_code'
generate double rust_requested_route = `rust_requested_route'
generate double rust_selected_route = `rust_selected_route'
generate double rust_solver_fallback = `rust_solver_fallback'
generate double rust_solver_fallback_error = `rust_solver_fallback_error'
generate double setup_seconds = `setup_seconds'
generate double leverage_seconds = `leverage_seconds'
export delimited using `"`output_csv'"', replace
di as result "FEVC PROJECTION AKM STATA PASS `role' rows=`rows' cores=`cores'"
exit 0
