version 18.0
clear all
set more off
set varabbrev off

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'

adopath ++ `"`pkgroot'"'
quietly run `"`pkgroot'/fevc.ado"'

set obs 300
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker + floor(time/2),25)
generate double control = time - 2.5
generate double shock_scale = .15 + .012*mod(worker,9) + .02*(time==5)
generate double disturbance = shock_scale * ///
    (sin((_n*17)/11) + .7*cos((_n*7)/13))
generate double outcome = 2 + .06*worker - .11*firm + .25*control + disturbance
set seed 14021985
generate double outcome_leverage = 2 + .06*worker - .11*firm + .25*control + ///
    shock_scale*rnormal()
generate double outcome_null = 0
generate byte copies = 2

local common_options worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1)     ///
    preconditioner(diagonal) probes(160) tolerance(1e-11)            ///
    inference(highrank) inferencemodel(structured_common)            ///
    inferencesimulations(100) inferenceseed(24681357) nodisplay

// Adding the inference attachment must not alter any component point result.
quietly fevc outcome control, worker(worker) firm(firm)              ///
    deletion(observation) algorithm(jla) engine(generic)             ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)            ///
    probes(160) batch(8) tolerance(1e-11) nodisplay
matrix baseline_results = e(results)
matrix baseline_b = e(b)

quietly fevc outcome control, `common_options' batch(8)
matrix q0_b = e(b)
matrix q0_V = e(V)
matrix q0_Vp = e(V_primitive)
matrix q0_spectrum = e(component_spectrum)
matrix q0_summary = e(structured_variance_summary)
matrix q0_folds = e(structured_variance_folds)
matrix q0_cv = e(structured_variance_cv)
matrix q0_results = e(results)

assert mreldif(baseline_results,q0_results) == 0
assert mreldif(baseline_b,q0_b) == 0
assert `"`e(status)'"' == "FEVC_STRUCTURED_Q0_INFERENCE"
assert `"`e(inference_model)'"' == "structured_common"
assert `"`e(result_family)'"' == "generic"
assert `"`e(inference_support_status)'"' == "supported_explicit"
assert strpos(`"`e(inference_capability)'"',"observation deletion") > 0
assert `"`e(inference_population)'"' == "movers"
assert `"`e(inference_deletion_requested)'"' == "observation"
assert `"`e(inference_deletion_selected)'"' == "observation"
assert `"`e(inference_population_requested)'"' == "movers"
assert `"`e(inference_population_selected)'"' == "movers"
assert `"`e(inference_model_requested)'"' == "structured_common"
assert `"`e(inference_model_selected)'"' == "structured_common"
assert `"`e(inference_backend_requested)'"' == "rust"
assert `"`e(inference_backend_selected)'"' == "rust"
assert `"`e(inference_solver_requested)'"' == "diagonal"
assert `"`e(inference_solver_selected)'"' == "DIAGONAL"
assert `"`e(inference_family_requested)'"' == "generic"
assert `"`e(inference_family_selected)'"' == "generic"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "generic"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "DIAGONAL"
assert `"`e(deletion)'"' == "observation"
assert `"`e(stayers)'"' == "movers"
assert `"`e(nuisance)'"' == "joint"
assert `"`e(inference_reference_requested)'"' == "q0"
assert `"`e(inference_reference_selected)'"' == "q0 diffuse"
assert `"`e(inference_reference)'"' == ///
    "q=0 Gaussian approximation with reported spectrum"
