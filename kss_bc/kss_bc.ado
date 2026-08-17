*! kss_bc 0.2.0-dev 16aug2026

program define kss_bc, eclass
    version 18.0

    if lower(strtrim(`"`0'"')) == ", version" {
        _kss_bc_impl `0'
        exit
    }

    // A cached compressed design is command-local state.  Clear a current
    // scale runtime defensively at entry so no interrupted prior invocation
    // can leak state into this estimate.
    capture mata: assert(kssbc_scale__api_level() == 2 &          ///
        kssbc_scale__build_id() ==                               ///
        "kss-bc-scale-api2-cached-state")
    if !_rc capture mata: kssbc_scale_runtime__reset()

    capture mata: kssbc_rng__api_level()
    local rng_runtime_loaded = (_rc == 0)
    capture mata: assert(kssbc_rng__api_level() == 2 &              ///
        kssbc_rng__build_id() ==                                   ///
        "kss-bc-rng-k1-mt64s-complete-guard-v2")
    if _rc {
        if `rng_runtime_loaded' {
            quietly _kss_bc_post_failure "STALE_RNG_RUNTIME"
            di as error "a different KSS RNG runtime is already loaded; restart Stata or run discard before retrying"
            exit 498
        }
        capture findfile kss_bc_rng.mata
        if _rc {
            quietly _kss_bc_post_failure "RNG_RUNTIME_NOT_FOUND"
            di as error "kss_bc_rng.mata was not found on the Stata adopath"
            exit 601
        }
        quietly do `"`r(fn)'"'
        capture mata: assert(kssbc_rng__api_level() == 2 &          ///
            kssbc_rng__build_id() ==                               ///
            "kss-bc-rng-k1-mt64s-complete-guard-v2")
        if _rc {
            quietly _kss_bc_post_failure "INVALID_RNG_RUNTIME"
            di as error "the installed KSS RNG runtime is incompatible with this command"
            exit 498
        }
    }

    tempname outer_rng_guard_rc outer_rng_restore_rc
    mata: st_numscalar("`outer_rng_guard_rc'",kssbc_rng__guard_begin())
    if scalar(`outer_rng_guard_rc') {
        quietly _kss_bc_post_failure "RNG_GUARD_FAILED"
        di as error "caller RNG and sort-jumbler state could not be captured"
        exit 498
    }
    capture quietly _kss_bc_stage_timer_ids
    if _rc {
        mata: st_numscalar("`outer_rng_restore_rc'",               ///
            kssbc_rng__guard_restore())
        quietly _kss_bc_post_failure "TIMER_RESERVATION_FAILED"
        di as error "two free command-stage timer IDs were not available"
        exit 498
    }
    local stage_selection_timer = r(selection_timer)
    local stage_validation_timer = r(validation_timer)
    global KSS_BC_STAGE_SELECTION_TIMER `stage_selection_timer'
    global KSS_BC_STAGE_VALIDATION_TIMER `stage_validation_timer'
    capture noisily _kss_bc_impl `0'
    local command_rc = _rc
    local outer_scale_reset_rc = 0
    capture mata: assert(kssbc_scale__api_level() == 2 &          ///
        kssbc_scale__build_id() ==                               ///
        "kss-bc-scale-api2-cached-state")
    if !_rc {
        capture mata: kssbc_scale_runtime__reset()
        local outer_scale_reset_rc = _rc
    }
    mata: st_numscalar("`outer_rng_restore_rc'",kssbc_rng__guard_restore())
    foreach stage_timer in `stage_selection_timer'                ///
        `stage_validation_timer' {
        capture quietly timer off `stage_timer'
        capture quietly timer clear `stage_timer'
    }
    macro drop KSS_BC_STAGE_SELECTION_TIMER
    macro drop KSS_BC_STAGE_VALIDATION_TIMER
    if scalar(`outer_rng_restore_rc') {
        quietly _kss_bc_post_failure "RNG_RESTORE_FAILED"
        di as error "caller RNG and sort-jumbler state could not be restored"
        exit 498
    }
    if `outer_scale_reset_rc' & !`command_rc' {
        quietly _kss_bc_post_failure "SCALE_STATE_RELEASE_FAILED"
        di as error "cached compressed state could not be released"
        exit 498
    }
    exit `command_rc'
end

program define _kss_bc_impl, eclass sortpreserve
    version 18.0

    if lower(strtrim(`"`0'"')) == ", version" {
        ereturn clear
        ereturn local cmd "kss_bc"
        ereturn local version "0.2.0-dev"
        ereturn local model "linear"
        ereturn local correction "kss"
        ereturn local status "DEVELOPMENT"
        di as txt "kss_bc 0.2.0-dev (16aug2026)"
        exit
    }

    syntax varlist(numeric fv min=1) [if] [in] [fw],             ///
        WORKER(varname) FIRM(varname) [                          ///
        DELETION(string) DELETIONID(varname)                     ///
        ALGORITHM(string) NUISANCE(string)                       ///
        TARGETWeight(varname numeric) STAYERS(string)            ///
        PROBEOrder(varname numeric)                              ///
        PROBES(integer 200) BATCH(string)                        ///
        ENGINE(string) WALLSeconds(integer 43200)                ///
        PREConditioner(string) MEMory_gib(real 4)                ///
        SEED(integer 8675309) TOLerance(real 1e-10)              ///
        MAXIter(integer 10000) EXACT_limit(integer 500)          ///
        RANK_tolerance(real 1e-10) BLOCK_tolerance(real 1e-10)   ///
        BLOCKSIZE_limit(integer 5000)                            ///
        PHYSICAL_limit(integer 50000000) NODISPlay               ///
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

    if "`preconditioner'" == "" local preconditioner auto
    local preconditioner = lower(strtrim("`preconditioner'"))
    if !inlist("`preconditioner'", "auto", "diagonal", "cmg") {
        quietly _kss_bc_post_failure "INVALID_PRECONDITIONER"
        di as error "preconditioner() must be auto, diagonal, or cmg"
        exit 198
    }
    if `memory_gib' < 1 | `memory_gib' > 56 | missing(`memory_gib') {
        quietly _kss_bc_post_failure "INVALID_MEMORY_ENVELOPE"
        di as error "memory_gib() must lie in [1,56]"
        exit 198
    }
    if "`engine'" == "" local engine auto
    local engine_requested = lower(strtrim("`engine'"))
    if !inlist("`engine_requested'", "auto", "compressed", "generic") {
        quietly _kss_bc_post_failure "INVALID_ENGINE"
        di as error "engine() must be auto, compressed, or generic"
        exit 198
    }
    if `wallseconds' < 300 | `wallseconds' > 43200 {
        quietly _kss_bc_post_failure "INVALID_WALL_ENVELOPE"
        di as error "wallseconds() must lie in [300,43200]"
        exit 198
    }
    if "`batch'" == "" local batch auto
    local batch_requested = lower(strtrim("`batch'"))
    local batch_routing_reason "caller supplied an explicit batch width"
    local batch_memory_budget_bytes = .
    local batch_column_forecast_bytes = .
    local batch_scratch_forecast_bytes = .
    if "`batch_requested'" == "auto" {
        // The production policy is finalized after the retained sample and
        // parameter dimensions are known.  Eight is the safe small-sample
        // default and the minimum production matrix-matrix width.
        local batch 8
        local batch_routing_reason "small-sample automatic batch floor"
    }
    else {
        capture confirm integer number `batch_requested'
        if _rc {
            quietly _kss_bc_post_failure "INVALID_TUNING"
            di as error "batch() must be auto or a positive integer"
            exit 198
        }
        local batch = real("`batch_requested'")
    }
    local active_processors = c(processors)

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
    if `tolerance' < 1e-15 | `tolerance' > 1e-4 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "tolerance() must lie in [1e-15,1e-4]"
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
    if `physical_limit' < 1 | `physical_limit' > 1000000000 {
        quietly _kss_bc_post_failure "INVALID_TUNING"
        di as error "physical_limit() must be between 1 and 1,000,000,000"
        exit 198
    }

    quietly timer on $KSS_BC_STAGE_SELECTION_TIMER
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
    if "`probeorder'" != "" {
        quietly count if `requested' & missing(`probeorder')
        if r(N) {
            quietly _kss_bc_post_failure "INVALID_PROBE_ORDER"
            di as error "probeorder() must be complete on the requested sample"
            exit 198
        }
        tempvar probeorder_tag
        quietly egen byte `probeorder_tag' = tag(`probeorder') if `requested'
        quietly count if `requested'
        local probeorder_rows = r(N)
        quietly count if `requested' & `probeorder_tag'
        if r(N) != `probeorder_rows' {
            quietly _kss_bc_post_failure "INVALID_PROBE_ORDER"
            di as error "probeorder() must uniquely identify every requested stored row"
            exit 198
        }
    }

    local controlvars
    if strtrim(`"`controls'"') != "" {
        capture quietly fvexpand `controls' if `touse'
        if _rc {
            quietly _kss_bc_post_failure "INVALID_CONTROLS"
            di as error "controls could not be expanded into numeric columns"
            exit _rc
        }
        local expanded_terms `r(varlist)'
        capture quietly fvrevar `expanded_terms' if `touse'
        if _rc {
            quietly _kss_bc_post_failure "INVALID_CONTROLS"
            di as error "controls could not be materialized as numeric columns"
            exit _rc
        }
        local expanded_controls `r(varlist)'
        local term_count : word count `expanded_terms'
        local control_count_expanded : word count `expanded_controls'
        if `term_count' != `control_count_expanded' {
            quietly _kss_bc_post_failure "INVALID_CONTROLS"
            di as error "factor-variable expansion did not preserve control metadata"
            exit 498
        }
        forvalues control_index = 1/`term_count' {
            local term : word `control_index' of `expanded_terms'
            local control : word `control_index' of `expanded_controls'
            capture quietly _ms_parse_parts `term'
            if _rc {
                quietly _kss_bc_post_failure "INVALID_CONTROLS"
                di as error "expanded control metadata could not be parsed"
                exit 498
            }
            // Only Stata-designated omitted factor terms are removed.  A
            // user-supplied zero or collinear regressor remains in the design
            // and must be rejected by the registered rank gates.
            if !r(omit) local controlvars `controlvars' `control'
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

    local expected_mata_build "kss-bc-api19-scale-experimental"
    capture mata: kssbc__api_level()
    local mata_runtime_loaded = (_rc == 0)
    capture mata: assert(kssbc__api_level() == 19 &                 ///
        kssbc__version() == "0.2.0-dev" &                         ///
        kssbc__build_id() == "`expected_mata_build'")
    if _rc {
        if `mata_runtime_loaded' {
            quietly _kss_bc_post_failure "STALE_MATA_RUNTIME"
            di as error "a different kss_bc Mata runtime is already loaded; restart Stata or run discard before retrying"
            exit 498
        }
        capture findfile kss_bc.mata
        if _rc {
            di as error "kss_bc.mata was not found on the Stata adopath"
            exit 601
        }
        quietly do `"`r(fn)'"'
        capture mata: assert(kssbc__api_level() == 19 &             ///
            kssbc__version() == "0.2.0-dev" &                     ///
            kssbc__build_id() == "`expected_mata_build'")
        if _rc {
            quietly _kss_bc_post_failure "INVALID_MATA_RUNTIME"
            di as error "the loaded kss_bc Mata runtime does not match this command build"
            exit 498
        }
    }

    // Ordinary binary64 summation cannot distinguish every integer total
    // above 2^53.  Reject such inputs before graph ranking so component mass,
    // observation-unit counts, and e(N_physical) remain literal integers.
    tempname exact_physical_total
    mata: st_numscalar("`exact_physical_total'",                  ///
        kssbc__exact_physical_total(                              ///
        st_data(., "`frequency'", "`touse'")))
    if missing(scalar(`exact_physical_total')) {
        quietly _kss_bc_post_failure "PHYSICAL_TOTAL_LIMIT"
        di as error "literal frequency total exceeds the exact binary64 integer range (2^53)"
        exit 498
    }

    capture mata: assert(kssbc_graph__api_level() == 18 &          ///
        kssbc_graph__build_id() ==                                 ///
        "kss-bc-graph-api18-deletion-multigraph-fixed-point")
    if _rc {
        capture findfile kss_bc_graph.mata
        if _rc {
            quietly _kss_bc_post_failure "GRAPH_RUNTIME_NOT_FOUND"
            di as error "kss_bc_graph.mata was not found on the Stata adopath"
            exit 601
        }
        quietly do `"`r(fn)'"'
        capture mata: assert(kssbc_graph__api_level() == 18 &      ///
            kssbc_graph__build_id() ==                             ///
            "kss-bc-graph-api18-deletion-multigraph-fixed-point")
        if _rc {
            quietly _kss_bc_post_failure "INVALID_GRAPH_RUNTIME"
            di as error "the loaded graph runtime does not match this command build"
            exit 498
        }
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
    capture noisily mata: kssbc_graph__stata_prune(                ///
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
    tempname retained_physical_total
    mata: st_numscalar("`retained_physical_total'",               ///
        kssbc__exact_physical_total(                              ///
        st_data(., "`frequency'", "`touse'")))
    if missing(scalar(`retained_physical_total')) {
        quietly _kss_bc_post_failure "MATA_RUNTIME_FAILED"
        di as error "retained literal frequency total could not be certified"
        exit 498
    }
    local retained_physical = scalar(`retained_physical_total')
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
    quietly timer off $KSS_BC_STAGE_SELECTION_TIMER
    quietly timer list $KSS_BC_STAGE_SELECTION_TIMER
    local sample_selection_seconds =                          ///
        r(t$KSS_BC_STAGE_SELECTION_TIMER)
    local selected_algorithm `algorithm'
    if "`selected_algorithm'" == "auto" {
        if `parameters' <= `exact_limit' local selected_algorithm exact
        else local selected_algorithm jla
    }

    // Construct the canonical semantic order before the compressed design.
    // The same rank is consumed by the generic JLA route, so this does not
    // make compressed eligibility part of the probe contract.
    if "`selected_algorithm'" == "jla" {
        tempvar semantic_target semantic_rank
        tempvar semantic_worker_min semantic_worker_max
        tempvar semantic_firm_min semantic_firm_max
        tempvar semantic_ambiguous
        quietly generate double `semantic_target' =               ///
            `target'/`frequency' if `touse'
        local semantic_key `depvar' `semantic_target'
        if "`probeorder'" != "" local semantic_key                ///
            `semantic_key' `probeorder'
        sort `semantic_key'
        quietly generate byte `semantic_ambiguous' = 0
        foreach control of local controlvars {
            tempvar semantic_control_min semantic_control_max
            quietly by `semantic_key': egen double                ///
                `semantic_control_min' = min(`control') if `touse'
            quietly by `semantic_key': egen double                ///
                `semantic_control_max' = max(`control') if `touse'
            quietly replace `semantic_ambiguous' = 1 if `touse' & ///
                `semantic_control_min' != `semantic_control_max'
        }
        quietly by `semantic_key': egen long `semantic_worker_min' = ///
            min(`id_worker') if `touse'
        quietly by `semantic_key': egen long `semantic_worker_max' = ///
            max(`id_worker') if `touse'
        quietly by `semantic_key': egen long `semantic_firm_min' = ///
            min(`id_firm') if `touse'
        quietly by `semantic_key': egen long `semantic_firm_max' = ///
            max(`id_firm') if `touse'
        quietly replace `semantic_ambiguous' = 1 if `touse' &     ///
            (`semantic_worker_min' != `semantic_worker_max' |    ///
             `semantic_firm_min' != `semantic_firm_max')
        if "`deletion'" == "match" {
            tempvar semantic_delete_min semantic_delete_max
            quietly by `semantic_key': egen long                  ///
                `semantic_delete_min' = min(`deletion_id') if `touse'
            quietly by `semantic_key': egen long                  ///
                `semantic_delete_max' = max(`deletion_id') if `touse'
            quietly replace `semantic_ambiguous' = 1 if `touse' & ///
                `semantic_delete_min' != `semantic_delete_max'
        }
        quietly count if `touse' & `semantic_ambiguous'
        if r(N) {
            quietly _kss_bc_post_failure "AMBIGUOUS_PROBE_ORDER"
            di as error "fixed-seed JLA cannot canonically order nonexchangeable rows with identical per-copy semantics; use algorithm(exact) or distinguish the rows"
            exit 498
        }
        quietly egen double `semantic_rank' =                     ///
            group(`semantic_key') if `touse'
        if "`deletion'" == "match" {
            sort `semantic_key' `id_worker' `id_firm' `deletion_id'
        }
        else sort `semantic_key' `id_worker' `id_firm'
    }

    local engine_selected generic
    local fastpath_eligible = 0
    local fastpath_status NOT_APPLICABLE
    local fastpath_message "selected algorithm does not use randomized probes"
    local coefficient_cells = `N_retained'
    local diagnostic_coefficient_cells = .
    local diagnostic_deletion_units = .
    local target_strata = `N_retained'
    local leverage_rng_calls_per_probe = 1
    local target_rng_calls_per_probe = 1
    local leverage_batch = `batch'
    local target_batch = `batch'
    local resource_peak_phase NOT_APPLICABLE
    local resource_status NOT_APPLICABLE
    local resource_message
    local compression_seconds = 0
    local life_mem_before_bytes = .
    tempname scale_prepare_diagnostics resource_components resource_forecasts
    tempname resource_scaling

    if "`selected_algorithm'" == "jla" {
        // Measure the row-resident caller state before constructing any
        // cached compressed arrays.  The resource model reconciles this
        // stage separately from transition, numerical, and restoration use.
        capture program list _kss_bc_lifecycle_memory
        if _rc {
            capture findfile kss_bc_lifecycle.ado
            if _rc {
                quietly _kss_bc_post_failure "LIFECYCLE_RUNTIME_NOT_FOUND"
                di as error "kss_bc_lifecycle.ado was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
        }
        quietly _kss_bc_lifecycle_memory, stage(selection)
        local life_mem_before_bytes = r(total_alloc_bytes)
        if missing(`life_mem_before_bytes') | `life_mem_before_bytes' <= 0 {
            quietly _kss_bc_post_failure "RAW_MEMORY_MEASUREMENT_FAILED"
            di as error "Stata did not provide a finite resident-memory measurement for resource admission"
            exit 498
        }

        capture mata: kssbc_scale__api_level()
        local scale_runtime_loaded = (_rc == 0)
        capture mata: assert(kssbc_scale__api_level() == 2 &       ///
            kssbc_scale__build_id() ==                            ///
            "kss-bc-scale-api2-cached-state")
        if _rc {
            if `scale_runtime_loaded' {
                quietly _kss_bc_post_failure "STALE_SCALE_RUNTIME"
                di as error "a different compressed-design runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile kss_bc_scale.mata
            if _rc {
                quietly _kss_bc_post_failure "SCALE_RUNTIME_NOT_FOUND"
                di as error "kss_bc_scale.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(kssbc_scale__api_level() == 2 &   ///
                kssbc_scale__build_id() ==                        ///
                "kss-bc-scale-api2-cached-state")
            if _rc {
                quietly _kss_bc_post_failure "INVALID_SCALE_RUNTIME"
                di as error "the installed compressed-design runtime is incompatible with this command"
                exit 498
            }
        }

        // Known-ineligible and explicitly generic calls never construct the
        // compressed state.  Eligible auto/explicit calls build the canonical
        // cell/unit/stratum representation exactly once and reuse it through
        // preserve/clear and all numerical work.
        if "`deletion'" != "match" {
            local fastpath_status FASTPATH_OBSERVATION_DELETION
            local fastpath_message "compressed leverage sums are not valid for observation deletion"
        }
        else if `control_count' > 0 {
            local fastpath_status FASTPATH_CONTROLS
            local fastpath_message "the experimental compressed engine does not support nuisance controls"
        }
        else if scalar(`retained_physical_total') >= 2^53 {
            local fastpath_status BINOMIAL_CONTRACT_UNSUPPORTED
            local fastpath_message "compressed binomial trials require total physical mass below 2^53"
        }
        else if `probes' > 16383 {
            local fastpath_status RNG_PROBE_RANGE_INVALID
            local fastpath_message "probe count exceeds the registered compressed RNG range"
        }
        else if "`engine_requested'" == "generic" {
            // Graph selection has already certified that every match lies in
            // one coefficient coordinate.  This is sufficient to retain the
            // exact semantic-atom RNG path without building compressed arrays.
            local fastpath_eligible = 1
            local fastpath_status FASTPATH_BYPASSED
            local fastpath_message "caller explicitly selected the generic engine"
        }
        else {
            quietly _kss_bc_lifecycle_timer_ids
            local compression_timer = r(transition_timer)
            local compression_aux_timer_1 = r(work_timer)
            local compression_aux_timer_2 = r(restore_timer)
            foreach compression_timer_id in `compression_timer'   ///
                `compression_aux_timer_1' `compression_aux_timer_2' {
                quietly timer clear `compression_timer_id'
            }
            capture quietly _kss_bc_lifecycle_phase,              ///
                phase(compression_transition)
            if _rc {
                quietly _kss_bc_lifecycle_release_timers,         ///
                    timers(`compression_timer'                    ///
                        `compression_aux_timer_1'                  ///
                        `compression_aux_timer_2')
                quietly _kss_bc_post_failure "PHASE_MARKER_FAILED"
                di as error "the compression-transition phase marker could not be written"
                exit 498
            }
            quietly timer on `compression_timer'
            capture noisily mata: kssbc_srt__prepare(             ///
                "`depvar'", "`id_worker'", "`id_firm'",       ///
                "`deletion_id'", "`frequency'", "`target'",  ///
                "`semantic_rank'", "`touse'", `rank_tolerance', ///
                "`scale_prepare_diagnostics'",                   ///
                "scale_prepare_status", "scale_prepare_message")
            local scale_prepare_rc = _rc
            quietly timer off `compression_timer'
            quietly timer list `compression_timer'
            local compression_seconds = r(t`compression_timer')
            quietly _kss_bc_lifecycle_release_timers,             ///
                timers(`compression_timer'                        ///
                    `compression_aux_timer_1'                      ///
                    `compression_aux_timer_2')
            if !`scale_prepare_rc' &                              ///
                "`scale_prepare_status'" == "PREPARED" {
                local diagnostic_coefficient_cells =              ///
                    `scale_prepare_diagnostics'[1,5]
                local diagnostic_deletion_units =                 ///
                    `scale_prepare_diagnostics'[1,6]
                local target_strata = `scale_prepare_diagnostics'[1,7]
                local leverage_rng_calls_per_probe =              ///
                    `scale_prepare_diagnostics'[1,14]
                local target_rng_calls_per_probe =                ///
                    `scale_prepare_diagnostics'[1,15]
                local coefficient_cells =                        ///
                    `diagnostic_coefficient_cells'
                local fastpath_eligible = 1
                local fastpath_status ELIGIBLE
                local fastpath_message "no-control match design has exact cell, deletion-unit, and target-stratum compression"
            }
            else {
                capture mata: kssbc_scale_runtime__reset()
                if `scale_prepare_rc' {
                    local fastpath_status SCALE_PREPARATION_FAILED
                    local fastpath_message "compressed-state construction stopped unexpectedly"
                }
                else {
                    local fastpath_status `scale_prepare_status'
                    local fastpath_message `"`scale_prepare_message'"'
                }
            }
        }

        if "`engine_requested'" == "compressed" & !`fastpath_eligible' {
            quietly _kss_bc_post_failure "`fastpath_status'"
            ereturn scalar N_retained = `N_retained'
            ereturn scalar N_physical = scalar(`retained_physical_total')
            ereturn scalar coefficient_cells = `coefficient_cells'
            ereturn scalar deletion_units = `diagnostic_deletion_units'
            ereturn scalar target_strata = `target_strata'
            ereturn local engine_requested "`engine_requested'"
            ereturn local engine_selected "WITHHELD"
            ereturn local fastpath_status "`fastpath_status'"
            ereturn local fastpath_message `"`fastpath_message'"'
            di as error `"`fastpath_message'"'
            exit 498
        }
        if "`engine_requested'" == "compressed" |                 ///
            ("`engine_requested'" == "auto" & `fastpath_eligible') {
            local engine_selected compressed
        }
        if "`engine_selected'" == "generic" {
            capture mata: kssbc_scale_runtime__reset()
            if _rc {
                quietly _kss_bc_post_failure "SCALE_STATE_RELEASE_FAILED"
                di as error "unused compressed command state could not be released"
                exit 498
            }
        }

        if "`engine_selected'" == "compressed" &                 ///
            "`batch_requested'" == "auto" {
            local leverage_batch = 8
            local target_batch = 8
            local scale_batch_budget = floor(`memory_gib'*1024^3*.25)
            foreach candidate in 16 32 64 {
                local leverage_candidate_bytes = 8*`candidate'*(   ///
                    6*`diagnostic_deletion_units'+                ///
                    4*`diagnostic_coefficient_cells'+3*`parameters')
                if `candidate' <= `probes' & `candidate' <= 32 &   ///
                    `leverage_candidate_bytes' <= `scale_batch_budget' {
                    local leverage_batch = `candidate'
                }
                local target_candidate_bytes = 8*`candidate'*(     ///
                    2*`target_strata'+                             ///
                    8*`diagnostic_coefficient_cells'+6*`parameters')
                if `candidate' <= `probes' & `candidate' <= 32 &   ///
                    `target_candidate_bytes' <= `scale_batch_budget' {
                    local target_batch = `candidate'
                }
            }
            local batch = `leverage_batch'
            local batch_routing_reason "separate largest cell/unit/stratum widths within the compressed 25% scratch reservation"
        }
        else {
            local leverage_batch = `batch'
            local target_batch = `batch'
        }
        if "`engine_selected'" == "generic" &                    ///
            "`batch_requested'" == "auto" {
            local generic_batch_budget = floor(`memory_gib'*1024^3*.35)
            local generic_physical_column =                       ///
                8*scalar(`retained_physical_total')
            local generic_column_bytes =                         ///
                8*(14*`N_retained'+12*`parameters')+              ///
                `generic_physical_column'
            local generic_processor_cap = cond(`active_processors'>=8,64,32)
            if `N_retained' >= 10000 {
                foreach candidate in 16 32 64 {
                    if `candidate' <= `generic_processor_cap' &   ///
                        `candidate' <= `probes' &                 ///
                        `candidate'*`generic_column_bytes' <=     ///
                        `generic_batch_budget' local batch = `candidate'
                }
            }
            local leverage_batch = `batch'
            local target_batch = `batch'
        }

        capture mata: kssbc_resource__api_level()
        local resource_runtime_loaded = (_rc == 0)
        capture mata: assert(kssbc_resource__api_level() == 5 &    ///
            kssbc_resource__build_id() ==                         ///
            "kss-bc-resource-api5-transition-highwater")
        if _rc {
            if `resource_runtime_loaded' {
                quietly _kss_bc_post_failure "STALE_RESOURCE_RUNTIME"
                di as error "a different resource-admission runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile kss_bc_resource.mata
            if _rc {
                quietly _kss_bc_post_failure "RESOURCE_RUNTIME_NOT_FOUND"
                di as error "kss_bc_resource.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(kssbc_resource__api_level() == 5 & ///
                kssbc_resource__build_id() ==                     ///
                "kss-bc-resource-api5-transition-highwater")
            if _rc {
                quietly _kss_bc_post_failure "INVALID_RESOURCE_RUNTIME"
                di as error "the installed resource-admission runtime is incompatible with this command"
                exit 498
            }
        }

        local resource_cells = `coefficient_cells'
        local resource_units = `diagnostic_deletion_units'
        if missing(`resource_units') local resource_units = `N_retained'
        local resource_strata = `target_strata'
        if missing(`resource_strata') local resource_strata = `N_retained'
        capture noisily mata: kssbc_resource__stata_model(         ///
            `N_retained', `retained_physical',                    ///
            `resource_cells', `resource_units', `resource_strata', ///
            `worker_levels', `firm_levels', `parameters',         ///
            `probes', `leverage_batch', `target_batch',           ///
            `leverage_rng_calls_per_probe',                       ///
            `target_rng_calls_per_probe',                         ///
            kssbc_resource__rng_call_upper(),                     ///
            `life_mem_before_bytes',                              ///
            `memory_gib'*1024^3, `wallseconds',                   ///
            "`resource_components'", "`resource_forecasts'",   ///
            "`resource_scaling'", "resource_model_status",     ///
            "resource_model_message", "compressed_resource_status", ///
            "compressed_resource_message", "generic_resource_status", ///
            "generic_resource_message", "compressed_peak_phase", ///
            "generic_peak_phase")
        if _rc | "`resource_model_status'" != "MODELED" {
            quietly _kss_bc_post_failure "RESOURCE_MODEL_FAILED"
            di as error "resource admission could not be constructed before probes"
            exit 498
        }
        local resource_row = cond("`engine_selected'" == "compressed",1,2)
        if `resource_row' == 1 {
            local resource_status `compressed_resource_status'
            local resource_message `"`compressed_resource_message'"'
            local resource_peak_phase `compressed_peak_phase'
        }
        else {
            local resource_status `generic_resource_status'
            local resource_message `"`generic_resource_message'"'
            local resource_peak_phase `generic_peak_phase'
        }
        local resource_non_solver_bytes =                         ///
            `resource_components'[`resource_row',2]+              ///
            `resource_components'[`resource_row',3]+              ///
            `resource_components'[`resource_row',4]+              ///
            `resource_components'[`resource_row',6]+              ///
            `resource_components'[`resource_row',8]+              ///
            `resource_components'[`resource_row',9]+              ///
            `resource_components'[`resource_row',11]
        if `resource_row' == 2 local resource_non_solver_bytes =  ///
            `resource_non_solver_bytes'+                          ///
            `resource_components'[`resource_row',1]
        local resource_selection_peak =                          ///
            `resource_forecasts'[`resource_row',1]
        local resource_transition_peak =                         ///
            `resource_forecasts'[`resource_row',2]
        local resource_restoration_peak =                        ///
            `resource_forecasts'[`resource_row',4]
        local resource_wall_forecast =                           ///
            `resource_forecasts'[`resource_row',8]
        local resource_hard_memory =                             ///
            `resource_forecasts'[`resource_row',11]
        local resource_hard_wall =                               ///
            `resource_forecasts'[`resource_row',12]
        if `resource_forecasts'[`resource_row',15] != 1 {
            quietly _kss_bc_post_failure "`resource_status'"
            ereturn scalar N_retained = `N_retained'
            ereturn scalar N_physical = scalar(`retained_physical_total')
            ereturn scalar coefficient_cells = `coefficient_cells'
            ereturn scalar deletion_units = `resource_units'
            ereturn scalar target_strata = `resource_strata'
            ereturn scalar resource_peak_bytes =                 ///
                `resource_forecasts'[`resource_row',5]
            ereturn scalar resource_mem_admit_bytes =             ///
                `resource_forecasts'[`resource_row',7]
            ereturn scalar resource_wall_upper_seconds =          ///
                `resource_forecasts'[`resource_row',8]
            ereturn scalar resource_wall_admit_seconds =          ///
                `resource_forecasts'[`resource_row',10]
            ereturn scalar resource_hard_mem_bytes =              ///
                `resource_forecasts'[`resource_row',11]
            ereturn scalar resource_hard_wall_seconds =           ///
                `resource_forecasts'[`resource_row',12]
            ereturn local engine_requested "`engine_requested'"
            ereturn local engine_selected "`engine_selected'"
            ereturn local resource_status "`resource_status'"
            ereturn local resource_message `"`resource_message'"'
            ereturn local resource_peak_phase "`resource_peak_phase'"
            di as error `"`resource_message'"'
            exit 498
        }
    }
    else if "`engine_requested'" == "compressed" {
        quietly _kss_bc_post_failure "FASTPATH_REQUIRES_JLA"
        ereturn local engine_requested "`engine_requested'"
        ereturn local engine_selected "WITHHELD"
        ereturn local fastpath_status "FASTPATH_REQUIRES_JLA"
        di as error "engine(compressed) requires algorithm(jla)"
        exit 498
    }

    // Forecast the largest estimator scratch family conservatively as
    // fourteen retained-row vectors, twelve coefficient vectors, and one
    // literal-physical-mass vector per simultaneous probe.  The generic
    // route therefore charges retained frequency mass for every deletion
    // mode, not only for observation deletion.  Automatic widths use at most
    // 35% of the caller's
    // declared envelope and never exceed the probe count.  Real CZ24/CZ25
    // profiling shows that four processors stop gaining beyond 32 columns;
    // the corresponding evidence-backed cap is 64 with eight or more.  Width
    // 128 remains available explicitly but is not selected automatically.
    // This policy is deterministic and is applied before solver routing or
    // random probes.
    local batch_memory_budget_bytes = floor(`memory_gib'*1024^3*.35)
    local batch_physical_column_bytes =                            ///
        8*scalar(`retained_physical_total')
    local batch_column_forecast_bytes = ///
        8*(14*`N_retained'+12*`parameters')+ ///
        `batch_physical_column_bytes'
    if "`batch_requested'" == "auto" & "`selected_algorithm'" == "jla" & ///
        "`engine_selected'" == "generic" {
        local batch_processor_cap = cond(`active_processors'>=8,64,32)
        if `N_retained' >= 10000 {
            foreach candidate in 16 32 64 {
                if `candidate' <= `batch_processor_cap' & ///
                    `candidate' <= `probes' & ///
                    `candidate'*`batch_column_forecast_bytes' <= ///
                    `batch_memory_budget_bytes' local batch = `candidate'
            }
            local batch_routing_reason ///
                "largest evidence-backed width within probe, processor, and 35% memory gates"
        }
    }
    if "`selected_algorithm'" == "jla" {
        if "`engine_selected'" == "compressed" {
            local batch_scratch_forecast_bytes =                  ///
                `resource_components'[1,6]
            local batch_memory_budget_bytes =                     ///
                floor(`memory_gib'*1024^3*.25)
            local batch_column_forecast_bytes =                   ///
                `batch_scratch_forecast_bytes'/max(1,`batch')
        }
        else local batch_scratch_forecast_bytes =                 ///
            `batch'*`batch_column_forecast_bytes'
    }
    else {
        local batch_scratch_forecast_bytes = 0
        local batch_routing_reason "exact algorithm does not consume probe batches"
    }

    // The memory envelope is a fail-closed allocation contract, not merely a
    // routing hint.  Reject both the automatic floor and explicit widths
    // before CMG/diagonal routing is entered and before Mata initializes the
    // production random stream.
    if "`selected_algorithm'" == "jla" &                         ///
        "`engine_selected'" == "generic" &                       ///
        `batch_scratch_forecast_bytes' > `batch_memory_budget_bytes' {
        if "`batch_requested'" == "auto" {
            local batch_routing_reason ///
                "automatic batch floor exceeds the 35% scratch-memory budget"
        }
        else {
            local batch_routing_reason ///
                "caller-supplied batch exceeds the 35% scratch-memory budget"
        }
        quietly _kss_bc_post_failure "BATCH_MEMORY_LIMIT"
        ereturn scalar N_retained = `N_retained'
        ereturn scalar N_physical = scalar(`retained_physical_total')
        ereturn scalar worker_levels = `worker_levels'
        ereturn scalar firm_levels = `firm_levels'
        ereturn scalar parameters = `parameters'
        ereturn scalar batch = `batch'
        ereturn scalar batch_memory_budget_bytes = ///
            `batch_memory_budget_bytes'
        ereturn scalar batch_column_forecast_bytes = ///
            `batch_column_forecast_bytes'
        ereturn scalar batch_physical_column_bytes = ///
            `batch_physical_column_bytes'
        ereturn scalar batch_scratch_forecast_bytes = ///
            `batch_scratch_forecast_bytes'
        ereturn scalar memory_forecast_bytes = ///
            `batch_scratch_forecast_bytes'
        ereturn scalar memory_gib = `memory_gib'
        ereturn scalar active_processors = `active_processors'
        ereturn scalar probes = `probes'
        ereturn scalar seed = `seed'
        ereturn scalar tolerance = `tolerance'
        ereturn local algorithm "`selected_algorithm'"
        ereturn local deletion "`deletion'"
        ereturn local batch_requested "`batch_requested'"
        ereturn local batch_routing_reason `"`batch_routing_reason'"'
        di as error "projected batch scratch exceeds the memory_gib() budget"
        exit 498
    }

    if "`selected_algorithm'" == "jla" &                         ///
        "`engine_selected'" == "generic" {
        if scalar(`retained_physical_total') > `physical_limit' {
            quietly _kss_bc_post_failure "PHYSICAL_COPY_LIMIT"
            di as error "JLA physical copies exceed physical_limit()"
            exit 498
        }
    }
    if "`selected_algorithm'" != "jla" & `control_count' > 0 {
        // A physical copy is ordered by outcome and per-copy target mass.
        // Raw IDs, stored-row representation, and control coordinates cannot
        // index either the sign stream or the row-anchor canonicalizer: each
        // can change under a contract-preserving relabeling, split, or
        // invertible control reparameterization.  An explicit probeorder()
        // key is an opt-in physical-observation semantic key for discrete
        // outcomes.  It must be complete and globally unique.  Existing
        // calls never use it implicitly.  Rows still tied on the resulting
        // semantic key are exchangeable only when their controls, model
        // coordinate, and match block agree.  Otherwise the backend withholds.
        tempvar semantic_target semantic_rank
        tempvar semantic_worker_min semantic_worker_max
        tempvar semantic_firm_min semantic_firm_max
        tempvar semantic_ambiguous
        quietly generate double `semantic_target' = ///
            `target'/`frequency' if `touse'
        local semantic_key `depvar' `semantic_target'
        if "`probeorder'" != "" local semantic_key ///
            `semantic_key' `probeorder'
        sort `semantic_key'
        quietly generate byte `semantic_ambiguous' = 0
        foreach control of local controlvars {
            tempvar semantic_control_min semantic_control_max
            quietly by `semantic_key': egen double `semantic_control_min' = ///
                min(`control') if `touse'
            quietly by `semantic_key': egen double `semantic_control_max' = ///
                max(`control') if `touse'
            quietly replace `semantic_ambiguous' = 1 if `touse' & ///
                `semantic_control_min' != `semantic_control_max'
        }
        quietly by `semantic_key': egen long `semantic_worker_min' = ///
            min(`id_worker') if `touse'
        quietly by `semantic_key': egen long `semantic_worker_max' = ///
            max(`id_worker') if `touse'
        quietly by `semantic_key': egen long `semantic_firm_min' = ///
            min(`id_firm') if `touse'
        quietly by `semantic_key': egen long `semantic_firm_max' = ///
            max(`id_firm') if `touse'
        quietly replace `semantic_ambiguous' = 1 if `touse' & ///
            (`semantic_worker_min' != `semantic_worker_max' | ///
             `semantic_firm_min' != `semantic_firm_max')
        if "`deletion'" == "match" {
            tempvar semantic_delete_min semantic_delete_max
            quietly by `semantic_key': egen long `semantic_delete_min' = ///
                min(`deletion_id') if `touse'
            quietly by `semantic_key': egen long `semantic_delete_max' = ///
                max(`deletion_id') if `touse'
            quietly replace `semantic_ambiguous' = 1 if `touse' & ///
                `semantic_delete_min' != `semantic_delete_max'
        }
        quietly count if `touse' & `semantic_ambiguous'
        if r(N) {
            if "`selected_algorithm'" == "jla" {
                quietly _kss_bc_post_failure "AMBIGUOUS_PROBE_ORDER"
                di as error "fixed-seed JLA cannot canonically order nonexchangeable rows with identical per-copy semantics; use algorithm(exact) or distinguish the rows"
            }
            else {
                quietly _kss_bc_post_failure "AMBIGUOUS_CONTROL_BASIS"
                di as error "controlled exact calculation cannot canonically order nonexchangeable rows with identical per-copy semantics; distinguish the rows"
            }
            exit 498
        }
        // This rank depends only on the canonical semantic tuple.  It is
        // invariant to raw row order and arbitrary dense worker/firm/deletion
        // encodings.  The compressed runtime reduces it to one unique key per
        // deletion unit and exact target stratum.
        quietly egen double `semantic_rank' = group(`semantic_key') if `touse'
        if "`deletion'" == "match" {
            sort `semantic_key' `id_worker' `id_firm' `deletion_id'
        }
        else sort `semantic_key' `id_worker' `id_firm'
    }
    else if "`selected_algorithm'" != "jla" & "`deletion'" == "match" {
        sort `id_worker' `id_firm' `deletion_id' `depvar' ///
            `frequency' `target'
    }
    else if "`selected_algorithm'" != "jla" {
        sort `id_worker' `id_firm' `depvar' `frequency' `target'
    }
    tempname raw_results diagnostics solver_rhs_diagnostics route_diagnostics
    tempname pilot_diagnostics scale_receipt
    tempname plugin correction
    tempname corrected kss_return mcse
    local mata_status
    local mata_message
    local selected_preconditioner NOT_APPLICABLE
    local routing_reason EXACT_ALGORITHM
    local fallback_status NOT_APPLICABLE
    local fallback_message
    local pilot_status
    local pilot_failure_reason
    local rng_contract NOT_APPLICABLE
    local rng_implementation NOT_APPLICABLE
    local rng_call_shape NOT_APPLICABLE
    local rng_runtime `"`c(stata_version)'"'
    local rng_leverage_domain NOT_APPLICABLE
    local rng_target_domain NOT_APPLICABLE
    local rng_leverage_probe_first = .
    local rng_leverage_probe_last = .
    local rng_target_probe_first = .
    local rng_target_probe_last = .
    local life_transition_seconds = 0
    local life_work_seconds = 0
    local life_restore_seconds = 0
    local life_sample_restored = 1
    local life_preserve_forced_disk = 0
    local life_mem_cleared_bytes = .
    local life_mem_work_bytes = .
    local life_mem_restored_bytes = .
    local life_method RAW_RESIDENT
    local mata_call_rc = 0
    local resource_receipt_rc = 0
    local resource_gate_applied NOT_APPLIED
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
        local rng_call_shape                            ///
            vector-parameter-or-scalar-chunk-canonical-atoms-v2
        capture mata: kssbc_rng__api_level()
        local rng_runtime_loaded = (_rc == 0)
        capture mata: assert(kssbc_rng__api_level() == 2 &         ///
            kssbc_rng__build_id() ==                              ///
            "kss-bc-rng-k1-mt64s-complete-guard-v2")
        if _rc {
            if `rng_runtime_loaded' {
                quietly _kss_bc_post_failure "STALE_RNG_RUNTIME"
                di as error "a different KSS RNG runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile kss_bc_rng.mata
            if _rc {
                quietly _kss_bc_post_failure "RNG_RUNTIME_NOT_FOUND"
                di as error "kss_bc_rng.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(kssbc_rng__api_level() == 2 &     ///
                kssbc_rng__build_id() ==                          ///
                "kss-bc-rng-k1-mt64s-complete-guard-v2")
            if _rc {
                quietly _kss_bc_post_failure "INVALID_RNG_RUNTIME"
                di as error "the installed RNG runtime is incompatible with this command"
                exit 498
            }
        }
        capture mata: assert(kssbc_rng__production_contract() != "")
        if _rc {
            quietly _kss_bc_post_failure "RNG_RUNTIME_UNREGISTERED"
            ereturn local rng_runtime `"`c(stata_version)'"'
            di as error "the current Stata runtime has no registered KSS probe contract"
            exit 498
        }
        capture mata: kssbc_cmg__api_level()
        local cmg_runtime_loaded = (_rc == 0)
        local expected_cmg_design ///
            "clean-room-cmg-inspired-degree3-hybrid-v5-robust-hierarchy"
        capture mata: assert(kssbc_cmg__api_level() == 5 &        ///
            kssbc_cmg__design_label() == "`expected_cmg_design'")
        if _rc {
            if `cmg_runtime_loaded' {
                quietly _kss_bc_post_failure "STALE_CMG_RUNTIME"
                di as error "a different CMG runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile kss_bc_cmg.mata
            if _rc {
                quietly _kss_bc_post_failure "CMG_RUNTIME_NOT_FOUND"
                di as error "kss_bc_cmg.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(kssbc_cmg__api_level() == 5 &    ///
                kssbc_cmg__design_label() ==                     ///
                "`expected_cmg_design'")
            if _rc {
                quietly _kss_bc_post_failure "INVALID_CMG_RUNTIME"
                di as error "the installed CMG runtime is incompatible with this command"
                exit 498
            }
        }
        capture mata: kssbc_solver__api_level()
        local solver_runtime_loaded = (_rc == 0)
        capture mata: assert(kssbc_solver__api_level() == 22 &     ///
            kssbc_solver__pilot_api() == 1 &                      ///
            kssbc_solver__build_id() ==                           ///
            "kss-bc-solver-api22-runtime-residency-receipt")
        if _rc {
            if `solver_runtime_loaded' {
                quietly _kss_bc_post_failure "STALE_SOLVER_RUNTIME"
                di as error "a different KSS solver adapter is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile kss_bc_solver.mata
            if _rc {
                quietly _kss_bc_post_failure "SOLVER_RUNTIME_NOT_FOUND"
                di as error "kss_bc_solver.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(kssbc_solver__api_level() == 22 & ///
                kssbc_solver__pilot_api() == 1 &                  ///
                kssbc_solver__build_id() ==                       ///
                "kss-bc-solver-api22-runtime-residency-receipt")
            if _rc {
                quietly _kss_bc_post_failure "INVALID_SOLVER_RUNTIME"
                di as error "the installed KSS solver adapter is incompatible with this command"
                exit 498
            }
        }
        if "`engine_selected'" == "compressed" {
            capture mata: kssbc_scale_engine__api_level()
            local scale_engine_loaded = (_rc == 0)
            capture mata: assert(                                 ///
                kssbc_scale_engine__api_level() == 1 &            ///
                kssbc_scale_engine__build_id() ==                 ///
                "kss-bc-scale-engine-cell-match-rngcursor-api1")
            if _rc {
                if `scale_engine_loaded' {
                    quietly _kss_bc_post_failure "STALE_SCALE_ENGINE"
                    di as error "a different compressed estimator runtime is already loaded; restart Stata or run discard before retrying"
                    exit 498
                }
                capture findfile kss_bc_scale_engine.mata
                if _rc {
                    quietly _kss_bc_post_failure "SCALE_ENGINE_NOT_FOUND"
                    di as error "kss_bc_scale_engine.mata was not found on the Stata adopath"
                    exit 601
                }
                quietly do `"`r(fn)'"'
                capture mata: assert(                             ///
                    kssbc_scale_engine__api_level() == 1 &        ///
                    kssbc_scale_engine__build_id() ==             ///
                    "kss-bc-scale-engine-cell-match-rngcursor-api1")
                if _rc {
                    quietly _kss_bc_post_failure "INVALID_SCALE_ENGINE"
                    di as error "the compressed estimator runtime is incompatible with this command"
                    exit 498
                }
            }

            capture mata: kssbc_scale_runtime__api_level()
            local scale_bridge_loaded = (_rc == 0)
            capture mata: assert(                                 ///
                kssbc_scale_runtime__api_level() == 1 &           ///
                kssbc_scale_runtime__build_id() ==                ///
                "kss-bc-scale-runtime-preserve-api1")
            if _rc {
                if `scale_bridge_loaded' {
                    quietly _kss_bc_post_failure "STALE_SCALE_BRIDGE"
                    di as error "a different compressed lifecycle bridge is already loaded; restart Stata or run discard before retrying"
                    exit 498
                }
                capture findfile kss_bc_scale_runtime.mata
                if _rc {
                    quietly _kss_bc_post_failure "SCALE_BRIDGE_NOT_FOUND"
                    di as error "kss_bc_scale_runtime.mata was not found on the Stata adopath"
                    exit 601
                }
                quietly do `"`r(fn)'"'
                capture mata: assert(                             ///
                    kssbc_scale_runtime__api_level() == 1 &       ///
                    kssbc_scale_runtime__build_id() ==            ///
                    "kss-bc-scale-runtime-preserve-api1")
                if _rc {
                    quietly _kss_bc_post_failure "INVALID_SCALE_BRIDGE"
                    di as error "the compressed lifecycle bridge is incompatible with this command"
                    exit 498
                }
            }

            quietly _kss_bc_lifecycle_timer_ids
            local transition_timer = r(transition_timer)
            local work_timer = r(work_timer)
            local restore_timer = r(restore_timer)
            foreach lifecycle_timer in `transition_timer' `work_timer' ///
                `restore_timer' {
                quietly timer clear `lifecycle_timer'
            }
            quietly count if `touse'
            local lifecycle_sample_N = r(N)
            quietly _datasignature `touse', nonames
            local lifecycle_sample_signature `"`r(datasignature)'"'

            capture mata: assert(                                 ///
                kssbc_scale_runtime__status() == "PREPARED")
            if _rc {
                capture mata: kssbc_scale_runtime__reset()
                quietly _kss_bc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _kss_bc_post_failure "FASTPATH_STATE_UNAVAILABLE"
                di as error "the canonical compressed command state is unavailable"
                exit 498
            }

            local caller_max_preservemem = c(max_preservemem)
            local old_preservemem_text : display %21.0f ///
                `caller_max_preservemem'
            local old_preservemem_text = strtrim("`old_preservemem_text'")
            local transition_rc = 0
            local preserve_active = 0
            quietly timer on `transition_timer'
            capture quietly set max_preservemem 0
            if _rc local transition_rc = _rc
            if !`transition_rc' {
                capture quietly preserve
                if _rc local transition_rc = _rc
                else local preserve_active = 1
            }
            capture quietly set max_preservemem `old_preservemem_text'
            if _rc & !`transition_rc' local transition_rc = _rc
            if !`transition_rc' {
                capture quietly clear
                if _rc local transition_rc = _rc
            }
            quietly timer off `transition_timer'
            quietly timer list `transition_timer'
            local life_transition_seconds = r(t`transition_timer')
            local life_preserve_forced_disk = 1
            local life_method PRESERVE_DISK
            if `transition_rc' {
                capture mata: kssbc_scale_runtime__reset()
                if `preserve_active' capture quietly restore
                quietly _kss_bc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _kss_bc_post_failure "DATA_LIFECYCLE_FAILED"
                di as error "disk-backed Stata preserve/clear transition failed"
                exit `transition_rc'
            }

            quietly _kss_bc_lifecycle_memory, stage(cleared)
            local life_mem_cleared_bytes = r(total_alloc_bytes)
            capture quietly _kss_bc_lifecycle_phase, phase(numerical)
            local numerical_marker_rc = _rc
            if `numerical_marker_rc' {
                capture mata: kssbc_scale_runtime__reset()
                capture quietly clear
                capture quietly restore
                quietly _kss_bc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _kss_bc_post_failure "PHASE_MARKER_FAILED"
                di as error "the numerical phase marker could not be written"
                exit 498
            }
            capture mata: assert(kssbc_solver__resource_config(  ///
                "compressed", `resource_non_solver_bytes',       ///
                `resource_selection_peak',                       ///
                `resource_transition_peak',                      ///
                `resource_restoration_peak',                     ///
                `resource_wall_forecast',                        ///
                `resource_hard_memory', `resource_hard_wall') == 0)
            local resource_gate_config_rc = _rc
            if `resource_gate_config_rc' {
                capture mata: kssbc_solver__resource_clear()
                capture mata: kssbc_scale_runtime__reset()
                capture quietly clear
                capture quietly restore
                quietly _kss_bc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _kss_bc_post_failure                     ///
                    "RESOURCE_GATE_CONFIGURATION_FAILED"
                di as error "the final routed resource gate could not be configured"
                exit 498
            }
            quietly timer on `work_timer'
            capture noisily mata: kssbc_scale_runtime__stata_run(   ///
                `probes', `leverage_batch', `target_batch',         ///
                `seed', `tolerance', `maxiter', `rank_tolerance',   ///
                `block_tolerance', `blocksize_limit',               ///
                "`preconditioner'", `memory_gib'*1024^3,           ///
                "`raw_results'", "`diagnostics'",                ///
                "`solver_rhs_diagnostics'",                       ///
                "`route_diagnostics'", "`pilot_diagnostics'",   ///
                "`scale_receipt'", "mata_status",               ///
                "mata_message", "selected_preconditioner",      ///
                "routing_reason", "fallback_status",            ///
                "fallback_message", "pilot_status",             ///
                "pilot_failure_reason", "rng_contract",         ///
                "rng_implementation", "rng_runtime")
            local mata_call_rc = _rc
            quietly timer off `work_timer'
            quietly timer list `work_timer'
            local life_work_seconds = r(t`work_timer')
            capture mata: kssbc_solver__stata_res_rcpt(          ///
                "`resource_components'",                        ///
                "`resource_forecasts'", `resource_row',          ///
                "resource_route_status",                        ///
                "resource_route_message",                       ///
                "resource_route_phase",                         ///
                "resource_gate_applied")
            local resource_receipt_rc = _rc
            if `resource_receipt_rc' {
                capture mata: kssbc_solver__resource_clear()
            }
            else if "`resource_gate_applied'" == "APPLIED" {
                local resource_status `resource_route_status'
                local resource_message `"`resource_route_message'"'
                local resource_peak_phase `resource_route_phase'
            }
            else if !`mata_call_rc' & "`mata_status'" == "CONVERGED" {
                local resource_receipt_rc = 498
            }
            quietly _kss_bc_lifecycle_memory, stage(after_work)
            local life_mem_work_bytes = r(total_alloc_bytes)

            quietly timer on `restore_timer'
            capture mata: kssbc_scale_runtime__reset()
            local scale_release_rc = _rc
            capture quietly clear
            capture quietly _kss_bc_lifecycle_phase, phase(restoration)
            local restoration_marker_rc = _rc
            capture quietly restore
            local restore_rc = _rc
            quietly timer off `restore_timer'
            quietly timer list `restore_timer'
            local life_restore_seconds = r(t`restore_timer')
            quietly _kss_bc_lifecycle_memory, stage(restored)
            local life_mem_restored_bytes = r(total_alloc_bytes)
            local life_sample_restored = 1
            capture confirm numeric variable `touse'
            if _rc local life_sample_restored = 0
            if `life_sample_restored' {
                quietly count if `touse'
                if r(N) != `lifecycle_sample_N' ///
                    local life_sample_restored = 0
            }
            if `life_sample_restored' {
                quietly _datasignature `touse', nonames
                if `"`r(datasignature)'"' !=                       ///
                    `"`lifecycle_sample_signature'"'               ///
                    local life_sample_restored = 0
            }
            quietly _kss_bc_lifecycle_release_timers,             ///
                timers(`transition_timer' `work_timer' `restore_timer')
            if `scale_release_rc' | `restore_rc' |                 ///
                !`life_sample_restored' {
                quietly _kss_bc_post_failure "DATA_RESTORATION_FAILED"
                di as error "caller data or estimation-sample restoration failed"
                exit 498
            }
            if `restoration_marker_rc' {
                quietly _kss_bc_post_failure "PHASE_MARKER_FAILED"
                di as error "the restoration phase marker could not be written"
                exit 498
            }
            if `resource_receipt_rc' {
                quietly _kss_bc_post_failure "RESOURCE_RECEIPT_FAILED"
                di as error "the final routed resource receipt could not be recorded"
                exit 498
            }
        }
        else {
            capture mata: assert(kssbc_solver__resource_config(  ///
                "generic", `resource_non_solver_bytes',          ///
                `resource_selection_peak',                       ///
                `resource_transition_peak',                      ///
                `resource_restoration_peak',                     ///
                `resource_wall_forecast',                        ///
                `resource_hard_memory', `resource_hard_wall') == 0)
            if _rc {
                capture mata: kssbc_solver__resource_clear()
                quietly _kss_bc_post_failure                     ///
                    "RESOURCE_GATE_CONFIGURATION_FAILED"
                di as error "the final routed resource gate could not be configured"
                exit 498
            }
            capture noisily mata: kssbc__stata_jla_routed(         ///
                "`depvar'", "`id_worker'", "`id_firm'",         ///
                `"`controlvars'"', "`frequency'", "`target'",   ///
                "`deletion_id'", "`touse'", "`semantic_rank'", ///
                `fastpath_eligible', "`deletion'",                ///
                "`nuisance'", `probes', `batch', `seed',          ///
                `tolerance', `maxiter', `rank_tolerance',          ///
                `block_tolerance', `blocksize_limit',              ///
                "`preconditioner'", `memory_gib'*1024^3,         ///
                "`raw_results'", "mata_status",                 ///
                "mata_message", "`diagnostics'",                ///
                "`solver_rhs_diagnostics'",                      ///
                "`route_diagnostics'",                           ///
                "selected_preconditioner", "routing_reason",   ///
                "fallback_status", "fallback_message",         ///
                "`pilot_diagnostics'", "pilot_status",          ///
                "pilot_failure_reason")
            local mata_call_rc = _rc
            capture mata: kssbc_solver__stata_res_rcpt(          ///
                "`resource_components'",                        ///
                "`resource_forecasts'", `resource_row',          ///
                "resource_route_status",                        ///
                "resource_route_message",                       ///
                "resource_route_phase",                         ///
                "resource_gate_applied")
            local resource_receipt_rc = _rc
            if `resource_receipt_rc' {
                capture mata: kssbc_solver__resource_clear()
                quietly _kss_bc_post_failure "RESOURCE_RECEIPT_FAILED"
                di as error "the final routed resource receipt could not be recorded"
                exit 498
            }
            if "`resource_gate_applied'" == "APPLIED" {
                local resource_status `resource_route_status'
                local resource_message `"`resource_route_message'"'
                local resource_peak_phase `resource_route_phase'
            }
            else if !`mata_call_rc' & "`mata_status'" == "CONVERGED" {
                quietly _kss_bc_post_failure "RESOURCE_RECEIPT_FAILED"
                di as error "the solver returned without applying the final routed resource gate"
                exit 498
            }
            mata: st_local("rng_contract",                       ///
                kssbc_rng__production_contract())
            if `fastpath_eligible' local rng_implementation       ///
                per_domain_stream_semantic_atoms
            else local rng_implementation                         ///
                per_domain_stream_physical_copies
        }
        local rng_leverage_domain leverage
        local rng_target_domain target
        local rng_leverage_probe_first = 1
        local rng_leverage_probe_last = `probes'
        local rng_target_probe_first = 1
        local rng_target_probe_last = `probes'
        if "`fallback_status'" == "" local fallback_status NOT_NEEDED
    }
    if "`selected_algorithm'" == "jla" & `mata_call_rc' {
        local mata_rc = `mata_call_rc'
        quietly _kss_bc_post_failure "MATA_RUNTIME_FAILED"
        di as error "the Mata backend stopped unexpectedly"
        exit `mata_rc'
    }
    if "`mata_status'" != "CONVERGED" {
        local failure_status `mata_status'
        local failure_message `"`mata_message'"'
        quietly _kss_bc_post_failure "`failure_status'"
        if "`selected_algorithm'" == "jla" {
            ereturn local engine_requested "`engine_requested'"
            ereturn local engine_selected "`engine_selected'"
            ereturn local fastpath_status "`fastpath_status'"
            ereturn local fastpath_message `"`fastpath_message'"'
            ereturn local resource_status "`resource_status'"
            ereturn local resource_message `"`resource_message'"'
            ereturn local resource_peak_phase "`resource_peak_phase'"
            ereturn local rng_contract "`rng_contract'"
            ereturn local rng_implementation "`rng_implementation'"
            ereturn local rng_call_shape "`rng_call_shape'"
            ereturn local rng_runtime "`rng_runtime'"
            ereturn local rng_leverage_domain "`rng_leverage_domain'"
            ereturn local rng_target_domain "`rng_target_domain'"
            ereturn local life_method "`life_method'"
            ereturn scalar coefficient_cells =                     ///
                `diagnostic_coefficient_cells'
            ereturn scalar deletion_units = `diagnostic_deletion_units'
            ereturn scalar target_strata = `target_strata'
            ereturn scalar life_transition_seconds =               ///
                `life_transition_seconds'
            ereturn scalar life_work_seconds = `life_work_seconds'
            ereturn scalar life_restore_seconds = `life_restore_seconds'
            ereturn scalar life_sample_restored = `life_sample_restored'
            ereturn scalar life_mem_before_bytes = `life_mem_before_bytes'
            ereturn scalar life_mem_cleared_bytes =                ///
                `life_mem_cleared_bytes'
            ereturn scalar life_mem_work_bytes = `life_mem_work_bytes'
            ereturn scalar life_mem_restored_bytes =               ///
                `life_mem_restored_bytes'
            ereturn scalar resource_peak_bytes =                   ///
                `resource_forecasts'[`resource_row',5]
            ereturn scalar resource_mem_admit_bytes =              ///
                `resource_forecasts'[`resource_row',7]
            ereturn scalar resource_wall_upper_seconds =           ///
                `resource_forecasts'[`resource_row',8]
            ereturn scalar resource_wall_admit_seconds =           ///
                `resource_forecasts'[`resource_row',10]
            ereturn scalar resource_hard_mem_bytes =               ///
                `resource_forecasts'[`resource_row',11]
            ereturn scalar resource_hard_wall_seconds =            ///
                `resource_forecasts'[`resource_row',12]
            ereturn local preconditioner_requested "`preconditioner'"
            ereturn local preconditioner_selected ///
                "`selected_preconditioner'"
            ereturn local routing_reason `"`routing_reason'"'
            ereturn local fallback_status "`fallback_status'"
            ereturn local fallback_message `"`fallback_message'"'
            ereturn local batch_requested "`batch_requested'"
            ereturn local batch_routing_reason `"`batch_routing_reason'"'
            ereturn scalar batch = `batch'
            ereturn scalar batch_memory_budget_bytes = ///
                `batch_memory_budget_bytes'
            ereturn scalar batch_column_forecast_bytes = ///
                `batch_column_forecast_bytes'
            ereturn scalar batch_scratch_forecast_bytes = ///
                `batch_scratch_forecast_bytes'
            ereturn scalar memory_gib = `memory_gib'
            ereturn scalar active_processors = `active_processors'
            matrix colnames `route_diagnostics' = planned_rhs memory_bytes ///
                setup_seconds hierarchy_levels edge_complexity ///
                vertex_complexity structural_bytes dense_factor_bytes ///
                workers firms hybrid_vertices hybrid_edges route_code ///
                predicted_vertices predicted_edges ///
                predicted_structural_bytes predicted_scratch_bytes ///
                pilot_cap diagonal_max_iterations cmg_max_iterations ///
                diagonal_pilot_seconds hierarchy_seconds ///
                cmg_pilot_seconds projected_work_ratio forecast_peak_bytes ///
                terminal_vertices
            ereturn scalar route_planned_rhs = `route_diagnostics'[1,1]
            ereturn scalar route_forecast_peak_bytes = ///
                `route_diagnostics'[1,25]
            ereturn scalar route_hierarchy_levels = ///
                `route_diagnostics'[1,4]
            ereturn scalar route_hybrid_vertices = ///
                `route_diagnostics'[1,11]
            ereturn scalar route_hybrid_edges = ///
                `route_diagnostics'[1,12]
            ereturn scalar route_terminal_vertices = ///
                `route_diagnostics'[1,26]
            ereturn scalar route_diagonal_max_iterations = ///
                `route_diagnostics'[1,19]
            ereturn scalar route_cmg_max_iterations = ///
                `route_diagnostics'[1,20]
            ereturn scalar route_projected_work_ratio = ///
                `route_diagnostics'[1,24]
            ereturn scalar memory_forecast_bytes =                 ///
                `resource_forecasts'[`resource_row',5]
            ereturn matrix route_diagnostics = `route_diagnostics'
            matrix colnames `pilot_diagnostics' = backend pilot attempted ///
                passed iterations complete_residual rhs_schur_actions     ///
                rhs_precond_apps backend_schur_actions                   ///
                backend_precond_apps projected_work status_gate          ///
                iteration_gate residual_gate work_gate failure_reason_code
            ereturn matrix route_pilot_diagnostics = `pilot_diagnostics'
            ereturn local route_pilot_status `"`pilot_status'"'
            ereturn local route_pilot_failure_reason                     ///
                `"`pilot_failure_reason'"'
        }
        di as error `"`failure_message'"'
        if inlist("`failure_status'", "EXACT_SIZE_LIMIT", "INVALID_INPUT", ///
            "INVALID_FREQUENCY", "INVALID_TARGET_WEIGHT",                 ///
            "INVALID_IDENTIFIER", "UNSUPPORTED_DELETION") exit 198
        if inlist("`failure_status'", "INVALID_NUISANCE",                  ///
            "INVALID_TOLERANCE", "BLOCK_SIZE_LIMIT",                     ///
            "CROSS_COORDINATE_MATCH") exit 198
        exit 498
    }

    quietly timer on $KSS_BC_STAGE_VALIDATION_TIMER
    local generic_identity_resid = .
    local generic_complete_resid = .
    if "`selected_algorithm'" == "jla" &                         ///
        "`engine_selected'" == "generic" {
        tempname generic_identity_residual generic_complete_residual
        capture mata: st_numscalar("`generic_identity_residual'", ///
            kssbc_solver__target_id_resid(                        ///
                st_matrix("`raw_results'")[1..3,.]))
        if _rc | missing(scalar(`generic_identity_residual')) {
            quietly _kss_bc_post_failure "TARGET_IDENTITY_FAILED"
            di as error "generic target accounting identity is nonfinite"
            exit 498
        }
        local generic_identity_resid =                            ///
            scalar(`generic_identity_residual')
        if `generic_identity_resid' >                             ///
            4096*2.2204460492503131e-16 {
            quietly _kss_bc_post_failure "TARGET_IDENTITY_FAILED"
            di as error "generic target accounting identity failed"
            exit 498
        }
        capture mata: st_numscalar("`generic_complete_residual'", ///
            kssbc_solver__rhs_resid_max(                          ///
                st_matrix("`solver_rhs_diagnostics'")))
        if _rc | missing(scalar(`generic_complete_residual')) {
            quietly _kss_bc_post_failure "SOLVER_DIAGNOSTICS_INVALID"
            di as error "generic complete-RHS residual diagnostics are invalid"
            exit 498
        }
        local generic_complete_resid =                            ///
            scalar(`generic_complete_residual')
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
    ereturn scalar graph_retained_edges = `graph_diagnostics'[1,13]
    ereturn scalar graph_bridge_units_removed = ///
        `graph_diagnostics'[1,14]
    ereturn scalar graph_bridge_rows_removed = ///
        `graph_diagnostics'[1,15]
    ereturn scalar graph_bridge_iterations = `graph_diagnostics'[1,16]
    ereturn scalar graph_fixedpoint_iterations = ///
        `graph_diagnostics'[1,17]
    ereturn scalar graph_final_bridge_units = `graph_diagnostics'[1,18]
    ereturn scalar N_stayers = `N_stayers'
    ereturn scalar N_stayer_rows = `N_stayer_rows'
    ereturn scalar worker_levels = `diagnostics'[1,3]
    ereturn scalar firm_levels = `diagnostics'[1,4]
    ereturn scalar parameters = `diagnostics'[1,5]
    ereturn scalar full_parameters = `diagnostics'[1,23]
    ereturn scalar correction_parameters = `diagnostics'[1,24]
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
    ereturn scalar setup_seconds = `diagnostics'[1,19]
    ereturn scalar preconditioner_seconds = `diagnostics'[1,19]
    ereturn scalar preconditioner_ratio = `diagnostics'[1,20]
    ereturn scalar control_schur_rcond = `diagnostics'[1,21]
    ereturn scalar deletion_rank_gap = `diagnostics'[1,22]
    if "`selected_algorithm'" == "jla" {
        matrix colnames `solver_rhs_diagnostics' = stage batch_start rhs ///
            iterations relative_residual converged
        ereturn matrix solver_rhs_diagnostics = `solver_rhs_diagnostics'
        matrix colnames `route_diagnostics' = planned_rhs memory_bytes ///
            setup_seconds hierarchy_levels edge_complexity ///
            vertex_complexity structural_bytes dense_factor_bytes ///
            workers firms hybrid_vertices hybrid_edges route_code ///
            predicted_vertices predicted_edges ///
            predicted_structural_bytes predicted_scratch_bytes ///
            pilot_cap diagonal_max_iterations cmg_max_iterations ///
            diagonal_pilot_seconds hierarchy_seconds ///
            cmg_pilot_seconds projected_work_ratio forecast_peak_bytes ///
            terminal_vertices
        ereturn scalar route_code = `route_diagnostics'[1,13]
        ereturn scalar route_planned_rhs = `route_diagnostics'[1,1]
        ereturn scalar route_forecast_peak_bytes = ///
            `route_diagnostics'[1,25]
        ereturn scalar route_hierarchy_levels = ///
            `route_diagnostics'[1,4]
        ereturn scalar route_hybrid_vertices = ///
            `route_diagnostics'[1,11]
        ereturn scalar route_hybrid_edges = ///
            `route_diagnostics'[1,12]
        ereturn scalar route_terminal_vertices = ///
            `route_diagnostics'[1,26]
        ereturn scalar memory_forecast_bytes = ///
            `route_diagnostics'[1,25]+`batch_scratch_forecast_bytes'
        ereturn scalar route_diagonal_max_iterations = ///
            `route_diagnostics'[1,19]
        ereturn scalar route_cmg_max_iterations = ///
            `route_diagnostics'[1,20]
        ereturn scalar route_projected_work_ratio = ///
            `route_diagnostics'[1,24]
        ereturn matrix route_diagnostics = `route_diagnostics'
        matrix colnames `pilot_diagnostics' = backend pilot attempted     ///
            passed iterations complete_residual rhs_schur_actions         ///
            rhs_precond_apps backend_schur_actions backend_precond_apps   ///
            projected_work status_gate iteration_gate residual_gate       ///
            work_gate failure_reason_code
        ereturn matrix route_pilot_diagnostics = `pilot_diagnostics'
        ereturn local route_pilot_status `"`pilot_status'"'
        ereturn local route_pilot_failure_reason                         ///
            `"`pilot_failure_reason'"'
        ereturn scalar schur_seconds = `diagnostics'[1,25]
        ereturn scalar preconditioner_apply_seconds = `diagnostics'[1,26]
        ereturn scalar pcg_seconds = `diagnostics'[1,27]
        ereturn scalar solver_backend_seconds = `diagnostics'[1,28]
        ereturn scalar solver_schur_actions = `diagnostics'[1,29]
        ereturn scalar solver_schur_batches = `diagnostics'[1,30]
        ereturn scalar solver_precond_applications = ///
            `diagnostics'[1,31]
        ereturn scalar solver_precond_batches = `diagnostics'[1,32]

        matrix colnames `resource_components' = raw_stata_bytes      ///
            cell_bytes deletion_unit_bytes target_stratum_bytes      ///
            cmg_hierarchy_bytes phase_scratch_bytes                  ///
            sorting_compression_bytes solve_ahead_bytes              ///
            output_certificate_bytes preservation_transition_bytes  ///
            runtime_resident_bytes
        matrix rownames `resource_components' = compressed generic
        matrix colnames `resource_forecasts' = select_peak_bytes     ///
            transition_peak_bytes numerical_peak_bytes               ///
            restore_peak_bytes peak_bytes memory_headroom_fraction   ///
            memory_admit_bytes wall_upper_seconds                    ///
            wall_headroom_fraction wall_admit_seconds                ///
            hard_mem_bytes hard_wall_seconds memory_admitted         ///
            wall_admitted admitted
        matrix rownames `resource_forecasts' = compressed generic
        matrix colnames `resource_scaling' = row_scale               ///
            structure_scale compressed_wall_upper                    ///
            generic_wall_upper physical_scale physical_observations  ///
            leverage_rng_calls target_rng_calls rng_call_seconds     ///
            rng_total_calls rng_wall_upper
        ereturn scalar resource_raw_stata_bytes =                    ///
            `resource_components'[`resource_row',1]
        ereturn scalar resource_cell_bytes =                         ///
            `resource_components'[`resource_row',2]
        ereturn scalar resource_deletion_unit_bytes =                ///
            `resource_components'[`resource_row',3]
        ereturn scalar resource_target_stratum_bytes =               ///
            `resource_components'[`resource_row',4]
        ereturn scalar resource_cmg_hierarchy_bytes =                ///
            `resource_components'[`resource_row',5]
        ereturn scalar resource_routed_solver_bytes =                ///
            `resource_components'[`resource_row',5]
        ereturn scalar resource_phase_scratch_bytes =                ///
            `resource_components'[`resource_row',6]
        ereturn scalar resource_sort_temp_bytes =                    ///
            `resource_components'[`resource_row',7]
        ereturn scalar resource_solve_ahead_bytes =                  ///
            `resource_components'[`resource_row',8]
        ereturn scalar resource_output_bytes =                       ///
            `resource_components'[`resource_row',9]
        ereturn scalar resource_preserve_bytes =                     ///
            `resource_components'[`resource_row',10]
        ereturn scalar resource_runtime_resident_bytes =             ///
            `resource_components'[`resource_row',11]
        ereturn scalar resource_select_peak_bytes =                 ///
            `resource_forecasts'[`resource_row',1]
        ereturn scalar resource_transition_peak_bytes =             ///
            `resource_forecasts'[`resource_row',2]
        ereturn scalar resource_numerical_peak_bytes =              ///
            `resource_forecasts'[`resource_row',3]
        ereturn scalar resource_restore_peak_bytes =                ///
            `resource_forecasts'[`resource_row',4]
        ereturn scalar resource_peak_bytes =                        ///
            `resource_forecasts'[`resource_row',5]
        ereturn scalar resource_memory_headroom =                   ///
            `resource_forecasts'[`resource_row',6]
        ereturn scalar resource_mem_admit_bytes =                   ///
            `resource_forecasts'[`resource_row',7]
        ereturn scalar resource_wall_upper_seconds =                ///
            `resource_forecasts'[`resource_row',8]
        ereturn scalar resource_wall_headroom =                     ///
            `resource_forecasts'[`resource_row',9]
        ereturn scalar resource_wall_admit_seconds =                ///
            `resource_forecasts'[`resource_row',10]
        ereturn scalar resource_hard_mem_bytes =                    ///
            `resource_forecasts'[`resource_row',11]
        ereturn scalar resource_hard_wall_seconds =                 ///
            `resource_forecasts'[`resource_row',12]
        ereturn scalar resource_memory_admitted =                   ///
            `resource_forecasts'[`resource_row',13]
        ereturn scalar resource_wall_admitted =                     ///
            `resource_forecasts'[`resource_row',14]
        ereturn scalar resource_admitted =                          ///
            `resource_forecasts'[`resource_row',15]
        ereturn scalar resource_physical_scale =                    ///
            `resource_scaling'[1,5]
        ereturn scalar resource_rng_lev_calls =                     ///
            `resource_scaling'[1,7]
        ereturn scalar resource_rng_target_calls =                  ///
            `resource_scaling'[1,8]
        ereturn scalar resource_rng_call_seconds =                  ///
            `resource_scaling'[1,9]
        ereturn scalar resource_rng_total_calls =                   ///
            `resource_scaling'[1,10]
        ereturn scalar resource_rng_wall_seconds =                  ///
            `resource_scaling'[1,11]
        ereturn scalar memory_forecast_bytes =                      ///
            `resource_forecasts'[`resource_row',5]
        ereturn scalar coefficient_cells = `diagnostic_coefficient_cells'
        ereturn scalar target_strata = `target_strata'
        ereturn scalar row_cell_compression =                       ///
            `N_retained'/`diagnostic_coefficient_cells'
        ereturn scalar units_per_cell =                             ///
            `diagnostic_deletion_units'/`diagnostic_coefficient_cells'
        ereturn scalar leverage_batch = `leverage_batch'
        ereturn scalar target_batch = `target_batch'
        ereturn scalar compression_seconds = `compression_seconds'
        ereturn scalar life_transition_seconds = `life_transition_seconds'
        ereturn scalar life_work_seconds = `life_work_seconds'
        ereturn scalar life_restore_seconds = `life_restore_seconds'
        ereturn scalar life_sample_restored = `life_sample_restored'
        ereturn scalar life_preserve_forced_disk =                  ///
            `life_preserve_forced_disk'
        ereturn scalar life_mem_before_bytes = `life_mem_before_bytes'
        ereturn scalar life_mem_cleared_bytes = `life_mem_cleared_bytes'
        ereturn scalar life_mem_work_bytes = `life_mem_work_bytes'
        ereturn scalar life_mem_restored_bytes = `life_mem_restored_bytes'
        ereturn scalar rng_master_seed = `seed'
        ereturn scalar rng_leverage_probe_first =                   ///
            `rng_leverage_probe_first'
        ereturn scalar rng_leverage_probe_last =                    ///
            `rng_leverage_probe_last'
        ereturn scalar rng_target_probe_first =                     ///
            `rng_target_probe_first'
        ereturn scalar rng_target_probe_last =                      ///
            `rng_target_probe_last'
        ereturn scalar residual_acceptance_tolerance =              ///
            max(1e-11,10*`tolerance')
        ereturn matrix resource_components = `resource_components'
        ereturn matrix resource_forecasts = `resource_forecasts'
        ereturn matrix resource_scaling = `resource_scaling'
        if "`engine_selected'" == "compressed" {
            matrix colnames `scale_receipt' = coefficient_cells     ///
                deletion_units target_strata reciprocal_residual    ///
                residual_gate target_identity_residual              ///
                complete_residual stored_rows physical_observations ///
                numerical_seconds rng_seconds
            ereturn scalar correction_reciprocal_residual =        ///
                `scale_receipt'[1,4]
            ereturn scalar target_identity_residual =              ///
                `scale_receipt'[1,6]
            ereturn scalar complete_residual_max =                 ///
                `scale_receipt'[1,7]
            ereturn scalar rng_seconds = `scale_receipt'[1,11]
            ereturn matrix scale_receipt = `scale_receipt'
        }
        else {
            ereturn scalar correction_reciprocal_residual = .
            ereturn scalar target_identity_residual =              ///
                `generic_identity_resid'
            ereturn scalar complete_residual_max =                ///
                `generic_complete_resid'
            ereturn scalar rng_seconds = .
        }
        ereturn local engine_requested "`engine_requested'"
        ereturn local engine_selected "`engine_selected'"
        ereturn local fastpath_status "`fastpath_status'"
        ereturn local fastpath_message `"`fastpath_message'"'
        ereturn local resource_status "`resource_status'"
        ereturn local resource_message `"`resource_message'"'
        ereturn local resource_peak_phase "`resource_peak_phase'"
        ereturn local life_method "`life_method'"
        ereturn local rng_contract "`rng_contract'"
        ereturn local rng_implementation "`rng_implementation'"
        ereturn local rng_call_shape "`rng_call_shape'"
        ereturn local rng_runtime "`rng_runtime'"
        ereturn local rng_leverage_domain "`rng_leverage_domain'"
        ereturn local rng_target_domain "`rng_target_domain'"
        ereturn local residual_normalization                        ///
            "l2_rhs_or_absolute_zero_rhs"
        ereturn local quotient_convention "full_firm_zero_sum"
        ereturn local grounding_convention                          ///
            "last_firm_zero_after_quotient_with_grounded_equation_checked"
        ereturn local scale_status = cond(                          ///
            "`engine_selected'" == "compressed",                 ///
            "EXPERIMENTAL_SCALE_ENGINE", "GENERAL_ENGINE")
    }
    else {
        ereturn scalar memory_forecast_bytes = 0
        ereturn scalar schur_seconds = 0
        ereturn scalar preconditioner_apply_seconds = 0
        ereturn scalar pcg_seconds = 0
        ereturn scalar solver_backend_seconds = 0
        ereturn scalar solver_schur_actions = 0
        ereturn scalar solver_schur_batches = 0
        ereturn scalar solver_precond_applications = 0
        ereturn scalar solver_precond_batches = 0
    }
    ereturn scalar batch = `batch'
    ereturn scalar batch_memory_budget_bytes = ///
        `batch_memory_budget_bytes'
    ereturn scalar batch_column_forecast_bytes = ///
        `batch_column_forecast_bytes'
    ereturn scalar batch_physical_column_bytes = ///
        `batch_physical_column_bytes'
    ereturn scalar batch_scratch_forecast_bytes = ///
        `batch_scratch_forecast_bytes'
    ereturn scalar memory_gib = `memory_gib'
    ereturn scalar active_processors = `active_processors'
    ereturn scalar seed = `seed'
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar physical_limit = `physical_limit'
    ereturn scalar numerical_mcse_available = ///
        ("`selected_algorithm'" == "jla")
    ereturn local cmd "kss_bc"
    ereturn local cmdline `"kss_bc `0'"'
    ereturn local version "0.2.0-dev"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local algorithm "`selected_algorithm'"
    ereturn local batch_requested "`batch_requested'"
    ereturn local batch_routing_reason `"`batch_routing_reason'"'
    ereturn local preconditioner_requested "`preconditioner'"
    ereturn local preconditioner_selected "`selected_preconditioner'"
    ereturn local routing_reason `"`routing_reason'"'
    ereturn local fallback_status "`fallback_status'"
    ereturn local fallback_message `"`fallback_message'"'
    ereturn local deletion "`deletion'"
    ereturn local nuisance "`nuisance'"
    ereturn local target_population = cond("`deletion'" == "match", ///
        "movers", "retained observations")
    ereturn local sample_selection = cond("`deletion'" == "match", ///
        "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT",                  ///
        "MATLAB_LEAVEONEWORKER_COMPONENT")
    ereturn local connectedness_status = cond("`deletion'" == "match", ///
        "DELETION_UNIT_BRIDGE_FREE", "LEAVE_ONE_WORKER_CONNECTED")
    ereturn local frequency_convention "literal physical copies"
    ereturn local probe_order = cond("`probeorder'" == "", ///
        "outcome and per-copy target mass", ///
        "outcome, per-copy target mass, and explicit physical-observation key")
    ereturn local targetweight_convention ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local inference "not implemented"
    ereturn local numerical_error = cond("`selected_algorithm'" == "exact", ///
        "deterministic dense numerical backend", "conditional probe MCSE")
    if "`selected_algorithm'" == "exact" {
        ereturn local deletion_rank_certificate "dense Woodbury plus direct rank gate"
    }
    else if `control_count' > 0 {
        ereturn local deletion_rank_certificate "full-fit within-cell trace and direct factor gates"
    }
    else {
        ereturn local deletion_rank_certificate "FE graph and spectral JLA gate"
    }
    if "`engine_selected'" == "compressed" &                     ///
        "`selected_algorithm'" == "jla" {
        ereturn local status "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
    }
    else ereturn local status "KSS_POINT_ESTIMATES_ONLY"

    quietly timer off $KSS_BC_STAGE_VALIDATION_TIMER
    quietly timer list $KSS_BC_STAGE_VALIDATION_TIMER
    local validation_seconds = r(t$KSS_BC_STAGE_VALIDATION_TIMER)
    ereturn scalar sample_selection_seconds = `sample_selection_seconds'
    ereturn scalar validation_seconds = `validation_seconds'

    if "`nodisplay'" == "" _kss_bc_display
end

program define _kss_bc_post_failure, eclass
    version 18.0
    args failure_status
    ereturn clear
    ereturn local cmd "kss_bc"
    ereturn local version "0.2.0-dev"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local status "WITHHELD"
    ereturn local withholding_status "`failure_status'"
end

program define _kss_bc_stage_timer_ids, rclass
    version 18.0
    local timers
    forvalues id = 31/50 {
        quietly capture timer list `id'
        if missing(r(t`id')) local timers `timers' `id'
        local timer_count : word count `timers'
        if `timer_count' == 2 continue, break
    }
    local timer_count : word count `timers'
    if `timer_count' != 2 exit 498
    return scalar selection_timer = real(word("`timers'",1))
    return scalar validation_timer = real(word("`timers'",2))
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
