version 18.0
clear
set more off
set varabbrev off

input double(y_minus_xb worker firm observation_key)
1.0 1 1 1
1.2 1 2 2
1.4 2 1 3
1.8 2 2 4
2.1 3 1 5
2.6 3 2 6
end

tempfile input_dta
quietly save `input_dta'
local output_dir `"`c(tmpdir)'/kss-scale-driver-`c(processid)'"'
capture mkdir `"`output_dir'"'
// c(processid) is empty on some Stata launchers, so remove the retired file
// before verifying that this invocation does not recreate it.
capture erase `"`output_dir'/route_pilot_diagnostics.csv"'

capture noisily do fevc/benchmarks/scc/kss_scale_driver.do ///
    route_receipt_smoke `"`input_dta'"' ///
    0000000000000000000000000000000000000000000000000000000000000000 ///
    1111111111111111111111111111111111111111111111111111111111111111 ///
    2222222222222222222222222222222222222222 ///
    local 1 2 8675309 auto 4 4 4 4 - - - `"`output_dir'"' 3600
assert _rc == 0

confirm file `"`output_dir'/route_diagnostics.csv"'
confirm file `"`output_dir'/summary.csv"'
capture confirm file `"`output_dir'/route_pilot_diagnostics.csv"'
assert _rc != 0

quietly import delimited using ///
    `"`output_dir'/route_diagnostics.csv"', clear
assert _N == 1
assert experiment_id == "route_receipt_smoke"
assert route_row == 1
confirm numeric variable planned_rhs route_code hierarchy_seconds
confirm numeric variable reserved18 reserved19 reserved20 reserved21 ///
    reserved23 reserved24
assert missing(reserved18) & missing(reserved19) & missing(reserved20)
assert missing(reserved21) & missing(reserved23) & missing(reserved24)

quietly import delimited using `"`output_dir'/summary.csv"', clear
assert route_evidence_available == 1
assert route_planned_rhs == 7
assert !missing(preconditioner_selected)
assert !missing(routing_reason)
assert inlist(route_api,"KSS-ROUTE-STRUCTURAL-V1",              ///
    "VCKSS-NATIVE-COMPRESSED-PLANNED-V4-V7")
assert option_contract == "KSS-STREAMLINE-OPTIONS-V1"

di as result "PASS test_scale_scc_driver.do"
