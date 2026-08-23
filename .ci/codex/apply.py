from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text(encoding="utf-8")

anchor = """// Generic JLA is never inferred from a partial tuple.
"""
planned = r'''// The qualified V4/V7 planner is public only for the complete explicit tuple.
tempname planned_reference planned_memory
quietly varcomp_kss outcome control [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(generic) preconditioner(auto) ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) wallseconds(60) nodisplay
matrix `planned_reference' = e(results)
matrix `planned_memory' = e(rust_memory_receipt)
assert mreldif(`planned_reference',`public_reference') == 0
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
assert `"`e(rust_capability_profile)'"' == "PLANNED_V1"
assert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_result_cap_schema) == 3 & e(rust_result_cap_profile) == 4
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 2 & e(route_code) == 2
assert e(rust_solver_fallback) == 0 & e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert e(leverage_batch) >= 1 & e(leverage_batch) <= e(probes)
assert e(target_batch) >= 1 & e(target_batch) <= e(probes)
assert e(batch) == max(e(leverage_batch),e(target_batch))
assert e(rust_plan_schema) == 1
assert e(rust_plan_route_schema) == 2
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_wallseconds_forecast) >= 0
assert e(rust_wallseconds_advisory) >= 0
assert e(rust_wallseconds_margin) >= 0
assert e(rust_plan_solve_peak_forecast_bytes) == `planned_memory'[1,11]
assert `planned_memory'[1,12] == max(`planned_memory'[1,5],`planned_memory'[1,11])
assert `planned_memory'[1,12] <= `planned_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
forvalues row = 1/3 {
    assert abs(`planned_reference'[`row',4]-`planned_reference'[`row',1]- ///
        `planned_reference'[`row',2]-2*`planned_reference'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`planned_reference'[1,`column']-`planned_reference'[2,`column']- ///
        `planned_reference'[3,`column']) <= 1e-10
}
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// A partial planned tuple remains unsupported.
capture quietly varcomp_kss outcome control, worker(worker) firm(firm) ///
    deletion(observation) backend(rust) rng(counter_v1) algorithm(jla) ///
    engine(generic) preconditioner(auto) probes(4) nodisplay
assert _rc == 498 & `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"

'''
if text.count(anchor) != 1:
    raise RuntimeError(f"planned public test anchor: expected one match, found {text.count(anchor)}")
text = text.replace(anchor, planned + anchor, 1)

old_list = '''foreach auto_option in "algorithm(auto)" "engine(auto)"           ///
    "engine(compressed)" "preconditioner(auto)" {
'''
new_list = '''foreach auto_option in "algorithm(auto)" "engine(auto)"           ///
    "engine(compressed)" {
'''
if text.count(old_list) != 1:
    raise RuntimeError(f"obsolete auto-option list: expected one match, found {text.count(old_list)}")
text = text.replace(old_list, new_list, 1)
old_branch = '''    if "`auto_option'" == "preconditioner(auto)" local preconditioner_option
'''
if text.count(old_branch) != 1:
    raise RuntimeError(f"obsolete preconditioner branch: expected one match, found {text.count(old_branch)}")
text = text.replace(old_branch, "", 1)

path.write_text(text, encoding="utf-8")
print("added public planned-route fixture and repaired negative expectations")
