version 18.0
clear all
set more off
set varabbrev off

args experiment_id input_dta input_sha bundle_sha source_commit fixture ///
    scale_arg probes_arg seed_arg batch_arg memory_arg ///
    requested_slots_arg actual_slots_arg processors_arg frequency_var ///
    target_var deletion_var output_dir hard_wall_arg

// Preserve the literal source-bound option tuple.  Replication fixtures may
// replace deletion_var below with a temporary variable used only internally.
local submitted_frequency_var "`frequency_var'"
local submitted_target_var "`target_var'"
local submitted_deletion_var "`deletion_var'"
local deletion_mode "match"
local option_contract "KSS-SCALE-OPTIONS-V1"

local scale_factor = real("`scale_arg'")
local probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")
local declared_memory = real("`memory_arg'")
local requested_slots = real("`requested_slots_arg'")
local actual_slots = real("`actual_slots_arg'")
local requested_processors = real("`processors_arg'")
local hard_wall_seconds = real("`hard_wall_arg'")

if !ustrregexm("`experiment_id'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`input_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`bundle_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !inlist("`fixture'", "local", "cz24", "cz25", "cz18", ///
        "well_connected", "ring") | ///
    !inlist(`scale_factor', 1, 2, 4, 8, 16) | ///
    (!inlist("`fixture'", "well_connected", "ring") & ///
        `scale_factor' != 1) | ///
    ("`fixture'" == "ring" & `scale_factor' != 2) | ///
    missing(`probes') | `probes' < 2 | `probes' != floor(`probes') | ///
    missing(`benchmark_seed') | `benchmark_seed' < 0 | ///
    `benchmark_seed' > 2147483646 | ///
    !("`batch_arg'" == "auto" | ustrregexm("`batch_arg'", "^[0-9]+$")) | ///
    missing(`declared_memory') | `declared_memory' < 1 | ///
    `declared_memory' > 56 | ///
    missing(`requested_slots') | missing(`actual_slots') | ///
    `requested_slots' != floor(`requested_slots') | ///
    `actual_slots' != floor(`actual_slots') | ///
    `requested_slots' < 4 | `actual_slots' != `requested_slots' | ///
    `requested_processors' != 4 | ///
    missing(`hard_wall_seconds') | `hard_wall_seconds' < 300 | ///
    `hard_wall_seconds' > 43200 {
    di as error "invalid KSS-SCALE SCC driver arguments"
    exit 198
}

capture mkdir `"`output_dir'"'
confirm file `"`input_dta'"'
capture set processors `requested_processors'
local actual_processors = c(processors)
if c(MP) != 1 | `actual_processors' != `requested_processors' {
    di as error "Stata/MP did not honor the four-processor request"
    exit 459
}

local load_started = clock(c(current_date)+" "+c(current_time), "DMY hms")
quietly use `"`input_dta'"', clear
local load_finished = clock(c(current_date)+" "+c(current_time), "DMY hms")
local load_seconds = (`load_finished'-`load_started')/1000

confirm numeric variable worker firm y_minus_xb observation_key
isid observation_key
quietly count
local base_input_rows = r(N)
local input_rows = r(N)
adopath ++ "`c(pwd)'/kss_bc"

local weight_spec
if "`frequency_var'" != "-" {
    confirm numeric variable `frequency_var'
    local weight_spec "[fw=`frequency_var']"
}
if "`target_var'" != "-" confirm numeric variable `target_var'
if "`deletion_var'" != "-" confirm variable `deletion_var'

local fixture_base_rows = .
local fixture_base_physical = .
local fixture_base_workers = .
local fixture_base_firms = .
local fixture_base_cells = .
local fixture_base_units = .
local fixture_connector_rows = .
local fixture_expected_rows = .
local fixture_expected_physical = .
local fixture_expected_workers = .
local fixture_expected_firms = .
local fixture_expected_cells = .
local fixture_expected_units = .
local fixture_conductance = .
local fixture_lambda2 = .
local fixture_lambda_max = .
local fixture_condition_proxy = .
local fixture_min_degree = .
local fixture_max_degree = .
local fixture_construction_seconds = 0

if inlist("`fixture'", "well_connected", "ring") & `scale_factor' >= 2 {
    local fixture_timer = 99
    quietly timer clear `fixture_timer'
    quietly timer on `fixture_timer'
    quietly do "`c(pwd)'/kss_bc/benchmarks/kss_scale_fixtures.mata"
    quietly do "`c(pwd)'/kss_bc/benchmarks/kss_scale_fixtures.do"
    tempvar fixture_deletion fixture_copy fixture_connector
    if "`deletion_var'" == "-" {
        quietly egen double `fixture_deletion' = group(worker firm)
        local deletion_var "`fixture_deletion'"
    }
    local fixture_options "design(`fixture') copies(`scale_factor')"
    local fixture_options "`fixture_options' worker(worker) firm(firm)"
    local fixture_options "`fixture_options' deletionid(`deletion_var')"
    local fixture_options "`fixture_options' outcome(y_minus_xb)"
    local fixture_options "`fixture_options' copyvar(`fixture_copy')"
    local fixture_options "`fixture_options' connectorvar(`fixture_connector')"
    local fixture_options "`fixture_options' rowkey(observation_key)"
    if "`frequency_var'" != "-" ///
        local fixture_options "`fixture_options' frequency(`frequency_var')"
    if "`target_var'" != "-" ///
        local fixture_options "`fixture_options' target(`target_var')"
    quietly kssbc_scale_fixture, `fixture_options'
    local fixture_base_rows = r(base_rows)
    local fixture_base_physical = r(base_physical)
    local fixture_base_workers = r(base_workers)
    local fixture_base_firms = r(base_firms)
    local fixture_base_cells = r(base_cells)
    local fixture_base_units = r(base_deletion_units)
    local fixture_connector_rows = r(connector_rows)
    local fixture_expected_rows = r(expected_rows)
    local fixture_expected_physical = r(expected_physical)
    local fixture_expected_workers = r(expected_workers)
    local fixture_expected_firms = r(expected_firms)
    local fixture_expected_cells = r(expected_cells)
    local fixture_expected_units = r(expected_deletion_units)
    local fixture_conductance = r(copy_cut_conductance)
    local fixture_lambda2 = r(normalized_lambda2)
    local fixture_lambda_max = r(normalized_lambda_max)
    local fixture_condition_proxy = r(normalized_condition_proxy)
    local fixture_min_degree = r(minimum_weighted_degree)
    local fixture_max_degree = r(maximum_weighted_degree)
    quietly count
    local input_rows = r(N)
    isid observation_key

    // The fixture helper and production compressed engine intentionally have
    // separate implementations of several kssbc_scale__* diagnostics.  The
    // constructed Stata data and copied scalar receipts survive a Mata clear;
    // remove the fixture runtime now so kss_bc can load its registered
    // production runtime without a namespace collision.
    capture program drop kssbc_scale_fixture
    mata: mata clear
    quietly timer off `fixture_timer'
    quietly timer list `fixture_timer'
    local fixture_construction_seconds = r(t`fixture_timer')
    quietly timer clear `fixture_timer'
}

local command_options "worker(worker) firm(firm) deletion(match)"
local command_options "`command_options' algorithm(jla) engine(auto)"
local command_options "`command_options' probes(`probes') seed(`benchmark_seed')"
local command_options "`command_options' batch(`batch_arg') memory_gib(`declared_memory')"
local command_options "`command_options' preconditioner(auto) tolerance(1e-10)"
local command_options "`command_options' maxiter(20000) wallseconds(`hard_wall_seconds')"
local command_options "`command_options' probeorder(observation_key) nodisplay"
if "`target_var'" != "-" ///
    local command_options "`command_options' targetweight(`target_var')"
if "`deletion_var'" != "-" ///
    local command_options "`command_options' deletionid(`deletion_var')"

local command_started = clock(c(current_date)+" "+c(current_time), "DMY hms")
capture noisily kss_bc y_minus_xb `weight_spec', `command_options'
local command_rc = _rc
local command_finished = clock(c(current_date)+" "+c(current_time), "DMY hms")
local command_seconds = (`command_finished'-`command_started')/1000

tempname route_evidence pilot_evidence
local route_evidence_available = 0
local pilot_evidence_available = 0
capture matrix `route_evidence' = e(route_diagnostics)
if !_rc local route_evidence_available = 1
capture matrix `pilot_evidence' = e(route_pilot_diagnostics)
if !_rc local pilot_evidence_available = 1

local estimator_status `"`e(status)'"'
if `"`estimator_status'"' == "" local estimator_status "COMMAND_FAILURE"
local engine_requested "auto"
local engine_selected `"`e(engine_selected)'"'
if `"`engine_selected'"' == "" local engine_selected `"`e(engine)'"'
if `"`engine_selected'"' == "" local engine_selected "generic"
local fallback_status `"`e(fallback_status)'"'
if `"`fallback_status'"' == "" local fallback_status "NOT_REPORTED"
local fallback_message `"`e(fallback_message)'"'
local fastpath_status `"`e(fastpath_status)'"'
if `"`fastpath_status'"' == "" local fastpath_status "NOT_REPORTED"
local preconditioner_selected `"`e(preconditioner_selected)'"'
local routing_reason `"`e(routing_reason)'"'
local route_pilot_status `"`e(route_pilot_status)'"'
local route_pilot_failure_reason `"`e(route_pilot_failure_reason)'"'
local lifecycle_method `"`e(life_method)'"'
local resource_peak_phase `"`e(resource_peak_phase)'"'
local resource_status `"`e(resource_status)'"'
local rng_contract `"`e(rng_contract)'"'
local rng_implementation `"`e(rng_implementation)'"'
local rng_runtime `"`e(rng_runtime)'"'
local rng_leverage_domain `"`e(rng_leverage_domain)'"'
local rng_target_domain `"`e(rng_target_domain)'"'
local residual_normalization `"`e(residual_normalization)'"'
local quotient_convention `"`e(quotient_convention)'"'
local grounding_convention `"`e(grounding_convention)'"'
local estimator_status_ok = (                                      ///
    `"`engine_selected'"' == "compressed" &                       ///
    `"`estimator_status'"' ==                                     ///
        "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES") |               ///
    (`"`engine_selected'"' == "generic" &                         ///
    `"`estimator_status'"' == "KSS_POINT_ESTIMATES_ONLY")

foreach scalar_name in N_stored N_retained N_physical worker_levels ///
    firm_levels deletion_units coefficient_cells target_strata batch ///
    solver_iterations solver_max_residual residual_acceptance_tolerance ///
    max_leverage ///
    fit_seconds leverage_seconds target_seconds rng_seconds ///
    correction_seconds ///
    setup_seconds schur_seconds preconditioner_apply_seconds pcg_seconds ///
    solver_schur_actions solver_precond_applications ///
    route_hierarchy_levels route_hybrid_vertices route_hybrid_edges ///
    route_terminal_vertices route_planned_rhs ///
    route_diagonal_max_iterations route_cmg_max_iterations ///
    route_projected_work_ratio route_forecast_peak_bytes ///
    memory_forecast_bytes ///
    sample_selection_seconds compression_seconds validation_seconds ///
    life_transition_seconds life_work_seconds ///
    life_restore_seconds life_sample_restored ///
    life_mem_before_bytes life_mem_cleared_bytes ///
    life_mem_work_bytes life_mem_restored_bytes ///
    resource_select_peak_bytes resource_transition_peak_bytes ///
    resource_numerical_peak_bytes resource_restore_peak_bytes ///
    resource_peak_bytes resource_mem_headroom ///
    resource_mem_admit_bytes resource_wall_upper_seconds ///
    resource_wall_headroom resource_wall_admit_seconds ///
    resource_hard_mem_bytes resource_hard_wall_seconds ///
    resource_raw_stata_bytes resource_cell_bytes ///
    resource_deletion_unit_bytes resource_target_stratum_bytes ///
    resource_cmg_hierarchy_bytes resource_phase_scratch_bytes ///
    resource_sort_compress_bytes resource_solve_ahead_bytes ///
    resource_output_cert_bytes resource_preserve_bytes ///
    resource_runtime_resident_bytes {
    local `scalar_name' = .
    capture local `scalar_name' = e(`scalar_name')
}
// Normalize the installed command's public e() spellings into the stable SCC
// receipt names used by the validator.
capture local resource_mem_headroom = e(resource_memory_headroom)
capture local resource_sort_compress_bytes = e(resource_sort_temp_bytes)
capture local resource_output_cert_bytes = e(resource_output_bytes)
foreach rng_scalar in rng_master_seed rng_leverage_probe_first ///
    rng_leverage_probe_last rng_target_probe_first ///
    rng_target_probe_last {
    local `rng_scalar' = .
    capture local `rng_scalar' = e(`rng_scalar')
}
if missing(`resource_hard_mem_bytes') ///
    local resource_hard_mem_bytes = `declared_memory'*1024^3
if missing(`resource_hard_wall_seconds') ///
    local resource_hard_wall_seconds = `hard_wall_seconds'

if `route_evidence_available' {
    preserve
    clear
    local route_rows = rowsof(`route_evidence')
    quietly set obs `route_rows'
    svmat double `route_evidence', names(col)
    generate str64 experiment_id = "`experiment_id'"
    generate long route_row = _n
    order experiment_id route_row
    export delimited using `"`output_dir'/route_diagnostics.csv"', replace
    restore
}

if `pilot_evidence_available' {
    preserve
    clear
    local pilot_rows = rowsof(`pilot_evidence')
    quietly set obs `pilot_rows'
    svmat double `pilot_evidence', names(col)
    generate str64 experiment_id = "`experiment_id'"
    generate long pilot_row = _n
    order experiment_id pilot_row
    export delimited using ///
        `"`output_dir'/route_pilot_diagnostics.csv"', replace
    restore
}

local plugin_worker = .
local plugin_firm = .
local plugin_covariance = .
local plugin_total = .
local correction_worker = .
local correction_firm = .
local correction_covariance = .
local correction_total = .
local corrected_worker = .
local corrected_firm = .
local corrected_covariance = .
local corrected_total = .
local sample_semantics_valid = 0
local diagnostic_coefficient_cells = .
local diagnostic_deletion_units = .
local diagnostic_seconds = .
local rhs_count = .
local rhs_max_residual = .

if `command_rc' == 0 & `estimator_status_ok' {
    tempname estimates rhs_evidence
    matrix `estimates' = e(results)
    local plugin_worker = `estimates'[1,1]
    local plugin_firm = `estimates'[1,2]
    local plugin_covariance = `estimates'[1,3]
    local plugin_total = `estimates'[1,4]
    local correction_worker = `estimates'[2,1]
    local correction_firm = `estimates'[2,2]
    local correction_covariance = `estimates'[2,3]
    local correction_total = `estimates'[2,4]
    local corrected_worker = `estimates'[3,1]
    local corrected_firm = `estimates'[3,2]
    local corrected_covariance = `estimates'[3,3]
    local corrected_total = `estimates'[3,4]

    matrix `rhs_evidence' = e(solver_rhs_diagnostics)
    local rhs_count = rowsof(`rhs_evidence')
    mata: st_local("rhs_max_residual", ///
        strofreal(max(st_matrix("`rhs_evidence'")[,5]), "%21.17g"))
    preserve
    clear
    quietly set obs `rhs_count'
    svmat double `rhs_evidence', names(col)
    generate str64 experiment_id = "`experiment_id'"
    order experiment_id stage batch_start rhs iterations ///
        relative_residual converged
    export delimited using `"`output_dir'/rhs.csv"', replace
    restore

    tempvar retained_sample
    generate byte `retained_sample' = e(sample)
    quietly count if `retained_sample'
    local sample_semantics_valid = ///
        (r(N) == `N_retained' & r(N) > 0)
    local diagnostic_started = clock(c(current_date)+" "+c(current_time), ///
        "DMY hms")
    preserve
    quietly keep if `retained_sample'
    quietly keep worker firm
    quietly duplicates drop
    quietly count
    local diagnostic_coefficient_cells = r(N)
    restore
    preserve
    quietly keep if `retained_sample'
    if "`deletion_var'" == "-" {
        quietly keep worker firm
    }
    else {
        quietly keep `deletion_var'
    }
    quietly duplicates drop
    quietly count
    local diagnostic_deletion_units = r(N)
    restore
    local diagnostic_finished = clock(c(current_date)+" "+c(current_time), ///
        "DMY hms")
    local diagnostic_seconds = ///
        (`diagnostic_finished'-`diagnostic_started')/1000
    drop `retained_sample'
}

if missing(`coefficient_cells') ///
    local coefficient_cells = `diagnostic_coefficient_cells'
if missing(`target_strata') & "`engine_selected'" != "compressed" ///
    local target_strata = 0

local import_selection_seconds = `load_seconds'
local import_selection_seconds = ///
    `import_selection_seconds'+`fixture_construction_seconds'
if !missing(`sample_selection_seconds') ///
    local import_selection_seconds = ///
        `import_selection_seconds'+`sample_selection_seconds'
local compression_transition_seconds = 0
if !missing(`compression_seconds') ///
    local compression_transition_seconds = ///
        `compression_transition_seconds'+`compression_seconds'
if !missing(`life_transition_seconds') ///
    local compression_transition_seconds = ///
        `compression_transition_seconds'+`life_transition_seconds'
local numerical_computation_seconds = `command_seconds'
if !missing(`life_work_seconds') ///
    local numerical_computation_seconds = `life_work_seconds'
local restoration_seconds = 0
if !missing(`life_restore_seconds') ///
    local restoration_seconds = `life_restore_seconds'

preserve
clear
quietly set obs 4
generate str32 stage = ""
replace stage = "import_selection" in 1
replace stage = "compression_transition" in 2
replace stage = "numerical_computation" in 3
replace stage = "restoration" in 4
generate double elapsed_seconds = .
replace elapsed_seconds = `import_selection_seconds' in 1
replace elapsed_seconds = `compression_transition_seconds' in 2
replace elapsed_seconds = `numerical_computation_seconds' in 3
replace elapsed_seconds = `restoration_seconds' in 4
generate double forecast_peak_bytes = .
replace forecast_peak_bytes = `resource_select_peak_bytes' in 1
replace forecast_peak_bytes = `resource_transition_peak_bytes' in 2
replace forecast_peak_bytes = `resource_numerical_peak_bytes' in 3
replace forecast_peak_bytes = `resource_restore_peak_bytes' in 4
generate double observed_allocation_bytes = .
replace observed_allocation_bytes = ///
    `life_mem_before_bytes' in 1
replace observed_allocation_bytes = ///
    `life_mem_cleared_bytes' in 2
replace observed_allocation_bytes = ///
    `life_mem_work_bytes' in 3
replace observed_allocation_bytes = ///
    `life_mem_restored_bytes' in 4
// These are allocated-memory readings taken at lifecycle boundaries.  They
// are useful lower bounds, but they are not within-phase RSS peaks.
generate str40 measurement_kind = "stata_allocated_endpoint_not_peak"
order stage elapsed_seconds forecast_peak_bytes observed_allocation_bytes ///
    measurement_kind
export delimited using `"`output_dir'/stage_memory.csv"', replace
restore

preserve
clear
quietly set obs 1
generate str64 experiment_id = "`experiment_id'"
generate str24 fixture = "`fixture'"
generate double scale_factor = `scale_factor'
generate str40 source_commit = "`source_commit'"
generate str64 bundle_sha256 = "`bundle_sha'"
generate str64 input_sha256 = "`input_sha'"
generate str32 option_contract = "`option_contract'"
generate str32 frequency_var = "`submitted_frequency_var'"
generate str32 target_var = "`submitted_target_var'"
generate str32 deletion_var = "`submitted_deletion_var'"
generate str16 deletion_mode = "`deletion_mode'"
generate double requested_slots = `requested_slots'
generate double actual_slots = `actual_slots'
generate double mem_per_core_gib = `declared_memory'/`requested_slots'
generate double requested_stata_processors = `requested_processors'
generate double actual_stata_processors = `actual_processors'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
generate byte stata_mp = c(MP)
generate double base_input_rows = `base_input_rows'
generate double input_rows = `input_rows'
generate double requested_probes = `probes'
generate double seed = `benchmark_seed'
generate double tolerance = 1e-10
generate double declared_memory_gib = `declared_memory'
generate str16 batch_requested = "`batch_arg'"
generate double command_rc = `command_rc'
generate double load_seconds = `load_seconds'
generate double fixture_construction_seconds = ///
    `fixture_construction_seconds'
generate double import_selection_seconds = `import_selection_seconds'
generate double command_seconds = `command_seconds'
generate double diagnostic_seconds = `diagnostic_seconds'
generate str40 estimator_status = "`estimator_status'"
generate str16 engine_requested = "`engine_requested'"
generate str16 engine_selected = "`engine_selected'"
generate str40 fallback_status = "`fallback_status'"
generate strL fallback_message = `"`fallback_message'"'
generate str40 fastpath_status = "`fastpath_status'"
generate str16 preconditioner_selected = "`preconditioner_selected'"
generate strL routing_reason = `"`routing_reason'"'
generate strL route_pilot_status = `"`route_pilot_status'"'
generate strL route_pilot_failure_reason = ///
    `"`route_pilot_failure_reason'"'
generate byte route_evidence_available = `route_evidence_available'
generate byte pilot_evidence_available = `pilot_evidence_available'
generate str32 resource_status = "`resource_status'"
generate str64 rng_contract = "`rng_contract'"
generate str32 rng_implementation = "`rng_implementation'"
generate str12 rng_runtime = "`rng_runtime'"
generate double rng_master_seed = `rng_master_seed'
generate str16 rng_leverage_domain = "`rng_leverage_domain'"
generate str16 rng_target_domain = "`rng_target_domain'"
generate double rng_leverage_probe_first = `rng_leverage_probe_first'
generate double rng_leverage_probe_last = `rng_leverage_probe_last'
generate double rng_target_probe_first = `rng_target_probe_first'
generate double rng_target_probe_last = `rng_target_probe_last'
generate double N_stored = `N_stored'
generate double N_retained = `N_retained'
generate double N_physical = `N_physical'
generate double worker_levels = `worker_levels'
generate double firm_levels = `firm_levels'
generate double deletion_units = `deletion_units'
generate double coefficient_cells = `coefficient_cells'
generate double target_strata = `target_strata'
generate double diagnostic_coefficient_cells = ///
    `diagnostic_coefficient_cells'
generate double diagnostic_deletion_units = `diagnostic_deletion_units'
generate double fixture_base_rows = `fixture_base_rows'
generate double fixture_base_physical = `fixture_base_physical'
generate double fixture_base_workers = `fixture_base_workers'
generate double fixture_base_firms = `fixture_base_firms'
generate double fixture_base_cells = `fixture_base_cells'
generate double fixture_base_units = `fixture_base_units'
generate double fixture_connector_rows = `fixture_connector_rows'
generate double fixture_expected_rows = `fixture_expected_rows'
generate double fixture_expected_physical = `fixture_expected_physical'
generate double fixture_expected_workers = `fixture_expected_workers'
generate double fixture_expected_firms = `fixture_expected_firms'
generate double fixture_expected_cells = `fixture_expected_cells'
generate double fixture_expected_units = `fixture_expected_units'
generate double fixture_conductance = `fixture_conductance'
generate double fixture_lambda2 = `fixture_lambda2'
generate double fixture_lambda_max = `fixture_lambda_max'
generate double fixture_condition_proxy = `fixture_condition_proxy'
generate double fixture_min_degree = `fixture_min_degree'
generate double fixture_max_degree = `fixture_max_degree'
generate byte sample_semantics_valid = `sample_semantics_valid'
generate double selected_batch = `batch'
generate double solver_iterations = `solver_iterations'
generate double solver_max_residual = `solver_max_residual'
generate double residual_acceptance_tolerance = ///
    `residual_acceptance_tolerance'
generate str40 residual_normalization = "`residual_normalization'"
generate str32 residual_equation_contract = "original_rhs_worker_firm_v1"
generate str40 quotient_convention = "`quotient_convention'"
generate str80 grounding_convention = "`grounding_convention'"
generate str32 grounded_coordinate_handling = "included_in_full_residual"
generate double rhs_count = `rhs_count'
generate double rhs_max_residual = `rhs_max_residual'
generate double max_leverage = `max_leverage'
generate double fit_seconds = `fit_seconds'
generate double leverage_seconds = `leverage_seconds'
generate double target_seconds = `target_seconds'
generate double rng_seconds = `rng_seconds'
generate double correction_seconds = `correction_seconds'
generate str48 timing_attribution_contract = ///
    "rng_and_correction_nested_nonadditive"
generate double setup_seconds = `setup_seconds'
generate double schur_seconds = `schur_seconds'
generate double preconditioner_apply_seconds = ///
    `preconditioner_apply_seconds'
generate double pcg_seconds = `pcg_seconds'
generate double solver_schur_actions = `solver_schur_actions'
generate double solver_precond_applications = ///
    `solver_precond_applications'
generate double route_hierarchy_levels = `route_hierarchy_levels'
generate double route_hybrid_vertices = `route_hybrid_vertices'
generate double route_hybrid_edges = `route_hybrid_edges'
generate double route_terminal_vertices = `route_terminal_vertices'
generate double route_planned_rhs = `route_planned_rhs'
generate double route_diagonal_max_iterations = ///
    `route_diagonal_max_iterations'
generate double route_cmg_max_iterations = `route_cmg_max_iterations'
generate double route_projected_work_ratio = `route_projected_work_ratio'
generate double route_forecast_peak_bytes = `route_forecast_peak_bytes'
generate double memory_forecast_bytes = `memory_forecast_bytes'
generate double plugin_worker = `plugin_worker'
generate double plugin_firm = `plugin_firm'
generate double plugin_covariance = `plugin_covariance'
generate double plugin_total = `plugin_total'
generate double correction_worker = `correction_worker'
generate double correction_firm = `correction_firm'
generate double correction_covariance = `correction_covariance'
generate double correction_total = `correction_total'
generate double corrected_worker = `corrected_worker'
generate double corrected_firm = `corrected_firm'
generate double corrected_covariance = `corrected_covariance'
generate double corrected_total = `corrected_total'
generate str16 lifecycle_method = "`lifecycle_method'"
generate double life_transition_seconds = `life_transition_seconds'
generate double life_work_seconds = `life_work_seconds'
generate double life_restore_seconds = `life_restore_seconds'
generate double life_sample_restored = `life_sample_restored'
generate double resource_selection_peak_bytes = ///
    `resource_select_peak_bytes'
generate double resource_transition_peak_bytes = ///
    `resource_transition_peak_bytes'
generate double resource_numerical_peak_bytes = ///
    `resource_numerical_peak_bytes'
generate double resource_restoration_peak_bytes = ///
    `resource_restore_peak_bytes'
generate double resource_peak_bytes = `resource_peak_bytes'
generate str32 resource_peak_phase = "`resource_peak_phase'"
generate double resource_mem_headroom = `resource_mem_headroom'
generate double resource_mem_admit_bytes = `resource_mem_admit_bytes'
generate double resource_wall_upper_seconds = ///
    `resource_wall_upper_seconds'
generate double resource_wall_headroom = `resource_wall_headroom'
generate double resource_wall_admit_seconds = ///
    `resource_wall_admit_seconds'
generate double resource_hard_mem_bytes = `resource_hard_mem_bytes'
generate double resource_hard_wall_seconds = `resource_hard_wall_seconds'
generate double resource_raw_stata_bytes = `resource_raw_stata_bytes'
generate double resource_cell_bytes = `resource_cell_bytes'
generate double resource_deletion_unit_bytes = ///
    `resource_deletion_unit_bytes'
generate double resource_target_stratum_bytes = ///
    `resource_target_stratum_bytes'
generate double resource_cmg_hierarchy_bytes = ///
    `resource_cmg_hierarchy_bytes'
generate double resource_phase_scratch_bytes = ///
    `resource_phase_scratch_bytes'
generate double resource_sort_compress_bytes = ///
    `resource_sort_compress_bytes'
generate double resource_solve_ahead_bytes = `resource_solve_ahead_bytes'
generate double resource_output_cert_bytes = `resource_output_cert_bytes'
generate double resource_preserve_bytes = `resource_preserve_bytes'
generate double resource_runtime_resident_bytes = ///
    `resource_runtime_resident_bytes'
export delimited using `"`output_dir'/summary.csv"', replace
restore

tempname marker
if `command_rc' != 0 | !`estimator_status_ok' |                 ///
    !`sample_semantics_valid' |                                  ///
    `rhs_count' != 3*`probes'+1 {
    file open `marker' using `"`output_dir'/stata.fail"', ///
        write text replace
    file write `marker' "KSS_SCALE_ESTIMATOR_FAILURE `experiment_id' " ///
        "`bundle_sha' `source_commit' `input_sha'" _n
    file close `marker'
    di as error "KSS-SCALE estimator failure: `experiment_id'"
    if `command_rc' != 0 exit `command_rc'
    exit 459
}

file open `marker' using `"`output_dir'/stata.pass"', write text replace
file write `marker' "KSS_SCALE_ESTIMATOR_PASS `experiment_id' " ///
    "`bundle_sha' `source_commit' `input_sha'" _n
file close `marker'
di as result "KSS-SCALE ESTIMATOR PASS: `experiment_id'"
exit 0
