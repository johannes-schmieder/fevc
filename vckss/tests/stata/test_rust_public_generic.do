version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

quietly run `"`package_dir'/vckss.ado"'
vckss_rust clear
set obs 96
generate long cell = floor((_n-1)/2)
generate double worker = floor(cell/4)+1
generate double firm = mod(cell,4)+1
generate double replicate = mod(_n-1,2)
generate double deletion_id = cell+1
generate double outcome = .7*worker-.45*firm+.3*replicate+mod(7*(_n-1),5)/11
generate double frequency = mod(_n-1,3)+1
generate double target_weight = .5+mod(5*(_n-1),7)/3
generate double control = (worker-.4*firm)*(replicate+1)+mod(3*(_n-1),7)/17
sort cell replicate
set rng kiss32
set seed 20260822
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1)        ///
    targetweight(target_weight) nodisplay
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "generic"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(status)'"' == "KSS_POINT_ESTIMATES_ONLY"
assert `"`e(version)'"' == "0.4.0-dev"
assert strpos(`"`e(cmdline)'"',"algorithm(jla)") > 0
assert strpos(`"`e(cmdline)'"',"backend(rust)") > 0
assert e(rust_cap_schema) == 2
assert e(rust_cap_profile_code) == 3
assert e(rust_rng_contract_code) == 1
assert e(probes) == 7
assert e(leverage_batch) == 2 & e(target_batch) == 2
assert e(full_parameters) == e(worker_levels)+e(firm_levels)
assert e(parameters) == e(full_parameters)
assert e(correction_parameters) == e(full_parameters)
assert e(inverse_relres) >= . & e(information_rcond) >= .
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_working_fit_residual) <= e(residual_acceptance_tolerance)
assert inlist(e(rust_full_fit_zero_rhs),0,1)
assert e(rust_max_complete_residual) == e(complete_residual_max)
assert e(rust_maker_relres) == e(correction_reciprocal_residual)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
assert rowsof(e(rust_rhs_receipts)) == 1+1+3*7
assert colsof(e(rust_rhs_receipts)) == 15
assert e(rust_rhs_v2_copy_bytes) == rowsof(e(rust_rhs_receipts))*216
assert e(rust_rhs_receipt_schema) == 2
assert e(rust_control_rhs_count) == 1
assert e(rust_requested_engine_code) == 2
assert e(rust_selected_engine_code) == 2
assert e(rust_batch_mode_code) == 1
assert e(rust_stayers_mode_code) == 1
assert e(rust_target_weight_mode_code) == 1
assert e(rust_deletion_source_code) == 2
assert e(rust_probeorder_supplied) == 0
assert e(rust_wallseconds_supplied) == 0
assert e(rust_frequency_use_code) == 1
assert e(rust_solve_physical_limit) == e(physical_limit)
assert e(rust_result_cap_schema) == 2 & e(rust_result_cap_profile) == 3
assert e(rust_solve_signature_hi) == e(rust_cap_signature_hi)
assert e(rust_solve_signature_lo) == e(rust_cap_signature_lo)
assert colsof(e(rust_memory_receipt)) == 12
assert colsof(e(rust_request_capability_receipt)) == 23
assert colsof(e(rust_control_rank_receipt)) == 10
assert e(rust_support_flags) == 38
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(algorithm_option_supplied) == 1
assert e(engine_option_supplied) == 1
assert e(preconditioner_option_supplied) == 1
assert e(batch_option_supplied) == 1
assert e(stayers_option_supplied) == 0
assert e(targetweight_option_supplied) == 1
tempname initial_control_rank
matrix `initial_control_rank' = e(rust_control_rank_receipt)
assert `initial_control_rank'[1,1] ==                    ///
    `initial_control_rank'[1,2]/`initial_control_rank'[1,3]
assert `initial_control_rank'[1,1] > `initial_control_rank'[1,8]
assert `initial_control_rank'[1,4] >= 0
assert `initial_control_rank'[1,5] >= 0 & `initial_control_rank'[1,5] < .25
assert `initial_control_rank'[1,6] > 0
assert `initial_control_rank'[1,7] == e(rust_rhs_receipts)[1,7]
assert `initial_control_rank'[1,7] <= `initial_control_rank'[1,10]
assert `initial_control_rank'[1,8] == max(e(rust_rank_tolerance),1e-12)
assert `initial_control_rank'[1,9] == 1e-13
assert `initial_control_rank'[1,10] == 1e-11
foreach result_matrix in results plugin correction kss numerical_mcse decomposition {
    confirm matrix e(`result_matrix')
}
capture confirm matrix e(V)
assert _rc != 0
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

tempname public_reference repeat_reference private_reference
matrix `public_reference' = e(results)
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1)        ///
    targetweight(target_weight) nodisplay
matrix `repeat_reference' = e(results)
assert mreldif(`public_reference',`repeat_reference') == 0

// Match IDs only label deletion blocks: the implicit cell partition and a
// strictly increasing relabel preserve the same canonical block ordering.
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) nuisance(joint) algorithm(jla) backend(rust)        ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2)   ///
    probes(7) seed(81227) tolerance(1e-12) memory_gib(1)                ///
    targetweight(target_weight) nodisplay
assert mreldif(e(results),`public_reference') == 0
generate double deletion_relabel = 1000000+17*deletion_id
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_relabel) nuisance(joint)       ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic)       ///
    preconditioner(diagonal) batch(2) probes(7) seed(81227)            ///
    tolerance(1e-12) memory_gib(1) targetweight(target_weight) nodisplay
assert mreldif(e(results),`public_reference') == 0

// The private direct lifecycle is a same-atom Counter-V1 reference for the
// public boundary, not an independent estimator oracle.
preserve
quietly vckss_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1) controls(1) ///
    frequencyused(1) engine(generic) batchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid) physicallimit(50000000)
