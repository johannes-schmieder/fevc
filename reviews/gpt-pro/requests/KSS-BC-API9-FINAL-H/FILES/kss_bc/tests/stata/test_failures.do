version 18.0

preserve
clear
input double(y worker firm match)
1 1 1 10
2 1 2 11
3 2 1 20
4 2 2 21
5 3 1 30
6 3 2 31
end

replace match = 10 in 3
capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(status)'" == "WITHHELD"
assert "`e(withholding_status)'" == "CROSS_COORDINATE_MATCH"

replace match = 20 in 3
replace y = . in 1
capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert _rc == 459
assert "`e(withholding_status)'" == "MATCH_INPUT_MISSING"
restore

preserve
clear
input double(y worker firm match frequency target)
1.0 1 1 11 1 1
1.1 1 1 11 1 1
1.2 1 2 12 1 1
1.3 1 2 12 1 1
1.4 2 1 21 1 1
1.5 2 1 21 1 1
1.6 2 2 22 1 1
1.7 2 2 22 1 1
end

replace frequency = 1.5 in 1
capture noisily kss_bc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_FREQUENCY"
replace frequency = . in 1
capture noisily kss_bc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_FREQUENCY"
replace frequency = 1 in 1

replace target = -1 in 1
capture noisily kss_bc y, worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_TARGET_WEIGHT"
replace target = . in 1
capture noisily kss_bc y, worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_TARGET_WEIGHT"
replace target = 0
capture noisily kss_bc y, worker(worker) firm(firm) ///
    deletion(observation) targetweight(target) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_TARGET_WEIGHT"
replace target = 1

capture noisily kss_bc y, worker(worker) firm(firm) ///
    deletion(observation) deletionid(match) algorithm(exact) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "UNSUPPORTED_DELETION_ID"

capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) exact_limit(2) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "EXACT_SIZE_LIMIT"

capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) blocksize_limit(1) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "BLOCK_SIZE_LIMIT"

generate byte constant = 1
capture noisily kss_bc y constant, worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) probes(5) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "SINGULAR_NUISANCE_BLOCK"
restore

preserve
clear
set obs 24
generate long worker = floor((_n-1)/4)
generate byte time = mod(_n-1,4)
generate byte firm = .
generate long match = .
local firms 0 0 1 1 0 2 2 1 1 2 3 3 2 3 0 0 3 1 1 2 3 3 2 0
local matches 10 10 11 11 20 21 21 22 30 31 32 32 40 41 42 42 50 51 51 52 60 60 61 62
forvalues row = 1/24 {
    local value : word `row' of `firms'
    quietly replace firm = `value' in `row'
    local value : word `row' of `matches'
    quietly replace match = `value' in `row'
}
generate double y = worker + firm + time/10
generate byte block_only_control = match == 10
capture noisily kss_bc y block_only_control, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "NONESTIMABLE_DELETION"

capture noisily kss_bc y block_only_control, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) algorithm(jla) probes(2) ///
    seed(1) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "UNVERIFIED_DELETION_RANK"
restore

// Large cell-constant control components caused cancellation in the old
// cross-product-minus-cell-means scatter. Deleting match 11 makes the control
// exactly collinear with the FE intercept, so both backends must withhold.
preserve
clear
input double(y worker firm match z)
1 1 1 11 0
1 1 1 11 0
1 1 1 11 5120
1 1 2 12 25165824
1 1 2 12 25165824
1 1 3 13 25165824
1 2 1 21 25165824
1 2 2 22 25165824
1 2 3 23 25165824
end

capture noisily kss_bc y z, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "NONESTIMABLE_DELETION"

capture noisily kss_bc y z, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) algorithm(jla) probes(2) ///
    seed(1) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "UNVERIFIED_DELETION_RANK"
restore

// An ill-conditioned control basis made both old numerical rank gates accept
// deletion of match 1 even though the deleted joint design has rank four of
// five.  Direct deleted-scatter/information factorizations must reject it.
preserve
clear
input double(y worker firm match z1 z2)
1 1 1 1  11502.278164839073 -31359.35955592377
2 1 1 1 -11502.278164839073  31359.35955592377
3 1 2 2  -482.9683774669384  1317.946194040493
4 1 2 2   482.9683774669384 -1317.946194040493
5 1 2 3  -482.9683774669384  1317.946194040493
6 1 2 3   482.9683774669384 -1317.946194040493
7 1 2 4  -482.9683774669384  1317.946194040493
8 1 2 4   482.9683774669384 -1317.946194040493
9 2 1 5 0 0
10 2 1 5 0 0
11 2 2 6 0 0
12 2 2 6 0 0
end

capture noisily kss_bc y z1 z2, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "NONESTIMABLE_DELETION"

capture noisily kss_bc y z1 z2, worker(worker) firm(firm) ///
    deletion(match) deletionid(match) algorithm(jla) probes(20) ///
    seed(1) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "UNVERIFIED_DELETION_RANK"
restore

preserve
clear
input double(y worker firm)
. 1 1
. 1 2
. 2 1
. 2 2
end
capture noisily kss_bc y, worker(worker) firm(firm) ///
    deletion(observation) algorithm(exact) nodisplay
assert _rc == 2000
assert "`e(withholding_status)'" == "NO_USABLE_OBSERVATIONS"
restore

di as result "PASS test_failures.do"
