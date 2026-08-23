from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{path}: expected one anchor, found {count}: {old[:80]!r}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "varcomp_kss/tests/stata/test_rust_planned_compressed.do",
    "assert r(plan_applicability) == 3\n",
    "assert r(plan_applicability) == 2\n",
)

qualifier = "rust/stata_backend/qualify_macos.sh"

replace_once(
    qualifier,
    "source-local Rust developer routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned generic JLA V4/V7 with automatic route, independent batching, and wall advisory; support mask 38 plus request-capability receipts",
    "source-local Rust developer routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned compressed and generic JLA V4/V7 with frozen engine and route receipts, independent batching, and wall advisory; support mask 38 plus request-capability receipts",
)
replace_once(
    qualifier,
    "source-local Rust developer routes tested on macOS arm64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned generic JLA V4/V7 with automatic route, independent batching, and wall advisory; x86_64 runtime untested; support mask 38 plus request-capability receipts",
    "source-local Rust developer routes tested on macOS arm64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, and planned compressed and generic JLA V4/V7 with frozen engine and route receipts, independent batching, and wall advisory; x86_64 runtime untested; support mask 38 plus request-capability receipts",
)
replace_once(
    qualifier,
    '''  [[ "${available}" == *'tested on macOS arm64 and Rosetta x86_64'* ]] || \\
    fail "available receipt scope omitted Rosetta qualification"
''',
    '''  [[ "${available}" == *'tested on macOS arm64 and Rosetta x86_64'* ]] || \\
    fail "available receipt scope omitted Rosetta qualification"
  [[ "${available}" == *'planned compressed and generic JLA V4/V7'* ]] || \\
    fail "available receipt scope omitted compressed V4/V7 qualification"
''',
)
replace_once(
    qualifier,
    '''  "${package_dir}/tests/stata/test_rust_planned_v4.do"
  "${package_dir}/tests/stata/test_rust_public_exact.do"
''',
    '''  "${package_dir}/tests/stata/test_rust_planned_v4.do"
  "${package_dir}/tests/stata/test_rust_planned_compressed.do"
  "${package_dir}/tests/stata/test_rust_public_exact.do"
''',
)

# Thin and universal execution on native arm64.
replace_once(
    qualifier,
    '''run_stata_case arm64 private-planned-v4 \\
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
  'VARCOMP_KSS RUST PLANNED V4 PASS' "${test_package_dir}"
run_stata_case arm64 public-exact \\
''',
    '''run_stata_case arm64 private-planned-v4 \\
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
  'VARCOMP_KSS RUST PLANNED V4 PASS' "${test_package_dir}"
run_stata_case arm64 private-planned-compressed \\
  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\
  'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS' "${test_package_dir}"
run_stata_case arm64 public-exact \\
''',
)
replace_once(
    qualifier,
    '''run_stata_case arm64 universal-private-planned-v4 \\
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
  'VARCOMP_KSS RUST PLANNED V4 PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-exact \\
''',
    '''run_stata_case arm64 universal-private-planned-v4 \\
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
  'VARCOMP_KSS RUST PLANNED V4 PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-private-planned-compressed \\
  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\
  'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-exact \\
''',
)

# Thin and universal execution under Rosetta when available.
replace_once(
    qualifier,
    '''  run_stata_case x86_64 private-planned-v4 \\
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
    'VARCOMP_KSS RUST PLANNED V4 PASS' "${test_package_dir}"
  run_stata_case x86_64 public-exact \\
''',
    '''  run_stata_case x86_64 private-planned-v4 \\
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
    'VARCOMP_KSS RUST PLANNED V4 PASS' "${test_package_dir}"
  run_stata_case x86_64 private-planned-compressed \\
    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\
    'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS' "${test_package_dir}"
  run_stata_case x86_64 public-exact \\
''',
)
replace_once(
    qualifier,
    '''  run_stata_case x86_64 universal-private-planned-v4 \\
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
    'VARCOMP_KSS RUST PLANNED V4 PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-exact \\
''',
    '''  run_stata_case x86_64 universal-private-planned-v4 \\
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \\
    'VARCOMP_KSS RUST PLANNED V4 PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-private-planned-compressed \\
    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \\
    'VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-exact \\
''',
)

replace_once(
    qualifier,
    "  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid;planned-generic-jla-v4-v7-engine-generic-route-auto-independent-batches-wall-advisory-counter-v1\\n'\n",
    "  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid;planned-compressed-jla-v4-v7-engine-auto-to-compressed-route-diagonal-explicit-batches-counter-v1-fweights-stored-targetweights-matchid;planned-generic-jla-v4-v7-engine-generic-route-auto-independent-batches-wall-advisory-counter-v1\\n'\n",
)
replace_once(
    qualifier,
    "  printf 'excluded_claims=public-release,production,Windows,Linux,native-Intel,public-automatic-Rust-routing,engine-auto,CMG,stayers,probeorder,scale,human-license-provenance-review\\n'\n",
    "  printf 'excluded_claims=public-release,production,Windows,Linux,native-Intel,public-automatic-Rust-routing,public-compressed-engine-auto,algorithm-auto,CMG,stayers,probeorder,scale,human-license-provenance-review\\n'\n",
)

# Receipt PASS markers.
for prefix in (
    "arm64_private",
    "arm64_universal_private",
    "x86_64_private",
    "x86_64_universal_private",
):
    indent = "    " if prefix.startswith("x86_64") else "  "
    replace_once(
        qualifier,
        f"{indent}printf '{prefix}_planned_v4=VARCOMP_KSS RUST PLANNED V4 PASS\\n'\n",
        f"{indent}printf '{prefix}_planned_v4=VARCOMP_KSS RUST PLANNED V4 PASS\\n'\n"
        f"{indent}printf '{prefix}_planned_compressed=VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS\\n'\n",
    )

# Reproducible command descriptions. Also fill the pre-existing omission for
# the generic planned-V4 command beside the new compressed command.
command_insertions = (
    ("arm64_private", "arm64", "<temporary-thin-package>", "  "),
    ("arm64_universal_private", "arm64", "<temporary-universal-package>", "  "),
    ("x86_64_private", "x86_64", "<temporary-thin-package>", "    "),
    ("x86_64_universal_private", "x86_64", "<temporary-universal-package>", "    "),
)
for prefix, arch_name, package_name, indent in command_insertions:
    replace_once(
        qualifier,
        f"{indent}printf 'command.test_{prefix}_generic=arch -{arch_name} <stata-binary> -b do varcomp_kss/tests/stata/test_rust_generic_jla.do {package_name}\\n'\n",
        f"{indent}printf 'command.test_{prefix}_generic=arch -{arch_name} <stata-binary> -b do varcomp_kss/tests/stata/test_rust_generic_jla.do {package_name}\\n'\n"
        f"{indent}printf 'command.test_{prefix}_planned_v4=arch -{arch_name} <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_v4.do {package_name}\\n'\n"
        f"{indent}printf 'command.test_{prefix}_planned_compressed=arch -{arch_name} <stata-binary> -b do varcomp_kss/tests/stata/test_rust_planned_compressed.do {package_name}\\n'\n",
    )