local private_schema = r(request_schema)
local private_profile = r(profile_code)
local private_signature_hi = r(request_signature_hi)
local private_signature_lo = r(request_signature_lo)
tempvar private_keep
quietly vckss_rust prepare worker firm deletion_id outcome frequency ///
    target_weight control, cleanup generate(`private_keep') memorygib(1) ///
    deletion(match)
local private_handle = r(handle)
quietly vckss_rust solve `private_handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) seed(81227) probes(7) leveragebatch(2) ///
    targetbatch(2) tolerance(1e-12) engine(generic) batchmode(explicit) ///
    stayers(movers) targetweightmode(explicit) deletionsource(matchid) ///
    physicallimit(50000000) capabilityschema(`private_schema')       ///
    capabilityprofile(`private_profile') frequencyused(1)           ///
    signaturehi(`private_signature_hi') signaturelo(`private_signature_lo')
quietly vckss_rust result `private_handle'
matrix `private_reference' = r(result)
assert mreldif(`public_reference',`private_reference') == 0
quietly vckss_rust release `private_handle'
restore
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

// Omitted and explicit automatic backends both select Rust after capability
// preflight, and omitted rng() resolves to Counter-V1.
foreach automatic_backend in omitted auto {
    local backend_option
    if "`automatic_backend'" == "auto" local backend_option backend(auto)
    quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
        deletion(observation) nuisance(joint) algorithm(jla) engine(generic) ///
        preconditioner(diagonal) batch(2) probes(4) seed(81227)       ///
        targetweight(target_weight) `backend_option' nodisplay
    assert `"`e(backend_requested)'"' == "auto"
    assert `"`e(backend_selected)'"' == "rust"
    assert e(backend_fallback) == 0
    assert `"`e(rng_requested)'"' == "auto"
    assert `"`e(rng_selected)'"' == "counter_v1"
    assert e(backend_option_supplied) == ("`automatic_backend'" == "auto")
}

// The out-of-box request uses MATLAB-like JLA and resolves engine, route, and
// batch from the same frozen native plan.
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target_weight) probes(4)       ///
    seed(81227) memory_gib(1) nodisplay
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert e(algorithm_option_supplied) == 0
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(batch_requested)'"' == "auto"
assert e(backend_fallback) == 0
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

// The qualified V4/V7 planner also accepts a fully explicit strict tuple.
tempname planned_reference planned_memory
local planned_rng `"`c(rng)'"'
local planned_stream = c(rngstream)
local planned_state `"`c(rngstate)'"'
local planned_sortedby : sortedby
quietly _datasignature
local planned_signature `"`r(datasignature)'"'
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(auto) ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) wallseconds(60) nodisplay
matrix `planned_reference' = e(results)
matrix `planned_memory' = e(rust_memory_receipt)
assert mreldif(`planned_reference',`public_reference') == 0
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
assert `"`e(rust_capability_profile)'"' == "PLANNED_V1"
assert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 2 & e(route_code) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert e(leverage_batch) >= 1 & e(leverage_batch) <= e(probes)
assert e(target_batch) >= 1 & e(target_batch) <= e(probes)
assert e(batch) == max(e(leverage_batch),e(target_batch))
tempname planned_rhs_public
matrix `planned_rhs_public' = e(solver_rhs_diagnostics)
forvalues row = 1/`=rowsof(`planned_rhs_public')' {
    local planned_stage = `planned_rhs_public'[`row',1]
    local planned_rhs = `planned_rhs_public'[`row',3]
    if `planned_stage' == 4 {
        local planned_probe = `planned_rhs'-1
        local planned_start = floor(`planned_probe'/e(leverage_batch))* ///
            e(leverage_batch)+1
        assert `planned_rhs_public'[`row',2] == `planned_start'
    }
    else if `planned_stage' == 5 {
        local planned_probe = floor((`planned_rhs'-1)/2)
        local planned_start = floor(`planned_probe'/e(target_batch))* ///
            e(target_batch)+1
        assert `planned_rhs_public'[`row',2] == `planned_start'
    }
    else assert `planned_rhs_public'[`row',2] == 1
}
assert e(rust_plan_schema) == 1
assert e(rust_plan_route_schema) == 2
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_wallseconds_forecast) >= 0
assert e(rust_wallseconds_advisory) >= 0
assert e(rust_wallseconds_margin) >= 0
assert e(rust_plan_solve_peak_bytes) == `planned_memory'[1,11]
assert `planned_memory'[1,12] == max(`planned_memory'[1,5],`planned_memory'[1,11])
assert `planned_memory'[1,12] <= `planned_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
forvalues row = 1/3 {
    assert abs(`planned_reference'[`row',4]-`planned_reference'[`row',1]- ///
        `planned_reference'[`row',2]-2*`planned_reference'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`planned_reference'[1,`column']-`planned_reference'[2,`column']- ///
        `planned_reference'[3,`column']) <= 1e-10
}
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`planned_rng'"'
assert c(rngstream) == `planned_stream'
assert `"`c(rngstate)'"' == `"`planned_state'"'
local planned_sortedby_after : sortedby
assert `"`planned_sortedby_after'"' == `"`planned_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`planned_signature'"'


// Exercise requested algorithm(auto) with the generic JLA result family
// directly before opening the public router.  exact_limit(2) forces JLA;
// controls make the registered engine(auto) eligibility irrelevant here.
local auto_algorithm_rng `"`c(rng)'"'
local auto_algorithm_stream = c(rngstream)
local auto_algorithm_state `"`c(rngstate)'"'
local auto_algorithm_sortedby : sortedby
quietly _datasignature
local auto_algorithm_signature `"`r(datasignature)'"'
quietly count
local auto_algorithm_nscope = r(N)
local auto_algorithm_ncomplete = r(N)
quietly vckss_rust probe
local auto_algorithm_core = r(core_ready_flags)
local auto_algorithm_support = r(support_flags)
tempvar auto_algorithm_touse
generate byte `auto_algorithm_touse' = 1
capture noisily _vckss_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `auto_algorithm_touse' `auto_algorithm_nscope' ///
    `auto_algorithm_ncomplete' 0 0 7 2 81227 1e-12 10000 1 auto generic  ///
    1 1 1 1 1 1 1 0 `auto_algorithm_core' `auto_algorithm_support'       ///
    "nodisplay" match joint 2 1e-10 1e-10 5000 50000000 control 1 1    ///
    "vckss outcome control [fw=frequency], backend(rust) algorithm(auto) engine(generic)" ///
    auto auto 1 60
