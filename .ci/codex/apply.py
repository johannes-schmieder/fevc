from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed.do")
text = path.read_text()
anchor = """quietly _vckss_rust_reconcile_comp_v7 `probes' 81227 10000 1e-12 ///
    `prepared_workers' `prepared_firms' 1e-10 1e-10 1 2 0 1 2 2 1 2 1 ///
    `prepared_memory_limit' `prepared_input_copy' `prepared_peak'       ///
    `prepared_resident' `signature_hi' `signature_lo' 50000000 0 0
assert r(ok) == 1
"""
replacement = """quietly _vckss_rust_reconcile_comp_v7 `probes' 81227 10000 1e-12 ///
    `prepared_workers' `prepared_firms' 1e-10 1e-10 1 2 0 1 2 2 1 2 1 ///
    `prepared_memory_limit' `prepared_input_copy' `prepared_peak'       ///
    `prepared_resident' `signature_hi' `signature_lo' 50000000 0 0
local compressed_reconcile_ok = r(ok)
local compressed_reconcile_detail `"`r(detail)'"'
if `compressed_reconcile_ok' != 1 {
    di as error "COMPRESSED_V7_RECONCILE_FAIL: `compressed_reconcile_detail'"
    return list
}
assert `compressed_reconcile_ok' == 1
"""
if text.count(anchor) != 1:
    raise SystemExit("compressed reconcile assertion anchor changed")
path.write_text(text.replace(anchor, replacement))
