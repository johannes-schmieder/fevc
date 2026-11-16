p = '<session-tmp>/e2e/veneto_e2e.do'; s = open(p).read()
x = """assert mreldif(point,b)<1e-10
"""
y = """* Standard errors do not change the estimator, but on the native backend they select another executor, so the two
* estimates agree to solver tolerance (the package's own regression uses the same bound).
local point_reldif = mreldif(point,b)
assert `point_reldif'<1e-5
"""
assert s.count(x) == 1; s = s.replace(x, y)
x = """file write `out' "seconds_point=`s_point'" _n"""
assert s.count(x) == 1; s = s.replace(x, """file write `out' "point_reldif=`point_reldif'" _n "seconds_point=`s_point'" _n""")
open(p, 'w').write(s); print('patched')
