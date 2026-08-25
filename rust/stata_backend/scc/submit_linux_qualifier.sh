#!/usr/bin/env bash
set -euo pipefail

if (( $# != 1 )); then
  printf 'usage: submit_linux_qualifier.sh RUN_DIR\n' >&2
  exit 198
fi
run_dir=$1
[[ "${run_dir}" == /projectnb/welfgr/vckss/runs/* ]] || {
  printf 'RUN_DIR must be an isolated vckss SCC run\n' >&2
  exit 198
}
test -d "${run_dir}"
test ! -L "${run_dir}"
source_commit=$(tr -d '[:space:]' < "${run_dir}/source_commit.txt")
bundle_sha256=$(tr -d '[:space:]' < "${run_dir}/bundle.sha256")
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]]
[[ "${bundle_sha256}" =~ ^[0-9a-f]{64}$ ]]
script=${run_dir}/source/rust/stata_backend/scc/run_linux_qualifier.sge
test -x "${script}"
test ! -e "${run_dir}/submissions/linux-qualifier.job_id"
job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt=02:00:00 \
  -l mem_per_core=4G -j y \
  -o "${run_dir}/logs/linux-qualifier.stdout.txt" \
  -v "VCKSS_RUN_DIR=${run_dir},VCKSS_SOURCE_COMMIT=${source_commit},VCKSS_BUNDLE_SHA256=${bundle_sha256}" \
  "${script}")
job_id=${job_id%%.*}
[[ "${job_id}" =~ ^[0-9]+$ ]] || {
  printf 'invalid qsub response\n' >&2
  exit 1
}
printf '%s\n' "${job_id}" > \
  "${run_dir}/submissions/linux-qualifier.job_id"
printf 'VCKSS_SCC_LINUX_SUBMITTED job_id=%s run_dir=%s source_commit=%s bundle_sha256=%s\n' \
  "${job_id}" "${run_dir}" "${source_commit}" "${bundle_sha256}"
