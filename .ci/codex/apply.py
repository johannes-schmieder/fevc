from pathlib import Path

helper_path = Path("varcomp_kss/_vckss_rust_reconcile_comp_v7.ado")
test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed.do")

helper = helper_path.read_text()
old_capture = """        leverage_batch_mode_code:r_lev_batch_mode                        ///
        target_batch_mode_code:r_tgt_batch_mode                          ///
"""
new_capture = """        batch_lev_mode:r_lev_batch_mode                                  ///
        batch_tgt_mode:r_tgt_batch_mode                                  ///
"""
if helper.count(old_capture) != 1:
    raise SystemExit("compressed V7 phase-mode capture anchor changed")
helper = helper.replace(old_capture, new_capture)
helper_path.write_text(helper)

test = test_path.read_text()
old_test = """assert r(batch_lev_sel) == 2
assert r(batch_tgt_sel) == 2
"""
new_test = """assert r(batch_lev_mode) == 1
assert r(batch_tgt_mode) == 1
assert r(batch_lev_sel) == 2
assert r(batch_tgt_sel) == 2
"""
if test.count(old_test) != 1:
    raise SystemExit("compressed V7 phase-mode assertion anchor changed")
test = test.replace(old_test, new_test)
test_path.write_text(test)
