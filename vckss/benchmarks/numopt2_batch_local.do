version 18.0
clear all
set more off
set varabbrev off

args source_root output_csv source_commit
if !ustrregexm("`source_commit'", "^[0-9a-f]{40}$") {
    di as error "invalid Optimization III batch benchmark arguments"
    exit 198
}
confirm file `"`source_root'/vckss/vckss.ado"'
adopath ++ `"`source_root'/vckss"'
capture set processors 4
if _rc | c(processors) != 4 exit 459

local workers 10000
local firms 1000
local probes 200
local seed 8675309
set obs 60000
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

// Compile and warm the complete command before measuring width sensitivity.
quietly vckss y [fw=frequency], worker(worker) firm(firm)             ///
    deletion(match) deletionid(match) targetweight(target)             ///
    probeorder(observation_key) algorithm(jla) engine(compressed)       ///
    preconditioner(diagonal) memory_gib(16) wallseconds(7200)          ///
    probes(`probes') batch(16) seed(`seed') tolerance(1e-10)           ///
    maxiter(20000) nodisplay
matrix reference = e(results)

matrix receipt = J(5,14,.)
matrix colnames receipt = batch command_s work_s setup_s schur_s       ///
    precond_s pcg_s iterations schur_actions schur_batches             ///
    max_residual peak_bytes result_mreldif identity_residual
local row = 0
foreach batch in 1 2 4 8 16 {
    local ++row
    quietly timer clear 80
    quietly timer on 80
    quietly vckss y [fw=frequency], worker(worker) firm(firm)         ///
        deletion(match) deletionid(match) targetweight(target)         ///
        probeorder(observation_key) algorithm(jla) engine(compressed)   ///
        preconditioner(diagonal) memory_gib(16) wallseconds(7200)      ///
        probes(`probes') batch(`batch') seed(`seed') tolerance(1e-10)  ///
        maxiter(20000) nodisplay
    quietly timer off 80
    quietly timer list 80
    matrix one_result = e(results)
    local difference = mreldif(reference,one_result)
    assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
    assert e(target_identity_residual) <= 1e-12
    assert `difference' <= 2e-9
    matrix receipt[`row',1] = (`batch',r(t80),e(life_work_seconds),   ///
        e(setup_seconds),e(schur_seconds),                            ///
        e(preconditioner_apply_seconds),e(pcg_seconds),               ///
        e(solver_iterations),e(solver_schur_actions),                 ///
        e(solver_schur_batches),e(solver_max_residual),               ///
        e(resource_peak_bytes),`difference',e(target_identity_residual))
}

clear
svmat double receipt, names(col)
generate str40 source_commit = "`source_commit'"
generate byte processors = c(processors)
generate int probes = `probes'
generate long seed = `seed'
order source_commit processors probes seed
export delimited using `"`output_csv'"', replace
di as result "KSS_NUMOPT2_BATCH_PASS `source_commit'"
exit 0
