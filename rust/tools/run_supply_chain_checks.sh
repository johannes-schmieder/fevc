#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf 'usage: run_supply_chain_checks.sh --receipt PATH --sbom-dir DIRECTORY\n' >&2
}

receipt=
sbom_dir=
while (( $# )); do
  case $1 in
    --receipt)
      (( $# >= 2 )) || { usage; exit 198; }
      receipt=$2
      shift 2
      ;;
    --sbom-dir)
      (( $# >= 2 )) || { usage; exit 198; }
      sbom_dir=$2
      shift 2
      ;;
    *) usage; exit 198 ;;
  esac
done
[[ -n "${receipt}" && -n "${sbom_dir}" ]] || { usage; exit 198; }
[[ ! -e "${receipt}" ]] || {
  printf 'refusing to overwrite supply-chain receipt: %s\n' "${receipt}" >&2
  exit 198
}
[[ ! -e "${sbom_dir}" ]] || {
  printf 'refusing to overwrite SBOM directory: %s\n' "${sbom_dir}" >&2
  exit 198
}
[[ -d $(dirname -- "${receipt}") && -d $(dirname -- "${sbom_dir}") ]] || {
  printf 'receipt and SBOM parent directories must exist\n' >&2
  exit 198
}

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "${script_dir}/../.." && pwd -P)
validator=${script_dir}/validate_supply_chain.py
python=${repo_root}/.venv/bin/python
[[ -x "${python}" && -f "${validator}" ]]
[[ $(git -C "${repo_root}" symbolic-ref --short HEAD) == main ]] || {
  printf 'supply-chain qualification requires main\n' >&2
  exit 198
}
[[ -z $(git -C "${repo_root}" status --porcelain --untracked-files=all) ]] || {
  printf 'supply-chain qualification requires a clean committed checkout\n' >&2
  exit 198
}
source_commit=$(git -C "${repo_root}" rev-parse HEAD)
source_epoch=$(git -C "${repo_root}" show -s --format=%ct HEAD)
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ && "${source_epoch}" =~ ^[0-9]+$ ]]

security_toolchain=${VCKSS_SECURITY_TOOLCHAIN:-1.97.1}
security_cargo=$(rustup which --toolchain "${security_toolchain}" cargo)
security_toolchain_bin=$(dirname -- "${security_cargo}")
cargo_audit=${VCKSS_CARGO_AUDIT:-$(command -v cargo-audit || true)}
cargo_cyclonedx=${VCKSS_CARGO_CYCLONEDX:-$(command -v cargo-cyclonedx || true)}
[[ -n "${cargo_audit}" && -x "${cargo_audit}" ]] || {
  printf 'cargo-audit 0.22.2 is required\n' >&2
  exit 198
}
[[ -n "${cargo_cyclonedx}" && -x "${cargo_cyclonedx}" ]] || {
  printf 'cargo-cyclonedx 0.5.9 is required\n' >&2
  exit 198
}
audit_version=$("${cargo_audit}" --version)
[[ "${audit_version}" == 'cargo-audit 0.22.2' ]] || {
  printf 'expected cargo-audit 0.22.2, found: %s\n' "${audit_version}" >&2
  exit 198
}
cyclonedx_bin=$(dirname -- "${cargo_cyclonedx}")
cyclonedx_version=$(env \
  PATH="${cyclonedx_bin}:${security_toolchain_bin}:/usr/bin:/bin" \
  "${security_cargo}" cyclonedx --version)
[[ "${cyclonedx_version}" == 'cargo-cyclonedx-cyclonedx 0.5.9' ]] || {
  printf 'expected cargo-cyclonedx 0.5.9, found: %s\n' "${cyclonedx_version}" >&2
  exit 198
}

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/vckss-supply-chain.XXXXXX")
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
mkdir "${temporary_root}/audits"
git -C "${repo_root}" archive "${source_commit}" rust | \
  tar -x -C "${temporary_root}"
archive_rust=${temporary_root}/rust

audit_db=${temporary_root}/advisory-db
"${cargo_audit}" audit --color never --deny warnings \
  --db "${audit_db}" --file "${archive_rust}/Cargo.lock" --json \
  > "${temporary_root}/audits/workspace.json"
"${cargo_audit}" audit --color never --deny warnings --no-fetch \
  --db "${audit_db}" --file "${archive_rust}/stata_backend/Cargo.lock" --json \
  > "${temporary_root}/audits/stata_backend.json"
"${cargo_audit}" audit --color never --deny warnings --no-fetch \
  --db "${audit_db}" --file "${archive_rust}/fuzz/Cargo.lock" --json \
  > "${temporary_root}/audits/fuzz.json"

