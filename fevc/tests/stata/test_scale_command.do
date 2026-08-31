version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "fevc/fevc.ado"
if _rc {
    capture confirm file "../../fevc.ado"
    if _rc {
        di as error "run from the repository root or fevc/tests/stata"
        exit 601
    }
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
quietly do `"`pkgroot'/fevc_rng.mata"'

// A deletion-safe K(3,3) fixture with three stored rows per coefficient
// cell.  The first cell contains two distinct deletion IDs, and the first
// deletion unit itself contains repeated stored rows.  The final row is a
// caller-owned out-of-scope observation used to certify exact e(sample).
set obs 28
generate long obsid = _n
generate byte scope = _n <= 27
generate byte cell = ceil(_n/3) if scope
generate byte within_cell = mod(_n-1,3)+1 if scope
generate byte worker_index = floor((cell-1)/3)+1 if scope
generate byte firm_index = mod(cell-1,3)+1 if scope
generate long worker = 10*worker_index if scope
generate long firm = 100*firm_index if scope
generate long match = 1000+cell if scope
replace match = 101 in 1/2
replace match = 102 in 3
generate long frequency = 1+mod(obsid,3) if scope
generate double target = frequency*(1+mod(obsid,3)/4) if scope
generate double y = 2+.7*worker_index-.3*firm_index +               ///
    .11*within_cell+.03*worker_index*firm_index +                  ///
    .007*obsid^2 if scope
generate double control = (within_cell-2)*(1+.1*worker_index) +    ///
    .02*firm_index*within_cell if scope
generate long atom_key = obsid
generate byte expected_sample = scope

replace worker = 999 in 28
replace firm = 9999 in 28
replace match = 99999 in 28
replace frequency = 1 in 28
replace target = 1 in 28
replace y = -123 in 28
replace control = 17 in 28
replace atom_key = 999999 in 28

quietly count if scope & cell==1 & match==101
assert r(N) == 2
quietly levelsof match if scope & cell==1, local(first_cell_matches)
local first_cell_match_count : word count `first_cell_matches'
assert `first_cell_match_count' == 2

label data "KSS compressed command lifecycle fixture"
label variable y "Outcome with caller metadata"
label variable worker "Worker identifier"
label define worker_label 10 "worker one" 20 "worker two"        ///
    30 "worker three" 999 "out of scope"
label values worker worker_label
format y %13.6f
char _dta[kss_scale_command] "caller dataset characteristic"
char y[kss_scale_command] "caller variable characteristic"
sort scope firm obsid

// Give the caller a real filename and then retain an unsaved data change.
// The compressed command must restore the filename, dirty flag, ordering,
// data, labels, formats, characteristics, and estimation-sample semantics.
tempfile caller_source
quietly save `"`caller_source'"', replace
quietly replace y = y+.125 in 1
assert c(changed) == 1
local caller_filename `"`c(filename)'"'
local caller_filedate `"`c(filedate)'"'
local caller_sortedby : sortedby
local caller_data_label : data label
local caller_y_label : variable label y
local caller_y_format : format y
local caller_worker_vallabel : value label worker
local caller_dta_char : char _dta[kss_scale_command]
local caller_y_char : char y[kss_scale_command]
quietly _datasignature
local caller_signature `"`r(datasignature)'"'
quietly _datasignature expected_sample, nonames
local caller_sample_signature `"`r(datasignature)'"'

local probes 40
local solver_tolerance 1e-10
tempname baseline_results baseline_plugin baseline_correction baseline_kss
tempname alternate_results relabeled_results rhs_baseline
tempname baseline_relabel_plugin relabeled_plugin
tempname generic_results generic_partition_results
tempname p40_timer p200_timer

// Seed and advance the latent mt64s streams used by the estimator before
// returning to a non-mt64s caller.  The public command must restore these
// latent states, the selected stream, the active algorithm/state, and the
// sort jumbler on success and on every typed failure.
set rng mt64s
set rngstream 1
set seed 1101
mata: runiform(7,1)
set rngstream 2
set seed 2202
mata: runiform(11,1)
set rngstream 3
set seed 3303
mata: runiform(13,1)
set rng kiss32
set seed 20260816
local caller_rng `"`c(rng)'"'
local caller_rngstream = c(rngstream)
local caller_rngstate `"`c(rngstate)'"'
mata: KSS_TEST_RNG_SUCCESS_BEFORE = vckss_rng__capture_full()
mata: assert(KSS_TEST_RNG_SUCCESS_BEFORE.status == "OK")

quietly timer clear 70
quietly timer on 70
quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)         ///
    stayers(movers)                                                 ///
    probeorder(atom_key) algorithm(jla) engine(compressed)         ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(`probes') batch(7) seed(8675309)                        ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
quietly timer off 70
quietly timer list 70
scalar `p40_timer' = r(t70)
mata: assert(vckss_scale_runtime__status() == "EMPTY")

// The forced compressed route is explicitly experimental and preserves the
// canonical distinction between nine coefficient cells and ten deletion
// units.  The target construction has several exact per-copy strata.
assert "`e(status)'" == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
assert "`e(scale_status)'" == "EXPERIMENTAL_SCALE_ENGINE"
assert "`e(engine_requested)'" == "compressed"
assert "`e(engine_selected)'" == "compressed"
assert "`e(fastpath_status)'" == "ELIGIBLE"
assert "`e(performance_profile_api)'" == "PREP-RHS-PERF-V1"
tempname prep_profile prep_boundary_profile prep_boundary_counts
tempname rhs_profile work_counters fe_buffer_profile
matrix `prep_profile' = e(prep_profile)
matrix `prep_boundary_profile' = e(prep_boundary_profile)
matrix `prep_boundary_counts' = e(prep_boundary_counts)
matrix `rhs_profile' = e(rhs_profile)
matrix `work_counters' = e(work_counters)
matrix `fe_buffer_profile' = e(fe_buffer_profile)
assert rowsof(`prep_profile') == 1 & colsof(`prep_profile') == 7
assert "`e(prep_boundary_profile_schema)'" == "PREP-BND-PERF-V1"
assert "`e(prep_boundary_counts_schema)'" == "PREP-BND-COUNTS-V1"
assert rowsof(`prep_boundary_profile') == 1 &                    ///
    colsof(`prep_boundary_profile') == 11
assert rowsof(`prep_boundary_counts') == 1 &                     ///
    colsof(`prep_boundary_counts') == 11
assert rowsof(`rhs_profile') == 1 & colsof(`rhs_profile') == 8
assert rowsof(`work_counters') == 1 & colsof(`work_counters') == 12
assert "`e(fe_buffer_profile_schema)'" == "FE-BUF-PERF-V1"
assert rowsof(`fe_buffer_profile') == 1 & colsof(`fe_buffer_profile') == 10
assert `fe_buffer_profile'[1,1] == 1
assert `fe_buffer_profile'[1,2] > 0
assert `fe_buffer_profile'[1,3] > 0
assert `fe_buffer_profile'[1,3]+`fe_buffer_profile'[1,4] == ///
    e(solver_schur_batches)
assert `fe_buffer_profile'[1,5] > 0
assert `fe_buffer_profile'[1,5]+`fe_buffer_profile'[1,6] == ///
    e(solver_schur_actions)
assert `fe_buffer_profile'[1,7] == `fe_buffer_profile'[1,4]
assert `fe_buffer_profile'[1,8] == 2*7
assert `fe_buffer_profile'[1,9] == 8*(2*7)*(9+3+3)
assert `fe_buffer_profile'[1,10] > 0
assert `work_counters'[1,1] == 2
assert `work_counters'[1,3] == 0
assert `work_counters'[1,4] > 0
assert `work_counters'[1,5] == `work_counters'[1,4]
assert `work_counters'[1,11] == ceil(`probes'/7)
assert `work_counters'[1,12] == ceil(`probes'/7)
assert `prep_profile'[1,7] >= 0
mata: assert(min(st_matrix("`prep_boundary_profile'")) >= 0)
assert `prep_boundary_counts'[1,1] == 2
assert `prep_boundary_counts'[1,2] == 1
assert `prep_boundary_counts'[1,3] == 0
assert `prep_boundary_counts'[1,4] == 0
assert `prep_boundary_counts'[1,5] == 1
assert `prep_boundary_counts'[1,6] == 4
assert `prep_boundary_counts'[1,7] == e(N_complete)
assert `prep_boundary_counts'[1,8] == 2
assert `prep_boundary_counts'[1,9] == e(N_retained)
assert `prep_boundary_counts'[1,10] == 7
assert `prep_boundary_counts'[1,11] == e(N_retained)
mata: assert(min(st_matrix("`rhs_profile'")) >= 0)
assert "`e(life_method)'" == "PRESERVE_DISK"
assert e(life_preserve_forced_disk) == 1
assert e(life_sample_restored) == 1
assert e(coefficient_cells) == 9
assert e(deletion_units) == 10
assert e(deletion_units) > e(coefficient_cells)
assert e(target_strata) == 27
assert e(N_retained) == 27
assert e(N_physical) == 54
assert e(probes) == `probes'
assert e(leverage_batch) == 7
assert e(target_batch) == 7
assert "`e(rng_contract)'" != ""
assert "`e(rng_contract)'" != "NOT_APPLICABLE"
assert "`e(rng_implementation)'" == "per_domain_stream_cursor"
assert "`e(rng_leverage_domain)'" == "leverage"
assert "`e(rng_target_domain)'" == "target"
assert e(rng_leverage_probe_first) == 1
assert e(rng_leverage_probe_last) == `probes'
assert e(rng_target_probe_first) == 1
assert e(rng_target_probe_last) == `probes'
mata:
KSS_TEST_RNG_SUCCESS_AFTER = vckss_rng__capture_full()
assert(KSS_TEST_RNG_SUCCESS_AFTER.status == "OK")
assert(KSS_TEST_RNG_SUCCESS_AFTER.active.algorithm ==
    KSS_TEST_RNG_SUCCESS_BEFORE.active.algorithm)
assert(KSS_TEST_RNG_SUCCESS_AFTER.active.stream ==
    KSS_TEST_RNG_SUCCESS_BEFORE.active.stream)
assert(KSS_TEST_RNG_SUCCESS_AFTER.active.state ==
    KSS_TEST_RNG_SUCCESS_BEFORE.active.state)
assert(KSS_TEST_RNG_SUCCESS_AFTER.sort_state ==
    KSS_TEST_RNG_SUCCESS_BEFORE.sort_state)
assert(KSS_TEST_RNG_SUCCESS_AFTER.mt64s_stream1_state ==
    KSS_TEST_RNG_SUCCESS_BEFORE.mt64s_stream1_state)
assert(KSS_TEST_RNG_SUCCESS_AFTER.mt64s_stream2_state ==
    KSS_TEST_RNG_SUCCESS_BEFORE.mt64s_stream2_state)
assert(KSS_TEST_RNG_SUCCESS_AFTER.mt64s_selected_stream_state ==
    KSS_TEST_RNG_SUCCESS_BEFORE.mt64s_selected_stream_state)
end

// Every fit, leverage, and target RHS has a complete original-system
// residual certificate.  Stage 1 is reserved for nuisance preparation; this
// no-control specialization has one stage-2 fit RHS, P stage-4 leverage
// RHSs, and 2P stage-5 target RHSs.
matrix `rhs_baseline' = e(solver_rhs_diagnostics)
assert rowsof(`rhs_baseline') == 1+3*`probes'
assert colsof(`rhs_baseline') == 6
mata:
rhs = st_matrix(st_local("rhs_baseline"))
gate = st_numscalar("e(residual_acceptance_tolerance)")
assert(sum(rhs[.,1]:==2) == 1)
assert(sum(rhs[.,1]:==4) == strtoreal(st_local("probes")))
assert(sum(rhs[.,1]:==5) == 2*strtoreal(st_local("probes")))
assert(uniqrows(sort(select(rhs[.,3],rhs[.,1]:==4),1)) ==
    (1::strtoreal(st_local("probes"))))
assert(uniqrows(sort(select(rhs[.,3],rhs[.,1]:==5),1)) ==
    (1::(2*strtoreal(st_local("probes")))))
assert(min(rhs[.,6]) == 1)
assert(max(rhs[.,5]) <= gate)
assert(abs(max(rhs[.,5])-st_numscalar("e(complete_residual_max)")) <=
    16*epsilon(max((1,max(rhs[.,5])))))
end
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)
assert e(correction_reciprocal_residual) <=                       ///
    max(1e-10,e(residual_acceptance_tolerance))
assert e(target_identity_residual) <= 1e-12
assert e(rng_seconds) < .
assert e(rng_seconds) >= 0
assert e(rng_seconds) <= e(leverage_seconds)+e(target_seconds)
assert "`e(residual_normalization)'" == "l2_rhs_or_absolute_zero_rhs"
assert "`e(quotient_convention)'" == "full_firm_zero_sum"
assert "`e(grounding_convention)'" ==                            ///
    "last_firm_zero_after_quotient_with_grounded_equation_checked"

// Target accounting is certified both componentwise and in the command's
// returned matrices: total = worker + firm + 2*covariance, and corrected is
// exactly plugin minus the coefficient-one finite correction up to the
// registered regrouping tolerance.
forvalues result_row = 1/3 {
    assert abs(el(e(results),`result_row',4) -                    ///
        (el(e(results),`result_row',1)+                           ///
         el(e(results),`result_row',2)+                           ///
         2*el(e(results),`result_row',3))) <= 1e-11
}
matrix `baseline_results' = e(results)
matrix `baseline_plugin' = e(plugin)
matrix `baseline_correction' = e(correction)
matrix `baseline_kss' = e(kss)
matrix __kss_scale_identity =                                    ///
    `baseline_plugin'-`baseline_correction'-`baseline_kss'
mata: assert(max(abs(st_matrix("__kss_scale_identity"))) <= 1e-12)
matrix drop __kss_scale_identity

// Exact caller RNG restoration covers the algorithm, stream selection, and
// complete current-generator state.  Probe atoms, not estimator reductions,
// are the route-invariant object.
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'

// e(sample) must reproduce the independently selected caller scope.  The
// command may retain Stata's internal e(sample) marker while estimates are
// active, but it must not alter any caller variable or its order/metadata.
quietly count if e(sample) != expected_sample
assert r(N) == 0
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
quietly _datasignature expected_sample, nonames
assert `"`r(datasignature)'"' == `"`caller_sample_signature'"'
assert `"`c(filename)'"' == `"`caller_filename'"'
assert `"`c(filedate)'"' == `"`caller_filedate'"'
assert c(changed) == 1
local restored_sortedby : sortedby
local restored_data_label : data label
local restored_y_label : variable label y
local restored_y_format : format y
local restored_worker_vallabel : value label worker
local restored_dta_char : char _dta[kss_scale_command]
local restored_y_char : char y[kss_scale_command]
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
assert `"`restored_data_label'"' == `"`caller_data_label'"'
assert `"`restored_y_label'"' == `"`caller_y_label'"'
assert `"`restored_y_format'"' == `"`caller_y_format'"'
assert `"`restored_worker_vallabel'"' == `"`caller_worker_vallabel'"'
assert `"`restored_dta_char'"' == `"`caller_dta_char'"'
assert `"`restored_y_char'"' == `"`caller_y_char'"'

// Changing only matrix-RHS partitions cannot change the logical probe atoms.
// The command result may differ in floating-point reduction order, so compare
// at a numerical tolerance instead of requiring byte equality.
quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)         ///
    stayers(movers)                                                 ///
    probeorder(atom_key) algorithm(jla) engine(compressed)         ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(`probes') batch(13) seed(8675309)                       ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
matrix `alternate_results' = e(results)
assert mreldif(`baseline_results',`alternate_results') <= 1e-11
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'

// Relabel every graph dimension nonmonotonically and permute the stored rows.
// Arbitrary ID relabeling may select another valid draw under the streamlined
// observed-ID contract. Deterministic targets and numerical gates remain.
replace worker = cond(worker_index==1,900,                       ///
    cond(worker_index==2,-20,77)) if scope
replace firm = cond(firm_index==1,501,                            ///
    cond(firm_index==2,-31,88)) if scope
replace match = 700000-17*match if scope
generate long shuffle_order = mod(7919*atom_key,104729)
gsort -scope -shuffle_order
drop shuffle_order
quietly _datasignature
local relabeled_signature `"`r(datasignature)'"'
local relabeled_sortedby : sortedby
quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)         ///
    stayers(movers)                                                 ///
    probeorder(atom_key) algorithm(jla) engine(compressed)         ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(`probes') batch(9) seed(8675309)                        ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
matrix `relabeled_results' = e(results)
matrix `baseline_relabel_plugin' = `baseline_results'[1,1..4]
matrix `relabeled_plugin' = `relabeled_results'[1,1..4]
assert mreldif(`baseline_relabel_plugin',`relabeled_plugin') <= 2e-11
forvalues result_row = 1/3 {
    assert abs(`relabeled_results'[`result_row',4] -             ///
        `relabeled_results'[`result_row',1] -                    ///
        `relabeled_results'[`result_row',2] -                    ///
        2*`relabeled_results'[`result_row',3]) <= 2e-10
}
assert e(coefficient_cells) == 9
assert e(deletion_units) == 10
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
quietly count if e(sample) != expected_sample
assert r(N) == 0
quietly _datasignature
assert `"`r(datasignature)'"' == `"`relabeled_signature'"'
local relabeled_restored_sortedby : sortedby
assert `"`relabeled_restored_sortedby'"' == `"`relabeled_sortedby'"'
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'

// Return to the baseline labels and caller order without relying on a nested
// preserve: the command itself exercises native disk-backed preserve.
replace worker = 10*worker_index if scope
replace firm = 100*firm_index if scope
replace match = 1000+cell if scope
replace match = 101 if obsid==1 | obsid==2
replace match = 102 if obsid==3
sort scope firm obsid
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// On an eligible no-control match design, a forced generic route consumes
// the same canonical semantic atoms as the compressed route.  The two
// numerical engines need only agree within the registered tolerance; a
// second generic partition verifies that matrix-RHS batching does not alter
// those atoms.
quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)         ///
    stayers(movers)                                                 ///
    probeorder(atom_key) algorithm(jla) engine(generic)            ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(`probes') batch(5) seed(8675309)                        ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
matrix `generic_results' = e(results)
assert "`e(engine_selected)'" == "generic"
assert "`e(fastpath_status)'" == "FASTPATH_BYPASSED"
matrix `prep_boundary_counts' = e(prep_boundary_counts)
assert `prep_boundary_counts'[1,4] == 1
assert `prep_boundary_counts'[1,5] == 2
assert `prep_boundary_counts'[1,10] == 0
assert `prep_boundary_counts'[1,11] == 0
assert "`e(rng_implementation)'" ==                              ///
    "per_domain_stream_semantic_atoms"
assert mreldif(`baseline_results',`generic_results') <= 2e-9
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) <= 1e-12
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'

quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)         ///
    stayers(movers)                                                 ///
    probeorder(atom_key) algorithm(jla) engine(generic)            ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(`probes') batch(13) seed(8675309)                       ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
matrix `generic_partition_results' = e(results)
assert mreldif(`generic_results',`generic_partition_results') <= 1e-11
assert mreldif(`baseline_results',`generic_partition_results') <= 2e-9
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'

// A focused P200 run remains cheap on this exact fixture and exercises the
// qualification probe count.  compressed mode uses direct exact binomial
// sums and therefore does not consume the generic physical-copy limit.
quietly timer clear 71
quietly timer on 71
quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target)         ///
    stayers(movers)                                                 ///
    probeorder(atom_key) algorithm(jla) engine(compressed)         ///
    preconditioner(diagonal) memory_gib(4) wallseconds(3600)       ///
    probes(200) batch(17) seed(8675309) physical_limit(1)          ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
quietly timer off 71
quietly timer list 71
scalar `p200_timer' = r(t71)
assert "`e(engine_selected)'" == "compressed"
assert e(probes) == 200
assert rowsof(e(solver_rhs_diagnostics)) == 601
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) <= 1e-12
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
mata: assert(vckss_scale_runtime__status() == "EMPTY")

// engine(auto) must select the general engine for controls and observation
// deletion.  These are successful typed fallbacks, not silent fast-path
// reinterpretations.
quietly fevc y control [fw=frequency] if scope,                  ///
    worker(worker) firm(firm) deletion(match) deletionid(match)    ///
    stayers(movers)                                                 ///
    targetweight(target) probeorder(atom_key) algorithm(jla)       ///
    engine(auto) preconditioner(diagonal) memory_gib(4)            ///
    wallseconds(3600) probes(200) batch(11) seed(8675309)          ///
    tolerance(`solver_tolerance') backend(mata) rng(stata) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "`e(scale_status)'" == "GENERAL_ENGINE"
assert "`e(engine_selected)'" == "generic"
assert "`e(fastpath_status)'" == "FASTPATH_CONTROLS"
assert "`e(life_method)'" == "RAW_RESIDENT"
matrix `prep_boundary_counts' = e(prep_boundary_counts)
assert `prep_boundary_counts'[1,4] == 1
assert `prep_boundary_counts'[1,5] == 2
assert `prep_boundary_counts'[1,10] == 0
assert `prep_boundary_counts'[1,11] == 0
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
mata: assert(vckss_scale_runtime__status() == "EMPTY")

quietly fevc y [fw=frequency] if scope, worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) probeorder(atom_key) ///
    stayers(movers)                                                 ///
    algorithm(jla) engine(auto) preconditioner(diagonal)            ///
    memory_gib(4) wallseconds(3600) probes(200) batch(11)           ///
    seed(8675309) tolerance(`solver_tolerance') backend(mata)       ///
    rng(stata) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "`e(scale_status)'" == "GENERAL_ENGINE"
assert "`e(engine_selected)'" == "generic"
assert "`e(fastpath_status)'" == "FASTPATH_OBSERVATION_DELETION"
assert "`e(life_method)'" == "RAW_RESIDENT"
matrix `prep_boundary_counts' = e(prep_boundary_counts)
assert `prep_boundary_counts'[1,2] == 0
assert `prep_boundary_counts'[1,4] == 1
assert `prep_boundary_counts'[1,5] == 2
assert `prep_boundary_counts'[1,10] == 0
assert `prep_boundary_counts'[1,11] == 0
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
mata: assert(vckss_scale_runtime__status() == "EMPTY")

// Forced compressed calls fail closed with the exact typed eligibility
// reason.  A caller cannot accidentally reinterpret either controls or
// observation deletion through the specialized algebra.
mata: KSS_TEST_RNG_FAILURE_BEFORE = vckss_rng__capture_full()
mata: assert(KSS_TEST_RNG_FAILURE_BEFORE.status == "OK")
capture noisily fevc y control [fw=frequency] if scope,          ///
    worker(worker) firm(firm) deletion(match) deletionid(match)    ///
    targetweight(target) probeorder(atom_key) algorithm(jla)       ///
    engine(compressed) probes(40) seed(8675309) backend(mata)      ///
    stayers(movers)                                                ///
    rng(stata) nodisplay
assert _rc == 498
assert "`e(status)'" == "WITHHELD"
assert "`e(withholding_status)'" == "FASTPATH_CONTROLS"
assert "`e(fastpath_status)'" == "FASTPATH_CONTROLS"
assert "`e(engine_selected)'" == "WITHHELD"
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
mata: assert(vckss_scale_runtime__status() == "EMPTY")
mata:
KSS_TEST_RNG_FAILURE_AFTER = vckss_rng__capture_full()
assert(KSS_TEST_RNG_FAILURE_AFTER.status == "OK")
assert(KSS_TEST_RNG_FAILURE_AFTER.active.algorithm ==
    KSS_TEST_RNG_FAILURE_BEFORE.active.algorithm)
assert(KSS_TEST_RNG_FAILURE_AFTER.active.stream ==
    KSS_TEST_RNG_FAILURE_BEFORE.active.stream)
assert(KSS_TEST_RNG_FAILURE_AFTER.active.state ==
    KSS_TEST_RNG_FAILURE_BEFORE.active.state)
assert(KSS_TEST_RNG_FAILURE_AFTER.sort_state ==
    KSS_TEST_RNG_FAILURE_BEFORE.sort_state)
assert(KSS_TEST_RNG_FAILURE_AFTER.mt64s_stream1_state ==
    KSS_TEST_RNG_FAILURE_BEFORE.mt64s_stream1_state)
assert(KSS_TEST_RNG_FAILURE_AFTER.mt64s_stream2_state ==
    KSS_TEST_RNG_FAILURE_BEFORE.mt64s_stream2_state)
assert(KSS_TEST_RNG_FAILURE_AFTER.mt64s_selected_stream_state ==
    KSS_TEST_RNG_FAILURE_BEFORE.mt64s_selected_stream_state)
end

capture noisily fevc y [fw=frequency] if scope,                 ///
    worker(worker) firm(firm) deletion(observation)                ///
    targetweight(target) probeorder(atom_key) algorithm(jla)       ///
    engine(compressed) probes(40) seed(8675309) backend(mata)      ///
    stayers(movers)                                                ///
    rng(stata) nodisplay
assert _rc == 498
assert "`e(status)'" == "WITHHELD"
assert "`e(withholding_status)'" == "FASTPATH_OBSERVATION_DELETION"
assert "`e(fastpath_status)'" == "FASTPATH_OBSERVATION_DELETION"
assert "`e(engine_selected)'" == "WITHHELD"
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
mata: assert(vckss_scale_runtime__status() == "EMPTY")

// The same low physical-copy ceiling that is irrelevant to the compressed
// binomial path must reject the literal-copy generic engine before probes.
capture noisily fevc y [fw=frequency] if scope,                 ///
    worker(worker) firm(firm) deletion(match) deletionid(match)    ///
    targetweight(target) probeorder(atom_key) algorithm(jla)       ///
    engine(generic) preconditioner(diagonal) memory_gib(4)         ///
    wallseconds(3600) probes(40) batch(7) seed(8675309)            ///
    physical_limit(1) tolerance(`solver_tolerance') backend(mata)  ///
    rng(stata) nodisplay
assert _rc == 498
assert "`e(status)'" == "WITHHELD"
assert "`e(withholding_status)'" == "PHYSICAL_COPY_LIMIT"
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_rngstream'
assert `"`c(rngstate)'"' == `"`caller_rngstate'"'
mata: assert(vckss_scale_runtime__status() == "EMPTY")

di as txt "compressed command fixture timings (seconds): P40="    ///
    as result %8.3f scalar(`p40_timer')                            ///
    as txt " P200=" as result %8.3f scalar(`p200_timer')

quietly cd `"`oldpwd'"'
di as result "PASS test_scale_command.do"
exit 0
