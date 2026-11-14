* Repeat-measurement standard errors on the Mata routes: the sample variance
* of every repeat-measurement set (an eligible stayer's observations; under
* observation deletion also a mover's match of two or more observations),
* the two-step model for the remaining units, and the high-rank covariance
* with the pair term of the sets. Expected values come from an independent
* dense calculation in Python on the same deterministic panels (exact trace
* term); the command simulates that term, so standard errors agree to its
* Monte Carlo error. Every fevc call names inference().
version 18.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

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

* Independent dense calculation: estimates, pair term, standard errors.
matrix expected_b = (2.193981373967747, 1.014791107858934, -.2004904720705475)
scalar expected_pair = 1.478986773550021e-06
matrix expected_se = (.2595212617894884, .2433340968591373, ///
    .1263905520856996, .2676009330004356)

program define _rm_check
    args draws sets
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_model)'"=="repeat_measurement"
    assert "`e(inference_variance_fit)'"=="repeat_measurement"
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_SE"
    assert e(inference_draws)==`draws'
    assert e(inference_repeat_sets)==`sets'
    * The estimated offset is held fixed, with or without controls.
    assert e(inference_nuisance_omitted)==1
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
    * inference(highrank) computes no leading direction.
    capture confirm matrix e(inference_intervals)
    assert _rc
    capture confirm matrix e(inference_leading)
    assert _rc
end

* Leading directions and the interval reported for every component, against
* the independent dense calculation in the lead_* matrices. design: tolerance
* of the eigenvalue and of the largest squared weight; value: of what the
* outcome and the unit variances determine; simulated: of what also rests on
* the pseudo-noise trace.
program define _rm_leading_check
    args request q1_components design value simulated
    assert "`e(status)'"=="FEVC_REPEAT_MEASUREMENT_Q1"
    assert "`e(inference)'"=="highrank"
    assert "`e(inference_leading_request)'"=="`request'"
    assert e(inference_q1_components)==`q1_components'
    assert e(inference_share_threshold)==.05
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
        assert reldif(`lead'[`j',5],lead_lambda[1,`j'])<`design'
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
    forvalues j = 1/3 {
        assert reldif(`lead'[`j',9],lead_maxw2[1,`j'])<max(`design',1e-7)*10
        assert reldif(abs(`lead'[`j',11]),lead_b1[1,`j'])<`value'
        assert reldif(`lead'[`j',12],lead_leaveout[1,`j'])<`value'
        assert reldif(`lead'[`j',13],lead_fitted[1,`j'])<`value'
        assert reldif(`lead'[`j',14],max(`lead'[`j',12],`lead'[`j',13]))<1e-12
        assert reldif(`lead'[`j',15],lead_theta_r[1,`j'])<`value'
        assert reldif(`lead'[`j',19],lead_linear[1,`j'])<`value'
        assert reldif(abs(`lead'[`j',17]),lead_cross[1,`j'])<`value'
        assert reldif(`lead'[`j',22],lead_pair[1,`j'])<max(`value',1e-6)
        assert reldif(`lead'[`j',20],lead_trace[1,`j'])<`simulated'
        assert reldif(`lead'[`j',18],lead_rr[1,`j'])<`simulated'
        assert reldif(`lead'[`j',24],lead_kappa[1,`j'])<`simulated'
        local width = lead_ub[1,`j']-lead_lb[1,`j']
        assert abs(`table'[`j',5]-lead_lb[1,`j'])<`simulated'*`width'
        assert abs(`table'[`j',6]-lead_ub[1,`j'])<`simulated'*`width'
        assert reldif(`table'[`j',10],lead_share[1,`j'])<2*`simulated'
    }
end

* --- point estimates are not touched -----------------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(none) nodisplay
matrix point = e(kss)
forvalues j = 1/3 {
    assert reldif(point[1,`j'],expected_b[1,`j'])<1e-10
}

* --- exact kernels: the independent calculation ------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(512) nodisplay
_rm_check 512 150
assert mreldif(point,e(kss))<1e-12
assert e(inference_independent_units)==1200
assert reldif(e(inference_pair_term),expected_pair)<1e-8
matrix shape = e(inference_repeat)
* Every match has two observations here, so the two mass terms of the scale
* are spanned by the intercept: three active scale terms, six profile terms.
assert shape[1,1]==3 & shape[1,2]==6 & shape[1,6]==600 & shape[1,7]==0
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * 512 draws: the simulated trace term is within a fraction of a per cent.
    assert reldif(ci[`j',2],expected_se[1,`j'])<0.01
}
* More draws move only the simulated part.
matrix b512 = e(b)
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) ///
    inferencedraws(64) nodisplay
