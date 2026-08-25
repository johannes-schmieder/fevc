#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: run_fuzz_checks.sh --receipt PATH\n' >&2
}

receipt=
while (( $# )); do
  case $1 in
    --receipt)
      (( $# >= 2 )) || { usage; exit 198; }
      receipt=$2
      shift 2
      ;;
    *) usage; exit 198 ;;
  esac
done
[[ -n "${receipt}" ]] || { usage; exit 198; }
[[ ! -e "${receipt}" ]] || {
  printf 'refusing to overwrite fuzz receipt: %s\n' "${receipt}" >&2
  exit 198
}
[[ -d $(dirname -- "${receipt}") ]] || {
  printf 'fuzz receipt parent does not exist: %s\n' "$(dirname -- "${receipt}")" >&2
  exit 198
}

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "${script_dir}/../.." && pwd -P)
fuzz_root=${repo_root}/rust/fuzz
corpus_source=${fuzz_root}/corpus/request_capability
[[ $(git -C "${repo_root}" symbolic-ref --short HEAD) == main ]] || {
  printf 'fuzz qualification requires main\n' >&2
  exit 198
}
[[ -z $(git -C "${repo_root}" status --porcelain --untracked-files=all) ]] || {
  printf 'fuzz qualification requires a clean committed checkout\n' >&2
  exit 198
}
source_commit=$(git -C "${repo_root}" rev-parse HEAD)
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]]

