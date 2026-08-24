from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    return text.replace(old, new)


plan_path = Path("varcomp_kss/_vckss_rust_plan_receipt.ado")
plan = plan_path.read_text()
plan = replace_once(
    plan,
    """        local invariant_names plan_struct plan_schema plan_alg_schema       ///
            plan_eng_schema plan_route_schema batch_schema wall_schema     ///
            ctr_schema mem_schema plan_resolved plan_frozen wall_routing   ///
            ctr_complete
        local invariant_values 1000 1 1 2 2 1 1 1 1 1 1 1 1 1
""",
    """        local invariant_names plan_struct plan_schema plan_alg_schema       ///
            plan_eng_schema batch_schema wall_schema ctr_schema mem_schema ///
            plan_resolved plan_frozen wall_routing ctr_complete
        local invariant_values 1000 1 1 2 1 1 1 1 1 1 1 1 1
""",
    "fixed schema invariants",
)
plan = replace_once(
    plan,
    """        else {
            local expected_batched = (`applicability' != 1)
            if scalar(__vckss_batch_determ) != `expected_batched' |      ///
""",
    """        else {
            local expected_route_schema = cond(`applicability'==1,1,2)
            local expected_batched = (`applicability' != 1)
            if scalar(__vckss_plan_route_schema) !=                      ///
                    `expected_route_schema' |                            ///
                scalar(__vckss_batch_determ) != `expected_batched' |    ///
""",
    "applicability-specific route schema",
)
plan_path.write_text(plan)


test_path = Path("varcomp_kss/tests/stata/test_rust_planned_v4.do")
test = test_path.read_text()
test = replace_once(
    test,
    "assert r(plan_route_schema) == 2\nassert r(plan_route_req) == 4 & r(plan_route_sel) == 4\n",
    "assert r(plan_route_schema) == 1\nassert r(plan_route_req) == 4 & r(plan_route_sel) == 4\n",
    "exact route schema assertion",
)
test_path.write_text(test)