assert _rc == 0
assert mreldif(e(results),`planned_reference') == 0
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "generic"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 2
assert e(rust_requested_engine_code) == 2
assert e(rust_selected_engine_code) == 2
assert e(rust_plan_struct_size) == 1000
assert e(rust_plan_schema) == 1
assert e(rust_plan_route_schema) == 2
assert e(rust_plan_resolved) == 1
assert e(rust_plan_frozen) == 1
assert e(rust_plan_applicability) == 3
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 2
assert e(rust_plan_engine_requested) == 2
assert e(rust_plan_engine_selected) == 2
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 2
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert e(rust_plan_rhs) == rowsof(e(rust_rhs_receipts))
assert e(rust_plan_full_dimension) == e(rust_solver_dimension)
assert e(rust_plan_leverage_batch) == e(leverage_batch)
assert e(rust_plan_target_batch) == e(target_batch)
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_cap_algorithm_deferred) == 1
assert e(rust_cap_engine_deferred) == 1
tempname auto_algorithm_capability
matrix `auto_algorithm_capability' = e(rust_request_capability_receipt)
assert `auto_algorithm_capability'[1,7] == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`auto_algorithm_rng'"'
assert c(rngstream) == `auto_algorithm_stream'
assert `"`c(rngstate)'"' == `"`auto_algorithm_state'"'
local auto_algorithm_sortedby_after : sortedby
assert `"`auto_algorithm_sortedby_after'"' == `"`auto_algorithm_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`auto_algorithm_signature'"'


// Forced diagonal enters V4/V7 only when automatic batching or wall planning
// is requested.  The explicit numeric-batch/no-wall tuple above remains V2.
tempname forced_diagonal_results forced_diagonal_memory
local forced_diagonal_rng `"`c(rng)'"'
local forced_diagonal_stream = c(rngstream)
local forced_diagonal_state `"`c(rngstate)'"'
local forced_diagonal_sortedby : sortedby
quietly _datasignature
local forced_diagonal_signature `"`r(datasignature)'"'
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) wallseconds(60) nodisplay
matrix `forced_diagonal_results' = e(results)
matrix `forced_diagonal_memory' = e(rust_memory_receipt)
assert mreldif(`forced_diagonal_results',`planned_reference') == 0
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
assert `"`e(rust_capability_profile)'"' == "PLANNED_V1"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2 & e(route_code) == 2
assert e(rust_full_fit_route) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert e(leverage_batch) >= 1 & e(leverage_batch) <= e(probes)
assert e(target_batch) >= 1 & e(target_batch) <= e(probes)
assert e(batch) == max(e(leverage_batch),e(target_batch))
assert e(rust_plan_schema) == 1 & e(rust_plan_route_schema) == 2
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_wallseconds_forecast) >= 0
assert e(rust_wallseconds_advisory) >= 0
assert e(rust_wallseconds_margin) >= 0
assert e(rust_plan_solve_peak_bytes) == `forced_diagonal_memory'[1,11]
assert `forced_diagonal_memory'[1,12] == max(                  ///
    `forced_diagonal_memory'[1,5],`forced_diagonal_memory'[1,11])
assert `forced_diagonal_memory'[1,12] <= `forced_diagonal_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`forced_diagonal_rng'"'
assert c(rngstream) == `forced_diagonal_stream'
assert `"`c(rngstate)'"' == `"`forced_diagonal_state'"'
local forced_diagonal_sortedby_after : sortedby
assert `"`forced_diagonal_sortedby_after'"' == `"`forced_diagonal_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`forced_diagonal_signature'"'


// Engine auto is public only where scientific eligibility guarantees generic.
tempname engine_auto_results engine_auto_memory engine_auto_capability
local engine_auto_rng `"`c(rng)'"'
local engine_auto_stream = c(rngstream)
local engine_auto_state `"`c(rngstate)'"'
local engine_auto_sortedby : sortedby
quietly _datasignature
local engine_auto_signature `"`r(datasignature)'"'
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) wallseconds(60) nodisplay
matrix `engine_auto_results' = e(results)
matrix `engine_auto_memory' = e(rust_memory_receipt)
matrix `engine_auto_capability' = e(rust_request_capability_receipt)
assert mreldif(`engine_auto_results',`forced_diagonal_results') == 0
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
assert `"`e(rust_capability_profile)'"' == "PLANNED_V1"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_cap_engine_deferred) == 1
assert `engine_auto_capability'[1,14] == 0
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 2
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2 & e(route_code) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert e(leverage_batch) >= 1 & e(leverage_batch) <= e(probes)
assert e(target_batch) >= 1 & e(target_batch) <= e(probes)
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_plan_solve_peak_bytes) == `engine_auto_memory'[1,11]
assert `engine_auto_memory'[1,12] == max(                       ///
    `engine_auto_memory'[1,5],`engine_auto_memory'[1,11])
assert `engine_auto_memory'[1,12] <= `engine_auto_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`engine_auto_rng'"'
assert c(rngstream) == `engine_auto_stream'
assert `"`c(rngstate)'"' == `"`engine_auto_state'"'
local engine_auto_sortedby_after : sortedby
assert `"`engine_auto_sortedby_after'"' == `"`engine_auto_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`engine_auto_signature'"'

// Forced CMG uses the same V4/V7 lifecycle but may never fall back.
tempname forced_cmg_results forced_cmg_memory
local forced_cmg_rng `"`c(rng)'"'
local forced_cmg_stream = c(rngstream)
local forced_cmg_state `"`c(rngstate)'"'
local forced_cmg_sortedby : sortedby
quietly _datasignature
local forced_cmg_signature `"`r(datasignature)'"'
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(cmg) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
matrix `forced_cmg_results' = e(results)
matrix `forced_cmg_memory' = e(rust_memory_receipt)
assert mreldif(`forced_cmg_results',`public_reference') <= 1e-9
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "cmg"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert `"`e(batch_requested)'"' == "2"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_route) == 3
assert e(rust_selected_route) == 3 & e(route_code) == 3
assert e(rust_full_fit_route) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 1
assert e(rust_leverage_batch_mode_code) == 1
assert e(rust_target_batch_mode_code) == 1
assert e(leverage_batch) == 2 & e(target_batch) == 2 & e(batch) == 2
assert e(rust_plan_schema) == 1 & e(rust_plan_route_schema) == 2
assert e(rust_wallseconds_supplied) == 0
assert e(rust_wallseconds_requested) == 0
assert e(rust_plan_solve_peak_bytes) == `forced_cmg_memory'[1,11]
assert `forced_cmg_memory'[1,12] == max(                       ///
    `forced_cmg_memory'[1,5],`forced_cmg_memory'[1,11])
