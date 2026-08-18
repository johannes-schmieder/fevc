version 18.0
clear all
set more off
set varabbrev off

args experiment_id stage prepared_dta prepared_sha estimator_input_sha ///
    wage_sha data_manifest_sha bundle_sha preconditioner batch_arg ///
    probes_arg seed_arg memory_arg temperature repetitions_arg ///
    processors_arg source_commit output_dir

local probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")
local declared_memory = real("`memory_arg'")
local repetitions = real("`repetitions_arg'")
if !ustrregexm("`experiment_id'", "^[A-Za-z0-9._-]+$") | ///
    !(inlist("`stage'", "bundle_smoke", "install_auto", "install_cmg", ///
        "license", "selector", "fixed", "cz18_preflight", ///
        "calibration", "full") | "`stage'" == "stress2x") | ///
    !ustrregexm("`bundle_sha'", "^[0-9a-f]{64}$") | ///
    !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") | ///
    !inlist("`preconditioner'", "-", "auto", "diagonal", "cmg") | ///
    !("`batch_arg'" == "-" | "`batch_arg'" == "auto" | ///
        ustrregexm("`batch_arg'", "^[0-9]+$")) | ///
    !inlist("`temperature'", "cold", "warm") | ///
    missing(`declared_memory') | `declared_memory' < 1 | ///
    `declared_memory' > 56 | missing(`repetitions') | ///
    `repetitions' != floor(`repetitions') | !inlist(`repetitions', 1, 2) {
    di as error "invalid KSS-PROD SCC driver arguments"
    exit 198
}
if "`stage'" != "license" & (missing(`probes') | `probes' < 2 | ///
    `probes' != floor(`probes')) {
    di as error "non-license stages require at least two probes"
    exit 198
}
capture mkdir `"`output_dir'"'
local requested_processors = real("`processors_arg'")
if missing(`requested_processors') | !inlist(`requested_processors', 4, 8) {
    di as error "invalid requested processor count"
    exit 198
}
capture set processors `requested_processors'
local actual_processors = c(processors)
if missing(`requested_processors') local requested_processors = `actual_processors'
if "`stage'" != "license" & (`actual_processors' != `requested_processors' | ///
    c(MP) != 1) {
    di as error "Stata/MP did not honor the requested processor count"
    exit 459
}

// A license probe records capacity without disguising an MP cap as a
// numerical failure. The wrapper launches a fresh Stata process for each
// version/slot request.
if "`stage'" == "license" {
    preserve
    clear
    set obs 1
    generate str64 experiment_id = "`experiment_id'"
    generate str24 stage = "license"
    generate double repetition = 1
    generate str64 bundle_sha256 = "`bundle_sha'"
    generate str40 source_commit = "`source_commit'"
    generate str12 stata_version = string(c(stata_version))
    generate str12 stata_flavor = c(flavor)
    generate byte stata_mp = c(MP)
    generate double requested_processors = `requested_processors'
    generate double actual_processors = `actual_processors'
    generate str32 estimator_status = cond(`actual_processors' == ///
        `requested_processors' & c(MP) == 1, "LICENSE_CAPACITY_PASS", ///
        "LICENSE_CAPACITY_FAILURE")
    export delimited using `"`output_dir'/prod_`experiment_id'.csv"', replace
    restore
    tempname license_marker
    if `actual_processors' != `requested_processors' | c(MP) != 1 {
        file open `license_marker' using ///
            `"`output_dir'/prod_`experiment_id'.stata.fail"', ///
            write text replace
        file write `license_marker' ///
            "KSS_PROD_LICENSE_CAPACITY_FAILURE `experiment_id' `bundle_sha'" _n
        file close `license_marker'
        di as error "requested Stata/MP processor capacity is unavailable"
        exit 459
    }
    file open `license_marker' using ///
        `"`output_dir'/prod_`experiment_id'.stata.pass"', write text replace
    file write `license_marker' ///
        "KSS_PROD_LICENSE_PASS `experiment_id' `bundle_sha'" _n
    file close `license_marker'
    di as result "KSS_PROD SCC LICENSE PASS: `experiment_id'"
    exit 0
}

local installed_public_path = 0
if inlist("`stage'", "install_auto", "install_cmg") {
    local install_root `"`output_dir'/plus"'
    capture mkdir `"`install_root'"'
    sysdir set PLUS `"`install_root'/"'
    net install varcomp_kss, from(`"`c(pwd)'/varcomp_kss"') replace
    discard
    mata: mata clear
    capture findfile varcomp_kss.ado
    if _rc | strpos(`"`r(fn)'"', `"`install_root'"') != 1 {
        di as error "normal net install did not activate the run-isolated package"
        exit 601
    }
    local installed_public_path = 1
}
else adopath ++ "`c(pwd)'/varcomp_kss"
if inlist("`stage'", "bundle_smoke", "install_auto", "install_cmg", "selector") {
    quietly set obs 800
    generate long worker = ceil(_n/4)
    generate byte within = mod(_n-1,4)
    generate long firm = mod(worker-1+(within>=2),20)+1
    generate int period = within+1
    generate double y_minus_xb = ///
        sin(worker/17)+cos(firm/7)+(within-1.5)/10
    generate long observation_key = _n
    drop within
}
else {
    if !ustrregexm("`prepared_sha'", "^[0-9a-f]{64}$") | ///
        !ustrregexm("`estimator_input_sha'", "^[0-9a-f]{64}$") | ///
        !ustrregexm("`wage_sha'", "^[0-9a-f]{64}$") | ///
        !ustrregexm("`data_manifest_sha'", "^[0-9a-f]{64}$") {
        di as error "data stages require complete input SHA-256 values"
        exit 198
    }
    confirm file `"`prepared_dta'"'
    quietly use `"`prepared_dta'"', clear
    confirm numeric variable worker firm period y_minus_xb observation_key
    isid observation_key
    if "`stage'" == "stress2x" {
        // Form two exact copies of the real CZ18 graph, then connect them by
        // a four-edge worker--firm cycle.  The two connector workers and two
        // cross-component firms ensure that no connector deletion unit is a
        // bridge.  This reaches at least twice every registered retained and
        // graph dimension, with strict row/worker growth, without weakening
        // the sample selector or changing its fixed-point rules.
        // Source identifiers need not be positive, dense, or stored as
        // doubles.  Promote them before arithmetic and shift the second copy
        // by the exact observed integer span so the two ID intervals are
        // disjoint for any signed source coding.
        recast double worker firm
        assert !missing(worker) & worker == floor(worker) & abs(worker) < 2^51
        assert !missing(firm) & firm == floor(firm) & abs(firm) < 2^51
        quietly summarize worker, meanonly
        local worker_min = r(min)
        local worker_max = r(max)
        local worker_shift = `worker_max'-`worker_min'+1
        quietly summarize firm, meanonly
        local firm_min = r(min)
        local firm_max = r(max)
        local firm_shift = `firm_max'-`firm_min'+1
        if `worker_shift' < 1 | `firm_shift' < 1 {
            di as error "invalid stress identifier span"
            exit 459
        }
        local connector_firm_a = `firm_min'
        generate byte __stress_copy = 0
        expand 2, generate(__stress_added_copy)
        replace __stress_copy = __stress_added_copy
        drop __stress_added_copy
        replace worker = worker+`worker_shift' if __stress_copy
        replace firm = firm+`firm_shift' if __stress_copy
        assert abs(worker) < 2^52-2 & abs(firm) < 2^52
        local connector_firm_b = `connector_firm_a'+`firm_shift'
        quietly summarize worker, meanonly
        local connector_worker_a = r(max)+1
        local connector_worker_b = r(max)+2
        local stress_rows_before_connectors = _N
        set obs `=`stress_rows_before_connectors'+4'
        replace worker = `connector_worker_a' in ///
            `=`stress_rows_before_connectors'+1'/`=`stress_rows_before_connectors'+2'
        replace worker = `connector_worker_b' in ///
            `=`stress_rows_before_connectors'+3'/`=`stress_rows_before_connectors'+4'
        replace firm = `connector_firm_a' in `=`stress_rows_before_connectors'+1'
        replace firm = `connector_firm_b' in `=`stress_rows_before_connectors'+2'
        replace firm = `connector_firm_a' in `=`stress_rows_before_connectors'+3'
        replace firm = `connector_firm_b' in `=`stress_rows_before_connectors'+4'
        replace period = 1 in `=`stress_rows_before_connectors'+1'
        replace period = 2 in `=`stress_rows_before_connectors'+2'
        replace period = 1 in `=`stress_rows_before_connectors'+3'
        replace period = 2 in `=`stress_rows_before_connectors'+4'
        replace y_minus_xb = 0 in ///
            `=`stress_rows_before_connectors'+1'/`=`stress_rows_before_connectors'+4'
        replace __stress_copy = 2 in ///
            `=`stress_rows_before_connectors'+1'/`=`stress_rows_before_connectors'+4'
        replace observation_key = _n
        isid observation_key
    }
}
quietly count
local input_rows = r(N)

