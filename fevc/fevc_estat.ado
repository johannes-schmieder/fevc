*! fevc postestimation 0.5.0-alpha.1 04sep2026

program define fevc_estat, rclass
    version 18.0
    if "`e(cmd)'" != "fevc" |                                ///
        inlist("`e(status)'", "WITHHELD", "ALPHA") {
        error 301
    }

    gettoken subcommand 0 : 0, parse(" ,")
    local subcommand = lower(strtrim("`subcommand'"))
    if "`subcommand'" == "" {
        di as err "estat subcommand required"
        di as err "available subcommands are decomposition, sample, computation, and diagnostics"
        exit 198
    }

    local length = strlen("`subcommand'")
    if `length' >= 3 & "`subcommand'" == substr("decomposition",1,`length') {
        _fevc_estat_decomposition `0'
    }
    else if `length' >= 3 & "`subcommand'" == substr("sample",1,`length') {
        _fevc_estat_sample `0'
    }
    else if `length' >= 3 & "`subcommand'" == substr("computation",1,`length') {
        _fevc_estat_computation `0'
    }
    else if `length' >= 3 & "`subcommand'" == substr("diagnostics",1,`length') {
        _fevc_estat_diagnostics `0'
    }
    else {
        di as err "estat subcommand `subcommand' is not allowed after fevc"
        exit 198
    }
end

program define _fevc_estat_decomposition
    version 18.0
    syntax [, FULL]
    if "`full'" != "" _fevc_display, decompositiononly full
    else _fevc_display, decompositiononly
end

program define _fevc_estat_sample
    version 18.0
    if strtrim(`"`0'"') != "" {
        di as err "estat sample does not allow options"
        exit 198
    }

    di as txt _newline "FEVC estimation sample"
    di as txt "{hline 61}"
    di as txt %-38s "Requested rows" as result %20.0fc e(N_requested)
    di as txt %-38s "Complete-case rows" as result %20.0fc e(N_complete)
    di as txt %-38s "Retained stored rows" as result             ///
        %20.0fc e(N_retained)
    di as txt %-38s "Retained physical observations" as result  ///
        %20.0fc e(N_physical)
    di as txt "{hline 61}"
    di as txt %-38s "Incomplete rows removed" as result          ///
        %20.0fc e(N_requested)-e(N_complete)

    if "`e(deletion)'" == "match" & "`e(stayers)'" == "both" {
        di as txt _newline "Mover-graph selection (stayers are assessed afterward)"
    }
    else di as txt _newline "Selection stages"
    di as txt %-38s "Rows in selected initial component"        ///
        as result %20.0fc e(N_initial_component)
    local selection_input = cond("`e(deletion)'"=="match",       ///
        "Mover input rows", "Leave-out input rows")
    local selection_removed = cond("`e(deletion)'"=="match",    ///
        "Rows removed before mover graph",                       ///
        "Rows removed before leave-out pruning")
    di as txt %-38s "`selection_input'" as result                ///
        %20.0fc e(N_mover_input)
    if "`e(deletion)'" == "match" & "`e(stayers)'" == "both" {
        tempname sample_accounting
        matrix `sample_accounting' = e(stayer_hybrid_sample_accounting)
        di as txt %-38s "Retained mover stored rows" as result  ///
            %20.0fc `sample_accounting'[1,1]
    }
    di as txt %-38s "Initial-component rows removed" as result  ///
        %20.0fc e(N_initial_component_dropped)
    di as txt %-38s "`selection_removed'" as result             ///
        %20.0fc e(N_mover_dropped)
    di as txt %-38s "Rows removed by graph pruning" as result   ///
        %20.0fc e(N_graph_dropped)

    if "`e(stayers)'" == "both" {
        di as txt _newline "Combined mover-stayer population"
        di as txt %-38s "Included stayer workers" as result      ///
            %20.0fc e(N_stayers)
        di as txt %-38s "Included stayer stored rows" as result ///
            %20.0fc e(N_stayer_rows)
        di as txt "Deletion convention: " as result             ///
            "`e(stayer_hybrid_deletion)'"
        di as txt "Assumption: " as result                       ///
            "`e(stayer_hybrid_assumption)'"
    }

    di as txt _newline "Retained design"
    di as txt %-38s "Worker levels" as result %20.0fc e(worker_levels)
    di as txt %-38s "Firm levels" as result %20.0fc e(firm_levels)
    di as txt %-38s "Deletion units" as result %20.0fc e(deletion_units)
    di as txt %-38s "Target-weight sum" as result               ///
        %20.7g e(target_weight_sum)
    di as txt "Target population: " as result "`e(target_population)'"
    di as txt "Sample rule: " as result "`e(sample_selection)'"

    if "`e(deletion)'" == "match" {
        di as txt _newline "Mover-graph pruning certificate"
        di as txt %-38s "Initial deletion edges" as result       ///
            %20.0fc e(graph_edges)
        di as txt %-38s "Retained deletion edges" as result      ///
            %20.0fc e(graph_retained_edges)
        di as txt %-38s "Insufficient-history workers removed"   ///
            as result %20.0fc e(graph_insufficient_workers)
        di as txt %-38s "Articulation workers removed" as result ///
            %20.0fc e(graph_articulation_workers)
        di as txt %-38s "Bridge deletion units removed" as result ///
            %20.0fc e(graph_bridge_units_removed)
        di as txt %-38s "Final bridge deletion units" as result ///
            %20.0fc e(graph_final_bridge_units)
        di as txt "Connectedness: " as result "`e(connectedness_status)'"
    }
