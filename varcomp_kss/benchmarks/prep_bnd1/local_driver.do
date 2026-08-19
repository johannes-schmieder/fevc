version 18.0
clear all
set more off
set varabbrev off

args source_root output_csv profile_csv source_label source_commit firms_arg probes_arg ///
    repetitions_arg run_kind input_dta pair_order round_arg
if "`firms_arg'" == "" local firms_arg 256
if "`probes_arg'" == "" local probes_arg 40
if "`repetitions_arg'" == "" local repetitions_arg 3
if "`run_kind'" == "" local run_kind local
if !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") |       ///
    !inlist("`source_label'", "baseline", "candidate") |    ///
    !inlist("`run_kind'", "local", "scc", "cz18") |       ///
    missing(real("`probes_arg'"),real("`repetitions_arg'")) | ///
    ("`run_kind'" != "cz18" & (missing(real("`firms_arg'")) | real("`firms_arg'") < 32)) | ///
    real("`probes_arg'") < 2 |                              ///
    !inlist(real("`repetitions_arg'"),1,3) {
    di as error "invalid PREP-BND-1 local benchmark arguments"
    exit 198
}
confirm file `"`source_root'/varcomp_kss/varcomp_kss.ado"'
adopath ++ `"`source_root'/varcomp_kss"'
capture set processors 4
if _rc | c(processors) != 4 {
    di as error "Stata/MP did not honor the four-processor request"
    exit 459
}

/* Candidate-neutral synthetic path: ten workers and 60 stored rows per firm,
   three coefficient/deletion/target cells per worker.  The CZ18 path consumes
   only the fixed input whose hash is enforced by the SCC wrapper. */
local firms = real("`firms_arg'")
local probes = real("`probes_arg'")
local seed 8675309
local repetitions = real("`repetitions_arg'")
local load_seconds 0
local input_sha256 "GENERATED_DETERMINISTIC"
if "`run_kind'" == "cz18" {
    if "`input_dta'" == "" | !inlist("`pair_order'", "ab", "ba") | ///
        missing(real("`round_arg'")) | !inlist(real("`round_arg'"),1,2,3) | ///
        `probes' != 20 | `repetitions' != 1 {
        di as error "invalid PREP-BND-1 CZ18 arguments"
        exit 198
    }
    confirm file `"`input_dta'"'
    local load_started = clock(c(current_date)+" "+c(current_time), "DMY hms")
    quietly use `"`input_dta'"', clear
    local load_finished = clock(c(current_date)+" "+c(current_time), "DMY hms")
    local load_seconds = (`load_finished'-`load_started')/1000
    confirm numeric variable worker firm y_minus_xb observation_key
    isid observation_key
    quietly count
    assert r(N) == 8201888
    local input_sha256 "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
    local firms 0
}
else {
    local workers = 10*`firms'
    set obs `=6*`workers''
    generate long observation_key = _n
    generate long worker = floor((_n-1)/6)+1
    generate byte within_worker = mod(_n-1,6)+1
    generate byte cell_slot = ceil(within_worker/2)
    generate long firm = mod(worker-1 +                         ///
        cond(cell_slot==1,0,cond(cell_slot==2,1,31)),`firms')+1
    generate long match = 3*(worker-1)+cell_slot
    generate byte frequency = 1+mod(observation_key,3)
    generate double target = frequency*(.5+mod(match,17)/17)
    generate double y = sin(worker/97)+cos(firm/31)+             ///
        .03*within_worker+sin(observation_key/113)
    sort observation_key
}

quietly _datasignature
local caller_signature `"`r(datasignature)'"'
local caller_sortedby : sortedby
set rng default
set seed 20260819
set sortseed 20260819
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'

tempname profile_post
tempfile profiles
postfile `profile_post' str12 source_label str40 source_commit int run ///
    str32 matrix_name str64 schema str64 metric double value using `profiles', replace

