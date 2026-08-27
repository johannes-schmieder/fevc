version 18.0
clear all
set more off
set varabbrev off
set linesize 255

args source_root input_dta output_csv role source_commit task_sha input_sha probes_arg seed_arg wall_arg
local probes = real("`probes_arg'")
local seed = real("`seed_arg'")
local wall = real("`wall_arg'")
if "`role'"!="vckss" |                                      ///
   !ustrregexm("`source_commit'","^[0-9a-f]{40}$") |          ///
   !ustrregexm("`task_sha'","^[0-9a-f]{64}$") |              ///
   "`input_sha'"!="1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575" | ///
   !inlist(`probes',20,200) | `seed'!=8675309 |                 ///
   `wall'<600 | `wall'>42600 {
    di as error "invalid full-CMG fixed-CZ18 Stata arguments"
    exit 198
}
confirm file `"`source_root'/vckss/vckss.ado"'
if strpos("`c(machine_type)'", "Mac") == 1 {
    confirm file `"`source_root'/vckss/vckss_rust_macos_arm64.plugin"'
}
else if c(os)=="Unix" {
    confirm file `"`source_root'/vckss/vckss_rust_linux_x64.plugin"'
}
else {
    di as error "full-CMG fixed-CZ18 benchmark supports only macOS and Linux"
    exit 198
}
confirm file `"`input_dta'"'
adopath ++ `"`source_root'/vckss"'
capture set processors 4
if _rc | c(processors)!=4 {
    di as error "Stata/MP did not honor four processors"
    exit 459
}

timer clear 70
timer on 70
quietly use `"`input_dta'"', clear
timer off 70
timer list 70
local import_seconds = r(t70)
confirm numeric variable worker firm period y_minus_xb observation_key
keep worker firm period y_minus_xb observation_key
rename y_minus_xb y
assert _N==8201888
quietly count if missing(observation_key,worker,firm,period,y)
assert r(N)==0
assert worker==floor(worker) & firm==floor(firm) &              ///
    observation_key==floor(observation_key)
isid observation_key
quietly egen byte worker_tag=tag(worker)
quietly egen byte firm_tag=tag(firm)
quietly egen byte cell_tag=tag(worker firm)
quietly count if worker_tag
assert r(N)==117529
quietly count if firm_tag
assert r(N)==10603
quietly count if cell_tag
assert r(N)==311730
drop worker_tag firm_tag cell_tag
sort worker period firm observation_key

