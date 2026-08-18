*! CMG-MATA-1 fixed-CZ18 MATLAB input preparation 18aug2026

version 18.0
clear all
set more off
set varabbrev off

args input_dta output_csv receipt_csv experiment input_sha source_commit ///
    bundle_sha processors_arg

local processors = real("`processors_arg'")
if !ustrregexm("`experiment'", "^[A-Za-z0-9._-]+$") |             ///
    !ustrregexm("`input_sha'", "^[0-9a-f]{64}$") |                 ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") |             ///
    !ustrregexm("`bundle_sha'", "^[0-9a-f]{64}$") |                ///
    strtrim(`"`input_dta'"') == "" | strtrim(`"`output_csv'"') == "" | ///
    strtrim(`"`receipt_csv'"') == "" | `processors' != 4 {
    di as error "invalid fixed-CZ18 MATLAB preparation arguments"
    exit 198
}
confirm file `"`input_dta'"'
capture set processors `processors'
if c(MP) != 1 | c(processors) != `processors' {
    di as error "Stata/MP did not honor four preparation processors"
    exit 459
}

timer clear 90
timer clear 91
timer clear 92
timer on 90
timer on 91
quietly use `"`input_dta'"', clear
timer off 91
confirm numeric variable worker firm period y_minus_xb observation_key
keep worker firm period y_minus_xb observation_key
quietly count
local rows = r(N)
if `rows' != 8201888 {
    di as error "fixed CZ18 retained row count changed"
    exit 459
}
capture assert !missing(worker, firm, period, y_minus_xb, observation_key)
if _rc exit 459
capture assert worker == floor(worker) & firm == floor(firm) & ///
    observation_key == floor(observation_key) &               ///
    abs(worker) < 2^53 & abs(firm) < 2^53 &                   ///
    abs(observation_key) < 2^53
if _rc exit 459
quietly isid observation_key

timer on 92
tempvar dense_worker dense_firm cell_tag worker_tag firm_tag
quietly egen long `dense_worker' = group(worker)
quietly egen long `dense_firm' = group(firm)
drop worker firm
rename `dense_worker' worker
rename `dense_firm' firm
quietly egen byte `cell_tag' = tag(worker firm)
quietly egen byte `worker_tag' = tag(worker)
quietly egen byte `firm_tag' = tag(firm)
quietly count if `cell_tag'
local cells = r(N)
quietly count if `worker_tag'
local workers = r(N)
quietly count if `firm_tag'
local firms = r(N)
drop `cell_tag' `worker_tag' `firm_tag'
if `workers' != 117529 | `firms' != 10603 | `cells' != 311730 {
    di as error "fixed CZ18 retained dimensions changed"
    exit 459
}
quietly sort worker period firm y_minus_xb observation_key
capture assert worker >= 1 & worker <= `workers' & ///
    firm >= 1 & firm <= `firms'
if _rc exit 459
timer off 92

timer clear 93
timer on 93
export delimited worker firm period y_minus_xb using `"`output_csv'"', ///
    replace novarnames
timer off 93
timer off 90
quietly timer list 90
local total_seconds = r(t90)
quietly timer list 91
local load_seconds = r(t91)
quietly timer list 92
local prepare_seconds = r(t92)
quietly timer list 93
local export_seconds = r(t93)

clear
quietly set obs 1
generate str32 experiment_id = "`experiment'"
generate str40 source_commit = "`source_commit'"
generate str64 bundle_sha256 = "`bundle_sha'"
generate str64 input_sha256 = "`input_sha'"
generate double rows = `rows'
generate double workers = `workers'
generate double firms = `firms'
generate double coefficient_cells = `cells'
generate double processors = c(processors)
generate double load_seconds = `load_seconds'
generate double prepare_seconds = `prepare_seconds'
generate double export_seconds = `export_seconds'
generate double total_seconds = `total_seconds'
export delimited using `"`receipt_csv'"', replace
di as result "CMG-MATA-1 CZ18 MATLAB PREPARE PASS: `experiment'"
exit 0
