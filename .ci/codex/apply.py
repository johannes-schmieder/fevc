from pathlib import Path

production = Path("varcomp_kss/varcomp_kss.ado")
text = production.read_text(encoding="utf-8")
start = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
end = "program define _vckss_impl, eclass sortpreserve\n"
if text.count(start) != 1 or text.count(end) != 1:
    raise RuntimeError("planned Rust program boundaries are not unique")
prefix, remainder = text.split(start, 1)
planned_body, suffix = remainder.split(end, 1)
planned = start + planned_body

old = """        `r_full_route'==`r_sel_route' &                          ///
"""
new = """        // The V6 full-fit route field is frozen as diagonal.  V7
        // plan_route_sel is authoritative for the actual selected route.
        `r_full_route'==2 &                                      ///
"""
if planned.count(old) != 1:
    raise RuntimeError(
        f"planned V6 full-fit route check: expected one block, found {planned.count(old)}"
    )
planned = planned.replace(old, new, 1)
production.write_text(prefix + planned + end + suffix, encoding="utf-8")

test_path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
test = test_path.read_text(encoding="utf-8")
old = """assert e(rust_selected_route) == 3 & e(route_code) == 3
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
"""
new = """assert e(rust_selected_route) == 3 & e(route_code) == 3
assert e(rust_full_fit_route) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
"""
if test.count(old) != 1:
    raise RuntimeError(
        f"public forced-CMG route assertions: expected one block, found {test.count(old)}"
    )
test_path.write_text(test.replace(old, new, 1), encoding="utf-8")
print("preserved the frozen V6 full-fit route while retaining V7 CMG selection")
