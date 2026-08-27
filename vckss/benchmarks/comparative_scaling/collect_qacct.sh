#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: collect_qacct.sh RUN_DIR ARRAY_JOB_ID\n' >&2
  exit 198
fi
run_dir=$1
job_id=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$job_id" =~ ^[0-9]+$ ]]
test -d "$run_dir/tasks" && test -d "$run_dir/validations"
mkdir -p "$run_dir/qacct"
harness=$run_dir/source/vckss/benchmarks/comparative_scaling
for task_id in $(seq 1 300); do
  qacct_path=$run_dir/qacct/$task_id.txt
  validation=$run_dir/validations/$task_id.json
  test ! -e "$qacct_path" && test ! -e "$validation"
  qacct -j "$job_id" -t "$task_id" > "$qacct_path"
  experiment=$(python3 "$harness/read_task.py" --manifest "$run_dir/input/tasks.tsv" \
    --task-id "$task_id" | sed -n '3p')
  python3 "$harness/validate_task.py" --job-dir "$run_dir/tasks/$experiment" \
    --qacct "$qacct_path" --output "$validation"
done
printf 'VCKSS_COMPARATIVE_SCALING_QACCT_COLLECTION_PASS %s\n' "$job_id"
