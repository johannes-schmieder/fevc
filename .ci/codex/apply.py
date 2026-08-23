from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text()
old = '''local compressed_preauto_sortedby_after : sortedby
assert `"`compressed_preauto_sortedby_after'"' == `"`compressed_preauto_sortedby'"'
'''
new = '''local compressed_preauto_sort_after : sortedby
assert `"`compressed_preauto_sort_after'"' == `"`compressed_preauto_sortedby'"'
'''
if text.count(old) != 1:
    raise SystemExit("compressed automatic-route restoration macro anchor changed")
path.write_text(text.replace(old, new))
