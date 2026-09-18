version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

fevc_rust clear
clear
set obs 96
generate long cell = floor((_n - 1) / 2)
generate double worker = floor(cell / 4) + 1
generate double firm = mod(cell, 4) + 1
generate double replicate = mod(_n - 1, 2)
generate double deletion_id = cell + 1
generate double outcome = 0.7 * worker - 0.45 * firm + 0.3 * replicate + ///
    mod(7 * (_n - 1), 5) / 11
generate double frequency = mod(_n - 1, 3) + 1
generate double target_weight = 0.5 + mod(5 * (_n - 1), 7) / 3
generate double control = (worker - 0.4 * firm) * (replicate + 1) + ///
    mod(3 * (_n - 1), 7) / 17

fevc_rust requestcapability, algorithm(jla) deletion(match)   ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1)          ///
    controls(0) frequencyused(1) engine(generic) batchmode(auto)     ///
    stayers(movers) targetweightmode(explicit) deletionsource(matchid)
assert r(supported) == 0
assert r(reason_code) == 28

local probes = 7
foreach deletion in match observation {
    local deletion_source matchid
    local deletion_code = 1
    if "`deletion'" == "observation" {
        local deletion_source observation
        local deletion_code = 2
    }
    foreach nuisance in joint fixedoffset {
        local nuisance_code = cond("`nuisance'" == "joint", 1, 2)
        forvalues q = 0/1 {
            local controls
            if `q' == 1 local controls control
            quietly fevc_rust requestcapability, algorithm(jla) ///
                deletion(`deletion') nuisance(`nuisance') route(diagonal) ///
                rngcontract(counter_v1) controls(`q') frequencyused(1) ///
                engine(generic) batchmode(explicit) stayers(movers)    ///
                targetweightmode(explicit) deletionsource(`deletion_source') ///
                physicallimit(50000000)
            assert r(supported) == 1
            assert r(request_schema) == 2
            assert r(profile_code) == 3
            assert r(engine_code) == 2
            assert r(batch_mode_code) == 1
            assert r(stayers_mode_code) == 1
            assert r(target_weight_mode_code) == 1
            local capability_schema = r(request_schema)
            local capability_profile = r(profile_code)
            local signature_hi = r(request_signature_hi)
            local signature_lo = r(request_signature_lo)

            quietly fevc_rust prepare worker firm deletion_id outcome ///
                frequency target_weight `controls', cleanup memorygib(1)      ///
                deletion(`deletion')
            local handle = r(handle)
            assert r(controls_count) == `q'
            assert r(deletion_mode_code) == `deletion_code'

            quietly fevc_rust solve `handle', algorithm(jla)          ///
                deletion(`deletion') nuisance(`nuisance') route(diagonal)    ///
                seed(81227) probes(`probes') leveragebatch(2) targetbatch(3) ///
                tolerance(1e-12) engine(generic) batchmode(explicit)         ///
                stayers(movers) targetweightmode(explicit)                   ///
                deletionsource(`deletion_source') physicallimit(50000000)    ///
                capabilityschema(`capability_schema')                        ///
                capabilityprofile(`capability_profile') frequencyused(1)     ///
                signaturehi(`signature_hi') signaturelo(`signature_lo')

            quietly fevc_rust result `handle'
            assert r(rhs_receipt_schema) == 2
            assert r(requested_engine_code) == 2
            assert r(selected_engine_code) == 2
            assert r(capability_schema) == `capability_schema'
            assert r(capability_profile) == `capability_profile'
            assert r(request_signature_hi) == `signature_hi'
            assert r(request_signature_lo) == `signature_lo'
            assert r(generic_controls_count) == `q'
            assert r(control_projection_rhs_count) == `q'
            assert r(deletion_mode_code) == `deletion_code'
            assert r(nuisance_mode_code) == `nuisance_code'
            assert r(full_fit_complete_residual) ==                    ///
                r(full_joint_fit_complete_residual)
            assert r(max_reciprocal_residual) == r(generic_maker_relative_residual)
            assert r(inverse_relative_residual) == 0
            assert r(actual_accounting_residual) <= 1e-10
            local distinct_working = (`q' == 1 & "`nuisance'" == "fixedoffset")
            local expected_rhs = `q' + 1 + `distinct_working' + 3 * `probes'
            assert r(rhs_receipt_rows) == `expected_rhs'
            assert r(caller_result_copy_bytes) == 0
            assert r(rhs_v2_caller_copy_bytes) == `expected_rhs' * 216
            assert r(result_forecast_bytes) == r(generic_result_forecast_bytes)
            assert r(solve_peak_forecast_bytes) == r(generic_peak_forecast_bytes)
            assert r(generic_result_forecast_bytes) >= r(rhs_v2_caller_copy_bytes)
            tempname rhs estimates
            matrix `rhs' = r(rhs_receipts)
            matrix `estimates' = r(result)
            assert colsof(`rhs') == 15
            assert rowsof(`rhs') == `expected_rhs'
            assert `rhs'[`q' + 1, 1] == 1
            assert `rhs'[`q' + 1, 2] == -1
            if `q' == 1 {
                assert `rhs'[1, 1] == 5
                assert `rhs'[1, 2] == 0
                assert `rhs'[1, 3] == 0
            }
            if `distinct_working' {
                assert `rhs'[`q' + 2, 1] == 4
                assert `rhs'[`q' + 2, 2] == -1
            }
            forvalues row = 1/`expected_rhs' {
                assert inlist(`rhs'[`row', 9], 1, 2)
                assert `rhs'[`row', 8] == (`rhs'[`row', 9] == 1)
                assert `rhs'[`row', 7] <= `rhs'[`row', 13]
            }
            mata: st_numscalar("__vckss_test_rhs_max", max(st_matrix("`rhs'")[.,7]))
            assert r(max_complete_residual) == scalar(__vckss_test_rhs_max)
            scalar drop __vckss_test_rhs_max
            forvalues column = 1/4 {
                assert abs(`estimates'[1,`column'] - `estimates'[2,`column'] - ///
                    `estimates'[3,`column']) <= 1e-10
            }
            quietly fevc_rust release `handle'
            quietly fevc_rust release `handle'
            quietly fevc_rust snapshot
            assert r(state) == 0
        }
    }
}

