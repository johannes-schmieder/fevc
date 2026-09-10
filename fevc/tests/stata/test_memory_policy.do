version 18.0
clear all
set more off
set varabbrev off
args package_dir
if `"`package_dir'"' == "" local package_dir "`c(pwd)'/fevc"
adopath ++ `"`package_dir'"'
set obs 480
gen long worker = ceil(_n/3)
gen byte period = mod(_n-1,3)
gen long firm = mod(worker-1+period*(1+floor((worker-1)/16)),16)+1
gen double y = worker/23 + firm/11 + mod(_n,13)/17
local original_rng `"`c(rngstate)'"'
foreach backend in rust mata {
    fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
        engine(generic) preconditioner(diagonal) batch(auto) probes(40) nodisplay
    assert e(memory_budget_supplied)==0 & missing(e(memory_gib))
    assert `"`e(memory_check)'"'=="warn"
    assert e(memory_forecast_bytes)>0 & e(memory_forecast_bytes)<.
    matrix baseline = e(results)
    local baseline_batch = e(batch)
    foreach policy in warn error off {
        fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
            engine(generic) preconditioner(diagonal) batch(auto) probes(40) ///
            memorycheck(`policy') nodisplay
        assert e(memory_budget_supplied)==0 & missing(e(memory_gib))
        assert e(batch)==`baseline_batch'
        matrix replay = e(results)
        mata: assert(max(abs(st_matrix("baseline")-st_matrix("replay"))) < 1e-10)
    }
    foreach policy in warn off {
        fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
            engine(generic) preconditioner(diagonal) batch(8) probes(40) ///
            memory_gib(.000001) memorycheck(`policy') nodisplay
        assert e(memory_budget_supplied)==1 & e(batch)==8
        assert e(memory_forecast_bytes)>e(memory_gib)*1024^3
    }
    fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
        engine(generic) preconditioner(cmg) batch(8) probes(40) ///
        memory_gib(.000001) memorycheck(off) nodisplay
    assert e(batch)==8 & "`e(preconditioner_selected)'"=="CMG"
    capture fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
        engine(generic) preconditioner(diagonal) batch(8) probes(40) ///
        memory_gib(.000001) memorycheck(error) nodisplay
    assert _rc!=0
    assert `"`c(rngstate)'"' == `"`original_rng'"'
    fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
        algorithm(exact) nodisplay
    assert e(memory_budget_supplied)==0 & e(memory_forecast_bytes)>0
    capture fevc y, worker(worker) firm(firm) stayers(movers) backend(`backend') ///
        algorithm(exact) memory_gib(.000001) memorycheck(error) nodisplay
    assert _rc!=0
}
gen long observation_key = _n
foreach policy in warn off {
    fevc y, worker(worker) firm(firm) stayers(movers) backend(rust) ///
        rng(counter_v1) algorithm(jla) engine(auto) preconditioner(auto) ///
        probeorder(observation_key) batch(auto) probes(40) ///
        memory_gib(.0001) memorycheck(`policy') nodisplay
    assert "`e(cmg_backend)'"=="CMG_FULL_V2"
    assert e(memory_forecast_bytes)>e(memory_gib)*1024^3
    assert e(memory_admission_forecast_bytes)>=e(memory_forecast_bytes)
}
foreach invalid in 0 -1 . garbage {
    capture fevc y, worker(worker) firm(firm) memory_gib(`invalid') nodisplay
    assert _rc==198
}
assert "$VCKSS_MEMORY_ACTIVE"=="" & "$VCKSS_MEMORY_PRESENT"==""
assert `"`c(rngstate)'"' == `"`original_rng'"'
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
di "FEVC MEMORY POLICY PASS"
