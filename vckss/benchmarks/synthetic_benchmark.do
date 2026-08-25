version 18.0
clear all
set more off
set varabbrev off

args run_id scenario workers_arg firms_arg probes_arg seed_arg output_dir source_commit

local n_workers = real("`workers_arg'")
local n_firms = real("`firms_arg'")
local n_probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")

if strtrim("`run_id'") == "" | strtrim("`scenario'") == "" | ///
    strtrim("`output_dir'") == "" | strlen("`source_commit'") < 7 {
    di as error "benchmark arguments are incomplete"
    exit 198
}
if !ustrregexm("`run_id'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`scenario'", "^[A-Za-z0-9._-]+$") {
    di as error "run_id and scenario must be safe path labels"
    exit 198
}
if missing(`n_workers') | missing(`n_firms') | missing(`n_probes') | ///
    missing(`benchmark_seed') | `n_workers' != floor(`n_workers') | ///
    `n_firms' != floor(`n_firms') | `n_probes' != floor(`n_probes') | ///
    `benchmark_seed' != floor(`benchmark_seed') | `n_firms' < 3 | ///
    `n_workers' < 2*`n_firms' | `n_probes' < 2 {
    di as error "invalid benchmark dimensions, probes, or seed"
    exit 198
}

capture confirm file "vckss/vckss.ado"
if _rc {
    di as error "run synthetic_benchmark.do from the staged source root"
    exit 601
}
adopath ++ "`c(pwd)'/vckss"

timer clear 80
timer clear 81
timer on 80

local n_rows = 4*`n_workers'
set obs `n_rows'
generate long worker = floor((_n-1)/4) + 1
generate byte period = mod(_n-1,4) + 1
generate long home_firm = mod(worker-1,`n_firms') + 1
generate long firm = cond(period <= 2, home_firm, mod(home_firm,`n_firms')+1)
generate long actual_match = 2*(worker-1) + (period > 2) + 1
generate byte frequency = 1 + mod(actual_match,2)
generate double target_mass = .5 + mod(_n-1,17)/17
generate double c1 = cond(mod(period,2)==1,-.5,.5)
generate double c2 = cond(period==1,-.8, ///
    cond(period==2,.3,cond(period==3,.6,-.2)))

set seed `benchmark_seed'
generate double epsilon = rnormal()
generate double y = sin(worker/97) + cos(firm/31) + ///
    .35*c1 - .18*c2 + epsilon

timer off 80
quietly timer list 80
scalar data_prep_seconds = r(t80)

timer on 81
capture noisily vckss y c1 c2 [fw=frequency], ///
    worker(worker) firm(firm) deletion(match) deletionid(actual_match) ///
    algorithm(jla) nuisance(joint) targetweight(target_mass) ///
    probes(`n_probes') batch(8) seed(`benchmark_seed') ///
    tolerance(1e-8) maxiter(20000) exact_limit(50) nodisplay
local command_rc = _rc
timer off 81
quietly timer list 81
scalar command_seconds = r(t81)

if `command_rc' {
    tempname failure
    file open `failure' using "`output_dir'/`scenario'.failure.txt", ///
        write text replace
    file write `failure' "VCKSS_BENCHMARK_FAIL rc=`command_rc'" _n
    file close `failure'
    exit `command_rc'
}
if "`e(status)'" != "KSS_POINT_ESTIMATES_ONLY" {
    di as error "vckss did not return its success status"
    exit 498
}

tempname result_matrix mcse_matrix solver_rhs_matrix
matrix `result_matrix' = e(results)
matrix `mcse_matrix' = e(numerical_mcse)
matrix `solver_rhs_matrix' = e(solver_rhs_diagnostics)
local returned_algorithm "`e(algorithm)'"
local returned_status "`e(status)'"
local returned_backend_requested "`e(backend_requested)'"
local returned_backend_selected "`e(backend_selected)'"
local returned_rng_requested "`e(rng_requested)'"
local returned_rng_selected "`e(rng_selected)'"
local returned_engine_requested "`e(engine_requested)'"
local returned_engine_selected "`e(engine_selected)'"
local ret_preconditioner_requested "`e(preconditioner_requested)'"
local ret_preconditioner_selected "`e(preconditioner_selected)'"
local returned_result_family "`e(result_family)'"
local stata_version "`c(stata_version)'"
local stata_flavor "`c(flavor)'"

