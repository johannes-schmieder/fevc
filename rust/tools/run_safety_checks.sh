#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: run_safety_checks.sh --receipt PATH\n' >&2
}

receipt=
while (( $# )); do
  case $1 in
    --receipt)
      (( $# >= 2 )) || { usage; exit 198; }
      receipt=$2
      shift 2
      ;;
    *)
      usage
      exit 198
      ;;
  esac
done
[[ -n "${receipt}" ]] || { usage; exit 198; }
[[ ! -e "${receipt}" ]] || {
  printf 'refusing to overwrite safety receipt: %s\n' "${receipt}" >&2
  exit 198
}
[[ -d $(dirname -- "${receipt}") ]] || {
  printf 'safety receipt parent does not exist: %s\n' "$(dirname -- "${receipt}")" >&2
  exit 198
}

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "${script_dir}/../.." && pwd -P)
manifest=${repo_root}/rust/Cargo.toml
plugin_manifest=${repo_root}/rust/crates/vckss-plugin/Cargo.toml
shim_root=${repo_root}/rust/stata_backend

[[ $(git -C "${repo_root}" symbolic-ref --short HEAD) == main ]] || {
  printf 'safety qualification requires main\n' >&2
  exit 198
}
[[ -z $(git -C "${repo_root}" status --porcelain --untracked-files=all) ]] || {
  printf 'safety qualification requires a clean committed checkout\n' >&2
  exit 198
}
source_commit=$(git -C "${repo_root}" rev-parse HEAD)
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]]

miri_toolchain=${VCKSS_MIRI_TOOLCHAIN:-nightly-2026-08-23}
miri_cargo=$(rustup which --toolchain "${miri_toolchain}" cargo)
miri_rustc=$(rustup which --toolchain "${miri_toolchain}" rustc)
miri_bin=$(dirname -- "${miri_cargo}")
[[ -x "${miri_bin}/cargo-miri" ]] || {
  printf 'Miri is not installed for %s; run rustup component add --toolchain %s miri\n' \
    "${miri_toolchain}" "${miri_toolchain}" >&2
  exit 198
}

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/vckss-safety.XXXXXX")
receipt_temporary=
cleanup() {
  if [[ -n "${receipt_temporary}" && -f "${receipt_temporary}" ]]; then
    rm -f -- "${receipt_temporary}" || true
  fi
  if [[ -d "${temporary_root}" ]]; then
    chmod -R u+w "${temporary_root}" 2>/dev/null || true
    rm -rf -- "${temporary_root}" || true
  fi
}
trap cleanup EXIT

export CARGO_TARGET_DIR=${temporary_root}/miri-target
export MIRIFLAGS=${MIRIFLAGS:--Zmiri-strict-provenance}

miri_cargo() {
  env \
    PATH="${miri_bin}:/usr/bin:/bin" \
    RUSTUP_TOOLCHAIN="${miri_toolchain}" \
    RUSTC_WRAPPER= \
    RUSTC_WORKSPACE_WRAPPER= \
    "${miri_cargo}" "$@"
}

miri_cargo miri test --manifest-path "${manifest}" \
  -p vckss-plugin --lib
miri_library_status=PASS

miri_cargo miri test --manifest-path "${manifest}" \
  -p vckss-plugin --test context_registry
miri_registry_status=PASS

miri_engine_tests=(
  headers_and_every_output_capacity_fail_before_full_value_access_or_write
  v2_memory_admission_fails_before_column_descriptor_access
  invalid_and_unknown_callback_contracts_fail_closed
  prepare_user_breaks_leave_zero_generation_and_empty_registry
  interrupted_stayer_copy_leaves_generation_releasable
)
for test_name in "${miri_engine_tests[@]}"; do
  miri_cargo miri test --manifest-path "${manifest}" \
    -p vckss-plugin --test engine_ffi "${test_name}" -- --exact
done
miri_engine_status=PASS

host_os=$(uname -s)
case "${host_os}" in
  Darwin)
    shim_system=APPLEMAC
    dead_code_linker_flag=-Wl,-dead_strip
    ;;
  Linux)
    shim_system=OPUNIX
    dead_code_linker_flag=-Wl,--gc-sections
    ;;
  *)
    printf 'C-shim sanitizer qualification is unsupported on %s\n' "${host_os}" >&2
    exit 198
    ;;
