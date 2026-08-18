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
varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) seed(1) nodisplay
matrix reference = e(results)
scalar reference_rss = e(weighted_rss)
scalar reference_units = e(deletion_units)

generate long split_match = match
replace split_match = 99 in 2
varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(split_match) algorithm(exact) nodisplay
assert e(deletion_units) == reference_units+1
assert mreldif(reference,e(results)) > 1e-7

generate str8 worker_s = "w" + string(worker,"%02.0f")
generate str8 firm_s = "f" + string(firm,"%02.0f")
generate str12 match_s = "m" + string(match,"%08.0f")
varcomp_kss y c1 c2, worker(worker_s) firm(firm_s) deletion(match) ///
    deletionid(match_s) algorithm(exact) seed(999) nodisplay
assert mreldif(reference,e(results)) < 2e-12
assert abs(reference_rss-e(weighted_rss)) < 2e-12

generate long worker_r = 1000 - 17*worker
generate long firm_r = 700 + 13*(3-firm)
generate long match_r = 100000 - 7*match
varcomp_kss y c1 c2, worker(worker_r) firm(firm_r) deletion(match) ///
    deletionid(match_r) algorithm(exact) nodisplay
assert mreldif(reference,e(results)) < 2e-11

// The fixed-seed JLA stream is stable within observed IDs. Arbitrary ID
// relabeling may select another valid draw; deterministic plug-in targets and
// numerical identities remain invariant.
sort obsid
varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) probes(400) batch(11) ///
    seed(20261001) tolerance(1e-12) nodisplay
matrix jla_id_reference = e(results)
varcomp_kss y c1 c2, worker(worker_r) firm(firm_r) deletion(match) ///
    deletionid(match_r) algorithm(jla) probes(400) batch(11) ///
    seed(20261001) tolerance(1e-12) nodisplay
matrix jla_id_relabel = e(results)
matrix jla_id_reference_plugin = jla_id_reference[1,1..4]
matrix jla_id_relabel_plugin = jla_id_relabel[1,1..4]
assert mreldif(jla_id_reference_plugin,jla_id_relabel_plugin) < 2e-10
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)
generate double z1 = -c1
generate double z2 = c2
varcomp_kss y z1 z2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) probes(400) batch(11) ///
    seed(20261001) tolerance(1e-12) nodisplay
matrix jla_control_relabel = e(results)
matrix jla_control_relabel_plugin = jla_control_relabel[1,1..4]
assert mreldif(jla_id_reference_plugin,jla_control_relabel_plugin) < 2e-10
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)

// Canonicalize the control span before any adaptively stopped solve.  This
// reviewer-supplied invertible map has determinant four and changed accepted
// API-10 output even though the conceptual sign stream was unchanged.
generate double control_t1 = c1 - 3*c2
generate double control_t2 = c1 + c2
foreach nuisance_mode in joint fixedoffset {
    local transform_seed = cond("`nuisance_mode'"=="joint",3,1)
    varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
        deletionid(match) algorithm(jla) nuisance(`nuisance_mode') ///
        probes(2) batch(1) seed(`transform_seed') tolerance(1e-4) nodisplay
    matrix control_basis_reference = e(results)
    varcomp_kss y control_t1 control_t2, worker(worker) firm(firm) ///
        deletion(match) deletionid(match) algorithm(jla) ///
        nuisance(`nuisance_mode') probes(2) batch(1) ///
        seed(`transform_seed') tolerance(1e-4) nodisplay
    assert mreldif(control_basis_reference,e(results)) < 2e-10
}

// The largest allowed iterative tolerance must not make the accepted quotient
// solution depend on which raw firm label becomes the omitted base.
varcomp_kss y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(jla) probes(80) batch(7) ///
    seed(20261002) tolerance(1e-4) nodisplay
matrix loose_id_reference = e(results)
varcomp_kss y, worker(worker_r) firm(firm_r) deletion(match) ///
    deletionid(match_r) algorithm(jla) probes(80) batch(7) ///
    seed(20261002) tolerance(1e-4) nodisplay
matrix loose_id_relabel = e(results)
matrix loose_id_reference_plugin = loose_id_reference[1,1..4]
matrix loose_id_relabel_plugin = loose_id_relabel[1,1..4]
assert mreldif(loose_id_reference_plugin,loose_id_relabel_plugin) < 2e-10
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)

// The six-row K(2,3) attack still checks the full quotient residual. The
// relabelings may use different randomized draws.
preserve
clear
input double(y worker firm)
-15 1 3
-8  2 2
 6  2 3
11  2 1
13  1 1
14  1 2
end
generate long worker_swap = 101-worker
generate long firm_swap23 = cond(firm==2,3,cond(firm==3,2,1))
generate long firm_swap13 = cond(firm==1,3,cond(firm==3,1,2))
varcomp_kss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) probes(2) batch(1) seed(1) tolerance(1e-4) nodisplay
matrix k23_reference = e(results)
varcomp_kss y, worker(worker_swap) firm(firm_swap23) deletion(observation) ///
    algorithm(jla) probes(2) batch(1) seed(1) tolerance(1e-4) nodisplay
