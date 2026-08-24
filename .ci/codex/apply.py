from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text()
anchor = """// Exercise requested algorithm(auto) with the generic JLA result family
// directly before opening the public router.  exact_limit(2) forces JLA;
"""
replacement = """// Exercise requested algorithm(auto) with the generic JLA result family
// directly before opening the public router.  exact_limit(2) forces JLA;
quietly run `\"`package_dir'/varcomp_kss.ado\"'
"""
if text.count(anchor) != 1:
    raise SystemExit(f"generic auto test anchor changed: found {text.count(anchor)}")
path.write_text(text.replace(anchor, replacement))
