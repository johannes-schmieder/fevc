version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

vckss_rust requestcapability, algorithm(exact)                 ///
    deletion(observation) nuisance(fixedoffset) route(auto)          ///
    rngcontract(none) controls(1) frequencyused(1)
assert r(struct_size) == 64
assert r(request_schema) == 1
assert r(supported) == 1
assert r(reason_code) == 0
assert r(profile_code) == 1
assert r(algorithm_code) == 1
assert r(deletion_mode_code) == 2
assert r(nuisance_mode_code) == 2
assert r(solver_route_code) == 0
assert r(rng_contract_code) == 0
assert r(controls_count) == 1
assert r(frequency_use_code) == 1
local signature_hi = r(request_signature_hi)
local signature_lo = r(request_signature_lo)
assert `"`r(profile)'"' == "EXACT_V1"
vckss_rust requestcapability, algorithm(exact)                 ///
    deletion(observation) nuisance(fixedoffset) route(auto)          ///
    rngcontract(none) controls(1) frequencyused(1)
assert r(request_signature_hi) == `signature_hi'
assert r(request_signature_lo) == `signature_lo'

vckss_rust requestcapability, algorithm(exact) deletion(match) ///
    nuisance(joint) route(auto) rngcontract(counter_v1) controls(0)  ///
    frequencyused(0)
assert r(supported) == 0
assert r(reason_code) == 10
assert `"`r(reason)'"' == "EXACT_RNG_NOT_NONE"

vckss_rust requestcapability, algorithm(jla) deletion(match)   ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1)          ///
    controls(0) frequencyused(1)
assert r(supported) == 1
assert r(profile_code) == 2
assert `"`r(profile)'"' == "JLA_COUNTER_V1"
vckss_rust requestcapability, algorithm(jla) deletion(match)   ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1)          ///
    controls(1) frequencyused(1)
assert r(supported) == 0
assert r(reason_code) == 14

set obs 8
generate long worker = cond(_n <= 4, 1, 2)
generate long firm = cond(inlist(_n, 1, 2, 5, 6), 1, 2)
generate long deletion_id = _n
generate double outcome = .
generate double frequency = 1
generate double target_weight = .
generate double control = .
local outcomes 1 2 0 2.5 -1 1.5 2 -2
local targets 1 2 2 1 3 1 2 4
local controls -1.5 -0.5 0.5 1.5 -1 0 1 2
forvalues row = 1/8 {
    local value : word `row' of `outcomes'
    quietly replace outcome = `value' in `row'
    local value : word `row' of `targets'
    quietly replace target_weight = `value' in `row'
    local value : word `row' of `controls'
    quietly replace control = `value' in `row'
}

foreach nuisance in joint fixedoffset {
    vckss_rust prepare worker firm deletion_id outcome frequency  ///
        target_weight control, cleanup generate(keep_`nuisance')        ///
        deletion(match) memorygib(1)
    local handle = r(handle)
    assert r(controls_count) == 1
    assert r(deletion_mode_code) == 1
    assert `"`r(deletion)'"' == "match"
    assert keep_`nuisance' == 1
    vckss_rust solve `handle', algorithm(exact) deletion(match)   ///
        nuisance(`nuisance') exactlimit(100) blocksizelimit(100)
    vckss_rust result `handle'
    matrix result = r(result)
    assert `"`r(selected_algorithm)'"' == "exact"
    assert `"`r(nuisance_mode)'"' == "`nuisance'"
    assert r(rhs_receipt_rows) == 0
    assert r(parameters) == cond("`nuisance'" == "joint", 4, 3)
    assert r(full_parameters) == 4
    assert r(correction_parameters) == cond("`nuisance'" == "joint", 4, 3)
    assert r(exact_diagnostic_flags) == 511
    assert r(working_fit_complete_residual) <= r(full_residual_tolerance)
    assert r(inverse_sqrt_relative_residual) >= 0
    assert r(maker_relative_residual) >= 0
    assert r(control_basis_relative_residual) >= 0
    assert r(control_basis_forward_error) >= 0
    assert r(deletion_rank_gap) > 0
    assert r(firm_zero_sum_residual) >= 0
    assert r(fit_peak_forecast_bytes) > 0
    assert r(correction_peak_forecast_bytes) > 0
    assert r(exact_peak_forecast_bytes) >= r(fit_peak_forecast_bytes)
    assert r(exact_peak_forecast_bytes) >= r(correction_peak_forecast_bytes)
    assert r(actual_accounting_residual) <= 1e-10
    assert abs(r(weighted_rss) - 14.25) <= 1e-10
    if "`nuisance'" == "joint" {
        matrix expected_plugin =                                      ///
            (0.5273437500000006, 0.3845214843749999,                  ///
             0.043945312500000014, 0.9997558593750004)
        matrix expected_correction =                                  ///
            (0.6024169921874988, 2.037963867187495,                   ///
             0.06848144531249988, 2.7773437499999933)
        assert abs(r(max_leverage) - 0.5) <= 1e-10
    }
    else {
        matrix expected_plugin =                                      ///
            (0.5273437499999998, 0.38452148437499933,                 ///
             0.043945312499999944, 0.999755859374999)
        matrix expected_correction =                                  ///
            (0.33398437500000006, 0.35068359375,                     ///
             0.014648437499999896, 0.7139648437500001)
        assert abs(r(max_leverage) - 0.375) <= 1e-10
    }
    forvalues component = 1/4 {
        assert abs(result[1,`component'] - expected_plugin[1,`component']) <= 1e-10
        assert abs(result[2,`component'] - expected_correction[1,`component']) <= 1e-10
        assert abs(result[1,`component'] - result[2,`component'] -       ///
            result[3,`component']) <= 1e-10
        assert result[4,`component'] == 0
    }
    vckss_rust release `handle'
}

vckss_rust prepare worker firm deletion_id outcome frequency      ///
    target_weight, cleanup generate(keep_observation)                   ///
    deletion(observation) memorygib(1)
local handle = r(handle)
assert `"`r(deletion)'"' == "observation"
assert r(deletion_mode_code) == 2
vckss_rust solve `handle', algorithm(exact) deletion(observation) ///
    nuisance(joint) exactlimit(100) blocksizelimit(100)
vckss_rust result `handle'
assert `"`r(selected_algorithm)'"' == "exact"
assert `"`r(deletion_mode)'"' == "observation"
assert r(rhs_receipt_rows) == 0
assert r(exact_diagnostic_flags) == 497
assert r(inverse_sqrt_relative_residual) == 0
assert r(maker_relative_residual) == 0
assert r(actual_accounting_residual) <= 1e-10
vckss_rust release `handle'

di as result "VCKSS RUST EXACT CONTROLS PASS"
exit 0
