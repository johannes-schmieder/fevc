*! version 0.4.0-alpha.1 22aug2026
capture program drop _vckss_rust_release_idle
program define _vckss_rust_release_idle, rclass
    version 18.0
    args plugin handle
    capture _vckss_rust_plugin_call `plugin', release `handle'
    local release_rc = _rc
    capture _vckss_rust_plugin_call `plugin', snapshot
    local snapshot_rc = _rc
    local certified = 0
    if !`release_rc' & !`snapshot_rc' {
        local certified = scalar(__vckss_rust_state) == 0 &              ///
            scalar(__vckss_rust_handle) == 0 &                           ///
            scalar(__vckss_rust_last_released) == real("`handle'")
    }
    capture scalar drop __vckss_rust_state
    capture scalar drop __vckss_rust_handle
    capture scalar drop __vckss_rust_last_released
    return scalar certified = `certified'
end

program define vckss_rust, rclass
    version 18.0

    gettoken subcommand 0 : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "" {
        di as err "Rust backend subcommand required"
        di as err "valid subcommands: probe, capabilities, requestcapability, version, selftest, prepare, augmentstayers, augmentprojection, solve, result, projectionresult, fullcmgreceipt, stayerresult, snapshot, release, clear, lasterror"
        exit 198
    }

    local plugin
    if strpos("`c(machine_type)'", "Mac") == 1 {
        local plugin _vckss_rust_macos
    }
    else if "`c(os)'" == "Windows" {
        local plugin _vckss_rust_windows
    }
    else if "`c(os)'" == "Unix" {
        local plugin _vckss_rust_linux
    }
    else {
        di as err "unsupported operating system for the Rust backend: `c(os)'"
        exit 198
    }
    capture findfile `plugin'.ado
    if _rc {
        di as err "Rust native plugin loader was not found on the Stata adopath"
        exit 601
    }
    local plugin_loader `"`r(fn)'"'
    quietly run `"`plugin_loader'"'

    if "`subcommand'" == "probe" {
        if strtrim(`"`0'"') != "" {
            di as err "probe does not accept additional arguments"
            exit 198
        }
        _vckss_rust_plugin_call `plugin', probe
        return scalar abi_compiled = scalar(__vckss_rust_abi_compiled)
        return scalar abi_runtime = scalar(__vckss_rust_abi_runtime)
        return scalar core_ready_flags = scalar(__vckss_rust_core_flags)
        return scalar support_flags = scalar(__vckss_rust_support_flags)
        return scalar deterministic_parallelism = scalar(__vckss_rust_deterministic)
        foreach name in abi_compiled abi_runtime core_flags support_flags deterministic {
            capture scalar drop __vckss_rust_`name'
        }
        return local backend "rust"
        return local subcommand "probe"
        exit
    }

    if "`subcommand'" == "lasterror" {
        if strtrim(`"`0'"') != "" {
            di as err "lasterror does not accept additional arguments"
            exit 198
        }
        _vckss_rust_plugin_call `plugin', lasterror
        return scalar native_error_code = scalar(__vckss_rust_error_code)
        return local native_error_status `"`r(native_error_status)'"'
        return local native_error_detail `"`r(native_error_detail)'"'
        capture scalar drop __vckss_rust_error_code
        return local backend "rust"
        return local subcommand "lasterror"
        exit
    }

    if "`subcommand'" == "requestcapability" {
        syntax, ALGORITHM(string) DELETION(string) NUISANCE(string)  ///
            ROUTE(string) RNGCONTRACT(string) CONTROLS(integer)     ///
            FREQUENCYUSED(integer) [ENGINE(string) BATCHMODE(string) ///
            STAYERS(string) TARGETWEIGHTMODE(string)                 ///
            DELETIONSOURCE(string) PROBEORDERSUPPLIED(integer 0)     ///
            WALLSECONDSSUPPLIED(integer 0) PHYSICALLIMIT(real 50000000) ///
            LEVERAGEBATCHMODE(string) TARGETBATCHMODE(string)        ///
            FALLBACK(integer -1) WALLSECONDS(real 0)]
        local algorithm = lower(strtrim("`algorithm'"))
        local deletion = lower(strtrim("`deletion'"))
        local nuisance = lower(strtrim("`nuisance'"))
        local route = lower(strtrim("`route'"))
        local rngcontract = lower(strtrim("`rngcontract'"))
        local engine = lower(strtrim("`engine'"))
        if !inlist("`algorithm'", "auto", "exact", "jla") {
            di as err "algorithm() must be auto, exact, or jla"
            exit 198
        }
        if !inlist("`deletion'", "match", "observation") {
            di as err "deletion() must be match or observation"
            exit 198
        }
        if !inlist("`nuisance'", "joint", "fixedoffset") {
            di as err "nuisance() must be joint or fixedoffset"
            exit 198
        }
        if !inlist("`route'", "auto", "exact", "diagonal", "cmg") {
            di as err "route() must be auto, exact, diagonal, or cmg"
            exit 198
        }
        if !inlist("`rngcontract'", "none", "counter_v1") {
            di as err "rngcontract() must be none or counter_v1"
            exit 198
        }
        if missing(`controls') | `controls' < 0 {
            di as err "controls() must be a nonnegative integer"
            exit 198
        }
        if !inlist(`frequencyused', 0, 1) {
            di as err "frequencyused() must be zero or one"
            exit 198
        }
        local frequency_use unit
        if `frequencyused' == 1 local frequency_use literal
        if "`engine'" == "" {
            _vckss_rust_plugin_call `plugin', requestcapability `algorithm' `deletion' ///
                `nuisance' `route' `rngcontract' `controls' `frequency_use'
        }
        else {
            if !inlist("`engine'", "auto", "compressed", "generic") {
                di as err "engine() must be auto, compressed, or generic"
                exit 198
            }
            local batchmode = lower(strtrim("`batchmode'"))
            if "`batchmode'" == "" local batchmode explicit
            if !inlist("`batchmode'", "auto", "explicit", "independent") {
                di as err "batchmode() must be auto, explicit, or independent"
                exit 198
            }
            local stayers = lower(strtrim("`stayers'"))
            if "`stayers'" == "" local stayers movers
            if !inlist("`stayers'", "movers", "all") {
                di as err "stayers() must be movers or all"
                exit 198
            }
            local targetweightmode = lower(strtrim("`targetweightmode'"))
            if "`targetweightmode'" == "" local targetweightmode frequency
            if !inlist("`targetweightmode'", "frequency", "explicit") {
                di as err "targetweightmode() must be frequency or explicit"
                exit 198
            }
            local deletionsource = lower(strtrim("`deletionsource'"))
            if "`deletionsource'" == "" {
                local deletionsource cell
                if "`deletion'" == "observation" local deletionsource observation
            }
            if !inlist("`deletionsource'", "cell", "matchid", "observation") {
                di as err "deletionsource() must be cell, matchid, or observation"
                exit 198
            }
            if !inlist(`probeordersupplied', 0, 1) |                  ///
                !inlist(`wallsecondssupplied', 0, 1) {
                di as err "probeordersupplied() and wallsecondssupplied() must be zero or one"
                exit 198
            }
            if missing(`physicallimit') | `physicallimit' <= 0 |      ///
                `physicallimit' > 9007199254740992 |                  ///
                `physicallimit' != floor(`physicallimit') {
                di as err "physicallimit() must be an integer in [1,2^53]"
                exit 198
            }
            local physical_arg = strtrim(strofreal(`physicallimit', "%21.0f"))
            local leveragebatchmode = lower(strtrim("`leveragebatchmode'"))
            local targetbatchmode = lower(strtrim("`targetbatchmode'"))
            local planned = "`engine'" != "generic" | "`algorithm'" == "auto" | ///
                "`route'" == "cmg" | "`batchmode'" == "independent" |           ///
                "`leveragebatchmode'" != "" | "`targetbatchmode'" != "" |       ///
                `fallback' >= 0 | `wallsecondssupplied' == 1
            if `planned' {
                if "`leveragebatchmode'" == "" local leveragebatchmode `batchmode'
                if "`targetbatchmode'" == "" local targetbatchmode `batchmode'
                if !inlist("`leveragebatchmode'", "auto", "explicit") |          ///
                    !inlist("`targetbatchmode'", "auto", "explicit") {
                    di as err "planned phase batch modes must be auto or explicit"
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
                if `fallback' < 0 local fallback = ("`route'" == "auto")
                if !inlist(`fallback', 0, 1) | (`fallback' == 1 & "`route'" != "auto") {
                    di as err "fallback() must be zero, or one only with route(auto)"
                    exit 198
                }
                if (`wallsecondssupplied' == 0 & `wallseconds' != 0) |       ///
                    (`wallsecondssupplied' == 1 &                           ///
                        (missing(`wallseconds') | `wallseconds' <= 0)) {
                    di as err "wallseconds() must be zero when omitted and finite positive when supplied"
                    exit 198
                }
                local wallseconds_arg = strtrim(strofreal(`wallseconds', "%21.17g"))
                _vckss_rust_plugin_call `plugin', requestcapability `algorithm' `deletion' ///
                    `nuisance' `route' `rngcontract' `controls' `frequency_use' ///
                    `engine' `batchmode' `stayers' `targetweightmode'          ///
                    `deletionsource' `probeordersupplied' `wallsecondssupplied' ///
                    `physical_arg' `leveragebatchmode' `targetbatchmode'        ///
                    `fallback' `wallseconds_arg'
            }
            else {
                _vckss_rust_plugin_call `plugin', requestcapability `algorithm' `deletion' ///
                    `nuisance' `route' `rngcontract' `controls' `frequency_use' ///
                    `engine' `batchmode' `stayers' `targetweightmode'          ///
                    `deletionsource' `probeordersupplied' `wallsecondssupplied' ///
                    `physical_arg'
            }
        }

        return scalar struct_size = scalar(__vckss_rust_cap_struct)
        return scalar abi_version = scalar(__vckss_rust_cap_abi)
        return scalar request_schema = scalar(__vckss_rust_cap_schema)
        return scalar supported = scalar(__vckss_rust_cap_supported)
        return scalar reason_code = scalar(__vckss_rust_cap_reason)
        return scalar profile_code = scalar(__vckss_rust_cap_profile)
        return scalar algorithm_code = scalar(__vckss_rust_cap_algorithm)
        return scalar deletion_mode_code = scalar(__vckss_rust_cap_deletion)
        return scalar nuisance_mode_code = scalar(__vckss_rust_cap_nuisance)
        return scalar solver_route_code = scalar(__vckss_rust_cap_route)
        return scalar rng_contract_code = scalar(__vckss_rust_cap_rng)
        return scalar controls_count = scalar(__vckss_rust_cap_controls)
        return scalar frequency_use_code = scalar(__vckss_rust_cap_frequency)
        return scalar request_signature_hi = scalar(__vckss_rust_cap_signature_hi)
        return scalar request_signature_lo = scalar(__vckss_rust_cap_signature_lo)
        if "`engine'" != "" {
            return scalar engine_code = scalar(__vckss_rust_cap_engine)
            return scalar batch_mode_code = scalar(__vckss_rust_cap_batch)
            return scalar stayers_mode_code = scalar(__vckss_rust_cap_stayers)
            return scalar target_weight_mode_code = scalar(__vckss_rust_cap_target)
            return scalar deletion_source_code = scalar(__vckss_rust_cap_delsource)
            return scalar probeorder_supplied = scalar(__vckss_rust_cap_probeorder)
            return scalar wallseconds_supplied = scalar(__vckss_rust_cap_wall)
            return scalar physical_limit = scalar(__vckss_rust_cap_physlimit)
            if `planned' {
                return scalar leverage_batch_mode_code = scalar(__vckss_rust_cap_levmode)
                return scalar target_batch_mode_code = scalar(__vckss_rust_cap_tgtmode)
                return scalar automatic_fallback_allowed = scalar(__vckss_rust_cap_autofallback)
                return scalar wallseconds = scalar(__vckss_rust_cap_wallseconds)
                return scalar algorithm_resolution_deferred = scalar(__vckss_rust_cap_alg_deferred)
                return scalar engine_resolution_deferred = scalar(__vckss_rust_cap_eng_deferred)
                return scalar route_resolution_deferred = scalar(__vckss_rust_cap_route_deferred)
                return scalar leverage_batch_deferred = scalar(__vckss_rust_cap_lev_deferred)
                return scalar target_batch_resolution_deferred = scalar(__vckss_rust_cap_tgt_deferred)
                return scalar wall_advisory_only = scalar(__vckss_rust_cap_wall_advisory)
            }
        }
        local reason_code = scalar(__vckss_rust_cap_reason)
        local profile_code = scalar(__vckss_rust_cap_profile)
        local reason SUPPORTED
        if `reason_code' == 1 local reason UNKNOWN_SCHEMA
        else if `reason_code' == 2 local reason UNKNOWN_ALGORITHM
        else if `reason_code' == 3 local reason ALGORITHM_AUTO_UNRESOLVED
        else if `reason_code' == 4 local reason UNKNOWN_DELETION
        else if `reason_code' == 5 local reason UNKNOWN_NUISANCE
        else if `reason_code' == 6 local reason UNKNOWN_SOLVER_ROUTE
        else if `reason_code' == 7 local reason UNKNOWN_RNG_CONTRACT
        else if `reason_code' == 8 local reason UNKNOWN_FREQUENCY_USE
        else if `reason_code' == 9 local reason CONTROLS_LIMIT
        else if `reason_code' == 10 local reason EXACT_RNG_NOT_NONE
        else if `reason_code' == 11 local reason EXACT_SOLVER_ROUTE
        else if `reason_code' == 12 local reason JLA_DELETION
        else if `reason_code' == 13 local reason JLA_NUISANCE
        else if `reason_code' == 14 local reason JLA_CONTROLS
        else if `reason_code' == 15 local reason JLA_RNG
        else if `reason_code' == 16 local reason UNKNOWN_ENGINE
        else if `reason_code' == 17 local reason EXACT_ENGINE
        else if `reason_code' == 18 local reason JLA_ENGINE_AUTO_UNRESOLVED
        else if `reason_code' == 19 local reason JLA_GENERIC_SOLVER_ROUTE
        else if `reason_code' == 20 local reason UNKNOWN_BATCH_MODE
        else if `reason_code' == 21 local reason UNKNOWN_STAYERS_MODE
        else if `reason_code' == 22 local reason UNKNOWN_TARGET_WEIGHT_MODE
        else if `reason_code' == 23 local reason UNKNOWN_DELETION_UNIT_SOURCE
        else if `reason_code' == 24 local reason PROBEORDER_UNSUPPORTED
        else if `reason_code' == 25 local reason WALLSECONDS_UNSUPPORTED
        else if `reason_code' == 26 local reason PHYSICAL_LIMIT
        else if `reason_code' == 27 local reason DELETION_UNIT_SOURCE_MISMATCH
        else if `reason_code' == 28 local reason BATCH_MODE_UNSUPPORTED
        else if `reason_code' == 29 local reason STAYERS_MODE_UNSUPPORTED
        else if `reason_code' == 30 local reason BATCH_SUMMARY_MISMATCH
        else if `reason_code' == 31 local reason WALLSECONDS_VALUE
        else if `reason_code' == 32 local reason FALLBACK_ROUTE_MISMATCH
        else if `reason_code' == 33 local reason AUTO_RNG
        else if `reason_code' == 34 local reason AUTO_ENGINE_COMPRESSED
        else if `reason_code' == 35 local reason COMPRESSED_SCIENTIFIC_INELIGIBILITY
        else if `reason_code' != 0 local reason UNKNOWN_REASON
        local profile NONE
        if `profile_code' == 1 local profile EXACT_V1
        else if `profile_code' == 2 local profile JLA_COUNTER_V1
        else if `profile_code' == 3 local profile JLA_GENERIC_COUNTER_V1
        else if `profile_code' == 4 local profile PLANNED_V1
        return local reason "`reason'"
        return local profile "`profile'"
        return local algorithm "`algorithm'"
        return local deletion "`deletion'"
        return local nuisance "`nuisance'"
        return local route "`route'"
        return local rng_contract "`rngcontract'"
        return local frequency_use "`frequency_use'"
        if "`engine'" != "" {
            return local engine "`engine'"
            return local batch_mode "`batchmode'"
            return local stayers "`stayers'"
            return local target_weight_mode "`targetweightmode'"
            return local deletion_source "`deletionsource'"
            if `planned' {
                return local leverage_batch_mode "`leveragebatchmode'"
                return local target_batch_mode "`targetbatchmode'"
            }
        }
        return local backend "rust"
        return local subcommand "requestcapability"
        foreach name in struct abi schema supported reason profile algorithm ///
            deletion nuisance route rng controls frequency signature_hi signature_lo {
            capture scalar drop __vckss_rust_cap_`name'
        }
        foreach name in engine batch stayers target delsource probeorder wall physlimit {
            capture scalar drop __vckss_rust_cap_`name'
        }
        foreach name in levmode tgtmode autofallback wallseconds alg_deferred  ///
            eng_deferred route_deferred lev_deferred tgt_deferred wall_advisory {
            capture scalar drop __vckss_rust_cap_`name'
        }
        exit
    }

    if inlist("`subcommand'", "capabilities", "version", "selftest", "clear") {
        if strtrim(`"`0'"') != "" {
            di as err "`subcommand' does not accept additional arguments"
            exit 198
        }
        _vckss_rust_plugin_call `plugin', `subcommand'
        return local backend "rust"
        return local subcommand "`subcommand'"
        exit
    }

    if "`subcommand'" == "snapshot" {
        if strtrim(`"`0'"') != "" {
            di as err "snapshot does not accept additional arguments"
            exit 198
        }
        _vckss_rust_plugin_call `plugin', snapshot
        return scalar state = scalar(__vckss_rust_state)
        return scalar handle = scalar(__vckss_rust_handle)
        return scalar last_released = scalar(__vckss_rust_last_released)
        capture scalar drop __vckss_rust_state
        capture scalar drop __vckss_rust_handle
        capture scalar drop __vckss_rust_last_released
        return local backend "rust"
        return local subcommand "snapshot"
        exit
    }

    if "`subcommand'" == "augmentstayers" {
        syntax varlist(min=5 numeric) [if] [in], HANDLE(integer)
        if `handle' <= 0 {
            di as err "augmentstayers requires one positive integer native generation"
            exit 198
        }
        local var_count : word count `varlist'
        local controls_count = `var_count' - 5
        marksample touse, novarlist
        markout `touse' `varlist'
        capture noisily _vckss_rust_plugin_call `plugin' `touse' `varlist' ///
            if `touse', augmentstayers `handle' `controls_count'
        if _rc exit _rc

        foreach pair in hyb_aug_struct:struct_size                 ///
            hyb_aug_schema:schema_version                          ///
            hyb_mover_rows:mover_stored_rows                      ///
            hyb_stayer_rows:stayer_stored_rows                    ///
            hyb_total_rows:combined_stored_rows                   ///
            hyb_mover_mass:mover_physical_mass                    ///
            hyb_stayer_mass:stayer_physical_mass                  ///
            hyb_total_mass:combined_physical_mass                 ///
            hyb_mover_workers:mover_workers                       ///
            hyb_stayer_workers:stayer_workers                     ///
            hyb_total_workers:combined_workers                    ///
            hyb_firms:firms hyb_mover_del:mover_deletion_units    ///
            hyb_stayer_del:stayer_deletion_units                  ///
            hyb_total_del:combined_deletion_units                 ///
            hyb_mover_target:mover_target_mass                    ///
            hyb_stayer_target:stayer_target_mass                  ///
            hyb_total_target:combined_target_mass                 ///
            hyb_topology_hi:topology_checksum_hi                  ///
            hyb_topology_lo:topology_checksum_lo                  ///
            hyb_mem_limit:memory_limit_bytes                      ///
            hyb_caller_copy:caller_copy_bytes                     ///
            hyb_aug_peak:augmentation_peak_forecast_bytes         ///
            hyb_aug_resident:augmented_resident_bytes             ///
            hyb_prepared_bytes:total_prepared_resident_bytes {
            gettoken source target : pair, parse(":")
            gettoken colon target : target, parse(":")
            return scalar `target' = scalar(__vckss_`source')
        }
        return scalar handle = `handle'
        return scalar controls_count = `controls_count'
        return local backend "rust"
        return local subcommand "augmentstayers"
        foreach name in aug_struct aug_schema mover_rows stayer_rows total_rows ///
            mover_mass stayer_mass total_mass mover_workers stayer_workers     ///
            total_workers firms mover_del stayer_del total_del mover_target    ///
            stayer_target total_target topology_hi topology_lo mem_limit       ///
            caller_copy aug_peak aug_resident prepared_bytes {
            capture scalar drop __vckss_hyb_`name'
        }
        exit
    }

    if "`subcommand'" == "augmentprojection" {
        syntax varlist(min=1 numeric) [if] [in], HANDLE(integer)   ///
            PROJECTEFFECT(string) PROJECTWEIGHT(string) RANKTOLerance(real)
        if `handle' <= 0 {
            di as err "augmentprojection requires one positive integer native generation"
            exit 198
        }
        local effect = lower(strtrim("`projecteffect'"))
        local projection_weight = lower(strtrim("`projectweight'"))
        if !inlist("`effect'", "worker", "firm") |              ///
            !inlist("`projection_weight'", "frequency", "target") | ///
            missing(`ranktolerance') | `ranktolerance' < 1e-14 |  ///
            `ranktolerance' >= .1 {
            di as err "invalid projection effect, weight, or rank tolerance"
            exit 198
        }
        local project_count : word count `varlist'
        local rank_tolerance_arg : display %21.17f `ranktolerance'
        local rank_tolerance_arg = strtrim("`rank_tolerance_arg'")
        marksample touse, novarlist
        markout `touse' `varlist'
        capture noisily _vckss_rust_plugin_call `plugin' `touse' `varlist' ///
            if `touse', augmentprojection `handle' `project_count'  ///
            `effect' `projection_weight' `rank_tolerance_arg'
        if _rc exit _rc
        foreach pair in proj_aug_schema:schema_version              ///
            proj_rows:rows proj_columns:columns proj_effect:effect_code ///
            proj_weight:weight_code proj_copy:caller_copy_bytes     ///
            proj_aug_peak:augmentation_peak_forecast_bytes          ///
            proj_persistent:projection_persistent_bytes             ///
            proj_prepared:total_prepared_resident_bytes             ///
            proj_gram_rcond:gram_rcond proj_gram_relres:gram_relres ///
            proj_gram_orig:gram_original_relres {
            gettoken source target : pair, parse(":")
            gettoken colon target : target, parse(":")
            return scalar `target' = scalar(__vckss_`source')
        }
        return scalar handle = `handle'
        return scalar project_count = `project_count'
        return local effect "`effect'"
        return local weight "`projection_weight'"
        return local backend "rust"
        return local subcommand "augmentprojection"
        foreach name in aug_schema rows columns effect weight copy aug_peak ///
            persistent prepared gram_rcond gram_relres gram_orig {
            capture scalar drop __vckss_proj_`name'
        }
        exit
    }

    if "`subcommand'" == "projectionresult" {
        gettoken handle 0 : 0, parse(" ,")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "projectionresult requires one positive integer native generation"
            exit 198
        }
        syntax, COLUMNS(integer)
        if `columns' <= 0 | `columns' > c(max_matdim) {
            di as err "columns() must be in [1,c(max_matdim)]"
            exit 198
        }
        tempname coefficients covariance naive_covariance
        capture matrix `coefficients' = J(1,`columns',.)
        local allocation_rc = _rc
        if !`allocation_rc' capture matrix `covariance' = J(`columns',`columns',.)
        if !`allocation_rc' local allocation_rc = _rc
        if !`allocation_rc' capture matrix `naive_covariance' = J(`columns',`columns',.)
        if !`allocation_rc' local allocation_rc = _rc
        if `allocation_rc' {
            di as err "Stata could not allocate the projection result matrices"
            exit `allocation_rc'
        }
        _vckss_rust_plugin_call `plugin', projectionresult `handle' `columns' ///
            `coefficients' `covariance' `naive_covariance'
        return matrix coefficients = `coefficients'
        return matrix covariance = `covariance'
        return matrix naive_covariance = `naive_covariance'
        foreach pair in proj_result_schema:schema_version          ///
            proj_result_columns:columns proj_result_effect:effect_code ///
            proj_result_weight:weight_code proj_cov_min:covariance_minimum_eigenvalue ///
            proj_cov_max:covariance_maximum_eigenvalue             ///
            proj_psd_cleanup:psd_cleanup proj_proxy_min:proxy_minimum ///
            proj_proxy_max:proxy_maximum proj_max_iter:maximum_iterations ///
            proj_max_reduced:maximum_reduced_residual              ///
            proj_max_complete:maximum_complete_residual            ///
            proj_full_tol:full_residual_tolerance                  ///
            proj_peak:projection_peak_forecast_bytes               ///
            proj_result_bytes:result_bytes {
            gettoken source target : pair, parse(":")
            gettoken colon target : target, parse(":")
            return scalar `target' = scalar(__vckss_`source')
        }
        return scalar handle = real("`handle'")
        return local backend "rust"
        return local subcommand "projectionresult"
        foreach name in result_schema result_columns result_effect result_weight ///
            cov_min cov_max psd_cleanup proxy_min proxy_max max_iter     ///
            max_reduced max_complete full_tol peak result_bytes {
            capture scalar drop __vckss_proj_`name'
        }
        exit
    }

    if "`subcommand'" == "stayerresult" {
        syntax anything(name=handle id="native Rust generation")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "stayerresult requires one positive integer native generation"
            exit 198
        }
        _vckss_rust_plugin_call `plugin', stayerresult `handle'
        tempname result sources
        capture matrix `result' =                                      ///
            (scalar(__vckss_hyb_plugin_worker),                         ///
             scalar(__vckss_hyb_plugin_firm),                           ///
             scalar(__vckss_hyb_plugin_cov),                            ///
             scalar(__vckss_hyb_plugin_total) \                         ///
             scalar(__vckss_hyb_correction_worker),                     ///
             scalar(__vckss_hyb_correction_firm),                       ///
             scalar(__vckss_hyb_correction_cov),                        ///
             scalar(__vckss_hyb_correction_total) \                     ///
             scalar(__vckss_hyb_corrected_worker),                      ///
             scalar(__vckss_hyb_corrected_firm),                        ///
             scalar(__vckss_hyb_corrected_cov),                         ///
             scalar(__vckss_hyb_corrected_total) \ 0,0,0,0)
        local matrix_rc = _rc
        if !`matrix_rc' {
            capture matrix `sources' =                                 ///
                (scalar(__vckss_hyb_mover_worker),                      ///
                 scalar(__vckss_hyb_mover_firm),                        ///
                 scalar(__vckss_hyb_mover_cov),                         ///
                 scalar(__vckss_hyb_mover_total) \                      ///
                 scalar(__vckss_hyb_stayer_worker),                     ///
                 scalar(__vckss_hyb_stayer_firm),                       ///
                 scalar(__vckss_hyb_stayer_cov),                        ///
                 scalar(__vckss_hyb_stayer_total))
            local matrix_rc = _rc
        }
        if `matrix_rc' {
            quietly _vckss_rust_release_idle `plugin' `handle'
            di as err "Stata could not allocate the Rust stayer-hybrid result matrices"
            exit `matrix_rc'
        }
        matrix rownames `result' = plugin bias_correction corrected numerical_mcse
        matrix colnames `result' = worker_variance firm_variance       ///
            worker_firm_covariance total_variance
        matrix rownames `sources' = mover_match stayer_observation
        matrix colnames `sources' = worker_variance firm_variance      ///
            worker_firm_covariance total_variance
        return matrix result = `result'
        return matrix correction_source = `sources'
        foreach pair in hyb_result_schema:schema_version               ///
            hyb_weighted_rss:weighted_rss hyb_parameters:parameters   ///
            hyb_full_parameters:full_parameters                       ///
            hyb_corr_parameters:correction_parameters                 ///
            hyb_result_del:deletion_units                             ///
            hyb_max_leverage:max_leverage hyb_info_rcond:information_rcond ///
            hyb_inverse:inverse_relative_residual                     ///
            hyb_inverse_original:inverse_original_relres              ///
            hyb_inverse_sqrt:inverse_sqrt_relative_residual           ///
            hyb_maker:maker_relative_residual                         ///
            hyb_full_fit:full_fit_relative_residual                   ///
            hyb_working_fit:working_fit_relative_residual             ///
            hyb_fit_tolerance:fit_residual_tolerance                  ///
            hyb_control_relres:control_basis_relative_residual        ///
            hyb_control_fwd:control_basis_forward_error               ///
            hyb_delete_gap:deletion_rank_gap                          ///
            hyb_firm_zero_sum:firm_zero_sum_residual                  ///
            hyb_result_peak:peak_forecast_bytes                       ///
            hyb_fit_peak:fit_peak_forecast_bytes                     ///
            hyb_corr_peak:correction_peak_forecast_bytes             ///
            hyb_accounting:accounting_residual                        ///
            hyb_source_resid:source_accounting_residual {
            gettoken source target : pair, parse(":")
            gettoken colon target : target, parse(":")
            return scalar `target' = scalar(__vckss_`source')
        }
        return scalar handle = real("`handle'")
        return local backend "rust"
        return local subcommand "stayerresult"
        foreach prefix in hyb_plugin hyb_correction hyb_corrected      ///
            hyb_mover hyb_stayer {
            foreach component in worker firm cov total {
                capture scalar drop __vckss_`prefix'_`component'
            }
        }
        foreach name in result_schema weighted_rss parameters          ///
            full_parameters corr_parameters result_del max_leverage   ///
            info_rcond inverse inverse_original inverse_sqrt maker     ///
            full_fit working_fit fit_tolerance control_relres          ///
            control_fwd delete_gap firm_zero_sum result_peak fit_peak  ///
            corr_peak accounting source_resid {
            capture scalar drop __vckss_hyb_`name'
        }
        exit
    }

    if inlist("`subcommand'", "release", "result") {
        syntax anything(name=handle id="native Rust generation")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "`subcommand' requires one positive integer native generation"
            exit 198
        }
        if "`subcommand'" == "release" {
            _vckss_rust_plugin_call `plugin', release `handle'
            return scalar handle = real("`handle'")
            return local backend "rust"
            return local subcommand "release"
            exit
        }

        _vckss_rust_plugin_call `plugin', result `handle'
        local rhs_rows = scalar(__vckss_rust_rhs_rows)
        local rhs_schema = scalar(__vckss_rust_rhs_schema)
        local rhs_v1_copy = scalar(__vckss_rust_rhs_copy)
        local rhs_v2_copy = scalar(__vckss_rust_rhs_v2_copy)
        local projection_columns = scalar(__vckss_rust_proj_columns)
        local projection_peak = scalar(__vckss_rust_proj_peak)
        if missing(`rhs_rows') | `rhs_rows' < 0 |               ///
            `rhs_rows' != floor(`rhs_rows') |                   ///
            `rhs_rows' > c(max_matdim) {
            quietly _vckss_rust_release_idle `plugin' `handle'
            local cleanup_certified = r(certified)
            di as err "Rust returned an invalid RHS receipt row count"
            if !`cleanup_certified' {
                di as err "Rust result cleanup did not certify an idle native session"
            }
            exit 498
        }
        if !inlist(`rhs_schema', 0, 1, 2) |                             ///
            (`rhs_schema' == 0 & (`rhs_rows' != 0 |                     ///
                `rhs_v1_copy' != 0 | `rhs_v2_copy' != 0)) |             ///
            (`rhs_schema' == 1 & (`rhs_rows' <= 0 |                     ///
                `rhs_v1_copy' != `rhs_rows' * 112 | `rhs_v2_copy' != 0)) | ///
            (`rhs_schema' == 2 & (`rhs_rows' <= 0 |                     ///
                `rhs_v1_copy' != 0 | `rhs_v2_copy' != `rhs_rows' * 216)) {
            quietly _vckss_rust_release_idle `plugin' `handle'
            local cleanup_certified = r(certified)
            di as err "Rust returned an invalid RHS receipt schema/count/copy tuple"
            if !`cleanup_certified' {
                di as err "Rust result cleanup did not certify an idle native session"
            }
            exit 498
        }
        if missing(`projection_columns') | `projection_columns' < 0 |    ///
            `projection_columns' != floor(`projection_columns') |        ///
            `projection_columns' > c(max_matdim) |                       ///
            missing(`projection_peak') | `projection_peak' < 0 |         ///
            `projection_peak' != floor(`projection_peak') |              ///
            (`projection_columns' == 0 & `projection_peak' != 0) |       ///
            (`projection_columns' > 0 & `projection_peak' == 0) {
            quietly _vckss_rust_release_idle `plugin' `handle'
            di as err "Rust returned an invalid projection result dimension or memory receipt"
            exit 498
        }
        tempname rhs_receipts
        if `rhs_rows' > 0 {
            local rhs_columns = cond(`rhs_schema' == 2, 15, 8)
            capture matrix `rhs_receipts' = J(`rhs_rows',`rhs_columns',.)
            local rhs_allocation_rc = _rc
            if `rhs_allocation_rc' {
                quietly _vckss_rust_release_idle `plugin' `handle'
                local cleanup_certified = r(certified)
                di as err "Stata could not allocate the Rust RHS receipt matrix"
                if !`cleanup_certified' {
                    di as err "Rust result cleanup did not certify an idle native session"
                }
                exit `rhs_allocation_rc'
            }
            _vckss_rust_plugin_call `plugin', rhsresult `handle' `rhs_receipts'
            if `rhs_schema' == 2 {
                matrix colnames `rhs_receipts' = phase probe side route iterations ///
                    reduced_residual complete_residual zero_rhs status replacements ///
                    operator_applications preconditioner_applications tolerance      ///
                    residual_space solver_dimension
            }
            else {
                matrix colnames `rhs_receipts' = phase probe side route iterations ///
                    reduced_residual complete_residual zero_rhs
            }
        }
        tempname estimates
        capture matrix `estimates' =                                 ///
            (scalar(__vckss_plugin_worker),                           ///
             scalar(__vckss_plugin_firm),                             ///
             scalar(__vckss_plugin_cov),                              ///
             scalar(__vckss_plugin_total) \                           ///
             scalar(__vckss_correction_worker),                       ///
             scalar(__vckss_correction_firm),                         ///
             scalar(__vckss_correction_cov),                          ///
             scalar(__vckss_correction_total) \                       ///
             scalar(__vckss_corrected_worker),                        ///
             scalar(__vckss_corrected_firm),                          ///
             scalar(__vckss_corrected_cov),                           ///
             scalar(__vckss_corrected_total) \                        ///
             scalar(__vckss_mcse_worker),                             ///
             scalar(__vckss_mcse_firm),                               ///
             scalar(__vckss_mcse_cov),                                ///
             scalar(__vckss_mcse_total))
        local estimates_allocation_rc = _rc
        if `estimates_allocation_rc' {
            quietly _vckss_rust_release_idle `plugin' `handle'
            local cleanup_certified = r(certified)
            di as err "Stata could not allocate the Rust result matrix"
            if !`cleanup_certified' {
                di as err "Rust result cleanup did not certify an idle native session"
            }
            exit `estimates_allocation_rc'
        }
        matrix rownames `estimates' = plugin bias_correction corrected numerical_mcse
        matrix colnames `estimates' = worker_variance firm_variance          ///
            worker_firm_covariance total_variance

        local engine_requested = scalar(__vckss_rust_engine_requested)
        local engine_selected = scalar(__vckss_rust_engine_selected)
        local generic_controls = scalar(__vckss_rust_generic_controls)
        local control_rhs = scalar(__vckss_rust_g_control_rhs)
        local capability_schema = scalar(__vckss_rust_cap_schema_echo)
        local capability_profile = scalar(__vckss_rust_cap_profile_echo)
        local native_deletion = scalar(__vckss_rust_deletion_mode)
        local native_nuisance = scalar(__vckss_rust_nuisance_mode)
        local native_algorithm = scalar(__vckss_rust_algorithm_sel)
        local leverage_rhs = scalar(__vckss_rust_lev_rhs)
        local target_rhs = scalar(__vckss_rust_tgt_rhs)
        local signature_hi = scalar(__vckss_rust_solve_signature_hi)
        local signature_lo = scalar(__vckss_rust_solve_signature_lo)
        if `capability_schema' == 3 {
            capture noisily _vckss_rust_plan_receipt
            local plan_rc = _rc
            if `plan_rc' {
                quietly _vckss_rust_release_idle `plugin' `handle'
                local cleanup_certified = r(certified)
                if !`cleanup_certified' {
                    di as err "Rust V7 cleanup did not certify an idle native session"
                }
                exit `plan_rc'
            }
            return add
        }
        local receipt_mismatch = 0
        if `rhs_schema' == 2 {
            local distinct_working = (`native_nuisance' == 2 & `generic_controls' > 0)
            local expected_rhs = `generic_controls' + 1 + `distinct_working' + ///
                `leverage_rhs' + `target_rhs' + `projection_columns'
            local full_solver_dimension = scalar(__vckss_rust_solver_dimension)
            local firm_solver_dimension = `full_solver_dimension' - `generic_controls'
            if !((`capability_schema' == 2 & `engine_requested' == 2) |        ///
                 (`capability_schema' == 3 &                                  ///
                    inlist(`engine_requested', 0, 2))) |                      ///
                `engine_selected' != 2 | !inlist(`capability_schema', 2, 3) | ///
                (`capability_schema' == 2 & `capability_profile' != 3) |      ///
                (`capability_schema' == 3 & `capability_profile' != 4) |      ///
                `native_algorithm' != 2 | !inlist(`native_deletion', 1, 2) |  ///
                !inlist(`native_nuisance', 1, 2) |                            ///
                `generic_controls' != `control_rhs' |                        ///
                `rhs_rows' != `expected_rhs' |                               ///
                `rhs_v1_copy' != 0 | `rhs_v2_copy' != `rhs_rows' * 216 |     ///
                scalar(__vckss_rust_g_result_bytes) <                        ///
                    `rhs_v2_copy' |                                         ///
                scalar(__vckss_rust_result_bytes) !=                         ///
                    scalar(__vckss_rust_g_result_bytes) |                    ///
                scalar(__vckss_rust_solve_peak) !=                           ///
                    scalar(__vckss_rust_g_peak) |                            ///
                scalar(__vckss_rust_command_peak) != max(                   ///
                    scalar(__vckss_rust_prepare_peak),                       ///
                    scalar(__vckss_rust_g_peak)) |                           ///
                scalar(__vckss_rust_g_peak) != max(                         ///
                    scalar(__vckss_rust_g_canon_peak),                       ///
                    scalar(__vckss_rust_g_fit_peak),                         ///
                    scalar(__vckss_rust_g_geometry_peak),                    ///
                    scalar(__vckss_rust_g_lev_peak),                         ///
                    scalar(__vckss_rust_g_tgt_peak),                         ///
                    `projection_peak',                                       ///
                    scalar(__vckss_rust_g_maker_peak),                       ///
                    scalar(__vckss_rust_g_result_bytes)) |                   ///
                missing(`full_solver_dimension') |                          ///
                `full_solver_dimension' != floor(`full_solver_dimension') |  ///
                `firm_solver_dimension' <= 0 |                               ///
                scalar(__vckss_rust_full_complete) !=                        ///
                    scalar(__vckss_rust_g_full_joint) |                      ///
                scalar(__vckss_rust_max_reciprocal) !=                       ///
                    scalar(__vckss_rust_g_maker) |                           ///
                missing(`signature_hi') | missing(`signature_lo') |          ///
                `signature_hi' < 0 | `signature_hi' > 4294967295 |           ///
                `signature_lo' < 0 | `signature_lo' > 4294967295 {
                local receipt_mismatch = 1
            }
            if !`receipt_mismatch' {
                tempname control_projection_max
                scalar `control_projection_max' = 0
                if `generic_controls' > 0 {
                    forvalues row = 1/`generic_controls' {
                        if el(`rhs_receipts', `row', 1) != 5 |                ///
                            el(`rhs_receipts', `row', 2) != `row' - 1 |       ///
                            el(`rhs_receipts', `row', 3) != 0 |               ///
                            el(`rhs_receipts', `row', 13) !=                  ///
                                scalar(__vckss_rust_cr_resid_gate) |         ///
                            el(`rhs_receipts', `row', 14) != 1 |              ///
                            el(`rhs_receipts', `row', 15) !=                  ///
                                `firm_solver_dimension' {
                            local receipt_mismatch = 1
                        }
                        scalar `control_projection_max' = max(               ///
                            scalar(`control_projection_max'),                ///
                            el(`rhs_receipts', `row', 7))
                    }
                }
                if missing(scalar(__vckss_rust_cr_rcond)) |                 ///
                    missing(scalar(__vckss_rust_cr_small_lo)) |             ///
                    missing(scalar(__vckss_rust_cr_large_hi)) |             ///
                    missing(scalar(__vckss_rust_cr_proj_err)) |             ///
                    missing(scalar(__vckss_rust_cr_norm_err)) |             ///
                    missing(scalar(__vckss_rust_cr_fe_lo)) |                ///
                    missing(scalar(__vckss_rust_cr_max_proj)) |             ///
                    missing(scalar(__vckss_rust_cr_tol)) |                  ///
                    missing(scalar(__vckss_rust_cr_pcg_tol)) |              ///
                    missing(scalar(__vckss_rust_cr_resid_gate)) |           ///
                    scalar(__vckss_rust_cr_tol) <= 0 |                      ///
                    scalar(__vckss_rust_cr_pcg_tol) <= 0 |                  ///
                    scalar(__vckss_rust_cr_resid_gate) <= 0 |               ///
                    scalar(__vckss_rust_cr_pcg_tol) >                       ///
                        scalar(__vckss_rust_cr_resid_gate) {
                    local receipt_mismatch = 1
                }
                if `generic_controls' > 0 {
                    if scalar(__vckss_rust_cr_small_lo) <= 0 |              ///
                        scalar(__vckss_rust_cr_large_hi) <= 0 |             ///
                        scalar(__vckss_rust_cr_small_lo) >                  ///
                            scalar(__vckss_rust_cr_large_hi) |              ///
                        scalar(__vckss_rust_cr_rcond) !=                    ///
                            scalar(__vckss_rust_cr_small_lo) /              ///
                            scalar(__vckss_rust_cr_large_hi) |              ///
                        scalar(__vckss_rust_cr_rcond) <=                    ///
                            scalar(__vckss_rust_cr_tol) |                   ///
                        scalar(__vckss_rust_cr_proj_err) < 0 |              ///
                        scalar(__vckss_rust_cr_norm_err) < 0 |              ///
                        scalar(__vckss_rust_cr_norm_err) >= .25 |           ///
                        scalar(__vckss_rust_cr_fe_lo) <= 0 |                ///
                        scalar(__vckss_rust_cr_max_proj) < 0 |              ///
                        scalar(__vckss_rust_cr_max_proj) >                  ///
                            scalar(__vckss_rust_cr_resid_gate) |            ///
                        scalar(__vckss_rust_cr_max_proj) !=                 ///
                            scalar(`control_projection_max') {
                        local receipt_mismatch = 1
                    }
                }
                else if scalar(__vckss_rust_cr_rcond) != 1 |               ///
                    scalar(__vckss_rust_cr_small_lo) != 1 |                 ///
                    scalar(__vckss_rust_cr_large_hi) != 1 |                 ///
                    scalar(__vckss_rust_cr_proj_err) != 0 |                 ///
                    scalar(__vckss_rust_cr_norm_err) != 0 |                 ///
                    scalar(__vckss_rust_cr_fe_lo) != 0 |                    ///
                    scalar(__vckss_rust_cr_max_proj) != 0 |                 ///
                    scalar(`control_projection_max') != 0 {
                    local receipt_mismatch = 1
                }
                local row = `generic_controls' + 1
                if el(`rhs_receipts', `row', 1) != 1 |                       ///
                    el(`rhs_receipts', `row', 2) != -1 |                     ///
                    el(`rhs_receipts', `row', 3) != 0 |                       ///
                    el(`rhs_receipts', `row', 14) != 2 |                      ///
                    el(`rhs_receipts', `row', 15) != `full_solver_dimension' | ///
                    el(`rhs_receipts', `row', 5) !=                           ///
                        scalar(__vckss_rust_full_iterations) |                ///
                    el(`rhs_receipts', `row', 6) !=                           ///
                        scalar(__vckss_rust_full_reduced) |                   ///
                    el(`rhs_receipts', `row', 7) !=                           ///
                        scalar(__vckss_rust_full_complete) |                  ///
                    el(`rhs_receipts', `row', 8) !=                           ///
                        scalar(__vckss_rust_full_zero) |                      ///
                    el(`rhs_receipts', `row', 13) !=                          ///
                        scalar(__vckss_rust_full_tolerance) {
                    local receipt_mismatch = 1
                }
                local row = `row' + 1
                if `distinct_working' {
                    if el(`rhs_receipts', `row', 1) != 4 |                   ///
                        el(`rhs_receipts', `row', 2) != -1 |                 ///
                        el(`rhs_receipts', `row', 3) != 0 |                   ///
                        el(`rhs_receipts', `row', 14) != 1 |                  ///
                        el(`rhs_receipts', `row', 15) !=                      ///
                            `firm_solver_dimension' |                         ///
                        el(`rhs_receipts', `row', 13) !=                      ///
                            scalar(__vckss_rust_full_tolerance) |             ///
                        el(`rhs_receipts', `row', 7) !=                       ///
                            scalar(__vckss_rust_g_working_fit) {
                        local receipt_mismatch = 1
                    }
                    local row = `row' + 1
                }
                forvalues probe = 0/`=`leverage_rhs' - 1' {
                    if el(`rhs_receipts', `row', 1) != 2 |                   ///
                        el(`rhs_receipts', `row', 2) != `probe' |             ///
                        el(`rhs_receipts', `row', 3) != 0 |                   ///
                        el(`rhs_receipts', `row', 14) != 1 |                  ///
                        el(`rhs_receipts', `row', 15) !=                      ///
                            `firm_solver_dimension' |                         ///
                        el(`rhs_receipts', `row', 13) !=                      ///
                            scalar(__vckss_rust_full_tolerance) {
                        local receipt_mismatch = 1
                    }
                    local row = `row' + 1
                }
                forvalues target = 0/`=`target_rhs' - 1' {
                    if el(`rhs_receipts', `row', 1) != 3 |                   ///
                        el(`rhs_receipts', `row', 2) != floor(`target' / 2) | ///
                        el(`rhs_receipts', `row', 3) != 1 + mod(`target', 2) | ///
                        el(`rhs_receipts', `row', 14) !=                      ///
                            cond(`native_nuisance' == 1, 2, 1) |              ///
                        el(`rhs_receipts', `row', 15) !=                      ///
                            cond(`native_nuisance' == 1,                      ///
                                `full_solver_dimension',                     ///
                                `firm_solver_dimension') |                    ///
                        el(`rhs_receipts', `row', 13) !=                      ///
                            scalar(__vckss_rust_full_tolerance) {
                        local receipt_mismatch = 1
                    }
                    local row = `row' + 1
                }
                if `projection_columns' > 0 {
                    forvalues projection = 0/`=`projection_columns' - 1' {
                        if el(`rhs_receipts', `row', 1) != 6 |             ///
                            el(`rhs_receipts', `row', 2) != `projection' | ///
                            el(`rhs_receipts', `row', 3) != 0 |            ///
                            el(`rhs_receipts', `row', 14) !=               ///
                                cond(`native_nuisance' == 1, 2, 1) |       ///
                            el(`rhs_receipts', `row', 15) !=               ///
                                cond(`native_nuisance' == 1,               ///
                                    `full_solver_dimension',              ///
                                    `firm_solver_dimension') |            ///
                            el(`rhs_receipts', `row', 13) !=               ///
                                scalar(__vckss_rust_full_tolerance) {
                            local receipt_mismatch = 1
                        }
                        local row = `row' + 1
                    }
                }
                tempname rhs_max_reduced rhs_max_complete
                scalar `rhs_max_reduced' = 0
                scalar `rhs_max_complete' = 0
                forvalues row = 1/`rhs_rows' {
                    forvalues column = 1/15 {
                        if missing(el(`rhs_receipts', `row', `column')) {
                            local receipt_mismatch = 1
                        }
                    }
                    foreach column in 1 2 3 4 5 8 9 10 11 12 14 15 {
                        if el(`rhs_receipts', `row', `column') !=             ///
                            floor(el(`rhs_receipts', `row', `column')) {
                            local receipt_mismatch = 1
                        }
                    }
                    if el(`rhs_receipts', `row', 4) != 2 |                   ///
                        el(`rhs_receipts', `row', 5) < 0 |                    ///
                        el(`rhs_receipts', `row', 6) < 0 |                    ///
                        el(`rhs_receipts', `row', 7) < 0 |                    ///
                        !inlist(el(`rhs_receipts', `row', 8), 0, 1) |         ///
                        !inlist(el(`rhs_receipts', `row', 9), 1, 2) |         ///
                        el(`rhs_receipts', `row', 10) < 0 |                   ///
                        el(`rhs_receipts', `row', 11) < 0 |                   ///
                        el(`rhs_receipts', `row', 12) < 0 |                   ///
                        el(`rhs_receipts', `row', 13) <= 0 |                  ///
                        !inlist(el(`rhs_receipts', `row', 14), 1, 2) |        ///
                        el(`rhs_receipts', `row', 15) <= 0 |                  ///
                        el(`rhs_receipts', `row', 8) !=                      ///
                            (el(`rhs_receipts', `row', 9) == 1) |            ///
                        el(`rhs_receipts', `row', 7) >                       ///
                            el(`rhs_receipts', `row', 13) {
                        local receipt_mismatch = 1
                    }
                    if el(`rhs_receipts', `row', 8) == 1 &                   ///
                        (el(`rhs_receipts', `row', 5) != 0 |                 ///
                         el(`rhs_receipts', `row', 6) != 0 |                 ///
                         el(`rhs_receipts', `row', 10) != 0 |                ///
                         el(`rhs_receipts', `row', 11) != 0 |                ///
                         el(`rhs_receipts', `row', 12) != 0) {
                        local receipt_mismatch = 1
                    }
                    scalar `rhs_max_reduced' = max(                          ///
                        scalar(`rhs_max_reduced'),                           ///
                        el(`rhs_receipts', `row', 6))
                    scalar `rhs_max_complete' = max(                         ///
                        scalar(`rhs_max_complete'),                          ///
                        el(`rhs_receipts', `row', 7))
                }
                if scalar(`rhs_max_reduced') !=                              ///
                        scalar(__vckss_rust_max_reduced) |                   ///
                    scalar(`rhs_max_complete') !=                            ///
                        scalar(__vckss_rust_max_complete) {
                    local receipt_mismatch = 1
                }
            }
        }
        else if `rhs_schema' == 1 {
            if `engine_selected' != 1 |                                  ///
                `rhs_rows' != 1 + `leverage_rhs' + `target_rhs' |         ///
                `rhs_v1_copy' != `rhs_rows' * 112 | `rhs_v2_copy' != 0 {
                local receipt_mismatch = 1
            }
        }
        else if `engine_selected' != 3 | `rhs_rows' != 0 |              ///
            `rhs_v1_copy' != 0 | `rhs_v2_copy' != 0 {
            local receipt_mismatch = 1
        }
        if `receipt_mismatch' {
            quietly _vckss_rust_release_idle `plugin' `handle'
            local cleanup_certified = r(certified)
            di as err "Rust native result receipts did not reconcile"
            if !`cleanup_certified' {
                di as err "Rust result cleanup did not certify an idle native session"
            }
            exit 498
        }

        return matrix result = `estimates'
        return scalar seed = scalar(__vckss_rust_seed)
        return scalar probes = scalar(__vckss_rust_probes)
        return scalar leverage_probes_accepted =                       ///
            scalar(__vckss_rust_lev_accepted)
        return scalar target_probes_accepted =                         ///
            scalar(__vckss_rust_tgt_accepted)
        return scalar requested_route = scalar(__vckss_rust_route_requested)
        return scalar selected_route = scalar(__vckss_rust_route_selected)
        return scalar solver_fallback = scalar(__vckss_rust_fallback)
        return scalar solver_fallback_error =                          ///
            scalar(__vckss_rust_fallback_error)
        return scalar solver_dimension = scalar(__vckss_rust_solver_dimension)
        return scalar leverage_batch_width = scalar(__vckss_rust_lev_batch)
        return scalar target_batch_width = scalar(__vckss_rust_tgt_batch)
        return scalar rank_tolerance = scalar(__vckss_rust_rank_tolerance)
        return scalar block_tolerance = scalar(__vckss_rust_block_tolerance)
        return scalar full_residual_tolerance =                        ///
            scalar(__vckss_rust_full_tolerance)
        return scalar full_fit_route = scalar(__vckss_rust_full_route)
        return scalar full_fit_iterations =                            ///
            scalar(__vckss_rust_full_iterations)
        return scalar full_fit_reduced_residual =                      ///
            scalar(__vckss_rust_full_reduced)
        return scalar full_fit_complete_residual =                     ///
            scalar(__vckss_rust_full_complete)
        return scalar full_fit_zero_rhs = scalar(__vckss_rust_full_zero)
        return scalar leverage_rhs_count = scalar(__vckss_rust_lev_rhs)
        return scalar target_rhs_count = scalar(__vckss_rust_tgt_rhs)
        return scalar projection_columns = `projection_columns'
        return scalar projection_peak_forecast_bytes = `projection_peak'
        return scalar max_reduced_residual = scalar(__vckss_rust_max_reduced)
        return scalar max_complete_residual =                          ///
            scalar(__vckss_rust_max_complete)
        return scalar max_leverage = scalar(__vckss_rust_max_leverage)
        return scalar max_reciprocal_residual =                        ///
            scalar(__vckss_rust_max_reciprocal)
        return scalar accounting_residual = scalar(__vckss_rust_accounting)
        return scalar topology_checksum_hi = scalar(__vckss_rust_topology_hi)
        return scalar topology_checksum_lo = scalar(__vckss_rust_topology_lo)
        return scalar rng_contract_code = scalar(__vckss_rust_rng_contract)
        return scalar rhs_receipt_rows = scalar(__vckss_rust_rhs_rows)
        return scalar caller_result_copy_bytes = scalar(__vckss_rust_rhs_copy)
        return scalar requested_algorithm_code = scalar(__vckss_rust_algorithm_req)
        return scalar selected_algorithm_code = scalar(__vckss_rust_algorithm_sel)
        return scalar deletion_mode_code = scalar(__vckss_rust_deletion_mode)
        return scalar nuisance_mode_code = scalar(__vckss_rust_nuisance_mode)
        return scalar parameters = scalar(__vckss_rust_parameters)
        return scalar full_parameters = scalar(__vckss_rust_full_parameters)
        return scalar correction_parameters = scalar(__vckss_rust_corr_parameters)
        return scalar information_rcond = scalar(__vckss_rust_info_rcond)
        return scalar inverse_relative_residual = scalar(__vckss_rust_inverse_relres)
        return scalar exact_peak_forecast_bytes = scalar(__vckss_rust_exact_peak)
        return scalar exact_diagnostic_flags = scalar(__vckss_rust_exact_flags)
        return scalar working_fit_complete_residual = scalar(__vckss_rust_working_fit)
        return scalar inverse_sqrt_relative_residual = scalar(__vckss_rust_inverse_sqrt)
        return scalar maker_relative_residual = scalar(__vckss_rust_maker_relres)
        return scalar control_basis_relative_residual = scalar(__vckss_rust_control_relres)
        return scalar control_basis_forward_error = scalar(__vckss_rust_control_fwd_error)
        return scalar deletion_rank_gap = scalar(__vckss_rust_deletion_rank_gap)
        return scalar firm_zero_sum_residual = scalar(__vckss_rust_firm_zero_sum)
        return scalar fit_peak_forecast_bytes = scalar(__vckss_rust_fit_peak)
        return scalar correction_peak_forecast_bytes = scalar(__vckss_rust_correction_peak)
        return scalar actual_accounting_residual = scalar(__vckss_rust_actual_accounting)
        return scalar weighted_rss = scalar(__vckss_rust_weighted_rss)
        return scalar memory_limit_bytes = scalar(__vckss_rust_memory_limit)
        return scalar caller_copy_bytes = scalar(__vckss_rust_caller_copy)
        return scalar preparation_peak_forecast_bytes = scalar(__vckss_rust_prepare_peak)
        return scalar prepared_resident_bytes =                        ///
            scalar(__vckss_rust_prepared_resident)
        return scalar solver_setup_forecast_bytes = scalar(__vckss_rust_solver_setup)
        return scalar leverage_phase_forecast_bytes = scalar(__vckss_rust_leverage_phase)
        return scalar target_phase_forecast_bytes = scalar(__vckss_rust_target_phase)
        return scalar result_forecast_bytes = scalar(__vckss_rust_result_bytes)
        return scalar solve_peak_forecast_bytes = scalar(__vckss_rust_solve_peak)
        return scalar command_peak_forecast_bytes = scalar(__vckss_rust_command_peak)
        return scalar rhs_receipt_schema = scalar(__vckss_rust_rhs_schema)
        return scalar requested_engine_code = scalar(__vckss_rust_engine_requested)
        return scalar selected_engine_code = scalar(__vckss_rust_engine_selected)
        return scalar generic_diagnostic_flags = scalar(__vckss_rust_generic_flags)
        return scalar generic_controls_count = scalar(__vckss_rust_generic_controls)
        return scalar control_projection_rhs_count = scalar(__vckss_rust_g_control_rhs)
        return scalar control_rank_rcond = scalar(__vckss_rust_cr_rcond)
        return scalar control_rank_smallest_lower = scalar(__vckss_rust_cr_small_lo)
        return scalar control_rank_largest_upper = scalar(__vckss_rust_cr_large_hi)
        return scalar control_rank_projection_error = scalar(__vckss_rust_cr_proj_err)
        return scalar control_rank_normalization_err = scalar(__vckss_rust_cr_norm_err)
        return scalar control_rank_fe_info_lower = scalar(__vckss_rust_cr_fe_lo)
        return scalar control_rank_max_projection = scalar(__vckss_rust_cr_max_proj)
        return scalar control_rank_effective_tolerance = scalar(__vckss_rust_cr_tol)
        return scalar control_rank_projection_pcg_tol = scalar(__vckss_rust_cr_pcg_tol)
        return scalar control_rank_projection_gate = scalar(__vckss_rust_cr_resid_gate)
        return scalar generic_control_basis_relres = scalar(__vckss_rust_g_control_relres)
        return scalar generic_control_basis_fwd_err = scalar(__vckss_rust_g_control_fwd)
        return scalar generic_control_schur_rcond = scalar(__vckss_rust_g_schur_rcond)
        return scalar generic_control_schur_relres = scalar(__vckss_rust_g_schur_relres)
        return scalar generic_deletion_rank_gap = scalar(__vckss_rust_g_delete_gap)
        return scalar full_joint_fit_complete_residual = scalar(__vckss_rust_g_full_joint)
        return scalar generic_working_fit_residual = scalar(__vckss_rust_g_working_fit)
        return scalar generic_maker_relative_residual = scalar(__vckss_rust_g_maker)
        return scalar canonicalization_peak_bytes = scalar(__vckss_rust_g_canon_peak)
        return scalar generic_fit_peak_forecast_bytes = scalar(__vckss_rust_g_fit_peak)
        return scalar geometry_peak_forecast_bytes = scalar(__vckss_rust_g_geometry_peak)
        return scalar generic_leverage_peak_bytes = scalar(__vckss_rust_g_lev_peak)
        return scalar generic_target_peak_bytes = scalar(__vckss_rust_g_tgt_peak)
        return scalar maker_peak_forecast_bytes = scalar(__vckss_rust_g_maker_peak)
        return scalar generic_result_forecast_bytes = scalar(__vckss_rust_g_result_bytes)
        return scalar generic_peak_forecast_bytes = scalar(__vckss_rust_g_peak)
        return scalar rhs_v2_caller_copy_bytes = scalar(__vckss_rust_rhs_v2_copy)
        return scalar capability_schema = scalar(__vckss_rust_cap_schema_echo)
        return scalar capability_profile = scalar(__vckss_rust_cap_profile_echo)
        return scalar batch_mode_code = scalar(__vckss_rust_solve_batch)
        return scalar stayers_mode_code = scalar(__vckss_rust_solve_stayers)
        return scalar target_weight_mode_code = scalar(__vckss_rust_solve_target)
        return scalar deletion_source_code = scalar(__vckss_rust_solve_delsource)
        return scalar probeorder_supplied = scalar(__vckss_rust_solve_probeorder)
        return scalar wallseconds_supplied = scalar(__vckss_rust_solve_wall)
        return scalar frequency_use_code = scalar(__vckss_rust_solve_frequency)
        return scalar physical_limit = scalar(__vckss_rust_solve_physlimit)
        return scalar request_signature_hi = scalar(__vckss_rust_solve_signature_hi)
        return scalar request_signature_lo = scalar(__vckss_rust_solve_signature_lo)
        return scalar performance_schema = scalar(__vckss_rust_pf_schema)
        return scalar performance_flags = scalar(__vckss_rust_pf_flags)
        return scalar performance_ingest_ns = scalar(__vckss_rust_pf_ingest_ns)
        return scalar performance_canonicalize_ns = scalar(__vckss_rust_pf_canon_ns)
        return scalar performance_graph_ns = scalar(__vckss_rust_pf_graph_ns)
        return scalar performance_compress_ns = scalar(__vckss_rust_pf_compress_ns)
        return scalar performance_plan_ns = scalar(__vckss_rust_pf_plan_ns)
        return scalar performance_stayer_ns = scalar(__vckss_rust_pf_stayer_ns)
        return scalar performance_solve_ns = scalar(__vckss_rust_pf_solve_ns)
        return scalar performance_total_ns = scalar(__vckss_rust_pf_total_ns)
        return scalar handle = real("`handle'")
        if `rhs_rows' > 0 return matrix rhs_receipts = `rhs_receipts'
        local requested_algorithm_code = scalar(__vckss_rust_algorithm_req)
        local selected_algorithm_code = scalar(__vckss_rust_algorithm_sel)
        local deletion_mode_code = scalar(__vckss_rust_deletion_mode)
        local nuisance_mode_code = scalar(__vckss_rust_nuisance_mode)
        local requested_algorithm jla
        if `requested_algorithm_code' == 0 local requested_algorithm auto
        else if `requested_algorithm_code' == 1 local requested_algorithm exact
        local selected_algorithm jla
        if `selected_algorithm_code' == 1 local selected_algorithm exact
        local deletion_mode observation
        if `deletion_mode_code' == 1 local deletion_mode match
        local nuisance_mode fixedoffset
        if `nuisance_mode_code' == 1 local nuisance_mode joint
        local rng_contract none
        if scalar(__vckss_rust_rng_contract) == 1 local rng_contract VCKSS-COUNTER-V1
        local requested_engine unspecified
        if `engine_requested' == 1 local requested_engine compressed
        else if `engine_requested' == 2 local requested_engine generic
        else if `engine_requested' == 3 local requested_engine not_applicable
        local selected_engine not_applicable
        if `engine_selected' == 0 local selected_engine unspecified
        else if `engine_selected' == 1 local selected_engine compressed
        else if `engine_selected' == 2 local selected_engine generic
        return local requested_algorithm "`requested_algorithm'"
        return local selected_algorithm "`selected_algorithm'"
        return local deletion_mode "`deletion_mode'"
        return local nuisance_mode "`nuisance_mode'"
        return local rng_contract "`rng_contract'"
        return local requested_engine "`requested_engine'"
        return local selected_engine "`selected_engine'"
        return local backend "rust"
        return local subcommand "result"
        foreach name in plugin_worker plugin_firm plugin_cov plugin_total     ///
            correction_worker correction_firm correction_cov correction_total ///
            corrected_worker corrected_firm corrected_cov corrected_total      ///
            mcse_worker mcse_firm mcse_cov mcse_total {
            capture scalar drop __vckss_`name'
        }
        foreach name in seed probes lev_accepted tgt_accepted route_requested ///
            route_selected fallback fallback_error solver_dimension           ///
            lev_batch tgt_batch rank_tolerance block_tolerance full_tolerance ///
            full_route full_iterations full_reduced full_complete full_zero   ///
            lev_rhs                                                          ///
            tgt_rhs max_reduced max_complete max_leverage max_reciprocal       ///
            accounting topology_hi topology_lo rng_contract rhs_rows rhs_copy  ///
            algorithm_req algorithm_sel deletion_mode nuisance_mode parameters ///
            full_parameters corr_parameters info_rcond inverse_relres exact_peak ///
            exact_flags working_fit inverse_sqrt maker_relres control_relres     ///
            control_fwd_error deletion_rank_gap firm_zero_sum fit_peak           ///
            correction_peak actual_accounting                                   ///
            weighted_rss memory_limit caller_copy prepare_peak                 ///
            prepared_resident solver_setup leverage_phase target_phase         ///
            result_bytes solve_peak command_peak {
            capture scalar drop __vckss_rust_`name'
        }
        foreach name in engine_requested engine_selected generic_flags       ///
            generic_controls rhs_schema proj_columns proj_peak g_control_rhs cr_rcond cr_small_lo ///
            cr_large_hi cr_proj_err cr_norm_err cr_fe_lo cr_max_proj cr_tol  ///
            cr_pcg_tol cr_resid_gate g_control_relres g_control_fwd           ///
            g_schur_rcond g_schur_relres g_delete_gap g_full_joint           ///
            g_working_fit g_maker g_canon_peak g_fit_peak g_geometry_peak    ///
            g_lev_peak g_tgt_peak g_maker_peak g_result_bytes g_peak         ///
            rhs_v2_copy cap_schema_echo cap_profile_echo solve_batch         ///
            solve_stayers solve_target solve_delsource solve_probeorder      ///
            solve_wall solve_frequency solve_physlimit solve_signature_hi    ///
            solve_signature_lo pf_schema pf_flags pf_ingest_ns pf_canon_ns  ///
            pf_graph_ns pf_compress_ns pf_plan_ns pf_stayer_ns pf_solve_ns  ///
            pf_total_ns {
            capture scalar drop __vckss_rust_`name'
        }
        exit
    }

    if "`subcommand'" == "fullcmgreceipt" {
        syntax anything(name=handle id="native Rust generation")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "fullcmgreceipt requires one positive integer native generation"
            exit 198
        }
        _vckss_rust_plugin_call `plugin', fullcmgreceipt `handle'
        foreach pair in cmg_struct:struct_size cmg_schema:schema_version ///
            cmg_generation:generation cmg_backend_id:backend_identity    ///
            cmg_platform_os:platform_os cmg_platform_arch:platform_arch ///
            cmg_batch_mask:batch_strategy_mask                          ///
            cmg_threads_req:threads_requested cmg_threads_used:threads_used ///
            cmg_max_concurrency:maximum_concurrency cmg_vertices:vertices ///
            cmg_edges:edges cmg_levels:hierarchy_levels                 ///
            cmg_terminal:terminal_vertices cmg_graph_bytes:graph_copy_bytes ///
            cmg_hierarchy_bytes:hierarchy_bytes cmg_plan_bytes:plan_bytes ///
            cmg_ws_each:workspace_bytes_each cmg_ws_pool:workspace_pool_bytes ///
            cmg_admitted_peak:admitted_peak_bytes cmg_fit_tol:fit_effective_tolerance ///
            cmg_probe_tol:probe_effective_tolerance cmg_fit_inner:fit_initial_inner_tolerance ///
            cmg_probe_inner:probe_initial_inner_tolerance               ///
            cmg_refine_attempts:refinement_attempts cmg_refined_cols:refined_columns ///
            cmg_batch_calls:batch_calls cmg_rhs_count:rhs_count          ///
            cmg_serial_batches:serial_batches cmg_planned_batches:planned_batches ///
            cmg_across_batches:across_rhs_batches cmg_iterations:total_iterations ///
            cmg_operator_apps:total_operator_applications               ///
            cmg_precond_apps:total_preconditioner_apps                  ///
            cmg_max_reduced:maximum_reduced_residual                    ///
            cmg_max_complete:maximum_complete_residual cmg_graph_ns:graph_ns ///
            cmg_hierarchy_ns:hierarchy_plan_ns cmg_rhs_ns:rhs_ns        ///
            cmg_solve_ns:solve_ns cmg_extract_ns:extraction_ns          ///
            cmg_prep_peak:preparation_peak_bytes                        ///
            cmg_prepared_bytes:prepared_persistent_bytes                ///
            cmg_non_cmg_peak:non_cmg_command_peak_bytes                 ///
            cmg_pre_rng_forecast:pre_rng_forecast_bytes                 ///
            cmg_actual_retained:actual_retained_bytes                   ///
            cmg_allocator_allowance:allocator_allowance_bytes           ///
            cmg_max_batch_rhs:maximum_batch_rhs                         ///
            cmg_workspace_count:workspace_count {
            gettoken source target : pair, parse(":")
            gettoken colon target : target, parse(":")
            return scalar `target' = scalar(__vckss_`source')
        }
        return scalar handle = real("`handle'")
        // The C shim has already compared both fixed identities byte-for-byte
        // with the additive receipt before returning success. Export the
        // reconciled constants here instead of relying on plugin-local macro
        // scope across the rclass wrapper boundary.
        return local cmg_backend "CMG_FULL_V2"
        return local cmg_source_commit ///
            "92a12f2d572ca56b30a035220953f9dd4bced999"
        return local backend "rust"
        return local subcommand "fullcmgreceipt"
        capture macro drop __vckss_cmg_backend
        capture macro drop __vckss_cmg_source
        foreach name in struct schema generation backend_id platform_os platform_arch ///
            batch_mask threads_req threads_used max_concurrency vertices edges levels ///
            terminal graph_bytes hierarchy_bytes plan_bytes ws_each ws_pool admitted_peak ///
            fit_tol probe_tol fit_inner probe_inner refine_attempts refined_cols batch_calls ///
            rhs_count serial_batches planned_batches across_batches iterations operator_apps ///
            precond_apps max_reduced max_complete graph_ns hierarchy_ns rhs_ns solve_ns extract_ns ///
            prep_peak prepared_bytes non_cmg_peak pre_rng_forecast actual_retained              ///
            allocator_allowance max_batch_rhs workspace_count {
            capture scalar drop __vckss_cmg_`name'
        }
        exit
    }

    if "`subcommand'" == "solve" {
        gettoken handle 0 : 0, parse(" ,")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "solve requires one positive integer native generation"
            exit 198
        }
        syntax [, SEED(integer 8675309) PROBES(integer 200)              ///
            LEVERAGEBatch(integer 8) TARGETBatch(integer 8)              ///
            ROUTE(string) TOLerance(real 1e-10) MAXIter(integer 10000)    ///
            ALGORITHM(string) DELETION(string) NUISANCE(string)           ///
            EXACTLimit(integer 500) BLOCKSIZELimit(integer 5000)          ///
            RANKTolerance(real 1e-10) BLOCKTolerance(real 1e-10)           ///
            ENGINE(string) BATCHMODE(string) STAYERS(string)               ///
            TARGETWEIGHTMODE(string) DELETIONSOURCE(string)                ///
            PROBEORDERSUPPLIED(integer 0) WALLSECONDSSUPPLIED(integer 0)    ///
            PHYSICALLIMIT(real 50000000) CAPABILITYSCHEMA(integer 0)        ///
            CAPABILITYPROFILE(integer 0) FREQUENCYUSED(integer 0)           ///
            SIGNATUREHI(real 0) SIGNATURELO(real 0)                          ///
            LEVERAGEBATCHMODE(string) TARGETBATCHMODE(string)                ///
            FALLBACK(integer -1) WALLSECONDS(real 0) FULLCMG(integer 0) ///
            THREADS(integer 1) TOLERANCESUPPLIED(integer 0)]
        if missing(`seed') | missing(`probes') |                         ///
            missing(`leveragebatch') | missing(`targetbatch') |         ///
            missing(`tolerance') | missing(`maxiter') |                 ///
            `seed' < 0 | `probes' < 2 | `leveragebatch' < 0 |           ///
            `targetbatch' < 0 | `tolerance' <= 0 | `maxiter' <= 0 |        ///
            missing(`exactlimit') | `exactlimit' <= 0 |                   ///
            missing(`blocksizelimit') | `blocksizelimit' <= 0 |           ///
            missing(`ranktolerance') | `ranktolerance' <= 0 |             ///
            missing(`blocktolerance') | `blocktolerance' <= 0 {
            di as err "invalid Rust solve tuning option"
            exit 198
        }
        local route = lower(strtrim("`route'"))
        if "`route'" == "" local route auto
        if !inlist("`route'", "auto", "exact", "diagonal", "cmg") {
            di as err "route() must be auto, exact, diagonal, or cmg"
            exit 198
        }
        local algorithm = lower(strtrim("`algorithm'"))
        if "`algorithm'" == "" local algorithm jla
        if !inlist("`algorithm'", "auto", "exact", "jla") {
            di as err "algorithm() must be auto, exact, or jla"
            exit 198
        }
        local deletion = lower(strtrim("`deletion'"))
        if "`deletion'" == "" local deletion match
        if !inlist("`deletion'", "match", "observation") {
            di as err "deletion() must be match or observation"
            exit 198
        }
        local nuisance = lower(strtrim("`nuisance'"))
        if "`nuisance'" == "" local nuisance joint
        if !inlist("`nuisance'", "joint", "fixedoffset") {
            di as err "nuisance() must be joint or fixedoffset"
            exit 198
        }
        local tolerance_arg = strtrim(strofreal(`tolerance', "%21.17f"))
        local rank_tolerance_arg = strtrim(strofreal(`ranktolerance', "%21.17f"))
        local block_tolerance_arg = strtrim(strofreal(`blocktolerance', "%21.17f"))
        local engine = lower(strtrim("`engine'"))
        local planned_solve = (`capabilityschema' == 3 | `capabilityprofile' == 4)
        if `planned_solve' {
            if `capabilityschema' != 3 | `capabilityprofile' != 4 {
                di as err "planned Rust solve requires capability schema 3/profile 4"
                exit 198
            }
            if "`engine'" == "" local engine auto
            local batchmode = lower(strtrim("`batchmode'"))
            if "`batchmode'" == "" local batchmode explicit
            local leveragebatchmode = lower(strtrim("`leveragebatchmode'"))
            local targetbatchmode = lower(strtrim("`targetbatchmode'"))
            if "`batchmode'" == "independent" &                         ///
                ("`leveragebatchmode'" == "" | "`targetbatchmode'" == "") {
                di as err "batchmode(independent) requires both phase batch modes"
                exit 198
            }
            if "`leveragebatchmode'" == "" local leveragebatchmode `batchmode'
            if "`targetbatchmode'" == "" local targetbatchmode `batchmode'
            local stayers = lower(strtrim("`stayers'"))
            if "`stayers'" == "" local stayers movers
            local targetweightmode = lower(strtrim("`targetweightmode'"))
            if "`targetweightmode'" == "" local targetweightmode frequency
            local deletionsource = lower(strtrim("`deletionsource'"))
            if "`deletionsource'" == "" {
                local deletionsource cell
                if "`deletion'" == "observation" local deletionsource observation
            }
            if `fallback' < 0 local fallback = ("`route'" == "auto")
            foreach value in physicallimit signaturehi signaturelo {
                if missing(``value'') | ``value'' < 0 |                    ///
                    ``value'' != floor(``value'') {
                    di as err "`value'() must be a nonnegative exactly represented integer"
                    exit 198
                }
            }
            if `physicallimit' <= 0 | `physicallimit' > 9007199254740992 | ///
                `signaturehi' > 4294967295 | `signaturelo' > 4294967295 {
                di as err "physical limit or capability signature half is out of range"
                exit 198
            }
            local physical_arg = strtrim(strofreal(`physicallimit', "%21.0f"))
            local signature_hi_arg = strtrim(strofreal(`signaturehi', "%21.0f"))
            local signature_lo_arg = strtrim(strofreal(`signaturelo', "%21.0f"))
            local wallseconds_arg = strtrim(strofreal(`wallseconds', "%21.17g"))
            if !inlist(`fullcmg', 0, 1) | missing(`threads') |           ///
                `threads' <= 0 | `threads' != floor(`threads') |         ///
                !inlist(`tolerancesupplied', 0, 1) {
                di as err "invalid full-CMG solve controls"
                exit 198
            }
            if `fullcmg' {
                _vckss_rust_solve_v5 `plugin' `handle' `seed' `probes'   ///
                    `leveragebatch' `targetbatch' `route' `tolerance_arg' ///
                    `maxiter' `algorithm' `deletion' `nuisance' `exactlimit' ///
                    `blocksizelimit' `rank_tolerance_arg'                ///
                    `block_tolerance_arg' `engine' `batchmode' `stayers' ///
                    `targetweightmode' `deletionsource' `probeordersupplied' ///
                    `wallsecondssupplied' `physical_arg' `capabilityschema' ///
                    `capabilityprofile' `frequencyused' `signature_hi_arg' ///
                    `signature_lo_arg' `leveragebatchmode'               ///
                    `targetbatchmode' `fallback' `wallseconds_arg'        ///
                    `threads' `tolerancesupplied'
                return add
                exit
            }
            _vckss_rust_solve_v4 `plugin' `handle' `seed' `probes'        ///
                `leveragebatch' `targetbatch' `route' `tolerance_arg'    ///
                `maxiter' `algorithm' `deletion' `nuisance' `exactlimit' ///
                `blocksizelimit' `rank_tolerance_arg'                    ///
                `block_tolerance_arg' `engine' `batchmode' `stayers'     ///
                `targetweightmode' `deletionsource' `probeordersupplied' ///
                `wallsecondssupplied' `physical_arg' `capabilityschema'  ///
                `capabilityprofile' `frequencyused' `signature_hi_arg'   ///
                `signature_lo_arg' `leveragebatchmode'                   ///
                `targetbatchmode' `fallback' `wallseconds_arg'
            return add
            exit
        }
        if `leveragebatch' <= 0 | `targetbatch' <= 0 {
            di as err "legacy Rust solve requires positive leveragebatch() and targetbatch()"
            exit 198
        }
        if "`engine'" == "" {
            _vckss_rust_plugin_call `plugin', solve `handle' `seed' `probes'  ///
                `leveragebatch' `targetbatch' `route' `tolerance_arg' `maxiter' ///
                `algorithm' `deletion' `nuisance' `exactlimit' `blocksizelimit' ///
                `rank_tolerance_arg' `block_tolerance_arg'
        }
        else {
            if "`engine'" != "generic" {
                di as err "the additive solve interface currently requires engine(generic)"
                exit 198
            }
            local batchmode = lower(strtrim("`batchmode'"))
            if "`batchmode'" == "" local batchmode explicit
            local stayers = lower(strtrim("`stayers'"))
            if "`stayers'" == "" local stayers movers
            local targetweightmode = lower(strtrim("`targetweightmode'"))
            if "`targetweightmode'" == "" local targetweightmode frequency
            local deletionsource = lower(strtrim("`deletionsource'"))
            if "`deletionsource'" == "" {
                local deletionsource cell
                if "`deletion'" == "observation" local deletionsource observation
            }
            if "`algorithm'" != "jla" | "`route'" != "diagonal" |       ///
                "`batchmode'" != "explicit" | "`stayers'" != "movers" | ///
                !inlist("`targetweightmode'", "frequency", "explicit") | ///
                !inlist("`deletionsource'", "cell", "matchid", "observation") {
                di as err "invalid explicit generic-JLA engine tuple"
                exit 198
            }
            if !inlist(`probeordersupplied', 0, 1) |                    ///
                !inlist(`wallsecondssupplied', 0, 1) |                  ///
                !inlist(`frequencyused', 0, 1) |                        ///
                `capabilityschema' != 2 | `capabilityprofile' != 3 {
                di as err "generic solve requires its V2 capability schema/profile and valid semantic flags"
                exit 198
            }
            foreach value in physicallimit signaturehi signaturelo {
                if missing(``value'') | ``value'' < 0 | ``value'' != floor(``value'') {
                    di as err "`value'() must be a nonnegative exactly represented integer"
                    exit 198
                }
            }
            if `physicallimit' <= 0 | `physicallimit' > 9007199254740992 | ///
                `signaturehi' > 4294967295 | `signaturelo' > 4294967295 {
                di as err "physical limit or capability signature half is out of range"
                exit 198
            }
            local physical_arg = strtrim(strofreal(`physicallimit', "%21.0f"))
            local signature_hi_arg = strtrim(strofreal(`signaturehi', "%21.0f"))
            local signature_lo_arg = strtrim(strofreal(`signaturelo', "%21.0f"))
            _vckss_rust_plugin_call `plugin', solve `handle' `seed' `probes'  ///
                `leveragebatch' `targetbatch' `route' `tolerance_arg' `maxiter' ///
                `algorithm' `deletion' `nuisance' `exactlimit' `blocksizelimit' ///
                `rank_tolerance_arg' `block_tolerance_arg' `engine' `batchmode' ///
                `stayers' `targetweightmode' `deletionsource'                  ///
                `probeordersupplied' `wallsecondssupplied' `physical_arg'      ///
                `capabilityschema' `capabilityprofile' `frequencyused'         ///
                `signature_hi_arg' `signature_lo_arg'
        }
        return scalar handle = real("`handle'")
        return scalar seed = `seed'
        return scalar probes = `probes'
        return scalar exact_limit = `exactlimit'
        return scalar blocksize_limit = `blocksizelimit'
        if "`engine'" != "" {
            return scalar capability_schema = `capabilityschema'
            return scalar capability_profile = `capabilityprofile'
            return scalar request_signature_hi = `signaturehi'
            return scalar request_signature_lo = `signaturelo'
            return scalar physical_limit = `physicallimit'
            return scalar frequency_use_code = `frequencyused'
            return local engine "`engine'"
            return local batch_mode "`batchmode'"
            return local stayers "`stayers'"
            return local target_weight_mode "`targetweightmode'"
            return local deletion_source "`deletionsource'"
        }
        return local route "`route'"
        return local algorithm "`algorithm'"
        return local deletion "`deletion'"
        return local nuisance "`nuisance'"
        return local backend "rust"
        return local subcommand "solve"
        exit
    }

    if "`subcommand'" == "prepare" {
        syntax varlist(min=6 numeric) [if] [in],                        ///
            [CLEANUP GENerate(name) MEMORYGib(real 4) DELETION(string) ///
                PROBEOrder(name) IMPLICITMATCH]
        if `memorygib' <= 0 | missing(`memorygib') {
            di as err "memorygib() must be finite and positive"
            exit 198
        }
        local memory_bytes = floor(`memorygib' * 1073741824)
        if `memory_bytes' <= 0 | `memory_bytes' > 9007199254740992 {
            di as err "memorygib() is outside the exactly representable byte range"
            exit 198
        }
        local memory_arg = strtrim(strofreal(`memory_bytes', "%21.0f"))
        local deletion = lower(strtrim("`deletion'"))
        if "`deletion'" == "" local deletion match
        if !inlist("`deletion'", "match", "observation") {
            di as err "deletion() must be match or observation"
            exit 198
        }
        local var_count : word count `varlist'
        local controls_count = `var_count' - 6
        local probeorder = strtrim("`probeorder'")
        local probeorder_arg noprobeorder
        local implicit_match_arg explicitdeletion
        if "`implicitmatch'" != "" local implicit_match_arg implicitmatch
        local plugin_varlist `varlist'
        if "`probeorder'" != "" {
            confirm numeric variable `probeorder'
            local probeorder_arg probeorder
            local plugin_varlist `plugin_varlist' `probeorder'
        }
        marksample touse, novarlist
        markout `touse' `plugin_varlist'

        local retained `generate'
        if "`retained'" == "" tempvar retained
        else confirm new variable `retained'
        quietly generate byte `retained' = 0

        local cleanup_arg nocleanup
        if "`cleanup'" != "" local cleanup_arg cleanup
        capture noisily _vckss_rust_plugin_call `plugin' `touse' `plugin_varlist' `retained' ///
            if `touse', prepare `cleanup_arg' `memory_arg' `deletion' `controls_count' ///
            `probeorder_arg' `implicit_match_arg'
        local prepare_rc = _rc
        if `prepare_rc' {
            capture drop `retained'
            exit `prepare_rc'
        }
        local native_controls = scalar(__vckss_rust_controls_count)
        local native_deletion = scalar(__vckss_rust_prep_deletion)
        local expected_deletion = cond("`deletion'" == "match", 1, 2)
        if missing(`native_controls') | `native_controls' < 0 |       ///
            `native_controls' != floor(`native_controls') |          ///
            `native_controls' != `controls_count' |                  ///
            !inlist(`native_deletion', 1, 2) |                       ///
            `native_deletion' != `expected_deletion' {
            local failed_handle = scalar(__vckss_rust_handle)
            capture _vckss_rust_plugin_call `plugin', release `failed_handle'
            capture _vckss_rust_plugin_call `plugin', clear
            capture drop `retained'
            di as err "Rust native preparation facts did not reconcile with the request"
            exit 498
        }
        local native_deletion_name observation
        if `native_deletion' == 1 local native_deletion_name match

        return scalar handle = scalar(__vckss_rust_handle)
        return scalar input_rows = scalar(__vckss_rust_input_rows)
        return scalar retained_rows = scalar(__vckss_rust_retained_rows)
        return scalar workers = scalar(__vckss_rust_workers)
        return scalar firms = scalar(__vckss_rust_firms)
        return scalar cells = scalar(__vckss_rust_cells)
        return scalar deletion_units = scalar(__vckss_rust_deletion_units)
        return scalar target_strata = scalar(__vckss_rust_target_strata)
        return scalar target_weight_sum = scalar(__vckss_rust_target_sum)
        return scalar memory_limit_bytes = scalar(__vckss_rust_memory_limit)
        return scalar caller_copy_bytes = scalar(__vckss_rust_caller_copy)
        return scalar preparation_peak_forecast_bytes = scalar(__vckss_rust_prepare_peak)
        return scalar prepared_resident_bytes =                        ///
            scalar(__vckss_rust_prepared_resident)
        return scalar graph_input_rows = scalar(__vckss_rust_graph_input_rows)
        return scalar graph_retained_rows =                             ///
            scalar(__vckss_rust_graph_retained_rows)
        return scalar graph_input_physical_mass =                       ///
            scalar(__vckss_rust_graph_input_mass)
        return scalar graph_retained_physical_mass =                    ///
            scalar(__vckss_rust_graph_retained_mass)
        return scalar graph_initial_components =                        ///
            scalar(__vckss_rust_graph_init_comp)
        return scalar graph_maximum_components =                        ///
            scalar(__vckss_rust_graph_max_comp)
        return scalar graph_initial_component_rows =                    ///
            scalar(__vckss_rust_graph_initial_rows)
        return scalar graph_mover_input_rows =                          ///
            scalar(__vckss_rust_graph_mover_rows)
        return scalar graph_initial_deletion_edges =                    ///
            scalar(__vckss_rust_graph_initial_edges)
        return scalar graph_retained_deletion_edges =                   ///
            scalar(__vckss_rust_graph_keep_edges)
        return scalar graph_degree_workers_removed =                   ///
            scalar(__vckss_rust_graph_deg_removed)
        return scalar graph_artic_workers_removed =                    ///
            scalar(__vckss_rust_graph_art_removed)
        return scalar graph_bridge_units_removed =                     ///
            scalar(__vckss_rust_graph_bridge_units)
        return scalar graph_bridge_rows_removed =                      ///
            scalar(__vckss_rust_graph_bridge_rows)
        return scalar graph_degree_iterations =                        ///
            scalar(__vckss_rust_graph_deg_iters)
        return scalar graph_articulation_iterations =                  ///
            scalar(__vckss_rust_graph_art_iters)
        return scalar graph_bridge_iterations =                        ///
            scalar(__vckss_rust_graph_bridge_iters)
        return scalar graph_fixed_point_iterations =                   ///
            scalar(__vckss_rust_graph_fixed_iters)
        return scalar controls_count = `native_controls'
        return scalar deletion_mode_code = `native_deletion'
        return local retained_variable "`generate'"
        return local deletion "`native_deletion_name'"
        return local backend "rust"
        return local subcommand "prepare"

        foreach name in handle input_rows retained_rows workers firms cells  ///
            deletion_units target_strata target_sum memory_limit caller_copy ///
            prepare_peak prepared_resident graph_input_rows                  ///
            graph_retained_rows graph_input_mass graph_retained_mass         ///
            graph_init_comp graph_max_comp                                   ///
            graph_initial_rows graph_mover_rows graph_initial_edges          ///
            graph_keep_edges graph_deg_removed                               ///
            graph_art_removed graph_bridge_units graph_bridge_rows           ///
            graph_deg_iters graph_art_iters graph_bridge_iters               ///
            graph_fixed_iters {
            capture scalar drop __vckss_rust_`name'
        }
        capture scalar drop __vckss_rust_controls_count
        capture scalar drop __vckss_rust_prep_deletion
        exit
    }

    di as err "unknown Rust backend subcommand: `subcommand'"
    di as err "valid subcommands: probe, capabilities, requestcapability, version, selftest, prepare, augmentstayers, augmentprojection, solve, result, projectionresult, fullcmgreceipt, stayerresult, snapshot, release, clear, lasterror"
    exit 198
end
