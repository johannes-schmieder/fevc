version 18.0

capture noisily kss_bc, version
assert _rc == 0
assert "`e(cmd)'" == "kss_bc"
assert "`e(version)'" == "0.1.0-dev"
assert "`e(model)'" == "linear"
assert "`e(correction)'" == "kss"

capture findfile kss_bc.mata
assert _rc == 0
quietly do `"`r(fn)'"'
mata: assert(kssbc__version() == "0.1.0-dev")
mata: assert(kssbc__api_level() == 14)
mata: assert(kssbc__build_id() == "kss-bc-api14-lockstep-pcg-diagnostics")
mata: assert(kssbc__rounding_gamma(0) == 0)
mata: assert(kssbc__inverse_forward_error(1e-14,1e-4,16) > kssbc__inverse_forward_error(1e-14,1e-4,1))
mata: assert(missing(kssbc__inverse_forward_error(1e-4,1e-4,2)))
mata: assert(kssbc__exact_physical_total((2^52\2^52)) == 2^53)
mata: assert(missing(kssbc__exact_physical_total((2^52\2^52\1))))
mata: assert(abs(kssbc__max_column_relres((1,0),(0,1e12))-1) < 1e-15)
mata: assert(!hasmissing((8e307,1,2,3)))
mata: assert(!hasmissing((-8e307,0,0,0)))
mata: assert(hasmissing((8e307,1,2,3)-(-8e307,0,0,0)))

di as result "PASS test_load.do"
