#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  cat <<'EOF'
Usage: rust/stata_backend/qualify_macos.sh --receipt PATH [--stata PATH]
       [--artifacts-dir DIRECTORY]
       rust/stata_backend/qualify_macos.sh --selftest

Build and test local macOS arm64, x86_64, and universal developer candidates.
PATH is mandatory, must not already exist, and receives a sanitized receipt.
When DIRECTORY is supplied, sanitized Stata logs, source hashes, and exact
candidate binaries are copied there before raw temporary evidence is deleted.
EOF
}

fail() {
  printf 'macOS Rust plugin qualification failed: %s\n' "$*" >&2
  exit 1
}

qualification_scope() {
  case "$1" in
    AVAILABLE)
      printf '%s\n' \
        'source-local Rust alpha routes tested on macOS arm64 and Rosetta x86_64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, planned compressed and generic JLA V4/V7, public exact and generic-JLA mixed-deletion stayers(both) with a combined headline, explicit structured-common and leverage-only q=0/q=1 component inference, public backend(rust) engine(auto) compressed no-control match, and the qualified no-control match-JLA CMG_FULL_V2 cell through explicit Rust and automatic backend/RNG routing, plus automatic exact, automatic diagonal, forced CMG, independent or numeric batching, and wall advisory; support mask 38 plus request-capability receipts'
      ;;
    UNAVAILABLE)
      printf '%s\n' \
        'source-local Rust alpha routes tested on macOS arm64; exact, frozen compressed JLA, explicit generic diagonal numeric-batch Counter-V1, planned compressed and generic JLA V4/V7, public exact and generic-JLA mixed-deletion stayers(both) with a combined headline, explicit structured-common and leverage-only q=0/q=1 component inference, public backend(rust) engine(auto) compressed no-control match, and the qualified no-control match-JLA CMG_FULL_V2 cell through explicit Rust and automatic backend/RNG routing, plus automatic exact, automatic diagonal, forced CMG, independent or numeric batching, and wall advisory; x86_64 runtime untested; support mask 38 plus request-capability receipts'
      ;;
    *)
      fail "invalid Rosetta status for receipt scope: $1"
      ;;
  esac
}

tested_artifact_scope() {
  case "$1" in
    AVAILABLE)
      printf '%s\n' \
        'ad-hoc-signed thin arm64 and x86_64 candidates tested under architecture-specific names; byte-identical universal candidate tested separately under both aliases'
      ;;
    UNAVAILABLE)
      printf '%s\n' \
        'ad-hoc-signed thin arm64 candidate tested under its architecture-specific name; byte-identical universal candidate tested on arm64; x86_64 thin/universal slices built and inspected but runtime untested'
      ;;
    *)
      fail "invalid Rosetta status for tested-artifact scope: $1"
      ;;
  esac
}

qualifier_selftest() {
  local available unavailable artifacts_available artifacts_unavailable
  available=$(qualification_scope AVAILABLE)
  unavailable=$(qualification_scope UNAVAILABLE)
  artifacts_available=$(tested_artifact_scope AVAILABLE)
  artifacts_unavailable=$(tested_artifact_scope UNAVAILABLE)
  [[ "${available}" == *'tested on macOS arm64 and Rosetta x86_64'* ]] || \
    fail "available receipt scope omitted Rosetta qualification"
  [[ "${available}" == *'planned compressed and generic JLA V4/V7'* ]] || \
    fail "available receipt scope omitted compressed V4/V7 qualification"
  [[ "${available}" == *'public backend(rust) engine(auto) compressed'* ]] || \
    fail "available receipt scope omitted public compressed qualification"
  [[ "${available}" == *'public exact and generic-JLA mixed-deletion stayers(both) with a combined headline'* ]] || \
    fail "available receipt scope omitted public combined stayer qualification"
  [[ "${available}" == *'structured-common and leverage-only q=0/q=1 component inference'* ]] || \
    fail "available receipt scope omitted public component-inference qualification"
  [[ "${unavailable}" == \
    *'tested on macOS arm64;'*'x86_64 runtime untested'* ]] || \
    fail "unavailable receipt scope did not withhold x86_64 runtime qualification"
  [[ "${unavailable}" != *'tested on macOS arm64 and Rosetta x86_64'* ]] || \
    fail "unavailable receipt scope falsely qualified Rosetta"
  [[ "${artifacts_available}" == *'thin arm64 and x86_64 candidates tested'* ]] || \
    fail "available artifact scope omitted x86_64 runtime evidence"
  [[ "${artifacts_unavailable}" == *'x86_64 thin/universal slices built and inspected but runtime untested'* ]] || \
    fail "unavailable artifact scope did not withhold x86_64 runtime evidence"
  printf 'VCKSS_MACOS_QUALIFIER_SELFTEST_PASS\n'
}

