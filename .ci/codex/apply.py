from pathlib import Path

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
test = test_path.read_text()
old = """assert e(coefficient_cells) == 48
assert e(deletion_units) == 48
assert e(target_strata) == 48
assert e(target_weight_sum) == 96.25
"""
new = """assert e(coefficient_cells) == 48
assert e(deletion_units) == 48
assert e(target_strata) == 96
assert e(target_strata) == e(N_stored)
assert e(target_weight_sum) == 96.25
"""
if test.count(old) != 1:
    raise SystemExit("compressed public target-strata anchor changed")
test_path.write_text(test.replace(old, new))
