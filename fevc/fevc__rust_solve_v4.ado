*! version 0.4.0-alpha.1 23aug2026
program define fevc__rust_solve_v4, rclass
    version 18.0
    args plugin handle seed probes leveragebatch targetbatch route tolerance_arg ///
        maxiter algorithm deletion nuisance exactlimit blocksizelimit            ///
        rank_tolerance_arg block_tolerance_arg engine batchmode stayers          ///
        targetweightmode deletionsource probeordersupplied wallsecondssupplied   ///
        physical_arg capabilityschema capabilityprofile frequencyused           ///
        signature_hi_arg signature_lo_arg leveragebatchmode targetbatchmode      ///
        fallback wallseconds_arg execution threads componentbatchauto tolerancesupplied exactexecution

    // Optional additive executor; all existing calls retain the V4 selector.
    if "`execution'"=="" local execution = 0
    if "`componentbatchauto'"=="" local componentbatchauto = 0
    if "`tolerancesupplied'"=="" local tolerancesupplied = 0
    if "`exactexecution'"=="" local exactexecution = 0
    local solve_selector solve
    local execution_args
    if !inlist(`exactexecution',0,1,2) | (`exactexecution' & ///
        (`execution' | `componentbatchauto' | missing(`threads') | ///
        `threads'<=0 | `threads'>4294967295 | `threads'!=floor(`threads') | ///
        "`algorithm'"!=cond(`exactexecution'==1,"exact","auto"))) {
        di as err "request is outside the exact execution tuple"
        exit 198
    }
    if `exactexecution' {
        local solve_selector = cond(`exactexecution'==1,"solveexactexecution","solveexactresolvedv2")
        local execution_args `threads'
    }
    if !inlist(`execution',0,1,2,3) {
        di as err "invalid generic execution mode"
        exit 198
    }
    if !inlist(`componentbatchauto',0,1) | (`componentbatchauto' & !inlist(`execution',1,2)) | ///
        !inlist(`tolerancesupplied',0,1) {
        di as err "invalid component batch execution mode"
        exit 198
    }
    if `execution' {
        if (`execution'!=3 & ("`algorithm'"!="jla" | "`engine'"!="generic" | `fallback'!=0)) | ///
            (`execution'==1 & "`route'"!="diagonal") | ///
            (`execution'==2 & ("`route'"!="cmg" | "`batchmode'"!="auto")) | ///
            (`execution'==3 & (!inlist("`algorithm'","auto","jla") | ///
                !inlist("`engine'","auto","generic") | "`route'"!="auto" | ///
                "`batchmode'"!="auto" | `fallback'!=1)) | ///
            missing(`threads') | `threads'<=0 | `threads'>4294967295 | ///
            `threads'!=floor(`threads') {
            di as err "request is outside the generic execution tuple"
            exit 198
        }
        local solve_selector solveexecution
        local execution_args `threads' `execution'
        if `componentbatchauto' {
            local solve_selector solveexecutionv7
            local execution_args `threads' `execution' 1
        }
        if `execution'==3 {
            local solve_selector solveexecutionv8
            local execution_args `threads' `execution' `tolerancesupplied'
        }
    }

    if `capabilityschema' != 3 | `capabilityprofile' != 4 {
        di as err "planned Rust solve requires capability schema 3/profile 4"
        exit 198
    }
    if !inlist("`engine'", "auto", "compressed", "generic") {
        di as err "planned Rust solve requires engine(auto|compressed|generic)"
        exit 198
    }
    if !inlist("`batchmode'", "auto", "explicit", "independent") |       ///
        !inlist("`leveragebatchmode'", "auto", "explicit") |             ///
        !inlist("`targetbatchmode'", "auto", "explicit") {
        di as err "invalid planned Rust batch mode"
        exit 198
    }
    local expected_batchmode independent
    if "`leveragebatchmode'" == "`targetbatchmode'" {
        local expected_batchmode `leveragebatchmode'
    }
    if "`batchmode'" != "`expected_batchmode'" {
        di as err "batchmode() does not summarize the two planned phase modes"
        exit 198
    }
    if "`leveragebatchmode'" == "auto" & `leveragebatch' != 0 {
        di as err "leveragebatch() must be zero with leveragebatchmode(auto)"
        exit 198
    }
    if "`targetbatchmode'" == "auto" & `targetbatch' != 0 {
        di as err "targetbatch() must be zero with targetbatchmode(auto)"
        exit 198
    }
    if "`leveragebatchmode'" == "explicit" & `leveragebatch' <= 0 {
        di as err "leveragebatch() must be positive with leveragebatchmode(explicit)"
        exit 198
    }
    if "`targetbatchmode'" == "explicit" & `targetbatch' <= 0 {
        di as err "targetbatch() must be positive with targetbatchmode(explicit)"
        exit 198
    }
    if !inlist(`fallback', 0, 1) | (`fallback' == 1 & "`route'" != "auto") {
        di as err "fallback() must be zero, or one only with route(auto)"
        exit 198
    }
    if !inlist(`probeordersupplied', 0, 1) |                          ///
        !inlist(`wallsecondssupplied', 0, 1) |                         ///
        !inlist(`frequencyused', 0, 1) {
        di as err "invalid planned Rust semantic flag"
        exit 198
    }
    local wallseconds = real("`wallseconds_arg'")
    if (`wallsecondssupplied' == 0 & `wallseconds' != 0) |             ///
        (`wallsecondssupplied' == 1 &                                 ///
            (missing(`wallseconds') | `wallseconds' <= 0)) {
        di as err "invalid planned Rust wallseconds tuple"
        exit 198
    }
    foreach value in physical_arg signature_hi_arg signature_lo_arg {
        local parsed = real("``value''")
        if missing(`parsed') | `parsed' < 0 | `parsed' != floor(`parsed') {
            di as err "invalid planned Rust integer argument"
            exit 198
        }
    }
    if real("`physical_arg'") <= 0 |                                  ///
        real("`physical_arg'") > 9007199254740992 |                    ///
        real("`signature_hi_arg'") > 4294967295 |                      ///
        real("`signature_lo_arg'") > 4294967295 {
        di as err "planned Rust physical limit or signature half is out of range"
        exit 198
    }

    fevc__rust_plugin_call `plugin', `solve_selector' `handle' `seed' `probes' ///
        `leveragebatch' `targetbatch' `route' `tolerance_arg' `maxiter' ///
        `algorithm' `deletion' `nuisance' `exactlimit' `blocksizelimit' ///
        `rank_tolerance_arg' `block_tolerance_arg' `engine' `batchmode'  ///
        `stayers' `targetweightmode' `deletionsource'                    ///
        `probeordersupplied' `wallsecondssupplied' `physical_arg'        ///
        `capabilityschema' `capabilityprofile' `frequencyused'           ///
        `signature_hi_arg' `signature_lo_arg' `leveragebatchmode'        ///
        `targetbatchmode' `fallback' `wallseconds_arg' `execution_args'

    return scalar handle = real("`handle'")
    return scalar seed = `seed'
    return scalar probes = `probes'
    return scalar exact_limit = `exactlimit'
    return scalar blocksize_limit = `blocksizelimit'
    return scalar capability_schema = `capabilityschema'
    return scalar capability_profile = `capabilityprofile'
    return scalar request_signature_hi = real("`signature_hi_arg'")
    return scalar request_signature_lo = real("`signature_lo_arg'")
    return scalar physical_limit = real("`physical_arg'")
    return scalar frequency_use_code = `frequencyused'
    return scalar fallback_allowed = `fallback'
    return scalar wallseconds_supplied = `wallsecondssupplied'
    return scalar wallseconds = `wallseconds'
    return local engine "`engine'"
    return local batch_mode "`batchmode'"
    return local leverage_batch_mode "`leveragebatchmode'"
    return local target_batch_mode "`targetbatchmode'"
    return local stayers "`stayers'"
    return local target_weight_mode "`targetweightmode'"
    return local deletion_source "`deletionsource'"
    return local route "`route'"
    return local algorithm "`algorithm'"
    return local deletion "`deletion'"
    return local nuisance "`nuisance'"
    return local backend "rust"
    if `execution' {
        return scalar execution_mode = `execution'
        return scalar threads = `threads'
    }
    return local subcommand "`solve_selector'"
end
