version 18.0
clear all
set more off
set varabbrev off

args source_root output_csv source_label source_commit firms probes
if !ustrregexm("`source_commit'", "^[0-9a-f]{40}([0-9a-f]{24})?$" ) | ///
    !inlist("`source_label'", "baseline", "candidate") |          ///
    missing(real("`firms'"),real("`probes'")) |                    ///
    real("`firms'") < 2 | real("`probes'") < 2 {
    di as error "invalid Optimization III local benchmark arguments"
    exit 198
}
confirm file `"`source_root'/varcomp_kss/varcomp_kss.ado"'
adopath ++ `"`source_root'/varcomp_kss"'
capture set processors 4
if _rc | c(processors) != 4 {
    di as error "Stata/MP did not honor the four-processor request"
    exit 459
}

/* Candidate-neutral deterministic scaling fixture: ten workers per firm,
   six stored rows and three coefficient cells per worker, two stored rows
   per cell, and the caller-supplied probe count. */
local workers = 10*real("`firms'")
local firms = real("`firms'")
local probes = real("`probes'")
local seed 8675309
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

matrix receipt = J(3,59,.)
matrix colnames receipt = run command_s selection_s graph_s compression_s ///
    transition_s work_s restore_s setup_s fit_s leverage_s target_s       ///
    correction_s rng_s schur_s precond_s pcg_s iterations schur_actions   ///
    schur_batches precond_apps precond_batches max_residual peak_bytes    ///
    work_bytes n_rows cells units strata workers firms identity_residual  ///
    r11 r21 r31 r41 r12 r22 r32 r42 r13 r23 r33 r43 r14 r24 r34 r44     ///
    result_mreldif fe_applicable fe_workspace_builds fe_buffered_batches   ///
    fe_legacy_batches fe_buffered_columns fe_legacy_columns               ///
    fe_fallback_batches fe_max_width fe_workspace_bytes fe_avoided_bytes

forvalues run = 1/3 {
    quietly timer clear 80
    quietly timer on 80
    quietly varcomp_kss y [fw=frequency], worker(worker) firm(firm)           ///
        deletion(match) deletionid(match) targetweight(target)           ///
        probeorder(observation_key) algorithm(jla) engine(compressed)     ///
        preconditioner(diagonal) memory_gib(16) wallseconds(7200)        ///
        probes(`probes') batch(16) seed(`seed') tolerance(1e-10)         ///
        maxiter(20000) nodisplay
    quietly timer off 80
    quietly timer list 80
    matrix one_result = e(results)
    matrix fe_profile = e(fe_buffer_profile)
    if `run' == 1 matrix reference = one_result
    local result_difference = mreldif(reference,one_result)
    assert "`e(engine_selected)'" == "compressed"
    assert "`e(preconditioner_selected)'" == "DIAGONAL"
    assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
    assert e(target_identity_residual) <= 1e-12
    assert "`e(fe_buffer_profile_schema)'" == "FE-BUF-PERF-V1"
    assert colsof(fe_profile) == 10
    assert `result_difference' <= 2e-9
    matrix receipt[`run',1] = (`run',r(t80),                       ///
        e(sample_selection_seconds),e(graph_seconds),                 ///
        e(compression_seconds),e(life_transition_seconds),            ///
        e(life_work_seconds),e(life_restore_seconds),e(setup_seconds), ///
        e(fit_seconds),e(leverage_seconds),e(target_seconds),          ///
        e(correction_seconds),e(rng_seconds),e(schur_seconds),         ///
        e(preconditioner_apply_seconds),e(pcg_seconds),                ///
        e(solver_iterations),e(solver_schur_actions),                  ///
        e(solver_schur_batches),e(solver_precond_applications),        ///
        e(solver_precond_batches),e(solver_max_residual),              ///
        e(resource_peak_bytes),e(life_mem_work_bytes),e(N_stored),     ///
        e(coefficient_cells),e(deletion_units),e(target_strata),       ///
        e(worker_levels),e(firm_levels),e(target_identity_residual))
    matrix receipt[`run',33] = (vec(one_result)',`result_difference',fe_profile)
    local route`run' `e(preconditioner_selected)'
    local engine`run' `e(engine_selected)'
}

clear
svmat double receipt, names(col)
generate str12 source_label = "`source_label'"
generate str64 source_commit = "`source_commit'"
generate str8 temperature = cond(run==1,"cold","warm")
generate str12 route = ""
generate str12 engine = ""
forvalues run = 1/3 {
    replace route = "`route`run''" in `run'
    replace engine = "`engine`run''" in `run'
}
generate byte processors = c(processors)
generate int probes = `probes'
generate long seed = `seed'
order source_label source_commit temperature run processors probes seed
export delimited using `"`output_csv'"', replace

di as result "KSS_FE_BUF1_SCC_PASS `source_label' `source_commit' F`firms' P`probes'"
exit 0
