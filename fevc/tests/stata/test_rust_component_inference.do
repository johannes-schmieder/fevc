// Native observation-deletion component inference: repeat-measurement
// standard errors on the explicit generic Counter-V1 tuple, their batch
// invariance, the leading directions of inference(q1), the exact Mata route
// on a small design, and the requests that stop before native work. The
// removed variance models and their options stop with typed statuses.
version 15.0
clear all
set more off
set varabbrev off

args pkgroot plugindir
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'

adopath ++ `"`pkgroot'"'
if `"`plugindir'"'!="" adopath ++ `"`plugindir'"'
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

// Every call names its batch(); the batch-invariance checks below compare
// explicit widths.
local common_options worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1)     ///
    preconditioner(diagonal) probes(160) tolerance(1e-11)            ///
    centering(none) inference(highrank) inferenceseed(24681357) nodisplay

// The estimator is unchanged, but design-only probe ordering may change the
// finite-probe realization relative to a point-only legacy ordering.
quietly fevc outcome control, worker(worker) firm(firm)              ///
    deletion(observation) algorithm(jla) engine(generic)             ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)            ///
    probes(160) batch(8) tolerance(1e-11) centering(none) inference(none) nodisplay
matrix baseline_results = e(results)
matrix baseline_b = e(b)

quietly fevc outcome control, `common_options' batch(8)
matrix q0_b = e(b)
matrix q0_V = e(V)
matrix q0_Vp = e(V_primitive)
matrix q0_spectrum = e(component_spectrum)
matrix q0_ci = e(component_inference)
matrix q0_repeat = e(inference_repeat)
matrix q0_results = e(results)

forvalues target = 1/4 {
    assert reldif(baseline_results[1,`target'],q0_results[1,`target']) < 1e-9
    assert abs(baseline_b[1,`target']-q0_b[1,`target']) <= ///
        6*sqrt(baseline_results[4,`target']^2+q0_results[4,`target']^2)+1e-9
}
assert `"`e(status)'"' == "FEVC_REPEAT_MEASUREMENT_SE"
assert `"`e(inference)'"' == "highrank"
assert `"`e(inference_model)'"' == "repeat_measurement"
assert `"`e(inference_variance_fit)'"' == "repeat_measurement"
assert `"`e(result_family)'"' == "generic"
assert `"`e(inference_support_status)'"' == "fitted_model_for_movers"
assert strpos(`"`e(inference_capability)'"',"repeat-measurement") > 0
assert strpos(`"`e(inference_capability)'"',"observation deletion") > 0
assert `"`e(inference_population)'"' == "both"
assert `"`e(inference_deletion_requested)'"' == "observation"
assert `"`e(inference_deletion_selected)'"' == "observation"
assert e(inference_independent_units)==300
// The native route keeps the control in the operator and adds the
// first-order term of its estimation.
assert e(inference_nuisance_omitted)==0
assert el(e(component_unit_receipt),1,colnumb(e(component_unit_receipt),"deletion"))==2
assert el(e(component_unit_receipt),1,colnumb(e(component_unit_receipt),"effective_matches"))==0
assert `"`e(inference_population_requested)'"' == "both"
assert `"`e(inference_population_selected)'"' == "both"
assert `"`e(inference_model_requested)'"' == "repeat_measurement"
assert `"`e(inference_model_selected)'"' == "repeat_measurement"
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
assert `"`e(stayers)'"' == "both"
assert `"`e(nuisance)'"' == "joint"
assert `"`e(inference_reference_requested)'"' == "q0"
assert `"`e(inference_reference_selected)'"' == "q0 normal"
assert strpos(`"`e(inference_reference)'"',"normal approximation") > 0
assert strpos(`"`e(inference_kss_scope)'"',"not the KSS") > 0
assert strpos(`"`e(inference_q_condition)'"',"effective rank") > 0
assert strpos(`"`e(inference_execution_scope)'"',"does not establish") > 0
assert strpos(`"`e(inference_variance_warning)'"',"fitted variance model for movers") > 0
assert strpos(`"`e(inference_reference_guarantee)'"',"no dominant direction") > 0
assert rowsof(q0_V) == 4 & colsof(q0_V) == 4
assert rowsof(q0_Vp) == 3 & colsof(q0_Vp) == 3
assert rowsof(q0_spectrum) == 4 & colsof(q0_spectrum) == 15
assert el(q0_spectrum,1,colnumb(q0_spectrum,"max_influence_share")) > 0
assert el(q0_spectrum,1,colnumb(q0_spectrum,"max_influence_share")) <= 1
// No Gram fit and no structured variance model.
local returned_matrices : e(matrices)
foreach structured in residual_moment_diagnostics structured_variance_summary ///
    structured_variance_folds structured_variance_cv {
    assert !strpos(" `returned_matrices' "," `structured' ")
}
assert "`e(inference_gram_method)'"=="not applicable"
// Every match has two observations: 150 repeat-measurement sets.
assert rowsof(q0_repeat)==1 & colsof(q0_repeat)==14
assert e(inference_repeat_sets)==150 & el(q0_repeat,1,colnumb(q0_repeat,"sets"))==150
assert colsof(e(component_inference_receipt))==78
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"schema"))==5
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"model"))==4
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"reference"))==0
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"gram_probes"))==0
// Four solves per pseudo-noise draw, three for the kernel image of the
// outcome, and under joint nuisance with a control two per point probe for
// the fixed-offset pass. inference(highrank) also finds the leading
// directions, as the other modes do: the search, four remainder influences
// and four remainder kernels per draw.
assert e(inference_draws)==8 & e(inference_simulations)==4*8
assert e(inference_spectrum_probes)==8 & e(inference_spectrum_iterations)==0
assert e(inference_offset_pass_solves)==2*160
assert e(inference_leading_steps)>0
assert e(inference_solver_columns)==4*8+3+2*160+e(inference_leading_steps)+4+4*8
assert e(inference_solver_max_complete) <= e(inference_solver_tolerance)
assert e(inference_joint_posted)==1
assert `"`e(inference_leading_request)'"' == "highrank"
assert e(inference_q1_components)==0
matrix q0_table = e(inference_intervals)
assert rowsof(q0_table)==4 & colsof(q0_table)==16
forvalues target = 1/4 {
    assert q0_ci[`target',2]>0 & reldif(q0_ci[`target',2],sqrt(q0_V[`target',`target']))<1e-12
    // Every component keeps the normal interval.
    assert el(q0_table,`target',colnumb(q0_table,"regime"))==0
    assert el(q0_table,`target',colnumb(q0_table,"lb"))==el(q0_table,`target',colnumb(q0_table,"normal_lb"))
}

matrix map = (1,0,0 \ 0,1,0 \ 0,0,1 \ 1,1,2)
matrix q0_mapped = map*q0_Vp*map'
assert mreldif(q0_V,q0_mapped) < 1e-13
assert abs(det(q0_V)) < 1e-10

// Counter-V1 and design-only ordering make both point and inference results
// independent of the solver batch width.
quietly fevc outcome control, `common_options' batch(16)
assert mreldif(q0_results,e(results)) == 0
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(q0_V,`compat_assert_V') == 0
matrix drop `compat_assert_V'
assert mreldif(q0_spectrum,e(component_spectrum)) == 0
assert mreldif(q0_repeat,e(inference_repeat)) == 0
// The model's name is a no-op alias of the one method.
quietly fevc outcome control, `common_options' batch(8) inferencemodel(REPEAT_MEASUREMENT)
assert mreldif(q0_results,e(results)) == 0
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(q0_V,`compat_assert_V') == 0
matrix drop `compat_assert_V'
assert mreldif(q0_ci,e(component_inference)) == 0

// inference(q1): the same high-rank pass plus the leading direction of every
// component, reported with the q = 1 interval wherever it exists.
quietly fevc outcome_leverage control, worker(worker) firm(firm)    ///
    deletion(observation) algorithm(jla) engine(generic)            ///
    backend(rust) rng(counter_v1) preconditioner(diagonal) batch(8) ///
    probes(160) tolerance(1e-11) centering(none) inference(q1)      ///
    inferenceseed(97531)
assert inlist(`"`e(status)'"',"FEVC_REPEAT_MEASUREMENT_Q1","FEVC_REPEAT_MEASUREMENT_SE")
assert `"`e(inference_model)'"' == "repeat_measurement"
assert `"`e(result_family)'"' == "generic"
assert `"`e(inference_leading_request)'"' == "q1"
assert strpos(`"`e(inference_regime_rule)'"',"inference(q1)") > 0
assert e(inference_leading_unresolved)==0
assert e(inference_leading_steps)>0
matrix q1_table = e(inference_intervals)
matrix q1_lead = e(inference_leading)
assert rowsof(q1_table)==4 & colsof(q1_table)==16
assert rowsof(q1_lead)==4 & colsof(q1_lead)==28
local q1_components = 0
forvalues target = 1/4 {
    assert inlist(el(q1_table,`target',colnumb(q1_table,"regime")),0,1)
    // Under inference(q1) a component that has the q = 1 interval reports it.
    assert (el(q1_table,`target',colnumb(q1_table,"regime"))==1)==(el(q1_table,`target',colnumb(q1_table,"q1_status"))==0)
    assert el(q1_table,`target',colnumb(q1_table,"lb")) < el(q1_table,`target',colnumb(q1_table,"estimate"))
    assert el(q1_table,`target',colnumb(q1_table,"estimate")) < el(q1_table,`target',colnumb(q1_table,"ub"))
    assert el(q1_lead,`target',colnumb(q1_lead,"converged"))==1
    assert el(q1_lead,`target',colnumb(q1_lead,"eigen_share"))>=0 & el(q1_lead,`target',colnumb(q1_lead,"eigen_share"))<=1
    local q1_components = `q1_components'+(el(q1_table,`target',colnumb(q1_table,"regime"))==1)
}
assert e(inference_q1_components)==`q1_components'
local returned_matrices : e(matrices)
assert (strpos(" `returned_matrices' "," V ")>0)==(`q1_components'==0)
assert e(inference_joint_posted)==(`q1_components'==0)
if `q1_components'>0 assert `"`e(status)'"' == "FEVC_REPEAT_MEASUREMENT_Q1"
assert colsof(e(component_inference_receipt)) == 78
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"schema")) == 5
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"model")) == 4
assert el(e(component_inference_receipt),1,colnumb(e(component_inference_receipt),"reference")) == 0
assert el(e(component_augmentation_receipt),1,colnumb(e(component_augmentation_receipt),"schema")) == 1
// The search, four remainder influences and four remainder kernels per
// draw come on top of the high-rank pass.
assert e(inference_solver_columns) == 4*8+3+2*160+e(inference_leading_steps)+4+4*8
matrix q1_spectrum = e(component_spectrum)
forvalues target = 1/4 {
    assert el(q1_spectrum,`target',colnumb(q1_spectrum,"leading_share")) >= 0
    assert el(q1_spectrum,`target',colnumb(q1_spectrum,"leading_share")) <= 1
}
matrix q1_results_before_estat = e(results)
matrix q1_table_before_estat = e(inference_intervals)
estat diagnostics
assert mreldif(q1_results_before_estat,e(results)) == 0
assert mreldif(q1_table_before_estat,e(inference_intervals)) == 0

// The exact Mata route on a small fixed design: the same model, computed
// with exact kernels, and the removed options stop before any work.
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
local exact_options worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) backend(mata) rng(stata) centering(none)        ///
    inferenceseed(86420) targetweight(exact_target) nodisplay
quietly fevc outcome control exact_c2, `exact_options' inference(highrank)
assert `"`e(status)'"' == "FEVC_REPEAT_MEASUREMENT_SE"
assert `"`e(backend_requested)'"' == "mata"
assert `"`e(backend_selected)'"' == "mata"
assert `"`e(inference_model)'"' == "repeat_measurement"
assert `"`e(inference_variance_fit)'"' == "repeat_measurement"
assert e(inference_model_option_supplied) == 0
assert e(inference_independent_units) == 24
confirm matrix e(V)
foreach option in "inferencesimulations(100)" "inferencebins(16)" "inferencegramprobes(2048)" {
    capture noisily fevc outcome control exact_c2, `exact_options' inference(highrank) `option'
    assert _rc == 198
    assert `"`e(withholding_status)'"' == "INFERENCE_OPTION_REMOVED"
    local name = substr("`option'",1,strpos("`option'","(")-1)
    assert `"`e(withholding_detail)'"' == ///
        "`name'() applied only to the variance models removed in fevc 0.6.0"
}
restore

// The removed variance models stop before any other check, whatever the
// inference() request, and name the change.
foreach model in leaveout_residual target_lowess structured_common Structured_Leverage {
    foreach reference in highrank q1 none {
        capture noisily fevc outcome control, worker(worker) firm(firm)     ///
            deletion(observation) centering(none) inference(`reference')   ///
            inferencemodel(`model') nodisplay
        assert _rc == 198
        assert `"`e(withholding_status)'"' == "INFERENCE_MODEL_REMOVED"
        local name = lower("`model'")
        assert `"`e(withholding_detail)'"' == ///
            "inferencemodel(`name') was removed in fevc 0.6.0; fevc reports repeat-measurement standard errors by default (help fevc, Standard errors)"
    }
}

// Every unsupported request fails closed before returning a substitute.
// Natively, observation deletion needs unit frequencies.
capture noisily fevc outcome control [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) engine(generic) backend(rust)    ///
    rng(counter_v1) preconditioner(diagonal) batch(8) centering(none)     ///
    inference(highrank) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "REPEAT_MEASUREMENT_TUPLE_REQUIRED"

// Observation deletion with fixed nuisance offsets has no unit geometry.
capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(observation) nuisance(fixedoffset) algorithm(jla) engine(generic) ///
    backend(rust) rng(counter_v1) preconditioner(diagonal) batch(8)  ///
    centering(none) inference(highrank) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "REPEAT_MEASUREMENT_TUPLE_REQUIRED"

// Match deletion of the same panel: every match of two observations is one
// unit.
quietly fevc outcome control, worker(worker) firm(firm)             ///
    deletion(match) algorithm(jla) engine(generic) backend(rust)    ///
    rng(counter_v1) preconditioner(diagonal) batch(8) probes(160)   ///
    tolerance(1e-11) centering(none) inference(highrank) nodisplay
assert `"`e(status)'"' == "FEVC_REPEAT_MEASUREMENT_SE"
assert `"`e(inference_deletion_selected)'"' == "match"
assert e(inference_independent_units)==150
assert el(e(component_unit_receipt),1,colnumb(e(component_unit_receipt),"deletion"))==1

// Explicit both is the same population and inference as the default above.
fevc outcome control, `common_options' stayers(both) batch(8)
assert "`e(stayers)'"=="both" & e(stayers_option_supplied)==1
assert "`e(inference_population)'"=="both"
assert mreldif(q0_results,e(results))<1e-8
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(q0_V,`compat_assert_V')<1e-8
matrix drop `compat_assert_V'

// Automatic inference must also reach the attachment on the explicit
// diagonal tuple, including Windows' legacy executor. Its point estimate
// and selected backend/solver must agree with the strict request above.
quietly fevc outcome control, worker(worker) firm(firm)             ///
    deletion(observation) algorithm(jla) engine(generic) backend(rust) ///
    rng(counter_v1) preconditioner(diagonal) batch(8) probes(160)   ///
    tolerance(1e-11) centering(none) inference(auto)                ///
    inferenceseed(24681357) nodisplay
assert inlist(`"`e(status)'"',"FEVC_REPEAT_MEASUREMENT_SE", ///
    "FEVC_REPEAT_MEASUREMENT_Q1")
assert e(inference_computed_targets)==4
assert `"`e(inference_model)'"' == "repeat_measurement"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_selected)'"' == "DIAGONAL"
assert `"`e(algorithm)'"' == "jla"
assert mreldif(q0_results,e(results))<1e-8
confirm matrix e(component_inference)

// A null outcome must be withheld atomically rather than regularized or
// replaced by another estimator/reference family.
capture noisily fevc outcome_null control, worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) engine(generic) backend(rust) ///
    rng(counter_v1) preconditioner(diagonal) batch(8) centering(none) ///
    inference(q1) inferenceseed(13579) nodisplay
assert _rc == 498
assert `"`e(status)'"' == "WITHHELD"
assert strtrim(`"`e(withholding_status)'"') != ""
capture matrix list e(component_spectrum)
assert _rc != 0
capture matrix list e(inference_intervals)
assert _rc != 0

capture noisily fevc outcome control, worker(worker) firm(firm)     ///
    deletion(observation) centering(none) inference(highrank)       ///
    inferencemodel(unrestricted_kss) nodisplay
assert _rc == 198
assert `"`e(withholding_status)'"' == "INVALID_INFERENCE_MODEL"

quietly fevc_rust snapshot
assert r(state) == 0

di as result "PASS test_rust_component_inference.do"
exit 0
