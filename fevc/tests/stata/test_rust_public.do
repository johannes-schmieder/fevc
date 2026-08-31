version 18.0
clear all
set more off
set varabbrev off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

set obs 48
generate long obsid = _n
generate long worker_n = floor((_n-1)/4) + 1
generate byte firm_n = mod(_n-1,4) + 1
generate str8 worker = "w" + string(worker_n,"%02.0f")
generate str8 firm = "f" + string(firm_n,"%02.0f")
generate str20 match = worker + "_" + firm
generate byte frequency = 1 + mod(firm_n,2)
generate double target = 1 + mod(worker_n,3) + firm_n/10
generate double y = .7*worker_n - .35*firm_n +                 ///
    cond(mod(worker_n+firm_n,2),-.2,.3)
generate byte eligible = 1
sort obsid

set rng kiss32
set seed 20260821
quietly findfile fevc_rng.mata
quietly do `"`r(fn)'"'
mata: VCKSS_RUST_PUBLIC_BEFORE = vckss_rng__capture_full()
mata: assert(VCKSS_RUST_PUBLIC_BEFORE.status == "OK")
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

quietly fevc y if eligible in 1/48 [fw=frequency],       ///
    worker(worker) firm(firm) deletion(match) deletionid(match) ///
    targetweight(target) algorithm(jla) engine(compressed)      ///
    stayers(movers)                                             ///
    preconditioner(diagonal) batch(3) probes(6) seed(91827)     ///
    tolerance(1e-10) maxiter(10000) backend(rust)               ///
    rng(counter_v1) nodisplay

assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(rng_contract)'"' == "VCKSS-COUNTER-V1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(preconditioner_selected)'"' == "DIAGONAL"
assert `"`e(fallback_status)'"' == "NOT_NEEDED"
assert e(rust_support_flags) == 38
assert mod(floor(e(rust_core_ready_flags)/1),2) == 1
assert mod(floor(e(rust_core_ready_flags)/4),2) == 1
assert mod(floor(e(rust_core_ready_flags)/8),2) == 1
assert mod(floor(e(rust_core_ready_flags)/32),2) == 1
assert mod(floor(e(rust_core_ready_flags)/64),2) == 1
assert mod(floor(e(rust_core_ready_flags)/128),2) == 1
assert mod(floor(e(rust_core_ready_flags)/256),2) == 1
assert e(N_stored) == 48
assert e(N_physical) == 72
assert e(target_weight_sum) > 0
assert e(weighted_rss) >= 0
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert e(rust_full_fit_zero_rhs) == 0
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_leverage_probes_accepted) == 6
assert e(rust_target_probes_accepted) == 6
assert e(rust_leverage_rhs_count) == 6
assert e(rust_target_rhs_count) == 12
assert e(rng_master_seed) == 91827
assert e(rng_leverage_probe_first) == 1
assert e(rng_leverage_probe_last) == 6
assert e(rng_target_probe_first) == 1
assert e(rng_target_probe_last) == 6
assert rowsof(e(rust_rhs_receipts)) == 19
assert colsof(e(rust_rhs_receipts)) == 8
assert rowsof(e(solver_rhs_diagnostics)) == 19
assert colsof(e(solver_rhs_diagnostics)) == 6
assert e(rust_rhs_receipts)[1,1] == 1
assert e(rust_rhs_receipts)[1,2] == -1
assert e(rust_rhs_receipts)[1,3] == 0
assert e(rust_rhs_receipts)[2,1] == 2
assert e(rust_rhs_receipts)[8,1] == 3
assert e(rust_rhs_receipts)[8,3] == 1
assert e(rust_rhs_receipts)[9,3] == 2
assert e(solver_rhs_diagnostics)[2,3] == 1
assert e(solver_rhs_diagnostics)[7,3] == 6
assert e(solver_rhs_diagnostics)[8,3] == 1
assert e(solver_rhs_diagnostics)[9,3] == 2
assert e(solver_rhs_diagnostics)[18,3] == 11
assert e(solver_rhs_diagnostics)[19,3] == 12
mata: st_numscalar("__vckss_test_max_iterations", ///
    max(st_matrix("e(solver_rhs_diagnostics)")[.,4]))
