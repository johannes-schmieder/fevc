#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only

set -euo pipefail

expected_cmg_commit=dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10
toolchain=${VCKSS_SPIKE_TOOLCHAIN:-1.85.1}
cmg_root=${VCKSS_CMG_ROOT:?set VCKSS_CMG_ROOT to the standalone CMG checkout}
work_root=${VCKSS_SPIKE_WORK_ROOT:?set VCKSS_SPIKE_WORK_ROOT to a dedicated temporary directory}
allow_dirty=${VCKSS_SPIKE_ALLOW_DIRTY:-0}

repo_root=$(git rev-parse --show-toplevel)
repo_commit=$(git -C "${repo_root}" rev-parse HEAD)
cmg_root=$(cd "${cmg_root}" && pwd -P)
cmg_checkout_head=$(git -C "${cmg_root}" rev-parse HEAD)
cmg_commit=${expected_cmg_commit}
cmg_dirty=0

if ! git -C "${cmg_root}" cat-file -e "${cmg_commit}^{commit}"; then
  printf 'full-CMG spike cannot resolve required CMG commit %s\n' \
    "${expected_cmg_commit}" >&2
  exit 2
fi
if [[ -n $(git -C "${cmg_root}" status --porcelain) ]]; then
  cmg_dirty=1
fi
if [[ "${allow_dirty}" != 0 && "${allow_dirty}" != 1 ]]; then
  printf 'VCKSS_SPIKE_ALLOW_DIRTY must equal 0 or 1\n' >&2
  exit 2
fi
repo_dirty=0
if [[ -n $(git -C "${repo_root}" status --porcelain) ]]; then
  repo_dirty=1
fi
if [[ "${repo_dirty}" == 1 && "${allow_dirty}" != 1 ]]; then
  printf 'full-CMG spike requires a clean VCkss checkout; set VCKSS_SPIKE_ALLOW_DIRTY=1 only for unreported development builds\n' >&2
  exit 2
fi

case "${work_root}" in
  /private/tmp/*|/tmp/*) ;;
  *)
    printf 'VCKSS_SPIKE_WORK_ROOT must be below /private/tmp or /tmp\n' >&2
    exit 2
    ;;
esac

cargo_bin=$(rustup which --toolchain "${toolchain}" cargo)
rustc_bin=$(rustup which --toolchain "${toolchain}" rustc)
rustdoc_bin=$(rustup which --toolchain "${toolchain}" rustdoc)
rust_bin=$(dirname -- "${rustc_bin}")
rustc_version=$("${rustc_bin}" --version)
cargo_version=$("${cargo_bin}" --version)

cmg_source=${work_root}/cmg-source
cmg_target=${work_root}/cmg-target
vckss_target=${work_root}/vckss-target
candidate_dir=${work_root}/candidate
receipt=${work_root}/build-receipt.txt
if [[ -e "${cmg_source}" ]]; then
  printf 'archived CMG source path already exists: %s\n' "${cmg_source}" >&2
  exit 2
fi
mkdir -p "${cmg_source}" "${cmg_target}" "${vckss_target}" "${candidate_dir}"
git -C "${cmg_root}" archive "${cmg_commit}" | tar -x -C "${cmg_source}"
cp "${repo_root}/rust/full_cmg_spike/cmg_fused.rs" \
  "${cmg_source}/src/vckss_fused.rs"
{
  printf '\n#[cfg(feature = "parallel")]\n'
  printf 'mod vckss_fused;\n'
  printf '#[cfg(feature = "parallel")]\n'
  printf 'pub use vckss_fused::{VckssFusedPcgBatchResult, VckssFusedPcgColumnReport, VckssFusedPcgSolver, VckssFusedPcgWorkspace};\n'
} >>"${cmg_source}/src/lib.rs"

env PATH="${rust_bin}:${PATH}" RUSTC="${rustc_bin}" RUSTDOC="${rustdoc_bin}" \
  CARGO_TARGET_DIR="${cmg_target}" \
  "${cargo_bin}" build --manifest-path "${cmg_source}/Cargo.toml" \
  --release --features parallel --lib --locked --offline

cmg_rlib_count=$(find "${cmg_target}/release/deps" -maxdepth 1 \
  -type f -name 'libcmg-*.rlib' -print | wc -l | tr -d ' ')
if [[ "${cmg_rlib_count}" -ne 1 ]]; then
  printf 'expected exactly one standalone CMG rlib; found %s\n' \
    "${cmg_rlib_count}" >&2
  exit 2
fi
cmg_rlib=$(find "${cmg_target}/release/deps" -maxdepth 1 \
  -type f -name 'libcmg-*.rlib' -print)

spike_rustflags="--extern cmg_full=${cmg_rlib} -L dependency=${cmg_target}/release/deps"
env PATH="${rust_bin}:${PATH}" RUSTC="${rustc_bin}" RUSTDOC="${rustdoc_bin}" \
  CARGO_TARGET_DIR="${vckss_target}" RUSTFLAGS="${spike_rustflags}" \
  "${cargo_bin}" build --manifest-path "${repo_root}/rust/stata_backend/Cargo.toml" \
  --release --features cmg-full-spike --locked --offline

candidate=${candidate_dir}/vckss_rust_macos_arm64.plugin
cp "${vckss_target}/release/libvckss_stata.dylib" "${candidate}"
codesign --force --sign - --timestamp=none "${candidate}"
codesign --verify --strict "${candidate}"

plugin_sha256=$(shasum -a 256 "${candidate}" | awk '{print $1}')
cmg_rlib_sha256=$(shasum -a 256 "${cmg_rlib}" | awk '{print $1}')
fused_source_sha256=$(shasum -a 256 "${repo_root}/rust/full_cmg_spike/cmg_fused.rs" | awk '{print $1}')
{
  printf 'schema=CMG_FULL_SPIKE_BUILD_V1\n'
  printf 'vckss_commit=%s\n' "${repo_commit}"
  printf 'vckss_dirty=%s\n' "${repo_dirty}"
  printf 'cmg_commit=%s\n' "${cmg_commit}"
  printf 'cmg_checkout_head=%s\n' "${cmg_checkout_head}"
  printf 'cmg_worktree_dirty_ignored=%s\n' "${cmg_dirty}"
  printf 'toolchain=%s\n' "${toolchain}"
  printf 'rustc=%s\n' "${rustc_version}"
  printf 'cargo=%s\n' "${cargo_version}"
  printf 'target_arch=%s\n' "$(uname -m)"
  printf 'cmg_rlib_sha256=%s\n' "${cmg_rlib_sha256}"
  printf 'fused_source_sha256=%s\n' "${fused_source_sha256}"
  printf 'plugin_sha256=%s\n' "${plugin_sha256}"
  printf 'plugin=%s\n' "${candidate}"
} >"${receipt}"

printf 'CMG_FULL_SPIKE_BUILD_V1_PASS\n'
printf 'candidate=%s\n' "${candidate}"
printf 'receipt=%s\n' "${receipt}"
