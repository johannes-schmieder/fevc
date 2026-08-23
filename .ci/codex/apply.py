from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_v4.do")
text = path.read_text(encoding="utf-8")
old = "assert scalar(__vckss_rust_full_route) == 3\n"
new = "assert scalar(__vckss_rust_full_route) == 2\n"
if text.count(old) != 1:
    raise RuntimeError(
        f"frozen full-route prefix: expected one assertion, found {text.count(old)}"
    )
path.write_text(text.replace(old, new, 1), encoding="utf-8")
print("preserved the frozen V6 full-route prefix in forced-CMG regression")
