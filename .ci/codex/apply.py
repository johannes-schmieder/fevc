from pathlib import Path

path = Path("varcomp_kss/_vckss_rust_post_exact_v7.ado")
text = path.read_text(encoding="utf-8")
anchor = '    ereturn local route_api "VCKSS-NATIVE-EXACT-PLANNED-V4-V7"\n'
replacement = anchor + '    ereturn local result_family "exact"\n'
if text.count(anchor) != 1:
    raise SystemExit(f"exact route-api anchor changed: found {text.count(anchor)}")
path.write_text(text.replace(anchor, replacement, 1), encoding="utf-8")
