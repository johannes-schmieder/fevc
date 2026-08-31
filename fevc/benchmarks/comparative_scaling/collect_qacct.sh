#!/bin/bash
set -euo pipefail
if test "$#" != 4; then
  printf 'usage: collect_qacct.sh RUN_DIR ATTEMPT_ID ARRAY_JOB_ID TASK_IDS\n' >&2
  exit 198
fi
run_dir=$1
attempt_id=$2
job_id=$3
task_spec=$4
[[ "$run_dir" == /projectnb/welfgr/fevc/runs/* ]]
[[ "$attempt_id" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$job_id" =~ ^[0-9]+$ ]]
attempt=$run_dir/attempts/$attempt_id
test -d "$attempt/tasks" && test -d "$attempt/validations" && \
  test -d "$attempt/qacct"
harness=$run_dir/source/fevc/benchmarks/comparative_scaling
module purge
module load python3/3.12.4
python_bin=$(command -v python3)
test "$("$python_bin" --version 2>&1)" = "Python 3.12.4"
for task_id in $("$python_bin" "$harness/expand_task_ids.py" "$task_spec"); do
  qacct_path=$attempt/qacct/$task_id.txt
  validation=$attempt/validations/$task_id.json
  test ! -e "$qacct_path" && test ! -e "$validation"
  scheduler_task_id=$task_id
  task_map=$run_dir/submissions/$attempt_id.task-map.tsv
  if test -f "$task_map"; then
    test ! -L "$task_map"
    scheduler_task_id=$("$python_bin" "$harness/task_map.py" --map "$task_map" \
      --manifest-task-id "$task_id")
  fi
  qacct -j "$job_id" -t "$scheduler_task_id" > "$qacct_path"
  experiment=$("$python_bin" "$harness/read_task.py" --manifest "$run_dir/input/tasks.tsv" \
    --task-id "$task_id" | sed -n '3p')
  "$python_bin" "$harness/validate_task.py" --job-dir "$attempt/tasks/$experiment" \
    --qacct "$qacct_path" --output "$validation"
done
run_kind=$("$python_bin" -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["run_kind"])' \
  "$run_dir/run_identity.json")
case "$run_kind" in
  pilot-small)
    test "$task_spec" = 7
    "$python_bin" "$harness/validate_pilot.py" --run-dir "$run_dir" \
      --attempt-id "$attempt_id" --task-id 7 \
      --output "$run_dir/receipts/pilot.pass.json"
    ;;
  pilot-worst)
    test "$task_spec" = 298
    "$python_bin" "$harness/validate_pilot.py" --run-dir "$run_dir" \
      --attempt-id "$attempt_id" --task-id 298 \
      --output "$run_dir/receipts/pilot.pass.json"
    ;;
esac
printf 'VCKSS_COMPARATIVE_SCALING_QACCT_COLLECTION_PASS %s %s\n' \
  "$attempt_id" "$job_id"
