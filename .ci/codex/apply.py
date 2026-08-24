from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


reconcile_path = Path("varcomp_kss/_vckss_rust_reconcile_exact_v7.ado")
replace_once(
    reconcile_path,
    """    args algreq engreq delcode nuiscode workers firms controls ranktol ///
        blocktol tolerance memlimit inputcopy preppeak resident sighi  ///
""",
    """    args algreq engreq delcode nuiscode workers firms controls ranktol ///
        blocktol tolerance exactlimit memlimit inputcopy preppeak resident ///
        sighi                                                            ///
""",
    "exact reconciler argument list",
)
replace_once(
    reconcile_path,
    """        plan_alg_req:r_palgreq plan_alg_sel:r_palgsel                 ///
        plan_eng_req:r_pengreq plan_eng_sel:r_pengsel                 ///
""",
    """        plan_alg_req:r_palgreq plan_alg_sel:r_palgsel                 ///
        plan_alg_reason:r_palgreason plan_eng_req:r_pengreq           ///
        plan_eng_sel:r_pengsel plan_eng_reason:r_pengreason           ///
        plan_comp_elig:r_pcompelig plan_complexity:r_pcomplexity      ///
        plan_exact_limit:r_pexactlimit                               ///
""",
    "exact reconciler plan fields",
)
replace_once(
    reconcile_path,
    """        ranktol blocktol tolerance memlimit inputcopy preppeak resident    ///
        sighi siglo physlimit wallsup wallvalue targetmode delsource frequse {
""",
    """        ranktol blocktol tolerance exactlimit memlimit inputcopy preppeak  ///
        resident sighi siglo physlimit wallsup wallvalue targetmode       ///
        delsource frequse {
""",
    "exact reconciler expected arguments",
)
replace_once(
    reconcile_path,
    """        `tolerance'<=0 | `memlimit'<=0 | `memlimit'!=floor(`memlimit') | ///
""",
    """        `tolerance'<=0 | `exactlimit'<2 | `exactlimit'>2000 |       ///
        `exactlimit'!=floor(`exactlimit') | `memlimit'<=0 |           ///
        `memlimit'!=floor(`memlimit') |                               ///
""",
    "exact-limit argument validation",
)
replace_once(
    reconcile_path,
    """        r_planschema r_prouteschema r_presolved r_pfrozen r_papp       ///
        r_palgreq r_palgsel r_pengreq r_pengsel r_proutereq r_proutesel ///
""",
    """        r_planschema r_prouteschema r_presolved r_pfrozen r_papp       ///
        r_palgreq r_palgsel r_palgreason r_pengreq r_pengsel          ///
        r_pengreason r_pcompelig r_pcomplexity r_pexactlimit          ///
        r_proutereq r_proutesel                                      ///
""",
    "exact reconciler receipt names",
)
replace_once(
    reconcile_path,
    """            `r_papp'==1 & `r_palgreq'==`algreq' & `r_palgsel'==1 & ///
            `r_pengreq'==`engreq' & `r_pengsel'==3 &               ///
            `r_proutereq'==4 & `r_proutesel'==4 &                  ///
""",
    """            `r_papp'==1 & `r_palgreq'==`algreq' & `r_palgsel'==1 & ///
            `r_palgreason'==cond(`algreq'==0,3,1) &                 ///
            `r_pengreq'==`engreq' & `r_pengsel'==3 &               ///
            `r_pengreason'==1 & `r_pcompelig'==0 &                  ///
            `r_pcomplexity'==`fullparams' &                         ///
            `r_pexactlimit'==`exactlimit' &                         ///
            `r_proutereq'==4 & `r_proutesel'==4 &                  ///
""",
    "exact plan selection reconciliation",
)
replace_once(
    reconcile_path,
    """    return scalar plan_algorithm_requested = `r_palgreq'
    return scalar plan_algorithm_selected = `r_palgsel'
    return scalar plan_engine_requested = `r_pengreq'
    return scalar plan_engine_selected = `r_pengsel'
""",
    """    return scalar plan_algorithm_requested = `r_palgreq'
    return scalar plan_algorithm_selected = `r_palgsel'
    return scalar plan_algorithm_reason = `r_palgreason'
    return scalar plan_engine_requested = `r_pengreq'
    return scalar plan_engine_selected = `r_pengsel'
    return scalar plan_engine_reason = `r_pengreason'
    return scalar plan_compressed_eligibility = `r_pcompelig'
    return scalar plan_complexity = `r_pcomplexity'
    return scalar plan_exact_limit = `r_pexactlimit'
""",
    "exact reconciler returned plan facts",
)

