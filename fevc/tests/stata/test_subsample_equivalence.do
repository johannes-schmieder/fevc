version 18.0
clear all
set more off
set varabbrev off
args package_dir
if `"`package_dir'"' != "" adopath ++ `"`package_dir'"'

capture quietly fevc_rust probe
local native = (_rc==0)

// The compact dataset is an independent oracle for the requested population.
// Fix row order and seed; Mata also needs a tie-break key for repeated rows.
// Rust stayers(both) does not admit probeorder(), so use its canonical ordering.
program define _fevc_subsample_pair
    args model options filter
    local qualifier if wanted
    if "`filter'"=="in" local qualifier in 1/106
    local caller_rng `"`c(rngstate)'"'
    local caller_sort_rng `"`c(sortrngstate)'"'
    quietly datasignature
    local caller_data `"`r(datasignature)'"'
    quietly fevc `model' `qualifier', `options'
    assert rowid==_n
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    assert `"`c(sortrngstate)'"'==`"`caller_sort_rng'"'
    quietly datasignature
    assert `"`r(datasignature)'"'==`"`caller_data'"'
    tempvar chosen
    generate byte `chosen'=e(sample)
    assert inlist(`chosen',0,1)
    assert !`chosen' if !wanted
    quietly count if `chosen'
    assert e(N_retained)==r(N)
    local fields N N_retained N_physical worker_levels firm_levels deletion_units ///
        N_stayers N_stayer_rows
    if "`e(deletion)'"=="match" & "`e(stayers)'"=="both" local fields `fields' ///
        stayer_hybrid_N_stayers stayer_hybrid_N_stayer_rows ///
        stayer_hybrid_N_stayer_physical stayer_hybrid_N_singleton_drop ///
        stayer_hybrid_N_unattached stayer_hybrid_target_mass
    local index=0
    tempname sample_counts points mcse rows
    local field_count : word count `fields'
    matrix `sample_counts'=J(1,`field_count',.)
    foreach field of local fields {
        local ++index
        assert !missing(e(`field'))
        matrix `sample_counts'[1,`index']=e(`field')
    }
    matrix `points'=e(results)
    matrix `mcse'=e(numerical_mcse)
    local backend `e(backend_selected)'
    local projected = strpos("`options'","project(")>0
    tempname projection_b projection_V
    if `projected' {
        matrix `projection_b'=e(projection_b)
        matrix `projection_V'=e(projection_V)
    }
    mata: st_matrix("`rows'",select(st_data(.,"rowid"),st_data(.,"`chosen'")))
    preserve
    quietly keep if wanted
    quietly fevc `model', `options'
    assert "`e(backend_selected)'"=="`backend'"
    generate byte compact_chosen=e(sample)
    mata: assert(st_matrix("`rows'")==select(st_data(.,"rowid"),st_data(.,"compact_chosen")))
    local index=0
    foreach field of local fields {
        local ++index
        assert abs(e(`field')-`sample_counts'[1,`index'])<= ///
            1e-10*max(1,abs(`sample_counts'[1,`index']))
    }
    assert mreldif(`points',e(results))<1e-8
    assert mreldif(`mcse',e(numerical_mcse))<1e-8
    if `projected' {
        assert mreldif(`projection_b',e(projection_b))<1e-8
        assert mreldif(`projection_V',e(projection_V))<1e-8
    }
    restore
    if "`backend'"=="rust" {
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
end

// A well-connected mover component, eligible stayers, a physical singleton,
// and a smaller component test both frozen eligibility and graph exclusions.
set obs 110
generate long rowid=_n
generate long worker=ceil(_n/8)
generate long firm=ceil(mod(_n-1,8)/2)+1
replace worker=101 in 97/99
replace firm=1 in 97/99
replace worker=102 in 100/101
replace firm=2 in 100/101
replace worker=103 in 102
replace firm=3 in 102
replace worker=104 in 103/104
replace firm=9 in 103/104
replace worker=105 in 105/106
replace firm=8 in 105
replace firm=9 in 106
replace worker=1 in 107
replace firm=1 in 107
replace worker=101 in 108
replace firm=1 in 108
replace worker=999 in 109
replace firm=9 in 109
replace worker=1 in 110
replace firm=4 in 110
generate long match_id=100*worker+firm
generate double frequency=1+mod(rowid,2)
generate double target=frequency*(1+mod(rowid,5)/10)
generate double z=cos(rowid*.7)
generate double y=.03*worker-.2*firm+.4*z+sin(rowid*1.7)
generate byte wanted=rowid<=106
tempfile fixture
quietly save `fixture'
local routes mata_jla mata_exact
if `native' local routes `routes' rust_exact rust_jla
local pairs=0
foreach design in basic controls weighted pooled pooled_bare {
    foreach route of local routes {
        foreach filter in only_movers if in missing_outside exclude_mover {
            quietly use `fixture', clear
            local model y
            local opts worker(worker) firm(firm) deletion(match) ///
                seed(9262026) nodisplay
            if "`filter'"=="in" local opts `opts' stayers(both)
            if inlist("`design'","controls","weighted","pooled") local model y z
            if inlist("`design'","weighted","pooled") {
                local model `model' [fw=frequency]
                local opts `opts' targetweight(target)
            }
            if "`design'"=="weighted" local opts `opts' nuisance(fixedoffset)
            if inlist("`design'","pooled","pooled_bare") {
                quietly replace firm=ceil(firm/2)
                // Original blocks remain distinct at one coefficient firm.
                quietly replace firm=1 if worker==1
                local opts `opts' deletionid(match_id)
            }
            if "`filter'"=="only_movers" quietly replace wanted=rowid<=96
            if "`filter'"=="exclude_mover" quietly replace wanted=0 in 1
            if "`filter'"=="missing_outside" {
                foreach variable in worker firm match_id frequency target y z {
                    quietly replace `variable'=. if !wanted
                    quietly replace `variable'=.a in 108
                }
            }
            if substr("`route'",1,4)=="mata" local opts `opts' backend(mata) rng(stata) probeorder(rowid)
            else local opts `opts' backend(rust) rng(counter_v1)
            if strpos("`route'","exact") local opts `opts' algorithm(exact)
            else local opts `opts' algorithm(jla) engine(generic) ///
                preconditioner(diagonal) probes(33) batch(8)
            display "SUBSAMPLE_PAIR `design' `route' `filter'"
            _fevc_subsample_pair "`model'" "`opts'" "`filter'"
            local ++pairs
            if "`filter'"=="only_movers" {
                assert e(stayer_hybrid_N_stayers)==0
                assert e(stayer_hybrid_N_stayer_rows)==0
            }
        }
    }
}

// Preserve the unaffected mover-only and observation-deletion populations.
foreach route of local routes {
    foreach population in match_movers observation_both observation_movers projection {
        if "`population'"=="projection" & !inlist("`route'","mata_exact","rust_jla") continue
        if "`population'"=="projection" & "`route'"=="rust_jla" & ///
            !(strpos(lower("`c(machine_type)'"),"mac") | "`c(os)'"=="Unix") {
            display "SUBSAMPLE_PROJECTION_SKIPPED: unsupported native platform"
            continue
        }
        quietly use `fixture', clear
        quietly replace wanted=0 in 1
        local deletion match
        local stayers movers
        if strpos("`population'","observation") local deletion observation
        if inlist("`population'","observation_both","projection") local stayers both
        local opts worker(worker) firm(firm) deletion(`deletion') stayers(`stayers') ///
            targetweight(target) seed(9262026) nodisplay
        if "`population'"=="projection" local opts `opts' project(y) projecteffect(firm)
        if substr("`route'",1,4)=="mata" local opts `opts' backend(mata) rng(stata) probeorder(rowid)
        else local opts `opts' backend(rust) rng(counter_v1)
        if strpos("`route'","exact") local opts `opts' algorithm(exact)
        else local opts `opts' algorithm(jla) engine(generic) ///
            preconditioner(diagonal) probes(33) batch(8)
        display "SUBSAMPLE_CONTROL `route' `population'"
        _fevc_subsample_pair "y z [fw=frequency]" "`opts'" "exclude_mover"
        local ++pairs
    }
}

// The automatic/compressed no-control routes share the same sample contract.
foreach backend in mata rust {
    if "`backend'"=="rust" & !`native' continue
    quietly use `fixture', clear
    local rng stata
    local order probeorder(rowid)
    if "`backend'"=="rust" {
        local rng counter_v1
        local order
    }
    _fevc_subsample_pair "y" "worker(worker) firm(firm) backend(`backend') rng(`rng') `order' algorithm(jla) engine(auto) preconditioner(diagonal) probes(33) batch(8) seed(9262026) nodisplay" "if"
    local ++pairs
    // Inputs missing inside the requested match sample must still fail closed.
    quietly replace y=. in 1
    local caller_rng `"`c(rngstate)'"'
    quietly datasignature
    local caller_data `"`r(datasignature)'"'
    capture noisily fevc y if wanted, worker(worker) firm(firm) ///
        backend(`backend') algorithm(exact) nodisplay
    assert _rc==459
    assert "`e(withholding_status)'"=="MATCH_INPUT_MISSING"
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly datasignature
    assert `"`r(datasignature)'"'==`"`caller_data'"'
}
if !`native' display "SUBSAMPLE_NATIVE_SKIPPED: no loadable plugin"
display "SUBSAMPLE_EQUIVALENCE_PAIRS=" `pairs'
display "PASS test_subsample_equivalence.do"
