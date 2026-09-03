version 18.0
clear all
set more off
set varabbrev off

set obs 24
generate long worker = floor((_n-1)/4)
generate byte period = mod(_n-1,4)
generate double c1 = period-1.5
generate double c2 = period==2
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
generate double y = 1.5+.3*worker-.2*firm+.4*c1-.15*c2+noise

fevc y c1 c2, worker(worker) firm(firm)                 ///
    deletion(observation) algorithm(exact) nodisplay
assert "`e(estat_cmd)'" == "fevc_estat"
matrix b_before = e(b)
matrix decomposition_before = e(decomposition)

estat decomposition
assert mreldif(b_before,e(b)) == 0
estat decomposition, full
assert mreldif(decomposition_before,e(decomposition)) == 0
estat sample
estat computation
estat diagnostics
estat dec
estat sam
estat com
estat dia
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"

capture noisily estat sample, full
assert _rc == 198
capture noisily estat unknown
assert _rc == 198

ereturn clear
capture noisily estat decomposition
assert _rc == 301

fevc, version
assert "`e(estat_cmd)'" == ""
capture noisily fevc_estat decomposition
assert _rc == 301

di as result "PASS test_postestimation.do"
