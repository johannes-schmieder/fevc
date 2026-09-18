version 18.0
clear all
set more off
args package_dir
set processors 4
adopath ++ `"`package_dir'"'
set obs 4
generate worker = _n
generate firm = mod(_n,2)
generate y = _n/7
local rng `"`c(rngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
capture noisily fevc y, worker(worker) firm(firm) backend(rust)
assert _rc==198
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
assert c(processors)==4 & `"`c(rngstate)'"'==`"`rng'"'
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
display as result "PASS thread_contract_failure_smoke.do"
