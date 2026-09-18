version 18.0
clear all
set more off
set varabbrev off
args package_dir trace thread_caps
if "`thread_caps'"=="" local thread_caps "1 4 7"
adopath ++ `"`package_dir'"'
quietly fevc_rust probe
assert r(exact_api)==1 & r(exact_resolved_api)==2 & r(exact_legacy_api)==1
set obs 99
generate long key = _n
generate long worker = floor((_n-1)/8)+1
generate int firm = mod(floor((_n-1)/2),4)+1
replace worker = 14 in 99
replace firm = 1 in 97/98
replace firm = 2 in 99
generate long deletion_id = floor((_n-1)/2)+1
generate double y = .7*worker-.45*firm+.3*mod(key,2)+mod(7*key,5)/11
generate double control = (worker-.4*firm)*(mod(key,2)+1)+mod(3*key,7)/17
generate byte frequency = mod(key,3)+1
replace frequency = 2 in 99
generate double target = .5+mod(5*key,7)/3
sort key
local rng `"`c(rngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
foreach deletion in observation match {
    local deletion_option
    if "`deletion'"=="match" local deletion_option deletionid(deletion_id)
    foreach nuisance in joint fixedoffset {
        foreach population in movers both {
            quietly fevc y control [fw=frequency], worker(worker) firm(firm) ///
                deletion(`deletion') `deletion_option' nuisance(`nuisance') ///
                stayers(`population') targetweight(target) algorithm(exact) ///
                backend(mata) rng(stata) nodisplay
            matrix reference = e(results)
            foreach algorithm in exact auto {
                foreach threads of local thread_caps {
                    set processors `threads'
                    display as text "PUBLIC_EXACT deletion=`deletion' nuisance=`nuisance' population=`population' algorithm=`algorithm' threads=`threads'"
                    if "`trace'"=="1" set trace on
                    fevc y control [fw=frequency], worker(worker) firm(firm) ///
                        deletion(`deletion') `deletion_option' nuisance(`nuisance') ///
                        stayers(`population') targetweight(target) algorithm(`algorithm') ///
                        backend(rust) nodisplay
                    set trace off
                    assert `"`e(algorithm)'"'=="exact"
                    assert `"`e(algorithm_requested)'"'=="`algorithm'"
                    assert `"`e(rust_execution_mode)'"'=="exact_parallel"
                    assert e(active_processors)==`threads' & e(rust_native_threads)==`threads'
                    matrix work = e(rust_exact_execution)
                    assert work[1,4]==`threads' & inrange(work[1,5],1,`threads')
                    assert work[1,7]==1+("`deletion'"=="match" & "`population'"=="both")
                    assert work[1,10]<=e(residual_acceptance_tolerance)
                    assert e(probes)==0 & e(rng_master_seed)==0
                    assert mreldif(e(results),reference)<1e-8
                    quietly fevc_rust snapshot
                    assert r(state)==0 & r(handle)==0
                    assert `"`c(rngstate)'"'==`"`rng'"'
                }
            }
        }
    }
}
quietly _datasignature
assert `"`r(datasignature)'"'==`"`signature'"'
display as result "PASS public_exact_smoke.do"
