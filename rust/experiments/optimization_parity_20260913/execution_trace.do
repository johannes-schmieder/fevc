version 18.0
clear all
set more off
set varabbrev off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 4
set obs 288
generate long key = _n
generate long worker = ceil(key/8)
generate byte slot = mod(key-1,8)
generate int firm = floor(slot/2)+1
replace firm = mod(worker-1,4)+1 if key>256
generate double x = (worker-.4*firm)*(mod(slot,2)+1)+mod(3*key,7)/17
generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101+.2*x
generate byte copies = 1+mod(key,3)
generate double mass = .7+mod(13*key,11)/7
sort worker firm key
local state `"`c(rngstate)'"'
local sortstate `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach deletion in observation match {
    foreach nuisance in joint fixedoffset {
        foreach population in both movers {
            local common worker(worker) firm(firm) deletion(`deletion') nuisance(`nuisance') ///
                stayers(`population') targetweight(mass) algorithm(jla) backend(rust) ///
                rng(counter_v1) engine(generic) probes(33) seed(81227) nodisplay
            quietly fevc y x [fw=copies], `common' preconditioner(cmg) batch(1)
            matrix reference = e(kss)
            generate byte reference_sample = e(sample)
            foreach threads in 1 4 7 {
                set processors `threads'
                foreach width in auto 7 8 {
                    noisily fevc y x [fw=copies], `common' preconditioner(diagonal) batch(`width')
                    set tracedepth 1
                    set trace on
                    assert "`e(rust_execution_mode)'"=="diagonal_queue"
                    assert "`e(rust_execution_schema)'"=="VCKSS-GENERIC-EXECUTION-V1"
                    matrix work = e(rust_execution_receipt)
                    assert work[1,"threads"]==`threads'
                    assert work[1,"mode"]==1 & work[1,"rank"]==1
                    assert work[1,"point"]==99 & work[1,"projection"]==0
                    assert work[1,"component"]==0 & work[1,"gram"]==0
                    assert work[1,"fit"]==1+("`nuisance'"=="fixedoffset")
                    assert work[1,"logical"]==work[1,"queued"]+work[1,"fit"]
                    assert work[1,"active"]>=1 & work[1,"active"]<=`threads'
                    assert work[1,"cmg"]==0 & work[1,"cmg_concurrency"]==0
                    assert work[1,"residual"]<=e(residual_acceptance_tolerance)
                    if "`width'"=="auto" {
                        assert e(leverage_batch)==min(33,max(32,8*`threads'))
                        assert e(target_batch)==min(33,max(32,4*`threads'))
                    }
                    else assert e(leverage_batch)==`width' & e(target_batch)==`width'
                    assert e(sample)==reference_sample
                    assert mreldif(e(kss),reference)<1e-8
                    assert e(memory_budget_supplied)==0
                    assert `"`c(rngstate)'"'==`"`state'"'
                    assert `"`c(sortrngstate)'"'==`"`sortstate'"'
                    quietly fevc_rust snapshot
                    assert r(state)==0 & r(handle)==0
                }
            }
            drop reference_sample
        }
    }
}
set processors 4
// Strict budget rejection and advisory/off success preserve explicit widths.
foreach policy in error warn off {
    capture noisily fevc y x [fw=copies], worker(worker) firm(firm) deletion(observation) ///
        algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) ///
        batch(8) probes(33) memory_gib(.000001) memorycheck(`policy') nodisplay
    if "`policy'"=="error" assert _rc!=0
    else {
        assert _rc==0
        assert e(leverage_batch)==8 & e(target_batch)==8
    }
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
assert `"`c(rngstate)'"'==`"`state'"'
assert `"`c(sortrngstate)'"'==`"`sortstate'"'
