#!/usr/bin/env bash
set -euo pipefail

if (( $# != 3 )); then
  printf 'usage: deploy_inference_repair_campaign.sh SOURCE_ROOT RUN_ID MANIFEST\n' >&2
  exit 198
fi
source_root=$(CDPATH= cd -- "$1" && pwd -P)
run_id=$2
manifest=$(CDPATH= cd -- "$(dirname "$3")" && pwd -P)/$(basename "$3")
[[ "${run_id}" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]] || {
  printf 'invalid sortable run ID\n' >&2
  exit 198
}
[[ $(git -C "${source_root}" symbolic-ref --short HEAD) == main ]]
[[ -z $(git -C "${source_root}" status --porcelain --untracked-files=all) ]] || {
  printf 'SCC campaign deployment requires a clean committed checkout\n' >&2
  exit 198
}
test -f "${manifest}"
source_commit=$(git -C "${source_root}" rev-parse HEAD)
manifest_commit=$("${source_root}/.venv/bin/python" - "${manifest}" <<'PY'
import json
import sys
print(json.load(open(sys.argv[1], encoding="utf-8"))["source"]["commit"])
PY
)
test "${manifest_commit}" = "${source_commit}"
manifest_sha256=$(shasum -a 256 "${manifest}" | awk '{print $1}')

temporary=$(mktemp -d "${TMPDIR:-/tmp}/fevc-inference-repair-bundle.XXXXXX")
trap 'rm -rf "${temporary}"' EXIT
archive=${temporary}/source.tar.gz
builder=${source_root}/rust/stata_backend/scc/build_linux_bundle.py
builder_output=$("${source_root}/.venv/bin/python" "${builder}" \
  --root "${source_root}" --commit "${source_commit}" --output "${archive}")
bundle_sha256=$(sed -nE \
  's/^.*bundle_sha256=([0-9a-f]{64}).*$/\1/p' <<< "${builder_output}")
[[ "${bundle_sha256}" =~ ^[0-9a-f]{64}$ ]]

remote_root=/projectnb/welfgr/vckss
run_dir=${remote_root}/runs/${run_id}
ssh scc bash -s -- "${run_dir}" "${run_id}" <<'REMOTE_PREPARE'
set -euo pipefail
run_dir=$1
run_id=$2
[[ "${run_dir}" == /projectnb/welfgr/vckss/runs/* ]]
[[ "${run_id}" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]]
test ! -e "${run_dir}"
mkdir -p "${run_dir}/input" "${run_dir}/logs/tasks" \
  "${run_dir}/output/tasks" "${run_dir}/output/aggregate" \
  "${run_dir}/receipts" "${run_dir}/artifacts" \
  "${run_dir}/qacct" "${run_dir}/submissions"
REMOTE_PREPARE

rsync -av --partial "${archive}" "scc:${run_dir}/input/source.tar.gz"
rsync -av --partial "${manifest}" "scc:${run_dir}/input/campaign-manifest.json"
ssh scc bash -s -- "${run_dir}" "${source_commit}" "${bundle_sha256}" \
  "${manifest_sha256}" <<'REMOTE_VERIFY'
set -euo pipefail
run_dir=$1
source_commit=$2
bundle_sha256=$3
manifest_sha256=$4
archive=${run_dir}/input/source.tar.gz
manifest=${run_dir}/input/campaign-manifest.json
test "$(sha256sum "${archive}" | awk '{print $1}')" = "${bundle_sha256}"
test "$(sha256sum "${manifest}" | awk '{print $1}')" = "${manifest_sha256}"
test ! -e "${run_dir}/source"
tar --no-same-owner --no-same-permissions -xzf "${archive}" -C "${run_dir}"
(
  cd "${run_dir}/source"
  sha256sum -c SOURCE_FILES.sha256
)
test "$(tr -d '[:space:]' < "${run_dir}/source/SOURCE_COMMIT.txt")" = "${source_commit}"
test -z "$(find "${run_dir}/source" -type l -print -quit)"
printf '%s\n' "${source_commit}" > "${run_dir}/source_commit.txt"
printf '%s\n' "${bundle_sha256}" > "${run_dir}/bundle.sha256"
printf '%s\n' "${manifest_sha256}" > "${run_dir}/manifest.sha256"
chmod -R a-w "${run_dir}/source" "${run_dir}/input"
printf 'FEVC_INFERENCE_REPAIR_CAMPAIGN_DEPLOYED run_dir=%s source_commit=%s bundle_sha256=%s manifest_sha256=%s\n' \
  "${run_dir}" "${source_commit}" "${bundle_sha256}" "${manifest_sha256}"
REMOTE_VERIFY
