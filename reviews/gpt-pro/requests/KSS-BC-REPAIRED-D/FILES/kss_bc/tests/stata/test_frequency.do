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
generate long freq = cond(mod(_n-1,3)==0,2,1)
generate double target = .5 + (_n-1)/24

kss_bc y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target) ///
    algorithm(exact) nodisplay
matrix weighted_plugin = e(plugin)
matrix weighted_correction = e(correction)
assert e(N_physical) == 32
assert abs(el(e(plugin),1,1) - .23071509091874573) < 2e-10
assert abs(el(e(correction),1,1) + .031321804369551273) < 2e-10

kss_bc y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target) ///
    algorithm(jla) probes(4000) batch(13) seed(20260818) ///
    tolerance(1e-12) nodisplay
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(weighted_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

kss_bc y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
matrix observation_plugin = e(plugin)
matrix observation_correction = e(correction)
assert e(deletion_units) == 32

kss_bc y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(jla) ///
    probes(4000) batch(13) seed(20260819) tolerance(1e-12) nodisplay
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(observation_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

expand freq
bysort worker firm match time noise: generate long copy = _n
bysort worker firm match time noise: generate long copies = _N
generate double expanded_target = target/copies
kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(expanded_target) algorithm(exact) nodisplay
assert mreldif(weighted_plugin,e(plugin)) < 2e-10
assert mreldif(weighted_correction,e(correction)) < 2e-9

kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(expanded_target) algorithm(jla) ///
    probes(4000) batch(13) seed(20260818) tolerance(1e-12) nodisplay
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(weighted_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

kss_bc y c1 c2, worker(worker) firm(firm) deletion(observation) ///
    targetweight(expanded_target) algorithm(exact) nodisplay
assert mreldif(observation_plugin,e(plugin)) < 2e-10
assert mreldif(observation_correction,e(correction)) < 2e-9

kss_bc y c1 c2, worker(worker) firm(firm) deletion(observation) ///
    targetweight(expanded_target) algorithm(jla) probes(4000) batch(13) ///
    seed(20260819) tolerance(1e-12) nodisplay
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(observation_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

di as result "PASS test_frequency.do"