forvalues run = 1/`repetitions' {
    quietly timer clear 80
    quietly timer on 80
    if "`run_kind'" == "cz18" {
        quietly varcomp_kss y_minus_xb, worker(worker) firm(firm)        ///
            deletion(match) probeorder(observation_key) algorithm(jla)  ///
            engine(auto) preconditioner(auto) memory_gib(56)            ///
            wallseconds(1680) probes(`probes') batch(auto) seed(`seed') ///
            tolerance(1e-10) maxiter(20000) nodisplay
    }
    else {
        quietly varcomp_kss y [fw=frequency], worker(worker) firm(firm)  ///
            deletion(match) deletionid(match) targetweight(target)      ///
            probeorder(observation_key) algorithm(jla) engine(compressed) ///
            preconditioner(diagonal) memory_gib(16) wallseconds(3600)   ///
            probes(`probes') batch(16) seed(`seed') tolerance(1e-10)    ///
            maxiter(20000) nodisplay
    }
    quietly timer off 80
    quietly timer list 80
    local command_seconds = r(t80)

    matrix one_result = e(results)
    matrix prep_profile = e(prep_profile)
    matrix fe_profile = e(fe_buffer_profile)
    if `run' == 1 matrix reference = one_result
    local result_difference = mreldif(reference,one_result)
    assert "`e(engine_selected)'" == "compressed"
    if "`run_kind'" == "cz18" assert "`e(preconditioner_selected)'" == "CMG"
    else assert "`e(preconditioner_selected)'" == "DIAGONAL"
    assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
    assert e(target_identity_residual) <= 1e-12
    assert "`e(fe_buffer_profile_schema)'" == "FE-BUF-PERF-V1"
    assert colsof(prep_profile) == 7 & colsof(fe_profile) == 10
    assert `result_difference' <= 2e-9

    local prep_names : colnames prep_profile
    forvalues column = 1/`=colsof(prep_profile)' {
        local metric : word `column' of `prep_names'
        post `profile_post' ("`source_label'") ("`source_commit'") (`run') ///
            ("prep_profile") ("PREP-RHS-PERF-V1") ("`metric'")          ///
            (prep_profile[1,`column'])
    }
    capture matrix prep_boundary_profile = e(prep_boundary_profile)
    local prep_boundary_present = (_rc == 0)
    if `prep_boundary_present' {
        assert "`e(prep_boundary_profile_schema)'" == "PREP-BND-PERF-V1"
        assert colsof(prep_boundary_profile) == 11
        local boundary_names : colnames prep_boundary_profile
        forvalues column = 1/`=colsof(prep_boundary_profile)' {
            local metric : word `column' of `boundary_names'
            post `profile_post' ("`source_label'") ("`source_commit'") (`run') ///
                ("prep_boundary_profile") ("PREP-BND-PERF-V1") ("`metric'")   ///
                (prep_boundary_profile[1,`column'])
        }
    }
    capture matrix prep_boundary_counts = e(prep_boundary_counts)
    if !_rc {
        assert `"`e(prep_boundary_counts_schema)'"' == "PREP-BND-COUNTS-V1"
        local counter_schema "PREP-BND-COUNTS-V1"
        local counter_names : colnames prep_boundary_counts
        forvalues column = 1/`=colsof(prep_boundary_counts)' {
            local metric : word `column' of `counter_names'
            post `profile_post' ("`source_label'") ("`source_commit'") (`run') ///
                ("prep_boundary_counts") ("`counter_schema'") ("`metric'") ///
                (prep_boundary_counts[1,`column'])
        }
    }

    tempvar in_sample
    generate byte `in_sample' = e(sample)
    quietly count if `in_sample'
    local sample_count = r(N)
    quietly _datasignature `in_sample', nonames
    local sample_signature`run' `"`r(datasignature)'"'
    drop `in_sample'
    quietly _datasignature
    local data_restored = (`"`r(datasignature)'"' == `"`caller_signature'"')
    local rng_restored = (`"`c(rngstate)'"' == `"`rng_before'"')
    local sort_rng_restored = (`"`c(sortrngstate)'"' == `"`sort_rng_before'"')
    local restored_sortedby : sortedby
    local sort_restored = (`"`restored_sortedby'"' == `"`caller_sortedby'"')
    assert `data_restored' & `rng_restored' & `sort_rng_restored' & `sort_restored'
    assert e(life_sample_restored) == 1

    matrix one = (`run',`load_seconds',`command_seconds',e(sample_selection_seconds), ///
        e(graph_seconds),prep_profile[1,3],e(compression_seconds),      ///
        prep_profile[1,7],e(life_transition_seconds),e(life_work_seconds), ///
        e(life_restore_seconds),e(setup_seconds),e(fit_seconds),       ///
        e(leverage_seconds),e(target_seconds),e(correction_seconds),   ///
        e(rng_seconds),e(schur_seconds),e(preconditioner_apply_seconds), ///
        e(pcg_seconds),e(solver_iterations),e(solver_schur_actions),   ///
        e(solver_schur_batches),e(solver_precond_applications),        ///
        e(solver_precond_batches),e(solver_max_residual),              ///
        e(residual_acceptance_tolerance),e(resource_peak_bytes),       ///
        e(life_mem_work_bytes),e(N_stored),e(N_retained),              ///
        e(coefficient_cells),e(deletion_units),e(target_strata),       ///
        e(worker_levels),e(firm_levels),e(target_identity_residual),   ///
        `sample_count',e(life_sample_restored),`data_restored',        ///
        `rng_restored',`sort_rng_restored',`sort_restored',            ///
        vec(one_result)',`result_difference',`prep_boundary_present',fe_profile)
    if `run' == 1 matrix receipt = one
    else matrix receipt = receipt \ one
    local route`run' `e(preconditioner_selected)'
    local engine`run' `e(engine_selected)'
    local status`run' `e(status)'
    local batch`run' = e(batch)
}
postclose `profile_post'

matrix colnames receipt = run load_s command_s selection_s graph_s semantic_s ///
    compression_s prep_observed_s transition_s work_s restore_s setup_s fit_s ///
    leverage_s target_s correction_s rng_s schur_s precond_s pcg_s iterations ///
    schur_actions schur_batches precond_apps precond_batches max_residual      ///
    acceptance peak_bytes work_bytes n_rows n_retained cells units strata     ///
    workers firms identity_residual sample_count life_sample_restored          ///
    data_restored rng_restored sort_rng_restored sort_restored                 ///
    r11 r21 r31 r41 r12 r22 r32 r42 r13 r23 r33 r43 r14 r24 r34 r44         ///
    result_mreldif prep_boundary_present fe_applicable fe_workspace_builds     ///
    fe_buffered_batches fe_legacy_batches fe_buffered_columns fe_legacy_columns ///
    fe_fallback_batches fe_max_width fe_workspace_bytes fe_avoided_bytes

clear
svmat double receipt, names(col)
generate str12 source_label = "`source_label'"
generate str40 source_commit = "`source_commit'"
generate str8 temperature = cond(run==1,"cold","warm")
generate str12 route = ""
generate str12 engine = ""
generate str48 estimator_status = ""
generate int selected_batch = .
generate str244 sample_signature = ""
forvalues run = 1/`repetitions' {
    replace route = "`route`run''" in `run'
    replace engine = "`engine`run''" in `run'
    replace estimator_status = "`status`run''" in `run'
    replace selected_batch = `batch`run'' in `run'
    replace sample_signature = `"`sample_signature`run''"' in `run'
}
generate byte processors = c(processors)
generate int probes = `probes'
generate long seed = `seed'
generate str64 input_sha256 = "`input_sha256'"
generate str2 pair_order = "`pair_order'"
generate byte pair_round = real("`round_arg'")
order source_label source_commit temperature run processors probes seed ///
    input_sha256 pair_order pair_round selected_batch route engine       ///
    estimator_status sample_signature
export delimited using `"`output_csv'"', replace

use `profiles', clear
export delimited using `"`profile_csv'"', replace
if "`run_kind'" == "local" ///
    di as result "KSS_PREP_BND1_LOCAL_PASS `source_label' `source_commit'"
else if "`run_kind'" == "scc" ///
    di as result "KSS_PREP_BND1_SCC_PASS `source_label' `source_commit' F`firms' P`probes'"
else di as result "KSS_PREP_BND1_CZ18_PASS `source_label' `source_commit' `pair_order' R`round_arg'"
exit 0
