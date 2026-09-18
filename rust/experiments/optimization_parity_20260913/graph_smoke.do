version 18.0
clear all
set more off
set varabbrev off
args package inputs
adopath ++ `"`package'"'
quietly run `"`package'/fevc.ado"'
set processors 4
foreach pattern in degree5_well_mixed degree5_segmented mixed_well_mixed mixed_segmented degree4_bottleneck {
    import delimited using `"`inputs'/`pattern'.csv"', clear varnames(1)
    assert _N==8000
    local state `"`c(rngstate)'"'
    local sortstate `"`c(sortrngstate)'"'
    quietly datasignature
    local signature `"`r(datasignature)'"'
    foreach deletion in match observation {
        fevc y, worker(worker) firm(firm) deletion(`deletion') stayers(both) ///
            backend(rust) rng(counter_v1) algorithm(jla) engine(auto) ///
            preconditioner(auto) batch(auto) probes(200) seed(104729) nodisplay
        assert e(N_retained)==8000 & e(sample)
        assert `"`e(stayers)'"'=="both"
        assert `"`e(cmg_backend)'"'=="CMG_FULL_V2"
        assert e(cmg_threads_requested)==4 & e(cmg_threads_used)==4
        assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
        assert `"`c(rngstate)'"'==`"`state'"'
        assert `"`c(sortrngstate)'"'==`"`sortstate'"'
        quietly datasignature
        assert `"`r(datasignature)'"'==`"`signature'"'
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
        display "FIVE_GRAPH_LOCAL_CELL_PASS `pattern' `deletion'"
    }
}
display "FIVE_GRAPH_LOCAL_SMOKE_PASS"
