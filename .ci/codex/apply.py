from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_v4.do")
text = path.read_text(encoding="utf-8")
anchor = r'''quietly varcomp_kss_rust snapshot
assert r(state) == 0

di as result "VARCOMP_KSS RUST PLANNED V4 PASS"
'''
block = r'''quietly varcomp_kss_rust snapshot
assert r(state) == 0

// Inspect a forced-CMG V4/V7 result before the public rclass wrapper applies
// its legacy-prefix reconciliation.  This remains a private regression test:
// the V6 route prefix is deliberately diagonal, while V7 and every RHS row
// carry the actual forced-CMG route.
quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(cmg) rngcontract(counter_v1) controls(0)        ///
    frequencyused(1) engine(generic) batchmode(explicit)                  ///
    leveragebatchmode(explicit) targetbatchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid) fallback(0)        ///
    wallsecondssupplied(0) wallseconds(0) physicallimit(50000000)
assert r(supported) == 1 & r(request_schema) == 3 & r(profile_code) == 4
assert r(solver_route_code) == 3 & r(automatic_fallback_allowed) == 0
local cmg_schema = r(request_schema)
local cmg_profile = r(profile_code)
local cmg_signature_hi = r(request_signature_hi)
local cmg_signature_lo = r(request_signature_lo)

quietly varcomp_kss_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local cmg_handle = r(handle)
quietly varcomp_kss_rust solve `cmg_handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(cmg) seed(81227) probes(7) leveragebatch(2)      ///
    targetbatch(2) tolerance(1e-12) engine(generic) batchmode(explicit)   ///
    leveragebatchmode(explicit) targetbatchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`cmg_schema')                ///
    capabilityprofile(`cmg_profile') frequencyused(1)                    ///
    signaturehi(`cmg_signature_hi') signaturelo(`cmg_signature_lo')      ///
    fallback(0) wallsecondssupplied(0) wallseconds(0)

quietly _vckss_rust_plugin_call _vckss_rust_macos, result `cmg_handle'
assert scalar(__vckss_rust_cap_schema_echo) == 3
assert scalar(__vckss_rust_cap_profile_echo) == 4
assert scalar(__vckss_rust_route_requested) == 2
assert scalar(__vckss_rust_route_selected) == 2
assert scalar(__vckss_plan_route_req) == 3
assert scalar(__vckss_plan_route_sel) == 3
assert scalar(__vckss_plan_route_fallback) == 0
assert scalar(__vckss_plan_route_error) == 0
assert scalar(__vckss_rust_full_route) == 3
assert scalar(__vckss_rust_solve_peak) == scalar(__vckss_mem_command)
assert scalar(__vckss_rust_command_peak) == max(                     ///
    scalar(__vckss_rust_prepare_peak),scalar(__vckss_mem_command))

di as result "VCKSS_CMG_RAW solver_setup="                         ///
    scalar(__vckss_rust_solver_setup) " g_canon="                   ///
    scalar(__vckss_rust_g_canon_peak) " g_fit="                    ///
    scalar(__vckss_rust_g_fit_peak) " g_geometry="                 ///
    scalar(__vckss_rust_g_geometry_peak) " g_lev="                 ///
    scalar(__vckss_rust_g_lev_peak) " g_tgt="                      ///
    scalar(__vckss_rust_g_tgt_peak) " g_maker="                    ///
    scalar(__vckss_rust_g_maker_peak) " g_result="                 ///
    scalar(__vckss_rust_g_result_bytes) " g_peak="                 ///
    scalar(__vckss_rust_g_peak)
di as result "VCKSS_CMG_PLAN mem_setup=" scalar(__vckss_mem_setup) ///
    " mem_fit=" scalar(__vckss_mem_fit)                             ///
    " mem_correction=" scalar(__vckss_mem_correction)               ///
    " mem_lev=" scalar(__vckss_mem_leverage)                        ///
    " mem_tgt=" scalar(__vckss_mem_target)                          ///
    " mem_result=" scalar(__vckss_mem_result)                       ///
    " mem_nonbatched=" scalar(__vckss_mem_nonbatched)               ///
    " mem_command=" scalar(__vckss_mem_command)                     ///
    " shared_cmg=" scalar(__vckss_mem_shared_cmg)                   ///
    " cmg_workspace=" scalar(__vckss_mem_cmg_workspace)             ///
    " cmg_cells=" scalar(__vckss_mem_cmg_cells)                     ///
    " cmg_groups=" scalar(__vckss_mem_cmg_groups)                   ///
    " cmg_graph=" scalar(__vckss_mem_cmg_graph)

local cmg_rhs_rows = scalar(__vckss_rust_rhs_rows)
tempname cmg_rhs_receipts
matrix `cmg_rhs_receipts' = J(`cmg_rhs_rows',15,.)
quietly _vckss_rust_plugin_call _vckss_rust_macos, rhsresult       ///
    `cmg_handle' `cmg_rhs_receipts'
forvalues row = 1/`cmg_rhs_rows' {
    assert `cmg_rhs_receipts'[`row',4] == 3
}
quietly _vckss_rust_plan_receipt
assert r(plan_route_req) == 3 & r(plan_route_sel) == 3
assert r(plan_route_fallback) == 0 & r(plan_route_error) == 0
assert r(mem_command) == scalar(__vckss_rust_solve_peak)
assert scalar(__vckss_rust_route_requested) == 3
assert scalar(__vckss_rust_route_selected) == 3
quietly varcomp_kss_rust release `cmg_handle'
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0

di as result "VARCOMP_KSS RUST PLANNED V4 PASS"
'''
if text.count(anchor) != 1:
    raise RuntimeError(
        f"forced-CMG private receipt anchor: expected one block, found {text.count(anchor)}"
    )
path.write_text(text.replace(anchor, block, 1), encoding="utf-8")
print("added raw forced-CMG V6/V7 receipt regression coverage")
