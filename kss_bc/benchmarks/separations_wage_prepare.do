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
    logrwage xb estabfe
quietly keep if estabfe < .
quietly keep if !missing(logrwage,xb,`worker_name',`firm_name',`time_name')
quietly isid `worker_name' `firm_name' `time_name'

if "`sample_mode'" == "small" {
    sort `worker_name' `firm_name' `time_name'
    tempvar worker_tag worker_rank
    by `worker_name': generate byte `worker_tag' = _n == 1
    generate long `worker_rank' = sum(`worker_tag')
    quietly keep if `worker_rank' <= `max_workers'
    drop `worker_tag' `worker_rank'
}

generate double y_minus_xb = logrwage-xb
rename `worker_name' worker
rename `firm_name' firm
rename `time_name' period
keep worker firm period y_minus_xb
order worker firm period y_minus_xb
sort worker firm period
quietly isid worker firm period
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
export delimited using `"`output_dir'/prepared.csv"', replace
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
