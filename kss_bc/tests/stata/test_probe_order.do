version 18.0
clear all
set more off
set varabbrev off

// Discrete real outcomes can tie across distinct model coordinates.  The
// ordinary path must continue to fail closed.  An explicit physical-row key
// may resolve only that ordering ambiguity; it cannot be inferred silently.
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

capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probes(40) batch(8) seed(8675309) ///
    tolerance(1e-10) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "AMBIGUOUS_PROBE_ORDER"

generate double incomplete_key = observation_key
replace incomplete_key = . in 1
capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(incomplete_key) probes(40) ///
    seed(8675309) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_PROBE_ORDER"

generate double duplicate_key = observation_key
replace duplicate_key = 1 in 2
capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(duplicate_key) probes(40) ///
    seed(8675309) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_PROBE_ORDER"

set seed 20260815
kss_bc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(observation_key) probes(40) batch(1) ///
    seed(8675309) tolerance(1e-10) nodisplay
local rng_reference `"`c(rngstate)'"'
assert "`e(probe_order)'" == ///
    "outcome, per-copy target mass, and explicit physical-observation key"
assert e(solver_max_residual) <= 1e-10
matrix probe_reference = e(results)
matrix rhs_reference = e(solver_rhs_diagnostics)

gsort -observation_key
kss_bc y, worker(worker_relabel) firm(firm_relabel) deletion(match) ///
    algorithm(jla) probeorder(observation_key) probes(40) batch(17) ///
    seed(8675309) tolerance(1e-10) nodisplay
assert observation_key == 7-_n
assert `"`c(rngstate)'"' == `"`rng_reference'"'
assert mreldif(probe_reference,e(results)) < 1e-14
assert e(solver_max_residual) <= 1e-10
matrix rhs_changed = e(solver_rhs_diagnostics)
mata:
for (name=1; name<=2; name++) {
    rhs = st_matrix(name == 1 ? "rhs_reference" : "rhs_changed")
    assert(rows(rhs) == 121)
    assert(max(rhs[.,5]) <= 1e-10)
    assert(min(rhs[.,6]) == 1)
    assert(sum(rhs[.,1]:==2) == 1)
    assert(sum(rhs[.,1]:==4) == 40)
    assert(sum(rhs[.,1]:==5) == 80)
}
end

di as result "PASS test_probe_order.do"
exit 0
