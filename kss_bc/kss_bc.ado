*! kss_bc 0.2.0-dev 15aug2026

program define kss_bc, eclass sortpreserve
    version 18.0

    if lower(strtrim(`"`0'"')) == ", version" {
        ereturn clear
        ereturn local cmd "kss_bc"
        ereturn local version "0.2.0-dev"
        ereturn local model "linear"
        ereturn local correction "kss"
        ereturn local status "DEVELOPMENT"
        di as txt "kss_bc 0.2.0-dev (15aug2026)"
        exit
    }

    syntax varlist(numeric fv min=1) [if] [in] [fw],             ///
        WORKER(varname) FIRM(varname) [                          ///
        DELETION(string) DELETIONID(varname)                     ///
        ALGORITHM(string) NUISANCE(string)                       ///
        TARGETWeight(varname numeric) STAYERS(string)            ///
        PROBEOrder(varname numeric)                              ///
        PROBES(integer 200) BATCH(string)                        ///
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

    local expected_mata_build "kss-bc-api18-production-cmg-routing"
    capture mata: kssbc__api_level()
    local mata_runtime_loaded = (_rc == 0)
    capture mata: assert(kssbc__api_level() == 18 &                 ///
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
        capture mata: assert(kssbc__api_level() == 18 &             ///
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

    // Forecast the largest estimator scratch family conservatively as
    // fourteen retained-row vectors plus twelve coefficient vectors per
    // simultaneous probe.  Observation deletion additionally materializes
    // one literal-physical-copy sign vector per simultaneous probe, so its
    // column forecast must use the retained frequency total as well as the
    // stored-row count.  Automatic widths use at most 35% of the caller's
    // declared envelope and never exceed the probe count.  Real CZ24/CZ25
    // profiling shows that four processors stop gaining beyond 32 columns;
    // the corresponding evidence-backed cap is 64 with eight or more.  Width
    // 128 remains available explicitly but is not selected automatically.
    // This policy is deterministic and is applied before solver routing or
    // random probes.
    local batch_memory_budget_bytes = floor(`memory_gib'*1024^3*.35)
    local batch_physical_column_bytes = 0
    if "`deletion'" == "observation" {
        local batch_physical_column_bytes = ///
            8*scalar(`retained_physical_total')
    }
    local batch_column_forecast_bytes = ///
        8*(14*`N_retained'+12*`parameters')+ ///
        `batch_physical_column_bytes'
    if "`batch_requested'" == "auto" & "`selected_algorithm'" == "jla" {
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
        local batch_scratch_forecast_bytes = ///
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
    if "`selected_algorithm'" == "jla" & ///
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

    if "`selected_algorithm'" == "jla" {
        if scalar(`retained_physical_total') > `physical_limit' {
            quietly _kss_bc_post_failure "PHYSICAL_COPY_LIMIT"
            di as error "JLA physical copies exceed physical_limit()"
            exit 498
        }
    }
    if "`selected_algorithm'" == "jla" | `control_count' > 0 {
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
        tempvar semantic_target semantic_worker_min semantic_worker_max
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
        if "`deletion'" == "match" {
            sort `semantic_key' `id_worker' `id_firm' `deletion_id'
        }
        else sort `semantic_key' `id_worker' `id_firm'
    }
    else if "`deletion'" == "match" {
        sort `id_worker' `id_firm' `deletion_id' `depvar' ///
            `frequency' `target'
    }
    else {
        sort `id_worker' `id_firm' `depvar' `frequency' `target'
    }
    tempname raw_results diagnostics solver_rhs_diagnostics route_diagnostics
    tempname plugin correction
    tempname corrected kss_return mcse
    local mata_status
    local mata_message
    local selected_preconditioner NOT_APPLICABLE
    local routing_reason EXACT_ALGORITHM
    local fallback_status NOT_APPLICABLE
    local fallback_message
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
        capture mata: assert(kssbc_solver__api_level() == 18 &     ///
            kssbc_solver__build_id() ==                           ///
            "kss-bc-solver-api18-production-routing")
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
            capture mata: assert(kssbc_solver__api_level() == 18 & ///
                kssbc_solver__build_id() ==                       ///
                "kss-bc-solver-api18-production-routing")
            if _rc {
                quietly _kss_bc_post_failure "INVALID_SOLVER_RUNTIME"
                di as error "the installed KSS solver adapter is incompatible with this command"
                exit 498
            }
        }
        capture noisily mata: kssbc__stata_jla_routed(             ///
            "`depvar'", "`id_worker'", "`id_firm'",             ///
            `"`controlvars'"', "`frequency'", "`target'",      ///
            "`deletion_id'", "`touse'", "`deletion'",         ///
            "`nuisance'", `probes', `batch', `seed',            ///
            `tolerance', `maxiter', `rank_tolerance',            ///
            `block_tolerance', `blocksize_limit',                ///
            "`preconditioner'", `memory_gib'*1024^3,             ///
            "`raw_results'", "mata_status",                     ///
            "mata_message", "`diagnostics'",                    ///
            "`solver_rhs_diagnostics'", "`route_diagnostics'", ///
            "selected_preconditioner", "routing_reason",       ///
            "fallback_status", "fallback_message")
        if "`fallback_status'" == "" local fallback_status NOT_NEEDED
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
        if "`selected_algorithm'" == "jla" {
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
            ereturn scalar memory_forecast_bytes = ///
                `route_diagnostics'[1,25]+ ///
                `batch_scratch_forecast_bytes'
            ereturn matrix route_diagnostics = `route_diagnostics'
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
        ereturn scalar schur_seconds = `diagnostics'[1,25]
        ereturn scalar preconditioner_apply_seconds = `diagnostics'[1,26]
        ereturn scalar pcg_seconds = `diagnostics'[1,27]
        ereturn scalar solver_backend_seconds = `diagnostics'[1,28]
        ereturn scalar solver_schur_actions = `diagnostics'[1,29]
        ereturn scalar solver_schur_batches = `diagnostics'[1,30]
        ereturn scalar solver_precond_applications = ///
            `diagnostics'[1,31]
        ereturn scalar solver_precond_batches = `diagnostics'[1,32]
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
    ereturn local status "KSS_POINT_ESTIMATES_ONLY"

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
