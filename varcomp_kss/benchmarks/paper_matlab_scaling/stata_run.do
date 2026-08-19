version 18.0
clear all
set more off
set varabbrev off

args source_root input_csv output_csv source_commit task_sha input_sha structure connectivity rows_arg degree_arg probes_arg seed_arg
local rows = real("`rows_arg'")
local degree = real("`degree_arg'")
local probes = real("`probes_arg'")
local seed = real("`seed_arg'")
if !ustrregexm("`source_commit'","^[0-9a-f]{40}$") | ///
   !ustrregexm("`task_sha'","^[0-9a-f]{64}$") |       ///
   !ustrregexm("`input_sha'","^[0-9a-f]{64}$") |     ///
   !inlist(`rows',7680,30720,122880,491520,1966080) |  ///
   !inlist(`degree',2,3,6) | `probes'!=200 | missing(`seed') | `seed'<1 {
    di as error "invalid paper MATLAB-scaling Stata arguments"
    exit 198
}
if ("`structure'"=="strong_d2" & ("`connectivity'"!="strong" | `degree'!=2)) | ///
   ("`structure'"=="strong_d3" & ("`connectivity'"!="strong" | `degree'!=3)) | ///
   ("`structure'"=="strong_d6" & ("`connectivity'"!="strong" | `degree'!=6)) | ///
   ("`structure'"=="weak_d3"   & ("`connectivity'"!="weak"   | `degree'!=3)) {
    di as error "graph structure contract changed"
    exit 198
}
confirm file `"`source_root'/varcomp_kss/varcomp_kss.ado"'
confirm file `"`input_csv'"'
adopath ++ `"`source_root'/varcomp_kss"'
capture set processors 4
if _rc | c(processors)!=4 {
    di as error "Stata/MP did not honor four processors"
    exit 459
}

local workers = `rows'/`degree'
local firms = `workers'/40
assert `workers'==floor(`workers') & `firms'==floor(`firms')
local import_started = clock(c(current_date)+" "+c(current_time),"DMY hms")
import delimited using `"`input_csv'"', clear varnames(1) asdouble bindquote(strict)
local import_finished = clock(c(current_date)+" "+c(current_time),"DMY hms")
local import_seconds = (`import_finished'-`import_started')/1000
confirm numeric variable observation_key worker firm period match y
assert _N==`rows'
isid observation_key
isid worker firm
assert worker==floor(worker) & firm==floor(firm) & match==floor(match)
quietly count if missing(observation_key,worker,firm,period,match,y)
assert r(N)==0
sort worker period firm

quietly _datasignature
local data_signature `"`r(datasignature)'"'
set rng default
set seed 20260819
set sortseed 20260819
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'
timer clear 80
timer on 80
quietly varcomp_kss y, worker(worker) firm(firm) deletion(match)       ///
    probeorder(observation_key) algorithm(jla) engine(compressed)     ///
    preconditioner(auto) memory_gib(48) wallseconds(28800)            ///
    probes(`probes') batch(auto) seed(`seed') tolerance(1e-10)        ///
    maxiter(20000) nodisplay
timer off 80
timer list 80
local command_seconds = r(t80)

assert "`e(algorithm)'"=="jla"
assert "`e(engine_selected)'"=="compressed"
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
assert r(N)==`rows'
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

clear
set obs 1
generate str40 schema = "PAPER-MATLAB-SCALING-STATA-V1"
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
generate byte processors = c(processors)
generate str32 sample_contract = "same_literal_match_rows_v1"
generate str32 target_contract = "uniform_stored_rows_v1"
generate str48 estimator_status = "`status'"
generate str16 engine = "`engine'"
generate str16 preconditioner = "`route'"
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
generate double plugin_worker = result[1,1]
generate double correction_worker = result[2,1]
generate double corrected_worker = result[3,1]
generate double plugin_firm = result[1,2]
generate double correction_firm = result[2,2]
generate double corrected_firm = result[3,2]
generate double plugin_covariance = result[1,3]
generate double correction_covariance = result[2,3]
generate double corrected_covariance = result[3,3]
generate double plugin_total = result[1,4]
generate double correction_total = result[2,4]
generate double corrected_total = result[3,4]
generate byte data_restored = `data_restored'
generate byte rng_restored = `rng_restored'
generate byte sort_rng_restored = `sort_rng_restored'
export delimited using `"`output_csv'"', replace
di as result "PAPER_MATLAB_SCALING_STATA_PASS `source_commit'"
exit 0
