version 18.0

clear
set obs 3
generate long sentinel = _n
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

foreach example in exact_controls jla_controls weights_targets {
    varcomp_kss_run `example' using varcomp_kss.sthlp
    assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
    assert rowsof(e(decomposition)) == 4
    quietly _datasignature
    assert `"`r(datasignature)'"' == `"`caller_signature'"'
}

capture noisily varcomp_kss_run missing_example using varcomp_kss.sthlp
assert _rc == 111
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

capture noisily varcomp_kss_run exact_controls using missing_varcomp_help.sthlp
assert _rc == 601
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

di as result "PASS test_help_examples.do"