local batch_option "batch(`batch_arg')"
local route_option "preconditioner(`preconditioner')"
local memory_option "memory_gib(`declared_memory')"
local algorithm_requested = cond("`stage'" == "install_auto", "auto", "jla")
local command_options "deletion(match) algorithm(`algorithm_requested') probes(`probes')"
local command_options "`command_options' `batch_option' `route_option'"
local command_options "`command_options' `memory_option'"
local command_options "`command_options' probeorder(observation_key)"
local command_options "`command_options' seed(`benchmark_seed')"
local command_options "`command_options' tolerance(1e-10) maxiter(20000) nodisplay"

// Warm evidence is intentionally a separate Stata process and output. Its
// first command is an unmeasured cache/JIT prime with the identical bytes,
// sample, seed, and tuning. Cold evidence measures the first invocation.
if "`temperature'" == "warm" {
    capture noisily varcomp_kss y_minus_xb, worker(worker) firm(firm) `command_options'
    if _rc {
        di as error "warm-prime invocation failed"
        exit _rc
    }
}

tempfile result_rows
tempname post_handle
postfile `post_handle' str64 experiment_id str24 stage str8 temperature ///
    double repetition str40 source_commit str64 bundle_sha256 ///
    str64 data_manifest_sha256 str64 prepared_sha256 ///
    str64 estimator_input_sha256 ///
    str64 wage_input_sha256 str12 stata_version str12 stata_flavor ///
    byte stata_mp double requested_processors actual_processors input_rows ///
    requested_probes seed tolerance declared_memory_gib command_rc ///
    command_seconds N_stored N_retained N_requested N_mover_input ///
    N_graph_dropped worker_levels firm_levels deletion_units ///
    graph_edges graph_retained_edges graph_bridge_units_removed ///
    graph_bridge_rows_removed graph_fixedpoint_iterations ///
    selected_batch batch_scratch_forecast_bytes memory_forecast_bytes ///
    route_planned_rhs route_hierarchy_levels route_hybrid_vertices ///
    route_hybrid_edges route_terminal_vertices ///
    route_diagonal_max_iterations ///
    route_cmg_max_iterations route_forecast_peak_bytes ///
    solver_iterations solver_max_residual max_leverage ///
    fit_seconds leverage_seconds ///
    target_seconds correction_seconds setup_seconds schur_seconds ///
    preconditioner_apply_seconds pcg_seconds solver_schur_batches ///
    solver_precond_batches plugin_worker ///
    plugin_firm plugin_covariance plugin_total corrected_worker ///
    corrected_firm corrected_covariance corrected_total ///
    byte installed_public_path str8 algorithm_requested str8 algorithm_selected ///
    str12 batch_requested str16 preconditioner_requested ///
    str16 preconditioner_selected str160 routing_reason ///
    str40 fallback_status str244 fallback_message ///
    str40 estimator_status byte rng_state_reproducible using `result_rows'

local any_failure = 0
local rng_reference
forvalues repetition = 1/`repetitions' {
    // Mata's profiling service owns Stata's timer registry while varcomp_kss is
    // running.  Measure the enclosing command from the wall clock instead of
    // starting an outer timer that Mata would invalidate.
    local command_started = clock(c(current_date)+" "+c(current_time), ///
        "DMY hms")
    capture noisily varcomp_kss y_minus_xb, worker(worker) firm(firm) `command_options'
    local command_rc = _rc
    local command_finished = clock(c(current_date)+" "+c(current_time), ///
        "DMY hms")
    local command_seconds = (`command_finished'-`command_started')/1000
    local rng_after `"`c(rngstate)'"'
    if `repetition' == 1 local rng_reference `"`rng_after'"'
    local rng_reproducible = (`"`rng_reference'"' == `"`rng_after'"')

    local estimator_status `"`e(status)'"'
    if `"`estimator_status'"' == "" local estimator_status "COMMAND_FAILURE"
    local requested_return `"`e(preconditioner_requested)'"'
    local selected_return `"`e(preconditioner_selected)'"'
    local algorithm_return `"`e(algorithm)'"'
    local requested_return = lower(strtrim("`requested_return'"))
    local selected_return = lower(strtrim("`selected_return'"))
    local routing_reason `"`e(routing_reason)'"'
    local fallback_status `"`e(fallback_status)'"'
    local fallback_message `"`e(fallback_message)'"'
    local batch_requested `"`e(batch_requested)'"'
    if `"`requested_return'"' == "" local requested_return "`preconditioner'"
    if `"`algorithm_return'"' == "" local algorithm_return "UNKNOWN"
    if `"`batch_requested'"' == "" local batch_requested "`batch_arg'"
    if `"`fallback_status'"' == "" & `command_rc' != 0 ///
        local fallback_status "COMMAND_FAILURE"
    local fallback_message = subinstr(`"`fallback_message'"', char(34), "'", .)
    local routing_reason = subinstr(`"`routing_reason'"', char(34), "'", .)

    foreach scalar_name in N_stored N_retained N_requested N_mover_input ///
        N_graph_dropped worker_levels firm_levels deletion_units ///
        graph_edges graph_retained_edges graph_bridge_units_removed ///
        graph_bridge_rows_removed graph_fixedpoint_iterations batch ///
        batch_scratch_forecast_bytes memory_forecast_bytes ///
        route_planned_rhs route_hierarchy_levels route_hybrid_vertices ///
        route_hybrid_edges route_terminal_vertices ///
        route_diagonal_max_iterations ///
        route_cmg_max_iterations route_forecast_peak_bytes ///
        solver_iterations solver_max_residual max_leverage ///
        fit_seconds leverage_seconds ///
        target_seconds correction_seconds setup_seconds schur_seconds ///
        preconditioner_apply_seconds pcg_seconds solver_schur_batches ///
        solver_precond_batches {
        local `scalar_name' = .
        capture local `scalar_name' = e(`scalar_name')
    }
    local plugin_worker = .
    local plugin_firm = .
    local plugin_covariance = .
    local plugin_total = .
    local corrected_worker = .
    local corrected_firm = .
    local corrected_covariance = .
    local corrected_total = .
    if `command_rc' == 0 & `"`estimator_status'"' == ///
        "KSS_POINT_ESTIMATES_ONLY" {
        tempname estimates
        matrix `estimates' = e(results)
        local plugin_worker = `estimates'[1,1]
        local plugin_firm = `estimates'[1,2]
        local plugin_covariance = `estimates'[1,3]
        local plugin_total = `estimates'[1,4]
        local corrected_worker = `estimates'[3,1]
        local corrected_firm = `estimates'[3,2]
        local corrected_covariance = `estimates'[3,3]
        local corrected_total = `estimates'[3,4]

        if "`algorithm_return'" == "jla" {
            // Preserve the complete original-system residual certificate for
            // every routed RHS. The aggregate maximum is not sufficient.
            tempname rhs_evidence
            matrix `rhs_evidence' = e(solver_rhs_diagnostics)
            preserve
            clear
            quietly set obs `=rowsof(`rhs_evidence')'
            svmat double `rhs_evidence', names(col)
            generate long repetition = `repetition'
            generate str64 experiment_id = "`experiment_id'"
            order experiment_id repetition stage batch_start rhs iterations ///
                relative_residual converged
            export delimited using ///
                `"`output_dir'/rhs_repetition_`repetition'.csv"', replace
            restore
        }

        if `repetition' == 1 {
            tempvar retained_sample
            generate byte `retained_sample' = e(sample)
            preserve
            keep if `retained_sample'
            keep worker firm
            duplicates drop
            sort worker firm
            export delimited using ///
                `"`output_dir'/retained_matches.csv"', replace
            restore
            if "`stage'" == "full" {
                preserve
                keep if `retained_sample'
                keep worker firm period y_minus_xb observation_key
                sort worker observation_key
                isid observation_key
                save `"`output_dir'/retained_sample.dta"', replace
                restore
            }
            drop `retained_sample'
        }
    }
    else local any_failure = 1

    post `post_handle' ("`experiment_id'") ("`stage'") ///
        ("`temperature'") (`repetition') ("`source_commit'") ///
        ("`bundle_sha'") ("`data_manifest_sha'") ///
        ("`prepared_sha'") ("`estimator_input_sha'") ///
        ("`wage_sha'") (string(c(stata_version))) ///
        (c(flavor)) (c(MP)) (`requested_processors') (`actual_processors') ///
        (`input_rows') (`probes') (`benchmark_seed') (1e-10) ///
        (`declared_memory') (`command_rc') (`command_seconds') ///
        (`N_stored') (`N_retained') (`N_requested') ///
        (`N_mover_input') (`N_graph_dropped') ///
        (`worker_levels') (`firm_levels') ///
        (`deletion_units') (`graph_edges') ///
        (`graph_retained_edges') (`graph_bridge_units_removed') ///
        (`graph_bridge_rows_removed') (`graph_fixedpoint_iterations') ///
        (`batch') (`batch_scratch_forecast_bytes') ///
        (`memory_forecast_bytes') (`route_planned_rhs') ///
        (`route_hierarchy_levels') (`route_hybrid_vertices') ///
        (`route_hybrid_edges') (`route_terminal_vertices') ///
        (`route_diagonal_max_iterations') ///
        (`route_cmg_max_iterations') (`route_forecast_peak_bytes') ///
        (`solver_iterations') (`solver_max_residual') ///
        (`max_leverage') (`fit_seconds') ///
        (`leverage_seconds') (`target_seconds') (`correction_seconds') ///
        (`setup_seconds') (`schur_seconds') ///
        (`preconditioner_apply_seconds') (`pcg_seconds') ///
        (`solver_schur_batches') (`solver_precond_batches') ///
        (`plugin_worker') (`plugin_firm') (`plugin_covariance') ///
        (`plugin_total') (`corrected_worker') (`corrected_firm') ///
        (`corrected_covariance') (`corrected_total') ///
        (`installed_public_path') ("`algorithm_requested'") ///
        ("`algorithm_return'") ///
        ("`batch_requested'") ("`requested_return'") ///
        ("`selected_return'") (`"`routing_reason'"') ///
        ("`fallback_status'") (`"`fallback_message'"') ///
        ("`estimator_status'") (`rng_reproducible')
}
postclose `post_handle'
preserve
quietly use `result_rows', clear
export delimited using `"`output_dir'/prod_`experiment_id'.csv"', replace
restore

tempname marker
if `any_failure' {
    file open `marker' using `"`output_dir'/prod_`experiment_id'.stata.fail"', ///
        write text replace
    file write `marker' ///
        "KSS_PROD_TYPED_FAILURE `experiment_id' `bundle_sha'" _n
    file close `marker'
    di as error "KSS_PROD SCC typed estimator failure: `experiment_id'"
    exit 459
}
file open `marker' using `"`output_dir'/prod_`experiment_id'.stata.pass"', ///
    write text replace
file write `marker' "KSS_PROD_ESTIMATOR_PASS `experiment_id' `bundle_sha'" _n
file close `marker'
di as result "KSS_PROD SCC ESTIMATOR PASS: `experiment_id'"
exit 0