assert `forced_cmg_memory'[1,12] <= `forced_cmg_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`forced_cmg_rng'"'
assert c(rngstream) == `forced_cmg_stream'
assert `"`c(rngstate)'"' == `"`forced_cmg_state'"'
local forced_cmg_sortedby_after : sortedby
assert `"`forced_cmg_sortedby_after'"' == `"`forced_cmg_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`forced_cmg_signature'"'

// Planned admission is based on the effective request; omitted batch() uses
// the same registered automatic policy as an explicitly supplied batch(auto).
quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) preconditioner(auto) probes(4) nodisplay
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(batch_requested)'"' == "auto"
assert e(batch_option_supplied) == 0
assert inlist(`"`e(preconditioner_selected)'"',"diagonal","cmg")
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

// Omitted algorithm(), engine(), and preconditioner() each resolve from their
// documented effective defaults; supplied flags are provenance, not admission.
quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) engine(generic) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert `"`e(algorithm)'"' == "jla"
assert e(algorithm_option_supplied) == 0
assert `"`e(backend_selected)'"' == "rust"

quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert `"`e(engine_requested)'"' == "auto"
assert e(engine_option_supplied) == 0
assert `"`e(engine_selected)'"' == "generic"

quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) batch(2) probes(4) nodisplay
assert `"`e(preconditioner_requested)'"' == "auto"
assert e(preconditioner_option_supplied) == 0
assert inlist(`"`e(preconditioner_selected)'"',"diagonal","cmg")
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) preconditioner(diagonal) probes(4) nodisplay
assert `"`e(batch_requested)'"' == "auto"
assert e(batch_option_supplied) == 0
assert `"`e(backend_selected)'"' == "rust"

quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) algorithm(jla) engine(generic) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(backend_selected)'"' == "rust"

quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) preconditioner(diagonal) batch(2) probes(4) ///
    stayers(movers) nodisplay
assert `"`e(stayers)'"' == "movers"
assert e(stayers_option_supplied) == 1

capture quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) preconditioner(diagonal) batch(2) probes(4) ///
    probeorder(replicate) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
assert `"`e(backend_selected)'"' == ""
assert `"`e(rng_selected)'"' == ""
foreach auto_option in "algorithm(auto)" "engine(compressed)" {
    local algorithm_option algorithm(jla)
    local engine_option engine(generic)
    local preconditioner_option preconditioner(diagonal)
    if "`auto_option'" == "algorithm(auto)" local algorithm_option
    if inlist("`auto_option'","engine(auto)","engine(compressed)") ///
        local engine_option
    capture quietly vckss outcome control, worker(worker) firm(firm) ///
        deletion(observation) backend(rust) rng(counter_v1)        ///
        `algorithm_option' `engine_option' `preconditioner_option' ///
        batch(2) probes(4) `auto_option' nodisplay
    assert _rc == 498
    assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
    assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
}


// No-control match engine(auto) is resolved before Counter-V1 begins and may
// select the compressed result family.  The public poster must retain that
// family's own V1/V7 receipts rather than manufacture generic diagnostics.
local compressed_auto_rng `"`c(rng)'"'
local compressed_auto_stream = c(rngstream)
local compressed_auto_state `"`c(rngstate)'"'
local compressed_auto_sortedby : sortedby
quietly _datasignature
local compressed_auto_signature `"`r(datasignature)'"'
quietly vckss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(4) seed(81227) tolerance(1e-12) memory_gib(1) nodisplay
assert `"`e(cmd)'"' == "vckss"
assert `"`e(status)'"' == "KSS_POINT_ESTIMATES_ONLY"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-COMPRESSED-PLANNED-V4-V7"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(deletion)'"' == "match"
assert `"`e(nuisance)'"' == "joint"
assert `"`e(deletion_rank_certificate)'"' == ///
    "compressed match graph, quotient, and complete-model residual gates"
assert e(N_requested) == 96
assert e(N_complete) == 96
assert e(N_retained) == 96
assert e(N_physical) == 192
assert e(worker_levels) == 12
assert e(firm_levels) == 4
assert e(parameters) == e(worker_levels)+e(firm_levels)-1
assert e(full_parameters) == e(parameters)
assert e(correction_parameters) == e(parameters)
assert e(controls_count) == 0
assert e(coefficient_cells) == 48
assert e(deletion_units) == 48
assert e(target_strata) == 48
assert e(target_weight_sum) == e(N_physical)
assert e(probes) == 4
assert e(seed) == 81227
assert e(rust_requested_algorithm_code) == 2
assert e(rust_selected_algorithm_code) == 2
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_solver_dimension) == e(firm_levels)-1
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_result_cap_schema) == 3
assert e(rust_result_cap_profile) == 4
assert e(rust_cap_schema) == 3
assert e(rust_cap_profile_code) == 4
assert e(rust_cap_engine_deferred) == 1
assert e(rust_plan_schema) == 1
assert e(rust_plan_route_schema) == 2
assert e(rust_plan_resolved) == 1
assert e(rust_plan_frozen) == 1
assert e(rust_plan_applicability) == 2
assert e(rust_plan_algorithm_requested) == 2
assert e(rust_plan_algorithm_selected) == 2
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_route_requested) == 2
assert e(rust_plan_route_selected) == 2
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0
assert e(rust_pre_rng_lo) == 0
assert e(rust_plan_rhs) == 1+3*e(probes)
assert e(rust_plan_full_dimension) == e(firm_levels)-1
assert e(rust_leverage_probes_accepted) == e(probes)
assert e(rust_target_probes_accepted) == e(probes)
assert e(rust_leverage_rhs_count) == e(probes)
assert e(rust_target_rhs_count) == 2*e(probes)
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert inrange(e(leverage_batch),1,e(probes))
assert inrange(e(target_batch),1,e(probes))
assert e(rust_stayers_mode_code) == 1
assert e(rust_target_weight_mode_code) == 0
assert e(rust_deletion_source_code) == 2
assert e(rust_probeorder_supplied) == 0
assert e(rust_wallseconds_supplied) == 0
assert e(rust_wall_request_applicable) == 0
assert e(rust_wallseconds_requested) == 0
assert e(rust_frequency_use_code) == 1
assert e(rust_solve_physical_limit) == e(physical_limit)
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)
assert e(rust_rhs_v2_copy_bytes) == 0
assert e(rust_result_copy_bytes) == 112*(1+3*e(probes))
assert e(algorithm_option_supplied) == 1
assert e(engine_option_supplied) == 1
assert e(preconditioner_option_supplied) == 1
assert e(batch_option_supplied) == 1
assert e(targetweight_option_supplied) == 0

tempname compressed_auto_results compressed_auto_rhs compressed_auto_public_rhs
    tempname compressed_auto_memory compressed_auto_prep compressed_auto_graph
    tempname compressed_auto_cap compressed_auto_receipt
