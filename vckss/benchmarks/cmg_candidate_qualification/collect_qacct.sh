#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: collect_qacct.sh RUN_DIR ARRAY_JOB_ID\n' >&2
  exit 198
fi
run_dir=${1%/}
job_id=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* && "$job_id" =~ ^[0-9]+$ ]]
harness=$run_dir/sources/candidate/vckss/benchmarks/cmg_candidate_qualification
module purge
module load python3/3.12.4
python_bin=$(command -v python3)
test "$("$python_bin" --version 2>&1)" = "Python 3.12.4"
for task_id in $(seq 1 72); do
  qacct_path=$run_dir/qacct/$task_id.txt
  validation=$run_dir/validations/$task_id.json
  test ! -e "$qacct_path" && test ! -e "$validation"
  qacct -j "$job_id" -t "$task_id" > "$qacct_path"
  "$python_bin" "$harness/validate_task.py" --run-dir "$run_dir" \
    --task-id "$task_id" --qacct "$qacct_path" --output "$validation"
done
"$python_bin" "$harness/aggregate.py" --run-dir "$run_dir" \
  --output-dir "$run_dir/collection"
printf 'VCKSS_CMG_CANDIDATE_QUALIFICATION_COLLECTION_PASS %s\n' "$job_id"