fuzz_seconds=${VCKSS_FUZZ_SECONDS:-60}
[[ "${fuzz_seconds}" =~ ^[0-9]+$ ]] || {
  printf 'VCKSS_FUZZ_SECONDS must be an integer\n' >&2
  exit 198
}
(( fuzz_seconds >= 10 && fuzz_seconds <= 3600 )) || {
  printf 'VCKSS_FUZZ_SECONDS must be between 10 and 3600\n' >&2
  exit 198
}
fuzz_toolchain=${VCKSS_FUZZ_TOOLCHAIN:-nightly-2026-08-23}
fuzz_cargo=$(rustup which --toolchain "${fuzz_toolchain}" cargo)
fuzz_bin_dir_toolchain=$(dirname -- "${fuzz_cargo}")
[[ -x "${fuzz_bin_dir_toolchain}/cargo-clippy" ]] || {
  printf 'Clippy is required for %s; run rustup component add --toolchain %s clippy\n' \
    "${fuzz_toolchain}" "${fuzz_toolchain}" >&2
  exit 198
}
fuzz_bin=${VCKSS_CARGO_FUZZ:-$(command -v cargo-fuzz || true)}
[[ -n "${fuzz_bin}" && -x "${fuzz_bin}" ]] || {
  printf 'cargo-fuzz 0.12.0 is required; install it with cargo install --version 0.12.0 --locked cargo-fuzz\n' >&2
  exit 198
}
fuzz_bin_dir=$(dirname -- "${fuzz_bin}")

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/vckss-fuzz.XXXXXX")
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
mkdir "${temporary_root}/corpus" "${temporary_root}/artifacts"
cp -R "${corpus_source}/." "${temporary_root}/corpus/"
seed_manifest=${temporary_root}/seed-corpus.sha256
while IFS= read -r seed_file; do
  seed_relative=${seed_file#"${corpus_source}/"}
  printf '%s  %s\n' "$(shasum -a 256 "${seed_file}" | awk '{print $1}')" \
    "${seed_relative}"
done < <(find "${corpus_source}" -type f -print | LC_ALL=C sort) > "${seed_manifest}"
seed_corpus_sha256=$(shasum -a 256 "${seed_manifest}" | awk '{print $1}')

export CARGO_TARGET_DIR=${temporary_root}/target
nightly_cargo() {
  env \
    PATH="${fuzz_bin_dir}:${fuzz_bin_dir_toolchain}:/usr/bin:/bin" \
    RUSTUP_TOOLCHAIN="${fuzz_toolchain}" \
    RUSTC_WRAPPER= \
    RUSTC_WORKSPACE_WRAPPER= \
    "${fuzz_cargo}" "$@"
}

fuzz_cargo() {
  (
    cd "${repo_root}/rust"
    nightly_cargo fuzz "$@"
  )
}

fuzz_version=$(fuzz_cargo --version)
[[ "${fuzz_version}" == *'cargo-fuzz 0.12.0'* ]] || {
  printf 'expected cargo-fuzz 0.12.0, found: %s\n' "${fuzz_version}" >&2
  exit 198
}
nightly_cargo clippy --manifest-path "${fuzz_root}/Cargo.toml" \
  --locked --all-targets -- -D warnings
fuzz_clippy_status=PASS
fuzz_cargo build request_capability
fuzz_log=${temporary_root}/fuzz-output.txt
fuzz_cargo run request_capability "${temporary_root}/corpus" \
  -- -max_total_time="${fuzz_seconds}" \
  -timeout=10 \
  -rss_limit_mb=4096 \
  -max_len=256 \
  -seed=20260825 \
  -artifact_prefix="${temporary_root}/artifacts/" \
  -verbosity=0 \
  -print_final_stats=1 2>&1 | tee "${fuzz_log}"
[[ -z $(find "${temporary_root}/artifacts" -type f -print -quit) ]] || {
  printf 'fuzzer produced a crash, timeout, or leak artifact\n' >&2
  exit 1
}
executed_units=$(sed -nE \
  's/^stat::number_of_executed_units:[[:space:]]*([0-9]+)$/\1/p' \
  "${fuzz_log}" | tail -n 1)
average_exec_per_second=$(sed -nE \
  's/^stat::average_exec_per_sec:[[:space:]]*([0-9]+)$/\1/p' \
  "${fuzz_log}" | tail -n 1)
new_units_added=$(sed -nE \
  's/^stat::new_units_added:[[:space:]]*([0-9]+)$/\1/p' \
  "${fuzz_log}" | tail -n 1)
peak_rss_mb=$(sed -nE \
  's/^stat::peak_rss_mb:[[:space:]]*([0-9]+)$/\1/p' \
  "${fuzz_log}" | tail -n 1)
for statistic in executed_units average_exec_per_second new_units_added peak_rss_mb; do
  [[ "${!statistic}" =~ ^[0-9]+$ ]] || {
    printf 'fuzzer omitted required final statistic: %s\n' "${statistic}" >&2
    exit 1
  }
done
(( executed_units > 0 && average_exec_per_second > 0 )) || {
  printf 'fuzzer reported no executed work\n' >&2
  exit 1
}

receipt_temporary=$(mktemp "${receipt}.tmp.XXXXXX")
{
  printf 'receipt_schema=VCKSS-FUZZ-QUALIFICATION-V1\n'
  printf 'status=PASS\n'
  printf 'scope=malformed V1/V2/V3 request-capability ABI headers, semantic tuples, null pointers, and output capacities\n'
  printf 'source_commit=%s\n' "${source_commit}"
  printf 'branch=main\n'
  printf 'worktree=clean\n'
  printf 'host_os=%s\n' "$(uname -s)"
  printf 'host_arch=%s\n' "$(uname -m)"
  printf 'fuzz_toolchain=%s\n' "${fuzz_toolchain}"
  printf 'cargo_fuzz=%s\n' "${fuzz_version}"
  printf 'fuzz_clippy=%s\n' "${fuzz_clippy_status}"
  printf 'target=request_capability\n'
  printf 'max_total_time_seconds=%s\n' "${fuzz_seconds}"
  printf 'max_input_bytes=256\n'
  printf 'seed=20260825\n'
  printf 'executed_units=%s\n' "${executed_units}"
  printf 'average_exec_per_second=%s\n' "${average_exec_per_second}"
  printf 'new_units_added=%s\n' "${new_units_added}"
  printf 'peak_rss_mb=%s\n' "${peak_rss_mb}"
  printf 'artifact_count=0\n'
  printf 'tracked_seed_corpus_sha256=%s\n' "${seed_corpus_sha256}"
  printf 'raw_logs=not retained; temporary corpus growth, build products, and artifacts are deleted\n'
  printf 'excluded_claims=unbounded-fuzzing,numerical-equivalence,dependency-advisories,license-review,SBOM,public-release,Windows\n'
  printf 'completed_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "${receipt_temporary}"
mv "${receipt_temporary}" "${receipt}"
receipt_temporary=
rmdir "${fuzz_root}/artifacts/request_capability" 2>/dev/null || true
rmdir "${fuzz_root}/artifacts" 2>/dev/null || true
printf 'VCKSS FUZZ QUALIFICATION PASS receipt=%s source_commit=%s\n' \
  "${receipt}" "${source_commit}"
