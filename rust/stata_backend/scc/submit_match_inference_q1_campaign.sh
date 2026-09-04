#!/usr/bin/env bash
set -euo pipefail

if (( $# != 1 )); then
  printf 'usage: submit_match_inference_q1_campaign.sh RUN_DIR\n' >&2
  exit 198
fi
run_dir=$1
[[ "${run_dir}" == /projectnb/welfgr/vckss/runs/* ]]
test -d "${run_dir}"
test ! -L "${run_dir}"
source_commit=$(tr -d '[:space:]' < "${run_dir}/source_commit.txt")
bundle_sha256=$(tr -d '[:space:]' < "${run_dir}/bundle.sha256")
manifest_sha256=$(tr -d '[:space:]' < "${run_dir}/manifest.sha256")
[[ "${source_commit}" =~ ^[0-9a-f]{40}$ ]]
[[ "${bundle_sha256}" =~ ^[0-9a-f]{64}$ ]]
[[ "${manifest_sha256}" =~ ^[0-9a-f]{64}$ ]]
manifest=${run_dir}/input/campaign-manifest.json
test "$(sha256sum "${manifest}" | awk '{print $1}')" = "${manifest_sha256}"
module load python3/3.13.8
test "$(python3 -c 'import sys; print(sys.version_info[:2])')" = "(3, 13)"
task_count=$(python3 - "${manifest}" <<'PY'
import json
import sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
assert value["task_count"] == len(value["tasks"])
print(value["task_count"])
PY
)
[[ "${task_count}" =~ ^[1-9][0-9]*$ ]]
for path in build.job_id tasks.job_id aggregate.job_id ledger.tsv; do
  test ! -e "${run_dir}/submissions/${path}"
done

common="FEVC_RUN_DIR=${run_dir},FEVC_SOURCE_COMMIT=${source_commit},FEVC_BUNDLE_SHA256=${bundle_sha256},FEVC_MANIFEST_SHA256=${manifest_sha256}"
build_script=${run_dir}/source/rust/stata_backend/scc/run_match_inference_q1_build.sge
task_script=${run_dir}/source/rust/stata_backend/scc/run_match_inference_q1_task.sge
aggregate_script=${run_dir}/source/rust/stata_backend/scc/run_match_inference_q1_aggregate.sge
test -x "${build_script}"
test -x "${task_script}"
test -x "${aggregate_script}"

build_id=$(qsub -terse -P welfgr -pe omp 1 -l h_rt=00:30:00 \
  -l mem_per_core=4G -j y -o "${run_dir}/logs/build.txt" \
  -v "${common}" "${build_script}")
build_id=${build_id%%.*}
[[ "${build_id}" =~ ^[0-9]+$ ]]
printf '%s\n' "${build_id}" > "${run_dir}/submissions/build.job_id"

tasks_id=$(qsub -terse -P welfgr -pe omp 1 -l h_rt=00:30:00 \
  -l mem_per_core=4G -j y -o "${run_dir}/logs/tasks" \
  -hold_jid "${build_id}" -t "1-${task_count}" -v "${common}" "${task_script}")
tasks_id=${tasks_id%%.*}
[[ "${tasks_id}" =~ ^[0-9]+$ ]]
printf '%s\n' "${tasks_id}" > "${run_dir}/submissions/tasks.job_id"

aggregate_id=$(qsub -terse -P welfgr -pe omp 1 -l h_rt=00:15:00 \
  -l mem_per_core=4G -j y -o "${run_dir}/logs/aggregate.txt" \
  -hold_jid "${tasks_id}" -v "${common}" "${aggregate_script}")
aggregate_id=${aggregate_id%%.*}
[[ "${aggregate_id}" =~ ^[0-9]+$ ]]
printf '%s\n' "${aggregate_id}" > "${run_dir}/submissions/aggregate.job_id"

{
  printf 'stage\tjob_id\tdependency\ttasks\tslots\th_rt\tmem_per_core\n'
  printf 'build\t%s\t\tscalar\t1\t00:30:00\t4G\n' "${build_id}"
  printf 'tasks\t%s\t%s\t1-%s\t1\t00:30:00\t4G\n' "${tasks_id}" "${build_id}" "${task_count}"
  printf 'aggregate\t%s\t%s\tscalar\t1\t00:15:00\t4G\n' "${aggregate_id}" "${tasks_id}"
} > "${run_dir}/submissions/ledger.tsv"
printf 'FEVC_MATCH_Q1_SUBMITTED build=%s tasks=%s aggregate=%s task_count=%s run_dir=%s\n' \
  "${build_id}" "${tasks_id}" "${aggregate_id}" "${task_count}" "${run_dir}"
