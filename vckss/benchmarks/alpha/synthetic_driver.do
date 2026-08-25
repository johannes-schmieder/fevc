version 18.0
clear all
set more off
set varabbrev off

args source_root output_csv case_id source_commit spec_sha backend workers_arg firms_arg probes_arg controls_arg deletion reps_arg
if !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") |                 ///
    !ustrregexm("`spec_sha'", "^[0-9a-f]{64}$") |                    ///
    !inlist("`backend'", "rust", "mata") |                           ///
    !inlist("`deletion'", "match", "observation") {
    di as error "invalid VCKSS-ALPHA-BENCH-V1 arguments"
    exit 198
}
local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local probes = real("`probes_arg'")
local controls = real("`controls_arg'")
local reps = real("`reps_arg'")
if `workers' < `firms' | `firms' < 32 | `probes' < 1 |              ///
    !inlist(`controls',0,1) | `reps' < 1 {
    di as error "invalid synthetic dimensions"
    exit 198
}
confirm file `"`source_root'/vckss/vckss.ado"'
adopath ++ `"`source_root'/vckss"'
capture set processors 4
if _rc | c(processors) != 4 exit 459

local rows = 6*`workers'
set obs `rows'
generate long observation_key = _n
generate long worker = floor((_n-1)/6)+1
generate byte within_worker = mod(_n-1,6)+1
generate byte cell_slot = ceil(within_worker/2)
generate long home = mod(worker-1,`firms')+1
generate long firm = mod(home-1 + cond(cell_slot==1,0,cond(cell_slot==2,1,31)),`firms')+1
generate long match = 3*(worker-1)+cell_slot
generate byte frequency = 1+mod(observation_key,3)
generate double target = frequency*(.5+mod(match,17)/17)
generate double control = (worker/`workers')*(within_worker-3.5)+mod(observation_key,7)/19
generate double y = sin(worker/97)+cos(firm/31)+.2*control+sin(observation_key/113)
quietly _datasignature
local data_signature `"`r(datasignature)'"'
set rng default
set seed 20260825
set sortseed 20260825
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'

matrix receipt = J(`reps',62,.)
matrix colnames receipt = run total_s n_stored n_physical n_retained workers firms cells units strata probes selected_batch iterations max_resid accept_tol identity_resid memory_bytes sample_n sample_ok data_ok rng_ok sort_ok result_diff r11 r21 r31 r41 r12 r22 r32 r42 r13 r23 r33 r43 r14 r24 r34 r44 mcse1 mcse2 mcse3 mcse4 ingest_s canon_s graph_s compress_s plan_s stayer_s solve_s native_s fit_s leverage_s target_s correction_s rng_s setup_s schur_s pcg_s precond_s rust_words rust_draws

local rhs
if `controls' == 1 local rhs control
local deletion_id match
if "`deletion'" == "observation" local deletion_id observation_key
forvalues run = 1/`reps' {
    quietly timer clear 80
    quietly timer on 80
    if "`backend'" == "rust" {
        quietly vckss y `rhs' [fw=frequency], worker(worker) firm(firm)  ///
            deletion(`deletion') deletionid(`deletion_id')             ///
            targetweight(target) probeorder(observation_key)            ///
            backend(rust) rng(counter_v1) algorithm(jla) engine(auto)   ///
            preconditioner(auto) memory_gib(56) wallseconds(7200)       ///
            probes(`probes') batch(auto) seed(8675309) tolerance(1e-10) ///
            maxiter(20000) nodisplay
    }
    else {
        quietly vckss y `rhs' [fw=frequency], worker(worker) firm(firm)  ///
            deletion(`deletion') deletionid(`deletion_id')             ///
            targetweight(target) probeorder(observation_key)            ///
            backend(mata) rng(stata) algorithm(jla) engine(auto)        ///
            preconditioner(auto) memory_gib(56) wallseconds(7200)       ///
            probes(`probes') batch(auto) seed(8675309) tolerance(1e-10) ///
            maxiter(20000) nodisplay
    }
    quietly timer off 80
    quietly timer list 80
    local total = r(t80)
    di as txt "ALPHA_BENCH_GATE command `e(cmd)' algorithm `e(algorithm)' residual " e(complete_residual_max) " / " e(residual_acceptance_tolerance) " identity " e(target_identity_residual)
    assert "`e(cmd)'" == "vckss"
    assert "`e(algorithm)'" == "jla"
    assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
    assert e(target_identity_residual) <= 1e-12
    capture confirm matrix e(V)
    di as txt "ALPHA_BENCH_GATE eV_rc " _rc
    assert _rc != 0
    matrix one_result = e(results)
    matrix one_mcse = e(numerical_mcse)
    if `run' == 1 matrix reference = one_result
    local result_difference = mreldif(reference,one_result)
    tempvar in_sample
    generate byte `in_sample' = e(sample)
    quietly count if `in_sample'
    local sample_n = r(N)
    local sample_ok = (`sample_n' == `rows')
    drop `in_sample'
    quietly _datasignature
    local data_ok = (`"`r(datasignature)'"' == `"`data_signature'"')
    local rng_ok = (`"`c(rngstate)'"' == `"`rng_before'"')
    local sort_ok = (`"`c(sortrngstate)'"' == `"`sort_rng_before'"')
    di as txt "ALPHA_BENCH_GATE state " `sample_ok' " " `data_ok' " " `rng_ok' " " `sort_ok'
    assert `sample_ok' & `data_ok' & `rng_ok' & `sort_ok'
    local ingest = .
    local canon = .
    local graph = .
    local compress = .
    local plan = .
    local stayer = .
    local solve = .
    local native = .
    local words = .
    local draws = .
    if "`backend'" == "rust" {
        matrix phase = e(rust_phase_profile)
        local ingest = phase[1,1]
        local canon = phase[1,2]
        local graph = phase[1,3]
        local compress = phase[1,4]
        local plan = phase[1,5]
        local stayer = phase[1,6]
        local solve = phase[1,7]
        local native = phase[1,8]
    }
    matrix receipt[`run',1] = (`run',`total',e(N_stored),e(N_physical),e(N_retained),e(worker_levels),e(firm_levels),e(coefficient_cells),e(deletion_units),e(target_strata),e(probes),e(batch),e(solver_iterations),e(complete_residual_max),e(residual_acceptance_tolerance),e(target_identity_residual),e(memory_forecast_bytes),`sample_n',`sample_ok',`data_ok',`rng_ok',`sort_ok',`result_difference')
    matrix receipt[`run',24] = (vec(one_result)',one_mcse,`ingest',`canon',`graph',`compress',`plan',`stayer',`solve',`native',e(fit_seconds),e(leverage_seconds),e(target_seconds),e(correction_seconds),e(rng_seconds),e(setup_seconds),e(schur_seconds),e(pcg_seconds),e(preconditioner_apply_seconds),`words',`draws')
    local engine`run' `e(engine_selected)'
    local route`run' `e(preconditioner_selected)'
    local status`run' `e(status)'
}

clear
svmat double receipt, names(col)
generate str32 case_id = "`case_id'"
generate str8 backend = "`backend'"
generate str40 source_commit = "`source_commit'"
generate str64 fixture_spec_sha256 = "`spec_sha'"
generate str12 deletion = "`deletion'"
generate byte controls = `controls'
generate byte processors = c(processors)
generate str8 temperature = cond(run==1,"cold","warm")
generate str16 engine = ""
generate str16 route = ""
generate str48 estimator_status = ""
forvalues run = 1/`reps' {
    replace engine = "`engine`run''" in `run'
    replace route = "`route`run''" in `run'
    replace estimator_status = "`status`run''" in `run'
}
order case_id backend source_commit fixture_spec_sha256 temperature run
export delimited using `"`output_csv'"', replace
di as result "VCKSS_ALPHA_BENCH_V1_PASS `case_id' `backend' `source_commit'"
exit 0
