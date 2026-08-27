#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: collect_preparation_qacct.sh RUN_DIR JOB_ID\n' >&2
  exit 198
fi
run_dir=${1%/}
job_id=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* && "$job_id" =~ ^[0-9]+$ ]]
harness=$run_dir/sources/candidate/vckss/benchmarks/cmg_candidate_qualification
receipt=$run_dir/receipts/preparation
test ! -e "$receipt/qacct.txt" && test ! -e "$receipt/qacct.pass.json"
qacct -j "$job_id" > "$receipt/qacct.txt"
python3 "$harness/validate_preparation.py" --run-dir "$run_dir" --output "$receipt/qacct.pass.json"
