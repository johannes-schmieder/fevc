program define fevc__observation_population
    version 18.0
    args deletion population touse original_stayer worker firm ///
        dense_worker dense_firm complete implicit_match pair_first unit_count ///
        worker_tag sort_calls deletionid rust_public rust_strict core_flags
    c_local population_failure "POPULATION_PREPARATION_FAILED"
    global VCKSS_ROUTE_POPULATION_COMPLETE `complete'
    global VCKSS_ROUTE_POPULATION_INPUT `complete'
    global VCKSS_ROUTE_STAYER_DROPPED 0
    if `implicit_match' {
        c_local N_stayers = 0
        c_local N_stayer_rows = 0
        exit
    }
    // The raw implicit-match path already supplies its maps.
    quietly egen long `dense_worker' = group(`worker') if `touse'
    quietly egen long `dense_firm' = group(`firm') if `touse'
    // Freeze eligibility from original units before any stayer augmentation.
    // Observation mode and implicit/default matches retain firm-count semantics.
    local history_id `dense_firm'
    local match_parallel 0
    if "`deletion'"=="match" & "`deletionid'"!="" {
        sort `deletionid' `dense_worker' `dense_firm'
        capture by `deletionid': assert                         ///
            `dense_worker'==`dense_worker'[1] &                  ///
            `dense_firm'==`dense_firm'[1] if `touse'
        if _rc {
            c_local population_failure "CROSS_COORDINATE_MATCH"
            di as error "each deletion ID must remain within one worker-firm coordinate"
            exit 198
        }
        sort `dense_worker' `dense_firm' `deletionid'
        tempvar parallel
        quietly by `dense_worker' `dense_firm': generate byte `parallel' = ///
            (`deletionid'!=`deletionid'[1]) if `touse'
        quietly count if `parallel'==1 & `touse'
        local match_parallel = (r(N)>0)
        local history_id `deletionid'
    }
    sort `dense_worker' `history_id'
    c_local prep_sort_calls = `sort_calls'+1+2*("`deletion'"=="match" & "`deletionid'"!="")
    quietly by `dense_worker' `history_id': generate byte `pair_first' = (_n==1) if `touse'
    quietly by `dense_worker': egen long `unit_count' = total(`pair_first') if `touse'
    quietly egen byte `worker_tag' = tag(`dense_worker') if `touse'
    quietly generate byte `original_stayer' = (`unit_count'==1) if `touse'
    quietly count if `worker_tag' & `unit_count'==1 & `touse'
    c_local N_stayers = r(N)
    quietly count if `unit_count'==1 & `touse'
    local stayer_rows = r(N)
    c_local N_stayer_rows `stayer_rows'
    // Older plugins count coefficient firms, including at final certification.
    // The additive flag admits parallel blocks only with the matching core.
    local flags = real("`core_flags'")
    local units_ready = !missing(`flags') & `flags'>=0 & ///
        `flags'==floor(`flags') & mod(floor(`flags'/32768),2)==1
    if `rust_public' & `match_parallel' & !`units_ready' {
        local backend_routing_reason ///
            "The loaded Rust plugin lacks deletion-unit mover support; Mata is required"
        c_local backend_routing_reason `"`backend_routing_reason'"'
        global VCKSS_ROUTE_BACKEND_REASON `"`backend_routing_reason'"'
        if `rust_strict' {
            global VCKSS_ROUTE_BACKEND_SELECTED ""
            global VCKSS_ROUTE_RNG_SELECTED ""
            c_local population_failure "RUST_PARALLEL_DELETION_UNSUPPORTED"
            di as error "update the Rust plugin or use backend(mata) and rng(stata) for parallel declared matches"
            exit 498
        }
        c_local rust_public = 0
        c_local backend_selected mata
        c_local rng_selected stata
        c_local backend_fallback = 1
        c_local backend_fallback_reason "RUST_PARALLEL_DELETION_UNSUPPORTED"
        c_local backend_fallback_phase "preflight"
        global VCKSS_ROUTE_BACKEND_SELECTED "mata"
        global VCKSS_ROUTE_RNG_SELECTED "stata"
        global VCKSS_ROUTE_FALLBACK 1
        global VCKSS_ROUTE_FB_REASON "RUST_PARALLEL_DELETION_UNSUPPORTED"
        global VCKSS_ROUTE_FB_PHASE "preflight"
    }

    if "`deletion'"!="observation" | "`population'"!="movers" exit
    if `stayer_rows'==0 exit
    // Select from frozen complete-case histories before graph preparation and
    // RNG. Do not reinterpret graph-dropped movers as eligible stayers.
    quietly replace `touse' = 0 if `touse' & `original_stayer'
    quietly count if `touse'
    local complete = r(N)
    global VCKSS_ROUTE_POPULATION_INPUT `complete'
    global VCKSS_ROUTE_STAYER_DROPPED `stayer_rows'
    c_local N_complete `complete'
    if `complete'==0 exit
    // Reuse the existing map storage rather than retain additional row arrays.
    quietly drop `dense_worker' `dense_firm'
    quietly egen long `dense_worker' = group(`worker') if `touse'
    quietly egen long `dense_firm' = group(`firm') if `touse'
end
