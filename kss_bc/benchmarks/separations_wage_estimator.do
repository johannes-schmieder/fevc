version 18.0
clear all
set more off
set varabbrev off

args label route prepared_dta prepared_sha probes_arg seed_arg ///
    memory_gib_arg projected_seconds_arg projection_basis output_dir ///
    source_commit wage_input_sha

local probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")
local memory_gib = real("`memory_gib_arg'")
local projected_seconds = real("`projected_seconds_arg'")
if !ustrregexm("`label'", "^[A-Za-z0-9._-]+$") | ///
    !inlist("`route'", "exact", "b1", "cmg") | ///
    !ustrregexm("`prepared_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`wage_input_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !ustrregexm("`projection_basis'", "^[A-Za-z0-9._-]+$") | ///
    missing(`probes') | `probes' != floor(`probes') | `probes' < 2 | ///
    missing(`benchmark_seed') | `benchmark_seed' != floor(`benchmark_seed') | ///
    missing(`memory_gib') | `memory_gib' < 1 | `memory_gib' > 56 | ///
    missing(`projected_seconds') | `projected_seconds' <= 0 | ///
    `projected_seconds' > 5400 {
    di as error "invalid Separations wage estimator arguments"
    exit 198
}
confirm file `"`prepared_dta'"'
adopath ++ "`c(pwd)'/kss_bc"
quietly use `"`prepared_dta'"', clear
confirm numeric variable worker firm period y_minus_xb observation_key
isid observation_key
quietly count
local input_rows = r(N)

if "`route'" == "cmg" {
    quietly do "kss_bc/kss_bc.mata"
    quietly do "shared/cmg/generated/kssbc_cmg_core.mata"
    quietly do "kss_bc/tests/support/kss_cmg_adapter.mata"
    mata: mata drop kssbc__stata_jla()
    quietly do "kss_bc/tests/support/kss_cmg_bridge_override.mata"
    global KSSBC_CMG_MEMORY_GIB `memory_gib'
}

timer clear 81
timer on 81
if "`route'" == "exact" {
    capture noisily kss_bc y_minus_xb, worker(worker) firm(firm) ///
        deletion(match) algorithm(exact) nodisplay
}
else {
    capture noisily kss_bc y_minus_xb, worker(worker) firm(firm) ///
        deletion(match) algorithm(jla) probes(`probes') batch(8) ///
        probeorder(observation_key) seed(`benchmark_seed') ///
        tolerance(1e-10) maxiter(20000) nodisplay
}
local command_rc = _rc
timer off 81
quietly timer list 81
scalar command_seconds = r(t81)

local route_status "NOT_APPLICABLE"
local route_message ""
if "`route'" == "cmg" {
    local route_status "$KSSBC_CMG_STATUS"
    local route_message "$KSSBC_CMG_MESSAGE"
}
local estimator_status "FAILED"
if `command_rc' == 0 local estimator_status "`e(status)'"
local converged = (`command_rc' == 0 & ///
    "`estimator_status'" == "KSS_POINT_ESTIMATES_ONLY")

tempname result_matrix mcse_matrix solver_rhs_matrix route_diagnostics
matrix `route_diagnostics' = J(1,10,.)
if "`route'" == "cmg" {
    capture matrix `route_diagnostics' = KSSBC_CMG_ROUTE_DIAGNOSTICS
}
if `converged' {
    matrix `result_matrix' = e(results)
    if "`route'" == "exact" matrix `mcse_matrix' = J(1,4,.)
    else {
        matrix `mcse_matrix' = e(numerical_mcse)
        matrix `solver_rhs_matrix' = e(solver_rhs_diagnostics)
    }
    generate byte __kss_sample = e(sample)
    preserve
    keep if __kss_sample
    keep worker firm
    duplicates drop
    sort worker firm
    save `"`output_dir'/retained_matches.dta"', replace
    restore
}

preserve
clear
set obs 1
generate str64 label = "`label'"
generate str8 route = "`route'"
generate str40 source_commit = "`source_commit'"
generate str64 prepared_sha256 = "`prepared_sha'"
generate str64 wage_input_sha256 = "`wage_input_sha'"
generate str40 estimator_status = "`estimator_status'"
generate str40 route_status = "`route_status'"
generate str160 route_message = `"`route_message'"'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
generate str64 projection_basis = "`projection_basis'"
generate double projected_seconds = `projected_seconds'
generate double input_rows = `input_rows'
generate double requested_probes = `probes'
generate double seed = `benchmark_seed'
generate double tolerance = 1e-10
generate double memory_gib = `memory_gib'
generate str32 probe_order = "observation_key"
generate byte converged = `converged'
generate double command_rc = `command_rc'
generate double command_seconds = scalar(command_seconds)

foreach scalar_name in N_stored N_physical N_retained worker_levels ///
    firm_levels parameters full_parameters correction_parameters ///
    deletion_units target_weight_sum max_leverage weighted_rss ///
    inverse_relres deletion_rank_gap graph_seconds fit_seconds ///
    setup_seconds leverage_seconds target_seconds correction_seconds ///
    solver_iterations solver_max_residual schur_seconds ///
    preconditioner_apply_seconds pcg_seconds solver_backend_seconds ///
    solver_schur_actions solver_schur_batches ///
    solver_precond_applications solver_precond_batches probes {
    generate double `scalar_name' = .
    if `converged' replace `scalar_name' = e(`scalar_name')
}
generate double planned_rhs = `route_diagnostics'[1,1]
generate double hierarchy_setup_seconds = `route_diagnostics'[1,3]
generate double hierarchy_levels = `route_diagnostics'[1,4]
generate double structural_bytes = `route_diagnostics'[1,7]
generate double dense_factor_bytes = `route_diagnostics'[1,8]

local target_names worker firm covariance total
foreach prefix in plugin correction corrected mcse {
    foreach target_name of local target_names {
        generate double `prefix'_`target_name' = .
    }
}
if `converged' {
    forvalues target_index = 1/4 {
        local target_name : word `target_index' of `target_names'
        replace plugin_`target_name' = `result_matrix'[1,`target_index']
        replace correction_`target_name' = `result_matrix'[2,`target_index']
        replace corrected_`target_name' = `result_matrix'[3,`target_index']
        replace mcse_`target_name' = `mcse_matrix'[1,`target_index']
    }
}
export delimited using ///
    `"`output_dir'/separations_`label'_`route'.csv"', replace
restore

if `converged' & "`route'" != "exact" {
    preserve
    clear
    svmat double `solver_rhs_matrix', names(col)
    generate str64 label = "`label'"
    generate str8 route = "`route'"
    generate str40 source_commit = "`source_commit'"
    generate str12 stata_version = string(c(stata_version))
    generate str12 stata_flavor = c(flavor)
    order label route source_commit stata_version stata_flavor stage ///
        batch_start rhs iterations relative_residual converged
    export delimited using ///
        `"`output_dir'/separations_`label'_`route'_rhs.csv"', replace
    restore
}

tempname marker
file open `marker' using ///
    `"`output_dir'/separations_`label'_`route'.stata.pass"', ///
    write text replace
if `converged' {
    file write `marker' ///
        "KSS_BC_SEPARATIONS_CONVERGED `route' `label' `source_commit'" _n
}
else {
    file write `marker' ///
        "KSS_BC_SEPARATIONS_TYPED_FAILURE `route' `label' `route_status' `source_commit'" _n
}
file close `marker'

if "`route'" == "cmg" {
    macro drop KSSBC_CMG_MEMORY_GIB KSSBC_CMG_STATUS KSSBC_CMG_MESSAGE
}
if !`converged' & inlist("`route'", "exact", "b1") {
    di as error "`route' failed on the Separations wage benchmark"
    exit cond(`command_rc' == 0,498,`command_rc')
}
if !`converged' & "`route'" == "cmg" & ///
    ("`route_status'" == "" | "`route_status'" == "INVALID_INPUT") {
    di as error "forced CMG failed without an admissible typed status"
    exit cond(`command_rc' == 0,498,`command_rc')
}
di as result "KSS_BC SEPARATIONS ESTIMATOR PASS: `route' `label'"
exit 0
