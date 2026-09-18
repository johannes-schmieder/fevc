version 18.0
clear all
set more off
set varabbrev off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 4
set obs 4096
generate long key = _n
generate long worker = ceil(_n/8)
generate byte slot = mod(_n-1,8)
generate int firm = mod(worker-1+3*floor(slot/2),256)+1
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101
sort worker firm key
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach deletion in observation match {
    quietly fevc y, worker(worker) firm(firm) deletion(`deletion') ///
        nuisance(joint) stayers(movers) probeorder(key) backend(rust) rng(counter_v1) ///
        algorithm(jla) engine(generic) preconditioner(cmg) batch(auto) ///
        probes(33) seed(81227) tolerance(1e-10) nodisplay
    matrix reference = e(kss)
    generate byte reference_sample = e(sample)
    foreach threads in 1 4 {
        set processors `threads'
        noisily fevc y, worker(worker) firm(firm) deletion(`deletion') ///
            nuisance(joint) stayers(movers) probeorder(key) backend(rust) rng(counter_v1) ///
            algorithm(jla) engine(auto) preconditioner(auto) batch(auto) ///
            probes(33) seed(81227) tolerance(1e-10) nodisplay
        assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
        assert e(cmg_threads_used) == `threads'
        assert e(cmg_threads_requested) == `threads'
        assert e(leverage_batch) == min(33,max(32,8*`threads'))
        assert e(target_batch) == min(33,max(32,4*`threads'))
        assert e(full_cmg_receipt)[1,45] == max(e(leverage_batch),2*e(target_batch))
        assert e(sample) == reference_sample
        matrix candidate = e(kss)
        forvalues column = 1/4 {
            assert abs(candidate[1,`column']-reference[1,`column']) <= 1e-8*max(1,abs(reference[1,`column']))
        }
        assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
        assert `"`c(rngstate)'"' == `"`state'"'
        assert `"`c(sortrngstate)'"' == `"`sortstate'"'
        quietly fevc_rust snapshot
        assert r(state) == 0 & r(handle) == 0
    }
    drop reference_sample
}
quietly _datasignature
assert `"`r(datasignature)'"' == `"`signature'"'
quietly fevc y, worker(worker) firm(firm) deletion(observation) ///
    nuisance(joint) stayers(movers) probeorder(key) backend(auto) rng(auto) ///
    algorithm(jla) engine(auto) preconditioner(auto) batch(auto) probes(33) seed(81227) nodisplay
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
assert e(full_cmg_receipt)[1,18] == 1e-10
assert e(full_cmg_receipt)[1,19] == 1e-6
assert e(rust_rhs_receipts)[1,13] == 1e-9
assert e(rust_rhs_receipts)[2,13] == max(1e-11,10*1e-6)
assert e(memory_budget_supplied) == 0
capture noisily fevc y, worker(worker) firm(firm) deletion(observation) ///
    nuisance(joint) stayers(movers) probeorder(key) backend(auto) rng(auto) ///
    algorithm(jla) engine(auto) preconditioner(auto) batch(auto) probes(33) ///
    memory_gib(.000001) memorycheck(error) nodisplay
assert _rc != 0
assert `"`c(rngstate)'"' == `"`state'"'
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
display "PASS test_rust_observation_full_cmg.do"