mata: st_numscalar("__vckss_test_max_complete", ///
    max(st_matrix("e(solver_rhs_diagnostics)")[.,5]))
assert e(solver_iterations) == scalar(__vckss_test_max_iterations)
assert abs(e(complete_residual_max)-scalar(__vckss_test_max_complete)) <= ///
    4096*c(epsdouble)
mata: assert(all(st_matrix("e(solver_rhs_diagnostics)")[.,6] :== 1))
scalar drop __vckss_test_max_iterations
scalar drop __vckss_test_max_complete
assert e(rust_memory_receipt)[1,11] <= e(rust_memory_receipt)[1,1]
assert e(rust_memory_receipt)[1,3] == 19*14*8
assert e(rust_memory_receipt)[1,11] == e(memory_forecast_bytes)
assert e(rust_memory_receipt)[1,4] ==                       ///
    e(rust_memory_receipt)[1,2] + e(rust_preparation_receipt)[1,1]*768 + 4096
assert e(rust_memory_receipt)[1,2] + e(rust_memory_receipt)[1,5] <= ///
    e(rust_memory_receipt)[1,1]
assert e(rust_memory_receipt)[1,6] > 0
assert e(rust_memory_receipt)[1,7] > 0
assert e(rust_memory_receipt)[1,8] > 0
assert e(rust_memory_receipt)[1,9] >= e(rust_memory_receipt)[1,3]
assert e(rust_memory_receipt)[1,10] >=                      ///
    e(rust_memory_receipt)[1,5] + e(rust_memory_receipt)[1,6] + ///
    e(rust_memory_receipt)[1,9] +                            ///
    max(e(rust_memory_receipt)[1,7],e(rust_memory_receipt)[1,8])
assert e(rust_preparation_receipt)[1,2] == e(N_retained)
assert e(rust_preparation_receipt)[1,8] == e(target_weight_sum)
assert e(rust_graph_receipt)[1,5] > 0
assert e(rust_graph_receipt)[1,6] >= e(rust_graph_receipt)[1,5]
assert e(rust_graph_receipt)[1,10] == e(deletion_units)
assert e(rust_graph_receipt)[1,18] ==                         ///
    e(rust_graph_receipt)[1,15] + e(rust_graph_receipt)[1,16] + ///
    e(rust_graph_receipt)[1,17]
assert abs(e(results)[1,4] -                              ///
    (e(results)[1,1]+e(results)[1,2]+2*e(results)[1,3])) < 1e-10
assert abs(e(results)[2,4] -                              ///
    (e(results)[2,1]+e(results)[2,2]+2*e(results)[2,3])) < 1e-10
assert abs(e(results)[3,4] -                              ///
    (e(results)[3,1]+e(results)[3,2]+2*e(results)[3,3])) < 1e-10
