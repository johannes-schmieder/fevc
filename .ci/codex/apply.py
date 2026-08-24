from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    return text.replace(old, new)


ado_path = Path("varcomp_kss/varcomp_kss.ado")
ado = ado_path.read_text()
planned_start = ado.index("program define _vckss_rust_generic_planned, eclass sortpreserve")
planned_end = ado.index("\nend\n", planned_start) + len("\nend\n")
planned = ado[planned_start:planned_end]

planned = replace_once(
    planned,
    """    local control_count : word count `controls'\n    local engine_requested = lower(strtrim(\"`enginerequested'\"))\n    local engine_expected_code = cond(\"`engine_requested'\"==\"auto\",0,2)\n    local engine_defer_expected = cond(\"`engine_requested'\"==\"auto\",1,0)\n""",
    """    local control_count : word count `controls'\n    local algorithm_requested = lower(strtrim(\"`algorithm_requested'\"))\n    local algorithm_expected_code = cond(\"`algorithm_requested'\"==\"auto\",0,2)\n    local algorithm_defer_expected = cond(\"`algorithm_requested'\"==\"auto\",1,0)\n    local engine_requested = lower(strtrim(\"`enginerequested'\"))\n    local engine_expected_code = cond(\"`engine_requested'\"==\"auto\",0,2)\n    local engine_defer_expected = cond(\"`algorithm_requested'\"==\"auto\" | ///\n        \"`engine_requested'\"==\"auto\",1,0)\n""",
    "planned algorithm normalization",
)

lines = planned.splitlines(keepends=True)
validation_hits = [
    i for i, line in enumerate(lines)
    if 'if !inlist("`engine_requested\'","generic","auto")' in line
]
if len(validation_hits) != 1:
    raise SystemExit(f"planned validation anchor: expected one line, found {len(validation_hits)}")
i = validation_hits[0]
lines[i:i + 1] = [
    '    if !inlist("`algorithm_requested\'","jla","auto") |             ///\n',
    '        ("`algorithm_requested\'"=="auto" &                         ///\n',
    '            "`preconditioner_requested\'"!="auto") |                ///\n',
    '        !inlist("`engine_requested\'","generic","auto") |          ///\n',
]
planned = "".join(lines)

planned = replace_once(
    planned,
    "`cap_algorithm_code'==2 &",
    "`cap_algorithm_code'==`algorithm_expected_code' &",
    "planned capability algorithm code",
)
planned = replace_once(
    planned,
    "`cap_alg_defer'==0 &",
    "`cap_alg_defer'==`algorithm_defer_expected' &",
    "planned capability algorithm deferral",
)
planned = replace_once(
    planned,
    "`blocktol' `nuisance_code' `route_expected_code'",
    "`blocktol' `algorithm_expected_code' `nuisance_code' `route_expected_code'",
    "compressed reconciler algorithm argument",
)
planned = replace_once(
    planned,
    "`r_algorithm_req'==2 & `r_algorithm_sel'==2 &",
    "`r_algorithm_req'==`algorithm_expected_code' & `r_algorithm_sel'==2 &",
    "generic result algorithm receipt",
)
ado = ado[:planned_start] + planned + ado[planned_end:]
ado_path.write_text(ado)

reconcile_path = Path("varcomp_kss/_vckss_rust_reconcile_comp_v7.ado")
reconcile = reconcile_path.read_text()
algorithm_arg_anchor = "rank_tolerance block_tolerance nuisance_code route_requested"
if reconcile.count(algorithm_arg_anchor) != 2:
    raise SystemExit(
        f"compressed algorithm argument anchors: expected two, found {reconcile.count(algorithm_arg_anchor)}"
    )
reconcile = reconcile.replace(
    algorithm_arg_anchor,
    "rank_tolerance block_tolerance algorithm_requested nuisance_code route_requested",
)
reconcile = replace_once(
    reconcile,
    "if `ok' & (!inlist(`nuisance_code',1,2) |",
    "if `ok' & (!inlist(`algorithm_requested',0,2) |                 ///\n        !inlist(`nuisance_code',1,2) |",
    "compressed algorithm semantic validation",
)
reconcile = replace_once(
    reconcile,
    "`r_alg_req'==2 & `r_alg_sel'==2 &",
    "`r_alg_req'==`algorithm_requested' & `r_alg_sel'==2 &",
    "compressed result algorithm receipt",
)
reconcile = replace_once(
    reconcile,
    "`r_plan_alg_req'==2 & `r_plan_alg_sel'==2 &",
    "`r_plan_alg_req'==`algorithm_requested' & `r_plan_alg_sel'==2 &",
    "compressed plan algorithm receipt",
)
reconcile_path.write_text(reconcile)

post_path = Path("varcomp_kss/_vckss_rust_post_comp_v7.ado")
post = post_path.read_text()
post = replace_once(
    post,
    "`cap_algorithm'==2 & `cap_deletion'==1 &",
    "`cap_algorithm'==`h_algreq' & `cap_deletion'==1 &",
    "compressed poster capability algorithm",
)
post_path.write_text(post)

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
test = test_path.read_text()
insert_anchor = "// The public engine-auto boundary must preserve the compressed result family.\n"
if test.count(insert_anchor) != 1:
    raise SystemExit("algorithm-auto compressed test insertion anchor changed")
auto_test = r'''// The native V3 planner also accepts an explicit algorithm(auto) request.
// Force the retained identified dimension above exact_limit() so this case
// exercises requested-auto/selected-JLA without opening the public router yet.
tempvar auto_algorithm_touse
generate byte `auto_algorithm_touse' = 1
capture noisily _vckss_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `auto_algorithm_touse' `nscope' `ncomplete'     ///
    `nstayers' `nstayerrows' 7 2 81227 1e-12 10000 1 auto auto             ///
    1 1 1 1 1 1 1 0 `core_flags' `support_flags' "nodisplay"             ///
    match joint 2 1e-10 1e-10 5000 50000000 "" 1 1                       ///
    "varcomp_kss outcome [fw=frequency], backend(rust) algorithm(auto) engine(auto)" ///
    auto auto 0 0
assert _rc == 0
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "exact"
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 2
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 2
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_resolved) == 1
assert e(rust_plan_frozen) == 1
assert e(rust_plan_applicability) == 2
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 1
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 1
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_cap_schema) == 3
assert e(rust_cap_profile_code) == 4
assert e(rust_cap_engine_deferred) == 1
tempname auto_algorithm_cap
auto_algorithm_cap = 0
matrix `auto_algorithm_cap' = e(rust_request_capability_receipt)
assert `auto_algorithm_cap'[1,7] == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local auto_algorithm_sortedby : sortedby
assert `"`auto_algorithm_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

'''
# Remove a harmless Python-like assignment before writing Stata source.
auto_test = auto_test.replace("auto_algorithm_cap = 0\n", "")
test_path.write_text(test.replace(insert_anchor, auto_test + insert_anchor))
