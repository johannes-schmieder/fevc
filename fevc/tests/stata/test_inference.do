version 15.0
clear all
set more off

set obs 24
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time - 1.5
generate double c2 = time == 2
generate byte firm = .
generate double noise = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise
generate double z = worker + time/10
generate double target = 1 + time/10

// Component standard errors are repeat-measurement standard errors, the only
// method since fevc 0.6.0, here on the exact Mata route.
local exact backend(mata) algorithm(exact)
fevc y c1 c2, worker(worker) firm(firm) `exact' ///
    deletion(observation) centering(none) targetweight(target) inference(none) nodisplay
matrix point = e(b)
capture matrix list e(V)
assert _rc != 0
assert "`e(inference)'" == "not implemented"

set seed 456789
local rng_before `"`c(rngstate)'"'
fevc y c1 c2, worker(worker) firm(firm) `exact' ///
    deletion(observation) centering(none) inference(highrank) ///
    inferenceseed(42) targetweight(target) nodisplay
local rng_after `"`c(rngstate)'"'
assert `"`rng_before'"' == `"`rng_after'"'
assert "`e(algorithm)'" == "exact"
assert "`e(backend_selected)'" == "mata"
assert "`e(rng_selected)'" == "stata"
assert "`e(version)'" == "0.6.0"
assert "`e(inference)'" == "highrank"
assert "`e(inference_model)'" == "repeat_measurement"
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_SE"
tempname compat_assert_b
matrix `compat_assert_b' = e(b)
assert mreldif(point,`compat_assert_b') < 2e-12
matrix drop `compat_assert_b'
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert rowsof(`compat_assert_V') == 4 & colsof(`compat_assert_V') == 4
matrix drop `compat_assert_V'
assert rowsof(e(V_primitive)) == 3 & colsof(e(V_primitive)) == 3
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert reldif(el(`compat_assert_V',4,4), el(e(V_primitive),1,1) + ///
    el(e(V_primitive),2,2) + 4*el(e(V_primitive),3,3) + ///
    2*el(e(V_primitive),1,2) + 4*el(e(V_primitive),1,3) + ///
    4*el(e(V_primitive),2,3)) < 1e-10
matrix drop `compat_assert_V'
assert rowsof(e(component_inference)) == 4
assert colsof(e(component_inference)) == 4
matrix highrank_V = e(V)

fevc y c1 c2, worker(worker) firm(firm) `exact' ///
    deletion(observation) centering(none) inference(highrank) ///
    inferenceseed(42) targetweight(target) nodisplay
tempname compat_assert_V
matrix `compat_assert_V' = e(V)
assert mreldif(highrank_V,`compat_assert_V') == 0
matrix drop `compat_assert_V'

// inference(q1): the q = 1 interval of every component that has one, from
// the leading direction of the component.
fevc y c1 c2, worker(worker) firm(firm) `exact' ///
    deletion(observation) centering(none) inference(q1) inferenceseed(42)
