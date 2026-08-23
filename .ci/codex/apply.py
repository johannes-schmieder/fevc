from __future__ import annotations

# Trusted one-shot transformation: bind planned V4/V7 into qualification.
from pathlib import Path


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source block, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"replaced {label}")


def replace_expected(path: Path, old: str, new: str, expected: int, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != expected:
        raise RuntimeError(f"{label}: expected {expected} source blocks, found {count}")
    path.write_text(text.replace(old, new), encoding="utf-8")
    print(f"replaced {label} ({expected})")


qualifier = Path("rust/stata_backend/qualify_macos.sh")

replace_once(
    qualifier,
    """        'source-local explicit Rust developer routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, and explicit generic diagonal numeric-batch Counter-V1; support mask 38 plus request-capability receipts'\n""",
    """        'source-local Rust developer routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned generic JLA V4/V7 with automatic route, independent batching, and wall advisory; support mask 38 plus request-capability receipts'\n""",
    "available qualifier scope",
)
replace_once(
    qualifier,
    """        'source-local explicit Rust developer routes tested on macOS arm64; exact, frozen compressed JLA, and explicit generic diagonal numeric-batch Counter-V1; x86_64 runtime untested; support mask 38 plus request-capability receipts'\n""",
    """        'source-local Rust developer routes tested on macOS arm64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned generic JLA V4/V7 with automatic route, independent batching, and wall advisory; x86_64 runtime untested; support mask 38 plus request-capability receipts'\n""",
    "arm64-only qualifier scope",
)

replace_once(
    qualifier,
    """  \"${package_dir}/varcomp_kss_rust.ado\"\n  \"${package_dir}/_vckss_rust_plugin_call.ado\"\n  \"${package_dir}/_vckss_rust_macos.ado\"\n""",
    """  \"${package_dir}/varcomp_kss_rust.ado\"\n  \"${package_dir}/_vckss_rust_plugin_call.ado\"\n  \"${package_dir}/_vckss_rust_solve_v4.ado\"\n  \"${package_dir}/_vckss_rust_plan_receipt.ado\"\n  \"${package_dir}/_vckss_rust_macos.ado\"\n""",
    "qualifier helper source inputs",
)
replace_once(
    qualifier,
    """  \"${package_dir}/tests/stata/test_rust_generic_jla.do\"\n  \"${package_dir}/tests/stata/test_rust_public_exact.do\"\n""",
    """  \"${package_dir}/tests/stata/test_rust_generic_jla.do\"\n  \"${package_dir}/tests/stata/test_rust_planned_v4.do\"\n  \"${package_dir}/tests/stata/test_rust_public_exact.do\"\n""",
    "qualifier planned test source input",
)

replace_once(
    qualifier,
    """run_stata_case arm64 private-generic \\\n  \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n  'VARCOMP_KSS RUST GENERIC JLA PASS' \"${test_package_dir}\"\nrun_stata_case arm64 public-exact \\\n""",
    """run_stata_case arm64 private-generic \\\n  \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n  'VARCOMP_KSS RUST GENERIC JLA PASS' \"${test_package_dir}\"\nrun_stata_case arm64 private-planned-v4 \\\n  \"${package_dir}/tests/stata/test_rust_planned_v4.do\" \\\n  'VARCOMP_KSS RUST PLANNED V4 PASS' \"${test_package_dir}\"\nrun_stata_case arm64 public-exact \\\n""",
    "arm64 thin planned V4 case",
)
replace_once(
    qualifier,
    """run_stata_case arm64 universal-private-generic \\\n  \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n  'VARCOMP_KSS RUST GENERIC JLA PASS' \"${universal_test_package_dir}\"\nrun_stata_case arm64 universal-public-exact \\\n""",
    """run_stata_case arm64 universal-private-generic \\\n  \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n  'VARCOMP_KSS RUST GENERIC JLA PASS' \"${universal_test_package_dir}\"\nrun_stata_case arm64 universal-private-planned-v4 \\\n  \"${package_dir}/tests/stata/test_rust_planned_v4.do\" \\\n  'VARCOMP_KSS RUST PLANNED V4 PASS' \"${universal_test_package_dir}\"\nrun_stata_case arm64 universal-public-exact \\\n""",
    "arm64 universal planned V4 case",
)
replace_once(
    qualifier,
    """  run_stata_case x86_64 private-generic \\\n    \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n    'VARCOMP_KSS RUST GENERIC JLA PASS' \"${test_package_dir}\"\n  run_stata_case x86_64 public-exact \\\n""",
    """  run_stata_case x86_64 private-generic \\\n    \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n    'VARCOMP_KSS RUST GENERIC JLA PASS' \"${test_package_dir}\"\n  run_stata_case x86_64 private-planned-v4 \\\n    \"${package_dir}/tests/stata/test_rust_planned_v4.do\" \\\n    'VARCOMP_KSS RUST PLANNED V4 PASS' \"${test_package_dir}\"\n  run_stata_case x86_64 public-exact \\\n""",
    "x86 thin planned V4 case",
)
replace_once(
    qualifier,
    """  run_stata_case x86_64 universal-private-generic \\\n    \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n    'VARCOMP_KSS RUST GENERIC JLA PASS' \"${universal_test_package_dir}\"\n  run_stata_case x86_64 universal-public-exact \\\n""",
    """  run_stata_case x86_64 universal-private-generic \\\n    \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n    'VARCOMP_KSS RUST GENERIC JLA PASS' \"${universal_test_package_dir}\"\n  run_stata_case x86_64 universal-private-planned-v4 \\\n    \"${package_dir}/tests/stata/test_rust_planned_v4.do\" \\\n    'VARCOMP_KSS RUST PLANNED V4 PASS' \"${universal_test_package_dir}\"\n  run_stata_case x86_64 universal-public-exact \\\n""",
    "x86 universal planned V4 case",
)

replace_once(
    qualifier,
    """  \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n  \"${package_dir}/tests/stata/test_rust_public_exact.do\" \\\n""",
    """  \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n  \"${package_dir}/tests/stata/test_rust_planned_v4.do\" \\\n  \"${package_dir}/tests/stata/test_rust_public_exact.do\" \\\n""",
    "arm64 qualified install planned fixture",
)
replace_once(
    qualifier,
    """    \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n    \"${package_dir}/tests/stata/test_rust_public_exact.do\" \\\n""",
    """    \"${package_dir}/tests/stata/test_rust_generic_jla.do\" \\\n    \"${package_dir}/tests/stata/test_rust_planned_v4.do\" \\\n    \"${package_dir}/tests/stata/test_rust_public_exact.do\" \\\n""",
    "x86 qualified install planned fixture",
)

replace_once(
    qualifier,
    """  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid\\n'\n  printf 'excluded_claims=public-release,production,Windows,Linux,native-Intel,automatic-Rust-routing,generic-auto-routing,CMG,batch-auto,stayers,probeorder,wallseconds,scale,human-license-provenance-review\\n'\n""",
    """  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid;planned-generic-jla-v4-v7-engine-generic-route-auto-independent-batches-wall-advisory-counter-v1\\n'\n  printf 'excluded_claims=public-release,production,Windows,Linux,native-Intel,public-automatic-Rust-routing,engine-auto,CMG,stayers,probeorder,scale,human-license-provenance-review\\n'\n""",
    "qualifier tested and excluded claims",
)

replace_once(
    qualifier,
    """  printf 'arm64_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n  printf 'arm64_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    """  printf 'arm64_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n  printf 'arm64_private_planned_v4=VARCOMP_KSS RUST PLANNED V4 PASS\\n'\n  printf 'arm64_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    "arm64 planned receipt marker",
)
replace_once(
    qualifier,
    """  printf 'arm64_universal_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n  printf 'arm64_universal_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    """  printf 'arm64_universal_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n  printf 'arm64_universal_private_planned_v4=VARCOMP_KSS RUST PLANNED V4 PASS\\n'\n  printf 'arm64_universal_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    "arm64 universal planned receipt marker",
)
replace_once(
    qualifier,
    """    printf 'x86_64_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n    printf 'x86_64_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    """    printf 'x86_64_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n    printf 'x86_64_private_planned_v4=VARCOMP_KSS RUST PLANNED V4 PASS\\n'\n    printf 'x86_64_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    "x86 planned receipt marker",
)
replace_once(
    qualifier,
    """    printf 'x86_64_universal_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n    printf 'x86_64_universal_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    """    printf 'x86_64_universal_private_generic=VARCOMP_KSS RUST GENERIC JLA PASS\\n'\n    printf 'x86_64_universal_private_planned_v4=VARCOMP_KSS RUST PLANNED V4 PASS\\n'\n    printf 'x86_64_universal_public_exact=VARCOMP_KSS RUST PUBLIC EXACT PASS\\n'\n""",
    "x86 universal planned receipt marker",
)

run_all = Path("varcomp_kss/tests/stata/run_all.do")
replace_once(
    run_all,
    """    do `\"`pkgroot'/tests/stata/test_rust_generic_jla.do\"' `\"`pkgroot'\"'\n    do `\"`pkgroot'/tests/stata/test_rust_public_exact.do\"' `\"`pkgroot'\"'\n""",
    """    do `\"`pkgroot'/tests/stata/test_rust_generic_jla.do\"' `\"`pkgroot'\"'\n    do `\"`pkgroot'/tests/stata/test_rust_planned_v4.do\"' `\"`pkgroot'\"'\n    do `\"`pkgroot'/tests/stata/test_rust_public_exact.do\"' `\"`pkgroot'\"'\n""",
    "quick/full planned V4 fixture",
)
