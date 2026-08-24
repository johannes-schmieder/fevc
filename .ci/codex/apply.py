from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
text = path.read_text(encoding="utf-8")
marker = "// Planned algorithm(auto) selecting exact: direct V4/V7 Stata bridge certificate."
if text.count(marker) != 1:
    raise SystemExit("planned auto-exact bridge block is absent or duplicated")
old = """local exact_sortedby_after : sortedby
assert `\"`exact_sortedby_after'\"' == `\"`exact_sortedby'\"'
quietly _datasignature
assert `\"`r(datasignature)'\"' == `\"`exact_signature'\"'
restore
"""
new = """local exact_sortedby_after : sortedby
assert `\"`exact_sortedby_after'\"' == `\"`exact_sortedby'\"'
capture drop `xkeep'
quietly _datasignature
assert `\"`r(datasignature)'\"' == `\"`exact_signature'\"'
restore
"""
if text.count(old) != 1:
    raise SystemExit("exact bridge signature anchor changed")
path.write_text(text.replace(old, new), encoding="utf-8")
