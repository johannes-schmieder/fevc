*! Simple outcome-centering interface
program define fevc__centering, eclass
    version 18
    gettoken action 0 : 0
    macro shift
    if "`action'"=="request" {
        args centering inference project
    local centering = lower(strtrim("`centering'"))
    if "`centering'"=="" local centering mean
    if !inlist("`centering'","none","mean","corrected") {
        global VCKSS_CENTER_ERROR "INVALID_CENTERING"
        exit 198
    }
    global VCKSS_CENTERING `centering'
    if "`centering'"!="none" & !inlist(lower(strtrim("`inference'")),"","none") {
        global VCKSS_CENTER_ERROR "CENTERING_INFERENCE_UNSUPPORTED"
        di as error "inference() requires centering(none)"
        exit 498
    }
    if "`centering'"=="corrected" & "`project'"!="" {
        global VCKSS_CENTER_ERROR "CENTERING_INFERENCE_UNSUPPORTED"
        di as error "project() supports centering(none) or centering(mean)"
        exit 498
    }
        exit
    }
    if "`action'"=="tuning" {
        args probes batch seed tolerance maxiter exact_limit rank_tolerance block_tolerance blocksize_limit physical_limit
    if `probes' < 2 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "probes() must be at least two"
        exit 198
    }
    if `batch' < 1 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "batch() must be positive"
        exit 198
    }
    if `seed' < 0 | `seed' > 2147483646 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "seed() must be between zero and 2,147,483,646"
        exit 198
    }
    if `tolerance' < 1e-15 | `tolerance' > 1e-4 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "tolerance() must lie in [1e-15,1e-4]"
        exit 198
    }
    if `maxiter' < 1 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "maxiter() must be positive"
        exit 198
    }
    if `exact_limit' < 2 | `exact_limit' > 2000 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "exact_limit() must be between 2 and 2,000"
        exit 198
    }
    if `rank_tolerance' < 1e-14 | `rank_tolerance' >= 0.1 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "rank_tolerance() must lie in [1e-14,0.1)"
        exit 198
    }
    if `block_tolerance' < 1e-14 | `block_tolerance' >= 1 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "block_tolerance() must lie in [1e-14,1)"
        exit 198
    }
    if `blocksize_limit' < 1 | `blocksize_limit' > 1000000 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "blocksize_limit() must be between 1 and 1,000,000"
        exit 198
    }
    if `physical_limit' < 1 | `physical_limit' > 1000000000 {
        global VCKSS_CENTER_ERROR "INVALID_TUNING"
        di as error "physical_limit() must be between 1 and 1,000,000,000"
        exit 198
    }
        exit
    }
    if "`action'"=="post" {
        ereturn local centering "$VCKSS_CENTERING"
        ereturn local mcse_centering "fixed observed mean"
        if "$VCKSS_CENTERING"=="none" ereturn local mcse_centering "uncentered"
        if "$VCKSS_CENTERING"=="corrected" {
            ereturn local mcse_centering "fixed observed mean and fixed centering increment"
            if "`e(algorithm)'"=="jla" & "$VCKSS_NMC_MODE"!="off" & "$VCKSS_REPORT_LEVEL"!="0" {
                di as txt "MCSE treats the centering correction as fixed; its uncertainty is excluded."
            }
        }
    }
end
