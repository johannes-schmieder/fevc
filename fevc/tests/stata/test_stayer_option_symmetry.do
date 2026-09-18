version 18.0
clear all
set more off
set varabbrev off
args package_dir
if `"`package_dir'"' != "" adopath ++ `"`package_dir'"'

set obs 54
generate long original_order = _n
generate long worker = ceil(_n/4) if _n<=48
generate long firm = mod(_n-1,4)+1 if _n<=48
replace worker = 101+floor((_n-49)/2) if _n>48
replace firm = 1+floor((_n-49)/2) if _n>48
generate double y = .03*worker-.2*firm+sin(_n*1.7)
generate double frequency = 1
replace frequency = 2 in 49
generate double target = 1+mod(_n,3)/5
generate double z = cos(_n*.7)
generate byte mover = worker<100
local caller_rng `"`c(rngstate)'"'
local caller_sort_rng `"`c(sortrngstate)'"'
quietly datasignature
local caller_data `"`r(datasignature)'"'

foreach route in mata_exact mata_jla rust_exact rust_generic rust_full auto_full {
    display "STAYER_OPTION_CASE `route'"
    if "`route'"=="mata_exact" local opts backend(mata) rng(stata) algorithm(exact)
    if "`route'"=="mata_jla" local opts backend(mata) rng(stata) algorithm(jla) engine(generic) preconditioner(diagonal) batch(16) probes(33)
    if "`route'"=="rust_exact" local opts backend(rust) algorithm(exact)
    if "`route'"=="rust_generic" local opts backend(rust) rng(counter_v1) algorithm(jla) engine(generic) preconditioner(cmg) batch(auto) probes(33) probeorder(original_order)
    if "`route'"=="rust_full" local opts backend(rust) rng(counter_v1) algorithm(jla) engine(auto) preconditioner(auto) batch(auto) probes(33) probeorder(original_order)
    if "`route'"=="auto_full" local opts algorithm(jla) probes(33) probeorder(original_order)
    fevc y, worker(worker) firm(firm) deletion(observation) stayers(both) `opts' tolerance(1e-10) nodisplay
    display "BOTH `e(stayers)' retained=" e(N_retained) " supplied=" e(stayers_option_supplied)
    assert "`e(stayers)'"=="both"
    assert e(N_retained)==54
    assert e(sample)
    assert e(stayers_option_supplied)==1
    matrix both = e(results)
    if inlist("`route'","rust_full","auto_full") assert "`e(cmg_backend)'"=="CMG_FULL_V2"
    fevc y, worker(worker) firm(firm) deletion(observation) `opts' tolerance(1e-10) nodisplay
    display "DEFAULT `e(stayers)' retained=" e(N_retained) " supplied=" e(stayers_option_supplied)
    assert "`e(stayers)'"=="both"
    assert e(stayers_option_supplied)==0
    assert e(N_retained)==54
    assert mreldif(both,e(results))<1e-8
    fevc y, worker(worker) firm(firm) deletion(observation) stayers(movers) `opts' tolerance(1e-10) nodisplay
    display "MOVERS `e(stayers)' retained=" e(N_retained) " complete=" e(N_complete) " excluded=" e(N_stayer_option_dropped)
    assert "`e(stayers)'"=="movers"
    assert e(N_retained)==48
    assert e(N_complete)==54
    assert e(N_stayer_option_dropped)==6
    assert e(N_graph_input)==48
    assert e(sample)==mover
    matrix movers = e(results)
    fevc y if mover, worker(worker) firm(firm) deletion(observation) stayers(both) `opts' tolerance(1e-10) nodisplay
    assert mreldif(movers,e(results))<1e-8
    assert e(N_stayer_option_dropped)==0
    assert original_order==_n
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    assert `"`c(sortrngstate)'"'==`"`caller_sort_rng'"'
    quietly datasignature
    assert `"`r(datasignature)'"'==`"`caller_data'"'
}