end

program define _fevc_estat_computation
    version 18.0
    if strtrim(`"`0'"') != "" {
        di as err "estat computation does not allow options"
        exit 198
    }

    local engine = lower(strtrim(`"`e(engine_selected)'"'))
    if "`engine'" == "" | upper("`engine'") == "NOT_APPLICABLE" {
        local engine "not applicable"
    }
    local preconditioner = lower(strtrim(`"`e(preconditioner_selected)'"'))
    if "`preconditioner'" == "" |                         ///
        upper("`preconditioner'") == "NOT_APPLICABLE" {
        local preconditioner "not applicable"
    }

    di as txt _newline "FEVC computation"
    di as txt "Algorithm: " as result "`e(algorithm)'"
    di as txt "Backend: requested=" as result "`e(backend_requested)'" ///
        as txt "; selected=" as result "`e(backend_selected)'"
    di as txt "Backend routing: " as result "`e(backend_routing_reason)'"
    di as txt "RNG: requested=" as result "`e(rng_requested)'"  ///
        as txt "; selected=" as result "`e(rng_selected)'"

    if "`e(algorithm)'" == "jla" {
        di as txt "Engine: requested=" as result "`e(engine_requested)'" ///
            as txt "; selected=" as result "`engine'"
        di as txt "Preconditioner: requested=" as result        ///
            "`e(preconditioner_requested)'" as txt "; selected=" ///
            as result "`preconditioner'"
        di as txt "Routing: " as result "`e(routing_reason)'"
        di as txt "JLA work: probes=" as result %9.0fc e(probes) ///
            as txt "; batch=" as result %9.0fc e(batch)          ///
            as txt "; seed=" as result %12.0fc e(seed)
        di as txt "PCG: tolerance=" as result %10.3g e(tolerance) ///
            as txt "; maximum iterations=" as result            ///
            %10.0fc e(maxiter)
    }
    else {
        di as txt "Identified coefficients: " as result         ///
            %12.0fc e(parameters)
    }

    if e(backend_fallback) == 1 {
        di as txt "Backend fallback: " as result                 ///
            "`e(backend_fallback_reason)'" as txt " at "         ///
            as result "`e(backend_fallback_phase)'"
    }
    else di as txt "Backend fallback: " as result "none"

    local fallback = upper(strtrim(`"`e(fallback_status)'"'))
    if !inlist("`fallback'", "", "NOT_NEEDED", "NOT_APPLICABLE") {
        di as txt "Solver fallback: " as result "`e(fallback_status)'"
        di as txt "  `e(fallback_message)'"
    }
    di as txt "Active processors reported by Stata: " as result ///
        %9.0fc e(active_processors)
    di as txt "Resource envelope: " as result %8.3g e(memory_gib) ///
        as txt " GiB"
