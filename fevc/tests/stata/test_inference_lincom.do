version 18.0
clear all
set more off

set obs 24
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
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
generate double y = 1.5 + .3*worker - .2*firm + noise

fevc y, worker(worker) firm(firm) deletion(observation) ///
    inference(highrank) inferencesimulations(100) ///
    inferenceseed(42) inferencebins(16) nodisplay
matrix posted_b = e(b)
matrix posted_V = e(V)

lincom worker_variance + firm_variance + 2*worker_firm_covariance
assert abs(r(estimate) - el(posted_b,1,4)) < 2e-12
assert abs(r(se) - sqrt(el(posted_V,4,4))) < 2e-12

di as result "PASS test_inference_lincom.do"
