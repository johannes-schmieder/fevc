version 18.0
clear all
set more off
set varabbrev off
set linesize 255

args package_root input_csv output_csv phase_start phase_end role source_commit task_sha input_sha structure connectivity rows_arg degree_arg probes_arg seed_arg cores_arg memory_arg timeout_arg
local rows = real("`rows_arg'")
local degree = real("`degree_arg'")
local probes = real("`probes_arg'")
local seed = real("`seed_arg'")
local cores = real("`cores_arg'")
local memory = real("`memory_arg'")
local timeout = real("`timeout_arg'")
if !inlist("`role'","mata","rust") |                         ///
   !ustrregexm("`source_commit'","^[0-9a-f]{40}$") |         ///
   !ustrregexm("`task_sha'","^[0-9a-f]{64}$") |              ///
   !ustrregexm("`input_sha'","^[0-9a-f]{64}$") |             ///
   !inlist(`rows',7680,30720,122880,491520,1966080) |          ///
   !inlist(`degree',2,3,6) | !inlist(`cores',1,2,4,8,16) |     ///
   `probes'!=200 | missing(`seed') | `seed'<1 |                ///
   `memory'<=0 | missing(`memory') | `timeout'!=10800 {
    di as error "invalid comparative-scaling Stata arguments"
    exit 198
}
if ("`structure'"=="strong_d2" & ("`connectivity'"!="strong" | `degree'!=2)) | ///
   ("`structure'"=="strong_d3" & ("`connectivity'"!="strong" | `degree'!=3)) | ///
   ("`structure'"=="strong_d6" & ("`connectivity'"!="strong" | `degree'!=6)) | ///
   ("`structure'"=="weak_d3"   & ("`connectivity'"!="weak"   | `degree'!=3)) {
    di as error "graph structure contract changed"
    exit 198
}
confirm file `"`package_root'/vckss/vckss.ado"'
if "`role'"=="rust" {
    confirm file `"`package_root'/vckss/vckss_rust_linux_x64.plugin"'
}
confirm file `"`input_csv'"'
adopath ++ `"`package_root'/vckss"'
capture set processors `cores'
if _rc | c(processors)!=`cores' {
    di as error "Stata/MP did not honor the active-core contract"
    exit 459
}

local workers = `rows'/`degree'
local firms = `workers'/40
assert `workers'==floor(`workers') & `firms'==floor(`firms')
timer clear 70
timer on 70
import delimited using `"`input_csv'"', clear varnames(1) asdouble bindquote(strict)
timer off 70
timer list 70
local import_seconds = r(t70)
confirm numeric variable observation_key worker firm period match y
assert _N==`rows'
isid observation_key
isid worker firm
quietly count if missing(observation_key,worker,firm,period,match,y)
assert r(N)==0
sort worker period firm

quietly _datasignature
local data_signature `"`r(datasignature)'"'
set rng default
set seed 20260827
set sortseed 20260827
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'

tempname phasefile
file open `phasefile' using `"`phase_start'"', write text replace
file write `phasefile' "START `role'" _n
file close `phasefile'
timer clear 80
timer on 80
if "`role'"=="rust" {
    capture noisily vckss y, worker(worker) firm(firm) deletion(match) ///
        probeorder(observation_key) backend(rust) rng(counter_v1)     ///
        algorithm(jla) engine(auto) preconditioner(auto) batch(auto) ///
        memory_gib(`memory') wallseconds(`timeout') probes(`probes') ///
        seed(`seed') maxiter(20000) nodisplay
}
else {
    capture noisily vckss y, worker(worker) firm(firm) deletion(match) ///
        probeorder(observation_key) backend(mata) rng(stata)            ///
        algorithm(jla) engine(auto) preconditioner(auto) batch(auto)    ///
        memory_gib(`memory') wallseconds(`timeout') probes(`probes')    ///
        seed(`seed') maxiter(20000) nodisplay
}
local command_rc = _rc
timer off 80
tempname phaseendfile
file open `phaseendfile' using `"`phase_end'"', write text replace
file write `phaseendfile' "END `role' rc=`command_rc'" _n
file close `phaseendfile'
if `command_rc' exit `command_rc'
timer list 80
local command_seconds = r(t80)

assert "`e(backend_requested)'"=="`role'"
assert "`e(backend_selected)'"=="`role'"
assert "`e(algorithm)'"=="jla"
if "`role'"=="rust" {
    assert "`e(rng_selected)'"=="counter_v1"
    assert "`e(cmg_backend)'"=="CMG_FULL_V2"
    assert "`e(cmg_source_commit)'"=="98768722fa21800d2cb91cd2182406c9db3bf979"
    assert e(cmg_threads_requested)==`cores' & e(cmg_threads_used)==`cores'
}
else {
    assert "`e(rng_selected)'"=="stata"
}
assert e(N_stored)==`rows' & e(N_retained)==`rows'
assert e(worker_levels)==`workers' & e(firm_levels)==`firms'
assert e(coefficient_cells)==`rows' & e(deletion_units)==`rows'
assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
assert e(target_identity_residual)<=1e-12
matrix result = e(results)
assert rowsof(result)==4 & colsof(result)==4
tempvar in_sample
generate byte `in_sample'=e(sample)
quietly count if `in_sample'
local sample_count = r(N)
assert `sample_count'==`rows'
drop `in_sample'
quietly _datasignature
local data_restored = (`"`r(datasignature)'"'==`"`data_signature'"')
local rng_restored = (`"`c(rngstate)'"'==`"`rng_before'"')
local sort_rng_restored = (`"`c(sortrngstate)'"'==`"`sort_rng_before'"')
assert `data_restored' & `rng_restored' & `sort_rng_restored'

local status `e(status)'
local engine `e(engine_selected)'
local route `e(preconditioner_selected)'
local selection = e(sample_selection_seconds)
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
local iterations = e(solver_iterations)
local max_residual = e(complete_residual_max)
local acceptance = e(residual_acceptance_tolerance)
local identity = e(target_identity_residual)
local peak_bytes = e(resource_peak_bytes)
local forecast_bytes = e(memory_forecast_bytes)

local cmg_backend "NONE"
local cmg_source "NONE"
local cmg_requested = .
local cmg_used = .
local cmg_retained = .
local cmg_admitted = .
local cmg_fit_tol = .
local cmg_probe_tol = .
local cmg_iterations = .
local cmg_applications = .
local cmg_preconditioner_applications = .
local cmg_max_reduced = .
local cmg_max_complete = .
local cmg_graph_seconds = .
local cmg_hierarchy_seconds = .
local cmg_rhs_seconds = .
local cmg_solve_seconds = .
local cmg_extraction_seconds = .
if "`role'"=="rust" {
    matrix cmg = e(full_cmg_receipt)
    assert rowsof(cmg)==1 & colsof(cmg)==46
    local cmg_backend `e(cmg_backend)'
    local cmg_source `e(cmg_source_commit)'
    local cmg_requested = cmg[1,5]
    local cmg_used = cmg[1,6]
    local cmg_admitted = cmg[1,17]
    local cmg_fit_tol = cmg[1,18]
    local cmg_probe_tol = cmg[1,19]
    local cmg_iterations = cmg[1,29]
    local cmg_applications = cmg[1,30]
    local cmg_preconditioner_applications = cmg[1,31]
    local cmg_max_reduced = cmg[1,32]
    local cmg_max_complete = cmg[1,33]
    local cmg_graph_seconds = cmg[1,34]/1e9
    local cmg_hierarchy_seconds = cmg[1,35]/1e9
    local cmg_rhs_seconds = cmg[1,36]/1e9
    local cmg_solve_seconds = cmg[1,37]/1e9
    local cmg_extraction_seconds = cmg[1,38]/1e9
    local cmg_retained = cmg[1,43]
}

