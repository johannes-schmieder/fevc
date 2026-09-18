program define fevc__observation_population
    version 18.0
    args deletion population touse original_stayer worker firm ///
        dense_worker dense_firm complete implicit_match pair_first firm_count ///
        worker_tag sort_calls
    global VCKSS_ROUTE_POPULATION_COMPLETE `complete'
    global VCKSS_ROUTE_POPULATION_INPUT `complete'
    global VCKSS_ROUTE_STAYER_DROPPED 0
    if `implicit_match' exit
    // Preserve the existing complete-case map and stayer classification for
    // both estimators. The raw implicit-match path already supplies its maps.
    quietly egen long `dense_worker' = group(`worker') if `touse'
    quietly egen long `dense_firm' = group(`firm') if `touse'
    sort `dense_worker' `dense_firm'
    c_local prep_sort_calls = `sort_calls'+1
    quietly by `dense_worker' `dense_firm': generate byte `pair_first' = (_n==1) if `touse'
    quietly by `dense_worker': egen long `firm_count' = total(`pair_first') if `touse'
    quietly egen byte `worker_tag' = tag(`dense_worker') if `touse'
    quietly generate byte `original_stayer' = (`firm_count'==1) if `touse'
    quietly count if `worker_tag' & `firm_count'==1 & `touse'
    c_local N_stayers = r(N)
    quietly count if `firm_count'==1 & `touse'
    local stayer_rows = r(N)
    c_local N_stayer_rows `stayer_rows'
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
