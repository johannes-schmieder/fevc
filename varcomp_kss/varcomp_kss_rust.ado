*! version 0.2.0 21aug2026
if strpos("`c(machine_type)'", "Mac") == 1 {
    if strpos("`c(machine_type)'", "Apple Silicon") {
        program _vckss_rust_macos, plugin using("varcomp_kss_rust_macos_arm64.plugin")
    }
    else {
        program _vckss_rust_macos, plugin using("varcomp_kss_rust_macos_x86_64.plugin")
    }
}
else if "`c(os)'" == "Windows" {
    program _vckss_rust_windows, plugin using("varcomp_kss_rust_windows_x64.plugin")
}
else if "`c(os)'" == "Unix" {
    program _vckss_rust_linux, plugin using("varcomp_kss_rust_linux_x64.plugin")
}

program define varcomp_kss_rust, rclass
    version 18.0

    gettoken subcommand 0 : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "" {
        di as err "Rust backend subcommand required"
        di as err "valid subcommands: probe, capabilities, version, selftest, prepare, solve, result, snapshot, release, clear, lasterror"
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

    if "`subcommand'" == "probe" {
        if strtrim(`"`0'"') != "" {
            di as err "probe does not accept additional arguments"
            exit 198
        }
        plugin call `plugin', probe
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
        plugin call `plugin', lasterror
        return scalar native_error_code = scalar(__vckss_rust_error_code)
        return local native_error_status `"$__vckss_rust_error_status"'
        return local native_error_detail `"$__vckss_rust_error_detail"'
        capture scalar drop __vckss_rust_error_code
        capture macro drop __vckss_rust_error_status
        capture macro drop __vckss_rust_error_detail
        return local backend "rust"
        return local subcommand "lasterror"
        exit
    }

    if inlist("`subcommand'", "capabilities", "version", "selftest", "clear") {
        if strtrim(`"`0'"') != "" {
            di as err "`subcommand' does not accept additional arguments"
            exit 198
        }
        plugin call `plugin', `subcommand'
        return local backend "rust"
        return local subcommand "`subcommand'"
        exit
    }

    if "`subcommand'" == "snapshot" {
        if strtrim(`"`0'"') != "" {
            di as err "snapshot does not accept additional arguments"
            exit 198
        }
        plugin call `plugin', snapshot
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

    if inlist("`subcommand'", "release", "result") {
        syntax anything(name=handle id="native Rust generation")
        capture confirm integer number `handle'
        if _rc | real("`handle'") <= 0 {
            di as err "`subcommand' requires one positive integer native generation"
            exit 198
        }
        if "`subcommand'" == "release" {
            plugin call `plugin', release `handle'
            return scalar handle = real("`handle'")
            return local backend "rust"
            return local subcommand "release"
            exit
        }

        plugin call `plugin', result `handle'
        local rhs_rows = scalar(__vckss_rust_rhs_rows)
        if missing(`rhs_rows') | `rhs_rows' <= 0 |              ///
            `rhs_rows' != floor(`rhs_rows') |                   ///
            `rhs_rows' > c(max_matdim) {
            di as err "Rust returned an invalid RHS receipt row count"
            exit 498
        }
        tempname rhs_receipts
        matrix `rhs_receipts' = J(`rhs_rows',8,.)
        plugin call `plugin', rhsresult `handle' `rhs_receipts'
        matrix colnames `rhs_receipts' = phase probe side route iterations ///
            reduced_residual complete_residual zero_rhs
        tempname estimates
        matrix `estimates' =                                         ///
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
        matrix rownames `estimates' = plugin bias_correction corrected numerical_mcse
        matrix colnames `estimates' = worker_variance firm_variance          ///
            worker_firm_covariance total_variance
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
        return scalar handle = real("`handle'")
        return matrix rhs_receipts = `rhs_receipts'
        return local rng_contract "VCKSS-COUNTER-V1"
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
            weighted_rss memory_limit caller_copy prepare_peak                 ///
            prepared_resident solver_setup leverage_phase target_phase         ///
            result_bytes solve_peak command_peak {
            capture scalar drop __vckss_rust_`name'
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
            ROUTE(string) TOLerance(real 1e-10) MAXIter(integer 10000)]
        if missing(`seed') | missing(`probes') |                         ///
            missing(`leveragebatch') | missing(`targetbatch') |         ///
            missing(`tolerance') | missing(`maxiter') |                 ///
            `seed' < 0 | `probes' < 2 | `leveragebatch' <= 0 |          ///
            `targetbatch' <= 0 | `tolerance' <= 0 | `maxiter' <= 0 {
            di as err "invalid Rust solve tuning option"
            exit 198
        }
        local route = lower(strtrim("`route'"))
        if "`route'" == "" local route auto
        if !inlist("`route'", "auto", "exact", "diagonal", "cmg") {
            di as err "route() must be auto, exact, diagonal, or cmg"
            exit 198
        }
        local tolerance_arg = strtrim(strofreal(`tolerance', "%21.17f"))
        plugin call `plugin', solve `handle' `seed' `probes'              ///
            `leveragebatch' `targetbatch' `route' `tolerance_arg' `maxiter'
        return scalar handle = real("`handle'")
        return scalar seed = `seed'
        return scalar probes = `probes'
        return local route "`route'"
        return local backend "rust"
        return local subcommand "solve"
        exit
    }

    if "`subcommand'" == "prepare" {
        syntax varlist(min=6 max=6 numeric) [if] [in],                   ///
            [CLEANUP GENerate(name) MEMORYGib(real 4)]
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
        marksample touse, novarlist
        markout `touse' `varlist'

        local retained `generate'
        if "`retained'" == "" tempvar retained
        else confirm new variable `retained'
        quietly generate byte `retained' = 0

        local cleanup_arg nocleanup
        if "`cleanup'" != "" local cleanup_arg cleanup
        capture noisily plugin call `plugin' `touse' `varlist' `retained' ///
            if `touse', prepare `cleanup_arg' `memory_arg'
        local prepare_rc = _rc
        if `prepare_rc' {
            capture drop `retained'
            exit `prepare_rc'
        }

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
        return local retained_variable "`generate'"
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
        exit
    }

    di as err "unknown Rust backend subcommand: `subcommand'"
    di as err "valid subcommands: probe, capabilities, version, selftest, prepare, solve, result, snapshot, release, clear, lasterror"
    exit 198
end