assert strpos(`"`e(inference_kss_scope)'"',"not unrestricted") > 0
assert strpos(`"`e(inference_q_condition)'"',"diffuse") > 0
assert strpos(`"`e(inference_execution_scope)'"',"does not establish") > 0
assert strpos(`"`e(inference_variance_warning)'"',"invalidate SEs") > 0
assert strpos(`"`e(inference_variance_warning)'"',"point estimates") > 0
assert strpos(`"`e(inference_reference_guarantee)'"',"strong identification") > 0
assert rowsof(q0_V) == 4 & colsof(q0_V) == 4
assert rowsof(q0_Vp) == 3 & colsof(q0_Vp) == 3
assert rowsof(q0_spectrum) == 4 & colsof(q0_spectrum) == 15
assert q0_spectrum[1,"max_influence_share"] > 0
assert q0_spectrum[1,"max_influence_share"] <= 1
assert rowsof(q0_summary) == 2 & colsof(q0_summary) == 12
assert rowsof(q0_folds) == 10 & colsof(q0_folds) == 15
assert rowsof(q0_cv) == 70 & colsof(q0_cv) == 7
assert e(inference_simulations) == 100
assert e(inference_spectrum_probes) == 128
assert e(inference_spectrum_iterations) == 128
assert e(inference_solver_columns) == 3+100+5*128+16*128+18
assert e(inference_solver_max_complete) <= e(inference_solver_tolerance)
assert q0_spectrum[1,7] >= 0 & q0_spectrum[1,7] <= 1
assert q0_spectrum[1,8] >= 0

matrix map = (1,0,0 \ 0,1,0 \ 0,0,1 \ 1,1,2)
matrix q0_mapped = map*q0_Vp*map'
assert mreldif(q0_V,q0_mapped) < 1e-13
assert abs(det(q0_V)) < 1e-10

