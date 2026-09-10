#!/bin/bash
set -euo pipefail
run_dir=${1:?usage: submit_scc.sh RUN_DIR prepare|smoke|pilot|exact|confirmation}
stage=${2:?}
[[ "$run_dir" == /projectnb/welfgr/vckss/five_way_scaling/runs/* ]]
mkdir -p "$run_dir/logs/$stage" "$run_dir/output/$stage" "$run_dir/submissions"
case "$stage" in
  prepare)
    raw=$(qsub -terse -P welfgr -q econ -l cpu_arch=cascadelake,cpu_type=Gold-6242 \
      -v FW_RUN_DIR="$run_dir" -o "$run_dir/logs/prepare" "$run_dir/code/prepare.sge") ;;
  smoke|exact)
    raw=$(qsub -terse -P welfgr -q econ -pe omp 4 -l h_rt=04:00:00,mem_per_core=8G,cpu_arch=cascadelake,cpu_type=Gold-6242 \
      -t 1 -v FW_RUN_DIR="$run_dir",FW_MANIFEST="$run_dir/input/$stage.tsv",FW_STAGE="$stage" \
      -o "$run_dir/logs/$stage" "$run_dir/code/run_task.sge") ;;
  pilot)
    raw=$(qsub -terse -P welfgr -q econ -pe omp32 32 -l h_rt=12:00:00,mem_per_core=8G,cpu_arch=cascadelake,cpu_type=Gold-6242 \
      -t 1 -v FW_RUN_DIR="$run_dir",FW_MANIFEST="$run_dir/input/pilot.tsv",FW_STAGE=pilot \
      -o "$run_dir/logs/pilot" "$run_dir/code/run_task.sge") ;;
  confirmation)
    test "$(awk 'END{print NR-1}' "$run_dir/input/confirmation.tsv")" = 45
    raw=$(qsub -terse -P welfgr -q econ -pe omp32 32 -l h_rt=12:00:00,mem_per_core=8G,cpu_arch=cascadelake,cpu_type=Gold-6242 \
      -t 1-45 -v FW_RUN_DIR="$run_dir",FW_MANIFEST="$run_dir/input/confirmation.tsv",FW_STAGE=confirmation \
      -o "$run_dir/logs/confirmation" "$run_dir/code/run_task.sge") ;;
  *) exit 198 ;;
esac
job=${raw%%.*}; [[ "$job" =~ ^[0-9]+$ ]]
printf '%s\n' "$job" | tee "$run_dir/submissions/$stage.job_id"
