version 18.0
clear all
set more off
set varabbrev off
args pkgroot plugindir
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
if `"`plugindir'"'!="" adopath ++ `"`plugindir'"'
quietly run `"`pkgroot'/fevc.ado"'
quietly fevc_rust probe
assert r(component_centering_api)==1

// This is the existing individual-inference execution fixture. Nonuniform
// target masses separate its modes; it is not a q0/q1 coverage experiment.
set obs 800
generate long worker = floor((_n-1)/40)
generate long firm = floor(mod(_n-1,40)/2)
generate long match = floor((_n-1)/2)+1
generate long rowid = _n
generate byte copies = 1+mod(match,3)
generate double target = (.75+mod((_n-1)*13,29)/31)*(1+.1*worker)^2*(1+.1*firm)^2
generate double control = .31*mod(_n-1,2)+mod(_n-1,7)/29
generate double shock = (mod((_n-1)*37+11,101)/50-1)*(1.4+.05*worker+.036*firm)
generate double outcome = 12+worker-.8*firm+1.4*control+shock
generate double translated = outcome+32
generate double z = sin(worker/5)+cos(firm/3)

// Independent working-mean calculation: joint uses y; fixedoffset subtracts
// the full-model control fit. Target mass never enters either mean.
quietly summarize outcome, meanonly
scalar mean_observation = r(mean)
generate double centered_observation = outcome-mean_observation
quietly regress outcome control i.worker i.firm [fw=copies]
scalar gamma = _b[control]
quietly summarize outcome [fw=copies], meanonly
scalar mean_match = r(mean)
quietly summarize control [fw=copies], meanonly
scalar mean_match = mean_match-gamma*r(mean)
generate double centered_match = outcome-mean_match
assert abs(mean_observation)>1
assert abs(mean_match)>1
assert abs(mean_match-mean_observation)>1e-3
sort rowid
set seed 721946
local caller_rng `"`c(rngstate)'"'
local caller_sort_rng `"`c(sortrngstate)'"'
local caller_algorithm `"`c(rng)'"'
local caller_stream = c(rngstream)
quietly datasignature
local caller_data `"`r(datasignature)'"'
local cells = 0