esac
sanitizer_cc=${CC:-clang}
command -v "${sanitizer_cc}" >/dev/null
sanitizer_flags=(
  -std=c11 -O1 -g -Wall -Wextra -Werror -fno-omit-frame-pointer
  -fsanitize=address,undefined
  -D"SYSTEM=${shim_system}"
  -I "${shim_root}/stata-spi"
  -I "${shim_root}/cshim"
  -I "${shim_root}/include"
)
sanitizer_environment=(
  ASAN_OPTIONS=abort_on_error=1:detect_leaks=1:halt_on_error=1
  UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1
)

"${sanitizer_cc}" "${sanitizer_flags[@]}" \
  "${shim_root}/tests/cshim_interrupt_test.c" \
  -o "${temporary_root}/cshim-interrupt"
env "${sanitizer_environment[@]}" "${temporary_root}/cshim-interrupt"
cshim_interrupt_sanitizer_status=PASS

"${sanitizer_cc}" "${sanitizer_flags[@]}" -ffunction-sections \
  "${shim_root}/tests/cshim_error_transport_test.c" \
  "${dead_code_linker_flag}" -o "${temporary_root}/cshim-error-transport"
env "${sanitizer_environment[@]}" "${temporary_root}/cshim-error-transport"
cshim_error_sanitizer_status=PASS

"${sanitizer_cc}" -std=c11 -Wall -Wextra -Werror \
  -I "${shim_root}/include" \
  -c "${shim_root}/tests/abi_header_compat_test.c" \
  -o "${temporary_root}/abi-header-compat.o"
abi_header_status=PASS

receipt_temporary=$(mktemp "${receipt}.tmp.XXXXXX")
started_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)
{
  printf 'receipt_schema=VCKSS-SAFETY-QUALIFICATION-V1\n'
  printf 'status=PASS\n'
  printf 'scope=bounded unsafe Rust FFI and registry Miri; C-shim ASan/UBSan\n'
  printf 'source_commit=%s\n' "${source_commit}"
  printf 'branch=main\n'
  printf 'worktree=clean\n'
  printf 'host_os=%s\n' "${host_os}"
  printf 'host_arch=%s\n' "$(uname -m)"
  printf 'miri_toolchain=%s\n' "${miri_toolchain}"
  printf 'miri_rustc=%s\n' "$("${miri_rustc}" --version)"
  printf 'miri_flags=%s\n' "${MIRIFLAGS}"
  printf 'miri_library=%s\n' "${miri_library_status}"
  printf 'miri_context_registry=%s\n' "${miri_registry_status}"
  printf 'miri_engine_ffi=%s\n' "${miri_engine_status}"
  printf 'miri_engine_cases=%s\n' "$(IFS=,; printf '%s' "${miri_engine_tests[*]}")"
  printf 'miri_numerical_scope=excluded; native Rust and Stata bitwise/numerical gates remain authoritative\n'
  printf 'miri_exact_stayer_solve=excluded; dense spectral solve is impractically slow under interpretation; bounded stayer interruption/release is included and native exact-stayer lifecycle remains mandatory\n'
  printf 'sanitizer_compiler=%s\n' "$("${sanitizer_cc}" --version | sed -n '1p')"
  printf 'cshim_interrupt_asan_ubsan=%s\n' "${cshim_interrupt_sanitizer_status}"
  printf 'cshim_error_transport_asan_ubsan=%s\n' "${cshim_error_sanitizer_status}"
  printf 'abi_header_compatibility=%s\n' "${abi_header_status}"
  printf 'raw_logs=not retained; command output is ephemeral and temporary binaries are deleted\n'
  printf 'excluded_claims=fuzzing,dependency-advisories,license-review,SBOM,public-release,Windows\n'
  printf 'completed_utc=%s\n' "${started_utc}"
} > "${receipt_temporary}"
mv "${receipt_temporary}" "${receipt}"
receipt_temporary=
printf 'VCKSS SAFETY QUALIFICATION PASS receipt=%s source_commit=%s\n' \
  "${receipt}" "${source_commit}"
