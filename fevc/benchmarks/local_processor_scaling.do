version 18.0
clear all
set more off
set varabbrev off

args output_dir run_id source_commit workers_arg firms_arg probes_arg ///
    seed_arg memory_gib_arg include_128_arg

local workers = real("`workers_arg'")
local firms = real("`firms_arg'")
local probes = real("`probes_arg'")
local benchmark_seed = real("`seed_arg'")
local memory_gib = real("`memory_gib_arg'")
local include_128 = real("`include_128_arg'")
if strtrim(`"`output_dir'"') == "" | ///
    !ustrregexm("`run_id'","^[A-Za-z0-9._-]+$") | ///
    strlen("`source_commit'") != 40 | ///
    missing(`workers') | missing(`firms') | missing(`probes') | ///
    missing(`benchmark_seed') | missing(`memory_gib') | ///
    missing(`include_128') | `workers' != floor(`workers') | ///
    `firms' != floor(`firms') | `probes' != floor(`probes') | ///
    `benchmark_seed' != floor(`benchmark_seed') | ///
    `workers' < 2*`firms' | `firms' < 8 | `probes' < 2 | ///
    `memory_gib' < 1 | `memory_gib' > 56 | ///
    !inlist(`include_128',0,1) {
    di as error "invalid local processor-scaling benchmark arguments"
    exit 198
}

capture confirm file "fevc/fevc.ado"
if _rc {
    di as error "run local_processor_scaling.do from the repository root"
    exit 601
}
capture mkdir `"`output_dir'"'
adopath ++ "`c(pwd)'/fevc"

// Compile every installed runtime before the timed cells.  The measured
// command times therefore compare estimator execution rather than first-use
// Mata compilation.
quietly do "fevc/fevc.mata"
quietly do "fevc/fevc_graph.mata"
quietly do "fevc/fevc_cmg.mata"
quietly do "fevc/fevc_solver.mata"

local n_rows = 4*`workers'
set obs `n_rows'
generate long worker = floor((_n-1)/4)+1
generate byte period = mod(_n-1,4)+1
generate long home_firm = mod(worker-1,`firms')+1
generate long firm = mod(home_firm-1+cond(period==1,0, ///
    cond(period==2,1,cond(period==3,17,83))),`firms')+1
generate long actual_match = 4*(worker-1)+period
generate long observation_key = _n
generate byte frequency = 1+mod(actual_match,2)
generate double target_mass = .5+mod(_n-1,17)/17
generate double y = sin(worker/97)+cos(firm/31)+period/101+ ///
    sin(observation_key/113)

local tolerance = 1e-10
local design_spec ///
    "moderate-jumps-v1|w=`workers'|f=`firms'|n=`n_rows'|p=`probes'|seed=`benchmark_seed'"
quietly datasignature
local input_datasignature `"`r(datasignature)'"'
local processors_list "4 8 4 8"
local batches_list "32 32 64 64"
if `include_128' {
    local processors_list "`processors_list' 4 8"
    local batches_list "`batches_list' 128 128"
}
local cells : word count `processors_list'

tempname post_handle
tempfile result_rows
postfile `post_handle' str32 run_id str40 source_commit ///
    str64 input_datasignature str12 stata_version str12 stata_flavor ///
    double requested_processors actual_processors requested_batch ///
    returned_batch workers firms stored_rows physical_rows probes seed ///
    tolerance memory_gib command_seconds graph_seconds setup_seconds ///
    fit_seconds leverage_seconds target_seconds correction_seconds ///
    schur_seconds preconditioner_apply_seconds pcg_seconds ///
    solver_backend_seconds solver_iterations solver_max_residual ///
    inverse_relres solver_schur_batches solver_precond_batches ///
    result_mreldif byte rng_state_equal ///
    str16 preconditioner_selected str160 routing_reason ///
    str40 fallback_status double corrected_worker corrected_firm ///
    corrected_covariance corrected_total double rss_kib ///
    str40 rss_status using `result_rows', replace

local rng_reference
forvalues cell = 1/`cells' {
    local requested_processors : word `cell' of `processors_list'
    local requested_batch : word `cell' of `batches_list'
    set processors `requested_processors'
    local actual_processors = c(processors)
    assert `actual_processors' == `requested_processors'

    timer clear 80
    timer on 80
    capture noisily fevc y [fw=frequency], ///
        worker(worker) firm(firm) deletion(match) ///
        deletionid(actual_match) targetweight(target_mass) ///
        probeorder(observation_key) algorithm(jla) nuisance(joint) ///
        preconditioner(diagonal) probes(`probes') batch(`requested_batch') ///
        seed(`benchmark_seed') tolerance(`tolerance') maxiter(20000) ///
        memory_gib(`memory_gib') engine(generic) nodisplay
    local command_rc = _rc
    timer off 80
    quietly timer list 80
    local command_seconds = r(t80)
    if `command_rc' {
        postclose `post_handle'
        di as error "processor/batch cell failed: processors=`requested_processors' batch=`requested_batch' rc=`command_rc'"
        exit `command_rc'
    }
    assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
    assert "`e(preconditioner_selected)'" == "DIAGONAL"
    assert e(batch) == `requested_batch'
    assert e(active_processors) == `requested_processors'
    assert e(probes) == `probes'
    assert e(solver_max_residual) <= 1e-9
    assert abs(el(e(correction),1,4)- ///
        (el(e(correction),1,1)+el(e(correction),1,2)+ ///
        2*el(e(correction),1,3))) < 2e-12

    tempname estimates
    matrix `estimates' = e(results)
    local rng_after `"`c(rngstate)'"'
    if `cell' == 1 {
        matrix reference_results = `estimates'
        local rng_reference `"`rng_after'"'
        local result_mreldif = 0
    }
    else local result_mreldif = mreldif(reference_results,`estimates')
    local rng_state_equal = (`"`rng_reference'"' == `"`rng_after'"')
    assert `rng_state_equal'
    assert `result_mreldif' <= 2e-9

    local selected_route = lower("`e(preconditioner_selected)'")
    local route_reason `"`e(routing_reason)'"'
    local route_reason = subinstr(`"`route_reason'"',char(34),"'",.)
    local fallback_status = lower("`e(fallback_status)'")
    local physical_rows = e(N_physical)
    local graph_seconds = e(graph_seconds)
    local setup_seconds = e(setup_seconds)
    local fit_seconds = e(fit_seconds)
    local leverage_seconds = e(leverage_seconds)
    local target_seconds = e(target_seconds)
    local correction_seconds = e(correction_seconds)
    local schur_seconds = e(schur_seconds)
    local preconditioner_apply_seconds = e(preconditioner_apply_seconds)
    local pcg_seconds = e(pcg_seconds)
    local solver_backend_seconds = e(solver_backend_seconds)
    local solver_iterations = e(solver_iterations)
    local solver_max_residual = e(solver_max_residual)
    local inverse_relres = e(inverse_relres)
    local solver_schur_batches = e(solver_schur_batches)
    local solver_precond_batches = e(solver_precond_batches)
    local corrected_worker = `estimates'[3,1]
    local corrected_firm = `estimates'[3,2]
    local corrected_covariance = `estimates'[3,3]
    local corrected_total = `estimates'[3,4]

    // Stata does not expose process peak RSS.  Run this driver under
    // `/usr/bin/time -l` on macOS to bind one process-level maximum to the
    // resulting suite; the CSV records that the per-cell value is unavailable.
    post `post_handle' ("`run_id'") ("`source_commit'") ///
        (`"`input_datasignature'"') (string(c(stata_version))) (c(flavor)) ///
        (`requested_processors') (`actual_processors') (`requested_batch') ///
        (e(batch)) (`workers') (`firms') (e(N_stored)) (`physical_rows') ///
        (`probes') (`benchmark_seed') (`tolerance') (`memory_gib') ///
        (`command_seconds') (`graph_seconds') (`setup_seconds') ///
        (`fit_seconds') (`leverage_seconds') (`target_seconds') ///
        (`correction_seconds') (`schur_seconds') ///
        (`preconditioner_apply_seconds') (`pcg_seconds') ///
        (`solver_backend_seconds') (`solver_iterations') ///
        (`solver_max_residual') (`inverse_relres') ///
        (`solver_schur_batches') (`solver_precond_batches') ///
        (`result_mreldif') (`rng_state_equal') ("`selected_route'") ///
        (`"`route_reason'"') ("`fallback_status'") ///
        (`corrected_worker') (`corrected_firm') ///
        (`corrected_covariance') (`corrected_total') (.) ///
        ("unavailable_per_cell; use time -l")
}
postclose `post_handle'

preserve
quietly use `result_rows', clear
sort requested_batch requested_processors
export delimited using ///
    `"`output_dir'/local_processor_scaling.csv"', replace
restore

tempname marker
file open `marker' using ///
    `"`output_dir'/local_processor_scaling.stata.pass"', write text replace
file write `marker' "KSS_LOCAL_PROCESSOR_SCALING_PASS `run_id' `source_commit'" _n
file close `marker'

di as result "KSS LOCAL PROCESSOR SCALING PASS: `run_id'"
exit 0
