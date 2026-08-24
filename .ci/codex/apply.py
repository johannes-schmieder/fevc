from pathlib import Path


def insert_once(text: str, anchor: str, addition: str, label: str) -> str:
    if addition in text:
        raise SystemExit(f"{label}: addition is already present")
    count = text.count(anchor)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    return text.replace(anchor, anchor + addition)


reconcile = Path("varcomp_kss/_vckss_rust_reconcile_exact_v7.ado")
poster = Path("varcomp_kss/_vckss_rust_post_exact_v7.ado")
for required in (reconcile, poster):
    if not required.is_file():
        raise SystemExit(f"planned exact helper is absent: {required}")

pkg_path = Path("varcomp_kss/varcomp_kss.pkg")
pkg = pkg_path.read_text()
pkg = insert_once(
    pkg,
    "f _vckss_rust_reconcile_exact_v7.ado\n",
    "f _vckss_rust_post_exact_v7.ado\n",
    "package manifest",
)
pkg_path.write_text(pkg)

install_path = Path("varcomp_kss/tests/stata/test_rust_public_install.do")
install = install_path.read_text()
old = """    _vckss_rust_plan_receipt.ado                         ///
    _vckss_rust_reconcile_comp_v7.ado                    ///
    _vckss_rust_post_comp_v7.ado _vckss_rust_macos.ado   ///
"""
new = """    _vckss_rust_plan_receipt.ado                         ///
    _vckss_rust_reconcile_comp_v7.ado                    ///
    _vckss_rust_reconcile_exact_v7.ado                   ///
    _vckss_rust_post_comp_v7.ado                         ///
    _vckss_rust_post_exact_v7.ado _vckss_rust_macos.ado  ///
"""
if install.count(old) != 1:
    raise SystemExit(f"clean-install helper anchor changed: found {install.count(old)}")
install_path.write_text(install.replace(old, new))

qualifier_path = Path("rust/stata_backend/qualify_macos.sh")
qualifier = qualifier_path.read_text()
old = """  \"${package_dir}/_vckss_rust_plan_receipt.ado\"
  \"${package_dir}/_vckss_rust_reconcile_comp_v7.ado\"
  \"${package_dir}/_vckss_rust_post_comp_v7.ado\"
"""
new = """  \"${package_dir}/_vckss_rust_plan_receipt.ado\"
  \"${package_dir}/_vckss_rust_reconcile_comp_v7.ado\"
  \"${package_dir}/_vckss_rust_reconcile_exact_v7.ado\"
  \"${package_dir}/_vckss_rust_post_comp_v7.ado\"
  \"${package_dir}/_vckss_rust_post_exact_v7.ado\"
"""
if qualifier.count(old) != 1:
    raise SystemExit(f"qualifier source-manifest anchor changed: found {qualifier.count(old)}")
qualifier_path.write_text(qualifier.replace(old, new))