cyclonedx() {
  env \
    PATH="${cyclonedx_bin}:${security_toolchain_bin}:/usr/bin:/bin" \
    SOURCE_DATE_EPOCH="${source_epoch}" \
    RUSTC_WRAPPER= \
    RUSTC_WORKSPACE_WRAPPER= \
    "${security_cargo}" cyclonedx \
      --manifest-path "$1" --format json --spec-version 1.5 --license-strict
}
cyclonedx "${archive_rust}/Cargo.toml"
cyclonedx "${archive_rust}/stata_backend/Cargo.toml"
cyclonedx "${archive_rust}/fuzz/Cargo.toml"

normalized=${temporary_root}/normalized
validation_output=$("${python}" "${validator}" \
  --sbom "${archive_rust}/crates/vckss-core/vckss-core.cdx.json" \
  --sbom "${archive_rust}/crates/vckss-plugin/vckss-plugin.cdx.json" \
  --sbom "${archive_rust}/stata_backend/vckss-stata.cdx.json" \
  --sbom "${archive_rust}/fuzz/vckss-fuzz.cdx.json" \
  --audit "workspace=${temporary_root}/audits/workspace.json" \
  --audit "stata_backend=${temporary_root}/audits/stata_backend.json" \
  --audit "fuzz=${temporary_root}/audits/fuzz.json" \
  --lock "workspace=${archive_rust}/Cargo.lock" \
  --lock "stata_backend=${archive_rust}/stata_backend/Cargo.lock" \
  --lock "fuzz=${archive_rust}/fuzz/Cargo.lock" \
  --archive-root "${temporary_root}" \
  --source-commit "${source_commit}" \
  --source-epoch "${source_epoch}" \
  --output-dir "${normalized}")
sbom_count=$(sed -nE 's/^sbom_count=([0-9]+)$/\1/p' <<< "${validation_output}")
component_records=$(sed -nE \
  's/^component_records=([0-9]+)$/\1/p' <<< "${validation_output}")
advisory_count=$(sed -nE 's/^advisory_count=([0-9]+)$/\1/p' <<< "${validation_output}")
database_commit=$(sed -nE \
  's/^rustsec_database_commit=([0-9a-f]{40})$/\1/p' <<< "${validation_output}")
manifest_sha256=$(sed -nE \
  's/^manifest_sha256=([0-9a-f]{64})$/\1/p' <<< "${validation_output}")
license_values=$(sed -nE 's/^license_values=(.*)$/\1/p' <<< "${validation_output}")
legacy_equivalences=$(sed -nE \
  's/^legacy_license_equivalences=(.*)$/\1/p' <<< "${validation_output}")
[[ "${sbom_count}" == 4 && "${component_records}" =~ ^[0-9]+$ ]]
(( component_records >= 4 && advisory_count > 0 ))
[[ "${database_commit}" =~ ^[0-9a-f]{40}$ ]]
[[ "${manifest_sha256}" =~ ^[0-9a-f]{64}$ ]]

receipt_temporary=$(mktemp "${receipt}.tmp.XXXXXX")
{
  printf 'receipt_schema=VCKSS-SUPPLY-CHAIN-QUALIFICATION-V1\n'
  printf 'status=PASS\n'
  printf 'source_commit=%s\n' "${source_commit}"
  printf 'branch=main\n'
  printf 'worktree=clean\n'
  printf 'security_toolchain=%s\n' "${security_toolchain}"
  printf 'cargo_audit=%s\n' "${audit_version}"
  printf 'cargo_cyclonedx=%s\n' "${cyclonedx_version}"
  printf 'rustsec_database_commit=%s\n' "${database_commit}"
  printf 'rustsec_advisory_count=%s\n' "${advisory_count}"
  printf 'audited_lockfiles=workspace,stata_backend,fuzz\n'
  printf 'vulnerabilities=0\n'
  printf 'warnings=0\n'
  printf 'cyclonedx_specification=1.5\n'
  printf 'sbom_count=%s\n' "${sbom_count}"
  printf 'component_records=%s\n' "${component_records}"
  printf 'license_values=%s\n' "${license_values}"
  printf 'legacy_license_equivalences=%s\n' "${legacy_equivalences}"
  printf 'sbom_manifest_sha256=%s\n' "${manifest_sha256}"
  printf 'raw_logs=not retained; advisory clone, archive, raw audits, and unnormalized SBOMs deleted\n'
  printf 'excluded_claims=human-license-provenance-approval,public-release,Windows\n'
  printf 'completed_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "${receipt_temporary}"
mv "${normalized}" "${sbom_dir}"
mv "${receipt_temporary}" "${receipt}"
receipt_temporary=
printf 'VCKSS SUPPLY CHAIN QUALIFICATION PASS receipt=%s source_commit=%s\n' \
  "${receipt}" "${source_commit}"
