#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: submit_scc.sh RUN_DIR prepare|qualification\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$mode" =~ ^(prepare|qualification)$ ]]
test -f "$run_dir/run_identity.json" && test -d "$run_dir/sources/candidate"
harness=$run_dir/sources/candidate/vckss/benchmarks/cmg_candidate_qualification
candidate=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["candidate_commit"])' "$run_dir/run_identity.json")
comparison=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["comparison_commit"])' "$run_dir/run_identity.json")
memory=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["mem_per_core_gib"])' "$run_dir/run_identity.json")
environment="VCS_RUN_DIR=$run_dir,VCS_CANDIDATE_COMMIT=$candidate,VCS_COMPARISON_COMMIT=$comparison"
submission=$run_dir/submissions/$mode.tsv
test ! -e "$submission"
if test "$mode" = prepare; then
  job_id=$(qsub -terse -v "$environment" -o "$run_dir/logs" "$harness/prepare_artifacts.sge")
  range=NONE
else
  test -f "$run_dir/receipts/preparation/wrapper.pass" && \
    test -f "$run_dir/receipts/preparation/qacct.pass.json"
  job_id=$(qsub -terse -t 1-72 -l "mem_per_core=${memory}G" -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
  range=1-72
fi
{
  printf 'key\tvalue\n'
  printf 'schema\tVCKSS-CMG-CANDIDATE-QUALIFICATION-SUBMISSION-V1\n'
  printf 'mode\t%s\n' "$mode"
  printf 'job_id\t%s\n' "$job_id"
  printf 'task_range\t%s\n' "$range"
  printf 'candidate_commit\t%s\n' "$candidate"
  printf 'comparison_commit\t%s\n' "$comparison"
  printf 'queue\tANY_ELIGIBLE\n'
  printf 'host\tANY_ELIGIBLE\n'
  printf 'cpu_type\tANY_ELIGIBLE\n'
  printf 'exclusive\tFALSE\n'
  printf 'client_task_throttle\tNONE\n'
  printf 'comparison_design\tSEQUENTIAL_PAIRED_IDENTICAL_INPUT_CPU\n'
} > "$submission"
printf 'VCKSS CMG CANDIDATE QUALIFICATION SUBMITTED mode=%s job=%s range=%s\n' "$mode" "$job_id" "$range"
