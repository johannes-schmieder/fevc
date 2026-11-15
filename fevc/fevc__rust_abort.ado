*! Releases a native generation after a failure and posts the typed
*! withholding status from the native error receipt.

// Its own ado-file: the native helper ado-files call it too, and a program
// defined after the first one of an ado-file is private to that file.
program define fevc__rust_abort, eclass
    version 15.0
    syntax , RC(integer) [HANDLE(real 0) PHASE(string) NORELEASE]

    if `rc' == 1 {
        if `handle' > 0 & "`norelease'" == "" {
            capture quietly fevc_rust release `handle'
        }
        capture quietly fevc_rust clear
        ereturn clear
        exit 1
    }

    capture quietly fevc_rust lasterror
    if _rc {
        local native_code = .
        local native_status "NATIVE_ERROR_UNAVAILABLE"
        local native_detail "The Rust failure occurred, but its structural error receipt could not be retrieved."
    }
    else {
        local native_code = r(native_error_code)
        local native_status `"`r(native_error_status)'"'
        local native_detail `"`r(native_error_detail)'"'
    }
    local public_rc = `rc'
    // Preserve the established public Mata status for a deletion identifier
    // that crosses worker-firm coordinates.  The native canonicalizer uses
    // the broader typed INVALID_IDENTIFIER family internally.
    if `"`native_status'"'=="INVALID_IDENTIFIER" &                 ///
        strpos(`"`native_detail'"',"deletion identifier")>0 {
        local native_status "CROSS_COORDINATE_MATCH"
    }
    if `"`native_status'"'=="RESOURCE_LIMIT" &                    ///
        strpos(`"`native_detail'"',"exact_limit()")>0 {
        local native_status "EXACT_SIZE_LIMIT"
        local public_rc = 198
    }
    if `"`native_status'"'=="RESOURCE_LIMIT" &                    ///
        strpos(`"`native_detail'"',"deletion block")>0 {
        local native_status "BLOCK_SIZE_LIMIT"
        local public_rc = 198
    }
    if `"`native_status'"'=="RESOURCE_LIMIT" &                    ///
        strpos(`"`native_detail'"',"physical-frequency total")>0 {
        local native_status "PHYSICAL_TOTAL_LIMIT"
        local public_rc = 498
    }
    if `"`native_status'"'=="SINGULAR_INFORMATION" &              ///
        "`phase'"=="solve_jla" {
        local native_status "SINGULAR_NUISANCE_BLOCK"
    }
    if `"`native_status'"'=="INVERSE_RESIDUAL_FAILED" &           ///
        strpos(`"`native_detail'"',"control_basis_gram")>0 {
        local native_status "AMBIGUOUS_CONTROL_BASIS"
    }
    if `"`native_status'"'=="GRAPH_UNIDENTIFIED" &                ///
        strpos(`"`native_detail'"',"tie for largest-component")>0 {
        local native_status "AMBIGUOUS_LARGEST_COMPONENT"
    }
    // A Stata-side failure can occur after native preparation (for example,
    // while copying the retained mask or returned matrices).  In that case
    // the native error slot is still the cleared OK receipt.  Preserve the
    // original Stata rc, clean the registry, and do not manufacture a typed
    // native failure with status OK.
    if !missing(`native_code') & `native_code' == 0 &          ///
        "`native_status'" == "OK" {
        if `handle' > 0 & "`norelease'" == "" {
            capture quietly fevc_rust release `handle'
        }
        capture quietly fevc_rust clear
        ereturn clear
        exit `rc'
    }
    if `handle' > 0 & "`norelease'" == "" {
        capture quietly fevc_rust release `handle'
    }
    capture quietly fevc_rust clear
    quietly fevc__post_failure "`native_status'" `"`native_detail'"'
    quietly fevc__control_failure_post `"`native_detail'"'
    ereturn scalar native_error_code = `native_code'
    ereturn local native_error_phase "`phase'"
    ereturn local backend_requested "rust"
    ereturn local backend_selected ""
    ereturn local backend_routing_reason                       ///
        "explicit Rust lifecycle failed; no fallback is permitted"
    ereturn local rng_requested `"${VCKSS_ROUTE_RNG_REQUESTED}"'
    ereturn local rng_selected ""
    ereturn scalar backend_option_supplied = 1
    ereturn scalar rng_option_supplied =                       ///
        real("${VCKSS_ROUTE_RNG_SUPPLIED}")
    exit `public_rc'
end
