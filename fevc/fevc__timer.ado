*! Private profiling bridge; no timer reservation may affect estimation.
program define fevc__timer, rclass
    version 18.0
    gettoken action slot : 0
    if "`action'"=="profiles" {
        _fevc_timer_profiles `slot'
        exit
    }
    if inlist("`action'","load","begin") {
        capture mata: assert(vckss_timer__api_level()==1)
        if _rc {
            capture mata: vckss_timer__api_level()
            if !_rc {
                di as error "incompatible timing runtime; restart Stata or run discard"
                exit 498
            }
            findfile fevc_timer.mata
            quietly do `"`r(fn)'"'
        }
    }
    if "`action'"=="load" exit
    if inlist("`action'","begin","end") {
        mata: vckss_timer__`action'()
        exit
    }
    local slot = real(strtrim("`slot'"))
    assert `slot'>=1 & `slot'<=128 & `slot'==floor(`slot')
    if "`action'"=="read" {
        tempname elapsed
        mata: st_numscalar("`elapsed'",vckss_timer__seconds(`slot'))
        return scalar seconds = scalar(`elapsed')
    }
    else if inlist("`action'","on","off","clear") {
        mata: vckss_timer__`action'(`slot')
    }
    else exit 198
end

program define _fevc_timer_profiles
    version 18.0
    syntax namelist(min=2 max=2), SELECTION(real) GRAPH(real) ///
        SEMANTIC(real) COMPRESSION(real) TRANSITION(real) RESTORE(real) ///
        MARK(real) GROUP(real) RUNTIME(real) GRAPHIO(real) MAP(real)
    gettoken summary detail : namelist
    local other = `selection'-`graph'
    if !missing(`other') local other = max(0,`other')
    local total = `other'+`graph'+`semantic'+`compression'+`transition'+`restore'
    matrix `summary' = (`other',`graph',`semantic',`compression', ///
        `transition',`restore',`total')
    matrix colnames `summary' = selection_other graph_prune semantic_order ///
        compression_prepare lifecycle_transition lifecycle_restore observed_total
    local total = `mark'+`group'+`runtime'+`graphio'+`graph'+`map'+ ///
        `semantic'+`compression'+`transition'+`restore'
    matrix `detail' = (`mark',`group',`runtime',`graphio',`graph',`map', ///
        `semantic',`compression',`transition',`restore',`total')
    matrix colnames `detail' = mark_validate initial_group runtime_setup ///
        graph_setup_io graph_prune retained_map semantic_order ///
        compression_prepare lifecycle_transition lifecycle_restore observed_total
end
