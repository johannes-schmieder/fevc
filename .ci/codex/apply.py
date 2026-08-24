from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    return text.replace(old, new)


ado_path = Path("varcomp_kss/varcomp_kss.ado")
ado = ado_path.read_text()
start = ado.index("program define _vckss_rust_generic_planned, eclass sortpreserve")
end = ado.index("\nend\n", start) + len("\nend\n")
planned = ado[start:end]

planned = replace_once(
    planned,
    """        batch_lev_mode:r_lev_batch_mode                              ///
        batch_tgt_mode:r_tgt_batch_mode plan_schema:r_plan_schema     ///
        plan_route_schema:r_plan_route_schema plan_route_req:r_plan_route_req ///
        plan_route_sel:r_plan_route_sel plan_route_fallback:r_plan_route_fallback ///
        plan_route_error:r_plan_route_error wall_requested:r_wall_requested_value ///
""",
    """        batch_lev_mode:r_lev_batch_mode                              ///
        batch_tgt_mode:r_tgt_batch_mode plan_struct:r_plan_struct    ///
        plan_schema:r_plan_schema plan_route_schema:r_plan_route_schema ///
        plan_resolved:r_plan_resolved plan_frozen:r_plan_frozen      ///
        plan_applicability:r_plan_applicability                      ///
        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel      ///
        plan_route_req:r_plan_route_req plan_route_sel:r_plan_route_sel ///
        plan_route_fallback:r_plan_route_fallback                    ///
        plan_route_error:r_plan_route_error plan_rhs:r_plan_rhs      ///
        plan_full_dim:r_plan_full_dim batch_lev_sel:r_batch_lev_sel  ///
        batch_tgt_sel:r_batch_tgt_sel batch_command:r_batch_command  ///
        ctr_complete:r_ctr_complete plan_res_rng_hi:r_pre_rng_hi     ///
        plan_res_rng_lo:r_pre_rng_lo                                ///
        wall_requested:r_wall_requested_value                       ///
""",
    "generic plan result collection",
)

planned = replace_once(
    planned,
    """        r_lev_batch_mode r_tgt_batch_mode r_plan_schema             ///
        r_plan_route_schema r_plan_route_req r_plan_route_sel       ///
        r_plan_route_fallback r_plan_route_error                    ///
        r_wall_requested_value r_wall_forecast_value                ///
""",
    """        r_lev_batch_mode r_tgt_batch_mode r_plan_struct             ///
        r_plan_schema r_plan_route_schema r_plan_resolved r_plan_frozen ///
        r_plan_applicability r_plan_alg_req r_plan_alg_sel          ///
        r_plan_eng_req r_plan_eng_sel r_plan_route_req r_plan_route_sel ///
        r_plan_route_fallback r_plan_route_error r_plan_rhs         ///
        r_plan_full_dim r_batch_lev_sel r_batch_tgt_sel             ///
        r_batch_command r_ctr_complete r_pre_rng_hi r_pre_rng_lo    ///
        r_wall_requested_value r_wall_forecast_value                ///
""",
    "generic plan receipt-number validation",
)

batch_marker = "    local batch_result_ok =                                      ///\n"
if planned.count(batch_marker) != 1:
    raise SystemExit("generic batch-result anchor changed")
plan_check = """    local plan_result_ok =                                       ///
        `r_plan_struct'==1000 & `r_plan_schema'==1 &             ///
        `r_plan_route_schema'==2 & `r_plan_resolved'==1 &        ///
        `r_plan_frozen'==1 & `r_plan_applicability'==3 &         ///
        `r_plan_alg_req'==`algorithm_expected_code' &            ///
        `r_plan_alg_sel'==2 &                                    ///
        `r_plan_eng_req'==`engine_expected_code' &               ///
        `r_plan_eng_sel'==2 &                                    ///
        `r_plan_rhs'==`expected_rhs_rows' &                       ///
        `r_plan_full_dim'==`r_dimension' &                        ///
        `r_batch_lev_sel'==`r_lev_batch' &                        ///
        `r_batch_tgt_sel'==`r_tgt_batch' &                        ///
        `r_batch_command'==`r_plan_mem_command' &                 ///
        `r_ctr_complete'==1 & `r_pre_rng_hi'==0 & `r_pre_rng_lo'==0
"""
planned = planned.replace(batch_marker, plan_check + batch_marker)

planned = replace_once(
    planned,
    """            `route_result_ok' &                                   ///
            `r_dimension'==`p_firms'+`control_count' &             ///
""",
    """            `route_result_ok' & `plan_result_ok' &                ///
            `r_dimension'==`p_firms'+`control_count' &             ///
""",
    "generic plan reconciliation gate",
)

planned = replace_once(
    planned,
    """    ereturn scalar rust_requested_route = `r_req_route'
    ereturn scalar rust_selected_route = `r_sel_route'
""",
    """    ereturn scalar rust_requested_algorithm_code = `r_algorithm_req'
    ereturn scalar rust_selected_algorithm_code = `r_algorithm_sel'
    ereturn scalar rust_requested_route = `r_req_route'
    ereturn scalar rust_selected_route = `r_sel_route'
""",
    "generic public algorithm codes",
)

