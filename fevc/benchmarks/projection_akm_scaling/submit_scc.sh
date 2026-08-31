#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: submit_scc.sh RUN_DIR MODE\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
case "$run_dir" in /projectnb/welfgr/fevc/runs/*) ;; *) exit 198 ;; esac
case "$mode" in
  prepare|gate|feasibility-480000|feasibility-1920000|feasibility-7680000|topup-480000|topup-1920000|topup-7680000) ;;
  *) exit 198 ;;
esac
source_commit=$(tr -d '[:space:]' < "$run_dir/source/SOURCE_COMMIT.txt")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
harness=$run_dir/source/fevc/benchmarks/projection_akm_scaling
test -f "$run_dir/receipts/deployment.pass"
submission=$run_dir/submissions/$mode.txt
test ! -e "$submission"

case "$mode" in
  prepare)
    test ! -e "$run_dir/artifacts"
    job=$(qsub -terse -v VPA_RUN_DIR="$run_dir",VPA_SOURCE_COMMIT="$source_commit" \
      -o "$run_dir/logs/prepare.log" "$harness/prepare.sge")
    ;;
  gate)
    test -f "$run_dir/receipts/preparation.pass"
    test ! -e "$run_dir/gate"
    job=$(qsub -terse -v VPA_RUN_DIR="$run_dir" \
      -o "$run_dir/logs/gate.log" "$harness/gate.sge")
    ;;
  *)
    rows=${mode##*-}
    stage_kind=${mode%%-*}
    manifest=$run_dir/manifests/$mode.tsv
    test -s "$manifest"
    if test "$stage_kind" = feasibility; then
      case "$rows" in
        480000) test -f "$run_dir/gate/gate.pass" ;;
        1920000) test -f "$run_dir/receipts/feasibility-480000.pass" ;;
        7680000) test -f "$run_dir/receipts/feasibility-1920000.pass" ;;
      esac
      case "$rows" in
        480000) h_rt=06:30:00 ;;
        1920000) h_rt=21:00:00 ;;
        7680000) h_rt=25:00:00 ;;
      esac
    else
      for required_rows in 480000 1920000 7680000; do
        test -f "$run_dir/receipts/feasibility-$required_rows.pass"
      done
      h_rt=12:30:00
    fi
    case "$rows" in
      480000) mem_per_core=4G ;;
      1920000) mem_per_core=8G ;;
      7680000) mem_per_core=16G ;;
    esac
    task_count=$(awk 'END {print NR-1}' "$manifest")
    [[ "$task_count" =~ ^[1-9][0-9]*$ ]]
    test ! -e "$run_dir/tasks/$mode"
    mkdir -p "$run_dir/tasks/$mode"
    job=$(qsub -terse -t 1-"$task_count" -pe omp 16 -binding linear:16 \
      -l h_rt="$h_rt" -l mem_per_core="$mem_per_core" \
      -v VPA_RUN_DIR="$run_dir",VPA_STAGE="$mode",VPA_MANIFEST="$manifest" \
      -o "$run_dir/logs/$mode-\$TASK_ID.log" "$harness/run_task.sge")
    ;;
esac
printf 'source_commit=%s\nmode=%s\njob=%s\n' "$source_commit" "$mode" "$job" > "$submission"
cat "$submission"
