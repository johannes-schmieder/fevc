version 18.0
clear all
set more off
args package_dir expected_native_v6
if "`expected_native_v6'"=="" local expected_native_v6 = 0
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set obs 128
generate long worker = ceil(_n/4)
generate int firm = mod(worker-1+mod(_n-1,4),16)+1
generate double x = sin(_n)
generate double y = .3*worker-.2*firm+mod(17*_n,29)/101
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
quietly fevc_rust probe
assert mod(floor(r(core_ready_flags)/2048),2)==1
assert mod(floor(r(core_ready_flags)/4096),2)==`expected_native_v6'
assert r(execution_api)==0
// A cached scalar must not make an old C transport appear current.
scalar __vckss_rust_execution_api = 1
foreach deletion in match observation {
    foreach options in "preconditioner(diagonal) batch(8)" "preconditioner(diagonal) batch(auto)" ///
        "preconditioner(cmg) project(x) projecteffect(firm)" {
        capture noisily fevc y, worker(worker) firm(firm) algorithm(jla) stayers(movers) ///
            deletion(`deletion') backend(rust) rng(counter_v1) engine(generic) ///
            `options' probes(9) nodisplay
        assert _rc==498
        assert "`e(withholding_status)'"=="RUST_BACKEND_UNQUALIFIED"
        assert `"`c(rngstate)'"'==`"`state'"'
        assert `"`c(sortrngstate)'"'==`"`sortstate'"'
        quietly _datasignature
        assert `"`r(datasignature)'"'==`"`signature'"'
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
        assert r(last_released)==0
    }
}
display "STALE_EXECUTION_RUNTIME_REJECTION_PASS"
