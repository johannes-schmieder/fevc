from pathlib import Path

path = Path("varcomp_kss/varcomp_kss.ado")
text = path.read_text(encoding="utf-8")
start = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
end = "program define _vckss_impl, eclass sortpreserve\n"
if text.count(start) != 1 or text.count(end) != 1:
    raise RuntimeError("planned Rust program boundaries are not unique")
prefix, remainder = text.split(start, 1)
planned_body, suffix = remainder.split(end, 1)
planned = start + planned_body

old = """            if `rhs_native'[`row',4]!=`r_sel_route' |             ///
                `rhs_native'[`row',5]<0 |                              ///
"""
new = """            // Column four belongs to the frozen V2 RHS prefix and
            // therefore remains diagonal.  The additive V7 plan fields above
            // are authoritative for the actual selected route.
            if `rhs_native'[`row',4]!=2 |                         ///
                `rhs_native'[`row',5]<0 |                         ///
"""
if planned.count(old) != 1:
    raise RuntimeError(
        f"planned RHS route reconciliation: expected one block, found {planned.count(old)}"
    )
planned = planned.replace(old, new, 1)
if "`rhs_native'[`row',4]!=`r_sel_route'" in planned:
    raise RuntimeError("selected V7 route still leaks into the frozen RHS prefix")

path.write_text(prefix + planned + end + suffix, encoding="utf-8")
print("restored frozen V2 RHS route reconciliation in the planned public path")