// Physical-copy and target mass semantics survive the population filter.
foreach backend in mata rust {
    fevc y z [fw=frequency], worker(worker) firm(firm) deletion(observation) stayers(both) targetweight(target) algorithm(exact) backend(`backend') nodisplay
    assert e(N_retained)==54 & e(N_physical)==55
    matrix weighted_both = e(results)
    fevc y z [fw=frequency], worker(worker) firm(firm) deletion(observation) targetweight(target) algorithm(exact) backend(`backend') nodisplay
    assert mreldif(weighted_both,e(results))<1e-8
    fevc y z [fw=frequency], worker(worker) firm(firm) deletion(observation) stayers(movers) targetweight(target) algorithm(exact) backend(`backend') nodisplay
    assert e(N_retained)==48 & e(N_physical)==48
    matrix weighted_movers = e(results)
    fevc y z if mover [fw=frequency], worker(worker) firm(firm) deletion(observation) stayers(both) targetweight(target) algorithm(exact) backend(`backend') nodisplay
    assert mreldif(weighted_movers,e(results))<1e-8
    // Match deletion retains its existing hybrid and mover-only meanings.
    fevc y, worker(worker) firm(firm) deletion(match) algorithm(exact) backend(`backend') nodisplay
    assert "`e(stayers)'"=="both" & e(N_retained)==54
    matrix match_both = e(results)
    fevc y, worker(worker) firm(firm) deletion(match) stayers(both) algorithm(exact) backend(`backend') nodisplay
    assert mreldif(match_both,e(results))<1e-8
    fevc y, worker(worker) firm(firm) deletion(match) stayers(movers) algorithm(exact) backend(`backend') nodisplay
    assert e(N_retained)==48 & e(sample)==mover
}

foreach backend in mata rust {
    capture noisily fevc y if !mover, worker(worker) firm(firm) deletion(observation) stayers(movers) backend(`backend') algorithm(exact) nodisplay
    assert _rc==2000
    assert "`e(status)'"=="WITHHELD"
    assert "`e(withholding_status)'"=="NO_MOVER_SAMPLE"
    fevc y, worker(worker) firm(firm) deletion(observation) backend(`backend') algorithm(exact) nodisplay
    assert e(N_retained)==54 & "`e(stayers)'"=="both"
}
assert original_order==_n
assert `"`c(rngstate)'"'==`"`caller_rng'"'
// Display and postestimation must not treat observation/both as hybrid.
fevc y, worker(worker) firm(firm) deletion(observation) backend(mata) algorithm(exact)
estat diagnostics
assert "`e(stayers)'"=="both"
fevc y, worker(worker) firm(firm) deletion(observation) stayers(movers) backend(rust) algorithm(exact)
estat diagnostics
assert e(N_stayer_option_dropped)==6

// Existing observation inference uses the same retained population under
// omitted and explicit both; population naming does not select augmentation.
foreach backend in mata rust {
    local opts algorithm(exact) backend(mata)
    if "`backend'"=="rust" local opts algorithm(jla) backend(rust) rng(counter_v1) engine(generic) preconditioner(diagonal) batch(16) probes(33)
    fevc y, worker(worker) firm(firm) deletion(observation) `opts' project(z) projecteffect(firm) nodisplay
    matrix projection_both_b = e(projection_b)
    matrix projection_both_V = e(projection_V)
    assert e(N_retained)==54 & "`e(stayers)'"=="both"
    fevc y, worker(worker) firm(firm) deletion(observation) stayers(both) `opts' project(z) projecteffect(firm) nodisplay
    assert mreldif(projection_both_b,e(projection_b))<1e-8
    assert mreldif(projection_both_V,e(projection_V))<1e-8
    fevc y, worker(worker) firm(firm) deletion(observation) stayers(movers) `opts' project(z) projecteffect(firm) nodisplay
    matrix projection_movers_b = e(projection_b)
    matrix projection_movers_V = e(projection_V)
    assert e(sample)==mover
    fevc y if mover, worker(worker) firm(firm) deletion(observation) stayers(both) `opts' project(z) projecteffect(firm) nodisplay
    assert mreldif(projection_movers_b,e(projection_b))<1e-8
    assert mreldif(projection_movers_V,e(projection_V))<1e-8
}
display "STAYER_OPTION_SYMMETRY_PASS"
