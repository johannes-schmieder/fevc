from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text()
load_line = "quietly run `\"`package_dir'/varcomp_kss.ado\"'\n"
if text.count(load_line) != 1:
    raise SystemExit(f"expected one mid-test source load, found {text.count(load_line)}")
text = text.replace(load_line, "")
setup_anchor = "adopath ++ `\"`package_dir'\"'\n\n"
if text.count(setup_anchor) != 1:
    raise SystemExit(f"generic test setup anchor changed: found {text.count(setup_anchor)}")
text = text.replace(setup_anchor, setup_anchor + load_line)
path.write_text(text)
