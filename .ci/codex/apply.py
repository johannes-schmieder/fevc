from __future__ import annotations

from pathlib import Path


path = Path("rust/stata_backend/qualify_macos.sh")
text = path.read_text(encoding="utf-8")


def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    text = text.replace(old, new, 1)
    print(f"replaced {label}")


replace_once(
    """  done < <(find "${temporary_root}" -maxdepth 3 -type f -name '*.log' -print0)
""",
    """  done < <(
    find "${temporary_root}" -maxdepth 3 -type f \\
      \\( -name '*.log' -o -name 'console.txt' \\) -print0
  )
""",
    "sanitized Stata console collection",
)

replace_once(
    """  "${arm64_install_root}" qualified \\
  "${package_dir}/tests/stata/test_rust_public.do" \\
  "${package_dir}/tests/stata/test_rust_exact_controls.do" \\
  "${package_dir}/tests/stata/test_rust_generic_jla.do" \\
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
  "${package_dir}/tests/stata/test_rust_public_exact.do" \\
  "${package_dir}/tests/stata/test_rust_public_generic.do"
""",
    """  "${arm64_install_root}" qualified \\
  "${package_dir}/tests/stata"
""",
    "arm64 clean-install argument collapse",
)

replace_once(
    """    "${x86_64_install_root}" qualified \\
    "${package_dir}/tests/stata/test_rust_public.do" \\
    "${package_dir}/tests/stata/test_rust_exact_controls.do" \\
    "${package_dir}/tests/stata/test_rust_generic_jla.do" \\
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
    "${package_dir}/tests/stata/test_rust_public_exact.do" \\
    "${package_dir}/tests/stata/test_rust_public_generic.do"
""",
    """    "${x86_64_install_root}" qualified \\
    "${package_dir}/tests/stata"
""",
    "x86_64 clean-install argument collapse",
)

replace_once(
    """  printf 'command.test_arm64_clean_install=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_public_install.do <temporary-thin-package> <isolated-plus> qualified <compressed-test> <exact-controls-test> <private-generic-test> <public-exact-test> <public-generic-test>\\n'
""",
    """  printf 'command.test_arm64_clean_install=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_public_install.do <temporary-thin-package> <isolated-plus> qualified <test-root>\\n'
""",
    "arm64 receipt command",
)

replace_once(
    """    printf 'command.test_x86_64_clean_install=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_public_install.do <temporary-thin-package> <isolated-plus> qualified <compressed-test> <exact-controls-test> <private-generic-test> <public-exact-test> <public-generic-test>\\n'
""",
    """    printf 'command.test_x86_64_clean_install=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_public_install.do <temporary-thin-package> <isolated-plus> qualified <test-root>\\n'
""",
    "x86_64 receipt command",
)

replace_once(
    """    fail "Stata ${architecture} ${label} returned ${return_code}; raw log removed on exit"
""",
    """    fail "Stata ${architecture} ${label} returned ${return_code}; sanitized transcript exported on exit"
""",
    "Stata return-code diagnostic",
)
replace_once(
    """    fail "Stata ${architecture} ${label} omitted PASS marker; raw log removed on exit"
""",
    """    fail "Stata ${architecture} ${label} omitted PASS marker; sanitized transcript exported on exit"
""",
    "Stata marker diagnostic",
)

path.write_text(text, encoding="utf-8")
