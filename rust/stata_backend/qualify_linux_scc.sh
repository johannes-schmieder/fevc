#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  cat <<'EOF'
Usage: rust/stata_backend/qualify_linux_scc.sh \
  --receipt PATH --source-commit SHA --bundle-sha256 SHA \
  [--stata PATH] [--artifacts-dir DIRECTORY]

Build and test one source-bound Linux x86-64 candidate. The caller must first
verify the immutable source bundle and run this script inside a scheduled SCC
compute job. PATH must not already exist. Raw Stata logs are temporary; the
optional artifacts directory receives sanitized logs and the tested binary.
EOF
}

fail() {
  printf 'SCC Linux Rust plugin qualification failed: %s\n' "$*" >&2
  exit 1
}

receipt_path=
source_commit=
bundle_sha256=
stata_binary=${VCKSS_STATA_BIN:-stata-mp}
artifacts_dir=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --receipt) receipt_path=${2:-}; shift 2 ;;
    --source-commit) source_commit=${2:-}; shift 2 ;;
    --bundle-sha256) bundle_sha256=${2:-}; shift 2 ;;
    --stata) stata_binary=${2:-}; shift 2 ;;
    --artifacts-dir) artifacts_dir=${2:-}; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; fail "unknown argument: $1" ;;
  esac
done

[[ -n "${receipt_path}" ]] || fail "--receipt is required"
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]] || \
  fail "--source-commit must be a full lowercase Git object ID"
[[ "${bundle_sha256}" =~ ^[0-9a-f]{64}$ ]] || \
  fail "--bundle-sha256 must be a lowercase SHA-256 digest"
[[ $(uname -s) == Linux ]] || fail "this qualifier runs only on Linux"
[[ $(uname -m) == x86_64 ]] || fail "native x86-64 Linux is required"

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/../.." && pwd)
package_dir=${repo_root}/vckss
manifest_path=${script_dir}/Cargo.toml
[[ -f "${repo_root}/SOURCE_COMMIT.txt" ]] || \
  fail "immutable bundle lacks SOURCE_COMMIT.txt"
[[ $(tr -d '[:space:]' < "${repo_root}/SOURCE_COMMIT.txt") == \
  "${source_commit}" ]] || fail "source commit binding mismatch"
[[ -f "${repo_root}/SOURCE_FILES.sha256" ]] || \
  fail "immutable bundle lacks SOURCE_FILES.sha256"

receipt_parent=$(dirname -- "${receipt_path}")
[[ -d "${receipt_parent}" ]] || fail "receipt parent does not exist"
receipt_parent=$(CDPATH= cd -- "${receipt_parent}" && pwd)
receipt_path=${receipt_parent}/$(basename -- "${receipt_path}")
[[ ! -e "${receipt_path}" ]] || fail "receipt already exists: ${receipt_path}"
if [[ -n "${artifacts_dir}" ]]; then
  [[ -d "${artifacts_dir}" ]] || fail "artifacts directory does not exist"
  artifacts_dir=$(CDPATH= cd -- "${artifacts_dir}" && pwd)
  [[ -z $(find "${artifacts_dir}" -mindepth 1 -maxdepth 1 -print -quit) ]] || \
    fail "artifacts directory is not empty"
fi

for command_name in awk cc cargo file find grep install ldd nm readelf \
  rustc rustfmt sed sha256sum sort stat; do
  command -v "${command_name}" >/dev/null 2>&1 || \
    fail "required command unavailable: ${command_name}"
done
stata_binary=$(command -v "${stata_binary}" 2>/dev/null || true)
[[ -n "${stata_binary}" && -x "${stata_binary}" ]] || \
  fail "Stata executable is unavailable"

rustc_version=$(rustc --version)
cargo_version=$(cargo --version)
[[ "${rustc_version}" == 'rustc 1.85.1 '* ]] || \
  fail "SCC qualification requires the pinned VCkss Rust 1.85.1 toolchain"
[[ "${cargo_version}" == 'cargo 1.85.1 '* ]] || \
  fail "SCC qualification requires the pinned VCkss Cargo 1.85.1 toolchain"

temporary_root=
receipt_temporary=
candidate=
source_manifest=

sanitize_stata_log() {
  local source=$1 destination=$2
  awk '
    BEGIN { started = 0 }
    /^\. / { started = 1 }
    started {
      if ($0 ~ /Licensed to:/ || $0 ~ /Serial number:/) next
      print
    }
  ' "${source}" > "${destination}"
}

