#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: submit_scc.sh RUN_DIR prepare|pilot-small|pilot-worst|production\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$mode" =~ ^(prepare|pilot-small|pilot-worst|production)$ ]]
test -d "$run_dir/source" && test -f "$run_dir/run_identity.json"
python_bin=$(command -v python3)
source_commit=$($python_bin -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["source_commit"])' \
  "$run_dir/run_identity.json")
bundle_sha=$($python_bin -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["bundle_sha256"])' \
  "$run_dir/run_identity.json")
memory=$($python_bin -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["mem_per_core_gib"])' \
  "$run_dir/run_identity.json")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$memory" =~ ^(4|6)$ ]]
source_dir=$run_dir/source
source_manifest=$run_dir/input/source.files.sha256
task_manifest=$run_dir/input/tasks.tsv
matlab_root=${VCS_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
test -d "$matlab_root"
environment="VCS_RUN_DIR=$run_dir,VCS_SOURCE_DIR=$source_dir,VCS_SOURCE_COMMIT=$source_commit,VCS_BUNDLE_SHA256=$bundle_sha,VCS_SOURCE_MANIFEST=$source_manifest,VCS_TASK_MANIFEST=$task_manifest,VCS_MATLAB_ROOT=$matlab_root"
harness=$source_dir/vckss/benchmarks/comparative_scaling
submission=$run_dir/submissions/$mode.tsv
test ! -e "$submission"
case "$mode" in
  prepare)
    test ! -e "$run_dir/artifacts"
    job_id=$(qsub -terse -v "$environment" -o "$run_dir/logs" \
      "$harness/prepare_artifacts.sge")
    range=NONE
    ;;
  pilot-small)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    job_id=$(qsub -terse -t 7 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=7
    ;;
  pilot-worst)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    job_id=$(qsub -terse -t 298 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=298
    ;;
  production)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    test "$(find "$run_dir/tasks" -mindepth 1 -maxdepth 1 -type d | wc -l)" = 0
    job_id=$(qsub -terse -t 1-300 -tc 2 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=1-300
    ;;
esac
{
  printf 'key\tvalue\n'
  printf 'schema\tVCKSS-COMPARATIVE-SCALING-SUBMISSION-V1\n'
  printf 'mode\t%s\n' "$mode"
  printf 'job_id\t%s\n' "$job_id"
  printf 'task_range\t%s\n' "$range"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'mem_per_core_gib\t%s\n' "$memory"
  printf 'queue\tecon\n'
  printf 'cpu_type\tGold-6242\n'
  printf 'exclusive\tTRUE\n'
} > "$submission"
printf 'VCKSS COMPARATIVE SCALING SUBMITTED: mode=%s job=%s range=%s\n' \
  "$mode" "$job_id" "$range"
