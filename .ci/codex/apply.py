from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    path.write_text(text.replace(old, new), encoding="utf-8")


pkg = Path("varcomp_kss/varcomp_kss.pkg")
replace_once(
    pkg,
    "f _vckss_rust_reconcile_comp_v7.ado\nf _vckss_rust_macos.ado",
    "f _vckss_rust_reconcile_comp_v7.ado\n"
    "f _vckss_rust_post_comp_v7.ado\n"
    "f _vckss_rust_macos.ado",
    "package compressed poster",
)

post_test = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do")
replace_once(
    post_test,
    'version 18.0\n'
    'clear all\n'
    'set more off\n\n'
    'local package_dir = subinstr("`c(pwd)\'","/tests/stata","",.)\n'
    'adopath ++ "`package_dir\'"\n',
    'version 18.0\n'
    'clear all\n'
    'set more off\n\n'
    'args package_dir\n'
    'if `"`package_dir\'"\' == "" {\n'
    '    local package_dir = subinstr("`c(pwd)\'","/tests/stata","",.)\n'
    '}\n'
    'adopath ++ `"`package_dir\'"\'\n',
    "standalone compressed post package argument",
)

install_test = Path("varcomp_kss/tests/stata/test_rust_public_install.do")
replace_once(
    install_test,
    "    _vckss_rust_plugin_call.ado _vckss_rust_solve_v4.ado ///\n"
    "    _vckss_rust_plan_receipt.ado _vckss_rust_macos.ado   ///\n"
    "    _vckss_rust_windows.ado _vckss_rust_linux.ado        ///\n",
    "    _vckss_rust_plugin_call.ado _vckss_rust_solve_v4.ado ///\n"
    "    _vckss_rust_plan_receipt.ado                         ///\n"
    "    _vckss_rust_reconcile_comp_v7.ado                    ///\n"
    "    _vckss_rust_post_comp_v7.ado _vckss_rust_macos.ado   ///\n"
    "    _vckss_rust_windows.ado _vckss_rust_linux.ado        ///\n",
    "clean-install compressed helpers",
)
replace_once(
    install_test,
    "        test_rust_exact_controls.do test_rust_generic_jla.do    ///\n"
    "        test_rust_planned_v4.do test_rust_public_exact.do       ///\n"
    "        test_rust_public_generic.do {\n",
    "        test_rust_exact_controls.do test_rust_generic_jla.do    ///\n"
    "        test_rust_planned_v4.do test_rust_planned_compressed.do ///\n"
    "        test_rust_planned_compressed_post.do                    ///\n"
    "        test_rust_public_exact.do test_rust_public_generic.do {\n",
    "clean-install compressed route tests",
)

