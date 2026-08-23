from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_v4.do")
text = path.read_text(encoding="utf-8")

old = "// the V6 route prefix is deliberately diagonal, while V7 and every RHS row\n// carry the actual forced-CMG route."
new = "// the frozen V6 result and V2 RHS prefixes deliberately remain diagonal,\n// while V7 carries the actual forced-CMG route."
if text.count(old) != 1:
    raise RuntimeError("forced-CMG compatibility comment did not match exactly once")
text = text.replace(old, new, 1)

old = "    assert `cmg_rhs_receipts'[`row',4] == 3"
new = "    assert `cmg_rhs_receipts'[`row',4] == 2"
if text.count(old) != 1:
    raise RuntimeError("forced-CMG RHS route assertion did not match exactly once")
text = text.replace(old, new, 1)

path.write_text(text, encoding="utf-8")