end

program define _fevc_estat_diagnostics
    version 18.0
    if strtrim(`"`0'"') != "" {
        di as err "estat diagnostics does not allow options"
        exit 198
    }

    di as txt _newline "FEVC numerical diagnostics"
    di as txt %-39s "Maximum leverage" as result                ///
        %20.7g e(max_leverage)
    if e(information_rcond) < . {
        di as txt %-39s "Information reciprocal condition"      ///
            as result %20.7g e(information_rcond)
    }
    if e(inverse_relres) < . {
        di as txt %-39s "Inverse relative residual" as result   ///
            %20.7g e(inverse_relres)
    }
    if "`e(algorithm)'" == "jla" {
        di as txt %-39s "Maximum solver iterations" as result   ///
            %20.0fc e(solver_iterations)
        di as txt %-39s "Maximum complete-system residual"      ///
            as result %20.7g e(solver_max_residual)
        if e(correction_reciprocal_residual) < . {
            di as txt %-39s "Correction reciprocal residual"    ///
                as result %20.7g e(correction_reciprocal_residual)
        }
        if e(target_identity_residual) < . {
            di as txt %-39s "Target identity residual" as result ///
                %20.7g e(target_identity_residual)
        }
    }
    di as txt %-39s "Weighted residual sum of squares" as result ///
        %20.7g e(weighted_rss)
    di as txt %-39s "Target-weighted outcome variance" as result ///
        %20.7g e(target_outcome_variance)
    di as txt %-39s "Regression-weighted outcome variance"       ///
        as result %20.7g e(regression_outcome_variance)
    di as txt %-39s "Residual variance" as result               ///
        %20.7g e(residual_variance)

    if e(memory_forecast_bytes) > 0 & e(memory_forecast_bytes) < . {
        di as txt %-39s "Forecast peak memory (GiB)" as result   ///
            %20.4f e(memory_forecast_bytes)/1073741824
    }
    if e(fit_seconds) < . {
        di as txt _newline "Stage timings (seconds)"
        di as txt %-39s "Sample selection" as result            ///
            %20.4f e(sample_selection_seconds)
        di as txt %-39s "Fit" as result %20.4f e(fit_seconds)
        di as txt %-39s "Leverage" as result                    ///
            %20.4f e(leverage_seconds)
        di as txt %-39s "Target calculation" as result          ///
            %20.4f e(target_seconds)
        di as txt %-39s "Correction" as result                  ///
            %20.4f e(correction_seconds)
        di as txt %-39s "Validation" as result                  ///
            %20.4f e(validation_seconds)
    }

    if e(numerical_mcse_available) {
        tempname mcse
        matrix `mcse' = e(numerical_mcse)
        di as txt _newline "JLA numerical MCSE (conditional on leverage sketch)"
        di as txt %-39s "Worker variance" as result             ///
            %20.7g `mcse'[1,1]
        di as txt %-39s "Firm variance" as result               ///
            %20.7g `mcse'[1,2]
        di as txt %-39s "Sorting: 2 x covariance" as result     ///
            %20.7g 2*`mcse'[1,3]
        di as txt %-39s "Total worker-firm variance" as result  ///
            %20.7g `mcse'[1,4]
        di as txt "These quantify randomized numerical error, not sampling uncertainty."
    }
    if inlist("`e(inference_model)'", "structured_common", "structured_leverage") {
        _fevc_display, inferencediagnosticsonly
    }
    di as txt _newline "Type " as result "ereturn list" as txt ///
        " for the complete machine-readable diagnostic record."
end