qualifier = Path("rust/stata_backend/qualify_macos.sh")
replace_once(
    qualifier,
    "        'source-local Rust developer routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned compressed and generic JLA V4/V7 with frozen engine and route receipts, independent batching, and wall advisory; support mask 38 plus request-capability receipts'",
    "        'source-local Rust developer routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, planned compressed and generic JLA V4/V7, and public backend(rust) engine(auto) compressed no-control match with automatic diagonal and forced CMG routes, independent or numeric batching, and wall advisory; support mask 38 plus request-capability receipts'",
    "available qualification scope",
)
replace_once(
    qualifier,
    "        'source-local Rust developer routes tested on macOS arm64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned compressed and generic JLA V4/V7 with frozen engine and route receipts, independent batching, and wall advisory; x86_64 runtime untested; support mask 38 plus request-capability receipts'",
    "        'source-local Rust developer routes tested on macOS arm64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, planned compressed and generic JLA V4/V7, and public backend(rust) engine(auto) compressed no-control match with automatic diagonal and forced CMG routes, independent or numeric batching, and wall advisory; x86_64 runtime untested; support mask 38 plus request-capability receipts'",
    "unavailable qualification scope",
)
replace_once(
    qualifier,
    '  [[ "${available}" == *\'planned compressed and generic JLA V4/V7\'* ]] || \\\n'
    '    fail "available receipt scope omitted compressed V4/V7 qualification"\n',
    '  [[ "${available}" == *\'planned compressed and generic JLA V4/V7\'* ]] || \\\n'
    '    fail "available receipt scope omitted compressed V4/V7 qualification"\n'
    '  [[ "${available}" == *\'public backend(rust) engine(auto) compressed\'* ]] || \\\n'
    '    fail "available receipt scope omitted public compressed qualification"\n',
    "qualification selftest public compressed",
)
replace_once(
    qualifier,
    '  "${package_dir}/_vckss_rust_plan_receipt.ado"\n'
    '  "${package_dir}/_vckss_rust_macos.ado"\n',
    '  "${package_dir}/_vckss_rust_plan_receipt.ado"\n'
    '  "${package_dir}/_vckss_rust_reconcile_comp_v7.ado"\n'
    '  "${package_dir}/_vckss_rust_post_comp_v7.ado"\n'
    '  "${package_dir}/_vckss_rust_macos.ado"\n',
    "qualifier compressed helper manifest",
)
replace_once(
    qualifier,
    '  "${package_dir}/tests/stata/test_rust_planned_compressed.do"\n'
    '  "${package_dir}/tests/stata/test_rust_public_exact.do"\n',
    '  "${package_dir}/tests/stata/test_rust_planned_compressed.do"\n'
    '  "${package_dir}/tests/stata/test_rust_planned_compressed_post.do"\n'
    '  "${package_dir}/tests/stata/test_rust_public_exact.do"\n',
    "qualifier compressed post test manifest",
)

case_insertions = [
    (
        'run_stata_case arm64 private-planned-compressed \\\n'
        '  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '  \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${test_package_dir}"\n'
        'run_stata_case arm64 public-exact \\\n',
        'run_stata_case arm64 private-planned-compressed \\\n'
        '  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '  \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${test_package_dir}"\n'
        'run_stata_case arm64 public-planned-compressed \\\n'
        '  "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \\\n'
        '  \'VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\' "${test_package_dir}"\n'
        'run_stata_case arm64 public-exact \\\n',
        "arm64 thin public compressed case",
    ),
    (
        'run_stata_case arm64 universal-private-planned-compressed \\\n'
        '  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '  \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${universal_test_package_dir}"\n'
        'run_stata_case arm64 universal-public-exact \\\n',
        'run_stata_case arm64 universal-private-planned-compressed \\\n'
        '  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '  \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${universal_test_package_dir}"\n'
        'run_stata_case arm64 universal-public-planned-compressed \\\n'
        '  "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \\\n'
        '  \'VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\' "${universal_test_package_dir}"\n'
        'run_stata_case arm64 universal-public-exact \\\n',
        "arm64 universal public compressed case",
    ),
    (
        '  run_stata_case x86_64 private-planned-compressed \\\n'
        '    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '    \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${test_package_dir}"\n'
        '  run_stata_case x86_64 public-exact \\\n',
        '  run_stata_case x86_64 private-planned-compressed \\\n'
        '    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '    \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${test_package_dir}"\n'
        '  run_stata_case x86_64 public-planned-compressed \\\n'
        '    "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \\\n'
        '    \'VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\' "${test_package_dir}"\n'
        '  run_stata_case x86_64 public-exact \\\n',
        "x86 thin public compressed case",
    ),
    (
        '  run_stata_case x86_64 universal-private-planned-compressed \\\n'
        '    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '    \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${universal_test_package_dir}"\n'
        '  run_stata_case x86_64 universal-public-exact \\\n',
        '  run_stata_case x86_64 universal-private-planned-compressed \\\n'
        '    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\\n'
        '    \'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\' "${universal_test_package_dir}"\n'
        '  run_stata_case x86_64 universal-public-planned-compressed \\\n'
        '    "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \\\n'
        '    \'VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\' "${universal_test_package_dir}"\n'
        '  run_stata_case x86_64 universal-public-exact \\\n',
        "x86 universal public compressed case",
    ),
]
for old, new, label in case_insertions:
    replace_once(qualifier, old, new, label)

