version 18.0
clear all
set more off
set varabbrev off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
foreach stayer_rows in 0 512 {
    clear
    set obs `=4096+`stayer_rows''
    generate long key = _n
    generate long worker = ceil(_n/8)
    generate byte slot = mod(_n-1,8)
    generate int firm = mod(worker-1+3*floor(slot/2),256)+1
    replace firm = mod(worker-1,256)+1 if key>4096
    generate double y = .3*worker-.2*firm+.1*slot+mod(17*key,29)/101
    local state `"`c(rngstate)'"'
    local sortstate `"`c(sortrngstate)'"'
    quietly _datasignature
    local signature `"`r(datasignature)'"'
    quietly fevc y, worker(worker) firm(firm) deletion(match) stayers(both) ///
        backend(rust) rng(counter_v1) algorithm(jla) engine(auto) ///
        preconditioner(cmg) batch(1) probes(33) seed(81227) tolerance(1e-10) nodisplay
    matrix reference = e(kss)
    generate byte reference_sample = e(sample)
    foreach threads in 1 4 {
        set processors `threads'
        foreach population in default both {
            local population_option
            if "`population'"=="both" local population_option stayers(both)
            fevc y, worker(worker) firm(firm) deletion(match) `population_option' ///
                backend(rust) rng(counter_v1) algorithm(jla) engine(auto) ///
                preconditioner(auto) batch(auto) probes(33) seed(81227) tolerance(1e-10) nodisplay
            display "POOLED_CASE rows=`stayer_rows' threads=`threads' population=`population' engine=`e(engine_selected)' cmg=`e(cmg_backend)' stayers=`e(stayers)'"
            display "POOLED_COUNTS N=" e(N_retained) " T=" e(cmg_threads_used) " batches=" e(leverage_batch) "," e(target_batch)
            assert `"`e(stayers)'"' == "both"
            assert `"`e(engine_selected)'"' == cond(`stayer_rows'==0,"compressed","generic")
            assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
            assert e(cmg_threads_requested)==`threads' & e(cmg_threads_used)==`threads'
            assert e(N_retained)==4096+`stayer_rows'
            assert e(sample)==reference_sample
            assert e(leverage_batch)==min(33,max(32,8*`threads'))
            assert e(target_batch)==min(33,max(32,4*`threads'))
            matrix candidate = e(kss)
            forvalues column=1/4 {
                assert abs(candidate[1,`column']-reference[1,`column'])<=1e-8*max(1,abs(reference[1,`column']))
            }
            assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
            assert `"`c(rngstate)'"'==`"`state'"'
            assert `"`c(sortrngstate)'"'==`"`sortstate'"'
            quietly fevc_rust snapshot
            assert r(state)==0 & r(handle)==0
        }
    }
    drop reference_sample
    quietly _datasignature
    assert `"`r(datasignature)'"'==`"`signature'"'
    // Omitted probeorder also keeps ordinary mover-only ingestion, not the
    // separately certified raw implicit-key optimization.
    quietly fevc y, worker(worker) firm(firm) deletion(match) stayers(movers) ///
        algorithm(jla) probes(33) seed(81227) nodisplay
    assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
    assert e(N_retained)==4096
}
display "PASS test_rust_pooled_full_cmg.do"