matrix `compressed_auto_results' = e(results)
matrix `compressed_auto_rhs' = e(rust_rhs_receipts)
matrix `compressed_auto_public_rhs' = e(solver_rhs_diagnostics)
matrix `compressed_auto_memory' = e(rust_memory_receipt)
matrix `compressed_auto_prep' = e(rust_preparation_receipt)
matrix `compressed_auto_graph' = e(rust_graph_receipt)
matrix `compressed_auto_cap' = e(rust_request_capability_receipt)
matrix `compressed_auto_receipt' = e(rust_compressed_receipt)
assert rowsof(`compressed_auto_results') == 4 & colsof(`compressed_auto_results') == 4
assert rowsof(`compressed_auto_rhs') == 1+3*e(probes)
assert colsof(`compressed_auto_rhs') == 8
assert rowsof(`compressed_auto_public_rhs') == rowsof(`compressed_auto_rhs')
assert colsof(`compressed_auto_public_rhs') == 6
assert colsof(`compressed_auto_memory') == 12
assert `compressed_auto_memory'[1,3] == e(rust_result_copy_bytes)
assert `compressed_auto_memory'[1,4] == 0
assert `compressed_auto_memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `compressed_auto_memory'[1,12] == max(                     ///
    `compressed_auto_memory'[1,5],`compressed_auto_memory'[1,11])
assert `compressed_auto_memory'[1,12] <= `compressed_auto_memory'[1,1]
assert colsof(`compressed_auto_prep') == 13
assert colsof(`compressed_auto_graph') == 18
assert colsof(`compressed_auto_cap') == 23
assert colsof(`compressed_auto_receipt') == 11
assert `compressed_auto_receipt'[1,2] == 1
assert `compressed_auto_receipt'[1,10] == 2
assert `compressed_auto_receipt'[1,11] == 1
forvalues row = 1/3 {
    assert abs(`compressed_auto_results'[`row',4]-                  ///
        `compressed_auto_results'[`row',1]-                        ///
        `compressed_auto_results'[`row',2]-                        ///
        2*`compressed_auto_results'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`compressed_auto_results'[1,`column']-              ///
        `compressed_auto_results'[2,`column']-                    ///
        `compressed_auto_results'[3,`column']) <= 1e-10
}
quietly count if e(sample)
assert r(N) == e(N_retained)
capture confirm matrix e(rust_generic_receipt)
assert _rc != 0
capture confirm matrix e(rust_control_rank_receipt)
assert _rc != 0
capture assert e(rust_generic_diagnostic_flags) < .
assert _rc != 0
capture assert e(rust_control_schur_rcond) < .
assert _rc != 0
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_auto_rng'"'
assert c(rngstream) == `compressed_auto_stream'
assert `"`c(rngstate)'"' == `"`compressed_auto_state'"'
local compressed_auto_sortedby_after : sortedby
assert `"`compressed_auto_sortedby_after'"' == `"`compressed_auto_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_auto_signature'"'


// Automatic preconditioning must preserve the compressed engine choice and
// select the registered exact/direct route for this small quotient before
// Counter-V1 begins.  Advisory wall planning may report work but may not
// change results or caller state.
local compressed_preauto_rng `"`c(rng)'"'
local compressed_preauto_stream = c(rngstream)
local compressed_preauto_state `"`c(rngstate)'"'
local compressed_preauto_sortedby : sortedby
quietly _datasignature
local compressed_preauto_signature `"`r(datasignature)'"'
quietly vckss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto) ///
    batch(auto) probes(4) seed(81227) tolerance(1e-12) memory_gib(1) ///
    wallseconds(60) nodisplay
tempname compressed_preauto_results compressed_preauto_memory ///
    compressed_preauto_rhs
matrix `compressed_preauto_results' = e(results)
matrix `compressed_preauto_memory' = e(rust_memory_receipt)
matrix `compressed_preauto_rhs' = e(rust_rhs_receipts)
assert mreldif(`compressed_preauto_results',`compressed_auto_results') <= 1e-12
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "exact"
assert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 1
assert e(route_code) == 1
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 1
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert colsof(`compressed_preauto_rhs') == 8
forvalues row = 1/`=rowsof(`compressed_preauto_rhs')' {
    assert `compressed_preauto_rhs'[`row',4] == 1
}
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_wallseconds_forecast) >= 0
assert e(rust_wallseconds_advisory) >= 0
assert e(rust_wallseconds_margin) >= 0
assert `compressed_preauto_memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `compressed_preauto_memory'[1,12] == max(                 ///
    `compressed_preauto_memory'[1,5],`compressed_preauto_memory'[1,11])
assert `compressed_preauto_memory'[1,12] <= `compressed_preauto_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_preauto_rng'"'
assert c(rngstream) == `compressed_preauto_stream'
assert `"`c(rngstate)'"' == `"`compressed_preauto_state'"'
local compressed_preauto_sort_after : sortedby
assert `"`compressed_preauto_sort_after'"' == `"`compressed_preauto_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_preauto_signature'"'

// A forced CMG compressed request must remain on CMG: setup failure may not
// fall back, batching is frozen before RNG, and the accepted scientific result
// must agree with the diagonal route up to registered numerical tolerance.
local compressed_cmg_rng `"`c(rng)'"'
local compressed_cmg_stream = c(rngstream)
local compressed_cmg_state `"`c(rngstate)'"'
local compressed_cmg_sortedby : sortedby
quietly _datasignature
local compressed_cmg_signature `"`r(datasignature)'"'
quietly vckss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(cmg) ///
    batch(2) probes(4) seed(81227) tolerance(1e-12) memory_gib(1) nodisplay
tempname compressed_cmg_results compressed_cmg_memory
matrix `compressed_cmg_results' = e(results)
matrix `compressed_cmg_memory' = e(rust_memory_receipt)
assert mreldif(`compressed_cmg_results',`compressed_auto_results') <= 1e-9
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "cmg"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 3
assert e(rust_selected_route) == 3
assert e(route_code) == 3
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 1
assert e(rust_leverage_batch_mode_code) == 1
assert e(rust_target_batch_mode_code) == 1
assert e(leverage_batch) == 2
assert e(target_batch) == 2
assert e(batch) == 2
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_route_requested) == 3
assert e(rust_plan_route_selected) == 3
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert `compressed_cmg_memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `compressed_cmg_memory'[1,12] == max(                     ///
    `compressed_cmg_memory'[1,5],`compressed_cmg_memory'[1,11])
