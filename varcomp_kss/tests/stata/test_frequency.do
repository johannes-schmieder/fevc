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

varcomp_kss y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target) ///
    algorithm(exact) nodisplay
matrix weighted_plugin = e(plugin)
matrix weighted_correction = e(correction)
assert e(N_physical) == 32
assert abs(el(e(plugin),1,1) - .23071509091874573) < 2e-10
assert abs(el(e(correction),1,1) + .031321804369551273) < 2e-10

varcomp_kss y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target) ///
    algorithm(jla) probes(4000) batch(13) seed(20260818) ///
    tolerance(1e-12) nodisplay
matrix weighted_match_jla = e(results)
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(weighted_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

varcomp_kss y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
matrix observation_plugin = e(plugin)
matrix observation_correction = e(correction)
assert e(deletion_units) == 32

varcomp_kss y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(jla) ///
    probes(4000) batch(13) seed(20260819) tolerance(1e-12) nodisplay
matrix weighted_observation_jla = e(results)
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(observation_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

// Splitting only one stored row must preserve the same physical-copy stream.
preserve
generate long source_row = _n
expand 2 if source_row == 1, generate(split_copy)
replace freq = 1 if source_row == 1
replace target = target/2 if source_row == 1
varcomp_kss y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(match) deletionid(match) targetweight(target) ///
    algorithm(jla) probes(4000) batch(13) seed(20260818) ///
    tolerance(1e-12) nodisplay
assert mreldif(weighted_match_jla,e(results)) < 2e-9
varcomp_kss y c1 c2 [fw=freq], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(jla) ///
    probes(4000) batch(13) seed(20260819) tolerance(1e-12) nodisplay
assert mreldif(weighted_observation_jla,e(results)) < 2e-9
restore

expand freq
bysort worker firm match time noise: generate long copy = _n
bysort worker firm match time noise: generate long copies = _N
generate double expanded_target = target/copies
varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(expanded_target) algorithm(exact) nodisplay
assert mreldif(weighted_plugin,e(plugin)) < 2e-10
assert mreldif(weighted_correction,e(correction)) < 2e-9

varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(expanded_target) algorithm(jla) ///
    probes(4000) batch(13) seed(20260818) tolerance(1e-12) nodisplay
assert mreldif(weighted_match_jla,e(results)) < 2e-9
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(weighted_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(observation) ///
    targetweight(expanded_target) algorithm(exact) nodisplay
assert mreldif(observation_plugin,e(plugin)) < 2e-10
assert mreldif(observation_correction,e(correction)) < 2e-9

varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(observation) ///
    targetweight(expanded_target) algorithm(jla) probes(4000) batch(13) ///
    seed(20260819) tolerance(1e-12) nodisplay
assert mreldif(weighted_observation_jla,e(results)) < 2e-9
forvalues target_index = 1/4 {
    assert abs(el(e(correction),1,`target_index') - ///
        el(observation_correction,1,`target_index')) < ///
        6*el(e(numerical_mcse),1,`target_index') + 4e-4
}

// Minimal accepted R=2 stream: a frequency-two row and its two literal
// copies must consume the same physical signs even when another per-copy
// target class shares the worker--firm cell and outcome.
clear
input double(y worker firm frequency target)
1 1 1 1 4
1 1 1 2 2
2 1 2 1 1
3 2 1 1 1
5 2 2 1 1
end
varcomp_kss y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
matrix six_copy_exact = e(results)
varcomp_kss y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(jla) ///
    probes(2) batch(1) seed(2) tolerance(1e-12) nodisplay
matrix six_copy_jla = e(results)
expand frequency
generate double split_target = target/frequency
replace frequency = 1
replace target = split_target
varcomp_kss y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
assert mreldif(six_copy_exact,e(results)) < 2e-12
varcomp_kss y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(jla) ///
    probes(2) batch(1) seed(2) tolerance(1e-12) nodisplay
assert mreldif(six_copy_jla,e(results)) < 2e-12

// Stored rows need not outnumber parameters when literal copies provide
// residual degrees of freedom and every physical-copy deletion retains rank.
clear
input double(y worker firm interaction frequency)
1 1 1 0 2
2 1 2 0 2
3 2 1 0 2
5 2 2 1 2
end
varcomp_kss y interaction [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
assert e(N_stored) == e(parameters)
assert e(N_physical) == 8
assert e(deletion_units) == 8
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"

di as result "PASS test_frequency.do"