preserve
clear
set obs 1
generate str64 run_id = "`run_id'"
generate str32 scenario = "`scenario'"
generate str40 source_commit = "`source_commit'"
generate str32 status = "`returned_status'"
generate str8 algorithm = "`returned_algorithm'"
generate str12 backend_requested = "`returned_backend_requested'"
generate str12 backend_selected = "`returned_backend_selected'"
generate str16 rng_requested = "`returned_rng_requested'"
generate str16 rng_selected = "`returned_rng_selected'"
generate str12 engine_requested = "`returned_engine_requested'"
generate str12 engine_selected = "`returned_engine_selected'"
generate str16 preconditioner_requested = "`ret_preconditioner_requested'"
generate str16 preconditioner_selected = "`ret_preconditioner_selected'"
generate str12 result_family = "`returned_result_family'"
generate str12 stata_version = "`stata_version'"
generate str12 stata_flavor = "`stata_flavor'"
generate double requested_workers = `n_workers'
generate double requested_firms = `n_firms'
generate double requested_probes = `n_probes'
generate double seed = `benchmark_seed'
generate double data_prep_seconds = scalar(data_prep_seconds)
generate double command_seconds = scalar(command_seconds)
generate double total_seconds = scalar(data_prep_seconds) + scalar(command_seconds)

foreach scalar_name in N_stored N_physical N_retained worker_levels ///
    firm_levels parameters full_parameters correction_parameters ///
    deletion_units target_weight_sum max_leverage ///
    weighted_rss information_rcond inverse_relres preconditioner_ratio ///
    control_schur_rcond deletion_rank_gap graph_seconds fit_seconds ///
    setup_seconds preconditioner_seconds ///
    leverage_seconds target_seconds correction_seconds solver_iterations ///
    solver_max_residual schur_seconds preconditioner_apply_seconds ///
    pcg_seconds solver_backend_seconds solver_schur_actions ///
    solver_schur_batches solver_precond_applications ///
    solver_precond_batches complete_residual_max ///
    target_identity_residual memory_forecast_bytes route_code batch ///
    rust_control_schur_rcond rust_control_schur_relres ///
    rust_deletion_rank_gap rust_maker_relres ///
    rust_actual_accounting_residual rust_plan_solve_peak_bytes ///
    rust_plan_nonbatched_peak_bytes rust_generic_peak_bytes ///
    rust_counter_plan_complete rust_pre_rng_hi rust_pre_rng_lo probes {
    generate double `scalar_name' = e(`scalar_name')
}

local target_names worker firm covariance total
forvalues target_index = 1/4 {
    local target_name : word `target_index' of `target_names'
    generate double plugin_`target_name' = `result_matrix'[1,`target_index']
    generate double correction_`target_name' = `result_matrix'[2,`target_index']
    generate double corrected_`target_name' = `result_matrix'[3,`target_index']
    generate double mcse_`target_name' = `mcse_matrix'[1,`target_index']
}
export delimited using "`output_dir'/`scenario'.csv", replace
restore

preserve
clear
svmat double `solver_rhs_matrix', names(col)
generate str64 run_id = "`run_id'"
generate str32 scenario = "`scenario'"
generate str40 source_commit = "`source_commit'"
generate str12 stata_version = "`stata_version'"
generate str12 stata_flavor = "`stata_flavor'"
order run_id scenario source_commit stata_version stata_flavor stage ///
    batch_start rhs iterations relative_residual converged
export delimited using "`output_dir'/`scenario'.rhs.csv", replace
restore

tempname marker
file open `marker' using "`output_dir'/`scenario'.stata.pass", ///
    write text replace
file write `marker' "VCKSS_BENCHMARK_PASS `scenario' `source_commit'" _n
file close `marker'

di as result "VCKSS BENCHMARK PASS: `scenario'"
exit 0
