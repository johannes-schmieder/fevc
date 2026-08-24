from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
text = path.read_text(encoding="utf-8")
old = '''quietly _vckss_rust_reconcile_exact_v7 0 0 1 1 `xworkers' `xfirms' 0 ///
    1e-10 1e-10 1e-12 500 `xmem' `xcopy' `xprep' `xresident'        ///
    `xsighi' `xsiglo' 50000000 0 0 1 2 1
assert r(ok) == 1
'''
new = '''quietly _vckss_rust_reconcile_exact_v7 0 0 1 1 `xworkers' `xfirms' 0 ///
    1e-10 1e-10 1e-12 500 `xmem' `xcopy' `xprep' `xresident'        ///
    `xsighi' `xsiglo' 50000000 0 0 1 2 1
local exact_reconcile_ok = r(ok)
local exact_reconcile_detail `"`r(detail)'"'
if `exact_reconcile_ok' != 1 {
    di as error "EXACT_V7_RECONCILE_FAIL: `exact_reconcile_detail'"
    return list
}
assert `exact_reconcile_ok' == 1
'''
if text.count(old) != 1:
    raise SystemExit(f"exact V7 diagnostic anchor changed: found {text.count(old)}")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
