version 18.0
clear all
set more off
set varabbrev off

mata: mata clear
capture noisily do "kss_bc/benchmarks/kss_scale_fixtures.mata"
if _rc quietly do "../../benchmarks/kss_scale_fixtures.mata"
capture noisily do "kss_bc/benchmarks/kss_scale_fixtures.do"
if _rc quietly do "../../benchmarks/kss_scale_fixtures.do"

input double(y worker firm deletion_unit frequency target observation_key)
1.00 101 501 901 1 1 1
1.20 101 501 901 2 2 2
0.90 101 501 905 1 1 3
1.10 101 503 902 1 1 4
1.30 107 501 903 1 1 5
1.40 107 503 904 1 1 6
end
tempfile base well
save `base'

kssbc_scale_fixture, design(well-connected) copies(4) ///
    worker(worker) firm(firm) deletionid(deletion_unit) outcome(y) ///
    frequency(frequency) target(target) copyvar(copy_id) ///
    connectorvar(connector) rowkey(observation_key)
assert "`r(status)'" == "CONVERGED"
assert "`r(design)'" == "well_connected"
assert r(base_rows) == 6
assert r(base_physical) == 7
assert r(base_cells) == 4
assert r(base_deletion_units) == 5
assert r(connector_pairs) == 6
assert r(connector_workers) == 12
assert r(connector_rows) == 24
assert r(expected_rows) == 48
assert r(expected_physical) == 52
assert r(expected_workers) == 20
assert r(expected_firms) == 8
assert r(expected_cells) == 40
assert r(expected_deletion_units) == 44
assert reldif(r(copy_cut_conductance),2/3) < 1e-14
assert reldif(r(normalized_lambda2),4/3) < 1e-14
assert reldif(r(normalized_condition_proxy),1) < 1e-14
matrix well_diagnostic = r(fixture_diagnostics)
assert well_diagnostic[1,"cells"] == 40
assert well_diagnostic[1,"deletion_units"] == 44
assert well_diagnostic[1,"components"] == 1
assert well_diagnostic[1,"bridge_units"] == 0
quietly count if connector == 0
assert r(N) == 24
quietly count if connector == 1
assert r(N) == 24
forvalues copy = 1/4 {
    quietly count if copy_id == `copy' & connector == 0
    assert r(N) == 6
}
bysort worker: egen byte firms_per_connector = nvals(firm) if connector
bysort worker: generate long rows_per_connector = _N if connector
assert firms_per_connector == 2 if connector
assert rows_per_connector == 2 if connector
drop firms_per_connector rows_per_connector
save `well'

// Canonical densification and sorting make the fixture invariant to raw-row
// order and arbitrary numeric spacing of the source IDs.
use `base', clear
generate double shuffle = mod(observation_key*17,11)
sort shuffle
drop shuffle
kssbc_scale_fixture, design(well_connected) copies(4) ///
    worker(worker) firm(firm) deletionid(deletion_unit) outcome(y) ///
    frequency(frequency) target(target) copyvar(copy_id) ///
    connectorvar(connector) rowkey(observation_key)
cf _all using `well'

use `base', clear
kssbc_scale_fixture, design(ring) copies(4) ///
    worker(worker) firm(firm) deletionid(deletion_unit) outcome(y) ///
    frequency(frequency) target(target) copyvar(copy_id) ///
    connectorvar(connector) rowkey(observation_key)
assert "`r(design)'" == "ring"
assert r(connector_pairs) == 4
assert r(connector_workers) == 8
assert r(connector_rows) == 16
assert r(expected_rows) == 40
assert r(expected_physical) == 44
assert r(expected_workers) == 16
assert r(expected_firms) == 8
assert r(expected_cells) == 32
assert r(expected_deletion_units) == 36
assert reldif(r(copy_cut_conductance),1/2) < 1e-14
assert reldif(r(normalized_lambda2),1) < 1e-14
assert reldif(r(normalized_lambda_max),2) < 1e-14
assert reldif(r(normalized_condition_proxy),2) < 1e-14
matrix ring_diagnostic = r(fixture_diagnostics)
assert ring_diagnostic[1,"components"] == 1
assert ring_diagnostic[1,"bridge_units"] == 0

// A deletion unit spanning coefficient cells is rejected before replication.
clear
input double(y worker firm deletion_unit)
1 1 1 11
2 1 2 11
3 2 1 12
4 2 2 13
end
capture noisily kssbc_scale_fixture, design(ring) copies(2) ///
    worker(worker) firm(firm) deletionid(deletion_unit) outcome(y)
assert _rc == 459

di as result "PASS test_scale_fixtures.do"