receipt_path=
stata_binary=/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp
artifacts_dir=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --receipt)
      [[ $# -ge 2 ]] || fail "--receipt requires a path"
      receipt_path=$2
      shift 2
      ;;
    --stata)
      [[ $# -ge 2 ]] || fail "--stata requires a path"
      stata_binary=$2
      shift 2
      ;;
    --artifacts-dir)
      [[ $# -ge 2 ]] || fail "--artifacts-dir requires a path"
      artifacts_dir=$2
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --selftest)
      [[ $# -eq 1 ]] || fail "--selftest does not accept other arguments"
      qualifier_selftest
      exit 0
      ;;
    *)
      usage >&2
      fail "unknown argument: $1"
      ;;
  esac
done

[[ -n "${receipt_path}" ]] || {
  usage >&2
  fail "--receipt is required"
}
[[ $(uname -s) == Darwin ]] || fail "this qualifier runs only on macOS"
[[ $(uname -m) == arm64 ]] || fail "native arm64 macOS is required"
[[ -x "${stata_binary}" ]] || fail "Stata executable not found: ${stata_binary}"

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/../.." && pwd)
package_dir=${repo_root}/fevc
manifest_path=${script_dir}/Cargo.toml

receipt_parent=$(dirname -- "${receipt_path}")
[[ -d "${receipt_parent}" ]] || fail "receipt parent directory does not exist"
receipt_parent=$(CDPATH= cd -- "${receipt_parent}" && pwd)
receipt_path=${receipt_parent}/$(basename -- "${receipt_path}")
[[ ! -e "${receipt_path}" ]] || fail "receipt path already exists: ${receipt_path}"

if [[ -n "${artifacts_dir}" ]]; then
  [[ -d "${artifacts_dir}" ]] || \
    fail "artifacts directory does not exist: ${artifacts_dir}"
  artifacts_dir=$(CDPATH= cd -- "${artifacts_dir}" && pwd)
  [[ -z $(find "${artifacts_dir}" -mindepth 1 -maxdepth 1 -print -quit) ]] || \
    fail "artifacts directory is not empty: ${artifacts_dir}"
fi

for required_command in arch awk clang codesign curl file git grep install \
  lipo nm otool paste rustup sed shasum sort sw_vers xcodebuild; do
  command -v "${required_command}" >/dev/null 2>&1 || \
    fail "required command is unavailable: ${required_command}"
done

rust_toolchain=1.85.1
rust_cargo=$(rustup which --toolchain "${rust_toolchain}" cargo) || \
  fail "could not resolve cargo for Rust ${rust_toolchain}"
rust_rustc=$(rustup which --toolchain "${rust_toolchain}" rustc) || \
  fail "could not resolve rustc for Rust ${rust_toolchain}"
[[ -x "${rust_cargo}" ]] || \
  fail "resolved Rust ${rust_toolchain} cargo is not executable: ${rust_cargo}"
[[ -x "${rust_rustc}" ]] || \
  fail "resolved Rust ${rust_toolchain} rustc is not executable: ${rust_rustc}"
rust_toolchain_bin=$(dirname -- "${rust_cargo}")
[[ "${rust_toolchain_bin}" == "$(dirname -- "${rust_rustc}")" ]] || \
  fail "Rust ${rust_toolchain} cargo and rustc are from different directories"
host_path=${PATH:-/usr/bin:/bin}
rustc_verbose=$("${rust_rustc}" -vV)
rust_host_target=$(awk \
  '$1 == "host:" {host = $2} END {if (host != "") print host}' \
  <<<"${rustc_verbose}")
[[ -n "${rust_host_target}" ]] || \
  fail "could not determine the Rust ${rust_toolchain} host target"

temporary_root=
receipt_temporary=
candidate_dir=
source_manifest=

sanitize_stata_log() {
  local source=$1
  local destination=$2
  # Stata's startup banner can contain license-holder information.  Retain
  # only the command transcript beginning at the first batch prompt.
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

  mkdir -p "${artifacts_dir}/stata-logs" "${artifacts_dir}/candidates"
  if [[ -n "${source_manifest}" && -f "${source_manifest}" ]]; then
    cp "${source_manifest}" "${artifacts_dir}/source-manifest.sha256"
  fi
  while IFS= read -r -d '' source; do
    relative=${source#"${temporary_root}/"}
    destination=${artifacts_dir}/stata-logs/${relative//\//__}.sanitized.log
    sanitize_stata_log "${source}" "${destination}"
  done < <(
    find "${temporary_root}" -maxdepth 3 -type f \
      \( -name '*.log' -o -name 'console.txt' \) -print0
  )
  if [[ -n "${candidate_dir}" && -d "${candidate_dir}" ]]; then
    while IFS= read -r -d '' source; do
      install -m 0755 "${source}" \
        "${artifacts_dir}/candidates/$(basename -- "${source}")"
    done < <(find "${candidate_dir}" -maxdepth 1 -type f -name '*.plugin' -print0)
  fi
  printf '%s\n' \
    'Stata logs in this directory begin at the first batch prompt.' \
    'Startup banners and raw temporary logs were not retained.' \
    > "${artifacts_dir}/SANITIZED_EVIDENCE.txt"
}

cleanup() {
  export_sanitized_evidence || true
  if [[ -n "${receipt_temporary}" && -f "${receipt_temporary}" ]]; then
    rm -f -- "${receipt_temporary}" || true
  fi
  if [[ -n "${temporary_root}" && -d "${temporary_root}" ]]; then
    rm -rf -- "${temporary_root}" || true
  fi
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/vckss-macos-qualify.XXXXXX")
spi_dir=${temporary_root}/stata-spi
cargo_target_dir=${temporary_root}/cargo-target
candidate_dir=${temporary_root}/candidates
test_package_dir=${temporary_root}/test-package
universal_test_package_dir=${temporary_root}/test-package-universal
mkdir -p "${candidate_dir}" "${test_package_dir}" \
  "${universal_test_package_dir}"

# Prove that the exact Cargo executable can launch the exact rustc before the
# real build. The dependency-free crate and all of its output remain temporary.
rust_preflight_dir=${temporary_root}/rust-toolchain-preflight
mkdir -p "${rust_preflight_dir}/src"
cat > "${rust_preflight_dir}/Cargo.toml" <<'EOF'
[package]
name = "vckss-rust-toolchain-preflight"
version = "0.0.0"
edition = "2021"

[workspace]
EOF
cat > "${rust_preflight_dir}/src/lib.rs" <<'EOF'
pub fn toolchain_preflight() {}
EOF
(
  cd "${rust_preflight_dir}"
  env \
    PATH="${rust_toolchain_bin}:${host_path}" \
    RUSTC="${rust_rustc}" \
    RUSTC_WRAPPER= \
    RUSTC_WORKSPACE_WRAPPER= \
    CARGO_TARGET_DIR="${rust_preflight_dir}/target" \
    "${rust_cargo}" check --offline --quiet --target "${rust_host_target}"
)

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

source_manifest=${temporary_root}/source-manifest.sha256
source_inputs=(
  "${repo_root}/rust/Cargo.toml"
  "${repo_root}/rust/Cargo.lock"
  "${repo_root}/rust/rust-toolchain.toml"
  "${repo_root}/rust/crates/vckss-core"
  "${repo_root}/rust/crates/vckss-plugin"
  "${repo_root}/rust/vendor/cmg/Cargo.toml"
  "${repo_root}/rust/vendor/cmg/src"
  "${repo_root}/rust/vendor/cmg/VENDOR.md"
  "${script_dir}/Cargo.toml"
  "${script_dir}/Cargo.lock"
  "${script_dir}/build.rs"
  "${script_dir}/cshim"
  "${script_dir}/include"
  "${script_dir}/src"
  "${script_dir}/tests"
  "${script_dir}/fetch_stata_spi.sh"
  "${script_dir}/stata-spi.sha256"
  "${script_dir}/qualify_macos.sh"
  "${script_dir}/README.md"
  "${package_dir}/fevc.ado"
  "${package_dir}/fevc.mata"
  "${package_dir}/fevc_graph.mata"
  "${package_dir}/fevc_cmg.mata"
  "${package_dir}/fevc_solver.mata"
  "${package_dir}/fevc_rng.mata"
  "${package_dir}/fevc_scale.mata"
  "${package_dir}/fevc_resource.mata"
  "${package_dir}/fevc_scale_engine.mata"
  "${package_dir}/fevc_scale_runtime.mata"
  "${package_dir}/_fevc_display.ado"
  "${package_dir}/_fevc_memory_options.ado"
  "${package_dir}/_fevc_lifecycle.ado"
  "${package_dir}/fevc_run.ado"
  "${package_dir}/fevc_rust.ado"
  "${package_dir}/_fevc_rust_plugin_call.ado"
  "${package_dir}/_fevc_rust_solve_v4.ado"
  "${package_dir}/_fevc_rust_solve_v5.ado"
  "${package_dir}/_fevc_rust_plan_receipt.ado"
  "${package_dir}/_fevc_rust_reconcile_comp_v7.ado"
  "${package_dir}/_fevc_rust_reconcile_exact_v7.ado"
  "${package_dir}/_fevc_rust_post_comp_v7.ado"
  "${package_dir}/_fevc_rust_post_exact_v7.ado"
  "${package_dir}/_fevc_rust_capture_stayers.ado"
  "${package_dir}/_fevc_rust_post_stayer_hybrid.ado"
  "${package_dir}/_fevc_rust_macos.ado"
  "${package_dir}/_fevc_rust_windows.ado"
  "${package_dir}/_fevc_rust_linux.ado"
  "${package_dir}/_fevc_rust_public_call.ado"
  "${package_dir}/_fevc_component_model_route.ado"
  "${package_dir}/_fevc_exact_inference_model_post.ado"
  "${package_dir}/_fevc_rust_component_attach.ado"
  "${package_dir}/_fevc_rust_component_fetch.ado"
  "${package_dir}/_fevc_rust_component_post.ado"
  "${package_dir}/_fevc_failure_guidance.ado"
  "${package_dir}/fevc.pkg"
  "${package_dir}/stata.toc"
  "${package_dir}/fevc.sthlp"
  "${package_dir}/README.md"
  "${package_dir}/TESTING.md"
  "${package_dir}/tests/stata/run_all.do"
  "${package_dir}/tests/stata/test_memory_policy.do"
  "${package_dir}/tests/stata/test_rust_plugin.do"
  "${package_dir}/tests/stata/test_rust_mata_diagnostic.do"
  "${package_dir}/tests/stata/test_rust_mata_shared_atoms.do"
  "${package_dir}/tests/stata/test_rust_public.do"
  "${package_dir}/tests/stata/test_rust_exact_controls.do"
  "${package_dir}/tests/stata/test_rust_generic_jla.do"
  "${package_dir}/tests/stata/test_rust_planned_v4.do"
  "${package_dir}/tests/stata/test_rust_planned_compressed.do"
  "${package_dir}/tests/stata/test_rust_planned_compressed_post.do"
  "${package_dir}/tests/stata/test_rust_full_cmg_v2.do"
  "${package_dir}/tests/stata/test_rust_public_exact.do"
  "${package_dir}/tests/stata/test_rust_public_generic.do"
  "${package_dir}/tests/stata/test_stayers_hybrid.do"
  "${package_dir}/tests/stata/test_rust_component_inference.do"
  "${package_dir}/tests/stata/test_rust_match_component_inference.do"
  "${package_dir}/tests/stata/test_rust_individual_inference.do"
  "${package_dir}/tests/stata/test_rust_public_install.do"
  "${package_dir}/tests/stata/test_backend_routing.do"
)

write_source_manifest() {
  local output=$1
  local source_file relative_path
  : > "${output}"
  while IFS= read -r source_file; do
    [[ -f "${source_file}" ]] || continue
    relative_path=${source_file#"${repo_root}/"}
    printf '%s  %s\n' "$(hash_file "${source_file}")" "${relative_path}" >> "${output}"
  done < <(find "${source_inputs[@]}" -type f -print | LC_ALL=C sort -u)
}

write_source_manifest "${source_manifest}"
source_hash_before=$(hash_file "${source_manifest}")
source_file_count=$(wc -l < "${source_manifest}" | tr -d ' ')
commit=$(git -C "${repo_root}" rev-parse HEAD)
branch=$(git -C "${repo_root}" symbolic-ref --short -q HEAD || printf 'detached')
if [[ -n $(git -C "${repo_root}" status --porcelain=v1 --untracked-files=all) ]]; then
  dirty_status=dirty
else
  dirty_status=clean
fi

VCKSS_STATA_SPI_DIR="${spi_dir}" "${script_dir}/fetch_stata_spi.sh"
spi_source_hash=$(hash_file "${spi_dir}/stplugin.c")
spi_header_hash=$(hash_file "${spi_dir}/stplugin.h")

cshim_interrupt_test_binary=${temporary_root}/vckss-cshim-interrupt-test
clang -std=c11 -Wall -Wextra -Werror \
  -DSYSTEM=APPLEMAC \
  -I "${spi_dir}" \
  -I "${script_dir}/cshim" \
  -I "${script_dir}/include" \
  "${script_dir}/tests/cshim_interrupt_test.c" \
  -o "${cshim_interrupt_test_binary}"
"${cshim_interrupt_test_binary}"
cshim_interrupt_test_status=PASS

cshim_error_test_binary=${temporary_root}/vckss-cshim-error-transport-test
clang -std=c11 -Wall -Wextra -Werror -ffunction-sections \
  -DSYSTEM=APPLEMAC \
  -I "${spi_dir}" \
  -I "${script_dir}/cshim" \
  -I "${script_dir}/include" \
  "${script_dir}/tests/cshim_error_transport_test.c" \
  -Wl,-dead_strip \
  -o "${cshim_error_test_binary}"
"${cshim_error_test_binary}"
cshim_error_transport_test_status=PASS

abi_header_object=${temporary_root}/vckss-abi-header-compat.o
clang -std=c11 -Wall -Wextra -Werror \
  -I "${script_dir}/include" \
  -c "${script_dir}/tests/abi_header_compat_test.c" \
  -o "${abi_header_object}"
abi_header_compat_test_status=PASS

plugin_cargo() {
  env \
    VCKSS_STATA_SPI_DIR="${spi_dir}" \
    CARGO_TARGET_DIR="${cargo_target_dir}" \
    PATH="${rust_toolchain_bin}:${host_path}" \
    RUSTC="${rust_rustc}" \
    RUSTC_WRAPPER= \
    RUSTC_WORKSPACE_WRAPPER= \
    "${rust_cargo}" "$@"
}

plugin_cargo fmt --manifest-path "${manifest_path}" --all -- --check
cargo_fmt_status=PASS
plugin_cargo clippy --manifest-path "${manifest_path}" \
  --locked --all-targets -- -D warnings
cargo_clippy_status=PASS
plugin_cargo test --manifest-path "${manifest_path}" --locked --all-targets
cargo_test_status=PASS

rustup target add --toolchain "${rust_toolchain}" \
  aarch64-apple-darwin x86_64-apple-darwin

build_target() {
  local target=$1
  local minimum_version=$2
  env \
    VCKSS_STATA_SPI_DIR="${spi_dir}" \
    CARGO_TARGET_DIR="${cargo_target_dir}" \
    MACOSX_DEPLOYMENT_TARGET="${minimum_version}" \
    PATH="${rust_toolchain_bin}:${host_path}" \
    RUSTC="${rust_rustc}" \
    RUSTC_WRAPPER= \
    RUSTC_WORKSPACE_WRAPPER= \
    "${rust_cargo}" build \
      --manifest-path "${manifest_path}" \
      --locked \
      --release \
      --target "${target}"
}

arm64_floor=11.0
x86_64_floor=10.13
build_target aarch64-apple-darwin "${arm64_floor}"
source_manifest_after_arm64=${temporary_root}/source-manifest-after-arm64.sha256
write_source_manifest "${source_manifest_after_arm64}"
[[ "${source_hash_before}" == "$(hash_file "${source_manifest_after_arm64}")" ]] || \
  fail "qualification source files changed during the arm64 build"
build_target x86_64-apple-darwin "${x86_64_floor}"
source_manifest_after_x86_64=${temporary_root}/source-manifest-after-x86_64.sha256
write_source_manifest "${source_manifest_after_x86_64}"
[[ "${source_hash_before}" == "$(hash_file "${source_manifest_after_x86_64}")" ]] || \
  fail "qualification source files changed during the x86_64 build"

arm64_candidate=${candidate_dir}/fevc_rust_macos_arm64.plugin
x86_64_candidate=${candidate_dir}/fevc_rust_macos_x86_64.plugin
universal_candidate=${candidate_dir}/fevc_rust_macos.plugin
cp "${cargo_target_dir}/aarch64-apple-darwin/release/libvckss_stata.dylib" \
  "${arm64_candidate}"
cp "${cargo_target_dir}/x86_64-apple-darwin/release/libvckss_stata.dylib" \
  "${x86_64_candidate}"
codesign --force --sign - --timestamp=none "${arm64_candidate}"
codesign --force --sign - --timestamp=none "${x86_64_candidate}"
lipo -create "${arm64_candidate}" "${x86_64_candidate}" \
  -output "${universal_candidate}"
codesign --force --sign - --timestamp=none "${universal_candidate}"

expected_install_id=@rpath/fevc_rust_macos.plugin
required_exports=(
  _pginit
  _stata_call
)
while IFS= read -r public_export; do
  required_exports[${#required_exports[@]}]=${public_export}
done < <(
  sed -nE \
    's/^[^()]*(vckss_rust_[[:alnum:]_]+)\(.*/_\1/p' \
    "${script_dir}/include/vckss_rust.h"
)
[[ ${#required_exports[@]} -gt 2 ]] || \
  fail "no public Rust exports found in include/vckss_rust.h"

architecture_count() {
  lipo -archs "$1" | awk '{print NF}'
}

require_architecture() {
  local artifact=$1
  local architecture=$2
  local architectures
  architectures=$(lipo -archs "${artifact}")
  case " ${architectures} " in
    *" ${architecture} "*) ;;
    *) fail "${artifact} lacks ${architecture}; found: ${architectures}" ;;
  esac
}

deployment_floor() {
  local artifact=$1
  local architecture=$2
  otool -arch "${architecture}" -l "${artifact}" | awk '
    $1 == "cmd" && ($2 == "LC_BUILD_VERSION" || $2 == "LC_VERSION_MIN_MACOSX") {
      found = 1
      next
    }
    found && ($1 == "minos" || $1 == "version") {
      if (value == "") value = $2
      found = 0
    }
    END {if (value != "") print value}
  '
}

install_id() {
  local artifact=$1
  local architecture=$2
  otool -arch "${architecture}" -D "${artifact}" | \
    awk '
      NR > 1 && ($1 ~ /^@/ || $1 ~ /^\//) && value == "" {value = $1}
      END {if (value != "") print value}
    '
}

dependency_list() {
  local artifact=$1
  local architecture=$2
  otool -arch "${architecture}" -L "${artifact}" | \
    awk 'NR > 1 && ($1 ~ /^@/ || $1 ~ /^\//) {print $1}' | paste -sd, -
}

verify_dependencies() {
  local artifact=$1
  local architecture=$2
  local dependency
  while IFS= read -r dependency; do
    case "${dependency}" in
      "${expected_install_id}"|/usr/lib/libSystem.B.dylib) ;;
      *) fail "unexpected ${architecture} dependency in ${artifact}: ${dependency}" ;;
    esac
  done < <(
    otool -arch "${architecture}" -L "${artifact}" | \
      awk 'NR > 1 && ($1 ~ /^@/ || $1 ~ /^\//) {print $1}'
  )
}

verify_exports() {
  local artifact=$1
  local architecture=$2
  local exports symbol
  exports=$(nm -arch "${architecture}" -gU "${artifact}" | awk '{print $NF}')
  for symbol in "${required_exports[@]}"; do
    grep -F -x -q "${symbol}" <<<"${exports}" || \
      fail "missing ${architecture} export ${symbol} in ${artifact}"
  done
}

verify_slice() {
  local artifact=$1
  local architecture=$2
  local expected_floor=$3
  require_architecture "${artifact}" "${architecture}"
  [[ $(deployment_floor "${artifact}" "${architecture}") == "${expected_floor}" ]] || \
    fail "unexpected ${architecture} deployment floor in ${artifact}"
  [[ $(install_id "${artifact}" "${architecture}") == "${expected_install_id}" ]] || \
    fail "unexpected ${architecture} install ID in ${artifact}"
  verify_dependencies "${artifact}" "${architecture}"
  verify_exports "${artifact}" "${architecture}"
  codesign --verify --strict "${artifact}"
}

[[ $(architecture_count "${arm64_candidate}") == 1 ]] || \
  fail "arm64 candidate is not thin"
[[ $(architecture_count "${x86_64_candidate}") == 1 ]] || \
  fail "x86_64 candidate is not thin"
[[ $(architecture_count "${universal_candidate}") == 2 ]] || \
  fail "universal candidate does not contain exactly two architectures"
verify_slice "${arm64_candidate}" arm64 "${arm64_floor}"
verify_slice "${x86_64_candidate}" x86_64 "${x86_64_floor}"
verify_slice "${universal_candidate}" arm64 "${arm64_floor}"
verify_slice "${universal_candidate}" x86_64 "${x86_64_floor}"

stata_architectures=$(lipo -archs "${stata_binary}")
case " ${stata_architectures} " in
  *" arm64 "*) ;;
  *) fail "Stata executable lacks native arm64 support" ;;
esac
if arch -x86_64 /usr/bin/true >/dev/null 2>&1; then
  rosetta_status=AVAILABLE
  case " ${stata_architectures} " in
    *" x86_64 "*) ;;
    *) fail "Rosetta is available, but Stata lacks x86_64 support" ;;
  esac
else
  rosetta_status=UNAVAILABLE
fi

# Test the exact signed thin candidates under their architecture-specific
# names.  The universal candidate is exercised separately through a second
# package so neither receipt can accidentally substitute one artifact for the
# other.
populate_test_package() {
  local destination=$1
  local arm64_artifact=$2
  local x86_64_artifact=$3
  local package_file
  for package_file in "${package_dir}"/*; do
    [[ -f "${package_file}" ]] || continue
    case "${package_file}" in
      *.plugin|*/fevc.pkg) continue ;;
    esac
    ln -s "${package_file}" "${destination}/$(basename -- "${package_file}")"
  done
  cp "${package_dir}/fevc.pkg" "${destination}/fevc.pkg"
  printf 'f fevc_rust_macos_arm64.plugin\nf fevc_rust_macos_x86_64.plugin\n' \
    >> "${destination}/fevc.pkg"
  cp "${arm64_artifact}" \
    "${destination}/fevc_rust_macos_arm64.plugin"
  cp "${x86_64_artifact}" \
    "${destination}/fevc_rust_macos_x86_64.plugin"
}

populate_test_package "${test_package_dir}" \
  "${arm64_candidate}" "${x86_64_candidate}"
populate_test_package "${universal_test_package_dir}" \
  "${universal_candidate}" "${universal_candidate}"

tested_arm64_thin_hash=$(hash_file \
  "${test_package_dir}/fevc_rust_macos_arm64.plugin")
tested_x86_64_thin_hash=$(hash_file \
  "${test_package_dir}/fevc_rust_macos_x86_64.plugin")
tested_universal_arm64_alias_hash=$(hash_file \
  "${universal_test_package_dir}/fevc_rust_macos_arm64.plugin")
tested_universal_x86_64_alias_hash=$(hash_file \
  "${universal_test_package_dir}/fevc_rust_macos_x86_64.plugin")
[[ "${tested_arm64_thin_hash}" == "$(hash_file "${arm64_candidate}")" ]] || \
  fail "arm64 thin test artifact does not match the signed candidate"
[[ "${tested_x86_64_thin_hash}" == "$(hash_file "${x86_64_candidate}")" ]] || \
  fail "x86_64 thin test artifact does not match the signed candidate"
[[ "${tested_universal_arm64_alias_hash}" == \
  "${tested_universal_x86_64_alias_hash}" ]] || \
  fail "universal test aliases differ"
[[ "${tested_universal_arm64_alias_hash}" == \
  "$(hash_file "${universal_candidate}")" ]] || \
  fail "universal test artifact does not match the signed candidate"

environment_probe=${temporary_root}/stata_environment.do
cat > "${environment_probe}" <<'EOF'
version 18.0
display as result "VCKSS_STATA_ENV version=`c(stata_version)' edition=`c(edition_real)' os=`c(os)' machine=`c(machine_type)'"
exit 0
EOF

last_run_directory=
run_stata_case() {
  local architecture=$1
  local label=$2
  local do_file=$3
  local marker=$4
  local run_directory=${temporary_root}/stata-${architecture}-${label}
  local -a stata_arguments
  local return_code
  shift 4
  mkdir -p "${run_directory}"
  stata_arguments=(-b do "${do_file}")
  while [[ $# -gt 0 ]]; do
    stata_arguments[${#stata_arguments[@]}]=$1
    shift
  done
  set +e
  (
    cd "${run_directory}"
    arch "-${architecture}" "${stata_binary}" "${stata_arguments[@]}"
  ) > "${run_directory}/console.txt" 2>&1
  return_code=$?
  set -e
  [[ ${return_code} -eq 0 ]] || \
    fail "Stata ${architecture} ${label} returned ${return_code}; sanitized transcript exported on exit"
  grep -R -F -q -- "${marker}" "${run_directory}" || \
    fail "Stata ${architecture} ${label} omitted PASS marker; sanitized transcript exported on exit"
  last_run_directory=${run_directory}
}

extract_stata_environment() {
  local run_directory=$1
  local environment
  environment=$(
    grep -R -F -h 'VCKSS_STATA_ENV ' "${run_directory}" | \
      tr -d '\r' | \
      sed -nE 's/^.*(VCKSS_STATA_ENV version=[0-9].*)$/\1/p' | \
      tail -n 1 || true
  )
  [[ -n "${environment}" ]] || \
    fail "Stata environment probe did not emit a usable environment record"
  printf '%s\n' "${environment}"
}

run_stata_case arm64 environment "${environment_probe}" VCKSS_STATA_ENV
stata_arm64_environment=$(extract_stata_environment "${last_run_directory}")
run_stata_case arm64 lifecycle \
  "${package_dir}/tests/stata/test_rust_plugin.do" \
  'FEVC RUST PLUGIN PASS' "${test_package_dir}"
run_stata_case arm64 diagnostic \
  "${package_dir}/tests/stata/test_rust_mata_diagnostic.do" \
  'FEVC RUST MATA DIAGNOSTIC PASS' "${test_package_dir}"
run_stata_case arm64 shared-atoms \
  "${package_dir}/tests/stata/test_rust_mata_shared_atoms.do" \
  'PASS test_rust_mata_shared_atoms.do' "${test_package_dir}"
run_stata_case arm64 public-route \
  "${package_dir}/tests/stata/test_rust_public.do" \
  'PASS test_rust_public.do' "${test_package_dir}"
run_stata_case arm64 exact-controls \
  "${package_dir}/tests/stata/test_rust_exact_controls.do" \
  'FEVC RUST EXACT CONTROLS PASS' "${test_package_dir}"
run_stata_case arm64 private-generic \
  "${package_dir}/tests/stata/test_rust_generic_jla.do" \
  'FEVC RUST GENERIC JLA PASS' "${test_package_dir}"
run_stata_case arm64 private-planned-v4 \
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \
  'FEVC RUST PLANNED V4 PASS' "${test_package_dir}"
run_stata_case arm64 private-planned-compressed \
  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \
  'FEVC RUST PLANNED COMPRESSED V4 PASS' "${test_package_dir}"
run_stata_case arm64 public-planned-compressed \
  "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \
  'FEVC RUST COMPRESSED PUBLIC ROUTES PASS' "${test_package_dir}"
run_stata_case arm64 public-full-cmg \
  "${package_dir}/tests/stata/test_rust_full_cmg_v2.do" \
  'PASS test_rust_full_cmg_v2.do' "${test_package_dir}"
run_stata_case arm64 public-exact \
  "${package_dir}/tests/stata/test_rust_public_exact.do" \
  'FEVC RUST PUBLIC EXACT PASS' "${test_package_dir}"
run_stata_case arm64 public-generic \
  "${package_dir}/tests/stata/test_rust_public_generic.do" \
  'FEVC RUST PUBLIC GENERIC PASS' "${test_package_dir}"
run_stata_case arm64 public-stayer-hybrid \
  "${package_dir}/tests/stata/test_stayers_hybrid.do" \
  'PASS test_stayers_hybrid.do' "${test_package_dir}"
run_stata_case arm64 public-component-inference \
  "${package_dir}/tests/stata/test_rust_component_inference.do" \
  'PASS test_rust_component_inference.do' "${test_package_dir}"
run_stata_case arm64 public-match-component-inference \
  "${package_dir}/tests/stata/test_rust_match_component_inference.do" \
  'PASS test_rust_match_component_inference.do' "${test_package_dir}"
run_stata_case arm64 public-individual-inference \
  "${package_dir}/tests/stata/test_rust_individual_inference.do" \
  'FEVC INDIVIDUAL INFERENCE PASS' "${test_package_dir}"
run_stata_case arm64 backend-routing \
  "${package_dir}/tests/stata/test_backend_routing.do" \
  'PASS test_backend_routing.do' "${test_package_dir}"
run_stata_case arm64 universal-lifecycle \
  "${package_dir}/tests/stata/test_rust_plugin.do" \
  'FEVC RUST PLUGIN PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-route \
  "${package_dir}/tests/stata/test_rust_public.do" \
  'PASS test_rust_public.do' "${universal_test_package_dir}"
run_stata_case arm64 universal-exact-controls \
  "${package_dir}/tests/stata/test_rust_exact_controls.do" \
  'FEVC RUST EXACT CONTROLS PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-private-generic \
  "${package_dir}/tests/stata/test_rust_generic_jla.do" \
  'FEVC RUST GENERIC JLA PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-private-planned-v4 \
  "${package_dir}/tests/stata/test_rust_planned_v4.do" \
  'FEVC RUST PLANNED V4 PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-private-planned-compressed \
  "${package_dir}/tests/stata/test_rust_planned_compressed.do" \
  'FEVC RUST PLANNED COMPRESSED V4 PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-planned-compressed \
  "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \
  'FEVC RUST COMPRESSED PUBLIC ROUTES PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-full-cmg \
  "${package_dir}/tests/stata/test_rust_full_cmg_v2.do" \
  'PASS test_rust_full_cmg_v2.do' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-exact \
  "${package_dir}/tests/stata/test_rust_public_exact.do" \
  'FEVC RUST PUBLIC EXACT PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-generic \
  "${package_dir}/tests/stata/test_rust_public_generic.do" \
  'FEVC RUST PUBLIC GENERIC PASS' "${universal_test_package_dir}"
run_stata_case arm64 universal-public-stayer-hybrid \
  "${package_dir}/tests/stata/test_stayers_hybrid.do" \
  'PASS test_stayers_hybrid.do' "${universal_test_package_dir}"
arm64_install_root=${temporary_root}/install-arm64
mkdir -p "${arm64_install_root}"
run_stata_case arm64 clean-install \
  "${package_dir}/tests/stata/test_rust_public_install.do" \
  'PASS test_rust_public_install.do' "${test_package_dir}" \
  "${arm64_install_root}" qualified \
  "${package_dir}/tests/stata"
arm64_unavailable_install_root=${temporary_root}/install-unavailable-arm64
mkdir -p "${arm64_unavailable_install_root}"
run_stata_case arm64 canonical-install-unavailable \
  "${package_dir}/tests/stata/test_rust_public_install.do" \
  'PASS test_rust_public_install.do' "${package_dir}" \
  "${arm64_unavailable_install_root}" unavailable
arm64_test_status=PASS_NATIVE

if [[ "${rosetta_status}" == AVAILABLE ]]; then
  run_stata_case x86_64 environment "${environment_probe}" VCKSS_STATA_ENV
  stata_x86_64_environment=$(extract_stata_environment "${last_run_directory}")
  run_stata_case x86_64 lifecycle \
    "${package_dir}/tests/stata/test_rust_plugin.do" \
    'FEVC RUST PLUGIN PASS' "${test_package_dir}"
  run_stata_case x86_64 diagnostic \
    "${package_dir}/tests/stata/test_rust_mata_diagnostic.do" \
    'FEVC RUST MATA DIAGNOSTIC PASS' "${test_package_dir}"
  run_stata_case x86_64 shared-atoms \
    "${package_dir}/tests/stata/test_rust_mata_shared_atoms.do" \
    'PASS test_rust_mata_shared_atoms.do' "${test_package_dir}"
  run_stata_case x86_64 public-route \
    "${package_dir}/tests/stata/test_rust_public.do" \
    'PASS test_rust_public.do' "${test_package_dir}"
  run_stata_case x86_64 exact-controls \
    "${package_dir}/tests/stata/test_rust_exact_controls.do" \
    'FEVC RUST EXACT CONTROLS PASS' "${test_package_dir}"
  run_stata_case x86_64 private-generic \
    "${package_dir}/tests/stata/test_rust_generic_jla.do" \
    'FEVC RUST GENERIC JLA PASS' "${test_package_dir}"
  run_stata_case x86_64 private-planned-v4 \
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \
    'FEVC RUST PLANNED V4 PASS' "${test_package_dir}"
  run_stata_case x86_64 private-planned-compressed \
    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \
    'FEVC RUST PLANNED COMPRESSED V4 PASS' "${test_package_dir}"
  run_stata_case x86_64 public-planned-compressed \
    "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \
    'FEVC RUST COMPRESSED PUBLIC ROUTES PASS' "${test_package_dir}"
  run_stata_case x86_64 public-full-cmg \
    "${package_dir}/tests/stata/test_rust_full_cmg_v2.do" \
    'PASS test_rust_full_cmg_v2.do' "${test_package_dir}"
  run_stata_case x86_64 public-exact \
    "${package_dir}/tests/stata/test_rust_public_exact.do" \
    'FEVC RUST PUBLIC EXACT PASS' "${test_package_dir}"
  run_stata_case x86_64 public-generic \
    "${package_dir}/tests/stata/test_rust_public_generic.do" \
    'FEVC RUST PUBLIC GENERIC PASS' "${test_package_dir}"
  run_stata_case x86_64 public-stayer-hybrid \
    "${package_dir}/tests/stata/test_stayers_hybrid.do" \
    'PASS test_stayers_hybrid.do' "${test_package_dir}"
  run_stata_case x86_64 public-component-inference \
    "${package_dir}/tests/stata/test_rust_component_inference.do" \
    'PASS test_rust_component_inference.do' "${test_package_dir}"
  run_stata_case x86_64 public-match-component-inference \
    "${package_dir}/tests/stata/test_rust_match_component_inference.do" \
    'PASS test_rust_match_component_inference.do' "${test_package_dir}"
  run_stata_case x86_64 public-individual-inference \
    "${package_dir}/tests/stata/test_rust_individual_inference.do" \
    'FEVC INDIVIDUAL INFERENCE PASS' "${test_package_dir}"
  run_stata_case x86_64 backend-routing \
    "${package_dir}/tests/stata/test_backend_routing.do" \
    'PASS test_backend_routing.do' "${test_package_dir}"
  run_stata_case x86_64 universal-lifecycle \
    "${package_dir}/tests/stata/test_rust_plugin.do" \
    'FEVC RUST PLUGIN PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-route \
    "${package_dir}/tests/stata/test_rust_public.do" \
    'PASS test_rust_public.do' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-exact-controls \
    "${package_dir}/tests/stata/test_rust_exact_controls.do" \
    'FEVC RUST EXACT CONTROLS PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-private-generic \
    "${package_dir}/tests/stata/test_rust_generic_jla.do" \
    'FEVC RUST GENERIC JLA PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-private-planned-v4 \
    "${package_dir}/tests/stata/test_rust_planned_v4.do" \
    'FEVC RUST PLANNED V4 PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-private-planned-compressed \
    "${package_dir}/tests/stata/test_rust_planned_compressed.do" \
    'FEVC RUST PLANNED COMPRESSED V4 PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-planned-compressed \
    "${package_dir}/tests/stata/test_rust_planned_compressed_post.do" \
    'FEVC RUST COMPRESSED PUBLIC ROUTES PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-full-cmg \
    "${package_dir}/tests/stata/test_rust_full_cmg_v2.do" \
    'PASS test_rust_full_cmg_v2.do' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-exact \
    "${package_dir}/tests/stata/test_rust_public_exact.do" \
    'FEVC RUST PUBLIC EXACT PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-generic \
    "${package_dir}/tests/stata/test_rust_public_generic.do" \
    'FEVC RUST PUBLIC GENERIC PASS' "${universal_test_package_dir}"
  run_stata_case x86_64 universal-public-stayer-hybrid \
    "${package_dir}/tests/stata/test_stayers_hybrid.do" \
    'PASS test_stayers_hybrid.do' "${universal_test_package_dir}"
  x86_64_install_root=${temporary_root}/install-x86_64
  mkdir -p "${x86_64_install_root}"
  run_stata_case x86_64 clean-install \
    "${package_dir}/tests/stata/test_rust_public_install.do" \
    'PASS test_rust_public_install.do' "${test_package_dir}" \
    "${x86_64_install_root}" qualified \
    "${package_dir}/tests/stata"
  x86_64_unavailable_install_root=${temporary_root}/install-unavailable-x86_64
  mkdir -p "${x86_64_unavailable_install_root}"
  run_stata_case x86_64 canonical-install-unavailable \
    "${package_dir}/tests/stata/test_rust_public_install.do" \
    'PASS test_rust_public_install.do' "${package_dir}" \
    "${x86_64_unavailable_install_root}" unavailable
  x86_64_test_status=PASS_ROSETTA
else
  stata_x86_64_environment=not-run
  x86_64_test_status=SKIPPED_ROSETTA_UNAVAILABLE
fi

source_manifest_after=${temporary_root}/source-manifest-after.sha256
write_source_manifest "${source_manifest_after}"
source_hash_after=$(hash_file "${source_manifest_after}")
[[ "${source_hash_before}" == "${source_hash_after}" ]] || \
  fail "qualification source files changed during the run"
[[ "${commit}" == "$(git -C "${repo_root}" rev-parse HEAD)" ]] || \
  fail "repository commit changed during the run"
if [[ -n $(git -C "${repo_root}" status --porcelain=v1 --untracked-files=all) ]]; then
  dirty_status_end=dirty
else
  dirty_status_end=clean
fi

staged_arm64=${package_dir}/fevc_rust_macos_arm64.plugin
staged_x86_64=${package_dir}/fevc_rust_macos_x86_64.plugin
staged_universal=${package_dir}/fevc_rust_macos.plugin
install -m 0755 "${arm64_candidate}" "${staged_arm64}"
install -m 0755 "${x86_64_candidate}" "${staged_x86_64}"
install -m 0755 "${universal_candidate}" "${staged_universal}"

arm64_hash=$(hash_file "${staged_arm64}")
x86_64_hash=$(hash_file "${staged_x86_64}")
universal_hash=$(hash_file "${staged_universal}")
[[ "${arm64_hash}" == "$(hash_file "${arm64_candidate}")" ]] || \
  fail "staged arm64 artifact hash changed"
[[ "${x86_64_hash}" == "$(hash_file "${x86_64_candidate}")" ]] || \
  fail "staged x86_64 artifact hash changed"
[[ "${universal_hash}" == "$(hash_file "${universal_candidate}")" ]] || \
  fail "staged universal artifact hash changed"
[[ "${arm64_hash}" == "${tested_arm64_thin_hash}" ]] || \
  fail "staged arm64 artifact differs from the exact thin artifact tested"
[[ "${x86_64_hash}" == "${tested_x86_64_thin_hash}" ]] || \
  fail "staged x86_64 artifact differs from the exact thin artifact tested"
[[ "${universal_hash}" == "${tested_universal_arm64_alias_hash}" ]] || \
  fail "staged universal artifact differs from the exact universal artifact tested"

if [[ "${dirty_status}" == dirty || "${dirty_status_end}" == dirty ]]; then
  classification=LOCAL_CHECKPOINT_DIRTY_TREE
elif [[ "${rosetta_status}" == AVAILABLE ]]; then
  classification=CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION
else
  classification=CLEAN_LOCAL_MACOS_ARM64_QUALIFICATION_X86_RUNTIME_UNTESTED
fi

rustc_version=$("${rust_rustc}" --version)
cargo_version=$("${rust_cargo}" --version)
clang_version=$(clang --version | sed -n '1p')
xcode_version=$(xcodebuild -version | paste -sd, -)
macos_version=$(sw_vers -productVersion)
required_export_text=$(printf '%s,' "${required_exports[@]}")
required_export_text=${required_export_text%,}
arm64_dependencies=$(dependency_list "${staged_universal}" arm64)
x86_64_dependencies=$(dependency_list "${staged_universal}" x86_64)
arm64_file_description=$(file -b "${staged_arm64}")
x86_64_file_description=$(file -b "${staged_x86_64}")
universal_file_description=$(file -b "${staged_universal}")
qualification_scope_text=$(qualification_scope "${rosetta_status}")
tested_artifact_scope_text=$(tested_artifact_scope "${rosetta_status}")

receipt_temporary=$(mktemp "${receipt_parent}/.$(basename -- "${receipt_path}").tmp.XXXXXX")
{
  printf 'VCKSS_MACOS_CANDIDATE_RECEIPT_V1\n'
  printf 'classification=%s\n' "${classification}"
  printf 'scope=%s\n' "${qualification_scope_text}"
  printf 'tested_routes=exact-match-observation-joint-fixedoffset-controls-factors-fweights-stored-targetweights-if-in-deletionid-rng-not-applicable;frozen-compressed-jla-match-joint-no-controls-counter-v1-fweights-stored-targetweights-if-in-deletionid;explicit-generic-jla-engine-generic-diagonal-numeric-batch-counter-v1-controls-q0-q32-factors-match-observation-joint-fixedoffset-fweights-stored-targetweights-if-in-deletionid;planned-compressed-jla-v4-v7-engine-auto-to-compressed-route-diagonal-explicit-batches-counter-v1-fweights-stored-targetweights-matchid-probeorder;public-exact-and-generic-jla-stayer-hybrid-backend-rust-stayers-both-combined-headline-mixed-deletion-augmentation-reconciliation-differential-oracle-counter-v1-lifecycle;public-generic-jla-observation-component-inference-structured-common-leverage-q0-q1-spectrum-counter-v1;public-compressed-jla-backend-rust-engine-auto-no-controls-match-joint-fixedoffset-auto-to-exact-auto-to-diagonal-forced-cmg-independent-numeric-batches-wall-advisory-counter-v1-fweights-stored-targetweights-matchid-probeorder;production-full-cmg-v2-no-control-match-jla-explicit-rust-auto-backend-auto-rng-counter-v1-implicit-match-memory-refinement-cancellation-lifecycle;planned-generic-jla-v4-v7-engine-generic-route-auto-independent-batches-wall-advisory-counter-v1-probeorder;public-generic-jla-probeorder-permutation-batch-invariance-clean-install\n'
  printf 'excluded_claims=public-release,Windows,Linux,native-Intel,representative-scale,human-license-provenance-review\n'
  printf 'tested_match_component_route=public-generic-jla-fixedoffset-movers-match-q0-q1-structured-common-leverage-diagonal-cmg-counter-v1-frequency-targetweight-declared-match-id-unit-receipt-v1\n'
  printf 'commit=%s\n' "${commit}"
  printf 'branch=%s\n' "${branch}"
  printf 'dirty_status_start=%s\n' "${dirty_status}"
  printf 'dirty_status_end=%s\n' "${dirty_status_end}"
  printf 'source_manifest_sha256=%s\n' "${source_hash_before}"
  printf 'source_file_count=%s\n' "${source_file_count}"
  printf 'stata_spi_stplugin_c_sha256=%s\n' "${spi_source_hash}"
  printf 'stata_spi_stplugin_h_sha256=%s\n' "${spi_header_hash}"
  printf 'cshim_interrupt_test=%s\n' "${cshim_interrupt_test_status}"
  printf 'cshim_error_transport_test=%s\n' "${cshim_error_transport_test_status}"
  printf 'abi_header_compat_test=%s\n' "${abi_header_compat_test_status}"
  printf 'cargo_fmt=%s\n' "${cargo_fmt_status}"
  printf 'cargo_clippy=%s\n' "${cargo_clippy_status}"
  printf 'cargo_test=%s\n' "${cargo_test_status}"
  printf 'qualifier_sha256=%s\n' "$(hash_file "${script_dir}/qualify_macos.sh")"
  printf 'host_macos=%s\n' "${macos_version}"
  printf 'rustc=%s\n' "${rustc_version}"
  printf 'cargo=%s\n' "${cargo_version}"
  printf 'clang=%s\n' "${clang_version}"
  printf 'xcode=%s\n' "${xcode_version}"
  printf 'stata_binary_sha256=%s\n' "$(hash_file "${stata_binary}")"
  printf 'stata_binary_architectures=%s\n' "${stata_architectures}"
  printf 'stata_arm64_environment=%s\n' "${stata_arm64_environment}"
  printf 'stata_x86_64_environment=%s\n' "${stata_x86_64_environment}"
  printf 'target_arm64=aarch64-apple-darwin\n'
  printf 'target_x86_64=x86_64-apple-darwin\n'
  printf 'deployment_floor_arm64=%s\n' "${arm64_floor}"
  printf 'deployment_floor_x86_64=%s\n' "${x86_64_floor}"
  printf 'install_id=%s\n' "${expected_install_id}"
  printf 'required_exports=%s\n' "${required_export_text}"
  printf 'universal_dependencies_arm64=%s\n' "${arm64_dependencies}"
  printf 'universal_dependencies_x86_64=%s\n' "${x86_64_dependencies}"
  printf 'artifact_arm64_sha256=%s\n' "${arm64_hash}"
  printf 'artifact_x86_64_sha256=%s\n' "${x86_64_hash}"
  printf 'artifact_universal_sha256=%s\n' "${universal_hash}"
  printf 'tested_arm64_thin_sha256=%s\n' "${tested_arm64_thin_hash}"
  printf 'tested_x86_64_thin_sha256=%s\n' "${tested_x86_64_thin_hash}"
  printf 'tested_universal_sha256=%s\n' "${tested_universal_arm64_alias_hash}"
  printf 'artifact_arm64_file=%s\n' "${arm64_file_description}"
  printf 'artifact_x86_64_file=%s\n' "${x86_64_file_description}"
  printf 'artifact_universal_file=%s\n' "${universal_file_description}"
  printf 'artifact_arm64_architectures=%s\n' "$(lipo -archs "${staged_arm64}")"
  printf 'artifact_x86_64_architectures=%s\n' "$(lipo -archs "${staged_x86_64}")"
  printf 'artifact_universal_architectures=%s\n' "$(lipo -archs "${staged_universal}")"
  printf 'stata_tested_artifact=%s\n' "${tested_artifact_scope_text}"
  printf 'package_policy=tracked manifest ships portable dispatcher and loader helpers but no plugin binaries or tests; qualifier generates a temporary local macOS artifact manifest containing the tested thin binaries\n'
  printf 'codesign=adhoc-verified\n'
  printf 'arm64_lifecycle=FEVC RUST PLUGIN PASS\n'
  printf 'arm64_diagnostic=FEVC RUST MATA DIAGNOSTIC PASS\n'
  printf 'arm64_shared_atoms=PASS test_rust_mata_shared_atoms.do\n'
  printf 'arm64_public_route=PASS test_rust_public.do\n'
  printf 'arm64_exact_controls=FEVC RUST EXACT CONTROLS PASS\n'
  printf 'arm64_private_generic=FEVC RUST GENERIC JLA PASS\n'
  printf 'arm64_private_planned_v4=FEVC RUST PLANNED V4 PASS\n'
  printf 'arm64_private_planned_compressed=FEVC RUST PLANNED COMPRESSED V4 PASS\n'
  printf 'arm64_public_planned_compressed=FEVC RUST COMPRESSED PUBLIC ROUTES PASS\n'
  printf 'arm64_public_exact=FEVC RUST PUBLIC EXACT PASS\n'
  printf 'arm64_public_generic=FEVC RUST PUBLIC GENERIC PASS\n'
  printf 'arm64_public_stayer_hybrid=PASS test_stayers_hybrid.do\n'
  printf 'arm64_public_component_inference=PASS test_rust_component_inference.do\n'
  printf 'arm64_public_match_component_inference=PASS test_rust_match_component_inference.do\n'
  printf 'arm64_public_individual_inference=PASS test_rust_individual_inference.do\n'
  printf 'arm64_backend_routing=PASS test_backend_routing.do\n'
  printf 'arm64_universal_lifecycle=FEVC RUST PLUGIN PASS\n'
  printf 'arm64_universal_public_route=PASS test_rust_public.do\n'
  printf 'arm64_universal_exact_controls=FEVC RUST EXACT CONTROLS PASS\n'
  printf 'arm64_universal_private_generic=FEVC RUST GENERIC JLA PASS\n'
  printf 'arm64_universal_private_planned_v4=FEVC RUST PLANNED V4 PASS\n'
  printf 'arm64_universal_private_planned_compressed=FEVC RUST PLANNED COMPRESSED V4 PASS\n'
  printf 'arm64_universal_public_planned_compressed=FEVC RUST COMPRESSED PUBLIC ROUTES PASS\n'
  printf 'arm64_universal_public_exact=FEVC RUST PUBLIC EXACT PASS\n'
  printf 'arm64_universal_public_generic=FEVC RUST PUBLIC GENERIC PASS\n'
  printf 'arm64_universal_public_stayer_hybrid=PASS test_stayers_hybrid.do\n'
  printf 'arm64_clean_install=PASS test_rust_public_install.do\n'
  printf 'arm64_canonical_install_unavailable=PASS test_rust_public_install.do\n'
  printf 'arm64_test_status=%s\n' "${arm64_test_status}"
  printf 'rosetta_status=%s\n' "${rosetta_status}"
  printf 'x86_64_test_status=%s\n' "${x86_64_test_status}"
  if [[ "${rosetta_status}" == AVAILABLE ]]; then
    printf 'x86_64_lifecycle=FEVC RUST PLUGIN PASS\n'
    printf 'x86_64_diagnostic=FEVC RUST MATA DIAGNOSTIC PASS\n'
    printf 'x86_64_shared_atoms=PASS test_rust_mata_shared_atoms.do\n'
    printf 'x86_64_public_route=PASS test_rust_public.do\n'
    printf 'x86_64_exact_controls=FEVC RUST EXACT CONTROLS PASS\n'
    printf 'x86_64_private_generic=FEVC RUST GENERIC JLA PASS\n'
    printf 'x86_64_private_planned_v4=FEVC RUST PLANNED V4 PASS\n'
    printf 'x86_64_private_planned_compressed=FEVC RUST PLANNED COMPRESSED V4 PASS\n'
    printf 'x86_64_public_planned_compressed=FEVC RUST COMPRESSED PUBLIC ROUTES PASS\n'
    printf 'x86_64_public_exact=FEVC RUST PUBLIC EXACT PASS\n'
    printf 'x86_64_public_generic=FEVC RUST PUBLIC GENERIC PASS\n'
    printf 'x86_64_public_stayer_hybrid=PASS test_stayers_hybrid.do\n'
    printf 'x86_64_public_component_inference=PASS test_rust_component_inference.do\n'
    printf 'x86_64_public_match_component_inference=PASS test_rust_match_component_inference.do\n'
    printf 'x86_64_public_individual_inference=PASS test_rust_individual_inference.do\n'
    printf 'x86_64_backend_routing=PASS test_backend_routing.do\n'
    printf 'x86_64_universal_lifecycle=FEVC RUST PLUGIN PASS\n'
    printf 'x86_64_universal_public_route=PASS test_rust_public.do\n'
    printf 'x86_64_universal_exact_controls=FEVC RUST EXACT CONTROLS PASS\n'
    printf 'x86_64_universal_private_generic=FEVC RUST GENERIC JLA PASS\n'
    printf 'x86_64_universal_private_planned_v4=FEVC RUST PLANNED V4 PASS\n'
    printf 'x86_64_universal_private_planned_compressed=FEVC RUST PLANNED COMPRESSED V4 PASS\n'
    printf 'x86_64_universal_public_planned_compressed=FEVC RUST COMPRESSED PUBLIC ROUTES PASS\n'
    printf 'x86_64_universal_public_exact=FEVC RUST PUBLIC EXACT PASS\n'
    printf 'x86_64_universal_public_generic=FEVC RUST PUBLIC GENERIC PASS\n'
    printf 'x86_64_universal_public_stayer_hybrid=PASS test_stayers_hybrid.do\n'
    printf 'x86_64_clean_install=PASS test_rust_public_install.do\n'
    printf 'x86_64_canonical_install_unavailable=PASS test_rust_public_install.do\n'
  fi
  printf 'command.fetch=VCKSS_STATA_SPI_DIR=<temporary>/stata-spi rust/stata_backend/fetch_stata_spi.sh\n'
  printf 'command.cshim_interrupt_test=clang -std=c11 -Wall -Wextra -Werror -DSYSTEM=APPLEMAC -I <temporary>/stata-spi -I rust/stata_backend/cshim -I rust/stata_backend/include rust/stata_backend/tests/cshim_interrupt_test.c -o <temporary>/vckss-cshim-interrupt-test; <temporary>/vckss-cshim-interrupt-test\n'
  printf 'command.cshim_error_transport_test=clang -std=c11 -Wall -Wextra -Werror -ffunction-sections -DSYSTEM=APPLEMAC -I <temporary>/stata-spi -I rust/stata_backend/cshim -I rust/stata_backend/include rust/stata_backend/tests/cshim_error_transport_test.c -Wl,-dead_strip -o <temporary>/vckss-cshim-error-transport-test; <temporary>/vckss-cshim-error-transport-test\n'
  printf 'command.abi_header_compat_test=clang -std=c11 -Wall -Wextra -Werror -I rust/stata_backend/include -c rust/stata_backend/tests/abi_header_compat_test.c -o <temporary>/vckss-abi-header-compat.o\n'
  printf 'command.cargo_fmt=<rust-1.85.1-cargo> fmt --manifest-path rust/stata_backend/Cargo.toml --all -- --check\n'
  printf 'command.cargo_clippy=VCKSS_STATA_SPI_DIR=<temporary>/stata-spi CARGO_TARGET_DIR=<temporary>/cargo-target <rust-1.85.1-cargo> clippy --manifest-path rust/stata_backend/Cargo.toml --locked --all-targets -- -D warnings\n'
  printf 'command.cargo_test=VCKSS_STATA_SPI_DIR=<temporary>/stata-spi CARGO_TARGET_DIR=<temporary>/cargo-target <rust-1.85.1-cargo> test --manifest-path rust/stata_backend/Cargo.toml --locked --all-targets\n'
  printf 'command.toolchain=rustup which --toolchain 1.85.1 cargo; rustup which --toolchain 1.85.1 rustc\n'
  printf 'command.toolchain_preflight=PATH=<rust-1.85.1-bin>:$PATH RUSTC=<rust-1.85.1-rustc> RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= CARGO_TARGET_DIR=<temporary>/rust-toolchain-preflight/target <rust-1.85.1-cargo> check --offline --quiet --target %s\n' "${rust_host_target}"
  printf 'command.targets=rustup target add --toolchain 1.85.1 aarch64-apple-darwin x86_64-apple-darwin\n'
  printf 'command.build_arm64=VCKSS_STATA_SPI_DIR=<temporary>/stata-spi CARGO_TARGET_DIR=<temporary>/cargo-target MACOSX_DEPLOYMENT_TARGET=%s PATH=<rust-1.85.1-bin>:$PATH RUSTC=<rust-1.85.1-rustc> RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= <rust-1.85.1-cargo> build --manifest-path rust/stata_backend/Cargo.toml --locked --release --target aarch64-apple-darwin\n' "${arm64_floor}"
  printf 'command.build_x86_64=VCKSS_STATA_SPI_DIR=<temporary>/stata-spi CARGO_TARGET_DIR=<temporary>/cargo-target MACOSX_DEPLOYMENT_TARGET=%s PATH=<rust-1.85.1-bin>:$PATH RUSTC=<rust-1.85.1-rustc> RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= <rust-1.85.1-cargo> build --manifest-path rust/stata_backend/Cargo.toml --locked --release --target x86_64-apple-darwin\n' "${x86_64_floor}"
  printf 'command.sign_thin=codesign --force --sign - --timestamp=none <thin-artifact>\n'
  printf 'command.universal=lipo -create <arm64> <x86_64> -output <universal>; codesign --force --sign - --timestamp=none <universal>\n'
  printf 'command.test_arm64_lifecycle=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_plugin.do <temporary-package>\n'
  printf 'command.test_arm64_diagnostic=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_mata_diagnostic.do <temporary-package>\n'
  printf 'command.test_arm64_shared_atoms=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_mata_shared_atoms.do <temporary-package>\n'
  printf 'command.test_arm64_public_route=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public.do <temporary-package>\n'
  printf 'command.test_arm64_exact_controls=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_exact_controls.do <temporary-thin-package>\n'
  printf 'command.test_arm64_private_generic=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_generic_jla.do <temporary-thin-package>\n'
  printf 'command.test_arm64_private_planned_v4=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_v4.do <temporary-thin-package>\n'
  printf 'command.test_arm64_private_planned_compressed=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_planned_compressed=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed_post.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_full_cmg=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_full_cmg_v2.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_exact=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public_exact.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_generic=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public_generic.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_stayer_hybrid=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_stayers_hybrid.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_component_inference=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_component_inference.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_match_component_inference=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_match_component_inference.do <temporary-thin-package>\n'
  printf 'command.test_arm64_public_individual_inference=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_individual_inference.do <temporary-thin-package>\n'
  printf 'command.test_arm64_backend_routing=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_backend_routing.do <temporary-thin-package>\n'
  printf 'command.test_arm64_universal_public_route=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_exact_controls=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_exact_controls.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_private_generic=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_generic_jla.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_private_planned_v4=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_v4.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_private_planned_compressed=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_public_planned_compressed=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed_post.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_public_full_cmg=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_full_cmg_v2.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_public_exact=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public_exact.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_public_generic=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public_generic.do <temporary-universal-package>\n'
  printf 'command.test_arm64_universal_public_stayer_hybrid=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_stayers_hybrid.do <temporary-universal-package>\n'
  printf 'command.test_arm64_clean_install=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public_install.do <temporary-thin-package> <isolated-plus> qualified <test-root>\n'
  printf 'command.test_arm64_canonical_install_unavailable=arch -arm64 <stata-binary> -b do fevc/tests/stata/test_rust_public_install.do fevc <isolated-plus> unavailable\n'
  if [[ "${rosetta_status}" == AVAILABLE ]]; then
    printf 'command.test_x86_64_lifecycle=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_plugin.do <temporary-package>\n'
    printf 'command.test_x86_64_diagnostic=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_mata_diagnostic.do <temporary-package>\n'
    printf 'command.test_x86_64_shared_atoms=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_mata_shared_atoms.do <temporary-package>\n'
    printf 'command.test_x86_64_public_route=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public.do <temporary-package>\n'
    printf 'command.test_x86_64_exact_controls=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_exact_controls.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_private_generic=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_generic_jla.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_private_planned_v4=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_v4.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_private_planned_compressed=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_planned_compressed=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed_post.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_full_cmg=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_full_cmg_v2.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_exact=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public_exact.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_generic=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public_generic.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_stayer_hybrid=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_stayers_hybrid.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_component_inference=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_component_inference.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_match_component_inference=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_match_component_inference.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_public_individual_inference=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_individual_inference.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_backend_routing=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_backend_routing.do <temporary-thin-package>\n'
    printf 'command.test_x86_64_universal_public_route=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_exact_controls=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_exact_controls.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_private_generic=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_generic_jla.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_private_planned_v4=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_v4.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_private_planned_compressed=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_public_planned_compressed=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_planned_compressed_post.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_public_full_cmg=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_full_cmg_v2.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_public_exact=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public_exact.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_public_generic=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public_generic.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_universal_public_stayer_hybrid=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_stayers_hybrid.do <temporary-universal-package>\n'
    printf 'command.test_x86_64_clean_install=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public_install.do <temporary-thin-package> <isolated-plus> qualified <test-root>\n'
    printf 'command.test_x86_64_canonical_install_unavailable=arch -x86_64 <stata-binary> -b do fevc/tests/stata/test_rust_public_install.do fevc <isolated-plus> unavailable\n'
  fi
  if [[ -n "${artifacts_dir}" ]]; then
    printf 'stata_logs=sanitized command transcripts copied to the requested artifacts directory; startup banners and raw logs deleted on exit\n'
  else
    printf 'stata_logs=temporary-only; deleted on exit; not copied to repository\n'
  fi
} > "${receipt_temporary}"
mv "${receipt_temporary}" "${receipt_path}"
receipt_temporary=

printf 'macOS Rust plugin candidate qualification complete: %s\n' "${classification}"
printf 'Sanitized receipt: %s\n' "${receipt_path}"
printf 'Staged ignored artifacts: %s, %s, %s\n' \
  "${staged_arm64}" "${staged_x86_64}" "${staged_universal}"