// Counter-V1 and semantic folds make both point and inference results
// independent of the solver batch width.
quietly fevc outcome control, `common_options' batch(16)
assert mreldif(q0_results,e(results)) == 0
assert mreldif(q0_V,e(V)) == 0
assert mreldif(q0_spectrum,e(component_spectrum)) == 0
assert mreldif(q0_summary,e(structured_variance_summary)) == 0
assert mreldif(q0_folds,e(structured_variance_folds)) == 0
assert mreldif(q0_cv,e(structured_variance_cv)) == 0

// q=1 changes only the reference approximation. It exposes rather than
// suppresses leading- and remainder-spectrum concentration diagnostics.
quietly fevc outcome_leverage control, worker(worker) firm(firm)    ///
    deletion(observation) algorithm(jla) engine(generic)            ///
    backend(rust) rng(counter_v1) preconditioner(diagonal) batch(8) ///
    probes(160) tolerance(1e-11) inference(q1)                      ///
    inferencemodel(structured_leverage) inferencesimulations(100)   ///
    inferenceseed(97531)
assert `"`e(status)'"' == "FEVC_STRUCTURED_Q1_INFERENCE"
assert `"`e(inference_model)'"' == "structured_leverage"
assert `"`e(result_family)'"' == "generic"
assert `"`e(inference_support_status)'"' == "supported_explicit"
assert `"`e(inference_reference_requested)'"' == "q1"
assert `"`e(inference_reference_selected)'"' == "q1 one-mode"
assert strpos(`"`e(inference_q_condition)'"',"one leading mode") > 0
assert strpos(`"`e(inference_q_condition)'"',"diffuse") > 0
assert strpos(`"`e(inference_reference_guarantee)'"',"at-least-nominal") > 0
assert strpos(`"`e(inference_reference_guarantee)'"',"conservative") > 0
assert rowsof(e(q1_inference)) == 4 & colsof(e(q1_inference)) == 17
assert rowsof(e(component_q1_diagnostics)) == 4
assert colsof(e(component_q1_diagnostics)) == 16
assert e(component_q1_diagnostics)[1,"remainder_identity_error"] >= 0
assert e(component_q1_diagnostics)[1,"recenter_var_b1"] < .
assert colsof(e(component_inference_receipt)) == 21
assert e(component_inference_receipt)[1,"critical_simulations"] == 100000
assert e(component_inference_receipt)[1,"maximum_remainder_identity_error"] >= 0
assert e(component_inference_receipt)[1,"schema"] == 3
assert e(component_inference_receipt)[1,"model"] == 2
assert e(component_inference_receipt)[1,"reference"] == 1
assert e(component_augmentation_receipt)[1,"schema"] == 1
assert e(component_augmentation_receipt)[1,"model"] == 2
assert e(component_augmentation_receipt)[1,"reference"] == 1
assert e(inference_solver_columns) == 3+100+5*128+16*128+18+4
matrix q1_spectrum = e(component_spectrum)
forvalues target = 1/4 {
    assert q1_spectrum[`target',"leading_share"] >= 0
    assert q1_spectrum[`target',"leading_share"] <= 1
    assert q1_spectrum[`target',"remainder_leading_share"] >= 0
    assert q1_spectrum[`target',"remainder_leading_share"] <= 1
    assert e(component_q1_diagnostics)[`target',"remainder_influence_share"] >= 0
    assert e(component_q1_diagnostics)[`target',"remainder_influence_share"] <= 1
}
matrix q1_results_before_estat = e(results)
estat diagnostics
assert mreldif(q1_results_before_estat,e(results)) == 0

// Omitting the structured model preserves the independent exact Mata
// comparator and cannot route automatically to q=0 or q=1 Rust inference.
preserve
keep in 1/24
replace worker = floor((_n-1)/4)
replace time = mod(_n-1,4)
replace control = time-1.5
generate byte exact_c2 = time==2
generate double exact_noise = .
generate double exact_target = 1+time/10
local exact_firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local exact_noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `exact_firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `exact_noises'
    quietly replace exact_noise = `value' in `row'
}
replace outcome = 1.5+.3*worker-.2*firm+.4*control-.15*exact_c2+exact_noise
quietly fevc outcome control exact_c2, worker(worker) firm(firm)  ///
    deletion(observation) algorithm(exact) backend(mata) rng(stata) ///
    inference(highrank) inferencesimulations(100) inferenceseed(86420) ///
    inferencebins(16) targetweight(exact_target) nodisplay
assert `"`e(status)'"' == "KSS_HIGHRANK_INFERENCE"
assert `"`e(backend_requested)'"' == "mata"
assert `"`e(backend_selected)'"' == "mata"
assert `"`e(inference_model)'"' == "target_lowess"
assert e(inference_model_option_supplied) == 0
capture matrix list e(component_spectrum)
assert _rc != 0
restore

// Every unsupported request fails closed before returning a substitute.
capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(observation) inference(highrank)                      ///
    inferencemodel(structured_common) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "STRUCTURED_INFERENCE_TUPLE_REQUIRED"

capture noisily fevc outcome control [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) engine(generic) backend(rust)    ///
    rng(counter_v1) preconditioner(diagonal) inference(highrank)          ///
    inferencemodel(structured_common) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "STRUCTURED_FREQUENCY_UNSUPPORTED"

capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(match) algorithm(jla) engine(generic) backend(rust)    ///
    rng(counter_v1) preconditioner(diagonal) inference(highrank)     ///
    inferencemodel(structured_common) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "STRUCTURED_INFERENCE_TUPLE_REQUIRED"

capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(observation) stayers(both) algorithm(jla) engine(generic) ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)          ///
    inference(highrank) inferencemodel(structured_common) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "STRUCTURED_INFERENCE_TUPLE_REQUIRED"

capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(observation) nuisance(fixedoffset) algorithm(jla) engine(generic) ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)          ///
    inference(highrank) inferencemodel(structured_common) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "STRUCTURED_INFERENCE_TUPLE_REQUIRED"

// A null outcome must be withheld atomically rather than regularized or
// replaced by another estimator/reference family.
capture noisily fevc outcome_null control, worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) engine(generic) backend(rust) ///
    rng(counter_v1) preconditioner(diagonal) inference(q1)          ///
    inferencemodel(structured_common) inferencesimulations(100)     ///
    inferenceseed(13579) nodisplay
assert _rc == 498
assert `"`e(status)'"' == "WITHHELD"
assert strtrim(`"`e(withholding_status)'"') != ""
capture matrix list e(component_spectrum)
assert _rc != 0

capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(observation) inference(highrank)                      ///
    inferencemodel(unrestricted_kss) nodisplay
assert _rc == 198
assert `"`e(withholding_status)'"' == "INVALID_INFERENCE_MODEL"

quietly fevc_rust snapshot
assert r(state) == 0

di as result "PASS test_rust_component_inference.do"
exit 0
