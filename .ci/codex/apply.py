from pathlib import Path

path = Path("varcomp_kss/varcomp_kss.ado")
text = path.read_text(encoding="utf-8")
start_marker = "program define _vckss_rust_generic_planned, eclass sortpreserve\n"
end_marker = "program define _vckss_impl, eclass sortpreserve\n"
if text.count(start_marker) != 1 or text.count(end_marker) != 1:
    raise RuntimeError("planned program boundaries are not unique")
prefix, remainder = text.split(start_marker, 1)
planned_body, suffix = remainder.split(end_marker, 1)
planned = start_marker + planned_body

old = """        automatic_fallback_allowed wallseconds                       ///
        algorithm_resolution_deferred engine_resolution_deferred     ///
        route_resolution_deferred leverage_batch_deferred            ///
        target_batch_resolution_deferred wall_advisory_only {
"""
new = """        automatic_fallback_allowed wallseconds wall_advisory_only {
"""
if planned.count(old) != 1:
    raise RuntimeError(f"capture field tail: expected one block, found {planned.count(old)}")
planned = planned.replace(old, new, 1)

old = """    }
    local cap_reason_name `\"`r(reason)'\"'
"""
new = """    }
    // Stata local-macro names are capped at 32 characters.  Keep the
    // externally frozen receipt field names but store the longest fields in
    // short, explicit aliases rather than constructing cap_<field> names.
    local cap_alg_defer = r(algorithm_resolution_deferred)
    local cap_eng_defer = r(engine_resolution_deferred)
    local cap_route_defer = r(route_resolution_deferred)
    local cap_lev_defer = r(leverage_batch_deferred)
    local cap_tgt_defer = r(target_batch_resolution_deferred)
    local cap_reason_name `\"`r(reason)'\"'
"""
if planned.count(old) != 1:
    raise RuntimeError(f"short capability alias insertion: expected one anchor, found {planned.count(old)}")
planned = planned.replace(old, new, 1)

old = """        automatic_fallback_allowed algorithm_resolution_deferred     ///
        engine_resolution_deferred route_resolution_deferred         ///
        leverage_batch_deferred target_batch_resolution_deferred     ///
        wall_advisory_only {
"""
new = """        automatic_fallback_allowed wall_advisory_only {
"""
if planned.count(old) != 1:
    raise RuntimeError(f"validation field tail: expected one block, found {planned.count(old)}")
planned = planned.replace(old, new, 1)

old = """    }
    if missing(`cap_wallseconds') | `cap_wallseconds'<0 local capability_ok = 0
"""
new = """    }
    foreach value in cap_alg_defer cap_eng_defer cap_route_defer     ///
        cap_lev_defer cap_tgt_defer {
        if missing(``value'') | ``value'' < 0 |                     ///
            ``value'' != floor(``value'') local capability_ok = 0
    }
    if missing(`cap_wallseconds') | `cap_wallseconds'<0 local capability_ok = 0
"""
if planned.count(old) != 1:
    raise RuntimeError(f"short capability alias validation: expected one anchor, found {planned.count(old)}")
planned = planned.replace(old, new, 1)

replacements = {
    "`cap_algorithm_resolution_deferred'": "`cap_alg_defer'",
    "`cap_engine_resolution_deferred'": "`cap_eng_defer'",
    "`cap_route_resolution_deferred'": "`cap_route_defer'",
    "`cap_leverage_batch_deferred'": "`cap_lev_defer'",
    "`cap_target_batch_resolution_deferred'": "`cap_tgt_defer'",
}
for old, new in replacements.items():
    count = planned.count(old)
    if count != 1:
        raise RuntimeError(f"{old}: expected one reference, found {count}")
    planned = planned.replace(old, new, 1)

for forbidden in (
    "cap_algorithm_resolution_deferred",
    "cap_target_batch_resolution_deferred",
):
    if forbidden in planned:
        raise RuntimeError(f"overlong local alias survived: {forbidden}")

path.write_text(prefix + planned + end_marker + suffix, encoding="utf-8")
print("shortened planned V3 capability aliases without changing receipt fields")