matrix k23_swap23 = e(results)
matrix k23_reference_plugin = k23_reference[1,1..4]
matrix k23_swap23_plugin = k23_swap23[1,1..4]
assert mreldif(k23_reference_plugin,k23_swap23_plugin) < 2e-10
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)
varcomp_kss y, worker(worker) firm(firm_swap13) deletion(observation) ///
    algorithm(jla) probes(2) batch(2) seed(1) tolerance(1e-4) nodisplay
matrix k23_swap13 = e(results)
matrix k23_swap13_plugin = k23_swap13[1,1..4]
assert mreldif(k23_reference_plugin,k23_swap13_plugin) < 2e-10
assert e(solver_max_residual) <= e(residual_acceptance_tolerance)
restore

// Exact mode is invariant to row order and to the seed.
gsort -obsid
varcomp_kss y c1 c2, worker(worker_s) firm(firm_s) deletion(match) ///
    deletionid(match_s) algorithm(exact) seed(42) nodisplay
assert mreldif(reference,e(results)) < 2e-12
assert obsid == 25-_n

// Factor-variable controls agree with their explicit dummy expansion.
sort obsid
varcomp_kss y i.time, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
matrix factor_result = e(results)
generate byte time1 = time == 1
generate byte time2 = time == 2
generate byte time3 = time == 3
varcomp_kss y time1 time2 time3, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert mreldif(factor_result,e(results)) < 2e-11

// Target mass changes the estimand but never the fitted least-squares model.
varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
matrix default_plugin = e(plugin)
scalar default_rss = e(weighted_rss)
generate double target = 1 + time + worker/10
quietly summarize target, meanonly
scalar target_total = r(sum)
varcomp_kss y c1 c2, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(target) algorithm(exact) nodisplay
assert abs(default_rss-e(weighted_rss)) < 2e-12
assert abs(e(target_weight_sum)-target_total) < 2e-12
assert mreldif(default_plugin,e(plugin)) > 1e-5

// Complete-case observation deletion and if restrictions agree exactly.
generate double y_missing = y
replace y_missing = . in 2
varcomp_kss y if obsid != 2, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
matrix restricted_result = e(results)
assert e(N_requested) == 23
assert e(N_complete) == 23
varcomp_kss y_missing, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
assert e(N_requested) == 24
assert e(N_complete) == 23
assert mreldif(restricted_result,e(results)) < 2e-12
quietly count if e(sample)
assert r(N) == e(N_stored)

// Singleton match blocks coincide with physical-observation deletion.
generate long singleton_match = obsid
varcomp_kss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) nodisplay
matrix observation_result = e(results)
varcomp_kss y, worker(worker) firm(firm) deletion(match) ///
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
varcomp_kss y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert e(N_stayers) == 1
assert e(N_stayer_rows) == 2
assert e(N_mover_input) == 4
assert e(N_retained) == 4
assert "`e(target_population)'" == "movers"

capture noisily varcomp_kss y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) stayers(both) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "STAYER_HYBRID_NOT_IMPLEMENTED"

varcomp_kss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) nodisplay
assert e(N_retained) == 6
assert "`e(target_population)'" == "retained observations"

di as result "PASS test_semantics.do"
