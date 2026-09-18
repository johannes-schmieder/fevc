*! version 0.4.0-alpha.1 26aug2026
program define _fevc_rust_solve_v5, rclass
    version 18.0
    args plugin handle seed probes leveragebatch targetbatch route tolerance_arg ///
        maxiter algorithm deletion nuisance exactlimit blocksizelimit            ///
        rank_tolerance_arg block_tolerance_arg engine batchmode stayers          ///
        targetweightmode deletionsource probeordersupplied wallsecondssupplied   ///
        physical_arg capabilityschema capabilityprofile frequencyused           ///
        signature_hi_arg signature_lo_arg leveragebatchmode targetbatchmode      ///
        fallback wallseconds_arg threads tolerancesupplied

    if `capabilityschema' != 3 | `capabilityprofile' != 4 {
        di as err "full-CMG Rust solve requires capability schema 3/profile 4"
        exit 198
    }
    if "`algorithm'" != "jla" | !inlist("`engine'","auto","generic") | ///
        !inlist("`route'","auto","cmg") | !inlist("`deletion'","match","observation") | ///
        !inlist("`nuisance'","joint","fixedoffset") | "`batchmode'" != "auto" | ///
        "`leveragebatchmode'" != "auto" |                          ///
        "`targetbatchmode'" != "auto" | !inlist("`stayers'","movers","all") | ///
        !inlist("`targetweightmode'","frequency","explicit") |     ///
        ("`deletion'"=="match" & !inlist("`deletionsource'","cell","matchid")) | ///
        ("`deletion'"=="observation" & "`deletionsource'"!="observation") | ///
        !inlist(`probeordersupplied',0,1) | ///
        !inlist(`frequencyused',0,1) | `leveragebatch' != 0 |        ///
        `targetbatch' != 0 | `fallback' != ("`route'"=="auto") {
        di as err "request is outside the qualified full-CMG Rust tuple"
        exit 198
    }
    if missing(`threads') | `threads' <= 0 | `threads' != floor(`threads') | ///
        !inlist(`tolerancesupplied', 0, 1) {
        di as err "invalid full-CMG threads or tolerance-supplied flag"
        exit 198
    }
    if !inlist(`wallsecondssupplied', 0, 1) {
        di as err "invalid full-CMG wallseconds-supplied flag"
        exit 198
    }
    local wallseconds = real("`wallseconds_arg'")
    if (`wallsecondssupplied' == 0 & `wallseconds' != 0) |           ///
        (`wallsecondssupplied' == 1 &                               ///
            (missing(`wallseconds') | `wallseconds' <= 0)) {
        di as err "invalid full-CMG wallseconds tuple"
        exit 198
    }
    foreach value in physical_arg signature_hi_arg signature_lo_arg {
        local parsed = real("``value''")
        if missing(`parsed') | `parsed' < 0 | `parsed' != floor(`parsed') {
            di as err "invalid full-CMG integer argument"
            exit 198
        }
    }
    if real("`physical_arg'") <= 0 |                               ///
        real("`physical_arg'") > 9007199254740992 |                ///
        real("`signature_hi_arg'") > 4294967295 |                  ///
        real("`signature_lo_arg'") > 4294967295 {
        di as err "full-CMG physical limit or signature half is out of range"
        exit 198
    }

    _fevc_rust_plugin_call `plugin', solvefull `handle' `seed' `probes' ///
        `leveragebatch' `targetbatch' `route' `tolerance_arg' `maxiter' ///
        `algorithm' `deletion' `nuisance' `exactlimit' `blocksizelimit' ///
        `rank_tolerance_arg' `block_tolerance_arg' `engine' `batchmode'  ///
        `stayers' `targetweightmode' `deletionsource'                    ///
        `probeordersupplied' `wallsecondssupplied' `physical_arg'        ///
        `capabilityschema' `capabilityprofile' `frequencyused'           ///
        `signature_hi_arg' `signature_lo_arg' `leveragebatchmode'        ///
        `targetbatchmode' `fallback' `wallseconds_arg' `threads'         ///
        `tolerancesupplied'

    return scalar handle = real("`handle'")
    return scalar seed = `seed'
    return scalar probes = `probes'
    return scalar threads = `threads'
    return scalar tolerance_supplied = `tolerancesupplied'
    return scalar capability_schema = `capabilityschema'
    return scalar capability_profile = `capabilityprofile'
    return scalar request_signature_hi = real("`signature_hi_arg'")
    return scalar request_signature_lo = real("`signature_lo_arg'")
    return scalar physical_limit = real("`physical_arg'")
    return scalar fallback_allowed = `fallback'
    return scalar wallseconds_supplied = `wallsecondssupplied'
    return scalar wallseconds = `wallseconds'
    return local cmg_backend "CMG_FULL_V2"
    return local route "`route'"
    return local backend "rust"
    return local subcommand "solvefull"
end
