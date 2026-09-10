#!/bin/bash
set -euo pipefail
run_dir=${1:?}; stage=${2:?}; job=${3:?}; expected=${4:?}
[[ "$run_dir" == /projectnb/welfgr/vckss/five_way_scaling/runs/* && "$job" =~ ^[0-9]+$ ]]
mkdir -p "$run_dir/qacct"
qacct -j "$job" > "$run_dir/qacct/$stage.txt"
module purge
module load python3/3.12.4
python3 "$run_dir/code/validate_qacct.py" --input "$run_dir/qacct/$stage.txt" \
  --expected "$expected" --output "$run_dir/qacct/$stage.json"
printf 'FEVC_FIVE_WAY_QACCT_PASS %s %s\n' "$stage" "$job"
