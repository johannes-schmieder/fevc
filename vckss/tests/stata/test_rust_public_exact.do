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

set obs 8
generate long obsid = _n
generate long worker = cond(_n <= 4, 1, 2)
generate long firm = cond(inlist(_n, 1, 2, 5, 6), 1, 2)
generate long deletion_id = _n
generate byte category = mod(_n,2)
generate byte eligible = 1
generate double y = .
generate double frequency = 1
generate double target = .
generate double control = .
local ys 1 2 0 2.5 -1 1.5 2 -2
local ts 1 2 2 1 3 1 2 4
local cs -1.5 -0.5 0.5 1.5 -1 0 1 2
forvalues row = 1/8 {
    local value : word `row' of `ys'
    quietly replace y = `value' in `row'
    local value : word `row' of `ts'
    quietly replace target = `value' in `row'
    local value : word `row' of `cs'
    quietly replace control = `value' in `row'
}
sort obsid
set rng kiss32
set seed 20260821
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

// Exact supports both nuisance conventions.  Every accepted RNG request and
// engine spelling produces the same deterministic result and consumes none.
foreach nuisance in joint fixedoffset {
    quietly vckss y control, worker(worker) firm(firm)          ///
        deletion(match) deletionid(deletion_id) algorithm(exact)     ///
        nuisance(`nuisance') targetweight(target) nodisplay
    tempname mata_results mata_plugin mata_correction rust_reference
    matrix `mata_results' = e(results)
    matrix `mata_plugin' = e(plugin)
    matrix `mata_correction' = e(correction)
    local mata_rss = e(weighted_rss)
    local mata_leverage = e(max_leverage)
    local first_rust = 1

    foreach requested_rng in omitted stata counter_v1 {
        local rng_option
        if "`requested_rng'" != "omitted" {
            local rng_option rng(`requested_rng')
        }
        foreach exact_engine in auto generic {
            quietly vckss y control, worker(worker) firm(firm)  ///
                deletion(match) deletionid(deletion_id) algorithm(exact) ///
                nuisance(`nuisance') targetweight(target)            ///
                backend(rust) `rng_option' engine(`exact_engine') nodisplay
            assert `"`e(version)'"' == "0.4.0-dev"
            assert `"`e(backend_selected)'"' == "rust"
            assert `"`e(algorithm)'"' == "exact"
            assert `"`e(deletion)'"' == "match"
            assert `"`e(nuisance)'"' == "`nuisance'"
            assert `"`e(engine_requested)'"' == "`exact_engine'"
            assert `"`e(engine_selected)'"' == "NOT_APPLICABLE"
            assert `"`e(preconditioner_selected)'"' == "NOT_APPLICABLE"
            assert `"`e(rng_requested)'"' ==                         ///
                cond("`requested_rng'" == "counter_v1",             ///
                    "counter_v1", "stata")
            assert `"`e(rng_selected)'"' == "NOT_APPLICABLE"
            assert `"`e(rng_contract)'"' == "NOT_APPLICABLE"
            assert e(rng_option_supplied) == ("`requested_rng'" != "omitted")
            assert e(rust_rng_contract_code) == 0
            assert e(rng_master_seed) == 0
            assert e(seed) == 0
            assert e(seed_requested) == 8675309
            assert e(probes) == 0
            assert e(probes_requested) == 200
            assert e(batch) == 0
            assert e(maxiter) == 0
            assert e(maxiter_requested) == 10000
            assert e(numerical_mcse_available) == 0
            assert e(route_planned_rhs) == 0
            assert e(rust_support_flags) == 38
            assert e(rust_cap_supported) == 1
            assert e(rust_cap_reason_code) == 0
            assert e(rust_cap_profile_code) == 1
            assert e(rust_cap_schema) == 1
            assert e(rust_request_capability_receipt)[1,12] == 1
            assert e(rust_request_capability_receipt)[1,13] == 0
            assert e(rust_exact_diagnostic_flags) == 511
            assert e(rust_full_fit_complete_residual) <=              ///
                e(residual_acceptance_tolerance)
            assert e(rust_working_fit_residual) <=                    ///
                e(residual_acceptance_tolerance)
            assert abs(e(rust_max_complete_residual) - max(           ///
                e(rust_full_fit_complete_residual),                   ///
                e(rust_working_fit_residual))) <= 4096*c(epsdouble)
            assert e(rust_inverse_sqrt_relres) >= 0
            assert e(rust_maker_relres) >= 0
            assert e(rust_control_basis_relres) >= 0
            assert e(rust_control_basis_forward_error) >= 0
            assert e(rust_deletion_rank_gap) > e(rust_block_tolerance)
            assert e(rust_firm_zero_sum_residual) >= 0
            assert e(rust_actual_accounting_residual) <= 1e-10
            assert e(rust_exact_memory_receipt)[1,3] == max(          ///
                e(rust_exact_memory_receipt)[1,1],                    ///
                e(rust_exact_memory_receipt)[1,2])
            assert e(rust_exact_memory_receipt)[1,5] <=              ///
                e(rust_exact_memory_receipt)[1,6]
            assert e(parameters) == cond("`nuisance'" == "joint",4,3)
            assert e(full_parameters) == 4
            assert e(correction_parameters) ==                       ///
                cond("`nuisance'" == "joint",4,3)
            assert mreldif(e(results),`mata_results') <= 1e-10
            assert mreldif(e(plugin),`mata_plugin') <= 1e-10
            assert mreldif(e(correction),`mata_correction') <= 1e-10
            assert abs(e(weighted_rss)-`mata_rss') <= 1e-10
            assert abs(e(max_leverage)-`mata_leverage') <= 1e-10
            if `first_rust' {
                matrix `rust_reference' = e(results)
                local first_rust = 0
            }
            else assert mreldif(e(results),`rust_reference') == 0
        }
    }
}
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'

// Factor controls become concrete nonomitted numeric columns before prepare.
quietly vckss y i.category, worker(worker) firm(firm)           ///
    deletion(match) deletionid(deletion_id) algorithm(exact) nodisplay
tempname mata_factor
matrix `mata_factor' = e(results)
quietly vckss y i.category, worker(worker) firm(firm)           ///
    deletion(match) deletionid(deletion_id) algorithm(exact)         ///
    backend(rust) engine(generic) nodisplay
assert e(rust_request_capability_receipt)[1,12] == 1
assert mreldif(e(results),`mata_factor') <= 1e-10

// Observation deletion counts literal physical copies and keeps target mass
// separate.  physical_limit() is only a JLA safeguard.
quietly replace frequency = 2 in 1
foreach nuisance in joint fixedoffset {
    quietly vckss y control [fw=frequency], worker(worker) firm(firm) ///
        deletion(observation) algorithm(exact) nuisance(`nuisance') ///
        targetweight(target) nodisplay
    tempname mata_observation
    matrix `mata_observation' = e(results)
    local mata_observation_units = e(deletion_units)
    quietly vckss y control [fw=frequency], worker(worker) firm(firm) ///
        deletion(observation) algorithm(exact) nuisance(`nuisance') ///
        targetweight(target) backend(rust) rng(stata) engine(generic) ///
        probes(7) batch(3) seed(99) tolerance(1e-12) maxiter(17)     ///
        physical_limit(1) nodisplay
    assert `"`e(target_population)'"' == "retained observations"
    assert e(deletion_units) == `mata_observation_units'
    assert e(deletion_units) == 9
    assert e(N_physical) == 9
    assert e(physical_limit) == 1
    assert e(physical_limit_applied) == 0
    assert `"`e(physical_limit_status)'"' == "NOT_APPLICABLE_TO_EXACT"
    assert e(probes) == 0 & e(probes_requested) == 7
    assert e(seed) == 0 & e(seed_requested) == 99
    assert e(batch) == 0 & e(batch_requested_numeric) == 3
    assert e(maxiter) == 0 & e(maxiter_requested) == 17
    assert e(tolerance) == 1e-12
    assert e(residual_acceptance_tolerance) == 1e-11
    assert e(rust_exact_diagnostic_flags) == 505
    assert e(rust_inverse_sqrt_relres) == 0
    assert e(rust_maker_relres) == 0
    assert e(correction_reciprocal_residual) == 0
    assert e(rust_request_capability_receipt)[1,13] == 1
    assert mreldif(e(results),`mata_observation') <= 1e-10
}

// A nontrivial if/in intersection is posted losslessly as e(sample).
quietly replace eligible = !inlist(obsid,1,8)
quietly vckss y if eligible in 2/7, worker(worker) firm(firm)   ///
    deletion(observation) algorithm(exact) nodisplay
tempname mata_subset
matrix `mata_subset' = e(results)
generate byte mata_sample = e(sample)
quietly vckss y if eligible in 2/7, worker(worker) firm(firm)   ///
    deletion(observation) algorithm(exact) backend(rust) nodisplay
assert mreldif(e(results),`mata_subset') <= 1e-10
quietly count if mata_sample != e(sample)
assert r(N) == 0
assert e(sample) == 0 if !eligible | !inrange(obsid,2,7)

// Row permutation and one-to-one ID relabeling preserve exact science.
quietly replace frequency = 1
quietly replace eligible = 1
quietly vckss y, worker(worker) firm(firm) deletion(match)     ///
    deletionid(deletion_id) algorithm(exact) nodisplay
tempname mata_order rust_order
matrix `mata_order' = e(results)
quietly vckss y, worker(worker) firm(firm) deletion(match)     ///
    deletionid(deletion_id) algorithm(exact) backend(rust) nodisplay
matrix `rust_order' = e(results)
assert mreldif(`rust_order',`mata_order') <= 1e-10
gsort -obsid
quietly vckss y, worker(worker) firm(firm) deletion(match)     ///
    deletionid(deletion_id) algorithm(exact) backend(rust) nodisplay
assert mreldif(e(results),`rust_order') <= 1e-10
quietly replace worker = 100 + 7*worker
quietly replace firm = 50 - 3*firm
quietly replace deletion_id = 1000 + 11*deletion_id
quietly vckss y, worker(worker) firm(firm) deletion(match)     ///
    deletionid(deletion_id) algorithm(exact) backend(rust) nodisplay
assert mreldif(e(results),`rust_order') <= 1e-10

// A corrupted detailed-V5 accounting receipt is rejected after result export
// and before any failure is posted; cleanup still leaves the engine idle.
capture program drop _vckss_rust_public_call
program define _vckss_rust_public_call, rclass
    version 18.0
    gettoken rust_subcommand rust_rest : 0, parse(" ,")
    vckss_rust `0'
    return add
    if lower(strtrim("`rust_subcommand'")) == "result" {
        return scalar actual_accounting_residual = 1
    }
end
capture quietly vckss y, worker(worker) firm(firm)             ///
    deletion(match) deletionid(deletion_id) algorithm(exact)         ///
    backend(rust) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "INTERNAL_INVARIANT_FAILED"
assert `"`e(native_error_phase)'"' == "result_reconcile"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
capture program drop _vckss_rust_public_call
program define _vckss_rust_public_call, rclass
    version 18.0
    vckss_rust `0'
    return add
end

// Unsupported engines fail before preparation.  Exact resource, rank, and
// deletion failures are typed and return the native lifecycle to idle.
capture quietly vckss y, worker(worker) firm(firm)             ///
    algorithm(exact) backend(rust) engine(compressed) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

capture quietly vckss y control, worker(worker) firm(firm)     ///
    algorithm(exact) backend(rust) exact_limit(2) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "RESOURCE_LIMIT"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

capture quietly vckss y, worker(worker) firm(firm)             ///
    algorithm(exact) backend(rust) memory_gib(1e-6) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',"RESOURCE_LIMIT",         ///
    "ALLOCATION_FAILED")
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

generate double zero_control = 0
capture quietly vckss y zero_control, worker(worker) firm(firm) ///
    algorithm(exact) backend(rust) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "SINGULAR_INFORMATION"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

generate double spike_control = obsid == 1
capture quietly vckss y spike_control, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) backend(rust) nodisplay
assert _rc != 0
assert `"`e(withholding_status)'"' == "NONESTIMABLE_DELETION"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

generate long large_block = firm
capture quietly vckss y, worker(worker) firm(firm)             ///
    deletion(match) deletionid(large_block) algorithm(exact)         ///
    backend(rust) blocksize_limit(1) nodisplay
assert _rc == 198
assert `"`e(withholding_status)'"' == "BLOCK_SIZE_LIMIT"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0

di as result "VCKSS RUST PUBLIC EXACT PASS"
exit 0