poster_path = Path("varcomp_kss/_vckss_rust_post_exact_v7.ado")
replace_once(
    poster_path,
    """        ranktol blocktol physicallimit preconditionerrequested         ///
""",
    """        ranktol blocktol exactlimit physicallimit preconditionerrequested ///
""",
    "exact poster argument list",
)
replace_once(
    poster_path,
    """        !missing(`delcode') & !missing(`nuiscode') &                 ///
        "`preconditionerrequested'"=="auto" &                       ///
""",
    """        !missing(`delcode') & !missing(`nuiscode') &                 ///
        `exactlimit'>=2 & `exactlimit'<=2000 &                       ///
        `exactlimit'==floor(`exactlimit') &                          ///
        "`preconditionerrequested'"=="auto" &                       ///
""",
    "exact poster exact-limit validation",
)
replace_once(
    poster_path,
    """        `ranktol' `blocktol' `tolerancerequested' `p_mem_limit'      ///
""",
    """        `ranktol' `blocktol' `tolerancerequested' `exactlimit'       ///
        `p_mem_limit'                                                 ///
""",
    "exact poster reconciler call",
)
replace_once(
    poster_path,
    """        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel      ///
""",
    """        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
        plan_alg_reason:r_plan_alg_reason                            ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel      ///
        plan_eng_reason:r_plan_eng_reason                            ///
        plan_comp_elig:r_plan_comp_elig                              ///
        plan_complexity:r_plan_complexity                            ///
        plan_exact_limit:r_plan_exact_limit                          ///
""",
    "exact poster plan collection",
)
replace_once(
    poster_path,
    """        `r_plan_applicability'==1 & `r_plan_resolved'==1 &          ///
        `r_plan_frozen'==1 & `r_ctr_complete'==1 &                  ///
""",
    """        `r_plan_applicability'==1 & `r_plan_resolved'==1 &          ///
        `r_plan_frozen'==1 & `r_plan_alg_reason'==3 &               ///
        `r_plan_engine_reason'==1 & `r_plan_comp_elig'==0 &         ///
        `r_plan_complexity'==`p_workers'+`p_firms'-1+`p_controls' & ///
        `r_plan_exact_limit'==`exactlimit' &                         ///
        `r_ctr_complete'==1 &                                       ///
""",
    "exact poster plan context gate",
)
replace_once(
    poster_path,
    """    ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
    ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
    ereturn scalar rust_plan_engine_requested = `r_plan_eng_req'
    ereturn scalar rust_plan_engine_selected = `r_plan_eng_sel'
""",
    """    ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
    ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
    ereturn scalar rust_plan_algorithm_reason = `r_plan_alg_reason'
    ereturn scalar rust_plan_engine_requested = `r_plan_eng_req'
    ereturn scalar rust_plan_engine_selected = `r_plan_eng_sel'
    ereturn scalar rust_plan_engine_reason = `r_plan_eng_reason'
    ereturn scalar rust_plan_compressed_eligibility = `r_plan_comp_elig'
    ereturn scalar rust_plan_complexity = `r_plan_complexity'
    ereturn scalar rust_plan_exact_limit = `r_plan_exact_limit'
""",
    "exact poster public plan facts",
)

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
replace_once(
    test_path,
    """    1e-10 1e-10 1e-12 `xmem' `xcopy' `xprep' `xresident'            ///
""",
    """    1e-10 1e-10 1e-12 500 `xmem' `xcopy' `xprep' `xresident'        ///
""",
    "direct exact reconciler exact-limit argument",
)
replace_once(
    test_path,
    """assert r(plan_algorithm_selected) == 1
assert r(plan_engine_requested) == 0
assert r(plan_engine_selected) == 3
""",
    """assert r(plan_algorithm_selected) == 1
assert r(plan_algorithm_reason) == 3
assert r(plan_engine_requested) == 0
assert r(plan_engine_selected) == 3
assert r(plan_engine_reason) == 1
assert r(plan_compressed_eligibility) == 0
assert r(plan_complexity) == 15
assert r(plan_exact_limit) == 500
""",
    "direct exact reconciler plan facts",
)
replace_once(
    test_path,
    """    joint 1e-10 1e-10 50000000 auto auto 1                         ///
""",
    """    joint 1e-10 1e-10 500 50000000 auto auto 1                     ///
""",
    "exact poster exact-limit argument",
)
replace_once(
    test_path,
    """assert e(rust_plan_algorithm_selected) == 1
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 3
""",
    """assert e(rust_plan_algorithm_selected) == 1
assert e(rust_plan_algorithm_reason) == 3
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 3
assert e(rust_plan_engine_reason) == 1
assert e(rust_plan_compressed_eligibility) == 0
assert e(rust_plan_complexity) == 15
assert e(rust_plan_exact_limit) == 500
""",
    "posted exact plan facts",
)

hits = {}
for path in Path("varcomp_kss").rglob("*"):
    if path.suffix not in {".ado", ".do"} or not path.is_file():
        continue
    count = path.read_text(encoding="utf-8").count("_vckss_rust_reconcile_exact_v7")
    if count:
        hits[str(path)] = count
expected = {
    "varcomp_kss/_vckss_rust_reconcile_exact_v7.ado": 1,
    "varcomp_kss/_vckss_rust_post_exact_v7.ado": 1,
    "varcomp_kss/tests/stata/test_rust_planned_compressed_post.do": 1,
}
if hits != expected:
    raise SystemExit(f"unexpected exact V7 reconciler call sites: {hits}")