planned = replace_once(
    planned,
    """    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested_value'
""",
    """    ereturn scalar rust_plan_struct_size = `r_plan_struct'
    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_plan_resolved = `r_plan_resolved'
    ereturn scalar rust_plan_frozen = `r_plan_frozen'
    ereturn scalar rust_plan_applicability = `r_plan_applicability'
    ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
    ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
    ereturn scalar rust_plan_engine_requested = `r_plan_eng_req'
    ereturn scalar rust_plan_engine_selected = `r_plan_eng_sel'
    ereturn scalar rust_plan_route_requested = `r_plan_route_req'
    ereturn scalar rust_plan_route_selected = `r_plan_route_sel'
    ereturn scalar rust_plan_route_fallback = `r_plan_route_fallback'
    ereturn scalar rust_plan_route_error = `r_plan_route_error'
    ereturn scalar rust_plan_rhs = `r_plan_rhs'
    ereturn scalar rust_plan_full_dimension = `r_plan_full_dim'
    ereturn scalar rust_plan_leverage_batch = `r_batch_lev_sel'
    ereturn scalar rust_plan_target_batch = `r_batch_tgt_sel'
    ereturn scalar rust_counter_plan_complete = `r_ctr_complete'
    ereturn scalar rust_pre_rng_hi = `r_pre_rng_hi'
    ereturn scalar rust_pre_rng_lo = `r_pre_rng_lo'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested_value'
""",
    "generic public plan receipt",
)

planned = replace_once(
    planned,
    """    ereturn scalar rust_cap_engine_deferred = `cap_eng_defer'
""",
    """    ereturn scalar rust_cap_algorithm_deferred = `cap_alg_defer'
    ereturn scalar rust_cap_engine_deferred = `cap_eng_defer'
""",
    "generic capability deferral scalars",
)

ado = ado[:start] + planned + ado[end:]
ado_path.write_text(ado)

test_path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
test = test_path.read_text()
anchor = """

// Forced diagonal enters V4/V7 only when automatic batching or wall planning
// is requested.  The explicit numeric-batch/no-wall tuple above remains V2.
"""
if test.count(anchor) != 1:
    raise SystemExit("generic algorithm-auto test insertion anchor changed")
block = r'''

// Exercise requested algorithm(auto) with the generic JLA result family
// directly before opening the public router.  exact_limit(2) forces JLA;
// controls make the registered engine(auto) eligibility irrelevant here.
local auto_algorithm_rng `"`c(rng)'"'
local auto_algorithm_stream = c(rngstream)
local auto_algorithm_state `"`c(rngstate)'"'
local auto_algorithm_sortedby : sortedby
quietly _datasignature
local auto_algorithm_signature `"`r(datasignature)'"'
quietly count
local auto_algorithm_nscope = r(N)
local auto_algorithm_ncomplete = r(N)
quietly varcomp_kss_rust probe
local auto_algorithm_core = r(core_ready_flags)
local auto_algorithm_support = r(support_flags)
tempvar auto_algorithm_touse
generate byte `auto_algorithm_touse' = 1
capture noisily _vckss_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `auto_algorithm_touse' `auto_algorithm_nscope' ///
    `auto_algorithm_ncomplete' 0 0 7 2 81227 1e-12 10000 1 auto generic  ///
    1 1 1 1 1 1 1 0 `auto_algorithm_core' `auto_algorithm_support'       ///
    "nodisplay" match joint 2 1e-10 1e-10 5000 50000000 control 1 1    ///
    "varcomp_kss outcome control [fw=frequency], backend(rust) algorithm(auto) engine(generic)" ///
    auto auto 1 60
assert _rc == 0
assert mreldif(e(results),`planned_reference') == 0
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "generic"
assert `"`e(engine_selected)'"' == "generic"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 2
assert e(rust_requested_engine_code) == 2
assert e(rust_selected_engine_code) == 2
assert e(rust_plan_struct_size) == 1000
assert e(rust_plan_schema) == 1
assert e(rust_plan_route_schema) == 2
assert e(rust_plan_resolved) == 1
assert e(rust_plan_frozen) == 1
assert e(rust_plan_applicability) == 3
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 2
assert e(rust_plan_engine_requested) == 2
assert e(rust_plan_engine_selected) == 2
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 2
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert e(rust_plan_rhs) == rowsof(e(rust_rhs_receipts))
assert e(rust_plan_full_dimension) == e(rust_solver_dimension)
assert e(rust_plan_leverage_batch) == e(leverage_batch)
assert e(rust_plan_target_batch) == e(target_batch)
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_cap_algorithm_deferred) == 1
assert e(rust_cap_engine_deferred) == 1
tempname auto_algorithm_capability
matrix `auto_algorithm_capability' = e(rust_request_capability_receipt)
assert `auto_algorithm_capability'[1,7] == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`auto_algorithm_rng'"'
assert c(rngstream) == `auto_algorithm_stream'
assert `"`c(rngstate)'"' == `"`auto_algorithm_state'"'
local auto_algorithm_sortedby_after : sortedby
assert `"`auto_algorithm_sortedby_after'"' == `"`auto_algorithm_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`auto_algorithm_signature'"'
'''
test_path.write_text(test.replace(anchor, block + anchor))
