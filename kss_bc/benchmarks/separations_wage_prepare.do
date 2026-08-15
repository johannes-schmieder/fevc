version 18.0
clear all
set more off
set varabbrev off

args label wage_input wage_input_sha sample_mode max_workers_arg output_dir ///
    source_commit separations_commit

local max_workers = real("`max_workers_arg'")
if !ustrregexm("`label'", "^[A-Za-z0-9._-]+$") | ///
    !inlist("`sample_mode'", "small", "full") | ///
    !ustrregexm("`wage_input_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !ustrregexm("`separations_commit'", "^[0-9a-f]{7,40}$") | ///
    missing(`max_workers') | `max_workers' != floor(`max_workers') | ///
    ("`sample_mode'" == "small" & `max_workers' < 100) | ///
    ("`sample_mode'" == "full" & `max_workers' != 0) {
    di as error "invalid Separations wage preparation arguments"
    exit 198
}
confirm file `"`wage_input'"'

timer clear 80
timer on 80
quietly use `"`wage_input'"', clear
local worker_name : char _dta[pers_unit_logrwage]
local firm_name : char _dta[estab_unit_logrwage]
local time_name : char _dta[time_logrwage]
if "`worker_name'" == "" | "`firm_name'" == "" | "`time_name'" == "" {
    di as error "wage artifact lacks the registered AKM unit characteristics"
    exit 498
}
confirm numeric variable `worker_name' `firm_name' `time_name' ///
    persid estabid time logrwage xb estabfe
quietly keep if estabfe < .
quietly keep if !missing(logrwage,xb,`worker_name',`firm_name',`time_name')
// Match the source project's KSS export contract.  The physical observation
// key is unique, while clustered analysis worker/firm units may legitimately
// repeat within the coarser registered time variable (for example, year).
quietly isid persid estabid time
sort `worker_name' `firm_name' `time_name' persid estabid time

if "`sample_mode'" == "small" {
    tempvar worker_tag worker_rank
    by `worker_name': generate byte `worker_tag' = _n == 1
    generate long `worker_rank' = sum(`worker_tag')
    quietly keep if `worker_rank' <= `max_workers'
    drop `worker_tag' `worker_rank'
}

generate double y_minus_xb = logrwage-xb
sort persid estabid time
generate double observation_key = _n
rename `worker_name' worker
rename `firm_name' firm
rename `time_name' period
keep worker firm period y_minus_xb observation_key
order worker firm period y_minus_xb observation_key
isid observation_key
tempvar analysis_duplicate
quietly duplicates tag worker firm period, generate(`analysis_duplicate')
quietly count if `analysis_duplicate' > 0
local aggregate_duplicate_rows = r(N)
drop `analysis_duplicate'
tempvar semantic_tie semantic_worker_min semantic_worker_max
tempvar semantic_firm_min semantic_firm_max
sort y_minus_xb
quietly by y_minus_xb: egen double `semantic_worker_min' = min(worker)
quietly by y_minus_xb: egen double `semantic_worker_max' = max(worker)
quietly by y_minus_xb: egen double `semantic_firm_min' = min(firm)
quietly by y_minus_xb: egen double `semantic_firm_max' = max(firm)
generate byte `semantic_tie' = ///
    `semantic_worker_min' != `semantic_worker_max' | ///
    `semantic_firm_min' != `semantic_firm_max'
quietly count if `semantic_tie'
local semantic_tie_rows = r(N)
drop `semantic_tie' `semantic_worker_min' `semantic_worker_max' ///
    `semantic_firm_min' `semantic_firm_max'
quietly count
local stored_rows = r(N)
egen byte __worker_tag = tag(worker)
egen byte __firm_tag = tag(firm)
quietly count if __worker_tag
local workers = r(N)
quietly count if __firm_tag
local firms = r(N)
drop __worker_tag __firm_tag
if `stored_rows' == 0 | `workers' < 2 | `firms' < 2 {
    di as error "prepared wage benchmark sample is empty or degenerate"
    exit 498
}

save `"`output_dir'/prepared.dta"', replace
preserve
keep worker firm period y_minus_xb
export delimited using `"`output_dir'/prepared.csv"', replace
restore
timer off 80
quietly timer list 80
local preparation_seconds = r(t80)

clear
set obs 1
generate str64 label = "`label'"
generate str8 sample_mode = "`sample_mode'"
generate str40 source_commit = "`source_commit'"
generate str40 separations_commit = "`separations_commit'"
generate str64 wage_input_sha256 = "`wage_input_sha'"
generate double requested_max_workers = `max_workers'
generate double stored_rows = `stored_rows'
generate double workers = `workers'
generate double firms = `firms'
generate double aggregate_duplicate_rows = `aggregate_duplicate_rows'
generate double semantic_tie_rows = `semantic_tie_rows'
generate double preparation_seconds = `preparation_seconds'
generate str12 stata_version = string(c(stata_version))
generate str12 stata_flavor = c(flavor)
export delimited using `"`output_dir'/prepare.csv"', replace

tempname marker
file open `marker' using `"`output_dir'/prepare.stata.pass"', ///
    write text replace
file write `marker' ///
    "KSS_BC SEPARATIONS PREPARE PASS `label' `source_commit'" _n
file close `marker'
di as result "KSS_BC SEPARATIONS PREPARE PASS: `label'"
exit 0
