* Repeat-measurement standard errors on the Mata routes: the sample variance
* of every repeat-measurement set (an eligible stayer's observations; under
* observation deletion also a mover's match of two or more observations),
* the two-step model for the remaining units, and the high-rank covariance
* with the pair term of the sets. Expected values come from an independent
* dense calculation in Python on the same deterministic panels (exact trace
* term); the command simulates that term, so standard errors agree to its
* Monte Carlo error. Every fevc call names inference().
version 15.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
* 1 when the native engine carries the first-order term of estimated control
* coefficients (0 for a plugin built before it): the native control checks.
local native_control_term 1

* Comparisons against expected numbers are true relative differences:
* reldif() divides by one plus the reference, which is far too loose for
* standard errors, traces and eigenvalues well below one. Tolerances for
* simulated quantities are about three times the largest deviation observed
* over five seeds; deterministic quantities agree to rounding.
program define _rm_close
    args value expected tolerance
    assert abs(`value'/`expected'-1)<`tolerance'
end
program define _rm_far
    args value expected distance
    assert abs(`value'/`expected'-1)>`distance'
end

* 200 movers with three jobs of two observations (ex2), and 150 stayers with
* four observations each at firms of the movers' network.
fevc, simulate_data(ex2) clear
keep log_wage worker_id firm_id
local N = _N
quietly set obs `=_N+600'
quietly replace worker_id = 200+ceil((_n-`N')/4) if _n>`N'
quietly replace firm_id = 1+mod(worker_id,61) if _n>`N'
quietly replace log_wage = sin(worker_id*1.7)+0.01*firm_id+ ///
    (0.2+0.3*mod(worker_id,3))*cos(_n*2.3+worker_id) if _n>`N'
generate double x1 = sin(_n*.37)+.2*firm_id/61
* A well-identified control whose estimation moves the standard errors of the
* worker variance and of the sum by a third or more: half the worker's mean
* outcome and a wave in the row index (the rows in this order).
capture drop x2 wbar2 rowid
generate long rowid = _n
egen double wbar2 = mean(log_wage), by(worker_id)
assert rowid==_n
generate double x2 = 0.5*wbar2 + 1.0*sin(rowid*1.3)

* Independent dense calculation: estimates, pair term, standard errors.
matrix expected_b = (2.193981373967747, 1.014791107858934, -.2004904720705475)
scalar expected_pair = 1.478986773550021e-06
matrix expected_se = (.2595212617894884, .2433340968591373, ///
    .1263905520856996, .2676009330004356)
* With a control, an independent dense calculation of the first-order term
* of its estimated coefficient (exact traces): ctl_ with x1, ctl2_ with x2;
* _old is what treating the coefficient as known gives. x1 moves standard
* errors by less than one per cent, so only x2 can tell the two apart.
matrix ctl_match_se = (0.2588614707212713, 0.24333718869610568, ///
    0.12638942397002775, 0.26734842312617657)
matrix ctl2_match_se = (0.32929725869457038, 0.24289970069440414, ///
    0.12858357369924972, 0.35839171643359002)
matrix ctl2_match_se_old = (0.2499999017611641, 0.24297619621603864, ///
    0.12728594530959025, 0.25644927787519051)

* controls: 1 when the request has controls and their first-order term.
program define _rm_check
    args draws sets controls
    if "`controls'"=="" local controls 0
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_model)'"=="repeat_measurement"
    assert "`e(inference_variance_fit)'"=="repeat_measurement"
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_SE"
    assert e(inference_draws)==`draws'
    assert e(inference_repeat_sets)==`sets'
    * Estimated control coefficients enter to first order, as in the
    * leave-out residual model; without controls nothing is omitted.
    assert e(inference_nuisance_omitted)==!`controls'
    assert (strpos("`e(inference_method)'","included to first order")>0)==`controls'
    assert strpos("`e(inference_offset_warning)'","first-order term")>0 | !`controls'
    assert strpos("`e(inference_offset_warning)'","no controls")>0 | `controls'
    assert strpos("`e(inference_variance_warning)'","model for movers")>0
    confirm matrix e(V)
    tempname V Vt b ci shape
    matrix `V' = e(V)
    matrix `Vt' = `V''
    matrix `b' = e(b)
    matrix `ci' = e(component_inference)
    matrix `shape' = e(inference_repeat)
    assert colsof(`shape')==14 & `shape'[1,5]==`sets'
    assert inrange(`shape'[1,1],1,5) & inrange(`shape'[1,2],1,6)
    assert `shape'[1,3]>0 & `shape'[1,8]>=0
    assert mreldif(`V',`Vt')<1e-12
    forvalues j = 1/4 {
        assert `V'[`j',`j']>0
        assert reldif(`ci'[`j',1],`b'[1,`j'])<1e-12
        assert reldif(`ci'[`j',2],sqrt(`V'[`j',`j']))<1e-12
    }
    local total = `V'[1,1]+`V'[2,2]+4*`V'[3,3]+2*`V'[1,2]+4*`V'[1,3]+4*`V'[2,3]
    assert reldif(`total',`V'[4,4])<1e-10
    * That combination has zero variance by construction, so lincom gives it
    * a standard error of zero up to rounding (missing if rounding goes
    * negative).
    quietly lincom worker_variance+firm_variance+2*worker_firm_covariance-total_variance
    assert abs(r(estimate))<1e-10*max(1,abs(`b'[1,4]))
    assert r(se)<1e-6*sqrt(`V'[4,4]) | missing(r(se))
    tempname rank
    matrix `rank' = e(inference_effective_rank)
    assert rowsof(`rank')==4 & `rank'[1,1]>=1
    assert e(inference_max_influence_share)>0 & e(inference_max_influence_share)<=1
    assert e(inference_mean_omitted)==0
    * inference(highrank) computes no leading direction.
    capture confirm matrix e(inference_intervals)
    assert _rc
    capture confirm matrix e(inference_leading)
    assert _rc
end

* Leading directions and the interval reported for every component, against
* the independent dense calculation in the lead_* matrices. tolerance: a 1 x 7
* matrix of true relative tolerances for (1) the eigenvalue, (2) the largest
* squared weight, (3) what the outcome and the unit variances determine
* (|b1|, the variances of the score, the remainder and its linear part),
* (4) the covariance of score and remainder, in units of the product of
* their standard deviations (it can be near zero), (5) the pair term, (6) the
* simulated trace and the remainder variance, (7) the curvature, the bounds
* (in widths of the interval) and the variance share. prefix: of the
* expected matrices, lead_ by default; another prefix marks a request with
* controls, whose expected matrices also hold the identity's leave-out
* variance (leaveout_identity, column 12) apart from the control-inclusive
* one, and the leading variance.
* The eigenvalue share of the leading direction (column 8) against the one
* of the exact route; its trace is simulated on every route. Largest
* deviation on the fixtures: linear solves 1.9%, native 3.5%.
program define _rm_share_check
    args expected tolerance
    tempname lead
    matrix `lead' = e(inference_leading)
    forvalues j = 1/3 {
        _rm_close `lead'[`j',8] `expected'[`j',8] `tolerance'
    }
end

program define _rm_leading_check
    args request q1_components tolerance prefix
    local p = cond("`prefix'"=="","lead_","`prefix'")
    forvalues i = 1/7 {
        local tol`i' = `tolerance'[1,`i']
    }
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_Q1"
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_leading_request)'"=="`request'"
    assert e(inference_q1_components)==`q1_components'
    assert e(inference_share_threshold)==.1
    assert e(inference_leading_unresolved)==0
    * A q = 1 interval for any component withholds the joint covariance.
    capture confirm matrix e(V)
    assert _rc
    assert e(inference_joint_posted)==0
    tempname lead table ci b
    matrix `lead' = e(inference_leading)
    matrix `table' = e(inference_intervals)
    matrix `ci' = e(component_inference)
    matrix `b' = e(b)
    assert rowsof(`lead')==4 & colsof(`lead')==28
    assert rowsof(`table')==4 & colsof(`table')==16
    forvalues j = 1/4 {
        assert `lead'[`j',1]==0 & `lead'[`j',2]==1
        _rm_close `lead'[`j',5] `p'lambda[1,`j'] `tol1'
        assert abs(`lead'[`j',6])<=abs(`lead'[`j',5])*(1+1e-9)
        assert `lead'[`j',16]<1e-6
        assert reldif(`table'[`j',1],`b'[1,`j'])<1e-12
        assert reldif(`table'[`j',2],`ci'[`j',2])<1e-12
        * The interval moves with the posted estimate.
        assert reldif(`table'[`j',5],`lead'[`j',26]+`b'[1,`j']-`lead'[`j',10])<1e-12
        assert reldif(`table'[`j',6],`lead'[`j',27]+`b'[1,`j']-`lead'[`j',10])<1e-12
        assert `table'[`j',5]<`table'[`j',1] & `table'[`j',1]<`table'[`j',6]
        local expected_regime = cond(`j'<4,1,"`request'"=="q1")
        assert `table'[`j',7]==`expected_regime'
        local low = cond(`expected_regime',5,3)
        assert `table'[`j',8]==`table'[`j',`low'] & `table'[`j',9]==`table'[`j',`low'+1]
        assert inrange(`lead'[`j',25],1.95,2.5)
    }
    * The direction of the sum is not unique: beyond its eigenvalue only the
    * three components are compared.
    forvalues j = 1/3 {
        _rm_close `lead'[`j',9] `p'maxw2[1,`j'] `tol2'
        _rm_close abs(`lead'[`j',11]) `p'b1[1,`j'] `tol3'
        _rm_close `lead'[`j',13] `p'fitted[1,`j'] `tol3'
        if "`prefix'"=="" {
            _rm_close `lead'[`j',12] `p'leaveout[1,`j'] `tol3'
            assert reldif(`lead'[`j',14],max(`lead'[`j',12],`lead'[`j',13]))<1e-12
        }
        else {
            _rm_close `lead'[`j',12] `p'leaveout_identity[1,`j'] `tol3'
            _rm_close `lead'[`j',14] `p'leading[1,`j'] `tol3'
        }
        _rm_close `lead'[`j',15] `p'theta_r[1,`j'] `tol3'
        _rm_close `lead'[`j',19] `p'linear[1,`j'] `tol3'
        * The sign of a direction is arbitrary.
        assert abs(abs(`lead'[`j',17])-abs(`p'cross[1,`j']))< ///
            `tol4'*sqrt(`p'fitted[1,`j']*`p'rr[1,`j'])
        _rm_close `lead'[`j',22] `p'pair[1,`j'] `tol5'
        _rm_close `lead'[`j',20] `p'trace[1,`j'] `tol6'
        _rm_close `lead'[`j',18] `p'rr[1,`j'] `tol6'
        _rm_close `lead'[`j',24] `p'kappa[1,`j'] `tol7'
        * With controls the posted estimate keeps them in the operator, and
        * the expected interval is that of the pass's fixed-offset estimate.
        local width = `p'ub[1,`j']-`p'lb[1,`j']
        local lb = cond("`prefix'"=="",`table'[`j',5],`lead'[`j',26])
        local ub = cond("`prefix'"=="",`table'[`j',6],`lead'[`j',27])
        assert abs(`lb'-`p'lb[1,`j'])<`tol7'*`width'
        assert abs(`ub'-`p'ub[1,`j'])<`tol7'*`width'
        _rm_close `table'[`j',10] `p'share[1,`j'] `tol7'
    }
end

* Whether the display of a command shows a text: expected (1 or 0), the
* text, the command.
program define _rm_display
    gettoken expected 0 : 0
    gettoken text 0 : 0
    tempfile out
    tempname fh
    quietly log using `"`out'"', text replace name(_rm_display)
    `0'
    quietly log close _rm_display
    local found 0
    file open `fh' using `"`out'"', read text
    file read `fh' line
    while r(eof)==0 {
        if strpos(`"`macval(line)'"',`"`text'"') local found 1
        file read `fh' line
    }
    file close `fh'
    assert `found'==`expected'
end

* --- point estimates are not touched -----------------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(none) nodisplay
matrix point = e(kss)
forvalues j = 1/3 {
    _rm_close point[1,`j'] expected_b[1,`j'] 1e-12
}

* --- exact kernels: the independent calculation ------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(512) nodisplay
_rm_check 512 150
assert mreldif(point,e(kss))<1e-12
assert e(inference_independent_units)==1200
_rm_close e(inference_pair_term) expected_pair 1e-12
matrix shape = e(inference_repeat)
* Every match has two observations here, so the two mass terms of the scale
* are spanned by the intercept: three active scale terms, six profile terms.
assert shape[1,1]==3 & shape[1,2]==6 & shape[1,6]==600 & shape[1,7]==0
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * 512 draws: the simulated trace term moves standard errors by at most
    * 0.4 per cent over five seeds.
    _rm_close ci[`j',2] expected_se[1,`j'] 0.0125
}
* More draws move only the simulated part.
matrix b512 = e(b)
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(64) nodisplay
_rm_check 64 150
tempname compat_assert_b
matrix `compat_assert_b' = e(b)
assert mreldif(b512,`compat_assert_b')<1e-12
matrix drop `compat_assert_b'
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * At most 1.1 per cent over five seeds.
    _rm_close ci[`j',2] expected_se[1,`j'] 0.035
}

* --- linear solves and projected leverages -----------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 150
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * Projection noise in leverages and target diagonals on top of the
    * draws: at most 3.4 per cent over five seeds.
    _rm_close ci[`j',2] expected_se[1,`j'] 0.1
}

* --- controls, the mover-only population, centering --------------------------
* The expected numbers include the first-order term of the estimated control
* coefficient. With x1 it moves standard errors by less than one per cent.
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(512) nodisplay
_rm_check 512 150 1
matrix ci = e(component_inference)
forvalues j = 1/4 {
    _rm_close ci[`j',2] ctl_match_se[1,`j'] 0.0125
}
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) nuisance(fixedoffset) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
_rm_check 8 150 1
* With x2 it raises the standard errors of the worker variance and of the sum
* by 32 and 40 per cent; within-unit products and their cross term with the
* leave-out residual enter because the stayers' units have several
* observations. 512 draws: within 0.3 per cent over five seeds, while
* treating the coefficient as known is excluded.
quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(512) nodisplay
_rm_check 512 150 1
matrix ci = e(component_inference)
forvalues j = 1/4 {
    _rm_close ci[`j',2] ctl2_match_se[1,`j'] 0.0125
}
foreach j in 1 4 {
    _rm_far ctl2_match_se_old[1,`j'] ctl2_match_se[1,`j'] 0.0125
    _rm_far ci[`j',2] ctl2_match_se_old[1,`j'] 0.0125
}
* Linear solves and projected leverages, 64 draws: within 2.9 per cent over
* five seeds, the known coefficient still excluded.
quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 150 1
matrix ci = e(component_inference)
forvalues j = 1/4 {
    _rm_close ci[`j',2] ctl2_match_se[1,`j'] 0.1
}
foreach j in 1 4 {
    _rm_far ctl2_match_se_old[1,`j'] ctl2_match_se[1,`j'] 0.1
    _rm_far ci[`j',2] ctl2_match_se_old[1,`j'] 0.1
}
* The display says that the term is included, and nothing about controls
* without them.
_rm_display 1 "Estimated controls enter to first order." fevc log_wage x2, ///
    worker(worker_id) firm(firm_id) backend(mata) algorithm(exact) ///
    inference(highrank) inferencemodel(repeat_measurement)
_rm_display 0 "first order" fevc log_wage, worker(worker_id) firm(firm_id) ///
    backend(mata) algorithm(exact) inference(highrank) ///
    inferencemodel(repeat_measurement)
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) stayers(movers) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
_rm_check 8 0
assert e(inference_pair_term)==0
foreach centering in none corrected {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
        algorithm(exact) centering(`centering') inference(highrank) ///
        inferencemodel(repeat_measurement) nodisplay
    _rm_check 8 150
}

* --- the native route, when the loaded plugin carries the method -------------
local native 0
capture quietly fevc_rust probe
if !_rc {
    if r(component_repeat_api)==1 & r(component_mixed_api)==1 local native 1
}
if `native' {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) inference(highrank) inferencemodel(repeat_measurement) ///
        inferencedraws(64) nodisplay
    _rm_check 64 150
    assert "`e(backend_selected)'"=="rust"
    assert colsof(e(component_inference_receipt))==78
    assert e(inference_independent_units)==1200
    * Projected target diagonals: the pair term agrees to projection noise
    * (0.8 per cent over five seeds), standard errors to 4.1 per cent.
    _rm_close e(inference_pair_term) expected_pair 0.025
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        _rm_close ci[`j',2] expected_se[1,`j'] 0.12
    }
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(highrank) inferencemodel(repeat_measurement) nodisplay
    _rm_check 8 150 `native_control_term'
    if `native_control_term' {
        * As without controls: 3.8 per cent over five seeds.
        quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(rust) ///
            probes(800) inference(highrank) inferencemodel(repeat_measurement) ///
            inferencedraws(64) nodisplay
        _rm_check 64 150 1
        matrix ci = e(component_inference)
        forvalues j = 1/4 {
            _rm_close ci[`j',2] ctl2_match_se[1,`j'] 0.12
        }
        foreach j in 1 4 {
            _rm_far ci[`j',2] ctl2_match_se_old[1,`j'] 0.12
        }
    }
    * inference(auto) alone is this model with the automatic interval.
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(auto) nodisplay
    assert "`e(inference_variance_fit)'"=="repeat_measurement"
    assert "`e(backend_selected)'"=="rust" & e(inference_auto)==1
    confirm matrix e(inference_intervals)
    * Standard errors do not change the estimator. With controls on the
    * native backend they select another executor, so agreement is to solver
    * tolerance.
    quietly fevc log_wage x1 x2, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(none) nodisplay
    matrix native_point = e(kss)
    quietly fevc log_wage x1 x2, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(highrank) nodisplay
    _rm_check 8 150 `native_control_term'
    assert mreldif(native_point,e(kss))<1e-5

    * engine(generic) with an automatic batch: the diagonal queue and the
    * direct CMG attachment executor carry the pass, whose component batch
    * receipt declares and selects no Gram width. The default is named as
    * inference(auto) because a suite may switch the plain default off. The
    * standard errors equal those of an explicit batch holding every probe,
    * and those of engine(generic) alone, to the 1e-8 the routes promise
    * for changes of batch width, solver and threads.
    foreach deletion in match observation {
        foreach inference in auto highrank {
            local request worker(worker_id) firm(firm_id) backend(rust) ///
                engine(generic) deletion(`deletion') inference(`inference') nodisplay
            quietly fevc log_wage, `request'
            matrix generic_ci = e(component_inference)
            foreach solver in diagonal cmg {
                quietly fevc log_wage, `request' preconditioner(`solver')
                assert substr("`e(status)'",1,24)=="FEVC_REPEAT_MEASUREMENT_"
                assert "`e(inference_model)'"=="repeat_measurement"
                assert "`e(rust_execution_mode)'"== ///
                    cond("`solver'"=="diagonal","diagonal_queue","direct_attachments")
                assert "`e(rust_component_batch_schema)'"=="VCKSS-COMPONENT-BATCH-V1"
                matrix batch = e(rust_component_batch_receipt)
                assert el(batch,1,colnumb(batch,"policy"))==1 & el(batch,1,colnumb(batch,"declared_component"))==1
                assert el(batch,1,colnumb(batch,"declared_gram"))==0 & el(batch,1,colnumb(batch,"gram"))==0
                assert inrange(el(batch,1,colnumb(batch,"component")),1,el(batch,1,colnumb(batch,"cap")))
                quietly fevc_rust snapshot
                assert r(state)==0 & r(handle)==0
                local automatic_status "`e(status)'"
                matrix automatic_ci = e(component_inference)
                local automatic_joint = e(inference_joint_posted)
                if `automatic_joint' matrix automatic_V = e(V)
                if "`inference'"=="auto" matrix automatic_table = e(inference_intervals)
                quietly fevc log_wage, `request' preconditioner(`solver') batch(256)
                assert "`e(status)'"=="`automatic_status'"
                capture confirm matrix e(rust_component_batch_receipt)
                assert _rc
                assert mreldif(e(component_inference),automatic_ci)<1e-8
                assert e(inference_joint_posted)==`automatic_joint'
                if `automatic_joint' {
                    tempname compat_assert_V
                    matrix `compat_assert_V' = e(V)
                    assert mreldif(`compat_assert_V',automatic_V)<1e-8
                    matrix drop `compat_assert_V'
                }
                if "`inference'"=="auto" assert mreldif(e(inference_intervals),automatic_table)<1e-8
                assert mreldif(automatic_ci,generic_ci)<1e-8
            }
        }
    }

    * A native failure inside a helper ado-file posts its typed status and
    * releases the generation: the component attachment and the component
    * batch receipt (strict and default requests), and the release and the
    * immutable re-export of the exact and the compressed posters. The
    * helpers reach the abort routine and the failure poster, which live in
    * their own ado-files; a helper cannot call another file's private
    * programs. The injected native failure is the same call on a generation
    * that does not exist.
    capture program drop fevc__rust_public_call
    program define fevc__rust_public_call, rclass
        version 15.0
        gettoken command rest : 0
        if "`command'"=="result" & !strpos(`"`rest'"',"componentinference(") {
            global RM_POSTER_RESULTS = $RM_POSTER_RESULTS + 1
        }
        if "`command'"=="$RM_HELPER_FAULT" {
            gettoken handle rest : rest, parse(" ,")
            fevc_rust `command' 2147483647 `rest'
            exit 498
        }
        tempname result
        fevc_rust `command' `rest'
        local altered = "`command'"=="result" & "$RM_HELPER_FAULT"=="reexport" & ///
            $RM_POSTER_RESULTS==2
        if `altered' {
            matrix `result' = r(result)
            matrix `result'[1,1] = `result'[1,1]+1
        }
        return add
        if `altered' return matrix result = `result'
    end
    tempfile helper_log
    quietly log using `"`helper_log'"', text replace name(rm_helper_faults)
    foreach case in componentbatchreceipt:component_batch_receipt ///
        augmentcomponentv5:component_augmentation {
        gettoken fault phase : case, parse(":")
        local phase : subinstr local phase ":" ""
        global RM_HELPER_FAULT `fault'
        global RM_POSTER_RESULTS 0
        capture noisily fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
            engine(generic) preconditioner(cmg) inference(highrank) nodisplay
        assert _rc==498 & "`e(status)'"=="WITHHELD"
        assert "`e(withholding_status)'"=="STALE_CONTEXT"
        assert "`e(native_error_phase)'"=="`phase'"
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
        * The default reports the point estimates and says why.
        capture noisily fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
            engine(generic) preconditioner(cmg) inference(auto) nodisplay
        assert _rc==0 & "`e(status)'"=="KSS_POINT_ESTIMATES_ONLY"
        assert strpos("`e(inference_auto_reason)'","STALE_CONTEXT")>0
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
    foreach case in exact:release:STALE_CONTEXT:release ///
        exact:reexport:INTERNAL_INVARIANT_FAILED:exact_reexport ///
        compressed:release:STALE_CONTEXT:release {
        tokenize "`case'", parse(":")
        local route algorithm(exact)
        if "`1'"=="compressed" local route algorithm(jla) stayers(movers)
        global RM_HELPER_FAULT `3'
        global RM_POSTER_RESULTS 0
        capture noisily fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
            `route' inference(none) nodisplay
        assert _rc==498 & "`e(status)'"=="WITHHELD"
        assert "`e(withholding_status)'"=="`5'" & "`e(native_error_phase)'"=="`7'"
        quietly fevc_rust snapshot
        assert r(state)==0 & r(handle)==0
    }
    quietly log close rm_helper_faults
    macro drop RM_HELPER_FAULT RM_POSTER_RESULTS
    program drop fevc__rust_public_call
    * No helper fell through to Stata's report of a missing command.
    tempname stream
    file open `stream' using `"`helper_log'"', read text
    file read `stream' line
    while r(eof)==0 {
        assert !strpos(`"`macval(line)'"',"is unrecognized")
        file read `stream' line
    }
    file close `stream'
    * Without the hook both routes succeed.
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        algorithm(exact) inference(none) nodisplay
    assert "`e(result_family)'"=="exact"
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        algorithm(jla) stayers(movers) inference(none) nodisplay
    assert "`e(result_family)'"=="compressed"
}
else {
    * A plugin without the capability refuses a strict native request.
    capture quietly fevc_rust probe
    if !_rc {
        capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
            inference(highrank) inferencemodel(repeat_measurement) nodisplay
        assert _rc==498 & "`e(withholding_status)'"=="REPEAT_MEASUREMENT_TUPLE_REQUIRED"
    }
}

* Closed-form spectra test eigensolver scaling and a repeated leading root.
* The repeated worker root spans (1,1,0) and (0,0,1); its canonical axis is
* (0,0,1), independently of the eigenbasis returned by the Stata runtime.
mata:
void _rm_exact_spectrum_check()
{
    struct vckss_rse_leading scalar lead
    struct vckss_target_matrices scalar target
    real colvector u, v
    real matrix expected, action
    real rowvector roots
    real scalar scale, exponent, t
    u = (1\1\0):/sqrt(2)
    v = (0\0\1)
    expected = (v,u,v,u)
    roots = (4,2,-3,6)
    for (exponent=-12; exponent<=12; exponent=exponent+12) {
        scale = 10^exponent
        target.worker = scale*4*(u*u'+v*v')
        target.firm = scale*(2*u*u'+v*v')
        target.covariance = -scale*3*v*v'
        vckss_rse__lead_exact(lead,I(3),I(3),target)
        assert(!hasmissing(lead.mode) & !hasmissing(lead.lambda))
        assert(max(abs(lead.lambda:/scale-roots))<1e-12)
        for (t=1; t<=4; t++) {
            assert(abs(abs(quadcross(expected[.,t],lead.mode[.,t]))-1)<1e-12)
            if (t==1) action = target.worker
            else if (t==2) action = target.firm
            else if (t==3) action = target.covariance
            else action = target.worker+target.firm+2*target.covariance
            assert(max(abs(action*lead.mode[.,t]-lead.lambda[t]*lead.mode[.,t]))/scale<1e-12)
        }
    }
}
_rm_exact_spectrum_check()
end

* --- observation deletion ----------------------------------------------------
* Every physical observation is a unit. The panel gets matches of one, two
* and three observations: a match of two or more is a repeat-measurement set
* like a stayer, and the 66 single observations of movers receive the model,
* fitted on all observations of movers because there are fewer than a hundred.
preserve
bysort worker_id (firm_id log_wage): generate long seq = _n
quietly drop if worker_id<=200 & mod(worker_id,3)==0 & seq==2
quietly expand 2 if worker_id<=200 & mod(worker_id,3)==1 & seq==1, generate(extra)
quietly replace log_wage = log_wage+0.1*cos(worker_id) if extra==1
drop seq extra
* x2 again on this panel, from its own outcome, in this order of the rows.
capture drop x2 wbar2 rowid
generate long rowid = _n
egen double wbar2 = mean(log_wage), by(worker_id)
assert rowid==_n
generate double x2 = 0.5*wbar2 + 1.0*sin(rowid*1.3)
assert _N==1801
* Independent dense calculation on this panel; with x1 and x2 the first-order
* term of the estimated control coefficient (ctl_ and ctl2_), and what
* treating it as known gives (_old). Every unit is one observation, so only
* the projection of the influences and of the draws carries the term.
matrix observation_b = (2.154005266392971, 1.171084753616503, -.2944140523794802)
scalar observation_pair = 1.434113200955210e-04
matrix observation_se = (.2661415525311538, .2647469861814654, ///
    .1714761737879706, .2121325734000382)
matrix ctl_obs_se = (0.26516337401303758, 0.26378308298667968, ///
    0.17046407246971429, 0.21113007349620666)
matrix ctl2_obs_se = (0.33485270819677193, 0.26508977680739748, ///
    0.17300039288024369, 0.32432675925384258)
matrix ctl2_obs_se_old = (0.26327930247483966, 0.26508776581736709, ///
    0.17171029402904928, 0.20905462370690128)

quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(none) nodisplay
matrix observation_point = e(kss)
forvalues j = 1/3 {
    _rm_close observation_point[1,`j'] observation_b[1,`j'] 1e-12
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684
assert mreldif(observation_point,e(kss))<1e-12
assert e(inference_independent_units)==1801
_rm_close e(inference_pair_term) observation_pair 1e-12
matrix shape = e(inference_repeat)
* Unit mass is one throughout, so the two mass terms of the scale are
* spanned by the intercept; 1,735 observations lie in the 684 sets.
assert shape[1,1]==3 & shape[1,2]==6 & shape[1,6]==1735 & shape[1,7]==0
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * At most 0.4 per cent over five seeds.
    _rm_close ci[`j',2] observation_se[1,`j'] 0.0125
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 684
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * At most one per cent over five seeds.
    _rm_close ci[`j',2] observation_se[1,`j'] 0.03
}
* Jointly estimated controls: the point estimate keeps them in the model,
* the standard error adds the first-order term of their estimation to the
* fixed-offset geometry. 512 draws: within 0.4 per cent over five seeds;
* with x2 treating the coefficient as known misses the worker variance and
* the sum by 21 and 36 per cent.
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684 1
matrix ci = e(component_inference)
forvalues j = 1/4 {
    _rm_close ci[`j',2] ctl_obs_se[1,`j'] 0.0125
}
quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684 1
matrix ci = e(component_inference)
forvalues j = 1/4 {
    _rm_close ci[`j',2] ctl2_obs_se[1,`j'] 0.0125
}
foreach j in 1 4 {
    _rm_far ctl2_obs_se_old[1,`j'] ctl2_obs_se[1,`j'] 0.0125
    _rm_far ci[`j',2] ctl2_obs_se_old[1,`j'] 0.0125
}
* Linear solves: within 0.9 per cent over five seeds.
quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 684 1
matrix ci = e(component_inference)
forvalues j = 1/4 {
    _rm_close ci[`j',2] ctl2_obs_se[1,`j'] 0.03
}
foreach j in 1 4 {
    _rm_far ctl2_obs_se_old[1,`j'] ctl2_obs_se[1,`j'] 0.03
    _rm_far ci[`j',2] ctl2_obs_se_old[1,`j'] 0.03
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) stayers(movers) ///
    inference(highrank) inferencemodel(repeat_measurement) nodisplay
_rm_check 8 534
if `native' {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(highrank) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_check 64 684
    assert "`e(backend_selected)'"=="rust"
    assert e(inference_independent_units)==1801
    * Pair term within 6.3 per cent, standard errors within 0.8 per cent
    * over five seeds.
    _rm_close e(inference_pair_term) observation_pair 0.19
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        _rm_close ci[`j',2] observation_se[1,`j'] 0.025
    }
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(highrank) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_check 64 684 `native_control_term'
    assert "`e(backend_selected)'"=="rust"
    * x1 moves these standard errors by less than half a per cent.
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        _rm_close ci[`j',2] ctl_obs_se[1,`j'] 0.035
    }
    if `native_control_term' {
        * x1 within 1.0, x2 within 0.8 per cent over five seeds.
        quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(rust) ///
            probes(800) deletion(observation) inference(highrank) ///
            inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
        _rm_check 64 684 1
        matrix ci = e(component_inference)
        forvalues j = 1/4 {
            _rm_close ci[`j',2] ctl2_obs_se[1,`j'] 0.03
        }
        foreach j in 1 4 {
            _rm_far ci[`j',2] ctl2_obs_se_old[1,`j'] 0.03
        }
    }
}

* --- leading directions and the interval of every component -------------------
* Independent dense calculation on this panel with the exact trace term: for
* the three components and the sum, the eigenvalue of largest magnitude; for
* the three components the largest squared weight, |b1|, its leave-out and
* fitted variance, the remainder, the pieces of its variance, the curvature,
* the q = 1 interval, and the share of the estimated variance that the
* direction carries. All three shares exceed five per cent on this small
* panel, so inference(auto) reports the q = 1 interval for the components
* and the normal one for their sum, whose nonzero eigenvalues are all equal.
matrix lead_lambda = (2.764747519099893e-03, 2.764747519099904e-03, ///
    -2.340540535428360e-03, 5.552470849529855e-04)
matrix lead_maxw2 = (3.317225075494883e-02, 4.691459575587980e-02, 3.739885944690360e-02)
matrix lead_b1 = (7.556849042172990, 8.188902778793103, 8.090264394952563)
matrix lead_leaveout = (8.596657515974970, 9.388525476993957, 9.258234379572137)
matrix lead_fitted = (8.969314421092490, 10.24868364227036, 9.854093710147076)
matrix lead_theta_r = (2.019889272110159, 1.011642861121473, -.1628893814284416)
matrix lead_linear = (8.652239936067969e-02, 6.992683386816935e-02, 2.268292414510630e-02)
matrix lead_cross = (3.820173950070329e-02, 2.581775816109461e-02, 3.181949546760893e-02)
matrix lead_trace = (3.462343893481505e-02, 2.235954035676728e-02, 9.353234503921372e-03)
matrix lead_pair = (1.208094850991734e-04, 3.286589800970942e-05, 9.598040749308599e-06)
matrix lead_rr = (5.201976991096382e-02, 4.760015940941178e-02, 1.333928768193424e-02)
matrix lead_kappa = (.2177914191161388, .2599239257024820, .4009365085810335)
matrix lead_lb = (1.653285199845768, .6769868869404063, -.7498829119067346)
matrix lead_ub = (2.760783074426984, 1.799920016872941, 4.992164588497078e-03)
matrix lead_share = (.2037353921059615, .2768883086940278, .4444660757414823)

* Tolerances by route (see _rm_leading_check), about three times the largest
* deviation over five seeds; on the exact route all but the simulated pieces
* agree to rounding. Largest deviations, exact: trace 1.5%, remainder
* variance 1.0%, curvature 0.5%, bounds 0.3% of the width, share 0.8%;
* linear solves: eigenvalue 8e-7, squared weight 3e-4, outcome pieces 1.3%,
* covariance 0.005, pair term 20%, trace 3.6%, interval pieces 2.4%;
* native: eigenvalue 8e-7, squared weight 3e-4, outcome pieces 2.2%,
* covariance 6e-4, pair term 7.7%, trace 6.2%, interval pieces 1.8%.
matrix tol_exact = (5e-12, 1e-12, 1e-12, 1e-12, 1e-12, 0.045, 0.025)
matrix tol_jla = (2.5e-6, 1e-3, 0.04, 0.016, 0.6, 0.11, 0.075)
matrix tol_native = (2.5e-6, 1e-3, 0.07, 0.002, 0.25, 0.19, 0.055)
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(auto) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_leading_check auto 3 tol_exact
assert e(inference_leading_steps)==0
assert mreldif(observation_point,e(kss))<1e-12
* inference(q1) reports the q = 1 interval for the sum as well.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(q1) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_leading_check q1 4 tol_exact
* Linear solves: the search stops at a relative residual of 1e-3, and the
* leverages and target diagonals are projected.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) deletion(observation) inference(auto) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_leading_check auto 3 tol_jla
assert inrange(e(inference_leading_steps),8,160)
* With x1 and x2: the same calculation with the first-order term in the
* leading variance (fitted and leave-out, projected weights), the covariance
* with the remainder, and the remainder's variance; the decomposition and
* its leave-out variance (leaveout_identity) do not change.
matrix ctl_obs_lead_lambda = (0.0027647475190998925, 0.0027647475190999038, ///
    -0.0023405405354283595, 0.00055524708495298554)
matrix ctl_obs_lead_maxw2 = (0.033172250754948833, 0.0469145957558798, 0.037398859446903604)
matrix ctl_obs_lead_b1 = (7.5443432023618779, 8.1698780728456022, 8.0740638060823198)
matrix ctl_obs_lead_leaveout_identity = (8.6092370534917251, 9.395477430838346, 9.2686356452803196)
matrix ctl_obs_lead_fitted = (8.9609804867890155, 10.243944907304856, 9.8465430073528619)
matrix ctl_obs_lead_leading = (8.9609804867890155, 10.243944907304856, 9.8465430073528619)
matrix ctl_obs_lead_theta_r = (2.0207375931172011, 1.0098969159359408, -0.16124373885896645)
matrix ctl_obs_lead_cross = (0.037438228694466717, -0.024836596652986653, -0.030872257197418231)
matrix ctl_obs_lead_linear = (0.086653131782526358, 0.069675424188866908, 0.022505144518818312)
matrix ctl_obs_lead_trace = (0.035147454978355613, 0.022422241443367429, 0.0093779205452576148)
matrix ctl_obs_lead_pair = (0.00012069422757634819, 3.2823937850109978e-05, 9.583661044409271e-06)
matrix ctl_obs_lead_rr = (0.051626371031747095, 0.047286006683349591, 0.013136807634605107)
matrix ctl_obs_lead_kappa = (0.21840576643451731, 0.26065321802267083, 0.40363697144929783)
matrix ctl_obs_lead_lb = (1.6555603503614518, 0.6762359017412205, -0.74579810998255236)
matrix ctl_obs_lead_ub = (2.7591335989264136, 1.7953403411309983, 0.0051856440832022399)
matrix ctl_obs_lead_share = (0.20433093765962521, 0.27739637154380481, 0.44749857199057935)
matrix ctl2_obs_lead_lambda = (0.0027647475190998925, 0.0027647475190999038, ///
    -0.0023405405354283595, 0.00055524708495298554)
matrix ctl2_obs_lead_maxw2 = (0.033172250754948833, 0.0469145957558798, 0.037398859446903604)
matrix ctl2_obs_lead_b1 = (7.5941389079783903, 8.2338702745553878, 8.1325287346348194)
matrix ctl2_obs_lead_leaveout_identity = (8.6093962328047233, 9.4041156575434268, 9.2726340624675299)
matrix ctl2_obs_lead_fitted = (9.017793843195081, 10.311260569857339, 9.911906512631095)
matrix ctl2_obs_lead_leading = (9.017793843195081, 10.311260569857339, 9.911906512631095)
matrix ctl2_obs_lead_theta_r = (1.9342683821177755, 1.0108240705832325, -0.1668342958548428)
matrix ctl2_obs_lead_cross = (-0.019150292035662594, -0.023942841533376648, -0.034671195935566598)
matrix ctl2_obs_lead_linear = (0.13273416911429556, 0.069944175644603798, 0.022764097760245367)
matrix ctl2_obs_lead_trace = (0.034797604635880483, 0.02238396759148038, 0.0093667953280096323)
matrix ctl2_obs_lead_pair = (0.00012069209733364341, 3.3080830586851259e-05, 9.6897671264887019e-06)
matrix ctl2_obs_lead_rr = (0.098057256575748722, 0.047593288883710269, 0.013406992199362224)
matrix ctl2_obs_lead_kappa = (0.15927073466729716, 0.26150390704753301, 0.40254172172406377)
matrix ctl2_obs_lead_lb = (1.4237832419219936, 0.67765632556825306, -0.75964688701239136)
matrix ctl2_obs_lead_ub = (2.7914078900877124, 1.802683063696024, 0.0018382957509354703)
matrix ctl2_obs_lead_share = (0.13072700954483191, 0.28103173008402527, 0.44399646551344862)
foreach control in x1 x2 {
    local p = cond("`control'"=="x1","ctl_obs_lead_","ctl2_obs_lead_")
    quietly fevc log_wage `control', worker(worker_id) firm(firm_id) backend(mata) ///
        algorithm(exact) deletion(observation) inference(auto) ///
        inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
    _rm_leading_check auto 3 tol_exact `p'
    assert e(inference_nuisance_omitted)==0
    * The share of the leading direction in the trace: with controls both
    * belong to the control-adjusted noise, on every route alike.
    matrix share_`control' = e(inference_leading)
    quietly fevc log_wage `control', worker(worker_id) firm(firm_id) backend(mata) ///
        algorithm(jla) probes(800) deletion(observation) inference(auto) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_leading_check auto 3 tol_jla `p'
    _rm_share_check share_`control' 0.10
}
if `native' & `native_control_term' {
    * As without controls (the largest deviations are the same).
    quietly fevc log_wage x2, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(auto) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_leading_check auto 3 tol_native ctl2_obs_lead_
    assert e(inference_nuisance_omitted)==0
    _rm_share_check share_x2 0.10
}
if `native' {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(auto) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_leading_check auto 3 tol_native
    assert "`e(backend_selected)'"=="rust"
    assert inrange(e(inference_leading_steps),8,160)
    * The offset pass and the leading directions are counted apart.
    assert e(inference_offset_pass_solves)==0
    assert e(inference_solver_columns)==4*64+3+e(inference_leading_steps)+4+4*64
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(q1) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_Q1"
    assert e(inference_q1_components)==4 & e(inference_offset_pass_solves)==2*800
}
restore

* Match deletion, where the sets are the stayers: the same request on the
* first panel, on every route.
foreach route in "backend(mata) algorithm(exact)" "backend(mata) algorithm(jla) probes(800)" ///
    "backend(rust) probes(800)" {
    if strpos("`route'","rust") & !`native' continue
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) `route' ///
        inference(auto) inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    assert inlist("`e(status)'","FEVC_REPEAT_MEASUREMENT_Q1","FEVC_REPEAT_MEASUREMENT_SE")
    assert "`e(inference_leading_request)'"=="auto"
    matrix table = e(inference_intervals)
    matrix lead = e(inference_leading)
    assert e(inference_q1_components)==(table[1,7]==1)+(table[2,7]==1)+(table[3,7]==1)
    assert table[4,7]==0
    forvalues j = 1/4 {
        assert lead[`j',2]==1 & lead[`j',16]<1e-6
        assert table[`j',8]<table[`j',1] & table[`j',1]<table[`j',9]
        assert (table[`j',7]==1)==(table[`j',10]>.1) | `j'==4
    }
    capture confirm matrix e(V)
    assert (_rc==0)==(e(inference_q1_components)==0)
}

* --- a variance estimate that is not positive --------------------------------
* Two groups of eight firms joined by two movers, and outcomes that are
* almost pure noise: the high-rank variance estimate of the worker variance is
* not positive. With the directions that is not a failure: the component has
* no standard error and is reported with the q = 1 interval when that exists,
* and with no interval otherwise.
preserve
clear
quietly set obs 62
generate long worker = _n
generate byte group = cond(_n<=30,0,cond(_n<=60,1,2))
generate int first = cond(group==2,1+mod(_n,8),1+8*group+mod(_n,8))
generate int second = cond(group==2,9+mod(_n*3,8),1+8*group+mod(_n*5+3,8))
quietly replace second = 1+8*group+mod(second-8*group,8) if second==first & group<2
quietly expand 4
bysort worker: generate byte spell = (_n>2)
generate int firm = cond(spell,second,first)
set seed 4
generate double y = .1*sin(worker)+.05*cos(firm*1.3)+.3*rnormal()
foreach route in "backend(mata) algorithm(exact)" "backend(rust) probes(800)" {
    if strpos("`route'","rust") & !`native' continue
    quietly fevc y, worker(worker) firm(firm) `route' inference(auto) ///
        inferencemodel(repeat_measurement) inferencedraws(256) nodisplay
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_Q1"
    matrix table = e(inference_intervals)
    assert missing(table[1,2]) & table[1,7]==1 & table[1,16]==0
    assert table[1,8]<table[1,1] & table[1,1]<table[1,9]
    forvalues j = 2/3 {
        * A normal interval needs a standard error; without either interval
        * no bounds are reported.
        if table[`j',7]==0 assert !missing(table[`j',2])
        if missing(table[`j',7]) assert missing(table[`j',8]) & missing(table[`j',9])
    }
    assert table[4,7]==0 & !missing(table[4,2])
    capture confirm matrix e(V)
    assert _rc
}
restore

* --- what the model does not accept ------------------------------------------
capture quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    deletion(observation) nuisance(fixedoffset) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
assert _rc==498 & "`e(withholding_status)'"=="REPEAT_MEASUREMENT_TUPLE_REQUIRED"
capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    inference(none) inferencemodel(repeat_measurement) nodisplay
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_WITHOUT_INFERENCE"
* fevc 0.6.0 removed the other variance models and their tuning options. A
* request for one stops before any other check, whatever inference() says,
* and names the change; a name the command never had is invalid.
foreach model in leaveout_residual target_lowess structured_common Structured_Leverage {
    local name = lower("`model'")
    foreach inference in none auto highrank q1 {
        capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
            centering(unknown) inference(`inference') inferencemodel(`model') nodisplay
        assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_REMOVED"
        assert `"`e(withholding_detail)'"'=="inferencemodel(`name') was removed in fevc 0.6.0; fevc reports repeat-measurement standard errors by default (help fevc, Standard errors)"
    }
}
capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    inference(highrank) inferencemodel(unrestricted_kss) nodisplay
assert _rc==198 & "`e(withholding_status)'"=="INVALID_INFERENCE_MODEL"
foreach option in inferencesimulations(1000) inferencebins(16) inferencegramprobes(2048) {
    local name = substr("`option'",1,strpos("`option'","(")-1)
    capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
        inference(highrank) `option' nodisplay
    assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_OPTION_REMOVED"
    assert `"`e(withholding_detail)'"'=="`name'() applied only to the variance models removed in fevc 0.6.0"
}
* Standard errors of the components and a block projection are separate
* calls; the default leaves the components to point estimates instead (below).
foreach inference in highrank q1 {
    capture quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
        algorithm(exact) project(x1) projecteffect(firm) inference(`inference') nodisplay
    assert _rc==498 & "`e(withholding_status)'"=="COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED"
}

* --- the one model ------------------------------------------------------------
* inferencemodel(repeat_measurement) names the only model: inference(highrank)
* and inference(q1) alone give exactly the same numbers, element by element.
program define _rm_same_save
    foreach name in b V V_primitive kss results decomposition component_inference ///
        component_raw_covariance component_spectrum inference_intervals ///
        inference_leading inference_repeat inference_effective_rank ///
        inference_diagnostics q0_status component_inference_receipt ///
        component_unit_receipt {
        capture matrix drop same_`name'
        // A missing e() matrix would otherwise be copied as one missing value.
        capture confirm matrix e(`name')
        if !_rc matrix same_`name' = e(`name')
    }
    global RM_SAME_STATUS "`e(status)'"
end
program define _rm_same_check
    assert "`e(status)'"=="$RM_SAME_STATUS" & "`e(inference_model)'"=="repeat_measurement"
    tempname A
    foreach name in b V V_primitive kss results decomposition component_inference ///
        component_raw_covariance component_spectrum inference_intervals ///
        inference_leading inference_repeat inference_effective_rank ///
        inference_diagnostics q0_status component_inference_receipt ///
        component_unit_receipt {
        capture confirm matrix same_`name'
        local saved = (_rc==0)
        capture confirm matrix e(`name')
        assert `saved'==(_rc==0)
        if !`saved' continue
        matrix `A' = e(`name')
        assert rowsof(`A')==rowsof(same_`name') & colsof(`A')==colsof(same_`name')
        forvalues i = 1/`=rowsof(`A')' {
            forvalues j = 1/`=colsof(`A')' {
                assert `A'[`i',`j']==same_`name'[`i',`j']
            }
        }
    }
end
foreach route in "backend(mata) algorithm(exact)" "backend(rust) probes(800)" {
    if strpos("`route'","rust") & !`native' continue
    foreach inference in highrank q1 {
        quietly fevc log_wage x1, worker(worker_id) firm(firm_id) `route' ///
            inference(`inference') inferencemodel(repeat_measurement) ///
            inferencedraws(16) nodisplay
        _rm_same_save
        quietly fevc log_wage x1, worker(worker_id) firm(firm_id) `route' ///
            inference(`inference') inferencedraws(16) nodisplay
        _rm_same_check
        assert "`e(status)'"==cond("`inference'"=="q1", ///
            "FEVC_REPEAT_MEASUREMENT_Q1","FEVC_REPEAT_MEASUREMENT_SE")
        assert "`e(inference_leading_request)'"==cond("`inference'"=="q1","q1","")
    }
}
macro drop RM_SAME_STATUS

* --- the default ---------------------------------------------------------------
* inference(auto) without a model is the repeat-measurement model with one
* interval per component: the same numbers as the explicit request.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) inferencemodel(repeat_measurement) nodisplay
matrix explicit = e(inference_intervals)
local explicit_status "`e(status)'"
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) nodisplay
assert "`e(inference_model)'"=="repeat_measurement"
assert "`e(inference_variance_fit)'"=="repeat_measurement"
assert e(inference_auto)==1 & "`e(inference_auto_reason)'"==""
assert "`e(inference_leading_request)'"=="auto" & "`e(status)'"=="`explicit_status'"
matrix default = e(inference_intervals)
forvalues i = 1/4 {
    forvalues j = 1/16 {
        assert default[`i',`j']==explicit[`i',`j'] | ///
            (missing(default[`i',`j']) & missing(explicit[`i',`j']))
    }
}
* It carries the options the pass accepts.
foreach options in "deletion(observation)" "stayers(movers)" ///
    "nuisance(fixedoffset)" "centering(corrected)" "centering(none)" {
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) `options' ///
        inference(auto) nodisplay
    if "`e(backend_selected)'"=="mata" | `native' {
        assert "`e(inference_model)'"=="repeat_measurement"
        confirm matrix e(inference_intervals)
        matrix table = e(inference_intervals)
        forvalues j = 1/4 {
            assert inlist(table[`j',7],0,1) & table[`j',8]<table[`j',9]
        }
    }
    else assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""
}
* Frequency weights stand for repeated observations: the compressed data give
* the numbers of the expanded data (exact kernels; the q = 1 limits to the
* accuracy of their simulated critical value), under both deletion modes.
preserve
generate long row = _n
generate int frequency = 1+(mod(row,5)==0)
generate double target_mass = 1+mod(worker_id,3)
bysort worker_id firm_id (row): generate int spell = 1+(_n>1 & mod(worker_id,4)==0)
egen long match_id = group(worker_id firm_id spell)
sort row
foreach deletion in match observation {
    quietly fevc log_wage [fw=frequency], worker(worker_id) firm(firm_id) ///
        backend(mata) algorithm(exact) deletion(`deletion') inference(auto) nodisplay
    assert "`e(inference_model)'"=="repeat_measurement"
    matrix compressed = e(inference_intervals)
    tempfile weighted
    quietly save `weighted'
    quietly expand frequency
    quietly fevc log_wage, worker(worker_id) firm(firm_id) ///
        backend(mata) algorithm(exact) deletion(`deletion') inference(auto) nodisplay
    matrix expanded = e(inference_intervals)
    use `weighted', clear
    forvalues i = 1/4 {
        forvalues j = 1/9 {
            assert reldif(compressed[`i',`j'],expanded[`i',`j'])<1e-5 | ///
                (missing(compressed[`i',`j']) & missing(expanded[`i',`j']))
        }
    }
}
* Deletion IDs (two spells of a worker at one firm declared independent) and
* target weights: the default carries them, and the two backends, which share
* no code for the pass, agree to projection and simulation noise.
foreach options in "deletionid(match_id)" "targetweight(target_mass)" {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
        algorithm(exact) `options' inference(auto) inferencedraws(64) nodisplay
    assert "`e(inference_model)'"=="repeat_measurement"
    matrix exact = e(inference_intervals)
    forvalues j = 1/4 {
        assert inlist(exact[`j',7],0,1) & exact[`j',8]<exact[`j',9]
    }
    if `native' {
        quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
            probes(800) `options' inference(auto) inferencedraws(64) nodisplay
        assert "`e(inference_model)'"=="repeat_measurement"
        matrix native = e(inference_intervals)
        forvalues j = 1/4 {
            assert abs(native[`j',1]-exact[`j',1])<0.25*exact[`j',2]
            assert reldif(native[`j',2],exact[`j',2])<0.15
        }
    }
}
if `native' {
    * Natively, observation deletion needs unit frequencies: the default then
    * returns point estimates and says why.
    quietly fevc log_wage [fw=frequency], worker(worker_id) firm(firm_id) ///
        backend(rust) deletion(observation) inference(auto) nodisplay
    capture confirm matrix e(inference_intervals)
    assert _rc
    assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""
}
restore
* A request the pass cannot carry returns point estimates and says why.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) project(x1) projecteffect(firm) inference(auto) nodisplay
capture confirm matrix e(V)
assert _rc
capture confirm matrix e(inference_intervals)
assert _rc
assert e(inference_auto)==1 & "`e(inference_auto_reason)'"!=""
* So does inference(none) with a projection; the projection is unchanged.
matrix default_projection = e(projection_results)
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) project(x1) projecteffect(firm) inference(none) nodisplay
assert "`e(status)'"=="KSS_PROJECTION_INFERENCE"
assert mreldif(default_projection,e(projection_results))==0

display "PASS test_repeat_measurement_se.do"
