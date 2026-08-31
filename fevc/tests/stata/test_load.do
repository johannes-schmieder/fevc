version 18.0

capture noisily fevc, version
assert _rc == 0
assert "`e(cmd)'" == "fevc"
assert "`e(version)'" == "0.5.0-alpha.1"
assert "`e(model)'" == "linear"
assert "`e(correction)'" == "kss"

capture findfile fevc.mata
assert _rc == 0
quietly do `"`r(fn)'"'
mata: assert(vckss__version() == "0.5.0-alpha.1")
mata: assert(vckss__api_level() == 21)
mata: assert(vckss__build_id() == "vckss-api21-stayer-hybrid")
mata: assert(vckss__rounding_gamma(0) == 0)
mata: assert(vckss__inverse_forward_error(1e-14,1e-4,16) > vckss__inverse_forward_error(1e-14,1e-4,1))
mata: assert(missing(vckss__inverse_forward_error(1e-4,1e-4,2)))
mata: assert(vckss__exact_physical_total((2^52\2^52)) == 2^53)
mata: assert(missing(vckss__exact_physical_total((2^52\2^52\1))))
mata: assert(abs(vckss__max_column_relres((1,0),(0,1e12))-1) < 1e-15)
mata: assert(!hasmissing((8e307,1,2,3)))
mata: assert(!hasmissing((-8e307,0,0,0)))
mata: assert(hasmissing((8e307,1,2,3)-(-8e307,0,0,0)))

mata:
factor = ((.12,.03) \ (.08,-.02) \ (-.04,.06) \ (.02,.01))
rhs = ((1,.5) \ (-.2,.1) \ (.3,-.4) \ (.7,.2))
dense_maker = I(rows(factor))-factor*factor'
dense_actions = invsym(dense_maker)*rhs
low_rank = vckss__low_rank_maker(factor,rhs,1e-12,1e-10)
assert(low_rank.status == "CONVERGED")
assert(vckss__norm2(low_rank.actions-dense_actions) < 1e-12)
assert(vckss__max_column_relres(
    dense_maker*low_rank.actions-rhs,rhs) < 1e-12)
wide_factor = factor[1..2,.],J(2,3,0)
wide_rhs = rhs[1..2,.]
wide_maker = I(rows(wide_factor))-wide_factor*wide_factor'
wide_result = vckss__low_rank_maker(wide_factor,wide_rhs,1e-12,1e-10)
assert(wide_result.status == "CONVERGED")
assert(vckss__norm2(
    wide_result.actions-invsym(wide_maker)*wide_rhs) < 1e-12)
singular = vckss__low_rank_maker((1.01\0\0\0),rhs,1e-12,1e-10)
assert(singular.status == "NONESTIMABLE_DELETION")
end

di as result "PASS test_load.do"
