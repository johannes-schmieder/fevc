from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
text = path.read_text()
old = '''local sortedby_after : sortedby
assert `"`sortedby_after'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// The public engine-auto boundary must preserve the compressed result family
'''
new = '''local sortedby_after : sortedby
assert `"`sortedby_after'"' == `"`caller_sortedby'"'
quietly count if touse != 1
assert r(N) == 0
// This standalone internal call posts e(sample) against the caller-owned
// touse variable.  Its result and sample have been fully checked above; clear
// that internal estimation result before comparing the raw caller dataset.
// The actual public commands below retain their active e(sample) while their
// complete data-restoration signatures are checked.
quietly ereturn clear
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// The public engine-auto boundary must preserve the compressed result family
'''
if text.count(old) != 1:
    raise SystemExit("compressed restoration anchor changed")
path.write_text(text.replace(old, new))
