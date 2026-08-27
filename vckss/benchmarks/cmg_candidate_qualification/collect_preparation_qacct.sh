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
module purge
module load python3/3.12.4
python_bin=$(command -v python3)
test "$("$python_bin" --version 2>&1)" = "Python 3.12.4"
qacct -j "$job_id" > "$receipt/qacct.txt"
"$python_bin" "$harness/validate_preparation.py" --run-dir "$run_dir" \
  --output "$receipt/qacct.pass.json"
