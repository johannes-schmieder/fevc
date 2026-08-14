version 18.0
clear
set obs 24

generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate double c1 = time - 1.5
generate double c2 = time == 2
generate byte firm = .
generate long match = .
generate double noise = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
local noises .2 -.1 .1 -.2 -.2 .3 -.1 .1 .1 -.2 .2 -.1 -.1 .2 -.2 .1 .3 -.2 .1 -.2 -.2 .1 .2 -.1
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
    local value : word `row' of `noises'
    quietly replace noise = `value' in `row'
}
generate double y = 1.5 + .3*worker - .2*firm + .4*c1 - .15*c2 + noise

kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nuisance(joint) nodisplay
matrix exact_correction = e(correction)

scalar low_error = 0
scalar high_error = 0
scalar low_mcse = 0
scalar high_mcse = 0
forvalues trial = 1/6 {
    local trial_seed = 20260900 + `trial'
    kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) algorithm(jla) nuisance(joint) ///
        probes(50) batch(7) seed(`trial_seed') tolerance(1e-12) nodisplay
    scalar low_error = low_error + ///
        (el(e(correction),1,4)-el(exact_correction,1,4))^2
    scalar low_mcse = low_mcse + el(e(numerical_mcse),1,4)

    kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) algorithm(jla) nuisance(joint) ///
        probes(800) batch(19) seed(`trial_seed') tolerance(1e-12) nodisplay
    scalar high_error = high_error + ///
        (el(e(correction),1,4)-el(exact_correction,1,4))^2
    scalar high_mcse = high_mcse + el(e(numerical_mcse),1,4)
}
assert high_error < low_error
assert high_mcse < .4*low_mcse

di as result "PASS test_jla_convergence.do"
