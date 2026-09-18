version 18.0
clear all
set more off
args package_dir input_file
local threads : environment PF_NATIVE_THREADS
set processors `=min(4,real("`threads'"))'
adopath ++ `"`package_dir'"'
import delimited using `"`input_file'"', clear varnames(1) asdouble
sort observation_key
local rng `"`c(rngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
fevc y, worker(worker) firm(firm) deletion(match) probeorder(observation_key) ///
    stayers(movers) algorithm(jla) backend(rust) rng(counter_v1) ///
    engine(auto) preconditioner(cmg) probes(200) seed(104729) ///
    batch(auto) exact_limit(2) nodisplay
assert "`e(engine_selected)'"=="compressed"
assert e(cmg_threads_requested)==real("`threads'")
assert e(cmg_threads_used)<=real("`threads'")
assert c(processors)==min(4,real("`threads'"))
assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
assert `"`c(rngstate)'"'==`"`rng'"'
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
display as result "PASS compressed_boundary_smoke.do"
