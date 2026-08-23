from pathlib import Path

production = Path("varcomp_kss/varcomp_kss.ado")
text = production.read_text(encoding="utf-8")
old = """        local active_batch = cond(`phase'==1,`r_lev_batch',`r_tgt_batch')
"""
new = """        local active_batch = cond(`phase'==2,`r_lev_batch',`r_tgt_batch')
"""
if text.count(old) != 1:
    raise RuntimeError(f"planned phase batch selector: expected one block, found {text.count(old)}")
text = text.replace(old, new, 1)
production.write_text(text, encoding="utf-8")

test_path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
test = test_path.read_text(encoding="utf-8")
anchor = r'''assert e(batch) == max(e(leverage_batch),e(target_batch))
'''
addition = r'''assert e(batch) == max(e(leverage_batch),e(target_batch))
tempname planned_rhs_public
matrix `planned_rhs_public' = e(solver_rhs_diagnostics)
forvalues row = 1/`=rowsof(`planned_rhs_public')' {
    local planned_stage = `planned_rhs_public'[`row',1]
    local planned_rhs = `planned_rhs_public'[`row',3]
    if `planned_stage' == 4 {
        local planned_probe = `planned_rhs'-1
        local planned_start = floor(`planned_probe'/e(leverage_batch))* ///
            e(leverage_batch)+1
        assert `planned_rhs_public'[`row',2] == `planned_start'
    }
    else if `planned_stage' == 5 {
        local planned_probe = floor((`planned_rhs'-1)/2)
        local planned_start = floor(`planned_probe'/e(target_batch))* ///
            e(target_batch)+1
        assert `planned_rhs_public'[`row',2] == `planned_start'
    }
    else assert `planned_rhs_public'[`row',2] == 1
}
'''
if test.count(anchor) != 1:
    raise RuntimeError(f"planned public batch receipt anchor: expected one match, found {test.count(anchor)}")
test = test.replace(anchor, addition, 1)
test_path.write_text(test, encoding="utf-8")
print("repaired planned phase batch starts and added public receipt checks")
