#!/bin/bash
set -euo pipefail
if (( $# < 2 || $# > 4 )); then
  printf 'usage: submit_scc.sh RUN_DIR prepare|pilot-small|pilot-worst|production [ATTEMPT_ID]\n' >&2
  printf '       submit_scc.sh RUN_DIR retry TASK_IDS ATTEMPT_ID\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$mode" =~ ^(prepare|pilot-small|pilot-worst|production|retry)$ ]]
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
run_kind=$($python_bin -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["run_kind"])' \
  "$run_dir/run_identity.json")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$memory" =~ ^[1-9][0-9]*$ ]]
[[ "$run_kind" =~ ^(preparation|pilot-small|pilot-worst|production)$ ]]
source_dir=$run_dir/source
source_manifest=$run_dir/input/source.files.sha256
task_manifest=$run_dir/input/tasks.tsv
matlab_root=${VCS_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
test -d "$matlab_root"
harness=$source_dir/vckss/benchmarks/comparative_scaling
attempt_id=$mode
task_ids=NONE
if test "$mode" = retry; then
  test "$#" = 4
  task_ids=$3
  attempt_id=$4
  [[ "$task_ids" =~ ^[0-9,-]+$ ]]
  test -n "$(python3 "$harness/expand_task_ids.py" "$task_ids")"
else
  (( $# <= 3 ))
  test "$#" = 3 && attempt_id=$3
fi
if test "$mode" = prepare; then
  :
elif test "$mode" = retry; then
  test "$run_kind" = production
else
  test "$mode" = "$run_kind"
fi
[[ "$attempt_id" =~ ^[A-Za-z0-9._-]+$ ]]
environment="VCS_RUN_DIR=$run_dir,VCS_SOURCE_DIR=$source_dir,VCS_SOURCE_COMMIT=$source_commit,VCS_BUNDLE_SHA256=$bundle_sha,VCS_SOURCE_MANIFEST=$source_manifest,VCS_TASK_MANIFEST=$task_manifest,VCS_MATLAB_ROOT=$matlab_root,VCS_ATTEMPT_ID=$attempt_id"
submission=$run_dir/submissions/$attempt_id.tsv
test ! -e "$submission"
test ! -e "$run_dir/attempts/$attempt_id"
case "$mode" in
  prepare)
    test ! -e "$run_dir/artifacts"
    job_id=$(qsub -terse -v "$environment" -o "$run_dir/logs" \
      "$harness/prepare_artifacts.sge")
    range=NONE
    ;;
  pilot-small)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    mkdir -p "$run_dir/attempts/$attempt_id/tasks" \
      "$run_dir/attempts/$attempt_id/validations" \
      "$run_dir/attempts/$attempt_id/qacct"
    job_id=$(qsub -terse -t 7 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=7
    ;;
  pilot-worst)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    mkdir -p "$run_dir/attempts/$attempt_id/tasks" \
      "$run_dir/attempts/$attempt_id/validations" \
      "$run_dir/attempts/$attempt_id/qacct"
    job_id=$(qsub -terse -t 298 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=298
    ;;
  production)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    pilot_small=$($python_bin -c \
      'import json,sys; print(json.load(open(sys.argv[1]))["pilot_small_run_id"])' \
      "$run_dir/run_identity.json")
    pilot_worst=$($python_bin -c \
      'import json,sys; print(json.load(open(sys.argv[1]))["pilot_worst_run_id"])' \
      "$run_dir/run_identity.json")
    "$python_bin" "$harness/verify_pilots.py" \
      --production "$run_dir/run_identity.json" \
      --small "/projectnb/welfgr/vckss/runs/$pilot_small/receipts/pilot.pass.json" \
      --worst "/projectnb/welfgr/vckss/runs/$pilot_worst/receipts/pilot.pass.json" \
      > "$run_dir/receipts/pilot_gate.pass"
    mkdir -p "$run_dir/attempts/$attempt_id/tasks" \
      "$run_dir/attempts/$attempt_id/validations" \
      "$run_dir/attempts/$attempt_id/qacct"
    job_id=$(qsub -terse -t 1-300 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=1-300
    ;;
  retry)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    retry_receipt=$run_dir/receipts/retry_authorizations/$attempt_id.json
    "$python_bin" "$harness/verify_retry.py" --run-dir "$run_dir" \
      --task-ids "$task_ids" --output "$retry_receipt"
    mkdir -p "$run_dir/attempts/$attempt_id/tasks" \
      "$run_dir/attempts/$attempt_id/validations" \
      "$run_dir/attempts/$attempt_id/qacct"
    job_id=$(qsub -terse -t "$task_ids" -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=$task_ids
    ;;
esac
{
  printf 'key\tvalue\n'
  printf 'schema\tVCKSS-COMPARATIVE-SCALING-SUBMISSION-V1\n'
  printf 'mode\t%s\n' "$mode"
  printf 'attempt_id\t%s\n' "$attempt_id"
  printf 'job_id\t%s\n' "$job_id"
  printf 'task_range\t%s\n' "$range"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'run_kind\t%s\n' "$run_kind"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'mem_per_core_gib\t%s\n' "$memory"
  printf 'queue\tANY_ELIGIBLE\n'
  printf 'cpu_type\tANY_ELIGIBLE\n'
  printf 'exclusive\tFALSE\n'
  printf 'array_concurrency\tSCHEDULER_MANAGED\n'
  printf 'client_task_throttle\tNONE\n'
  printf 'comparison_design\tPAIRED_WITHIN_TASK_HOST\n'
} > "$submission"
printf 'VCKSS COMPARATIVE SCALING SUBMITTED: mode=%s job=%s range=%s\n' \
  "$mode" "$job_id" "$range"
