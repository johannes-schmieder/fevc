program define _fevc_stayer_population_post, eclass
    version 18.0
    if "${VCKSS_ROUTE_STAYERS_REQUESTED}"=="" | "`e(cmd)'"!="fevc" exit
    if inlist("`e(status)'","WITHHELD","ALPHA") exit
    ereturn local stayers "${VCKSS_ROUTE_STAYERS_REQUESTED}"
    ereturn scalar stayers_option_supplied = real("${VCKSS_ROUTE_STAYERS_SUPPLIED}")
    ereturn local stayer_option_schema "FEVC-STAYER-POPULATION-V1"
    ereturn scalar N_graph_input = real("${VCKSS_ROUTE_POPULATION_INPUT}")
    ereturn scalar N_complete = real("${VCKSS_ROUTE_POPULATION_COMPLETE}")
    ereturn scalar N_stayer_option_dropped = real("${VCKSS_ROUTE_STAYER_DROPPED}")
    if "`e(deletion)'"=="observation" {
        ereturn local target_population = cond("`e(stayers)'"=="both", ///
            "retained observations", "movers")
        ereturn local sample_selection = cond("`e(stayers)'"=="both", ///
            "MATLAB_LEAVEONEWORKER_COMPONENT", ///
            "MOVER_ONLY_LEAVEONEWORKER_COMPONENT")
        // The unchanged raw ABI flag selects augmentation, not population.
        ereturn local native_stayers_convention "legacy augmentation flag; see e(stayers) for population"
        if "`e(inference_population)'"!="" {
            ereturn local inference_population "`e(stayers)'"
            ereturn local inference_population_requested "`e(stayers)'"
            ereturn local inference_population_selected "`e(stayers)'"
            if "`e(inference_capability)'"!="" {
                local capability `"`e(inference_capability)'"'
                if "`e(stayers)'"=="both" {
                    local capability : subinstr local capability "mover-only" "retained observation population", all
                }
                ereturn local inference_capability `"`capability'"'
            }
        }
        if "`e(projection_effect)'"!="" {
            ereturn local inference_deletion ///
                "`e(algorithm)' observation deletion; stayers `e(stayers)'"
        }
    }
end