export_sanitized_evidence() {
  local source relative destination
  [[ -n "${artifacts_dir}" ]] || return 0
  [[ -n "${temporary_root}" && -d "${temporary_root}" ]] || return 0
  mkdir -p "${artifacts_dir}/stata-logs" "${artifacts_dir}/candidate"
  if [[ -n "${source_manifest}" && -f "${source_manifest}" ]]; then
    cp "${source_manifest}" "${artifacts_dir}/qualification-source.sha256"
  fi
  while IFS= read -r -d '' source; do
    relative=${source#"${temporary_root}/"}
    destination=${artifacts_dir}/stata-logs/${relative//\//__}.sanitized.log
    sanitize_stata_log "${source}" "${destination}"
  done < <(find "${temporary_root}" -maxdepth 4 -type f \
    \( -name '*.log' -o -name 'console.txt' \) -print0)
  if [[ -n "${candidate}" && -f "${candidate}" ]]; then
    install -m 0755 "${candidate}" \
      "${artifacts_dir}/candidate/vckss_rust_linux_x64.plugin"
  fi
  printf '%s\n' \
    'Sanitized Stata logs begin at the first batch prompt.' \
    'Startup banners, license identifiers, raw logs, and build caches were deleted.' \
    > "${artifacts_dir}/SANITIZED_EVIDENCE.txt"
}

cleanup() {
  export_sanitized_evidence || true
  if [[ -n "${receipt_temporary}" && -f "${receipt_temporary}" ]]; then
    rm -f -- "${receipt_temporary}" || true
  fi
  if [[ -n "${temporary_root}" && -d "${temporary_root}" ]]; then
    chmod -R u+w "${temporary_root}" 2>/dev/null || true
    rm -rf -- "${temporary_root}" || true
  fi
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/vckss-linux-qualify.XXXXXX")
spi_dir=${temporary_root}/stata-spi
cargo_target_dir=${temporary_root}/cargo-target
candidate_dir=${temporary_root}/candidate
test_root=${temporary_root}/test-root
test_package_dir=${test_root}/vckss
install_root=${temporary_root}/install
mkdir -p "${candidate_dir}" "${test_root}" "${install_root}"

hash_file() {
  sha256sum "$1" | awk '{print $1}'
}

write_source_manifest() {
  local output=$1 source_file relative
  : > "${output}"
  while IFS= read -r -d '' source_file; do
    relative=${source_file#"${repo_root}/"}
    case "${relative}" in
      SOURCE_FILES.sha256) continue ;;
    esac
    printf '%s  %s\n' "$(hash_file "${source_file}")" "${relative}" \
      >> "${output}"
  done < <(find "${repo_root}" -type f -print0 | LC_ALL=C sort -z)
}

# The deployer binds every tracked archive member. Verify that contract again
# on the compute node before compiling and retain a narrower qualifier manifest
# to prove the build/test process did not mutate the extracted source.
(
  cd "${repo_root}"
  sha256sum -c SOURCE_FILES.sha256
)
source_manifest=${temporary_root}/qualification-source-before.sha256
write_source_manifest "${source_manifest}"
source_hash_before=$(hash_file "${source_manifest}")
source_file_count=$(wc -l < "${source_manifest}" | tr -d ' ')

VCKSS_STATA_SPI_DIR="${spi_dir}" "${script_dir}/fetch_stata_spi.sh"
spi_source_hash=$(hash_file "${spi_dir}/stplugin.c")
spi_header_hash=$(hash_file "${spi_dir}/stplugin.h")

cshim_interrupt=${temporary_root}/vckss-cshim-interrupt-test
cc -std=c11 -Wall -Wextra -Werror -DSYSTEM=OPUNIX \
  -I "${spi_dir}" -I "${script_dir}/cshim" -I "${script_dir}/include" \
  "${script_dir}/tests/cshim_interrupt_test.c" -o "${cshim_interrupt}"
"${cshim_interrupt}"
cshim_interrupt_status=PASS

cshim_error=${temporary_root}/vckss-cshim-error-transport-test
cc -std=c11 -Wall -Wextra -Werror -ffunction-sections -DSYSTEM=OPUNIX \
  -I "${spi_dir}" -I "${script_dir}/cshim" -I "${script_dir}/include" \
  "${script_dir}/tests/cshim_error_transport_test.c" \
  -Wl,--gc-sections -o "${cshim_error}"
"${cshim_error}"
cshim_error_status=PASS

abi_object=${temporary_root}/vckss-abi-header-compat.o
cc -std=c11 -Wall -Wextra -Werror -I "${script_dir}/include" \
  -c "${script_dir}/tests/abi_header_compat_test.c" -o "${abi_object}"
abi_status=PASS

plugin_cargo() {
  env VCKSS_STATA_SPI_DIR="${spi_dir}" \
    CARGO_TARGET_DIR="${cargo_target_dir}" \
    RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= cargo "$@"
}
plugin_cargo fmt --manifest-path "${manifest_path}" --all -- --check
cargo_fmt_status=PASS
plugin_cargo clippy --manifest-path "${manifest_path}" \
  --locked --all-targets -- -D warnings
cargo_clippy_status=PASS
plugin_cargo test --manifest-path "${manifest_path}" --locked --all-targets
cargo_test_status=PASS
plugin_cargo build --manifest-path "${manifest_path}" --locked --release

candidate=${candidate_dir}/vckss_rust_linux_x64.plugin
install -m 0755 "${cargo_target_dir}/release/libvckss_stata.so" "${candidate}"
candidate_sha256=$(hash_file "${candidate}")
candidate_file=$(file -b "${candidate}")
[[ "${candidate_file}" == *'ELF 64-bit LSB shared object, x86-64'* ]] || \
  fail "candidate is not an x86-64 ELF shared object: ${candidate_file}"
readelf -h "${candidate}" | grep -F -q 'Class:                             ELF64'
readelf -h "${candidate}" | grep -F -q 'Machine:                           Advanced Micro Devices X86-64'
readelf -h "${candidate}" | grep -F -q 'Type:                              DYN (Shared object file)'

dependencies=$(ldd "${candidate}" | awk '
  $1 ~ /^linux-vdso/ {print $1}
  $2 == "=>" {print $1}
  $1 ~ /^\// {n=split($1,p,"/"); print p[n]}
' | LC_ALL=C sort -u)
[[ -n "${dependencies}" ]] || fail "no Linux dependencies were parsed"
while IFS= read -r dependency; do
  case "${dependency}" in
    linux-vdso.so.*|libgcc_s.so.*|libc.so.*|libm.so.*|libpthread.so.*|\
      libdl.so.*|ld-linux-*.so.*) ;;
    *) fail "unexpected Linux dependency: ${dependency}" ;;
  esac
done <<< "${dependencies}"
dependency_text=$(paste -sd, - <<< "${dependencies}")

exports=$(nm -D --defined-only "${candidate}" | awk '{print $3}')
required_exports=(pginit stata_call)
while IFS= read -r symbol; do
  required_exports[${#required_exports[@]}]=${symbol}
done < <(sed -nE \
  's/^[^()]*(vckss_rust_[[:alnum:]_]+)\(.*/\1/p' \
  "${script_dir}/include/vckss_rust.h")
[[ ${#required_exports[@]} -gt 2 ]] || fail "no Rust ABI exports parsed"
for symbol in "${required_exports[@]}"; do
  grep -F -x -q "${symbol}" <<< "${exports}" || \
    fail "candidate lacks required export: ${symbol}"
done
required_export_text=$(printf '%s,' "${required_exports[@]}")
required_export_text=${required_export_text%,}

if [[ -n $(find "${package_dir}" -type f -name '*.plugin' -print -quit) ]]; then
  fail "immutable source bundle unexpectedly contains plugin binaries"
fi
cp -a "${package_dir}" "${test_package_dir}"
chmod -R u+w "${test_package_dir}"
cp "${package_dir}/vckss.pkg" "${test_package_dir}/vckss.pkg"
printf 'f vckss_rust_linux_x64.plugin\n' >> "${test_package_dir}/vckss.pkg"
install -m 0755 "${candidate}" \
  "${test_package_dir}/vckss_rust_linux_x64.plugin"
[[ $(hash_file "${test_package_dir}/vckss_rust_linux_x64.plugin") == \
  "${candidate_sha256}" ]] || fail "staged test artifact hash mismatch"

last_run_directory=
run_stata_case() {
  local label=$1 do_file=$2 marker=$3 return_code run_directory
  local working_directory marker_file
  shift 3
  run_directory=${temporary_root}/stata-${label}
  mkdir -p "${run_directory}"
  working_directory=${VCKSS_STATA_CASE_CWD:-${run_directory}}
  marker_file=${VCKSS_STATA_MARKER_FILE:-}
  [[ -d "${working_directory}" ]] || \
    fail "Stata ${label} working directory does not exist"
  if [[ -n "${marker_file}" ]]; then
    [[ "${marker_file}" == "${working_directory}/"* ]] || \
      fail "Stata ${label} marker file is outside its isolated working directory"
    [[ ! -e "${marker_file}" ]] || \
      fail "Stata ${label} marker file existed before the fresh process"
  fi
  set +e
  (
    cd "${working_directory}"
    "${stata_binary}" -q -b do "${do_file}" "$@"
  ) > "${run_directory}/console.txt" 2>&1
  return_code=$?
  set -e
  [[ ${return_code} -eq 0 ]] || \
    fail "Stata ${label} returned ${return_code}"
  if [[ -n "${marker_file}" ]]; then
    [[ -f "${marker_file}" ]] || \
      fail "Stata ${label} did not create its expected batch log"
    grep -F -q -- "${marker}" "${marker_file}" || \
      fail "Stata ${label} omitted PASS marker from its fresh batch log"
    cp "${marker_file}" "${run_directory}/$(basename -- "${marker_file}")"
  else
    grep -R -F -q -- "${marker}" "${run_directory}" || \
      fail "Stata ${label} omitted PASS marker"
  fi
  last_run_directory=${run_directory}
}

environment_do=${temporary_root}/environment.do
printf '%s\n' \
  'version 18.0' \
  'display as result "VCKSS_STATA_ENV version=`c(stata_version)'\'' edition=`c(edition_real)'\'' os=`c(os)'\'' machine=`c(machine_type)'\''"' \
  'exit 0' > "${environment_do}"
run_stata_case environment "${environment_do}" VCKSS_STATA_ENV
stata_environment=$(grep -R -F -h 'VCKSS_STATA_ENV ' \
  "${last_run_directory}" | tr -d '\r' | tail -n 1)
[[ "${stata_environment}" == *'version=19'*'os=Unix'* ]] || \
  fail "unexpected Stata environment: ${stata_environment}"

run_stata_case lifecycle \
  "${test_package_dir}/tests/stata/test_rust_plugin.do" \
  'VCKSS RUST PLUGIN PASS' "${test_package_dir}"
run_stata_case diagnostic \
  "${test_package_dir}/tests/stata/test_rust_mata_diagnostic.do" \
  'VCKSS RUST MATA DIAGNOSTIC PASS' "${test_package_dir}"
run_stata_case shared-atoms \
  "${test_package_dir}/tests/stata/test_rust_mata_shared_atoms.do" \
  'PASS test_rust_mata_shared_atoms.do' "${test_package_dir}"
run_stata_case public-route \
  "${test_package_dir}/tests/stata/test_rust_public.do" \
  'PASS test_rust_public.do' "${test_package_dir}"
VCKSS_STATA_CASE_CWD=${test_root} \
VCKSS_STATA_MARKER_FILE=${test_root}/run_all.log \
run_stata_case full-suite \
  "${test_package_dir}/tests/stata/run_all.do" \
  'VCKSS TEST SUITE PASS: full' full
run_stata_case clean-install \
  "${test_package_dir}/tests/stata/test_rust_public_install.do" \
  'PASS test_rust_public_install.do' "${test_package_dir}" \
  "${install_root}" qualified "${test_package_dir}/tests/stata"

source_after=${temporary_root}/qualification-source-after.sha256
write_source_manifest "${source_after}"
[[ "${source_hash_before}" == "$(hash_file "${source_after}")" ]] || \
  fail "qualification source files changed during build or testing"

host_kernel=$(uname -sr)
host_glibc=$(ldd --version | sed -n '1p')
host_cpu=$(uname -m)
source_manifest_binding=$(hash_file "${repo_root}/SOURCE_FILES.sha256")
receipt_temporary=$(mktemp "${receipt_parent}/.$(basename -- "${receipt_path}").tmp.XXXXXX")
{
  printf 'VCKSS_SCC_LINUX_CANDIDATE_RECEIPT_V1\n'
  printf 'classification=CLEAN_SCC_LINUX_X86_64_CANDIDATE_QUALIFICATION\n'
  printf 'scope=scheduled BU SCC Linux x86-64 Rust alpha candidate; full public exact, exact stayers(both), compressed JLA, generic JLA, qualified CMG_FULL_V2 explicit and automatic routing, Counter-V1, lifecycle, caller-state, typed-failure, and clean-install coverage under Stata MP 19\n'
  printf 'excluded_claims=public-release,Windows,macOS,native-Intel,representative-scale,human-license-provenance-review\n'
  printf 'source_commit=%s\n' "${source_commit}"
  printf 'source_bundle_sha256=%s\n' "${bundle_sha256}"
  printf 'source_files_manifest_sha256=%s\n' "${source_manifest_binding}"
  printf 'qualification_source_manifest_sha256=%s\n' "${source_hash_before}"
  printf 'qualification_source_file_count=%s\n' "${source_file_count}"
  printf 'candidate_sha256=%s\n' "${candidate_sha256}"
  printf 'candidate_file=%s\n' "${candidate_file}"
  printf 'candidate_dependencies=%s\n' "${dependency_text}"
  printf 'required_exports=%s\n' "${required_export_text}"
  printf 'stata_spi_stplugin_c_sha256=%s\n' "${spi_source_hash}"
  printf 'stata_spi_stplugin_h_sha256=%s\n' "${spi_header_hash}"
  printf 'cshim_interrupt_test=%s\n' "${cshim_interrupt_status}"
  printf 'cshim_error_transport_test=%s\n' "${cshim_error_status}"
  printf 'abi_header_compat_test=%s\n' "${abi_status}"
  printf 'cargo_fmt=%s\n' "${cargo_fmt_status}"
  printf 'cargo_clippy=%s\n' "${cargo_clippy_status}"
  printf 'cargo_test=%s\n' "${cargo_test_status}"
  printf 'rustc=%s\n' "${rustc_version}"
  printf 'cargo=%s\n' "${cargo_version}"
  printf 'host_kernel=%s\n' "${host_kernel}"
  printf 'host_glibc=%s\n' "${host_glibc}"
  printf 'host_cpu=%s\n' "${host_cpu}"
  printf 'stata_binary_sha256=%s\n' "$(hash_file "${stata_binary}")"
  printf 'stata_environment=%s\n' "${stata_environment}"
  printf 'linux_lifecycle=VCKSS RUST PLUGIN PASS\n'
  printf 'linux_diagnostic=VCKSS RUST MATA DIAGNOSTIC PASS\n'
  printf 'linux_shared_atoms=PASS test_rust_mata_shared_atoms.do\n'
  printf 'linux_public_route=PASS test_rust_public.do\n'
  printf 'linux_full_suite=VCKSS TEST SUITE PASS: full\n'
  printf 'linux_clean_install=PASS test_rust_public_install.do\n'
  printf 'command.module=module purge; PATH=<vckss-rust-1.85.1>/bin:$PATH; module load stata-mp/19\n'
  printf 'command.qualifier=rust/stata_backend/qualify_linux_scc.sh --receipt <run>/receipts/linux-qualification.txt --source-commit %s --bundle-sha256 %s --artifacts-dir <run>/artifacts\n' \
    "${source_commit}" "${bundle_sha256}"
  printf 'command.build=VCKSS_STATA_SPI_DIR=<temporary>/stata-spi CARGO_TARGET_DIR=<temporary>/cargo-target cargo build --manifest-path rust/stata_backend/Cargo.toml --locked --release\n'
  printf 'command.full_suite=stata-mp -q -b do vckss/tests/stata/run_all.do full\n'
  printf 'command.clean_install=stata-mp -q -b do vckss/tests/stata/test_rust_public_install.do <temporary-package> <isolated-plus> qualified <test-root>\n'
  printf 'raw_logs=temporary-only; sanitized logs exported and raw temporary evidence deleted on exit\n'
} > "${receipt_temporary}"
mv "${receipt_temporary}" "${receipt_path}"
receipt_temporary=

printf 'VCKSS_SCC_LINUX_QUALIFICATION_PASS source_commit=%s candidate_sha256=%s\n' \
  "${source_commit}" "${candidate_sha256}"