assert `compressed_cmg_memory'[1,12] <= `compressed_cmg_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_cmg_rng'"'
assert c(rngstream) == `compressed_cmg_stream'
assert `"`c(rngstate)'"' == `"`compressed_cmg_state'"'
local compressed_cmg_sortedby_after : sortedby
assert `"`compressed_cmg_sortedby_after'"' == `"`compressed_cmg_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_cmg_signature'"'

// With no controls, fixed-offset and joint nuisance dimensions coincide, but
// the native capability, plan, and public result must still retain the
// requested fixed-offset and explicit stored-row target-weight conventions.
local compressed_fixed_rng `"`c(rng)'"'
local compressed_fixed_stream = c(rngstream)
local compressed_fixed_state `"`c(rngstate)'"'
local compressed_fixed_sortedby : sortedby
quietly _datasignature
local compressed_fixed_signature `"`r(datasignature)'"'
quietly vckss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(fixedoffset) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(5) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
tempname compressed_fixed_results compressed_fixed_capability
matrix `compressed_fixed_results' = e(results)
matrix `compressed_fixed_capability' = e(rust_request_capability_receipt)
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(nuisance)'"' == "fixedoffset"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert e(controls_count) == 0
assert e(parameters) == e(worker_levels)+e(firm_levels)-1
assert e(full_parameters) == e(parameters)
assert e(correction_parameters) == e(parameters)
assert e(N_physical) == 192
assert e(target_weight_sum) == 144
assert e(rust_target_weight_mode_code) == 1
assert e(targetweight_option_supplied) == 1
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2
assert `compressed_fixed_capability'[1,9] == 2
assert `compressed_fixed_capability'[1,17] == 1
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
forvalues row = 1/3 {
    assert abs(`compressed_fixed_results'[`row',4]-               ///
        `compressed_fixed_results'[`row',1]-                     ///
        `compressed_fixed_results'[`row',2]-                     ///
        2*`compressed_fixed_results'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`compressed_fixed_results'[1,`column']-           ///
        `compressed_fixed_results'[2,`column']-                 ///
        `compressed_fixed_results'[3,`column']) <= 1e-10
}
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_fixed_rng'"'
assert c(rngstream) == `compressed_fixed_stream'
assert `"`c(rngstate)'"' == `"`compressed_fixed_state'"'
local compressed_fixed_sortedby_after : sortedby
assert `"`compressed_fixed_sortedby_after'"' == `"`compressed_fixed_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_fixed_signature'"'

// Registered deletion/nuisance/control/weight combinations are all public.
foreach deletion in match observation {
    local deletion_option
    if "`deletion'" == "match" local deletion_option deletionid(deletion_id)
    foreach nuisance in joint fixedoffset {
        quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
            deletion(`deletion') `deletion_option' nuisance(`nuisance') ///
            algorithm(jla) backend(rust) rng(counter_v1) engine(generic) ///
            preconditioner(diagonal) batch(3) probes(5) seed(81227) ///
            targetweight(target_weight) tolerance(1e-12) memory_gib(1) nodisplay
        assert `"`e(deletion)'"' == "`deletion'"
        assert `"`e(nuisance)'"' == "`nuisance'"
        assert e(rust_leverage_probes_accepted) == 5
        assert e(rust_target_probes_accepted) == 5
        assert e(target_weight_sum) == 144
        if "`deletion'" == "observation" assert e(deletion_units) == e(N_physical)
        else assert e(deletion_units) == 48
        if "`nuisance'" == "joint" assert e(parameters) == e(full_parameters)
        else assert e(parameters) == e(worker_levels)+e(firm_levels)-1
        local distinct_working = ("`nuisance'"=="fixedoffset")
        assert rowsof(e(rust_rhs_receipts)) == 1+1+`distinct_working'+3*5
    }
}

// Q=0 and factor controls use the actual materialized control count.
quietly vckss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(fixedoffset) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 0
assert rowsof(e(rust_rhs_receipts)) == 1+3*4
tempname q0_control_rank
matrix `q0_control_rank' = e(rust_control_rank_receipt)
assert `q0_control_rank'[1,1] == 1
assert `q0_control_rank'[1,2] == 1
assert `q0_control_rank'[1,3] == 1
forvalues column = 4/7 {
    assert `q0_control_rank'[1,`column'] == 0
}
assert `q0_control_rank'[1,8] == max(e(rust_rank_tolerance),1e-12)
assert `q0_control_rank'[1,9] == 1e-13
assert `q0_control_rank'[1,10] == 1e-11

// Multiple deletion IDs may occupy one coefficient cell.
generate long deletion_subcell = 2*cell+replicate+1
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_subcell) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(coefficient_cells) == 48
assert e(deletion_units) == 96
assert e(deletion_units) > e(coefficient_cells)

// Factor dummies and interactions materialize in the same semantic order as
// their explicit numeric columns and therefore use identical Counter atoms.
generate byte category = mod(7*_n,5)
forvalues level = 1/4 {
    generate byte category_`level' = category == `level'
}
generate double control_rep0 = control*(replicate==0)
generate double control_rep1 = control*(replicate==1)
tempname factor_results factor_rhs factor_interaction_results
tempname factor_interaction_rhs
quietly vckss outcome ib0.category [fw=frequency],            ///
    worker(worker) firm(firm)                                      ///
    deletion(observation) nuisance(joint) algorithm(jla)           ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 4
assert e(rust_cap_supported) == 1
matrix `factor_results' = e(results)
matrix `factor_rhs' = e(rust_rhs_receipts)
quietly vckss outcome category_1 category_2 category_3 category_4 ///
    [fw=frequency], worker(worker) firm(firm) deletion(observation) ///
    nuisance(joint) algorithm(jla) backend(rust)                    ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 4
