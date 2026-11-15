version 15.0
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
assert r(component_repeat_api)==1

// Native repeat-measurement standard errors under outcome centering. This is
// the existing individual-inference execution fixture: nonuniform target
// masses separate its modes; it is not a coverage experiment. Every request
// names an explicit batch, which the explicit generic tuple of this model needs.
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
tempname sort_results
_return hold `sort_results'
quietly query sortseed
local test_sort_state `"`r(sortseed)'"'
_return restore `sort_results'
local caller_sort_rng `"`test_sort_state'"'
local caller_algorithm `"`c(rng)'"'
local caller_stream = c(rngstream)
quietly datasignature
local caller_data `"`r(datasignature)'"'
local cells = 0
local mean_text "mean-centred estimate; pseudo-noise centred identically"

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

    quietly fevc outcome control `weight', `common' preconditioner(diagonal) inference(none)
    matrix point_only = e(results)
    assert "`e(centering)'"=="mean"
    assert "`e(inference_centering)'"==""
    assert e(sample)==1

    foreach reference in highrank q1 {
        local inference inference(`reference') inferenceseed(97531)
        display as text "CENTERED NATIVE CELL `deletion' `reference'"
        quietly fevc outcome control `weight', `common' `inference' preconditioner(diagonal)
        assert "`e(centering)'"=="mean"
        // The pseudo-noise is centred as the estimate is: the observed mean
        // is estimated, not held fixed.
        assert "`e(inference_centering)'"=="`mean_text'"
        assert e(inference_mean_omitted)==0
        // The control coefficient enters to first order.
        assert e(inference_nuisance_omitted)==0
        assert e(inference_independent_units)==`units'
        assert e(inference_simulations)==4*e(inference_draws)
        assert "`e(inference_model_selected)'"=="repeat_measurement"
        assert "`e(inference_variance_fit)'"=="repeat_measurement"
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
        matrix se_`deletion'_`reference' = base_intervals[1..4,2]
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
            matrix base_table = e(inference_intervals)
            matrix base_lead = e(inference_leading)
            matrix base_reported = base_table[1..4,1..10]
            local q1count = e(inference_q1_components)
            assert inrange(`q1count',1,4)
            local returned_matrices : e(matrices)
            assert !strpos(" `returned_matrices' "," V ")
            forvalues target_index = 1/4 {
                if base_table[`target_index',16]==0 {
                    // The estimate splits into its leading part and the
                    // remainder, and the interval contains it.
                    assert base_table[`target_index',7]==1
                    assert base_lead[`target_index',16]<1e-6
                    assert base_table[`target_index',8]<base_point[1,`target_index']
                    assert base_point[1,`target_index']<base_table[`target_index',9]
                }
            }
        }

        // Each transformation retains the same semantic Counter addresses.
        // A shift of the outcome leaves the mean-centred estimate and its
        // standard errors unchanged; manual centring with centering(none)
        // gives the same point estimate, but its pseudo-noise is not
        // centred, so its standard errors hold the observed mean fixed.
        foreach variant in explicit manual translated uncentered cmg mcseoff {
            local response outcome
            local center mean
            local solver diagonal
            local mcse
            if inlist("`variant'","manual","uncentered") local response centered_`deletion'
            if "`variant'"=="uncentered" local center none
            if "`variant'"=="translated" local response translated
            if "`variant'"=="cmg" local solver cmg
            if "`variant'"=="mcseoff" local mcse mcse(off)
            quietly fevc `response' control `weight', `common' `inference' ///
                preconditioner(`solver') centering(`center') `mcse'
            assert e(sample)==1
            assert e(inference_independent_units)==`units'
            assert mreldif(base_point,e(kss))<1e-8
            if "`variant'"!="mcseoff" {
                assert mreldif(base_mcse,e(mcse))<1e-8
                assert mreldif(base_mcsecov,e(mcse_cov))<1e-8
            }
            if "`variant'"=="uncentered" {
                assert "`e(inference_centering)'"=="uncentered"
                matrix candidate = e(component_inference)
                forvalues target_index = 1/4 {
                    assert reldif(candidate[`target_index',2],base_intervals[`target_index',2])<.02
                }
            }
            else {
                assert "`e(inference_centering)'"=="`mean_text'"
                assert e(inference_joint_available)==`joint'
                assert e(inference_computed_targets)==`q0count'
                assert mreldif(base_status,e(q0_status))==0
                assert mreldif(base_intervals,e(component_inference))<1e-8
                if `joint' assert mreldif(base_primitive,e(V_primitive))<1e-8
                if "`reference'"=="highrank" & `joint' {
                    tempname compat_assert_V
                    matrix `compat_assert_V' = e(V)
                    assert mreldif(base_V,`compat_assert_V')<1e-8
                    matrix drop `compat_assert_V'
                }
                if "`reference'"=="q1" {
                    assert e(inference_q1_components)==`q1count'
                    matrix candidate_table = e(inference_intervals)
                    matrix candidate_reported = candidate_table[1..4,1..10]
                    assert mreldif(base_reported,candidate_reported)<1e-8
                }
            }
            if "`variant'"=="cmg" assert "`e(inference_solver_selected)'"=="CMG"
            assert e(inference_mean_omitted)==0
            assert e(inference_solver_max_complete)<=e(inference_solver_tolerance)
            assert `"`c(rngstate)'"'==`"`caller_rng'"'
            tempname sort_results
            _return hold `sort_results'
            quietly query sortseed
            local test_sort_state `"`r(sortseed)'"'
            _return restore `sort_results'
            assert `"`test_sort_state'"'==`"`caller_sort_rng'"'
            assert `"`c(rng)'"'==`"`caller_algorithm'"'
            assert c(rngstream)==`caller_stream'
            assert rowid==_n
            quietly datasignature
            assert `"`r(datasignature)'"'==`"`caller_data'"'
            quietly fevc_rust snapshot
            assert r(state)==0 & r(handle)==0
        }

        // centering(corrected): the standard errors are those of the
        // mean-centred estimate with the corrected increment held fixed, and
        // a q = 1 interval moves with the posted estimate.
        quietly fevc outcome control `weight', `common' `inference' ///
            preconditioner(diagonal) centering(corrected)
        assert "`e(centering)'"=="corrected"
        assert "`e(inference_centering)'"=="standard error of the mean-centred estimate; corrected-centering increment treated as fixed"
        matrix candidate = e(component_inference)
        matrix candidate_se = candidate[1..4,2]
        assert mreldif(se_`deletion'_`reference',candidate_se)<1e-12
        if "`reference'"=="q1" {
            matrix candidate_table = e(inference_intervals)
            matrix shift = e(b)
            forvalues target_index = 1/4 {
                if candidate_table[`target_index',7]==1 {
                    foreach column in 5 6 {
                        assert abs(candidate_table[`target_index',`column']- ///
                            base_table[`target_index',`column']- ///
                            (shift[1,`target_index']-base_point[1,`target_index']))< ///
                            1e-10*max(1,abs(base_table[`target_index',`column']))
                    }
                }
            }
        }
        assert `"`c(rngstate)'"'==`"`caller_rng'"'
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
        local ++cells
    }
}

// Unequal physical-copy masses change the fixed mean but do not create new
// independent matches. Stored-row target mass is split across literal copies.
local copies_options worker(worker) firm(firm) deletion(match) deletionid(match) ///
    nuisance(fixedoffset) stayers(movers) algorithm(jla) engine(generic) ///
    backend(rust) rng(counter_v1) preconditioner(diagonal) probes(200) ///
    seed(721946) batch(8) tolerance(1e-12) targetweight(target) ///
    inference(highrank) inferenceseed(97531) nodisplay
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
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(compressed_V,`compat_assert_V')<1e-8
matrix drop `compat_assert_V'
restore

// Standard errors of the components and project() are separate calls; the
// default leaves the components to point estimates beside the projection.
local combined worker(worker) firm(firm) deletion(observation) stayers(movers) ///
    nuisance(joint) algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) probes(200) seed(721946) batch(8) tolerance(1e-12) ///
    targetweight(target) project(z) projecteffect(firm) nodisplay
foreach reference in highrank q1 {
    capture noisily fevc outcome control, `combined' inference(`reference') inferenceseed(97531)
    assert _rc==498
    assert "`e(withholding_status)'"=="COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED"
    assert "`e(inference_centering)'"==""
    assert `"`c(rngstate)'"'==`"`caller_rng'"'
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
}
quietly fevc outcome control, `combined' inference(none)
matrix projection_point = e(b)
matrix projection_only = e(projection_b)
quietly fevc outcome control, `combined' inference(auto)
assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""
tempname compat_assert_b
matrix `compat_assert_b' = e(b)
assert mreldif(projection_point,`compat_assert_b')<1e-12
matrix drop `compat_assert_b'
assert mreldif(projection_only,e(projection_b))<1e-12
capture confirm matrix e(V)
assert _rc

// The variance models removed in fevc 0.6.0 and their tuning options stop
// before any native work.
local removed worker(worker) firm(firm) deletion(observation) algorithm(jla) ///
    engine(generic) backend(rust) rng(counter_v1) preconditioner(diagonal) ///
    batch(8) nodisplay
foreach request in "inferencemodel(structured_common)" "inferencemodel(structured_leverage)" ///
    "inferencegramprobes(513)" "inferencesimulations(129)" {
    foreach reference in highrank q1 {
        capture noisily fevc outcome control, `removed' centering(corrected) ///
            inference(`reference') `request'
        assert _rc==198
        local expected INFERENCE_OPTION_REMOVED
        if strpos("`request'","inferencemodel") local expected INFERENCE_MODEL_REMOVED
        assert "`e(withholding_status)'"=="`expected'"
        assert "`e(inference_centering)'"==""
        assert `"`c(rngstate)'"'==`"`caller_rng'"'
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
}
assert `cells'==4
tempname sort_results
_return hold `sort_results'
quietly query sortseed
local test_sort_state `"`r(sortseed)'"'
_return restore `sort_results'
assert `"`test_sort_state'"'==`"`caller_sort_rng'"'
quietly datasignature
assert `"`r(datasignature)'"'==`"`caller_data'"'
assert "$VCKSS_CENTERING"==""
di as result "PASS test_component_centering_native.do cells=`cells'"
exit 0
