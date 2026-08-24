from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed.do")
text = path.read_text()
old = """    `prepared_workers' `prepared_firms' 1e-10 1e-10 1 2 0 1 2 2 1 2 1 ///
"""
new = """    `prepared_workers' `prepared_firms' 1e-10 1e-10 2 1 2 0 1 2 2 1 2 1 ///
"""
if text.count(old) != 1:
    raise SystemExit(
        f"compressed V7 direct-call anchor changed: found {text.count(old)}"
    )
path.write_text(text.replace(old, new))