assert mreldif(`factor_results',e(results)) == 0
assert mreldif(`factor_rhs',e(rust_rhs_receipts)) == 0
quietly vckss outcome c.control#ib0.replicate [fw=frequency], ///
    worker(worker) firm(firm) deletion(observation) nuisance(joint) ///
    algorithm(jla) backend(rust) rng(counter_v1) engine(generic)   ///
    preconditioner(diagonal) batch(2) probes(4) seed(81227)        ///
    memory_gib(1) nodisplay
assert e(controls_count) == 2
matrix `factor_interaction_results' = e(results)
matrix `factor_interaction_rhs' = e(rust_rhs_receipts)
quietly vckss outcome control_rep0 control_rep1 [fw=frequency], worker(worker) ///
    firm(firm) deletion(observation) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(controls_count) == 2
assert mreldif(`factor_interaction_results',e(results)) == 0
assert mreldif(`factor_interaction_rhs',e(rust_rhs_receipts)) == 0

// The advertised Q=32 boundary is accepted; Q=33 is declined before prepare.
preserve
clear
set obs 544
generate long row0 = _n-1
generate long repetition_q = mod(row0,34)
generate long firm_q = mod(floor(row0/34),4)+1
generate long worker_q = floor(row0/(4*34))+1
generate double outcome_q = .4*(worker_q-1)-.3*(firm_q-1)+      ///
    repetition_q/17+mod(row0,9)/23
generate double target_q = .75+(row0+1)/1000
local controls32
forvalues q = 1/33 {
    generate double q`q' = cos(c(pi)*(repetition_q+.5)*`q'/34)
    if `q' <= 32 local controls32 `controls32' q`q'
}
capture quietly vckss outcome_q `controls32', worker(worker_q) firm(firm_q) ///
    deletion(observation) nuisance(joint) algorithm(jla)            ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(3) seed(81227) tolerance(1e-11) memory_gib(1) ///
    targetweight(target_q) nodisplay
local q32_rc = _rc
if !`q32_rc' {
    assert e(controls_count) == 32
    assert e(full_parameters) == e(worker_levels)+e(firm_levels)-1+32
    assert e(parameters) == e(full_parameters)
}
capture quietly vckss outcome_q `controls32' q33, worker(worker_q) firm(firm_q) ///
    deletion(observation) nuisance(joint) algorithm(jla)            ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(3) seed(81227) memory_gib(1) nodisplay
local q33_rc = _rc
assert `q33_rc' == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
assert e(rust_cap_reason_code) == 9
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
restore

// if/in and observation deletion exclude incomplete rows while preserving
// the original requested-sample e(sample) map.
generate byte eligible = _n <= 80
generate double outcome_missing = outcome
replace outcome_missing = . in 10
quietly vckss outcome_missing control if eligible in 1/70 [fw=frequency], ///
    worker(worker) firm(firm) deletion(observation) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) seed(81227) memory_gib(1) nodisplay
assert e(N_requested) == 70 & e(N_complete) == 69
assert !e(sample) in 10
quietly count if e(sample)
assert r(N) == e(N_retained)
capture quietly vckss outcome_missing control if eligible in 1/70, ///
    worker(worker) firm(firm) deletion(match) deletionid(deletion_id) ///
    nuisance(joint) algorithm(jla) backend(rust) rng(counter_v1) engine(generic) ///
    preconditioner(diagonal) batch(2) probes(4) nodisplay
assert _rc == 459 & `"`e(withholding_status)'"' == "MATCH_INPUT_MISSING"

// Canonical row order makes storage permutation irrelevant.
generate long original_order = _n
gsort -cell -replicate
quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
assert mreldif(e(results),`public_reference') == 0
sort original_order

// Registered public resource and rank gates fail closed and return idle.
local failure_rng `"`c(rng)'"'
local failure_stream = c(rngstream)
local failure_state `"`c(rngstate)'"'
local failure_sortedby : sortedby
quietly _datasignature
local failure_signature `"`r(datasignature)'"'
capture quietly vckss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) physical_limit(1) memory_gib(1) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "PHYSICAL_COPY_LIMIT"
assert `"`e(native_error_phase)'"' == "physical_limit"
assert e(native_error_code) >= .
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
assert `"`c(rng)'"' == `"`failure_rng'"'
assert c(rngstream) == `failure_stream'
assert `"`c(rngstate)'"' == `"`failure_state'"'
local failure_sortedby_after : sortedby
assert `"`failure_sortedby_after'"' == `"`failure_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`failure_signature'"'
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
capture quietly vckss outcome, worker(worker) firm(firm)        ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
    batch(2) probes(4) blocksize_limit(1) nodisplay
assert _rc == 198 & `"`e(withholding_status)'"' == "BLOCK_SIZE_LIMIT"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
capture quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(.000001) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',"RESOURCE_LIMIT",       ///
    "ALLOCATION_FAILED")
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`failure_rng'"'
assert c(rngstream) == `failure_stream'
assert `"`c(rngstate)'"' == `"`failure_state'"'
local allocation_sortedby_after : sortedby
assert `"`allocation_sortedby_after'"' == `"`failure_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`failure_signature'"'

// The fully explicit generic route never silently accepts an unconverged fit.
capture quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) maxiter(1) memory_gib(1) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "PCG_MAXITER"
assert `"`e(native_error_phase)'"' == "solve"
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`failure_state'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`failure_signature'"'

generate double duplicate_control = control
local rank_rng_state `"`c(rngstate)'"'
local rank_sortedby : sortedby
quietly _datasignature
local rank_signature `"`r(datasignature)'"'
capture quietly vckss outcome control duplicate_control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(1) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',"SINGULAR_INFORMATION", ///
    "AMBIGUOUS_CONTROL_BASIS")
assert e(native_error_code) < .
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`rank_rng_state'"'
local rank_sortedby_after : sortedby
assert `"`rank_sortedby_after'"' == `"`rank_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`rank_signature'"'
generate byte spike_control = _n == 1
capture quietly vckss outcome spike_control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(1) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "UNVERIFIED_DELETION_RANK"
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

