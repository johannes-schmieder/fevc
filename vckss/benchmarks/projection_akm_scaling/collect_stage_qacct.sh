#!/bin/bash
set -euo pipefail
if test "$#" != 3; then
  printf 'usage: collect_stage_qacct.sh RUN_DIR STAGE JOB_ID\n' >&2
  exit 198
fi
run_dir=${1%/}
stage=$2
job_id=$3
case "$run_dir" in /projectnb/welfgr/vckss/runs/*) ;; *) exit 198 ;; esac
[[ "$job_id" =~ ^[1-9][0-9]*$ ]]
manifest=$run_dir/manifests/$stage.tsv
test -s "$manifest"
task_count=$(awk 'END {print NR-1}' "$manifest")
receipt=$run_dir/receipts/$stage-qacct.txt
test ! -e "$receipt"
temporary=$receipt.tmp.$$
for task_id in $(seq 1 "$task_count"); do
  printf 'TASK=%s\n' "$task_id" >> "$temporary"
  qacct -j "$job_id" -t "$task_id" >> "$temporary"
done
mv "$temporary" "$receipt"
module purge
module load python3/3.12.4
python3 "$run_dir/source/vckss/benchmarks/projection_akm_scaling/finalize_stage.py" \
  "$run_dir" "$stage" "$job_id"
