version 18.0
clear all
set more off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set obs 128
generate long worker = ceil(_n/4)
generate int firm = mod(worker-1+mod(_n-1,4),16)+1
generate double y = .3*worker-.2*firm+mod(17*_n,29)/101
quietly fevc_rust probe
assert mod(floor(r(core_ready_flags)/4096),2)==1
capture noisily fevc y, worker(worker) firm(firm) algorithm(jla) stayers(movers) ///
    deletion(observation) backend(rust) rng(counter_v1) engine(generic) ///
    preconditioner(diagonal) batch(auto) probes(9) nodisplay
local rc = _rc
display "OLD_TRANSPORT_RESULT rc=`rc' phase=`e(native_error_phase)' status=`e(withholding_status)'"
quietly fevc_rust snapshot
display "OLD_TRANSPORT_STATE state=" r(state) " handle=" r(handle) " released=" r(last_released)
