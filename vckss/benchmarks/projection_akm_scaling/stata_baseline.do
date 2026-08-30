version 18.0
clear all
set more off
args cores_arg processors_arg
local cores = real("`cores_arg'")
local processors = real("`processors_arg'")
assert inlist(`cores',4,16) & `processors'==min(4,`cores')
set processors `processors'
di as result "VCKSS PROJECTION AKM STATA BASELINE PASS cores=`cores' processors=`processors'"
exit 0
