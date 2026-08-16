version 18.0
clear all
set more off
set varabbrev off

args package_root output_dir
if `"`package_root'"' == "" | `"`output_dir'"' == "" {
    di as error "usage: test_separations_matlab_sample.do package_root output_dir"
    exit 198
}

clear
input double(worker firm period y_minus_xb observation_key)
1 1 1 1.01 1
1 2 2 1.02 2
2 2 1 1.21 3
2 3 2 1.22 4
3 3 1 1.31 5
3 4 2 1.32 6
4 4 1 1.41 7
4 1 2 1.42 8
5 4 1 1.51 9
5 5 2 1.52 10
end
isid observation_key
save `"`output_dir'/input.dta"', replace

tempname detail
file open `detail' using `"`output_dir'/detail.tsv"', write text replace
file write `detail' "1.01" _tab "1" _tab "1" _tab ".1" _n
file write `detail' "1.02" _tab "1" _tab "2" _tab ".1" _n
file write `detail' "1.21" _tab "2" _tab "2" _tab ".1" _n
file write `detail' "1.22" _tab "2" _tab "3" _tab ".1" _n
file write `detail' "1.31" _tab "3" _tab "3" _tab ".1" _n
file write `detail' "1.32" _tab "3" _tab "4" _tab ".1" _n
file write `detail' "1.41" _tab "4" _tab "4" _tab ".1" _n
file write `detail' "1.42" _tab "4" _tab "1" _tab ".1" _n
file write `detail' "1.51" _tab "5" _tab "4" _tab ".1" _n
file write `detail' "1.52" _tab "5" _tab "5" _tab ".1" _n
file close `detail'

// Independently exercise the bridge audit before articulation pruning.
mata: mata clear
quietly do `"`package_root'/kss_bc.mata"'
quietly do `"`package_root'/benchmarks/separations_sample.mata"'
quietly use `"`output_dir'/input.dta"', clear
generate byte __frequency = 1
generate byte __sample = 1
generate byte __keep = 0
tempname bridge_diagnostics
local bridge_status
local bridge_message
mata: kssbc_sep__stata_prune_bridges(                           ///
    "worker", "firm", "__frequency", "__sample", "__keep",   ///
    "`bridge_diagnostics'", "bridge_status", "bridge_message")
assert "`bridge_status'" == "CONVERGED"
assert `bridge_diagnostics'[1,1] == 2
quietly count if __keep
assert r(N) == 8

do `"`package_root'/benchmarks/separations_matlab_sample.do"' ///
    `"`package_root'"' fixture `"`output_dir'/input.dta"' ///
    `"0000000000000000000000000000000000000000000000000000000000000000"' ///
    `"`output_dir'/detail.tsv"' ///
    `"1111111111111111111111111111111111111111111111111111111111111111"' ///
    `"`output_dir'"' ///
    `"0000000000000000000000000000000000000000"' ///
    `"1111111111111111111111111111111111111111"' ///
    `"2222222222222222222222222222222222222222222222222222222222222222"'

quietly use `"`output_dir'/prepared.dta"', clear
assert _N == 8
assert worker <= 4
isid observation_key

confirm file `"`output_dir'/prepared.csv"'
import delimited using `"`output_dir'/prepared.csv"', clear
assert _N == 8
confirm numeric variable worker firm period y_minus_xb
assert !missing(worker,firm,period,y_minus_xb)

import delimited using `"`output_dir'/prepare.csv"', clear
assert sample_mode == "matlab"
assert sample_selection == "matlab_retained_bridge_core"
assert input_rows == 10
assert matlab_matches == 10
assert matlab_rows == 10
assert stored_rows == 8
assert workers == 4
assert firms == 4
assert matches == 8
assert graph_removed_rows == 2
assert bridge_removed_matches == 0
assert bridge_removed_rows == 0
assert audit_iterations == 1

local compare_run `"`output_dir'/compare-run"'
capture mkdir `"`compare_run'"'
capture mkdir `"`compare_run'/separations"'
capture mkdir `"`compare_run'/separations/fixture"'
capture mkdir `"`compare_run'/separations/fixture/b1"'
capture mkdir `"`compare_run'/separations/fixture/cmg"'
capture mkdir `"`compare_run'/separations/fixture/comparison"'
quietly use `"`output_dir'/prepared.dta"', clear
keep worker firm
duplicates drop
sort worker firm
save `"`compare_run'/separations/fixture/b1/retained_matches.dta"', replace
save `"`compare_run'/separations/fixture/cmg/retained_matches.dta"', replace
do `"`package_root'/benchmarks/separations_compare_samples.do"' ///
    fixture `"`compare_run'"' ///
    `"0000000000000000000000000000000000000000"'
import delimited using ///
    `"`compare_run'/separations/fixture/comparison/sample_overlap.csv"', ///
    clear asdouble
assert cmg_available == 1
assert b1_only_cmg == 0
assert cmg_only_b1 == 0
assert matlab_available == 0
assert missing(matlab_matches)

di as result "PASS test_separations_matlab_sample.do"
exit 0
