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
python_bin=$(command -v python3)
candidate=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["candidate_commit"])' "$run_dir/run_identity.json")
comparison=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["comparison_commit"])' "$run_dir/run_identity.json")
memory=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["mem_per_core_gib"])' "$run_dir/run_identity.json")
environment="VCS_RUN_DIR=$run_dir,VCS_CANDIDATE_COMMIT=$candidate,VCS_COMPARISON_COMMIT=$comparison"
submission=$run_dir/submissions/$mode.tsv
qstat_receipt=$run_dir/submissions/$mode.effective-qstat.txt
effective_receipt=$run_dir/submissions/$mode.effective-sge.json
test ! -e "$submission"
test ! -e "$qstat_receipt" && test ! -e "$effective_receipt"
job_id=
released=FALSE
cleanup_held_job() {
  if test "$released" != TRUE && test -n "$job_id"; then
    qdel "$job_id" >/dev/null 2>&1 || true
  fi
}
trap cleanup_held_job EXIT
if test "$mode" = prepare; then
  job_id=$(qsub -terse -h -v "$environment" -o "$run_dir/logs" "$harness/prepare_artifacts.sge")
  range=NONE
  slots=4
  binding=()
else
  test -f "$run_dir/receipts/preparation/wrapper.pass" && \
    test -f "$run_dir/receipts/preparation/qacct.pass.json"
  job_id=$(qsub -terse -h -t 1-72 -l "mem_per_core=${memory}G" -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
  range=1-72
  slots=16
  binding=(--require-binding)
fi
qstat -j "$job_id" > "$qstat_receipt"
"$python_bin" "$harness/verify_sge_submission.py" \
  --qstat "$qstat_receipt" --output "$effective_receipt" \
  --expected-slots "$slots" "${binding[@]}"
qrls "$job_id"
released=TRUE
trap - EXIT
{
  printf 'key\tvalue\n'
  printf 'schema\tVCKSS-CMG-CANDIDATE-QUALIFICATION-SUBMISSION-V2\n'
  printf 'mode\t%s\n' "$mode"
  printf 'job_id\t%s\n' "$job_id"
  printf 'task_range\t%s\n' "$range"
  printf 'candidate_commit\t%s\n' "$candidate"
  printf 'comparison_commit\t%s\n' "$comparison"
  printf 'queue\tANY_ELIGIBLE\n'
  printf 'host\tANY_ELIGIBLE\n'
  printf 'cpu_type\tANY_ELIGIBLE\n'
  printf 'exclusive\tFALSE\n'
  printf 'buyin_requested_by_harness\tFALSE\n'
  printf 'soft_buyin_injection\tSCC_GLOBAL_JSV_MANDATORY\n'
  printf 'effective_sge_receipt\t%s\n' "$effective_receipt"
  printf 'submission_control\tHOLD_VALIDATE_RELEASE\n'
  printf 'client_task_throttle\tNONE\n'
  printf 'comparison_design\tSEQUENTIAL_PAIRED_IDENTICAL_INPUT_CPU\n'
} > "$submission"
printf 'VCKSS CMG CANDIDATE QUALIFICATION SUBMITTED mode=%s job=%s range=%s\n' "$mode" "$job_id" "$range"