forvalues component = 1/4 {
    assert abs(e(results)[1,`component']-e(results)[2,`component']- ///
        e(results)[3,`component']) <= 4096*c(epsdouble)*           ///
        max(1,abs(e(results)[1,`component']),                      ///
            abs(e(results)[2,`component']),abs(e(results)[3,`component']))
}
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
local first_restored_sortedby : sortedby
assert `"`first_restored_sortedby'"' == `"`caller_sortedby'"'

tempname public_results public_rhs public_graph public_memory
matrix `public_results' = e(results)
matrix `public_rhs' = e(rust_rhs_receipts)
matrix `public_graph' = e(rust_graph_receipt)
matrix `public_memory' = e(rust_memory_receipt)
generate byte public_sample = e(sample)

// Match deletion and the worker-firm deletion ID are the public defaults.
quietly fevc y [fw=frequency], worker(worker) firm(firm) ///
    targetweight(target) algorithm(jla) engine(compressed)      ///
    stayers(movers)                                             ///
    preconditioner(diagonal) batch(3) probes(6) seed(91827)     ///
    tolerance(1e-10) maxiter(10000) backend(rust)               ///
    rng(counter_v1) nodisplay
assert `"`e(deletion)'"' == "match"
assert e(deletionid_option_supplied) == 0
assert mreldif(`public_results',e(results)) == 0
quietly count if public_sample != e(sample)
assert r(N) == 0

// The developer lifecycle receives the identical full validated columns.
tempvar dense_worker dense_firm dense_match rust_keep
quietly egen long `dense_worker' = group(worker)
quietly egen long `dense_firm' = group(firm)
quietly egen long `dense_match' = group(match)
quietly fevc_rust prepare `dense_worker' `dense_firm'    ///
    `dense_match' y frequency target, cleanup generate(`rust_keep') ///
    memorygib(4)
local handle = r(handle)
assert r(retained_rows) == 48
assert r(graph_retained_physical_mass) == 72
assert r(target_weight_sum) == e(target_weight_sum)
quietly fevc_rust solve `handle', seed(91827) probes(6) ///
    leveragebatch(3) targetbatch(3) route(diagonal)            ///
    tolerance(1e-10) maxiter(10000)
quietly fevc_rust result `handle'
assert mreldif(`public_results',r(result)) == 0
assert mreldif(`public_rhs',r(rhs_receipts)) == 0
quietly count if public_sample != `rust_keep'
assert r(N) == 0
quietly fevc_rust release `handle'
quietly fevc_rust snapshot
assert r(state) == 0
assert r(handle) == 0

// A genuinely nontrivial if/in intersection is carried through the native
// retained mask and posted losslessly as e(sample).
replace eligible = inrange(obsid,2,47)
quietly fevc y if eligible in 3/46 [fw=frequency],       ///
    worker(worker) firm(firm) deletion(match) targetweight(target) ///
    algorithm(jla) engine(compressed) preconditioner(diagonal)  ///
    stayers(movers)                                             ///
    batch(2) probes(4) seed(91827) tolerance(1e-10)             ///
    maxiter(10000) backend(rust) rng(counter_v1) nodisplay
quietly count if e(sample)
assert r(N) > 0 & r(N) < 48
assert e(sample) == 0 if !eligible | !inrange(obsid,3,46)
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
replace eligible = 1

// Counter-V1 and all receipts are invariant to caller row order and batching.
gsort -obsid
// The row-order perturbation intentionally advances Stata's independent sort
// jumbler.  Snapshot after that test action so every subsequent command must
// preserve the caller state it actually received.
mata: VCKSS_RUST_PUBLIC_BEFORE = vckss_rng__capture_full()
mata: assert(VCKSS_RUST_PUBLIC_BEFORE.status == "OK")
quietly fevc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)      ///
    algorithm(jla) engine(auto) preconditioner(diagonal)        ///
    batch(2) probes(6) seed(91827) tolerance(1e-10)             ///
    maxiter(10000) backend(rust) rng(counter_v1) nodisplay
assert mreldif(`public_results',e(results)) == 0
assert mreldif(`public_graph',e(rust_graph_receipt)) == 0
quietly count if public_sample != e(sample)
assert r(N) == 0

// A native preparation failure preserves its primary error and releases all
// engine state before returning to the caller.
capture quietly fevc y [fw=frequency],                   ///
    worker(worker) firm(firm) deletion(match) deletionid(match) ///
    targetweight(target) algorithm(jla) engine(compressed)      ///
    stayers(movers)                                             ///
    preconditioner(diagonal) batch(2) probes(6) seed(91827)     ///
    tolerance(1e-10) maxiter(10000) memory_gib(.000001)        ///
    backend(rust) rng(counter_v1) nodisplay
local preparation_failure_rc = _rc
assert `preparation_failure_rc' == 909
assert `"`e(status)'"' == "WITHHELD"
assert `"`e(withholding_status)'"' == "RESOURCE_LIMIT"
assert e(native_error_code) == 40
assert `"`e(native_error_phase)'"' == "prepare"
quietly fevc_rust snapshot
assert r(state) == 0
assert r(handle) == 0

// The post-prepare physical-mass policy failure releases before posting its
// typed result and leaves the native registry idle.
tempvar huge_frequency
generate double `huge_frequency' = 1100000
capture quietly fevc y [fw=`huge_frequency'],            ///
    worker(worker) firm(firm) deletion(match) targetweight(target) ///
    algorithm(jla) engine(compressed) preconditioner(diagonal)   ///
    stayers(movers)                                              ///
    batch(2) probes(4) seed(91827) tolerance(1e-10)              ///
    maxiter(10000) backend(rust) rng(counter_v1) nodisplay
assert _rc == 498
assert `"`e(status)'"' == "WITHHELD"
assert `"`e(withholding_status)'"' == "PHYSICAL_TOTAL_LIMIT"
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0

// A native error remains structurally retrievable until the next operation.
tempvar bad_frequency
generate double `bad_frequency' = frequency
replace `bad_frequency' = -1 in 1
capture noisily fevc_rust prepare `dense_worker' `dense_firm' ///
    `dense_match' y `bad_frequency' target, cleanup                ///
    generate(__rust_error_keep) memorygib(4)
assert _rc == 198
quietly fevc_rust lasterror
assert r(native_error_code) == 22
assert `"`r(native_error_status)'"' == "INVALID_WEIGHT"
assert strpos(`"`r(native_error_detail)'"',"frequency") > 0
quietly fevc_rust snapshot
assert r(state) == 0

// Replace only the public-call dispatcher with a test proxy.  Cleanup and
// lasterror continue to use the real helper, so injected faults cannot defeat
// the outer finally guard.
capture program drop _fevc_rust_public_call
program define _fevc_rust_public_call, rclass
    version 18.0
    local arguments `"`0'"'
    gettoken subcommand rest : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    local mode "$VCKSS_RUST_PUBLIC_TEST_MODE"

    if "`mode'" == "solve_break" & "`subcommand'" == "solve" exit 1
    if "`mode'" == "result_break" & "`subcommand'" == "result" exit 1
    if "`mode'" == "release_break" & "`subcommand'" == "release" exit 1
    if "`mode'" == "solve_failure" & "`subcommand'" == "solve" exit 459
    if "`mode'" == "result_failure" & "`subcommand'" == "result" exit 459
    if "`mode'" == "release_failure" & "`subcommand'" == "release" exit 459

    fevc_rust `arguments'
    if "`subcommand'" == "prepare" {
        local retained_variable `"`r(retained_variable)'"'
        if "`mode'" == "prepare_break" exit 1
        if "`mode'" == "prepare_failure" exit 459
        local prepare_scalars handle input_rows retained_rows workers firms ///
            cells deletion_units target_strata target_weight_sum           ///
            memory_limit_bytes caller_copy_bytes                           ///
            preparation_peak_forecast_bytes prepared_resident_bytes       ///
            graph_input_rows graph_retained_rows                           ///
            graph_input_physical_mass graph_retained_physical_mass         ///
            graph_initial_components graph_maximum_components              ///
            graph_initial_component_rows graph_mover_input_rows            ///
            graph_initial_deletion_edges graph_retained_deletion_edges     ///
            graph_degree_workers_removed graph_artic_workers_removed       ///
            graph_bridge_units_removed graph_bridge_rows_removed           ///
            graph_degree_iterations graph_articulation_iterations          ///
            graph_bridge_iterations graph_fixed_point_iterations
        local value_index = 0
        foreach name of local prepare_scalars {
            local value_index = `value_index' + 1
            local prepare_value`value_index' = r(`name')
        }
        // Preparation positions: memory limit=10, caller copy=11,
        // preparation peak=12, resident=13, initial/max components=18/19,
        // retained deletion edges=23, fixed-point iterations=31.
        if "`mode'" == "graph_zero_components" {
            local prepare_value18 = 0
            local prepare_value19 = 0
        }
        if "`mode'" == "graph_edge_mismatch" local prepare_value23 = ///
            `prepare_value23' + 1
        if "`mode'" == "graph_iteration_sum" local prepare_value31 = ///
            `prepare_value31' + 1
        if "`mode'" == "prepare_peak_formula" local prepare_value12 = ///
            `prepare_value12' + 8
        if "`mode'" == "prepare_resident_limit" local prepare_value13 = ///
            `prepare_value10'
        if "`mode'" == "copy_failure" drop `retained_variable'
        local value_index = 0
        foreach name of local prepare_scalars {
            local value_index = `value_index' + 1
            return scalar `name' = `prepare_value`value_index''
        }
        return local retained_variable "`retained_variable'"
        return local backend "rust"
        return local subcommand "prepare"
        exit 0
    }
    if "`subcommand'" == "result" {
        local result_scalars seed probes leverage_probes_accepted          ///
            target_probes_accepted requested_route selected_route          ///
            solver_fallback solver_fallback_error solver_dimension         ///
            leverage_batch_width target_batch_width rank_tolerance         ///
            block_tolerance full_residual_tolerance full_fit_route         ///
            full_fit_iterations full_fit_reduced_residual                  ///
            full_fit_complete_residual full_fit_zero_rhs                   ///
            leverage_rhs_count target_rhs_count max_reduced_residual       ///
            max_complete_residual max_leverage max_reciprocal_residual     ///
            accounting_residual topology_checksum_hi topology_checksum_lo ///
            rng_contract_code rhs_receipt_rows caller_result_copy_bytes    ///
            weighted_rss memory_limit_bytes caller_copy_bytes              ///
            preparation_peak_forecast_bytes prepared_resident_bytes       ///
            solver_setup_forecast_bytes leverage_phase_forecast_bytes     ///
            target_phase_forecast_bytes result_forecast_bytes             ///
            solve_peak_forecast_bytes command_peak_forecast_bytes handle
        local value_index = 0
        foreach name of local result_scalars {
            local value_index = `value_index' + 1
            local result_value`value_index' = r(`name')
        }
        tempname result_receipt rhs_receipt
        matrix `result_receipt' = r(result)
        matrix `rhs_receipt' = r(rhs_receipts)
        if "`mode'" == "raw_missing" matrix `result_receipt'[1,1] = .
        if "`mode'" == "corrected_algebra" matrix `result_receipt'[3,1] = ///
            `result_receipt'[3,1] + 1
        if "`mode'" == "rhs_sequence" matrix `rhs_receipt'[2,2] = 99
        if "`mode'" == "rhs_iteration" matrix `rhs_receipt'[2,5] = 10001
        if "`mode'" == "rhs_residual" matrix `rhs_receipt'[2,7] = .
        // Positions in result_scalars: solver_dimension=9,
        // max_complete_residual=23, accounting_residual=26,
        // RHS copy=31, prepared resident=36, solver setup=37,
        // leverage/target phase=38/39, result=40, solve/command peak=41/42.
        if "`mode'" == "solver_dimension" local result_value9 = ///
            `result_value9' + 1
        if "`mode'" == "max_summary" local result_value23 = ///
            `result_value23' + 1
        if "`mode'" == "accounting" local result_value26 = 1
        if "`mode'" == "solver_setup_zero" local result_value37 = 0
        if "`mode'" == "result_bytes_zero" local result_value40 = 0
        if "`mode'" == "result_under_copy" local result_value40 = ///
            `result_value31' - 1
        if "`mode'" == "solve_peak_inconsistent" {
            local result_value41 = `result_value36' + `result_value37' + ///
                `result_value40'
            local result_value42 = max(`result_value35',`result_value41')
        }
        return matrix result = `result_receipt'
        return matrix rhs_receipts = `rhs_receipt'
        local value_index = 0
        foreach name of local result_scalars {
            local value_index = `value_index' + 1
            return scalar `name' = `result_value`value_index''
        }
        return local rng_contract "VCKSS-COUNTER-V1"
        return local backend "rust"
        return local subcommand "result"
        exit 0
    }
    // The public lifecycle consumes no returned solve/release fields.
    return local backend "rust"
    return local subcommand "`subcommand'"
end

local strict_options worker(worker) firm(firm) deletion(match) ///
    targetweight(target) algorithm(jla) engine(compressed)     ///
    stayers(movers)                                            ///
    preconditioner(diagonal) batch(2) probes(4) seed(91827)    ///
    tolerance(1e-10) maxiter(10000) backend(rust) rng(counter_v1) ///
    nodisplay

foreach fault in prepare_break solve_break result_break release_break {
    global VCKSS_RUST_PUBLIC_TEST_MODE `fault'
    capture noisily fevc y [fw=frequency], `strict_options'
    di as text "lifecycle-fault `fault': rc=" _rc " cmd=`e(cmd)'"
    assert _rc == 1
    assert `"`e(cmd)'"' == ""
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

foreach fault in prepare_failure solve_failure result_failure release_failure {
    global VCKSS_RUST_PUBLIC_TEST_MODE `fault'
    capture noisily fevc y [fw=frequency], `strict_options'
    di as text "Stata-side fault `fault': rc=" _rc " cmd=`e(cmd)'"
    assert _rc == 459
    assert `"`e(cmd)'"' == ""
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

global VCKSS_RUST_PUBLIC_TEST_MODE copy_failure
capture quietly fevc y [fw=frequency], `strict_options'
assert _rc == 111
assert `"`e(cmd)'"' == ""
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0

foreach corruption in graph_zero_components graph_edge_mismatch       ///
    graph_iteration_sum prepare_peak_formula prepare_resident_limit {
    global VCKSS_RUST_PUBLIC_TEST_MODE `corruption'
    capture quietly fevc y [fw=frequency], `strict_options'
    di as text "corrupt-preparation `corruption': rc=" _rc       ///
        " status=`e(withholding_status)' phase=`e(native_error_phase)'"
    assert _rc == 498
    assert `"`e(status)'"' == "WITHHELD"
    assert `"`e(withholding_status)'"' == "INTERNAL_INVARIANT_FAILED"
    assert `"`e(native_error_phase)'"' == "preparation_reconcile"
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

foreach corruption in raw_missing corrected_algebra solver_dimension ///
    rhs_sequence rhs_iteration rhs_residual max_summary accounting    ///
    solver_setup_zero result_bytes_zero result_under_copy             ///
    solve_peak_inconsistent {
    global VCKSS_RUST_PUBLIC_TEST_MODE `corruption'
    capture quietly fevc y [fw=frequency], `strict_options'
    di as text "corrupt-receipt `corruption': rc=" _rc       ///
        " status=`e(withholding_status)' phase=`e(native_error_phase)'"
    assert _rc == 498
    assert `"`e(status)'"' == "WITHHELD"
    assert `"`e(withholding_status)'"' == "INTERNAL_INVARIANT_FAILED"
    assert `"`e(native_error_phase)'"' == "result_reconcile"
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

capture program drop _fevc_rust_public_call
program define _fevc_rust_public_call, rclass
    version 18.0
    fevc_rust `0'
    return add
end
macro drop VCKSS_RUST_PUBLIC_TEST_MODE

assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
mata:
VCKSS_RUST_PUBLIC_AFTER = vckss_rng__capture_full()
assert(VCKSS_RUST_PUBLIC_AFTER.status == "OK")
assert(VCKSS_RUST_PUBLIC_AFTER.active.algorithm ==
    VCKSS_RUST_PUBLIC_BEFORE.active.algorithm)
assert(VCKSS_RUST_PUBLIC_AFTER.active.stream ==
    VCKSS_RUST_PUBLIC_BEFORE.active.stream)
assert(VCKSS_RUST_PUBLIC_AFTER.active.state ==
    VCKSS_RUST_PUBLIC_BEFORE.active.state)
assert(VCKSS_RUST_PUBLIC_AFTER.sort_state ==
    VCKSS_RUST_PUBLIC_BEFORE.sort_state)
assert(VCKSS_RUST_PUBLIC_AFTER.mt64s_stream1_state ==
    VCKSS_RUST_PUBLIC_BEFORE.mt64s_stream1_state)
assert(VCKSS_RUST_PUBLIC_AFTER.mt64s_stream2_state ==
    VCKSS_RUST_PUBLIC_BEFORE.mt64s_stream2_state)
assert(VCKSS_RUST_PUBLIC_AFTER.mt64s_selected_stream_state ==
    VCKSS_RUST_PUBLIC_BEFORE.mt64s_selected_stream_state)
end

di as result "PASS test_rust_public.do"