// Missing/corrupt capability receipts fail before prepare; a corrupted V6
// echo fails after result export and still releases to idle.
capture program drop _vckss_rust_public_call
program define _vckss_rust_public_call, rclass
    version 18.0
    gettoken subcommand rest : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "prepare" {
        global VCKSS_GENERIC_PREPARE_CALLED = $VCKSS_GENERIC_PREPARE_CALLED+1
    }
    if "${VCKSS_GENERIC_FAULT}" == "missing_capability" &          ///
        "`subcommand'" == "requestcapability" exit 199
    if "${VCKSS_GENERIC_FAULT}" == "userbreak_solve" &             ///
        "`subcommand'" == "solve" exit 1
    quietly vckss_rust `0'
    local signature_lo = cond("`subcommand'"=="requestcapability", ///
        r(request_signature_lo),.)
    local result_signature_lo = cond("`subcommand'"=="result",     ///
        r(request_signature_lo),.)
    tempname returned_rhs
    if "`subcommand'" == "result" matrix `returned_rhs' = r(rhs_receipts)
    return add
    if "${VCKSS_GENERIC_FAULT}" == "corrupt_capability" &          ///
        "`subcommand'" == "requestcapability" {
        return scalar request_signature_lo = mod(`signature_lo'+1,4294967296)
    }
    if "${VCKSS_GENERIC_FAULT}" == "corrupt_result" &              ///
        "`subcommand'" == "result" {
        return scalar request_signature_lo = mod(`result_signature_lo'+1,4294967296)
    }
    if "`subcommand'" == "result" {
        local corrupt_field
        local corrupt_value .
        if "${VCKSS_GENERIC_FAULT}" == "cr_rcond" {
            local corrupt_field control_rank_rcond
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_small" {
            local corrupt_field control_rank_smallest_lower
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_large" {
            local corrupt_field control_rank_largest_upper
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_projection" {
            local corrupt_field control_rank_projection_error
            local corrupt_value -1
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_normalization" {
            local corrupt_field control_rank_normalization_err
            local corrupt_value .25
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_fe_bound" {
            local corrupt_field control_rank_fe_info_lower
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_max_projection" {
            local corrupt_field control_rank_max_projection
            local corrupt_value -1
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_effective_tol" {
            local corrupt_field control_rank_effective_tolerance
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_pcg_tol" {
            local corrupt_field control_rank_projection_pcg_tol
            local corrupt_value 0
        }
        if "${VCKSS_GENERIC_FAULT}" == "cr_residual_gate" {
            local corrupt_field control_rank_projection_gate
            local corrupt_value 0
        }
        if "`corrupt_field'" != "" {
            return scalar `corrupt_field' = `corrupt_value'
        }
        if "${VCKSS_GENERIC_FAULT}" == "rhs_control_complete" {
            matrix `returned_rhs'[1,7] = `returned_rhs'[1,7]+1e-15
            return matrix rhs_receipts = `returned_rhs'
        }
        if "${VCKSS_GENERIC_FAULT}" == "rhs_full_reduced" {
            matrix `returned_rhs'[2,6] = `returned_rhs'[2,6]+1e-15
            return matrix rhs_receipts = `returned_rhs'
        }
    }
end
local reconcile_rng `"`c(rng)'"'
local reconcile_stream = c(rngstream)
local reconcile_state `"`c(rngstate)'"'
local reconcile_sortedby : sortedby
quietly _datasignature
local reconcile_signature `"`r(datasignature)'"'
foreach fault in missing_capability corrupt_capability {
    global VCKSS_GENERIC_FAULT `fault'
    global VCKSS_GENERIC_PREPARE_CALLED 0
    capture quietly vckss outcome control, worker(worker) firm(firm) ///
        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
        rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
        probes(4) memory_gib(1) nodisplay
    assert _rc == 498
    assert "$VCKSS_GENERIC_PREPARE_CALLED" == "0"
    assert `"`e(native_error_phase)'"' == cond("`fault'"=="missing_capability", ///
        "request_capability","request_capability_reconcile")
    assert `"`e(backend_requested)'"' == "rust"
    assert `"`e(algorithm)'"' == "jla"
    assert `"`e(engine_requested)'"' == "generic"
    assert `"`e(preconditioner_requested)'"' == "diagonal"
    assert `"`e(rng_requested)'"' == "counter_v1"
    assert e(backend_option_supplied) == 1
    assert e(algorithm_option_supplied) == 1
    assert e(engine_option_supplied) == 1
    assert e(preconditioner_option_supplied) == 1
    assert e(batch_option_supplied) == 1
    assert e(rng_option_supplied) == 1
    assert e(native_error_code) >= .
    assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
    quietly vckss_rust snapshot
    assert r(state) == 0 & r(handle) == 0
    assert `"`c(rng)'"' == `"`reconcile_rng'"'
    assert c(rngstream) == `reconcile_stream'
    assert `"`c(rngstate)'"' == `"`reconcile_state'"'
    local reconcile_sortedby_after : sortedby
    assert `"`reconcile_sortedby_after'"' == `"`reconcile_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`reconcile_signature'"'
}
foreach fault in corrupt_result cr_rcond cr_small cr_large cr_projection ///
    cr_normalization cr_fe_bound cr_max_projection cr_effective_tol     ///
    cr_pcg_tol cr_residual_gate rhs_control_complete rhs_full_reduced {
    global VCKSS_GENERIC_FAULT `fault'
    global VCKSS_GENERIC_PREPARE_CALLED 0
    capture quietly vckss outcome control, worker(worker) firm(firm) ///
        deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
        rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
        probes(4) memory_gib(1) nodisplay
    assert _rc == 498
    assert `"`e(native_error_phase)'"' == "result_reconcile"
    assert "$VCKSS_GENERIC_PREPARE_CALLED" == "1"
    assert e(native_error_code) >= .
    assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
    quietly vckss_rust snapshot
    assert r(state) == 0 & r(handle) == 0
    assert `"`c(rng)'"' == `"`reconcile_rng'"'
    assert c(rngstream) == `reconcile_stream'
    assert `"`c(rngstate)'"' == `"`reconcile_state'"'
    local reconcile_sortedby_after : sortedby
    assert `"`reconcile_sortedby_after'"' == `"`reconcile_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`reconcile_signature'"'
}

// UserBreak is untyped, clears e(), releases the prepared handle, and restores
// the complete caller state through the outer finally guard.
global VCKSS_GENERIC_FAULT userbreak_solve
global VCKSS_GENERIC_PREPARE_CALLED 0
capture quietly vckss outcome control, worker(worker) firm(firm) ///
    deletion(observation) nuisance(joint) algorithm(jla) backend(rust) ///
    rng(counter_v1) engine(generic) preconditioner(diagonal) batch(2) ///
    probes(4) memory_gib(1) nodisplay
assert _rc == 1
assert `"`e(cmd)'"' == ""
assert "$VCKSS_GENERIC_PREPARE_CALLED" == "1"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`reconcile_rng'"'
assert c(rngstream) == `reconcile_stream'
assert `"`c(rngstate)'"' == `"`reconcile_state'"'
local reconcile_sortedby_after : sortedby
assert `"`reconcile_sortedby_after'"' == `"`reconcile_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`reconcile_signature'"'
global VCKSS_GENERIC_FAULT
global VCKSS_GENERIC_PREPARE_CALLED
capture program drop _vckss_rust_public_call

assert `q32_rc' == 0

di as result "VCKSS RUST PUBLIC GENERIC PASS"
exit 0
