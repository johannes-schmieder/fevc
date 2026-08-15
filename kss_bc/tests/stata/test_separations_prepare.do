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
