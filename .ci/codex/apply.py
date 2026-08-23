from pathlib import Path

test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
test = test_path.read_text()
old = """adopath ++ `\"`package_dir'\"'\n\nset obs 96\n"""
new = """adopath ++ `\"`package_dir'\"'\nquietly run `\"`package_dir'/varcomp_kss.ado\"'\n\nset obs 96\n"""
if test.count(old) != 1:
    raise SystemExit("public compressed standalone loader anchor changed")
test_path.write_text(test.replace(old, new))