replace_once(
    qualifier,
    "  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid;planned-compressed-jla-v4-v7-engine-auto-to-compressed-route-diagonal-explicit-batches-counter-v1-fweights-stored-targetweights-matchid;planned-generic-jla-v4-v7-engine-generic-route-auto-independent-batches-wall-advisory-counter-v1\\n'\n",
    "  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid;planned-compressed-jla-v4-v7-engine-auto-to-compressed-route-diagonal-explicit-batches-counter-v1-fweights-stored-targetweights-matchid;public-compressed-jla-backend-rust-engine-auto-no-controls-match-joint-fixedoffset-auto-to-diagonal-forced-cmg-independent-numeric-batches-wall-advisory-counter-v1-fweights-stored-targetweights-matchid;planned-generic-jla-v4-v7-engine-generic-route-auto-independent-batches-wall-advisory-counter-v1\\n'\n",
    "receipt tested routes",
)
replace_once(
    qualifier,
    "  printf 'excluded_claims=public-release,production,Windows,Linux,native-Intel,public-automatic-Rust-routing,public-compressed-engine-auto,algorithm-auto,CMG,stayers,probeorder,scale,human-license-provenance-review\\n'\n",
    "  printf 'excluded_claims=public-release,production,Windows,Linux,native-Intel,backend-auto-to-Rust,algorithm-auto,stayers,probeorder,scale,human-license-provenance-review\\n'\n",
    "receipt excluded claims",
)

receipt_insertions = [
    (
        "  printf 'arm64_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n",
        "  printf 'arm64_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n"
        "  printf 'arm64_public_planned_compressed=VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\\n'\n",
        "arm64 thin receipt field",
    ),
    (
        "  printf 'arm64_universal_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n",
        "  printf 'arm64_universal_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n"
        "  printf 'arm64_universal_public_planned_compressed=VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\\n'\n",
        "arm64 universal receipt field",
    ),
    (
        "    printf 'x86_64_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n",
        "    printf 'x86_64_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n"
        "    printf 'x86_64_public_planned_compressed=VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\\n'\n",
        "x86 thin receipt field",
    ),
    (
        "    printf 'x86_64_universal_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n",
        "    printf 'x86_64_universal_private_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n"
        "    printf 'x86_64_universal_public_planned_compressed=VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS\\n'\n",
        "x86 universal receipt field",
    ),
]
for old, new, label in receipt_insertions:
    replace_once(qualifier, old, new, label)

command_insertions = [
    (
        "  printf 'command.test_arm64_private_planned_compressed=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-thin-package>\\n'\n",
        "  printf 'command.test_arm64_private_planned_compressed=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-thin-package>\\n'\n"
        "  printf 'command.test_arm64_public_planned_compressed=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed_post.do <temporary-thin-package>\\n'\n",
        "arm64 thin command receipt",
    ),
    (
        "  printf 'command.test_arm64_universal_private_planned_compressed=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-universal-package>\\n'\n",
        "  printf 'command.test_arm64_universal_private_planned_compressed=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-universal-package>\\n'\n"
        "  printf 'command.test_arm64_universal_public_planned_compressed=arch -arm64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed_post.do <temporary-universal-package>\\n'\n",
        "arm64 universal command receipt",
    ),
    (
        "    printf 'command.test_x86_64_private_planned_compressed=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-thin-package>\\n'\n",
        "    printf 'command.test_x86_64_private_planned_compressed=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-thin-package>\\n'\n"
        "    printf 'command.test_x86_64_public_planned_compressed=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed_post.do <temporary-thin-package>\\n'\n",
        "x86 thin command receipt",
    ),
    (
        "    printf 'command.test_x86_64_universal_private_planned_compressed=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-universal-package>\\n'\n",
        "    printf 'command.test_x86_64_universal_private_planned_compressed=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do <temporary-universal-package>\\n'\n"
        "    printf 'command.test_x86_64_universal_public_planned_compressed=arch -x86_64 <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed_post.do <temporary-universal-package>\\n'\n",
        "x86 universal command receipt",
    ),
]
for old, new, label in command_insertions:
    replace_once(qualifier, old, new, label)