_rm_check 64 150
assert mreldif(b512,e(b))<1e-12
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],expected_se[1,`j'])<0.05
}

* --- linear solves and projected leverages -----------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 150
matrix ci = e(component_inference)
forvalues j = 1/4 {
    * Projection noise in leverages and target diagonals on top of the draws.
    assert reldif(ci[`j',2],expected_se[1,`j'])<0.15
}

* --- controls, the mover-only population, centering --------------------------
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(highrank) inferencemodel(repeat_measurement) nodisplay
_rm_check 8 150
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) nuisance(fixedoffset) inference(highrank) ///
    inferencemodel(repeat_measurement) nodisplay
_rm_check 8 150
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
    * Projected target diagonals: the pair term agrees to projection noise.
    assert reldif(e(inference_pair_term),expected_pair)<0.2
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        assert reldif(ci[`j',2],expected_se[1,`j'])<0.15
    }
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(highrank) inferencemodel(repeat_measurement) nodisplay
    _rm_check 8 150
    * The default on the same backend is still the leave-out residual method.
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        inference(auto) nodisplay
    assert "`e(inference_variance_fit)'"=="leaveout_residual"
    assert "`e(backend_selected)'"=="rust"
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
assert _N==1801
* Independent dense calculation on this panel; with x1 the estimated control
* index of the full regression is held fixed.
matrix observation_b = (2.154005266392971, 1.171084753616503, -.2944140523794802)
scalar observation_pair = 1.434113200955210e-04
matrix observation_se = (.2661415525311538, .2647469861814654, ///
    .1714761737879706, .2121325734000382)
matrix observation_se_x1 = (.2660014404422780, .2644807941651816, ///
    .1712614183010554, .2121175684510716)

quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(none) nodisplay
matrix observation_point = e(kss)
forvalues j = 1/3 {
    assert reldif(observation_point[1,`j'],observation_b[1,`j'])<1e-10
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684
assert mreldif(observation_point,e(kss))<1e-12
assert e(inference_independent_units)==1801
assert reldif(e(inference_pair_term),observation_pair)<1e-8
matrix shape = e(inference_repeat)
* Unit mass is one throughout, so the two mass terms of the scale are
* spanned by the intercept; 1,735 observations lie in the 684 sets.
assert shape[1,1]==3 & shape[1,2]==6 & shape[1,6]==1735 & shape[1,7]==0
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],observation_se[1,`j'])<0.01
}
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_check 64 684
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],observation_se[1,`j'])<0.15
}
* Jointly estimated controls: the point estimate keeps them in the model,
* the standard error holds their estimated index fixed.
quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(highrank) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_check 512 684
matrix ci = e(component_inference)
forvalues j = 1/4 {
    assert reldif(ci[`j',2],observation_se_x1[1,`j'])<0.01
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
    assert reldif(e(inference_pair_term),observation_pair)<0.2
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        assert reldif(ci[`j',2],observation_se[1,`j'])<0.15
    }
    quietly fevc log_wage x1, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(highrank) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_check 64 684
    assert "`e(backend_selected)'"=="rust"
    matrix ci = e(component_inference)
    forvalues j = 1/4 {
        assert reldif(ci[`j',2],observation_se_x1[1,`j'])<0.15
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

quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(auto) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_leading_check auto 3 1e-9 1e-7 0.03
assert e(inference_leading_steps)==0
assert mreldif(observation_point,e(kss))<1e-12
* inference(q1) reports the q = 1 interval for the sum as well.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) deletion(observation) inference(q1) ///
    inferencemodel(repeat_measurement) inferencedraws(512) nodisplay
_rm_leading_check q1 4 1e-9 1e-7 0.03
* Linear solves: the search stops at a relative residual of 1e-3, and the
* leverages and target diagonals are projected.
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(jla) probes(800) deletion(observation) inference(auto) ///
    inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
_rm_leading_check auto 3 1e-5 0.15 0.2
assert inrange(e(inference_leading_steps),8,160)
if `native' {
    quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(rust) ///
        probes(800) deletion(observation) inference(auto) ///
        inferencemodel(repeat_measurement) inferencedraws(64) nodisplay
    _rm_leading_check auto 3 1e-5 0.15 0.2
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
        assert (table[`j',7]==1)==(table[`j',10]>.05) | `j'==4
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
    inference(auto) inferencemodel(structured_common) nodisplay
assert _rc==198 & "`e(withholding_status)'"=="INFERENCE_MODEL_WITHOUT_INFERENCE"

* --- the default is unchanged ------------------------------------------------
quietly fevc log_wage, worker(worker_id) firm(firm_id) backend(mata) ///
    algorithm(exact) inference(auto) nodisplay
assert "`e(inference_variance_fit)'"=="leaveout_residual"
assert "`e(status)'"=="FEVC_LEAVEOUT_RESIDUAL_SE"
capture confirm matrix e(inference_repeat)
assert _rc

display "PASS test_repeat_measurement_se.do"