quietly fevc_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1) controls(0)    ///
    frequencyused(1) engine(generic) batchmode(explicit) stayers(movers)   ///
    targetweightmode(explicit) deletionsource(matchid)
local capability_schema = r(request_schema)
local capability_profile = r(profile_code)
local signature_hi = r(request_signature_hi)
local bad_signature_lo = mod(r(request_signature_lo) + 1, 4294967296)
quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local handle = r(handle)
capture noisily fevc_rust solve `handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) seed(81227) probes(5) leveragebatch(2)  ///
    targetbatch(2) engine(generic) batchmode(explicit) stayers(movers)     ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    capabilityschema(`capability_schema') capabilityprofile(`capability_profile') ///
    frequencyused(1) signaturehi(`signature_hi') signaturelo(`bad_signature_lo')
assert _rc == 198
quietly fevc_rust lasterror
assert r(native_error_code) == 11
quietly fevc_rust snapshot
assert r(state) == 4
capture noisily fevc_rust result `handle'
assert _rc == 198
quietly fevc_rust release `handle'
quietly fevc_rust release `handle'
quietly fevc_rust snapshot
assert r(state) == 0

// Corrupt each class of the private V2 result surface after a successful
// native result. Every local failure must release the retained generation and
// certify an idle session before returning.
quietly fevc_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1) controls(0)    ///
    frequencyused(1) engine(generic) batchmode(explicit) stayers(movers)   ///
    targetweightmode(explicit) deletionsource(matchid)
local capability_schema = r(request_schema)
local capability_profile = r(profile_code)
local signature_hi = r(request_signature_hi)
local signature_lo = r(request_signature_lo)

capture program drop fevc__rust_plugin_call
program define fevc__rust_plugin_call
    version 18.0
    gettoken plugin 0 : 0
    local native_arguments `"`0'"'
    local parsed = strtrim(subinstr(`"`0'"', ",", "", 1))
    gettoken subcommand rest : parsed
    plugin call `plugin' `native_arguments'
    local mode "$VCKSS_RUST_PRIVATE_TEST_MODE"
    if "`subcommand'" == "release" {
        global VCKSS_RUST_PRIVATE_RELEASES = $VCKSS_RUST_PRIVATE_RELEASES + 1
    }
    if "`subcommand'" == "snapshot" {
        global VCKSS_RUST_PRIVATE_SNAPSHOTS = $VCKSS_RUST_PRIVATE_SNAPSHOTS + 1
    }
    if "`subcommand'" == "result" {
        if "`mode'" == "bad_count" scalar __vckss_rust_rhs_rows = -1
        if "`mode'" == "bad_schema" scalar __vckss_rust_rhs_schema = 9
        if "`mode'" == "bad_schema0_rows" scalar __vckss_rust_rhs_rows = 1
        if "`mode'" == "bad_v1_copy" scalar __vckss_rust_rhs_copy = ///
            scalar(__vckss_rust_rhs_copy) + 1
        if "`mode'" == "bad_v2_copy" scalar __vckss_rust_rhs_v2_copy = ///
            scalar(__vckss_rust_rhs_v2_copy) + 1
        if "`mode'" == "bad_max_reduced" scalar __vckss_rust_max_reduced = ///
            scalar(__vckss_rust_max_reduced) + 1
        if "`mode'" == "bad_max_complete" scalar __vckss_rust_max_complete = ///
            scalar(__vckss_rust_max_complete) + 1
        if "`mode'" == "bad_zero_counts" scalar __vckss_rust_full_zero = 1
        if "`mode'" == "bad_cr_rcond" scalar __vckss_rust_cr_rcond = ///
            scalar(__vckss_rust_cr_rcond) + .25
        if "`mode'" == "bad_cr_small" scalar __vckss_rust_cr_small_lo = ///
            scalar(__vckss_rust_cr_small_lo) + .25
        if "`mode'" == "bad_cr_large" scalar __vckss_rust_cr_large_hi = ///
            scalar(__vckss_rust_cr_large_hi) + .25
        if "`mode'" == "bad_cr_projection" scalar __vckss_rust_cr_proj_err = -1
        if "`mode'" == "bad_cr_normalization" scalar __vckss_rust_cr_norm_err = .25
        if "`mode'" == "bad_cr_fe_lower" scalar __vckss_rust_cr_fe_lo = -1
        if "`mode'" == "bad_cr_max_projection" scalar __vckss_rust_cr_max_proj = ///
            scalar(__vckss_rust_cr_max_proj) + 1
        if "`mode'" == "bad_cr_effective_tolerance" scalar __vckss_rust_cr_tol = 2
        if "`mode'" == "bad_cr_pcg_tolerance" scalar __vckss_rust_cr_pcg_tol = 0
        if "`mode'" == "bad_cr_projection_gate" scalar __vckss_rust_cr_resid_gate = 0
        if "`mode'" == "bad_control_projection_complete" {
            scalar __vckss_rust_max_complete = scalar(__vckss_rust_cr_resid_gate)
        }
    }
    if "`subcommand'" == "rhsresult" {
        gettoken generation matrix_name : rest
        if "`mode'" == "bad_phase" matrix `matrix_name'[2,1] = 3
        if "`mode'" == "bad_probe" matrix `matrix_name'[2,2] = 99
        if "`mode'" == "bad_side" matrix `matrix_name'[2,3] = 2
        if "`mode'" == "bad_route" matrix `matrix_name'[2,4] = 1
        if "`mode'" == "bad_iterations" matrix `matrix_name'[2,5] = -1
        if "`mode'" == "bad_complete_negative" matrix `matrix_name'[2,7] = -1
        if "`mode'" == "bad_zero_flag" matrix `matrix_name'[2,8] = 2
        if "`mode'" == "bad_status" matrix `matrix_name'[2,9] = 7
        if "`mode'" == "bad_replacements" matrix `matrix_name'[2,10] = -1
        if "`mode'" == "bad_operator" matrix `matrix_name'[2,11] = .5
        if "`mode'" == "bad_preconditioner" matrix `matrix_name'[2,12] = -1
        if "`mode'" == "bad_negative_tolerance" matrix `matrix_name'[2,13] = -1
        if "`mode'" == "bad_space" matrix `matrix_name'[2,14] = 2
        if "`mode'" == "bad_dimension" matrix `matrix_name'[2,15] = ///
            `matrix_name'[2,15] + 1
        if "`mode'" == "bad_zero_counts" {
            matrix `matrix_name'[1,8] = 1
            matrix `matrix_name'[1,9] = 1
        }
        if "`mode'" == "bad_fractional_count" matrix `matrix_name'[2,10] = .5
        if "`mode'" == "bad_missing_residual" matrix `matrix_name'[2,6] = .
        if "`mode'" == "bad_tolerance" matrix `matrix_name'[2,7] = ///
            `matrix_name'[2,13] + 1
        if "`mode'" == "bad_control_projection_complete" {
            matrix `matrix_name'[1,7] = scalar(__vckss_rust_cr_resid_gate)
        }
    }
end

foreach corruption in bad_count bad_schema bad_phase bad_probe bad_side      ///
    bad_route bad_iterations bad_missing_residual bad_complete_negative      ///
    bad_zero_flag bad_status bad_replacements bad_fractional_count           ///
    bad_operator bad_preconditioner bad_negative_tolerance bad_space         ///
    bad_dimension bad_zero_counts bad_tolerance bad_max_reduced              ///
    bad_max_complete bad_v1_copy bad_v2_copy {
    global VCKSS_RUST_PRIVATE_TEST_MODE `corruption'
    global VCKSS_RUST_PRIVATE_RELEASES 0
    global VCKSS_RUST_PRIVATE_SNAPSHOTS 0
    quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
        target_weight, cleanup memorygib(1) deletion(match)
    local handle = r(handle)
    quietly fevc_rust solve `handle', algorithm(jla) deletion(match) ///
        nuisance(joint) route(diagonal) seed(81227) probes(5)              ///
        leveragebatch(2) targetbatch(2) engine(generic) batchmode(explicit) ///
        stayers(movers) targetweightmode(explicit) deletionsource(matchid)  ///
        capabilityschema(`capability_schema')                              ///
        capabilityprofile(`capability_profile') frequencyused(1)           ///
        signaturehi(`signature_hi') signaturelo(`signature_lo')
    capture noisily fevc_rust result `handle'
    assert _rc == 498
    assert $VCKSS_RUST_PRIVATE_RELEASES >= 1
    assert $VCKSS_RUST_PRIVATE_SNAPSHOTS >= 1
    global VCKSS_RUST_PRIVATE_TEST_MODE
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

// Every zero-control rank field has a frozen sentinel. Corruption after a
// successful result must be rejected locally and release the generation.
foreach corruption in bad_cr_rcond bad_cr_small bad_cr_large              ///
    bad_cr_projection bad_cr_normalization bad_cr_fe_lower                 ///
    bad_cr_max_projection bad_cr_pcg_tolerance bad_cr_projection_gate {
    global VCKSS_RUST_PRIVATE_TEST_MODE `corruption'
    global VCKSS_RUST_PRIVATE_RELEASES 0
    global VCKSS_RUST_PRIVATE_SNAPSHOTS 0
    quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
        target_weight, cleanup memorygib(1) deletion(match)
    local handle = r(handle)
    quietly fevc_rust solve `handle', algorithm(jla) deletion(match) ///
        nuisance(joint) route(diagonal) seed(81227) probes(5)              ///
        leveragebatch(2) targetbatch(2) engine(generic) batchmode(explicit) ///
        stayers(movers) targetweightmode(explicit) deletionsource(matchid)  ///
        capabilityschema(`capability_schema')                              ///
        capabilityprofile(`capability_profile') frequencyused(1)           ///
        signaturehi(`signature_hi') signaturelo(`signature_lo')
    capture noisily fevc_rust result `handle'
    assert _rc == 498
    assert $VCKSS_RUST_PRIVATE_RELEASES >= 1
    assert $VCKSS_RUST_PRIVATE_SNAPSHOTS >= 1
    global VCKSS_RUST_PRIVATE_TEST_MODE
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

// Positive-control certificates must reconcile every registered bound and
// the exact maximum complete residual of the logical control-projection rows.
quietly fevc_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1) controls(1)    ///
    frequencyused(1) engine(generic) batchmode(explicit) stayers(movers)   ///
    targetweightmode(explicit) deletionsource(matchid)
local q1_capability_schema = r(request_schema)
local q1_capability_profile = r(profile_code)
local q1_signature_hi = r(request_signature_hi)
local q1_signature_lo = r(request_signature_lo)
foreach corruption in bad_cr_rcond bad_cr_small bad_cr_large              ///
    bad_cr_projection bad_cr_normalization bad_cr_fe_lower                 ///
    bad_cr_max_projection bad_cr_effective_tolerance                       ///
    bad_cr_pcg_tolerance bad_cr_projection_gate                            ///
    bad_control_projection_complete {
    global VCKSS_RUST_PRIVATE_TEST_MODE `corruption'
    global VCKSS_RUST_PRIVATE_RELEASES 0
    global VCKSS_RUST_PRIVATE_SNAPSHOTS 0
    quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
        target_weight control, cleanup memorygib(1) deletion(match)
    local handle = r(handle)
    quietly fevc_rust solve `handle', algorithm(jla) deletion(match) ///
        nuisance(joint) route(diagonal) seed(81227) probes(5)              ///
        leveragebatch(2) targetbatch(2) engine(generic) batchmode(explicit) ///
        stayers(movers) targetweightmode(explicit) deletionsource(matchid)  ///
        capabilityschema(`q1_capability_schema')                           ///
        capabilityprofile(`q1_capability_profile') frequencyused(1)        ///
        signaturehi(`q1_signature_hi') signaturelo(`q1_signature_lo')
    capture noisily fevc_rust result `handle'
    assert _rc == 498
    assert $VCKSS_RUST_PRIVATE_RELEASES >= 1
    assert $VCKSS_RUST_PRIVATE_SNAPSHOTS >= 1
    global VCKSS_RUST_PRIVATE_TEST_MODE
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

// V1 compressed receipts use 112 bytes per row and must never echo V2 bytes.
foreach corruption in bad_v1_copy bad_v2_copy {
    global VCKSS_RUST_PRIVATE_TEST_MODE `corruption'
    global VCKSS_RUST_PRIVATE_RELEASES 0
    global VCKSS_RUST_PRIVATE_SNAPSHOTS 0
    quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
        target_weight, cleanup memorygib(1) deletion(match)
    local handle = r(handle)
    quietly fevc_rust solve `handle', algorithm(jla) deletion(match) ///
        nuisance(joint) route(diagonal) seed(81227) probes(5)             ///
        leveragebatch(2) targetbatch(2)
    capture noisily fevc_rust result `handle'
    assert _rc == 498
    assert $VCKSS_RUST_PRIVATE_RELEASES >= 1
    assert $VCKSS_RUST_PRIVATE_SNAPSHOTS >= 1
    global VCKSS_RUST_PRIVATE_TEST_MODE
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

// Exact receipts expose schema 0 and therefore no RHS rows or copy bytes.
foreach corruption in bad_schema0_rows bad_v1_copy bad_v2_copy {
    global VCKSS_RUST_PRIVATE_TEST_MODE `corruption'
    global VCKSS_RUST_PRIVATE_RELEASES 0
    global VCKSS_RUST_PRIVATE_SNAPSHOTS 0
    quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
        target_weight, cleanup memorygib(1) deletion(match)
    local handle = r(handle)
    quietly fevc_rust solve `handle', algorithm(exact) deletion(match) ///
        nuisance(joint) exactlimit(500) blocksizelimit(5000)
    capture noisily fevc_rust result `handle'
    assert _rc == 498
    assert $VCKSS_RUST_PRIVATE_RELEASES >= 1
    assert $VCKSS_RUST_PRIVATE_SNAPSHOTS >= 1
    global VCKSS_RUST_PRIVATE_TEST_MODE
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}

// The maximum-control DCT fixture has FE-projection PCG zero RHS rows whose
// separately certified complete W+F residuals are positive roundoff below
// tolerance. Those complete residuals must not be confused with reduced-space
// zero-RHS accounting.
fevc_rust clear
clear
set obs 544
generate long worker = floor((_n - 1) / (4 * 34)) + 1
generate long firm = mod(floor((_n - 1) / 34), 4) + 1
generate long rep = mod(_n - 1, 34)
generate long deletion_id = _n
generate double outcome = .7 * worker - .4 * firm + sin((rep + .5) / 7)
generate double frequency = 1
generate double target_weight = 1 + mod(rep, 3) / 5
local q32_controls
forvalues control = 1/32 {
    generate double q`control' = cos(_pi * (rep + .5) * `control' / 34)
    local q32_controls `q32_controls' q`control'
}

foreach nuisance in joint fixedoffset {
    quietly fevc_rust requestcapability, algorithm(jla)             ///
        deletion(observation) nuisance(`nuisance') route(diagonal)         ///
        rngcontract(counter_v1) controls(32) frequencyused(1)              ///
        engine(generic) batchmode(explicit) stayers(movers)                ///
        targetweightmode(explicit) deletionsource(observation)
    local capability_schema = r(request_schema)
    local capability_profile = r(profile_code)
    local signature_hi = r(request_signature_hi)
    local signature_lo = r(request_signature_lo)
    assert r(supported) == 1

    quietly fevc_rust prepare worker firm deletion_id outcome       ///
        frequency target_weight `q32_controls', cleanup memorygib(1)       ///
        deletion(observation)
    local handle = r(handle)
    assert r(controls_count) == 32
    quietly fevc_rust solve `handle', algorithm(jla)                ///
        deletion(observation) nuisance(`nuisance') route(diagonal)         ///
        seed(81227) probes(3) leveragebatch(2) targetbatch(2)              ///
        tolerance(1e-11) engine(generic) batchmode(explicit)               ///
        stayers(movers) targetweightmode(explicit)                         ///
        deletionsource(observation) capabilityschema(`capability_schema')  ///
        capabilityprofile(`capability_profile') frequencyused(1)           ///
        signaturehi(`signature_hi') signaturelo(`signature_lo')
    quietly fevc_rust result `handle'

    local distinct_working = ("`nuisance'" == "fixedoffset")
    local expected_rhs = 32 + 1 + `distinct_working' + 3 + 6
    assert r(rhs_receipt_schema) == 2
    assert r(generic_controls_count) == 32
    assert r(control_projection_rhs_count) == 32
    assert r(rhs_receipt_rows) == `expected_rhs'
    assert r(caller_result_copy_bytes) == 0
    assert r(rhs_v2_caller_copy_bytes) == 216 * `expected_rhs'
    tempname rhs
    matrix `rhs' = r(rhs_receipts)
    assert rowsof(`rhs') == `expected_rhs'
    assert colsof(`rhs') == 15

    local projection_zero = 0
    local positive_complete = 0
    forvalues row = 1/32 {
        assert `rhs'[`row',1] == 5
        assert `rhs'[`row',2] == `row' - 1
        assert `rhs'[`row',3] == 0
        assert `rhs'[`row',14] == 1
        assert `rhs'[`row',15] == 4
        assert `rhs'[`row',7] <= `rhs'[`row',13]
        if `rhs'[`row',8] == 1 {
            local projection_zero = `projection_zero' + 1
            assert `rhs'[`row',9] == 1
            assert `rhs'[`row',5] == 0
            assert `rhs'[`row',6] == 0
            assert `rhs'[`row',10] == 0
            assert `rhs'[`row',11] == 0
            assert `rhs'[`row',12] == 0
            if `rhs'[`row',7] > 0 {
                local positive_complete = `positive_complete' + 1
            }
        }
    }
    assert `projection_zero' > 0
    assert `positive_complete' > 0

    local row = 33
    assert `rhs'[`row',1] == 1 & `rhs'[`row',2] == -1
    assert `rhs'[`row',14] == 2 & `rhs'[`row',15] == 36
    local row = `row' + 1
    if `distinct_working' {
        assert `rhs'[`row',1] == 4 & `rhs'[`row',2] == -1
        assert `rhs'[`row',14] == 1 & `rhs'[`row',15] == 4
        local row = `row' + 1
    }
    forvalues probe = 0/2 {
        assert `rhs'[`row',1] == 2 & `rhs'[`row',2] == `probe'
        assert `rhs'[`row',3] == 0
        assert `rhs'[`row',14] == 1 & `rhs'[`row',15] == 4
        local row = `row' + 1
    }
    forvalues target = 0/5 {
        assert `rhs'[`row',1] == 3
        assert `rhs'[`row',2] == floor(`target' / 2)
        assert `rhs'[`row',3] == 1 + mod(`target', 2)
        assert `rhs'[`row',14] == cond("`nuisance'" == "joint", 2, 1)
        assert `rhs'[`row',15] == cond("`nuisance'" == "joint", 36, 4)
        local row = `row' + 1
    }
    assert `row' == `expected_rhs' + 1

    quietly fevc_rust release `handle'
    quietly fevc_rust release `handle'
    quietly fevc_rust snapshot
    assert r(state) == 0 & r(handle) == 0
}
macro drop VCKSS_RUST_PRIVATE_TEST_MODE
macro drop VCKSS_RUST_PRIVATE_RELEASES
macro drop VCKSS_RUST_PRIVATE_SNAPSHOTS

di as result "FEVC RUST GENERIC JLA PASS"
exit 0
