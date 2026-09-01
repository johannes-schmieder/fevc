#!/bin/bash
set -euo pipefail
if test "$#" != 3; then
  printf 'usage: collect_qacct.sh RUN_DIR ATTEMPT_ID ARRAY_JOB_ID\n' >&2
  exit 198
fi
run_dir=${1%/}
attempt_id=$2
job_id=$3
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$attempt_id" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$job_id" =~ ^[0-9]+$ ]]
attempt=$run_dir/attempts/$attempt_id
test -d "$attempt/tasks" && test -d "$attempt/validations" && \
  test -d "$attempt/qacct" && test -d "$attempt/bundles"
harness=$run_dir/source/fevc/benchmarks/fevc_matlab_2026
module purge
module load python3/3.12.4
python_bin=$(command -v python3)
test "$("$python_bin" --version 2>&1)" = "Python 3.12.4"
inventory=$run_dir/receipts/$attempt_id.generation.json
"$python_bin" "$harness/collect_generation.py" --run-dir "$run_dir" \
  --attempt-id "$attempt_id" --job-id "$job_id" --output "$inventory"
run_kind=$("$python_bin" -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["run_kind"])' \
  "$run_dir/run_identity.json")
case "$run_kind" in
  pilot-small|pilot-worst)
    "$python_bin" "$harness/validate_pilot.py" --run-dir "$run_dir" \
      --attempt-id "$attempt_id" --output "$run_dir/receipts/pilot.pass.json"
    ;;
esac
printf 'FEVC_MATLAB_2026_QACCT_COLLECTION_PASS attempt=%s job=%s\n' \
  "$attempt_id" "$job_id"
