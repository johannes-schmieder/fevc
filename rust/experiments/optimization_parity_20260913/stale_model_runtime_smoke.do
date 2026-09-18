version 18.0
clear all
set more off
args package_dir
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
assert mod(floor(r(core_ready_flags)/1024),2)==1
assert mod(floor(r(core_ready_flags)/2048),2)==0
foreach deletion in match observation {
    foreach nuisance in joint fixedoffset {
        capture noisily fevc y x, worker(worker) firm(firm) algorithm(jla) ///
            deletion(`deletion') nuisance(`nuisance') backend(rust) rng(counter_v1) ///
            engine(generic) preconditioner(cmg) batch(auto) probes(9) nodisplay
        assert _rc==498
        assert `"`e(withholding_status)'"'=="RUST_BACKEND_UNQUALIFIED"
        assert `"`c(rngstate)'"'==`"`state'"'
        assert `"`c(sortrngstate)'"'==`"`sortstate'"'
        quietly _datasignature
        assert `"`r(datasignature)'"'==`"`signature'"'
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
}
display "STALE_MODEL_RUNTIME_REJECTION_PASS"
