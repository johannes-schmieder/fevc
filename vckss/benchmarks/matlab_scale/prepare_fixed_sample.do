*! KSS-SCALE-1 maintained-MATLAB fixed-sample preparation 17aug2026

version 18.0
clear all
set more off
set varabbrev off

args source_root label input_dta input_sha scale_arg topology output_dir ///
    source_commit bundle_sha requested_slots_arg actual_slots_arg ///
    processors_arg

local scale = real("`scale_arg'")
local requested_slots = real("`requested_slots_arg'")
local actual_slots = real("`actual_slots_arg'")
local requested_processors = real("`processors_arg'")
local valid_case = (`scale' == 1 & "`topology'" == "well") | ///
    (inlist(`scale', 2, 4) & "`topology'" == "well_connected") | ///
    (`scale' == 2 & "`topology'" == "ring")
if !ustrregexm("`label'", "^[A-Za-z0-9._-]+$") | ///
    !ustrregexm("`input_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !ustrregexm("`bundle_sha'", "^[0-9a-f]{64}$") | ///
    !`valid_case' | `requested_slots' != 14 | `actual_slots' != 14 | ///
    `requested_processors' != 4 {
    di as error "invalid MATLAB scale fixed-sample preparation arguments"
    exit 198
}
confirm file `"`input_dta'"'
confirm file `"`source_root'/vckss/benchmarks/kss_scale_fixtures.do"'
confirm file `"`source_root'/vckss/benchmarks/kss_scale_fixtures.mata"'

capture set processors `requested_processors'
if c(MP) != 1 | c(processors) != `requested_processors' {
    di as error "Stata/MP did not honor the four-processor preparation request"
    exit 459
}

quietly do `"`source_root'/vckss/benchmarks/kss_scale_fixtures.mata"'
quietly do `"`source_root'/vckss/benchmarks/kss_scale_fixtures.do"'

timer clear 90
timer clear 91
timer clear 92
timer clear 93
timer clear 94
timer on 90

timer on 91
quietly use `"`input_dta'"', clear
timer off 91
confirm numeric variable worker firm period y_minus_xb observation_key
quietly count
local input_rows = r(N)
if `input_rows' == 0 {
    di as error "retained CZ18 input is empty"
    exit 2000
}
capture assert !missing(worker, firm, period, y_minus_xb, observation_key)
if _rc {
    di as error "retained CZ18 input contains missing required values"
    exit 459
}
capture assert worker == floor(worker) & firm == floor(firm) & ///
    observation_key == floor(observation_key) & ///
    abs(worker) < 2^53 & abs(firm) < 2^53 & ///
    abs(observation_key) < 2^53
if _rc {
    di as error "retained CZ18 identifiers must be finite integers"
    exit 459
}
quietly isid observation_key
// This benchmark consumes literal physical rows. Discard unrelated source
// columns before replication so no accidental weight or wide-data payload can
// enter the fixed four-column MATLAB contract.
keep worker firm period y_minus_xb observation_key

tempvar input_worker_tag input_firm_tag input_match_tag
quietly egen byte `input_worker_tag' = tag(worker)
quietly egen byte `input_firm_tag' = tag(firm)
quietly egen byte `input_match_tag' = tag(worker firm)
quietly count if `input_worker_tag'
local input_workers = r(N)
quietly count if `input_firm_tag'
local input_firms = r(N)
quietly count if `input_match_tag'
local input_matches = r(N)
drop `input_worker_tag' `input_firm_tag' `input_match_tag'
if `input_workers' < 2 | `input_firms' < 2 | `input_matches' < 2 {
    di as error "retained CZ18 input is graph-degenerate"
    exit 459
}

timer on 92
tempvar dense_worker dense_firm deletion_id frequency
quietly egen long `dense_worker' = group(worker)
quietly egen long `dense_firm' = group(firm)
drop worker firm
rename `dense_worker' worker
rename `dense_firm' firm
quietly egen long `deletion_id' = group(worker firm)
quietly generate byte `frequency' = 1
quietly sort worker firm period y_minus_xb observation_key
timer off 92

timer on 93
local connector_rows = 0
local copy_cut_conductance = .
local normalized_lambda2 = .
local normalized_lambda_max = .
local normalized_condition_proxy = .
if `scale' > 1 {
    vckss_scale_fixture, design(`topology') copies(`scale') ///
        worker(worker) firm(firm) deletionid(`deletion_id') ///
        outcome(y_minus_xb) frequency(`frequency') ///
        copyvar(scale_copy) connectorvar(scale_connector) ///
        rowkey(scale_row)
    local connector_rows = r(connector_rows)
    local copy_cut_conductance = r(copy_cut_conductance)
    local normalized_lambda2 = r(normalized_lambda2)
    local normalized_lambda_max = r(normalized_lambda_max)
    local normalized_condition_proxy = r(normalized_condition_proxy)

    // Connector workers are disjoint from copied workers. Give their two
    // incident observations a deterministic finite order for MATLAB.
    quietly sort worker firm scale_row
    quietly by worker: replace period = _n if scale_connector
    quietly replace observation_key = scale_row if scale_connector
}
else {
    quietly generate byte scale_copy = 1
    quietly generate byte scale_connector = 0
    quietly generate double scale_row = _n
}

tempname final_diagnostics
local diagnostic_status
local diagnostic_message
mata: vckss_scale__stata_diagnose( ///
    "worker", "firm", "`deletion_id'", "`frequency'", ///
    "`final_diagnostics'", "diagnostic_status", "diagnostic_message")
matrix colnames `final_diagnostics' = rows physical workers firms cells ///
    deletion_units components bridge_units minimum_weighted_degree ///
    maximum_weighted_degree
if "`diagnostic_status'" != "CONVERGED" {
    di as error "fixed-sample diagnostic failed: `diagnostic_status': `diagnostic_message'"
    exit 459
}
if el(`final_diagnostics', 1, 7) != 1 | ///
    el(`final_diagnostics', 1, 8) != 0 {
    di as error "fixed sample is disconnected or not match-deletion safe"
    exit 459
}
if el(`final_diagnostics', 1, 5) != ///
    el(`final_diagnostics', 1, 6) {
    di as error "fixed MATLAB fixture must have one match ID per coefficient cell"
    exit 459
}
timer off 93

quietly count
local output_rows = r(N)
local output_workers = el(`final_diagnostics', 1, 3)
local output_firms = el(`final_diagnostics', 1, 4)
local output_matches = el(`final_diagnostics', 1, 5)
local minimum_weighted_degree = el(`final_diagnostics', 1, 9)
local maximum_weighted_degree = el(`final_diagnostics', 1, 10)
capture assert !missing(period, y_minus_xb)
if _rc {
    di as error "fixture construction left a missing period or outcome"
    exit 459
}

// The final sort has deterministic tie-breakers. Exact duplicate exported
// rows are interchangeable, but their underlying source order is still
// fixed by scale copy, physical observation key, and fixture row key.
quietly sort worker period firm scale_copy observation_key scale_row

timer on 94
preserve
keep worker firm period y_minus_xb
rename y_minus_xb outcome
format worker firm %12.0f
format period outcome %24.17g
export delimited worker firm period outcome using ///
    `"`output_dir'/input.csv"', replace nolabel datafmt
restore

preserve
keep worker firm
quietly duplicates drop
quietly sort worker firm
quietly count
if r(N) != `output_matches' {
    di as error "canonical retained-key row count changed"
    exit 459
}
format worker firm %12.0f
export delimited worker firm using ///
    `"`output_dir'/retained_keys.canonical.txt"', ///
    replace novarnames nolabel datafmt
restore
timer off 94
timer off 90

quietly timer list 91
local load_seconds = r(t91)
quietly timer list 92
local normalization_seconds = r(t92)
quietly timer list 93
local fixture_seconds = r(t93)
quietly timer list 94
local export_seconds = r(t94)
quietly timer list 90
local total_seconds = r(t90)

clear
set obs 1
generate str40 schema = "kss_matlab_scale_prepare_summary_v1"
generate str16 status = "PASS"
generate str64 label = "`label'"
generate double scale = `scale'
generate str16 topology = "`topology'"
generate str20 sample_mode = "fixed_retained"
generate str40 source_commit = "`source_commit'"
generate str64 bundle_sha256 = "`bundle_sha'"
generate str64 source_input_sha256 = "`input_sha'"
generate double input_rows = `input_rows'
generate double input_workers = `input_workers'
generate double input_firms = `input_firms'
generate double input_matches = `input_matches'
generate double rows = `output_rows'
generate double workers = `output_workers'
generate double firms = `output_firms'
generate double matches = `output_matches'
generate double connector_rows = `connector_rows'
generate double copy_cut_conductance = `copy_cut_conductance'
generate double normalized_lambda2 = `normalized_lambda2'
generate double normalized_lambda_max = `normalized_lambda_max'
generate double normalized_condition_proxy = ///
    `normalized_condition_proxy'
generate double minimum_weighted_degree = `minimum_weighted_degree'
generate double maximum_weighted_degree = `maximum_weighted_degree'
generate double load_seconds = `load_seconds'
generate double normalization_seconds = `normalization_seconds'
generate double fixture_seconds = `fixture_seconds'
generate double export_seconds = `export_seconds'
generate double total_seconds = `total_seconds'
generate double requested_slots = `requested_slots'
generate double actual_slots = `actual_slots'
generate double requested_processors = `requested_processors'
generate double actual_processors = c(processors)
generate byte stata_mp = c(MP)
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
generate str40 retained_key_contract = "SORTED_INTEGER_CSV_UTF8_LF_V1"
export delimited using `"`output_dir'/prepare.csv"', replace

tempname marker
file open `marker' using `"`output_dir'/prepare.stata.pass"', ///
    write text replace
file write `marker' ///
    "KSS_MATLAB_SCALE_PREPARE_PASS `label' `source_commit' `bundle_sha' `input_sha'" _n
file close `marker'
di as result "KSS MATLAB SCALE PREPARE PASS: `label'"
exit 0
