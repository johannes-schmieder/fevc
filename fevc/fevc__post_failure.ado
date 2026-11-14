*! Posts a typed withholding status, its explanation, and the request's
*! routing metadata into e().

program define fevc__post_failure, eclass
    version 18.0
    args failure_status failure_detail
    quietly fevc__failure_guidance "`failure_status'"
    local failure_reason `"`r(reason)'"'
    local failure_suggestion `"`r(suggestion)'"'
    if `"`failure_detail'"' == "" local failure_detail `"`failure_reason'"'
    ereturn clear
    ereturn local cmd "fevc"
    ereturn local version "0.6.0"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local status "WITHHELD"
    ereturn local withholding_status "`failure_status'"
    ereturn local withholding_detail `"`failure_detail'"'
    ereturn local withholding_reason `"`failure_reason'"'
    ereturn local withholding_suggestion `"`failure_suggestion'"'
    if "${VCKSS_ROUTE_STAYERS_REQUESTED}"!="" ///
        ereturn local stayers "${VCKSS_ROUTE_STAYERS_REQUESTED}"
    if "${VCKSS_ROUTE_METADATA_READY}" == "1" {
        ereturn local backend_requested ///
            `"${VCKSS_ROUTE_BACKEND_REQUESTED}"'
        ereturn local backend_selected ///
            `"${VCKSS_ROUTE_BACKEND_SELECTED}"'
        ereturn local backend_routing_reason ///
            `"${VCKSS_ROUTE_BACKEND_REASON}"'
        ereturn scalar backend_option_supplied = ///
            real("${VCKSS_ROUTE_BACKEND_SUPPLIED}")
        ereturn local rng_requested `"${VCKSS_ROUTE_RNG_REQUESTED}"'
        ereturn local rng_selected `"${VCKSS_ROUTE_RNG_SELECTED}"'
        ereturn scalar rng_option_supplied = ///
            real("${VCKSS_ROUTE_RNG_SUPPLIED}")
        ereturn scalar backend_fallback = ///
            real("${VCKSS_ROUTE_FALLBACK}")
        ereturn local backend_fallback_reason ///
            `"${VCKSS_ROUTE_FB_REASON}"'
        ereturn local backend_fallback_phase ///
            `"${VCKSS_ROUTE_FB_PHASE}"'
        if "${VCKSS_ROUTE_ALGORITHM_REQUESTED}" != "" {
            ereturn local algorithm "${VCKSS_ROUTE_ALGORITHM_REQUESTED}"
            ereturn local engine_requested "${VCKSS_ROUTE_ENGINE_REQUESTED}"
            ereturn local preconditioner_requested ///
                "${VCKSS_ROUTE_PRECOND_REQUESTED}"
            ereturn local deletion "${VCKSS_ROUTE_DELETION_REQUESTED}"
            ereturn local nuisance "${VCKSS_ROUTE_NUISANCE_REQUESTED}"
            ereturn scalar algorithm_option_supplied =             ///
                real("${VCKSS_ROUTE_ALGORITHM_SUPPLIED}")
            ereturn scalar engine_option_supplied =                ///
                real("${VCKSS_ROUTE_ENGINE_SUPPLIED}")
            ereturn scalar preconditioner_option_supplied =        ///
                real("${VCKSS_ROUTE_PRECOND_SUPPLIED}")
            ereturn scalar batch_option_supplied =                 ///
                real("${VCKSS_ROUTE_BATCH_SUPPLIED}")
            ereturn scalar stayers_option_supplied =               ///
                real("${VCKSS_ROUTE_STAYERS_SUPPLIED}")
            ereturn scalar deletionid_option_supplied =            ///
                real("${VCKSS_ROUTE_DELETIONID_SUPPLIED}")
        }
    }
end
