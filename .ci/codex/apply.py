from pathlib import Path


PRODUCTION = Path("varcomp_kss/varcomp_kss.ado")
PUBLIC_TEST = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")

production = PRODUCTION.read_text(encoding="utf-8")
old = """            `preconditioner_supplied' & "`preconditioner'" == "auto" & ///
"""
new = """            `preconditioner_supplied' &                         ///
            inlist("`preconditioner'","auto","cmg") &             ///
"""
if production.count(old) != 1:
    raise RuntimeError(
        f"planned preconditioner predicate: expected one block, found {production.count(old)}"
    )
production = production.replace(old, new, 1)

old = (
    "The Rust route supports exact estimation, the frozen compressed JLA subset, "
    "the explicit generic-diagonal tuple, or the planned generic-auto tuple."
)
new = (
    "The Rust route supports exact estimation, the frozen compressed JLA subset, "
    "the explicit generic-diagonal tuple, or the planned generic auto/CMG tuple."
)
if production.count(old) != 1:
    raise RuntimeError(
        f"unsupported-route message: expected one occurrence, found {production.count(old)}"
    )
production = production.replace(old, new, 1)
PRODUCTION.write_text(production, encoding="utf-8")

test = PUBLIC_TEST.read_text(encoding="utf-8")
anchor = r'''quietly _datasignature
assert `"`r(datasignature)'"' == `"`planned_signature'"'

// A partial planned tuple remains unsupported.
'''
forced_cmg = r'''quietly _datasignature
assert `"`r(datasignature)'"' == `"`planned_signature'"'

// Forced CMG uses the same V4/V7 lifecycle but may never fall back.
tempname forced_cmg_results forced_cmg_memory
local forced_cmg_rng `"`c(rng)'"'
local forced_cmg_stream = c(rngstream)
local forced_cmg_state `"`c(rngstate)'"'
local forced_cmg_sortedby : sortedby
quietly _datasignature
local forced_cmg_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(cmg) ///
    batch(2) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
matrix `forced_cmg_results' = e(results)
matrix `forced_cmg_memory' = e(rust_memory_receipt)
assert mreldif(`forced_cmg_results',`public_reference') <= 1e-9
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "cmg"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert `"`e(batch_requested)'"' == "2"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_route) == 3
assert e(rust_selected_route) == 3 & e(route_code) == 3
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 1
assert e(rust_leverage_batch_mode_code) == 1
assert e(rust_target_batch_mode_code) == 1
assert e(leverage_batch) == 2 & e(target_batch) == 2 & e(batch) == 2
assert e(rust_plan_schema) == 1 & e(rust_plan_route_schema) == 2
assert e(rust_wallseconds_supplied) == 0
assert e(rust_wallseconds_requested) == 0
assert e(rust_plan_solve_peak_bytes) == `forced_cmg_memory'[1,11]
assert `forced_cmg_memory'[1,12] == max(                       ///
    `forced_cmg_memory'[1,5],`forced_cmg_memory'[1,11])
assert `forced_cmg_memory'[1,12] <= `forced_cmg_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`forced_cmg_rng'"'
assert c(rngstream) == `forced_cmg_stream'
assert `"`c(rngstate)'"' == `"`forced_cmg_state'"'
local forced_cmg_sortedby_after : sortedby
assert `"`forced_cmg_sortedby_after'"' == `"`forced_cmg_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`forced_cmg_signature'"'

// A partial planned tuple remains unsupported.
'''
if test.count(anchor) != 1:
    raise RuntimeError(
        f"forced CMG insertion anchor: expected one block, found {test.count(anchor)}"
    )
test = test.replace(anchor, forced_cmg, 1)

old = r'''foreach forbidden in "stayers(movers)" "probeorder(replicate)"      ///
    "wallseconds(10)" "preconditioner(cmg)" "batch(auto)" {
    local preconditioner_option preconditioner(diagonal)
    local batch_option batch(2)
    if "`forbidden'" == "preconditioner(cmg)" local preconditioner_option
    if "`forbidden'" == "batch(auto)" local batch_option
'''
new = r'''foreach forbidden in "stayers(movers)" "probeorder(replicate)"      ///
    "wallseconds(10)" "batch(auto)" {
    local preconditioner_option preconditioner(diagonal)
    local batch_option batch(2)
    if "`forbidden'" == "batch(auto)" local batch_option
'''
if test.count(old) != 1:
    raise RuntimeError(
        f"obsolete forced-CMG negative case: expected one block, found {test.count(old)}"
    )
test = test.replace(old, new, 1)
PUBLIC_TEST.write_text(test, encoding="utf-8")
print("exposed forced generic CMG and added public no-fallback qualification")
