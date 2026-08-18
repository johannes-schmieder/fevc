version 18.0
clear all
set more off
set varabbrev off

// Tied outcomes across distinct coordinates are valid. Observed dense IDs
// provide structure; probeorder() is only an optional complete tie-breaker.
input double(y worker firm)
 0 1 1
 0 1 2
 6 1 3
11 2 1
13 2 2
14 2 3
end
generate double observation_key = _n
generate long worker_relabel = 101-worker
generate long firm_relabel = 17*(4-firm)

set seed 20260817
local rng_before `"`c(rngstate)'"'
quietly varcomp_kss y, worker(worker) firm(firm) deletion(match)       ///
    algorithm(jla) probes(40) batch(1) seed(8675309)              ///
    tolerance(1e-10) nodisplay
assert "`e(status)'" == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
assert "`e(probe_order)'" ==                                    ///
    "observed IDs, outcome, controls, and per-copy target mass"
assert e(solver_max_residual) <= 1e-9
matrix row_reference = e(results)
assert `"`c(rngstate)'"' == `"`rng_before'"'

// Row permutation and batch partitioning preserve the same observed-ID draw.
gsort -observation_key
quietly varcomp_kss y, worker(worker) firm(firm) deletion(match)       ///
    algorithm(jla) probes(40) batch(17) seed(8675309)             ///
    tolerance(1e-10) nodisplay
assert observation_key == 7-_n
assert mreldif(row_reference,e(results)) < 1e-14
assert e(solver_max_residual) <= 1e-9
assert `"`c(rngstate)'"' == `"`rng_before'"'

// A complete but duplicate optional key is accepted; uniqueness is not a
// user-facing requirement.
generate double duplicate_key = 1
quietly varcomp_kss y, worker(worker) firm(firm) deletion(match)       ///
    algorithm(jla) probeorder(duplicate_key) probes(40) batch(8)  ///
    seed(8675309) tolerance(1e-10) nodisplay
assert "`e(status)'" == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
assert "`e(probe_order)'" ==                                    ///
    "observed IDs, outcome, controls, target mass, and optional tie-breaker"

generate double incomplete_key = duplicate_key
replace incomplete_key = . in 1
capture noisily varcomp_kss y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(incomplete_key) probes(40) seed(8675309) ///
    nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_PROBE_ORDER"

// Arbitrary ID relabeling may choose another valid randomized draw. It must
// preserve the deterministic plug-in target, accounting identity, and all
// numerical acceptance gates.
quietly varcomp_kss y, worker(worker_relabel) firm(firm_relabel)       ///
    deletion(match) algorithm(jla) probes(40) batch(8)            ///
    seed(8675309) tolerance(1e-10) nodisplay
assert "`e(status)'" == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
matrix relabeled = e(results)
matrix plugin_reference = row_reference[1,1..4]
matrix plugin_relabel = relabeled[1,1..4]
assert mreldif(plugin_reference,plugin_relabel) < 1e-12
forvalues result_row = 1/3 {
    assert abs(relabeled[`result_row',4] -                       ///
        relabeled[`result_row',1] - relabeled[`result_row',2] -  ///
        2*relabeled[`result_row',3]) < 1e-10
}
assert e(solver_max_residual) <= 1e-9
assert `"`c(rngstate)'"' == `"`rng_before'"'

di as result "PASS test_probe_order.do"
exit 0
