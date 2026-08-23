from pathlib import Path

ado = Path("varcomp_kss/varcomp_kss_rust.ado")
text = ado.read_text(encoding="utf-8")

replacements = {
    "scalar(__vckss_rust_cap_lev_mode)": "scalar(__vckss_rust_cap_levmode)",
    "scalar(__vckss_rust_cap_tgt_mode)": "scalar(__vckss_rust_cap_tgtmode)",
    "scalar(__vckss_rust_cap_fallback)": "scalar(__vckss_rust_cap_autofallback)",
    "foreach name in lev_mode tgt_mode fallback wallseconds alg_deferred  ///":
        "foreach name in levmode tgtmode autofallback wallseconds alg_deferred  ///",
}

for old, new in replacements.items():
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one occurrence of {old!r}, found {count}")
    text = text.replace(old, new, 1)

for required in (
    "scalar(__vckss_rust_cap_levmode)",
    "scalar(__vckss_rust_cap_tgtmode)",
    "scalar(__vckss_rust_cap_autofallback)",
    "foreach name in levmode tgtmode autofallback wallseconds alg_deferred  ///",
):
    if text.count(required) != 1:
        raise RuntimeError(f"postcondition failed for {required!r}")

cshim = Path("rust/stata_backend/cshim/stata_entry.c").read_text(encoding="utf-8")
for exported in (
    '"__vckss_rust_cap_levmode"',
    '"__vckss_rust_cap_tgtmode"',
    '"__vckss_rust_cap_autofallback"',
):
    if cshim.count(exported) != 1:
        raise RuntimeError(f"C shim scalar contract changed for {exported}")

ado.write_text(text, encoding="utf-8")
print("aligned planned capability scalar names with the C shim")
