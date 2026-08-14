version 18.0
clear
set obs 24

generate long obsid = _n
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

// Numeric, string, and reversed-value encodings must define the same result.
kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) seed(1) nodisplay
matrix reference = e(results)
scalar reference_rss = e(weighted_rss)
scalar reference_units = e(deletion_units)

generate long split_match = match
replace split_match = 99 in 2
kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(split_match) algorithm(exact) nodisplay
assert e(deletion_units) == reference_units+1
assert mreldif(reference,e(results)) > 1e-7

generate str8 worker_s = "w" + string(worker,"%02.0f")
generate str8 firm_s = "f" + string(firm,"%02.0f")
generate str12 match_s = "m" + string(match,"%08.0f")
kss_bc y c1 c2, worker(worker_s) firm(firm_s) deletion(match) ///
    deletionid(match_s) algorithm(exact) seed(999) nodisplay
assert mreldif(reference,e(results)) < 2e-12
assert abs(reference_rss-e(weighted_rss)) < 2e-12

generate long worker_r = 1000 - 17*worker
generate long firm_r = 700 + 13*(3-firm)
generate long match_r = 100000 - 7*match
kss_bc y c1 c2, worker(worker_r) firm(firm_r) deletion(match) ///
    deletionid(match_r) algorithm(exact) nodisplay
assert mreldif(reference,e(results)) < 2e-11

// Exact mode is invariant to row order and to the seed.
gsort -obsid
kss_bc y c1 c2, worker(worker_s) firm(firm_s) deletion(match) ///
    deletionid(match_s) algorithm(exact) seed(42) nodisplay
assert mreldif(reference,e(results)) < 2e-12
assert obsid == 25-_n

// Factor-variable controls agree with their explicit dummy expansion.
sort obsid
kss_bc y i.time, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
matrix factor_result = e(results)
generate byte time1 = time == 1
generate byte time2 = time == 2
generate byte time3 = time == 3
kss_bc y time1 time2 time3, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert mreldif(factor_result,e(results)) < 2e-11

// Target mass changes the estimand but never the fitted least-squares model.
kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
matrix default_plugin = e(plugin)
scalar default_rss = e(weighted_rss)
generate double target = 1 + time + worker/10
quietly summarize target, meanonly
scalar target_total = r(sum)
kss_bc y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(target) algorithm(exact) nodisplay
assert abs(default_rss-e(weighted_rss)) < 2e-12
assert abs(e(target_weight_sum)-target_total) < 2e-12
assert mreldif(default_plugin,e(plugin)) > 1e-5

// Complete-case observation deletion and if restrictions agree exactly.
generate double y_missing = y
replace y_missing = . in 2
kss_bc y if obsid != 2, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
matrix restricted_result = e(results)
assert e(N_requested) == 23
assert e(N_complete) == 23
kss_bc y_missing, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
assert e(N_requested) == 24
assert e(N_complete) == 23
assert mreldif(restricted_result,e(results)) < 2e-12
quietly count if e(sample)
assert r(N) == e(N_stored)

// Singleton match blocks coincide with physical-observation deletion.
generate long singleton_match = obsid
kss_bc y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) nodisplay
matrix observation_result = e(results)
kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(singleton_match) algorithm(exact) nodisplay
assert mreldif(observation_result,e(results)) < 2e-11

// Match headlines remove stayers; observation deletion can retain them.
clear
input double(y worker firm match)
1.0 1 1 11
1.2 1 2 12
1.4 2 1 21
1.8 2 2 22
2.0 3 1 31
2.2 3 1 31
end
kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert e(N_stayers) == 1
assert e(N_stayer_rows) == 2
assert e(N_mover_input) == 4
assert e(N_retained) == 4
assert "`e(target_population)'" == "movers"

capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) stayers(both) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "STAYER_HYBRID_NOT_IMPLEMENTED"

kss_bc y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) nodisplay
assert e(N_retained) == 6
assert "`e(target_population)'" == "retained observations"

di as result "PASS test_semantics.do"
