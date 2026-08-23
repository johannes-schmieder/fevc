from pathlib import Path

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed.do")
test = test_path.read_text()
old = "assert r(selected_engine) == 1\n"
new = "assert r(selected_engine_code) == 1\n"
if test.count(old) != 1:
    raise SystemExit("compressed helper selected-engine assertion anchor changed")
test_path.write_text(test.replace(old, new))
