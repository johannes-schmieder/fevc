version 18.0
clear all
set more off
set varabbrev off

args run_id route scenario workers_arg firms_arg probes_arg seed_arg ///
    memory_gib_arg projected_seconds_arg projection_basis output_dir ///
    source_commit

local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")
local memory_gib = real("`memory_gib_arg'")
local projected_seconds = real("`projected_seconds_arg'")

if !inlist("`route'", "b1", "cmg") | ///
    !inlist("`scenario'", "easy", "moderate", "weak") | ///
    !ustrregexm("`run_id'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`projection_basis'", "^[A-Za-z0-9._-]+$") | ///
    missing(`workers') | missing(`firms') | missing(`probes') | ///
    missing(`benchmark_seed') | missing(`memory_gib') | ///
    missing(`projected_seconds') | `workers' != floor(`workers') | ///
    `firms' != floor(`firms') | `probes' != floor(`probes') | ///
    `benchmark_seed' != floor(`benchmark_seed') | `workers' < 2*`firms' | ///
    `firms' < 8 | `probes' < 2 | `memory_gib' < 1 | ///
    `memory_gib' > 56 | `projected_seconds' <= 0 | ///
    `projected_seconds' > 5400 | strlen("`source_commit'") != 40 {
    di as error "invalid end-to-end KSS/CMG benchmark arguments"
    exit 198
}

capture confirm file "vckss/vckss.ado"
if _rc {
    di as error "run estimator_cmg_benchmark.do from the staged source root"
    exit 601
}
adopath ++ "`c(pwd)'/vckss"

timer clear 80
timer clear 81
timer on 80
local degree = cond("`scenario'" == "easy", 4, ///
    cond("`scenario'" == "moderate", 3, 2))
set seed `benchmark_seed'
set obs `=`degree'*`workers''
generate long worker = floor((_n-1)/`degree') + 1
generate byte link = mod(_n-1,`degree')
generate long firm = .
if "`scenario'" == "weak" {
    replace firm = mod(worker-1+link,`firms') + 1
}
else if "`scenario'" == "moderate" {
    replace firm = mod(worker-1+cond(link==2,17,link),`firms') + 1
}
else {
    replace firm = mod(worker-1,`firms') + 1 if link == 0
    replace firm = runiformint(1,`firms') if link > 0
}
generate double outcome = sin(worker/37) + cos(firm/19) + link/101
timer off 80
quietly timer list 80
scalar data_prep_seconds = r(t80)

if "`route'" == "cmg" {
    quietly do "vckss/vckss.mata"
    quietly do "vckss/vckss_cmg.mata"
    quietly do "vckss/tests/support/vckss_cmg_adapter.mata"
    mata: mata drop vckss__stata_jla()
    quietly do "vckss/tests/support/vckss_cmg_bridge_override.mata"
    global VCKSS_CMG_MEMORY_GIB `memory_gib'
}

timer on 81
// KSS-NUMOPT compares the Mata diagonal solver with the test-only Mata CMG
// bridge installed above. Rust-vs-Mata performance uses the alpha benchmark
// matrix and must not bypass this historical component qualification.
capture noisily vckss outcome, worker(worker) firm(firm) ///
    deletion(match) algorithm(jla) probes(`probes') batch(8) ///
    seed(`benchmark_seed') tolerance(1e-10) maxiter(20000) ///
    engine(generic) backend(mata) rng(stata) nodisplay
local command_rc = _rc
timer off 81
quietly timer list 81
scalar command_seconds = r(t81)

local route_status "NOT_APPLICABLE"
local route_message ""
if "`route'" == "cmg" {
    local route_status "$VCKSS_CMG_STATUS"
    local route_message "$VCKSS_CMG_MESSAGE"
}
local estimator_status "FAILED"
if `command_rc' == 0 local estimator_status "`e(status)'"
local converged = (`command_rc' == 0 & ///
    "`estimator_status'" == "KSS_POINT_ESTIMATES_ONLY")

tempname result_matrix mcse_matrix solver_rhs_matrix route_diagnostics
matrix `route_diagnostics' = J(1,12,.)
if "`route'" == "cmg" {
    capture matrix `route_diagnostics' = VCKSS_CMG_ROUTE_DIAGNOSTICS
}
if `converged' {
    matrix `result_matrix' = e(results)
    matrix `mcse_matrix' = e(numerical_mcse)
    matrix `solver_rhs_matrix' = e(solver_rhs_diagnostics)
}

preserve
clear
set obs 1
generate str64 run_id = "`run_id'"
generate str8 route = "`route'"
generate str16 scenario = "`scenario'"
generate str40 source_commit = "`source_commit'"
generate str40 estimator_status = "`estimator_status'"
generate str40 route_status = "`route_status'"
generate str160 route_message = `"`route_message'"'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
generate str64 projection_basis = "`projection_basis'"
generate double projected_seconds = `projected_seconds'
generate double requested_workers = `workers'
generate double requested_firms = `firms'
generate double requested_probes = `probes'
generate double seed = `benchmark_seed'
generate double tolerance = 1e-10
generate double memory_gib = `memory_gib'
generate byte converged = `converged'
generate double command_rc = `command_rc'
generate double data_prep_seconds = scalar(data_prep_seconds)
generate double command_seconds = scalar(command_seconds)
generate double total_seconds = scalar(data_prep_seconds)+scalar(command_seconds)

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
generate double edge_complexity = `route_diagnostics'[1,5]
generate double vertex_complexity = `route_diagnostics'[1,6]
generate double structural_bytes = `route_diagnostics'[1,7]
generate double dense_factor_bytes = `route_diagnostics'[1,8]
generate double hybrid_vertices = `route_diagnostics'[1,11]
generate double hybrid_edges = `route_diagnostics'[1,12]

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
    "`output_dir'/numopt_`scenario'_`route'.csv", replace
restore

if `converged' {
    preserve
    clear
    svmat double `solver_rhs_matrix', names(col)
    generate str64 run_id = "`run_id'"
    generate str8 route = "`route'"
    generate str16 scenario = "`scenario'"
    generate str40 source_commit = "`source_commit'"
    generate str12 stata_version = string(c(stata_version))
    generate str12 stata_flavor = c(flavor)
    order run_id route scenario source_commit stata_version stata_flavor ///
        stage batch_start rhs iterations relative_residual converged
    export delimited using ///
        "`output_dir'/numopt_`scenario'_`route'_rhs.csv", replace
    restore
}

tempname marker
file open `marker' using ///
    "`output_dir'/numopt_`scenario'_`route'.stata.pass", ///
    write text replace
if `converged' {
    file write `marker' ///
        "VCKSS_NUMOPT_CONVERGED `route' `scenario' `source_commit'" _n
}
else {
    file write `marker' ///
        "VCKSS_NUMOPT_TYPED_FAILURE `route' `scenario' `route_status' `source_commit'" _n
}
file close `marker'

if "`route'" == "cmg" {
    macro drop VCKSS_CMG_MEMORY_GIB VCKSS_CMG_STATUS VCKSS_CMG_MESSAGE
}
if !`converged' & !("`route'" == "cmg" & "`scenario'" == "easy" & ///
    inlist("`route_status'", "HIERARCHY_EDGE_LIMIT", ///
        "HIERARCHY_COMPLEXITY_LIMIT", "HIERARCHY_STALLED")) {
    di as error "unexpected end-to-end estimator failure"
    exit cond(`command_rc' == 0,498,`command_rc')
}
di as result "VCKSS NUMOPT BENCHMARK PASS: `route' `scenario'"
exit 0
