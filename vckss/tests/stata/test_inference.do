version 18.0
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

vckss y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) targetweight(target) nodisplay
matrix point = e(b)
capture matrix list e(V)
assert _rc != 0
assert "`e(inference)'" == "not implemented"

set seed 456789
local rng_before `"`c(rngstate)'"'
vckss y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) inference(highrank) ///
    inferencesimulations(100) inferenceseed(42) inferencebins(16) ///
    targetweight(target) nodisplay
local rng_after `"`c(rngstate)'"'
assert `"`rng_before'"' == `"`rng_after'"'
assert "`e(algorithm)'" == "exact"
assert "`e(backend_selected)'" == "mata"
assert "`e(rng_selected)'" == "stata"
assert "`e(version)'" == "0.5.0-alpha.1"
assert "`e(inference)'" == "highrank"
assert "`e(status)'" == "KSS_HIGHRANK_INFERENCE"
assert mreldif(point,e(b)) < 2e-12
assert rowsof(e(V)) == 4 & colsof(e(V)) == 4
assert rowsof(e(V_primitive)) == 3 & colsof(e(V_primitive)) == 3
assert abs(el(e(V),4,4) - (el(e(V_primitive),1,1) + ///
    el(e(V_primitive),2,2) + 4*el(e(V_primitive),3,3) + ///
    2*el(e(V_primitive),1,2) + 4*el(e(V_primitive),1,3) + ///
    4*el(e(V_primitive),2,3))) < 2e-12
assert rowsof(e(component_inference)) == 4
assert colsof(e(component_inference)) == 4
matrix highrank_V = e(V)

vckss y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) inference(highrank) ///
    inferencesimulations(100) inferenceseed(42) inferencebins(16) ///
    targetweight(target) nodisplay
assert mreldif(highrank_V,e(V)) < 1e-14

vckss y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) inference(q1) ///
    inferencesimulations(100) inferenceseed(42) inferencebins(16)
assert "`e(status)'" == "KSS_Q1_INFERENCE"
assert rowsof(e(q1_inference)) == 4
assert colsof(e(q1_inference)) == 17
forvalues row = 1/4 {
    assert el(e(q1_inference),`row',5) < el(e(q1_inference),`row',6)
    assert el(e(q1_inference),`row',8) > 0
    assert el(e(q1_inference),`row',8) <= 1
    assert el(e(q1_inference),`row',9) > 0
    assert el(e(q1_inference),`row',9) < 1
    assert el(e(q1_inference),`row',17) > 1.9
    assert el(e(q1_inference),`row',17) < 2.5
}

vckss y c1 c2, worker(worker) firm(firm) ///
    deletion(observation) project(z) projecteffect(worker) nodisplay
assert "`e(inference)'" == "none"
assert "`e(status)'" == "KSS_PROJECTION_INFERENCE"
capture matrix list e(V)
assert _rc != 0
assert rowsof(e(projection_V)) == 2

capture noisily vckss y, worker(worker) firm(firm) ///
    deletion(match) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "INFERENCE_DELETION_UNSUPPORTED"
capture noisily vckss y, worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "JLA_INFERENCE_UNSUPPORTED"
capture noisily vckss y, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "RUST_INFERENCE_UNSUPPORTED"
capture noisily vckss y, worker(worker) firm(firm) ///
    deletion(observation) rng(counter_v1) inference(highrank) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "COUNTER_INFERENCE_UNSUPPORTED"
generate byte copies = 2
capture noisily vckss y [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) inference(highrank) ///
    inferencesimulations(100) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "INFERENCE_FREQUENCY_UNSUPPORTED"
capture noisily vckss y [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) project(z) projecteffect(worker) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "INFERENCE_FREQUENCY_UNSUPPORTED"

clear
set obs 240
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker + floor(time/2),20)
generate double c1 = time - 2.5
generate double z = sin(worker/5) + cos(firm/3) + time/20
generate double noise = .25*sin((_n*17)/11) + .15*cos((_n*7)/13)
generate double y = 1 + .08*worker - .12*firm + .3*c1 + noise
generate double target = 1 + mod(time,3)/4

vckss y c1, worker(worker) firm(firm) deletion(observation) ///
    inference(highrank) inferencesimulations(200) inferenceseed(42) ///
    inferencebins(64) project(z) projecteffect(firm) nodisplay
assert "`e(status)'" == "KSS_HIGHRANK_AND_PROJECTION_INFERENCE"
assert rowsof(e(projection_results)) == 2
assert colsof(e(projection_results)) == 7
assert el(e(projection_results),1,3) == ///
    el(e(projection_results),1,1)/el(e(projection_results),1,2)
matrix projection_first = e(projection_results)
matrix projection_V_first = e(projection_V)

mata:
y = st_data(.,"y")
worker = st_data(.,"worker")
firm = st_data(.,"firm")
c1 = st_data(.,"c1")
z = st_data(.,"z")
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
sigma = (y:-mean(y)):*r:/(1:-h)
Z = (J(n,1,1),z)
loading = T'Z*invsym(Z'Z)
score = X*XXI*loading
oracle_b = loading'b
oracle_V = score'*(sigma:*score)
st_matrix("projection_b_oracle",oracle_b')
st_matrix("projection_V_oracle",oracle_V)
end
assert mreldif(e(projection_b),projection_b_oracle) < 1e-11
assert mreldif(e(projection_V),projection_V_oracle) < 1e-11

vckss y c1, worker(worker) firm(firm) deletion(observation) ///
    inference(highrank) inferencesimulations(200) inferenceseed(42) ///
    inferencebins(64) project(z) projecteffect(firm) nodisplay
assert mreldif(projection_first,e(projection_results)) < 1e-14
assert mreldif(projection_V_first,e(projection_V)) < 1e-14

vckss y c1, worker(worker) firm(firm) deletion(observation) ///
    targetweight(target) project(z) projecteffect(firm) ///
    projectweight(target) nodisplay
assert "`e(projection_weight)'" == "target"
assert rowsof(e(projection_results)) == 2

vckss y c1, worker(worker) firm(firm) deletion(observation) ///
    nuisance(fixedoffset) inference(highrank) ///
    inferencesimulations(100) inferenceseed(43) inferencebins(64) nodisplay
assert "`e(inference)'" == "highrank"

capture mata: vckss_inference__api_level()
assert _rc == 0
tempname critical
mata: st_numscalar("`critical'",vckss_inf__critical(.5,.95,100000,12345))
assert scalar(`critical') > 2.11 & scalar(`critical') < 2.17

di as result "PASS test_inference.do"