assert "`e(status)'" == "FEVC_REPEAT_MEASUREMENT_Q1"
assert "`e(inference_leading_request)'" == "q1"
assert rowsof(e(inference_intervals)) == 4 & colsof(e(inference_intervals)) == 16
assert rowsof(e(inference_leading)) == 4 & colsof(e(inference_leading)) == 28
capture matrix list e(V)
assert _rc != 0
matrix q1_unit_reference = e(inference_intervals)
forvalues row = 1/4 {
    assert inlist(q1_unit_reference[`row',7],0,1)
    assert q1_unit_reference[`row',8] < q1_unit_reference[`row',9]
}

// The estimates and the interval bounds scale with the square of the
// outcome; which interval a component receives, and the share behind that
// choice, do not change.
generate double y_rescaled = y
foreach scale in .01 .1 1 10 100 {
    quietly replace y_rescaled = `scale'*y
    quietly fevc y_rescaled c1 c2, worker(worker) firm(firm) `exact' ///
        deletion(observation) centering(none) inference(q1) ///
        inferenceseed(42) nodisplay
    matrix rescaled = e(inference_intervals)
    forvalues row = 1/4 {
        assert rescaled[`row',7] == q1_unit_reference[`row',7]
        assert reldif(rescaled[`row',10],q1_unit_reference[`row',10]) < 1e-9 | ///
            (missing(rescaled[`row',10]) & missing(q1_unit_reference[`row',10]))
        foreach column in 1 8 9 {
            assert reldif(rescaled[`row',`column']/(`scale'^2), ///
                q1_unit_reference[`row',`column']) < 1e-8
        }
    }
}

fevc y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) centering(none) project(z) projecteffect(worker) nodisplay
assert "`e(inference)'" == "none"
assert "`e(status)'" == "KSS_PROJECTION_INFERENCE"
capture matrix list e(V)
assert _rc != 0
assert rowsof(e(projection_V)) == 2

// What the repeat-measurement model does not carry stops before any work.
capture noisily fevc y c1, worker(worker) firm(firm) backend(mata) ///
    deletion(observation) nuisance(fixedoffset) centering(none) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "REPEAT_MEASUREMENT_TUPLE_REQUIRED"
capture noisily fevc y, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) algorithm(exact) centering(none) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "REPEAT_MEASUREMENT_TUPLE_REQUIRED"
capture noisily fevc y, worker(worker) firm(firm) ///
    deletion(observation) rng(counter_v1) algorithm(exact) centering(none) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "REPEAT_MEASUREMENT_TUPLE_REQUIRED"
generate byte copies = 2
capture noisily fevc y [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) backend(rust) centering(none) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "REPEAT_MEASUREMENT_TUPLE_REQUIRED"
// fevc 0.6.0 removed the other variance models and their tuning options.
capture noisily fevc y, worker(worker) firm(firm) ///
    deletion(observation) inference(highrank) inferencemodel(target_lowess) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INFERENCE_MODEL_REMOVED"
capture noisily fevc y, worker(worker) firm(firm) ///
    deletion(observation) inference(highrank) inferencesimulations(100) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INFERENCE_OPTION_REMOVED"
generate double ycentered = y - 1.5
quietly fevc ycentered [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) centering(none) project(z) projecteffect(worker) nodisplay
assert "`e(status)'" == "KSS_PROJECTION_INFERENCE"
assert rowsof(e(projection_V)) == 2

clear
set obs 240
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker + floor(time/2),20)
generate double c1 = time - 2.5
generate double z = sin(worker/5) + cos(firm/3) + time/20
generate double noise = .25*sin((_n*17)/11) + .15*cos((_n*7)/13)
generate double y = 1 + .08*worker - .12*firm + .3*c1 + noise
generate double yproj = y - 1
generate double ymatch = .08*worker - .12*firm + .3*c1 + ///
    .5*sin(2*_n/11) + .3*cos(2*_n/13)
generate double target = 1 + mod(time,3)/4

// Standard errors of the components and a block projection are separate calls.
capture noisily fevc yproj c1, worker(worker) firm(firm) deletion(observation) ///
    centering(none) inference(highrank) project(z) projecteffect(firm) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "COMPONENT_PROJECTION_COMBINATION_UNSUPPORTED"
fevc yproj c1, worker(worker) firm(firm) deletion(observation) ///
    centering(none) inference(none) project(z) projecteffect(firm) nodisplay
assert "`e(status)'" == "KSS_PROJECTION_INFERENCE"
assert rowsof(e(projection_results)) == 2
assert colsof(e(projection_results)) == 7
assert el(e(projection_results),1,3) == ///
    el(e(projection_results),1,1)/el(e(projection_results),1,2)
matrix projection_first = e(projection_results)
matrix projection_V_first = e(projection_V)

mata:
y = st_data(.,"yproj")
worker = st_data(.,"worker")
firm = st_data(.,"firm")
c1 = st_data(.,"c1")
z = st_data(.,"z")
ymatch = st_data(.,"ymatch")
time = st_data(.,"time")
n = rows(y)
X = J(n,40+19+1,0)
T = J(n,40+19+1,0)
for (i=1; i<=n; i++) {
    X[i,worker[i]+1] = 1
    if (firm[i] < 19) {
        X[i,40+firm[i]+1] = 1
        T[i,40+firm[i]+1] = 1
    }
    X[i,60] = c1[i]
}
XXI = invsym(X'X)
b = XXI*(X'y)
r = y-X*b
h = rowsum((X*XXI):*X)
sigma = y:*r:/(1:-h)
Z = (J(n,1,1),z)
loading = T'Z*invsym(Z'Z)
score = X*XXI*loading
oracle_b = loading'b
oracle_V = score'*(sigma:*score)
bm = XXI*(X'ymatch)
rm = ymatch-X*bm
oracle_match_V = J(2,2,0)
for (wi=0; wi<40; wi++) {
    for (mi=0; mi<3; mi++) {
        use = selectindex((worker:==wi):&(floor(time:/2):==mi))
        keep = selectindex(!((worker:==wi):&(floor(time:/2):==mi)))
        deleted_b = invsym(X[keep,.]'*X[keep,.])*(X[keep,.]'*ymatch[keep])
        deleted_e = ymatch[use]-X[use,.]*deleted_b
        score_y = score[use,.]'*ymatch[use]
        score_e = score[use,.]'*deleted_e
        oracle_match_V = oracle_match_V + .5:*(
            score_y*score_e'+score_e*score_y')
    }
}
oracle_match_b = loading'bm
st_matrix("projection_b_oracle",oracle_b')
st_matrix("projection_V_oracle",oracle_V)
st_matrix("projection_match_b_oracle",oracle_match_b')
st_matrix("projection_match_V_oracle",oracle_match_V)
end
assert mreldif(e(projection_b),projection_b_oracle) < 1e-11
assert mreldif(e(projection_V),projection_V_oracle) < 1e-11

fevc ymatch c1, worker(worker) firm(firm) ///
    centering(none) project(z) projecteffect(firm) nodisplay
assert "`e(deletion)'" == "match"
assert "`e(stayers)'" == "both"
assert "`e(inference_deletion)'" == "exact match deletion; stayers both"
assert mreldif(e(projection_b),projection_match_b_oracle) < 1e-11
assert mreldif(e(projection_V),projection_match_V_oracle) < 2e-10

fevc yproj c1, worker(worker) firm(firm) deletion(observation) ///
    centering(none) inference(none) project(z) projecteffect(firm) nodisplay
assert mreldif(projection_first,e(projection_results)) < 1e-14
assert mreldif(projection_V_first,e(projection_V)) < 1e-14

fevc yproj c1, worker(worker) firm(firm) deletion(observation) ///
    targetweight(target) centering(none) project(z) projecteffect(firm) ///
    projectweight(target) nodisplay
assert "`e(projection_weight)'" == "target"
assert rowsof(e(projection_results)) == 2

fevc y c1, worker(worker) firm(firm) deletion(match) `exact' ///
    nuisance(fixedoffset) centering(none) inference(highrank) inferenceseed(43) nodisplay
assert "`e(inference)'" == "highrank"
assert "`e(inference_model)'" == "repeat_measurement"

capture mata: vckss_inference__api_level()
assert _rc == 0
mata: assert(vckss_inference__api_level() == 2)
mata: assert(vckss_inference__build_id() == "vckss-inference-api2-q1-target-status-projection-mean1-component-mean1-mixed1")
tempname critical
mata: st_numscalar("`critical'",vckss_inf__critical(.5,.95,100000,12345))
assert scalar(`critical') > 2.11 & scalar(`critical') < 2.17

di as result "PASS test_inference.do"