foreach deletion in observation match {
    local weight
    local options deletion(observation) nuisance(joint)
    local units = 800
    if "`deletion'"=="match" {
        local weight [fw=copies]
        local options deletion(match) deletionid(match) nuisance(fixedoffset)
        local units = 400
    }
    local common worker(worker) firm(firm) stayers(movers) `options' ///
        algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
        targetweight(target) probes(200) batch(8) tolerance(1e-12) ///
        seed(721946) nodisplay

    quietly fevc outcome control `weight', `common' preconditioner(diagonal)
    matrix point_only = e(results)
    assert "`e(centering)'"=="mean"
    assert "`e(inference_centering)'"==""
    assert e(sample)==1

    foreach model in structured_common structured_leverage {
        foreach reference in highrank q1 {
            local inference inference(`reference') inferencemodel(`model') ///
                inferencesimulations(129) inferencegramprobes(513) inferenceseed(97531)
            display as text "CENTERED NATIVE CELL `deletion' `model' `reference'"
            quietly fevc outcome control `weight', `common' `inference' preconditioner(diagonal)
            assert "`e(centering)'"=="mean"
            assert "`e(inference_centering)'"=="fixed observed mean"
            assert e(inference_mean_omitted)==1
            assert e(inference_nuisance_omitted)==("`deletion'"=="match")
            assert e(inference_independent_units)==`units'
            assert e(inference_gram_probes)==513
            assert e(inference_simulations)==129
            assert "`e(inference_model_selected)'"=="`model'"
            assert "`e(inference_backend_selected)'"=="rust"
            assert "`e(preconditioner_selected)'"=="DIAGONAL"
            assert "`e(mcse_centering)'"=="fixed observed mean"
            assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
            assert e(sample)==1
            matrix base_results = e(results)
            matrix base_point = e(kss)
            matrix base_mcse = e(mcse)
            matrix base_mcsecov = e(mcse_cov)
            matrix base_intervals = e(component_inference)
            matrix base_status = e(q0_status)
            matrix base_fit = e(residual_moment_diagnostics)
            local joint = e(inference_joint_available)
            local q0count = e(inference_computed_targets)
            assert inrange(`q0count',1,4)
            if `joint' matrix base_primitive = e(V_primitive)
            if "`reference'"=="highrank" & `joint' {
                matrix base_V = e(V)
                matrix accounting = (1,0,0 \ 0,1,0 \ 0,0,1 \ 1,1,2)
                matrix mapped_V = accounting*base_primitive*accounting'
                assert mreldif(base_V,mapped_V)<1e-12
            }
            assert abs(base_point[1,4]-base_point[1,1]-base_point[1,2]-2*base_point[1,3]) ///
                <1e-8*max(1,abs(base_point[1,4]))

            // Observation attachments use design-only probe ordering. The
            // registered six-MCSE comparison applies to point-only ordering;
            // match attachments keep the same point and numerical MCSE path.
            forvalues target_index = 1/4 {
                scalar point_scale = max(1,abs(point_only[3,`target_index']),abs(base_results[3,`target_index']))
                scalar point_limit = 1e-8*point_scale
                if "`deletion'"=="observation" scalar point_limit = ///
                    max(point_limit,6*sqrt(point_only[4,`target_index']^2+base_results[4,`target_index']^2))
                assert abs(point_only[3,`target_index']-base_results[3,`target_index'])<=point_limit
            }
            if "`deletion'"=="match" assert mreldif(point_only,base_results)<1e-8

            if "`reference'"=="q1" {
                matrix base_q1 = e(q1_inference)
                matrix base_q1status = e(q1_status)
                matrix base_q1raw = e(component_q1_diagnostics)
                matrix base_spectrum = e(component_spectrum)
                matrix base_q1intervals = base_q1[1..4,1..6]
                local q1count = e(q1_computed_targets)
                assert inrange(`q1count',1,4)
                local returned_matrices : e(matrices)
                assert !strpos(" `returned_matrices' "," V ")
                forvalues target_index = 1/4 {
                    if base_q1status[`target_index',1]==0 {
                        scalar raw_recenter = base_q1raw[`target_index',"recenter_var_b1"]
                        scalar score = base_q1raw[`target_index',"leading_score"]
                        scalar remainder = base_q1raw[`target_index',"remainder_estimate"]
                        scalar leading = base_spectrum[`target_index',"lambda1"]*(score^2-raw_recenter)
                        assert raw_recenter<.
                        assert base_q1raw[`target_index',"leading_variance"]>0 & ///
                            base_q1raw[`target_index',"leading_variance"]<.
                        assert abs(base_point[1,`target_index']-leading-remainder) ///
                            <1e-8*max(1,abs(base_point[1,`target_index']),abs(remainder))
                        assert base_q1raw[`target_index',"remainder_identity_error"] ///
                            <1e-8*max(1,abs(remainder))
                    }
                }
            }

            // Each transformation retains the same semantic Counter addresses.
            // Inference probes themselves are not centered or regenerated.
            foreach variant in explicit shifted translated cmg mcseoff {
                local response outcome
                local center mean
                local solver diagonal
                local mcse
                if "`variant'"=="shifted" {
                    local response centered_`deletion'
                    local center none
                }
                if "`variant'"=="translated" local response translated
                if "`variant'"=="cmg" local solver cmg
                if "`variant'"=="mcseoff" local mcse mcse(off)
                quietly fevc `response' control `weight', `common' `inference' ///
                    preconditioner(`solver') centering(`center') `mcse'
                assert e(sample)==1
                assert e(inference_independent_units)==`units'
                assert e(inference_joint_available)==`joint'
                assert e(inference_computed_targets)==`q0count'
                assert mreldif(base_status,e(q0_status))==0
                assert mreldif(base_point,e(kss))<1e-8
                assert mreldif(base_intervals,e(component_inference))<1e-8
                assert mreldif(base_fit,e(residual_moment_diagnostics))<1e-8
                if `joint' assert mreldif(base_primitive,e(V_primitive))<1e-8
                if "`reference'"=="highrank" & `joint' assert mreldif(base_V,e(V))<1e-8
                if "`variant'"!="mcseoff" {
                    assert mreldif(base_mcse,e(mcse))<1e-8
                    assert mreldif(base_mcsecov,e(mcse_cov))<1e-8
                }
                if "`reference'"=="q1" {
                    assert e(q1_computed_targets)==`q1count'
                    assert mreldif(base_q1status,e(q1_status))==0
                    matrix candidate_q1 = e(q1_inference)
                    matrix candidate_q1intervals = candidate_q1[1..4,1..6]
                    assert mreldif(base_q1intervals,candidate_q1intervals)<1e-8
                    // Mode signs can differ across equivalent eigensolvers;
                    // these raw recenter/remainder quantities are sign invariant.
                    foreach field in recenter_var_b1 leading_variance remainder_estimate remainder_variance {
                        matrix raw_reference = base_q1raw[1..4,"`field'"]
                        matrix raw_candidate = e(component_q1_diagnostics)[1..4,"`field'"]
                        assert mreldif(raw_reference,raw_candidate)<1e-8
                    }
                }
                assert e(inference_mean_omitted)==("`center'"=="mean")
                if "`center'"=="mean" assert "`e(inference_centering)'"=="fixed observed mean"
                else assert "`e(inference_centering)'"=="uncentered"
                assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
                assert `"`c(rngstate)'"'==`"`caller_rng'"'
                assert `"`c(sortrngstate)'"'==`"`caller_sort_rng'"'
                assert `"`c(rng)'"'==`"`caller_algorithm'"'
                assert c(rngstream)==`caller_stream'
                assert rowid==_n
                quietly datasignature
                assert `"`r(datasignature)'"'==`"`caller_data'"'
                quietly fevc_rust snapshot
                assert r(state)==0 & r(handle)==0
            }
            local ++cells
        }
    }
}

// Unequal physical-copy masses change the fixed mean but do not create new
// independent matches. Stored-row target mass is split across literal copies.
local copies_options worker(worker) firm(firm) deletion(match) deletionid(match) ///
    nuisance(fixedoffset) stayers(movers) algorithm(jla) engine(generic) ///
    backend(rust) rng(counter_v1) preconditioner(diagonal) probes(200) ///
    seed(721946) batch(8) tolerance(1e-12) targetweight(target) ///
    inference(highrank) inferencemodel(structured_common) inferencegramprobes(513) ///
    inferencesimulations(129) inferenceseed(97531) nodisplay
quietly fevc outcome control [fw=copies], `copies_options'
matrix compressed_results = e(results)
matrix compressed_V = e(V)
assert e(inference_independent_units)==400
preserve
expand copies
replace target = target/copies
replace copies = 1
quietly fevc outcome control, `copies_options'
assert e(inference_independent_units)==400
assert mreldif(compressed_results,e(results))<1e-8
assert mreldif(compressed_V,e(V))<1e-8
restore

// Preserve the existing preflight boundary: a native prepared generation
// cannot currently carry both projection and component attachments.
local combined worker(worker) firm(firm) deletion(observation) stayers(movers) ///
    nuisance(joint) algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) probes(200) seed(721946) batch(8) tolerance(1e-12) ///
    targetweight(target) inference(highrank) inferencemodel(structured_common) ///
    inferencegramprobes(513) inferencesimulations(129) inferenceseed(97531) ///
    project(z) projecteffect(firm) nodisplay
capture noisily fevc outcome control, `combined'
assert _rc==498
assert "`e(withholding_status)'"=="COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED"
assert "`e(inference_centering)'"==""

foreach reference in highrank q1 {
    capture noisily fevc outcome control, worker(worker) firm(firm) ///
        deletion(observation) algorithm(jla) engine(generic) backend(rust) ///
        rng(counter_v1) preconditioner(diagonal) centering(corrected) ///
        inference(`reference') inferencemodel(structured_common) nodisplay
    assert _rc==498
    assert "`e(withholding_status)'"=="CENTERING_INFERENCE_UNSUPPORTED"
    assert "`e(inference_centering)'"==""
    local returned_scalars : e(scalars)
    assert !strpos(" `returned_scalars' "," inference_mean_omitted ")
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
assert `cells'==8
assert `"`c(sortrngstate)'"'==`"`caller_sort_rng'"'
quietly datasignature
assert `"`r(datasignature)'"'==`"`caller_data'"'
assert "$VCKSS_CENTERING"==""
di as result "PASS test_component_centering_native.do cells=`cells'"
exit 0
