#!/bin/bash -l
set -euo pipefail
if test "$#" != 5; then
  printf 'usage: run_pair.sh RUN_DIR ROWS REPLICATE OUTPUT_DIR INCLUDE_EXACT\n' >&2
  exit 198
fi
run_dir=${1%/}
rows=$2
replicate=$3
output=${4%/}
include_exact=$5
case "$run_dir" in /projectnb/welfgr/vckss/runs/*) ;; *) exit 198 ;; esac
[[ "$rows" =~ ^(6000|24000|96000)$ ]]
[[ "$replicate" =~ ^[0-3]$ ]]
[[ "$include_exact" =~ ^[01]$ ]]
test "$include_exact" = 0 || { test "$rows" = 6000 && test "$replicate" = 0; }
test ! -e "$output"
mkdir -p "$output"

source_dir=$run_dir/source
harness=$source_dir/vckss/benchmarks/projection_scaling
package=$run_dir/artifacts/package
mex_dir=$run_dir/artifacts/mex
matlab_root=/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay
source_commit=$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
test -f "$run_dir/receipts/preparation.pass"
test -f "$package/vckss/vckss_rust_linux_x64.plugin"
test -d "$mex_dir" && test -d "$matlab_root"
input=$run_dir/input/input-$rows.csv
test -s "$input"
case "$rows" in
  6000) probes=1256 ;;
  24000) probes=1456 ;;
  96000) probes=1656 ;;
esac
seed=$((20260830 + replicate))
scratch=$TMPDIR/vckss-projection-$JOB_ID-${SGE_TASK_ID:-gate}-$rows-$replicate
case "$scratch" in "$TMPDIR"/*) ;; *) exit 198 ;; esac
test ! -e "$scratch"
mkdir -p "$scratch"
chmod 700 "$scratch"

monitor=$source_dir/vckss/benchmarks/comparative_scaling/monitor_process_tree.py
parse_maxrss() {
  awk -F ': ' '/Maximum resident set size \(kbytes\)/ {print $2}' "$1"
}

run_stata() {
  local role=$1 role_dir=$output/$1 role_scratch=$scratch/$1
  mkdir -p "$role_dir" "$role_scratch"
  local phase_start=$role_dir/phase.start phase_end=$role_dir/phase.end
  module purge
  module load python3/3.12.4
  module load stata-mp/19
  export STATATMP=$role_scratch OMP_NUM_THREADS=4 MKL_NUM_THREADS=4
  export RAYON_NUM_THREADS=4
  local started finished root_pid monitor_rc app_rc
  started=$(date -u +%s.%N)
  set +e
  /usr/bin/time -v -o "$role_dir/resources.txt" \
    /usr/bin/timeout --signal=TERM --kill-after=60 20000 \
    taskset -c "$(taskset -pc $$ | sed 's/.*: //')" \
    stata-mp -q do "$harness/stata_run.do" \
      "$package" "$input" "$role_dir/result.csv" "$phase_start" \
      "$phase_end" "$role" "$source_commit" "$rows" "$probes" "$seed" \
      > "$role_dir/application.txt" 2>&1 &
  root_pid=$!
  python3 "$monitor" --root-pid "$root_pid" --interval-seconds .25 \
    --phase-start "$phase_start" --phase-end "$phase_end" \
    --output "$role_dir/process_tree.json" \
    > "$role_dir/process_tree.application.txt" 2>&1
  monitor_rc=$?
  wait "$root_pid"
  app_rc=$?
  set -e
  finished=$(date -u +%s.%N)
  python3 -c 'import sys; print(float(sys.argv[2])-float(sys.argv[1]))' \
    "$started" "$finished" > "$role_dir/whole_wall_seconds.txt"
  test "$app_rc" = 0 && test "$monitor_rc" = 0
  grep -F "VCKSS PROJECTION SCALING STATA PASS $role rows=$rows" \
    "$role_dir/application.txt"
  test -s "$role_dir/result.csv"
}

run_matlab() {
  local role_dir=$output/matlab role_scratch=$scratch/matlab
  mkdir -p "$role_dir" "$role_scratch"
  module purge
  module load python3/3.12.4
  module load matlab/2024b
  export VPS_OUTPUT_DIR=$role_dir VPS_SCRATCH_DIR=$role_scratch
  export VPS_INPUT_CSV=$input VPS_MATLAB_ROOT=$matlab_root VPS_MEX_DIR=$mex_dir
  export VPS_PHASE_START=$role_dir/phase.start VPS_PHASE_END=$role_dir/phase.end
  export VPS_SOURCE_COMMIT=$source_commit VPS_ROWS=$rows VPS_PROBES=$probes VPS_SEED=$seed
  export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
  local started finished root_pid monitor_rc app_rc
  started=$(date -u +%s.%N)
  set +e
  /usr/bin/time -v -o "$role_dir/resources.txt" \
    /usr/bin/timeout --signal=TERM --kill-after=60 20000 \
    taskset -c "$(taskset -pc $$ | sed 's/.*: //')" \
    matlab -batch "addpath('$harness'); matlab_run" \
      > "$role_dir/application.txt" 2>&1 &
  root_pid=$!
  python3 "$monitor" --root-pid "$root_pid" --interval-seconds .25 \
    --phase-start "$role_dir/phase.start" --phase-end "$role_dir/phase.end" \
    --output "$role_dir/process_tree.json" \
    > "$role_dir/process_tree.application.txt" 2>&1
  monitor_rc=$?
  wait "$root_pid"
  app_rc=$?
  set -e
  finished=$(date -u +%s.%N)
  python3 -c 'import sys; print(float(sys.argv[2])-float(sys.argv[1]))' \
    "$started" "$finished" > "$role_dir/whole_wall_seconds.txt"
  test "$app_rc" = 0 && test "$monitor_rc" = 0
  grep -F "VCKSS PROJECTION SCALING MATLAB PASS rows=$rows" \
    "$role_dir/application.txt"
  test -s "$role_dir/result.json"
}

module purge
module load stata-mp/19
mkdir -p "$scratch/baseline"
/usr/bin/time -v -o "$output/stata_baseline.resources.txt" \
  stata-mp -q do "$harness/stata_baseline.do" \
  > "$output/stata_baseline.application.txt" 2>&1
grep -F 'VCKSS PROJECTION SCALING STATA BASELINE PASS' \
  "$output/stata_baseline.application.txt"
baseline_rss=$(parse_maxrss "$output/stata_baseline.resources.txt")
[[ "$baseline_rss" =~ ^[1-9][0-9]*$ ]]
printf '%s\n' "$baseline_rss" > "$output/stata_baseline_rss_kib.txt"

if test "$include_exact" = 1; then
  run_stata exact
fi
size_index=0
test "$rows" = 24000 && size_index=1
test "$rows" = 96000 && size_index=2
if (( (size_index + replicate) % 2 == 0 )); then
  order=rust_matlab
  run_stata rust
  run_matlab
else
  order=matlab_rust
  run_matlab
  run_stata rust
fi
printf '%s\n' "$order" > "$output/order.txt"
printf 'VCKSS PROJECTION SCALING PAIR PASS rows=%s replicate=%s order=%s\n' \
  "$rows" "$replicate" "$order" > "$output/pair.pass"
