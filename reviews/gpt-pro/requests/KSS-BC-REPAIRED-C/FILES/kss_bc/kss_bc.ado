*! kss_bc 0.1.0-dev 14aug2026

program define kss_bc, eclass sortpreserve
    version 18.0

    if lower(strtrim(`"`0'"')) == ", version" {
        ereturn clear
        ereturn local cmd "kss_bc"
        ereturn local version "0.1.0-dev"
        ereturn local model "linear"
        ereturn local correction "kss"
        ereturn local status "DEVELOPMENT"
        di as txt "kss_bc 0.1.0-dev (14aug2026)"
        exit
    }

    syntax varlist(numeric fv min=1) [if] [in] [fw],             ///
        WORKER(varname) FIRM(varname) [                          ///
        DELETION(string) DELETIONID(varname)                     ///
        ALGORITHM(string) NUISANCE(string)                       ///
        TARGETWeight(varname numeric) STAYERS(string)            ///
        PROBES(integer 200) BATCH(integer 8)                     ///
        SEED(integer 8675309) TOLerance(real 1e-10)              ///
        MAXIter(integer 10000) EXACT_limit(integer 500)          ///
        RANK_tolerance(real 1e-10) BLOCK_tolerance(real 1e-10)   ///
        BLOCKSIZE_limit(integer 5000) NODISPlay                  ///
    ]

    gettoken depvar controls : varlist
    capture confirm numeric variable `depvar'
    if _rc {
        quietly _kss_bc_post_failure "INVALID_DEPVAR"
        di as error "the dependent variable must be a numeric variable"
        exit 109
    }
    if "`worker'" == "`firm'" {
        quietly _kss_bc_post_failure "INVALID_IDENTIFIER"
        di as error "worker() and firm() must identify distinct dimensions"
        exit 198
    }

    if "`deletion'" == "" local deletion match
    local deletion = lower(strtrim("`deletion'"))
    if !inlist("`deletion'", "match", "observation") {
        quietly _kss_bc_post_failure "UNSUPPORTED_DELETION"
        di as error "deletion() must be match or observation"
        exit 198
    }
    if "`deletion'" == "observation" & "`deletionid'" != "" {
        quietly _kss_bc_post_failure "UNSUPPORTED_DELETION_ID"
        di as error "deletionid() is not allowed with deletion(observation)"
        exit 198
    }

    if "`algorithm'" == "" local algorithm auto
    local algorithm = lower(strtrim("`algorithm'"))
    if !inlist("`algorithm'", "auto", "exact", "jla") {
        quietly _kss_bc_post_failure "UNSUPPORTED_ALGORITHM"
        di as error "algorithm() must be auto, exact, or jla"
        exit 198
    }
    if "`nuisance'" == "" local nuisance joint
    local nuisance = lower(strtrim("`nuisance'"))
    if !inlist("`nuisance'", "joint", "fixedoffset") {
        quietly _kss_bc_post_failure "INVALID_NUISANCE"
        di as error "nuisance() must be joint or fixedoffset"
        exit 198
    }
    if "`stayers'" == "" local stayers movers
    local stayers = lower(strtrim("`stayers'"))
    if !inlist("`stayers'", "movers", "both") {
        quietly _kss_bc_post_failure "INVALID_STAYER_CONVENTION"
        di as error "stayers() must be movers or both"
        exit 198
    }
    if "`deletion'" == "observation" & "`stayers'" == "both" {
        quietly _kss_bc_post_failure "UNSUPPORTED_STAYER_CONVENTION"
        di as error "stayers(both) applies only to deletion(match)"
        exit 198
    }
    if "`stayers'" == "both" {
        quietly _kss_bc_post_failure "STAYER_HYBRID_NOT_IMPLEMENTED"
        di as error "stayers(both) is withheld until the separately labeled hybrid target is implemented"
        exit 498
    }

    if `probes' < 2 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "probes() must be at least two"
        exit 198
    }
    if `batch' < 1 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "batch() must be positive"
        exit 198
    }
    if `seed' < 0 | `seed' > 2147483646 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "seed() must be between zero and 2,147,483,646"
        exit 198
    }
    if `tolerance' < 1e-15 | `tolerance' >= 1 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "tolerance() must lie in [1e-15,1)"
        exit 198
    }
    if `maxiter' < 1 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "maxiter() must be positive"
        exit 198
    }
    if `exact_limit' < 2 | `exact_limit' > 2000 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "exact_limit() must be between 2 and 2,000"
        exit 198
    }
    if `rank_tolerance' < 1e-14 | `rank_tolerance' >= 0.1 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "rank_tolerance() must lie in [1e-14,0.1)"
        exit 198
    }
    if `block_tolerance' < 1e-14 | `block_tolerance' >= 1 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "block_tolerance() must lie in [1e-14,1)"
        exit 198
    }
    if `blocksize_limit' < 1 | `blocksize_limit' > 1000000 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "blocksize_limit() must be between 1 and 1,000,000"
        exit 198
    }

    tempvar requested touse
    mark `requested' `if' `in'
    quietly count if `requested'
    local N_scope = r(N)
    local weightvar
    if "`weight'" != "" {
        local weightvar = substr("`exp'",2,.)
        capture confirm numeric variable `weightvar'
        if _rc {
            quietly _kss_bc_post_failure "INVALID_FREQUENCY"
            di as error "the frequency weight must be a numeric variable"
            exit 198
        }
        quietly count if `requested' & missing(`weightvar')
        if r(N) {
            quietly _kss_bc_post_failure "INVALID_FREQUENCY"
            di as error "frequency weights must be finite and strictly positive"
            exit 198
        }
        quietly summarize `weightvar' if `requested', meanonly
        if r(min) <= 0 | r(max) >= . {
            quietly _kss_bc_post_failure "INVALID_FREQUENCY"
            di as error "frequency weights must be finite and strictly positive"
            exit 198
        }
        quietly count if `weightvar' != floor(`weightvar') & `requested'
        if r(N) {
            quietly _kss_bc_post_failure "INVALID_FREQUENCY"
            di as error "frequency weights must be positive integers"
            exit 198
        }
    }

    if "`targetweight'" != "" {
        quietly count if `requested' & missing(`targetweight')
        if r(N) {
            quietly _kss_bc_post_failure "INVALID_TARGET_WEIGHT"
            di as error "targetweight() must be finite, nonnegative, and have positive total mass"
            exit 198
        }
        quietly summarize `targetweight' if `requested', meanonly
        if r(min) < 0 | r(max) >= . | r(sum) <= 0 {
            quietly _kss_bc_post_failure "INVALID_TARGET_WEIGHT"
            di as error "targetweight() must be finite, nonnegative, and have positive total mass"
            exit 198
        }
    }

    mark `touse' `if' `in'
    markout `touse' `depvar'
    markout `touse' `worker' `firm', strok
    if "`deletionid'" != "" markout `touse' `deletionid', strok

    local controlvars
    if strtrim(`"`controls'"') != "" {
        capture quietly fvrevar `controls' if `touse'
        if _rc {
            quietly _kss_bc_post_failure "INVALID_CONTROLS"
            di as error "controls could not be expanded into numeric columns"
            exit _rc
        }
        local expanded_controls `r(varlist)'
        if strtrim(`"`expanded_controls'"') != "" {
            markout `touse' `expanded_controls'
        }
        foreach control of local expanded_controls {
            // fvrevar materializes omitted factor bases as all-zero columns.
            // They carry no design information and must not enter the rank gate.
            quietly count if `control' != 0 & `touse'
            if r(N) local controlvars `controlvars' `control'
        }
        if strtrim(`"`controlvars'"') != "" markout `touse' `controlvars'
    }

    quietly count if `touse'
    local N_complete = r(N)
    if "`deletion'" == "match" & `N_complete' != `N_scope' {
        local missing_rows = `N_scope' - `N_complete'
        quietly _kss_bc_post_failure "MATCH_INPUT_MISSING"
        di as error "match deletion requires complete frozen inputs; `missing_rows' requested row(s) are incomplete"
        exit 459
    }
    if `N_complete' == 0 {
        quietly _kss_bc_post_failure "NO_USABLE_OBSERVATIONS"
        error 2000
    }

    tempvar frequency target
    if "`weightvar'" == "" quietly generate double `frequency' = 1 if `touse'
    else quietly generate double `frequency' = `weightvar' if `touse'
    if "`targetweight'" == "" quietly generate double `target' = `frequency' if `touse'
    else quietly generate double `target' = `targetweight' if `touse'

    tempvar initial_worker initial_firm pair_first firm_count worker_tag
    quietly egen long `initial_worker' = group(`worker') if `touse'
    quietly egen long `initial_firm' = group(`firm') if `touse'
    sort `initial_worker' `initial_firm'
    quietly by `initial_worker' `initial_firm': generate byte `pair_first' = ///
        (_n == 1) if `touse'
    quietly by `initial_worker': egen long `firm_count' = total(`pair_first') ///
        if `touse'
    quietly egen byte `worker_tag' = tag(`initial_worker') if `touse'
    quietly count if `worker_tag' & `firm_count' == 1 & `touse'
    local N_stayers = r(N)
    quietly count if `firm_count' == 1 & `touse'
    local N_stayer_rows = r(N)

    capture mata: assert(kssbc__api_level() == 6)
    if _rc {
        capture findfile kss_bc.mata
        if _rc {
            di as error "kss_bc.mata was not found on the Stata adopath"
            exit 601
        }
        quietly do `"`r(fn)'"'
    }

    tempvar graph_worker graph_firm graph_deletion graph_keep
    quietly egen long `graph_worker' = group(`worker') if `touse'
    quietly egen long `graph_firm' = group(`firm') if `touse'
    if "`deletion'" == "observation" {
        quietly generate double `graph_deletion' = _n if `touse'
    }
    else if "`deletionid'" != "" {
        quietly egen long `graph_deletion' = group(`deletionid') if `touse'
    }
    else {
        quietly egen long `graph_deletion' = group(`graph_worker' `graph_firm') ///
            if `touse'
    }
    quietly generate byte `graph_keep' = 0
    tempname graph_diagnostics
    local graph_status
    local graph_message
    capture noisily mata: kssbc__stata_prune_graph(                ///
        "`graph_worker'", "`graph_firm'", "`frequency'",       ///
        "`graph_deletion'", "`touse'", "`deletion'",          ///
        "`graph_keep'", "`graph_diagnostics'",                 ///
        "graph_status", "graph_message")
    if _rc {
        local graph_rc = _rc
        quietly _kss_bc_post_failure "GRAPH_RUNTIME_FAILED"
        di as error "leave-out graph construction stopped unexpectedly"
        exit `graph_rc'
    }
    if "`graph_status'" != "CONVERGED" {
        local failure_status `graph_status'
        local failure_message `"`graph_message'"'
        quietly _kss_bc_post_failure "`failure_status'"
        di as error `"`failure_message'"'
        if inlist("`failure_status'", "INVALID_GRAPH_INPUT",       ///
            "CROSS_COORDINATE_MATCH", "UNSUPPORTED_DELETION") exit 198
        if "`failure_status'" == "NO_MOVER_SAMPLE" exit 2000
        exit 498
    }
    quietly replace `touse' = `touse' & `graph_keep'
    quietly count if `touse'
    local N_retained = r(N)
    if `N_retained' == 0 {
        quietly _kss_bc_post_failure "NO_LEAVEOUT_COMPONENT"
        di as error "no observations remain in the leave-out-connected component"
        exit 498
    }
    local N_mover_input = `graph_diagnostics'[1,8]
    local N_initial_component = `graph_diagnostics'[1,9]
    local N_initial_component_dropped = `N_complete' - `N_initial_component'
    local N_mover_dropped = `N_initial_component' - `N_mover_input'
    local N_graph_dropped = `N_mover_input' - `N_retained'

    tempvar id_worker id_firm deletion_id
    quietly egen long `id_worker' = group(`worker') if `touse'
    quietly egen long `id_firm' = group(`firm') if `touse'
    if "`deletion'" == "observation" {
        quietly generate double `deletion_id' = _n if `touse'
    }
    else if "`deletionid'" != "" {
        quietly egen long `deletion_id' = group(`deletionid') if `touse'
    }
    else {
        quietly egen long `deletion_id' = group(`id_worker' `id_firm') if `touse'
    }

    quietly summarize `id_worker' if `touse', meanonly
    local worker_levels = r(max)
    quietly summarize `id_firm' if `touse', meanonly
    local firm_levels = r(max)
    local control_count : word count `controlvars'
    local parameters = `worker_levels' + `firm_levels' - 1 + `control_count'
    local selected_algorithm `algorithm'
    if "`selected_algorithm'" == "auto" {
        if `parameters' <= `exact_limit' local selected_algorithm exact
        else local selected_algorithm jla
    }

    // Canonicalize the stored-row order before indexed random projections.
    // Exact duplicate rows are exchangeable under both deletion contracts.
    if "`deletion'" == "match" {
        sort `id_worker' `id_firm' `deletion_id' `controlvars' `depvar' ///
            `frequency' `target'
    }
    else {
        sort `id_worker' `id_firm' `controlvars' `depvar' `frequency' `target'
    }
    tempname raw_results diagnostics plugin correction corrected kss_return mcse
    local mata_status
    local mata_message
    if "`selected_algorithm'" == "exact" {
        capture noisily mata: kssbc__stata_exact(                  ///
            "`depvar'", "`id_worker'", "`id_firm'",             ///
            `"`controlvars'"', "`frequency'", "`target'",      ///
            "`deletion_id'", "`touse'", "`deletion'",         ///
            "`nuisance'", `rank_tolerance', `block_tolerance',  ///
            `exact_limit', `blocksize_limit', "`raw_results'",  ///
            "mata_status", "mata_message", "`diagnostics'")
    }
    else {
        capture noisily mata: kssbc__stata_jla(                    ///
            "`depvar'", "`id_worker'", "`id_firm'",             ///
            `"`controlvars'"', "`frequency'", "`target'",      ///
            "`deletion_id'", "`touse'", "`deletion'",         ///
            "`nuisance'", `probes', `batch', `seed',            ///
            `tolerance', `maxiter', `rank_tolerance',            ///
            `block_tolerance', `blocksize_limit',                ///
            "`raw_results'", "mata_status", "mata_message",   ///
            "`diagnostics'")
    }
    if _rc {
        local mata_rc = _rc
        quietly _kss_bc_post_failure "MATA_RUNTIME_FAILED"
        di as error "the Mata backend stopped unexpectedly"
        exit `mata_rc'
    }
    if "`mata_status'" != "CONVERGED" {
        local failure_status `mata_status'
        local failure_message `"`mata_message'"'
        quietly _kss_bc_post_failure "`failure_status'"
        di as error `"`failure_message'"'
        if inlist("`failure_status'", "EXACT_SIZE_LIMIT", "INVALID_INPUT", ///
            "INVALID_FREQUENCY", "INVALID_TARGET_WEIGHT",                 ///
            "INVALID_IDENTIFIER", "UNSUPPORTED_DELETION") exit 198
        if inlist("`failure_status'", "INVALID_NUISANCE",                  ///
            "INVALID_TOLERANCE", "BLOCK_SIZE_LIMIT",                     ///
            "CROSS_COORDINATE_MATCH") exit 198
        exit 498
    }

    matrix colnames `raw_results' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix rownames `raw_results' = plugin bias_correction corrected ///
        numerical_mcse
    matrix `plugin' = `raw_results'[1,1..4]
    matrix `correction' = `raw_results'[2,1..4]
    matrix `corrected' = `raw_results'[3,1..4]
    matrix `kss_return' = `corrected'
    matrix `mcse' = `raw_results'[4,1..4]
    matrix colnames `plugin' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `correction' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `corrected' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `mcse' = worker_variance firm_variance ///
        worker_firm_covariance total_variance

    quietly summarize `frequency' if `touse', meanonly
    local N_physical = r(sum)
    ereturn clear
    ereturn post `corrected', obs(`N_physical') esample(`touse') ///
        depname(`depvar')
    ereturn matrix plugin = `plugin'
    ereturn matrix correction = `correction'
    ereturn matrix kss = `kss_return'
    ereturn matrix numerical_mcse = `mcse'
    ereturn matrix results = `raw_results'
    ereturn scalar N_stored = `diagnostics'[1,1]
    ereturn scalar N_physical = `diagnostics'[1,2]
    ereturn scalar N_requested = `N_scope'
    ereturn scalar N_complete = `N_complete'
    ereturn scalar N_retained = `N_retained'
    ereturn scalar N_mover_input = `N_mover_input'
    ereturn scalar N_initial_component = `N_initial_component'
    ereturn scalar N_initial_component_dropped = ///
        `N_initial_component_dropped'
    ereturn scalar N_mover_dropped = `N_mover_dropped'
    ereturn scalar N_graph_dropped = `N_graph_dropped'
    ereturn scalar graph_edges = `graph_diagnostics'[1,5]
    ereturn scalar graph_articulation_workers = ///
        `graph_diagnostics'[1,6]
    ereturn scalar graph_leaveout_components = `graph_diagnostics'[1,7]
    ereturn scalar graph_initial_component_rows = `graph_diagnostics'[1,9]
    ereturn scalar graph_insufficient_workers = ///
        `graph_diagnostics'[1,10]
    ereturn scalar graph_pruning_iterations = `graph_diagnostics'[1,11]
    ereturn scalar graph_seconds = `graph_diagnostics'[1,12]
    ereturn scalar N_stayers = `N_stayers'
    ereturn scalar N_stayer_rows = `N_stayer_rows'
    ereturn scalar worker_levels = `diagnostics'[1,3]
    ereturn scalar firm_levels = `diagnostics'[1,4]
    ereturn scalar parameters = `diagnostics'[1,5]
    ereturn scalar deletion_units = `diagnostics'[1,6]
    ereturn scalar target_weight_sum = `diagnostics'[1,7]
    ereturn scalar max_leverage = `diagnostics'[1,8]
    ereturn scalar information_rcond = `diagnostics'[1,9]
    ereturn scalar inverse_relres = `diagnostics'[1,10]
    ereturn scalar solver_iterations = `diagnostics'[1,11]
    ereturn scalar solver_max_residual = `diagnostics'[1,12]
    ereturn scalar probes = `diagnostics'[1,13]
    ereturn scalar weighted_rss = `diagnostics'[1,14]
    ereturn scalar fit_seconds = `diagnostics'[1,15]
    ereturn scalar leverage_seconds = `diagnostics'[1,16]
    ereturn scalar target_seconds = `diagnostics'[1,17]
    ereturn scalar correction_seconds = `diagnostics'[1,18]
    ereturn scalar preconditioner_seconds = `diagnostics'[1,19]
    ereturn scalar preconditioner_ratio = `diagnostics'[1,20]
    ereturn scalar control_schur_rcond = `diagnostics'[1,21]
    ereturn scalar deletion_rank_gap = `diagnostics'[1,22]
    ereturn scalar batch = `batch'
    ereturn scalar seed = `seed'
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar numerical_mcse_available = ///
        ("`selected_algorithm'" == "jla")
    ereturn local cmd "kss_bc"
    ereturn local cmdline `"kss_bc `0'"'
    ereturn local version "0.1.0-dev"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local algorithm "`selected_algorithm'"
    ereturn local deletion "`deletion'"
    ereturn local nuisance "`nuisance'"
    ereturn local target_population = cond("`deletion'" == "match", ///
        "movers", "retained observations")
    ereturn local sample_selection = cond("`deletion'" == "match", ///
        "MOVERS_MATLAB_LEAVEONEWORKER_COMPONENT",                  ///
        "MATLAB_LEAVEONEWORKER_COMPONENT")
    ereturn local connectedness_status "LEAVE_ONE_WORKER_CONNECTED"
    ereturn local frequency_convention "literal physical copies"
    ereturn local targetweight_convention ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local inference "not implemented"
    ereturn local numerical_error = cond("`selected_algorithm'" == "exact", ///
        "deterministic exact backend", "conditional probe MCSE")
    if "`selected_algorithm'" == "exact" {
        ereturn local deletion_rank_certificate "exact block inverse"
    }
    else if "`nuisance'" == "joint" & `control_count' > 0 {
        ereturn local deletion_rank_certificate "within-cell trace bound"
    }
    else {
        ereturn local deletion_rank_certificate "FE graph and spectral JLA gate"
    }
    ereturn local status "KSS_POINT_ESTIMATES_ONLY"

    if "`nodisplay'" == "" _kss_bc_display
end

program define _kss_bc_post_failure, eclass
    version 18.0
    args failure_status
    ereturn clear
    ereturn local cmd "kss_bc"
    ereturn local version "0.1.0-dev"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local status "WITHHELD"
    ereturn local withholding_status "`failure_status'"
end

program define _kss_bc_display
    version 18.0
    di as txt _newline "KSS leave-out bias-corrected variance components"
    di as txt "Deletion: " as result "`e(deletion)'"              ///
        as txt "   Algorithm: " as result "`e(algorithm)'"       ///
        as txt "   Nuisance: " as result "`e(nuisance)'"
    matlist e(results), names(rows) format(%12.8g)
    di as txt "Target population: `e(target_population)'"
    di as txt "Point estimates only; econometric inference is not implemented."
end
