#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 && $# != 3 && $# != 4 )); then
  printf 'usage: deploy_linux_bundle.sh SOURCE_ROOT RUN_ID [--working-tree | --snapshot PATH]\n' >&2
  exit 198
fi
source_root=$(CDPATH= cd -- "$1" && pwd -P)
run_id=$2
working_tree=0
snapshot_arguments=()
if (( $# == 4 )); then
  [[ "$3" == --snapshot ]] || { printf 'unknown deploy mode\n' >&2; exit 198; }
  working_tree=1
  snapshot_arguments=(--snapshot "$4")
fi
if (( $# == 3 )); then
  [[ "$3" == --working-tree ]] || { printf 'unknown deploy mode\n' >&2; exit 198; }
  working_tree=1
fi
[[ "${run_id}" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]] || {
  printf 'invalid sortable run ID\n' >&2
  exit 198
}
[[ $(git -C "${source_root}" symbolic-ref --short HEAD) == main ]] || {
  printf 'SCC deployment requires main\n' >&2
  exit 198
}
if (( ! working_tree )); then
  [[ -z $(git -C "${source_root}" status --porcelain --untracked-files=all) ]] || {
    printf 'SCC deployment requires a clean committed checkout\n' >&2
    exit 198
  }
fi
source_commit=$(git -C "${source_root}" rev-parse HEAD)
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]]
builder=${source_root}/rust/stata_backend/scc/build_linux_bundle.py
if (( working_tree )); then
  builder=${source_root}/rust/experiments/optimization_parity_20260913/build_dirty_linux_bundle.py
fi
[[ -f "${builder}" ]]

temporary=$(mktemp -d "${TMPDIR:-/tmp}/vckss-linux-bundle.XXXXXX")
trap 'rm -rf "${temporary}"' EXIT
archive=${temporary}/source.tar.gz
builder_output=$("${source_root}/.venv/bin/python" "${builder}" \
  --root "${source_root}" --commit "${source_commit}" \
  --output "${archive}" "${snapshot_arguments[@]}")
bundle_sha256=$(sed -nE \
  's/^.*bundle_sha256=([0-9a-f]{64}).*$/\1/p' <<< "${builder_output}")
[[ "${bundle_sha256}" =~ ^[0-9a-f]{64}$ ]]
[[ $(shasum -a 256 "${archive}" | awk '{print $1}') == "${bundle_sha256}" ]]

remote_root=/projectnb/welfgr/vckss
run_dir=${remote_root}/runs/${run_id}
ssh scc bash -s -- "${remote_root}" "${run_dir}" "${run_id}" <<'REMOTE_PREPARE'
set -euo pipefail
remote_root=$1
run_dir=$2
run_id=$3
test "${remote_root}" = /projectnb/welfgr/vckss
test "${run_dir}" = "${remote_root}/runs/${run_id}"
[[ "${run_id}" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]]
if [[ ! -e "${remote_root}" ]]; then mkdir "${remote_root}"; fi
test -d "${remote_root}"
test ! -L "${remote_root}"
if [[ ! -e "${remote_root}/runs" ]]; then mkdir "${remote_root}/runs"; fi
test -d "${remote_root}/runs"
test ! -L "${remote_root}/runs"
test ! -e "${run_dir}"
mkdir "${run_dir}"
mkdir "${run_dir}/input" "${run_dir}/logs" "${run_dir}/receipts" \
  "${run_dir}/artifacts" "${run_dir}/qacct" "${run_dir}/submissions"
REMOTE_PREPARE

rsync -av --partial "${archive}" "scc:${run_dir}/input/source.tar.gz"

ssh scc bash -s -- "${run_dir}" "${source_commit}" "${bundle_sha256}" "${working_tree}" <<'REMOTE_VERIFY'
set -euo pipefail
run_dir=$1
source_commit=$2
bundle_sha256=$3
working_tree=$4
[[ "${run_dir}" == /projectnb/welfgr/vckss/runs/* ]]
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]]
[[ "${bundle_sha256}" =~ ^[0-9a-f]{64}$ ]]
archive=${run_dir}/input/source.tar.gz
test -f "${archive}"
test ! -L "${archive}"
test "$(sha256sum "${archive}" | awk '{print $1}')" = "${bundle_sha256}"
test ! -e "${run_dir}/source"
tar --no-same-owner --no-same-permissions -xzf "${archive}" -C "${run_dir}"
test -d "${run_dir}/source"
test ! -L "${run_dir}/source"
test "$(tr -d '[:space:]' < "${run_dir}/source/SOURCE_COMMIT.txt")" = \
  "${source_commit}"
if [[ "${working_tree}" == 1 ]]; then
  test "$(tr -d '[:space:]' < "${run_dir}/source/SOURCE_SNAPSHOT_KIND.txt")" = \
    DIRTY_WORKTREE_SNAPSHOT_V1
else
  test ! -e "${run_dir}/source/SOURCE_SNAPSHOT_KIND.txt"
fi
(
  cd "${run_dir}/source"
  sha256sum -c SOURCE_FILES.sha256 >/dev/null
)
test -z "$(find "${run_dir}/source" -type l -print -quit)"
printf '%s\n' "${source_commit}" > "${run_dir}/source_commit.txt"
printf '%s\n' "${bundle_sha256}" > "${run_dir}/bundle.sha256"
printf '%s\n' VCKSS-SCC-LINUX-BUNDLE-V1 > "${run_dir}/bundle.kind"
chmod -R a-w "${run_dir}/source"
printf 'VCKSS_SCC_LINUX_BUNDLE_DEPLOYED run_dir=%s source_commit=%s bundle_sha256=%s\n' \
  "${run_dir}" "${source_commit}" "${bundle_sha256}"
REMOTE_VERIFY
