version 18.0

// A same-level runtime with a different semantic build token must never run.
mata: mata drop kssbc__build_id()
mata:
string scalar kssbc__build_id()
{
    return("stale-test-runtime")
}
end

clear
input double(y worker firm match)
1.0 1 1 11
1.2 1 2 12
1.4 2 1 21
1.8 2 2 22
end

capture noisily kss_bc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) algorithm(exact) nodisplay
assert _rc == 498
assert "`e(status)'" == "WITHHELD"
assert "`e(withholding_status)'" == "STALE_MATA_RUNTIME"

di as result "PASS test_stale_runtime.do"