clear
set obs 1
generate str48 schema = "VCKSS-COMPARATIVE-SCALING-STATA-V1"
generate str8 role = "`role'"
generate str8 application_status = "PASS"
generate str40 source_commit = "`source_commit'"
generate str64 task_sha256 = "`task_sha'"
generate str64 input_sha256 = "`input_sha'"
generate str16 structure = "`structure'"
generate str8 connectivity = "`connectivity'"
generate long rows = `rows'
generate long workers = `workers'
generate long firms = `firms'
generate byte cells_per_worker = `degree'
generate int probes = `probes'
generate long seed = `seed'
generate byte active_cores = c(processors)
generate str48 estimator_status = "`status'"
generate str16 engine = "`engine'"
generate str16 preconditioner = "`route'"
generate str16 cmg_backend = "`cmg_backend'"
generate str40 cmg_source_commit = "`cmg_source'"
generate double import_seconds = `import_seconds'
generate double command_seconds = `command_seconds'
generate double selection_seconds = `selection'
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
generate double solver_iterations = `iterations'
generate double max_complete_residual = `max_residual'
generate double residual_acceptance = `acceptance'
generate double target_identity_residual = `identity'
generate double resource_peak_bytes = `peak_bytes'
generate double memory_forecast_bytes = `forecast_bytes'
generate double cmg_threads_requested = `cmg_requested'
generate double cmg_threads_used = `cmg_used'
generate double cmg_admitted_peak_bytes = `cmg_admitted'
generate double cmg_actual_retained_bytes = `cmg_retained'
generate double cmg_fit_tolerance = `cmg_fit_tol'
generate double cmg_probe_tolerance = `cmg_probe_tol'
generate double cmg_total_iterations = `cmg_iterations'
generate double cmg_operator_applications = `cmg_applications'
generate double cmg_preconditioner_applications = `cmg_preconditioner_applications'
generate double cmg_max_reduced_residual = `cmg_max_reduced'
generate double cmg_max_complete_residual = `cmg_max_complete'
generate double cmg_graph_seconds = `cmg_graph_seconds'
generate double cmg_hierarchy_seconds = `cmg_hierarchy_seconds'
generate double cmg_rhs_seconds = `cmg_rhs_seconds'
generate double cmg_solve_seconds = `cmg_solve_seconds'
generate double cmg_extraction_seconds = `cmg_extraction_seconds'
generate long sample_count = `sample_count'
generate byte data_restored = `data_restored'
generate byte rng_restored = `rng_restored'
generate byte sort_rng_restored = `sort_rng_restored'
local names worker firm covariance total
forvalues column = 1/4 {
    local name : word `column' of `names'
    generate double plugin_`name' = result[1,`column']
    generate double correction_`name' = result[2,`column']
    generate double corrected_`name' = result[3,`column']
    generate double mcse_`name' = result[4,`column']
}
export delimited using `"`output_csv'"', replace
di as result "VCKSS_COMPARATIVE_SCALING_STATA_PASS `role' `source_commit'"
exit 0