quietly _datasignature
local data_signature `"`r(datasignature)'"'
set rng default
set seed 20260825
set sortseed 20260825
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'
timer clear 80
timer on 80
capture noisily vckss y, worker(worker) firm(firm) deletion(match) ///
    probeorder(observation_key) backend(rust) rng(counter_v1)    ///
    algorithm(jla) engine(auto) preconditioner(auto)             ///
    memory_gib(48) wallseconds(`wall') probes(`probes')          ///
    batch(auto) seed(`seed') maxiter(20000) nodisplay
local command_rc = _rc
timer off 80
timer list 80
local command_seconds = r(t80)
if `command_rc' {
    di as error "VCKSS_FULL_CMG_PRODUCTION_CZ18_STATA_FAILURE rc=`command_rc'"
    di as error "status=`e(status)' withholding_status=`e(withholding_status)'"
    di as error "native_error_code=`e(native_error_code)' native_error_phase=`e(native_error_phase)'"
    di as error `"withholding_reason=`e(withholding_reason)'"'
    di as error `"withholding_detail=`e(withholding_detail)'"'
    di as error `"withholding_suggestion=`e(withholding_suggestion)'"'
    ereturn list
    local failure_data_restored = 0
    local failure_rng_restored = (`"`c(rngstate)'"'==`"`rng_before'"')
    local failure_sort_rng_restored = (`"`c(sortrngstate)'"'==`"`sort_rng_before'"')
    capture quietly _datasignature
    if !_rc local failure_data_restored = (`"`r(datasignature)'"'==`"`data_signature'"')
    di as error "failure_data_restored=`failure_data_restored' failure_rng_restored=`failure_rng_restored' failure_sort_rng_restored=`failure_sort_rng_restored'"
    exit `command_rc'
}

assert "`e(backend_requested)'"=="rust"
assert "`e(backend_selected)'"=="rust"
assert "`e(rng_selected)'"=="counter_v1"
assert "`e(algorithm)'"=="jla"
assert "`e(engine_selected)'"=="compressed"
assert "`e(preconditioner_selected)'"=="CMG"
assert "`e(cmg_backend)'"=="CMG_FULL_V2"
assert "`e(cmg_source_commit)'"=="dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
assert e(cmg_threads_requested)==4 & e(cmg_threads_used)==4
assert e(N_stored)==8201888 & e(N_retained)==8201888
assert e(worker_levels)==117529 & e(firm_levels)==10603
assert e(coefficient_cells)==311730 & e(deletion_units)==311730
assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
assert e(target_identity_residual)<=1e-12
matrix result = e(results)
assert rowsof(result)==4 & colsof(result)==4
tempvar in_sample
generate byte `in_sample'=e(sample)
quietly count if `in_sample'
local sample_count = r(N)
assert `sample_count'==8201888
drop `in_sample'
quietly _datasignature
local data_restored = (`"`r(datasignature)'"'==`"`data_signature'"')
local rng_restored = (`"`c(rngstate)'"'==`"`rng_before'"')
local sort_rng_restored = (`"`c(sortrngstate)'"'==`"`sort_rng_before'"')
assert `data_restored' & `rng_restored' & `sort_rng_restored'

matrix phase = e(rust_phase_profile)
assert rowsof(phase)==1 & colsof(phase)==8
matrix cmg = e(full_cmg_receipt)
assert rowsof(cmg)==1 & colsof(cmg)==46
assert cmg[1,1]==2 & cmg[1,5]==4 & cmg[1,6]==4
assert cmg[1,25]==1+3*`probes'
assert cmg[1,33]<=e(residual_acceptance_tolerance)
assert cmg[1,39]<=cmg[1,42] & cmg[1,17]<=cmg[1,42]
local status `e(status)'
local selected_batch = e(selected_batch)
local iterations = e(solver_iterations)
local max_residual = e(complete_residual_max)
local acceptance = e(residual_acceptance_tolerance)
local identity = e(target_identity_residual)
local memory_forecast = e(memory_forecast_bytes)
local resource_peak = e(resource_peak_bytes)
local graph = e(graph_seconds)
local compression = e(compression_seconds)
local setup = e(setup_seconds)
local work = e(life_work_seconds)
local fit = e(fit_seconds)
local leverage = e(leverage_seconds)
local target = e(target_seconds)
local correction = e(correction_seconds)
local rng = e(rng_seconds)
local schur = e(schur_seconds)
local pcg = e(pcg_seconds)
local cmg_backend `e(cmg_backend)'
local cmg_source `e(cmg_source_commit)'

clear
set obs 1
generate str48 schema = "VCKSS-FULL-CMG-PRODUCTION-CZ18-STATA-V1"
generate str9 role = "`role'"
generate str8 application_status = "PASS"
generate str40 source_commit = "`source_commit'"
generate str64 task_sha256 = "`task_sha'"
generate str64 input_sha256 = "`input_sha'"
generate double rows = 8201888
generate long workers = 117529
generate long firms = 10603
generate long coefficient_cells = 311730
generate int probes = `probes'
generate long seed = `seed'
generate byte processors = c(processors)
generate str48 estimator_status = "`status'"
generate str16 engine = "compressed"
generate str16 preconditioner = "CMG"
generate str16 cmg_backend = "`cmg_backend'"
generate str40 cmg_source_commit = "`cmg_source'"
generate double import_seconds = `import_seconds'
generate double command_seconds = `command_seconds'
generate double graph_seconds = `graph'
generate double compression_seconds = `compression'
generate double setup_seconds = `setup'
generate double work_seconds = `work'
generate double fit_seconds = `fit'
generate double leverage_seconds = `leverage'
generate double target_seconds = `target'
generate double correction_seconds = `correction'
generate double rng_seconds = `rng'
generate double schur_seconds = `schur'
generate double pcg_seconds = `pcg'
generate double ingest_seconds = phase[1,1]
generate double canonical_seconds = phase[1,2]
generate double native_graph_seconds = phase[1,3]
generate double native_compression_seconds = phase[1,4]
generate double native_plan_seconds = phase[1,5]
generate double native_stayer_seconds = phase[1,6]
generate double native_solve_seconds = phase[1,7]
generate double native_total_seconds = phase[1,8]
generate double selected_batch = `selected_batch'
generate double solver_iterations = `iterations'
generate double max_complete_residual = `max_residual'
generate double residual_acceptance = `acceptance'
generate double target_identity_residual = `identity'
generate double memory_forecast_bytes = `memory_forecast'
generate double resource_peak_bytes = `resource_peak'
generate double cmg_batch_strategy_mask = cmg[1,4]
generate double cmg_threads_requested = cmg[1,5]
generate double cmg_threads_used = cmg[1,6]
generate double cmg_maximum_concurrency = cmg[1,7]
generate double cmg_vertices = cmg[1,8]
generate double cmg_edges = cmg[1,9]
generate double cmg_hierarchy_levels = cmg[1,10]
generate double cmg_graph_bytes = cmg[1,12]
generate double cmg_hierarchy_bytes = cmg[1,13]
generate double cmg_plan_bytes = cmg[1,14]
generate double cmg_workspace_pool_bytes = cmg[1,16]
generate double cmg_admitted_peak_bytes = cmg[1,17]
generate double cmg_fit_tolerance = cmg[1,18]
generate double cmg_probe_tolerance = cmg[1,19]
generate double cmg_fit_inner_tolerance = cmg[1,20]
generate double cmg_probe_inner_tolerance = cmg[1,21]
generate double cmg_refinement_attempts = cmg[1,22]
generate double cmg_refined_columns = cmg[1,23]
generate double cmg_batch_calls = cmg[1,24]
generate double cmg_rhs_count = cmg[1,25]
generate double cmg_total_iterations = cmg[1,29]
generate double cmg_operator_applications = cmg[1,30]
generate double cmg_preconditioner_applications = cmg[1,31]
generate double cmg_max_reduced_residual = cmg[1,32]
generate double cmg_max_complete_residual = cmg[1,33]
generate double cmg_graph_seconds = cmg[1,34]/1e9
generate double cmg_hierarchy_seconds = cmg[1,35]/1e9
generate double cmg_rhs_seconds = cmg[1,36]/1e9
generate double cmg_solve_seconds = cmg[1,37]/1e9
generate double cmg_extraction_seconds = cmg[1,38]/1e9
generate double cmg_preparation_peak_bytes = cmg[1,39]
generate double cmg_prepared_persistent_bytes = cmg[1,40]
generate double cmg_non_cmg_peak_bytes = cmg[1,41]
generate double cmg_pre_rng_forecast_bytes = cmg[1,42]
generate double cmg_actual_retained_bytes = cmg[1,43]
generate double cmg_allocator_allowance_bytes = cmg[1,44]
generate double cmg_maximum_batch_rhs = cmg[1,45]
generate double cmg_workspace_count = cmg[1,46]
forvalues column = 1/4 {
    generate double plugin`column' = result[1,`column']
    generate double correction`column' = result[2,`column']
    generate double corrected`column' = result[3,`column']
    generate double mcse`column' = result[4,`column']
}
generate long sample_count = `sample_count'
generate byte data_restored = `data_restored'
generate byte rng_restored = `rng_restored'
generate byte sort_rng_restored = `sort_rng_restored'
export delimited using `"`output_csv'"', replace
di as result "VCKSS_FULL_CMG_PRODUCTION_CZ18_STATA_PASS `role' `source_commit'"
exit 0
