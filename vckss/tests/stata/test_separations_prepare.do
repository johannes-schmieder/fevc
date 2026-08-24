version 18.0
clear all
set more off
set varabbrev off

args package_root output_dir
if `"`package_root'"' == "" | `"`output_dir'"' == "" {
    di as error "usage: do test_separations_prepare.do PACKAGE_ROOT OUTPUT_DIR"
    exit 198
}

set obs 8
generate long persid = ceil(_n/2)
generate long estabid = 100 + mod(persid,2)
generate long time = _n
generate long worker_cz_id = ceil(persid/2)
generate long analysis_estabid = 10 + mod(persid,2)
generate int year = 2020
generate double logrwage = 2 + _n/100
generate double xb = _n/1000
generate double estabfe = 0
quietly isid persid estabid time
char _dta[pers_unit_logrwage] worker_cz_id
char _dta[estab_unit_logrwage] analysis_estabid
char _dta[time_logrwage] year
save `"`output_dir'/input.dta"', replace

do `"`package_root'/benchmarks/separations_wage_prepare.do"' ///
    fixture `"`output_dir'/input.dta"' ///
    0000000000000000000000000000000000000000000000000000000000000000 ///
    full 0 `"`output_dir'"' ///
    0000000000000000000000000000000000000000 ///
    0000000000000000000000000000000000000000

// The small route selects a deterministic dense mover core, not an ID prefix.
preserve
clear
set obs 404
generate long persid = ceil(_n/2)
generate long estabid = cond(mod(_n,2)==1, ///
    cond(persid<=100,1,cond(persid<=200,2,3)), ///
    cond(persid<=100,2,cond(persid<=200,3,4)))
generate long time = 1 + mod(_n-1,2)
generate long worker_id = persid
generate long firm_id = estabid
generate int year = 2020 + time
generate double logrwage = 2 + persid/1000 + time/100
generate double xb = time/1000
generate double estabfe = 0
quietly isid persid estabid time
char _dta[pers_unit_logrwage] worker_id
char _dta[estab_unit_logrwage] firm_id
char _dta[time_logrwage] year
save `"`output_dir'/dense_input.dta"', replace
capture mkdir `"`output_dir'/dense"'
do `"`package_root'/benchmarks/separations_wage_prepare.do"' ///
    dense_fixture `"`output_dir'/dense_input.dta"' ///
    0000000000000000000000000000000000000000000000000000000000000000 ///
    small 100 `"`output_dir'/dense"' ///
    0000000000000000000000000000000000000000 ///
    0000000000000000000000000000000000000000
quietly use `"`output_dir'/dense/prepared.dta"', clear
egen byte __dense_worker = tag(worker)
quietly count if __dense_worker
assert r(N) == 100
quietly import delimited using `"`output_dir'/dense/prepare.csv"', clear
assert sample_selection == "dense_mover_core"
assert workers == 100
restore

capture mkdir `"`output_dir'/exact"'
capture mkdir `"`output_dir'/b1"'
capture mkdir `"`output_dir'/cmg"'
foreach route in exact b1 cmg {
    do `"`package_root'/benchmarks/separations_wage_estimator.do"' ///
        fixture `route' `"`output_dir'/prepared.dta"' ///
        0000000000000000000000000000000000000000000000000000000000000000 ///
        40 8675309 4 60 localfixture `"`output_dir'/`route'"' ///
        0000000000000000000000000000000000000000 ///
        0000000000000000000000000000000000000000000000000000000000000000
}
