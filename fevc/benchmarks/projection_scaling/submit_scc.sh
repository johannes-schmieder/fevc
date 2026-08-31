#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: submit_scc.sh /projectnb/welfgr/fevc/runs/RUN_ID prepare|gate|array\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
case "$run_dir" in /projectnb/welfgr/fevc/runs/*) ;; *) exit 198 ;; esac
[[ "$mode" =~ ^(prepare|gate|array)$ ]]
source_commit=$(tr -d '[:space:]' < "$run_dir/source/SOURCE_COMMIT.txt")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
harness=$run_dir/source/fevc/benchmarks/projection_scaling
test -f "$run_dir/receipts/deployment.pass"
submission=$run_dir/submissions/$mode.txt
test ! -e "$submission"

case "$mode" in
  prepare)
    test ! -e "$run_dir/artifacts"
    job=$(qsub -terse -v VPS_RUN_DIR="$run_dir",VPS_SOURCE_COMMIT="$source_commit" \
      -o "$run_dir/logs/prepare.log" "$harness/prepare.sge")
    ;;
  gate)
    test -f "$run_dir/receipts/preparation.pass"
    test ! -e "$run_dir/gate"
    job=$(qsub -terse -v VPS_RUN_DIR="$run_dir" \
      -o "$run_dir/logs/gate.log" "$harness/gate.sge")
    ;;
  array)
    test -f "$run_dir/gate/gate.pass"
    test ! -e "$run_dir/tasks"
    mkdir -p "$run_dir/tasks"
    job=$(qsub -terse -t 1-9 -v VPS_RUN_DIR="$run_dir" \
      -o "$run_dir/logs/pair-\$TASK_ID.log" "$harness/run_task.sge")
    ;;
esac
printf 'source_commit=%s\nmode=%s\njob=%s\n' \
  "$source_commit" "$mode" "$job" > "$submission"
cat "$submission"
