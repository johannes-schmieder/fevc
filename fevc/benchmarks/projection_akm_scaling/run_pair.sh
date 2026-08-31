#!/bin/bash -l
set -euo pipefail
if test "$#" != 6; then
  printf 'usage: run_pair.sh RUN_DIR MANIFEST TASK_ID OUTPUT INCLUDE_EXACT GATE\n' >&2
  exit 198
fi
run_dir=${1%/}
manifest=$2
task_id=$3
output=${4%/}
include_exact=$5
gate_mode=$6
case "$run_dir" in /projectnb/welfgr/fevc/runs/*) ;; *) exit 198 ;; esac
[[ "$task_id" =~ ^[1-9][0-9]*$ ]]
[[ "$include_exact" =~ ^[01]$ ]]
[[ "$gate_mode" =~ ^[01]$ ]]
test ! -e "$output"
mkdir -p "$output"

source_dir=$run_dir/source
harness=$source_dir/fevc/benchmarks/projection_akm_scaling
package=$run_dir/artifacts/package
mex_dir=$run_dir/artifacts/mex
matlab_root=/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay
source_commit=$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
test -f "$run_dir/receipts/preparation.pass"
test -f "$package/fevc/fevc_rust_linux_x64.plugin"
test -d "$mex_dir" && test -d "$matlab_root"
(cd "$run_dir/artifacts" && sha256sum -c "$run_dir/receipts/artifact_manifest.sha256" >/dev/null)
test "$(sha256sum "$matlab_root/codes/leave_out_COMPLETE.m" | awk '{print $1}')" = 54b30ebdc51b4c94873e2e3f205bbf865179220e3ad0df0e382922db7c84fc58
test "$(sha256sum "$matlab_root/codes/lincom_KSS.m" | awk '{print $1}')" = 71fb47ce35d26c91dbf97926031359ed0eb31d9916b07c167c577867620ee5a9

module purge
module load python3/3.12.4
task_line=$(python3 "$harness/manifest.py" read "$manifest" "$task_id")
IFS=$'\t' read -r schema stage manifest_id rows workers firms cores replicate probes seed order role_cap maxiter memory_gib task_sha <<< "$task_line"
test "$schema" = FEVC-PROJECTION-AKM-TASK-V2
test "$manifest_id" = "$task_id"
[[ "$rows" =~ ^(6000|480000|1920000|7680000)$ ]]
[[ "$cores" =~ ^(4|16)$ ]]
[[ "$order" =~ ^(rust_matlab|matlab_rust)$ ]]
[[ "$maxiter" =~ ^(20000|40000)$ ]]
[[ "$task_sha" =~ ^[0-9a-f]{64}$ ]]
if test "$gate_mode" = 1; then
  test "$rows" = 6000 && test "$cores" = 4 && test "$replicate" = 0
fi
input=$run_dir/input/input-$rows.csv
input_receipt=$run_dir/input/input-$rows.json
input_sha_file=$run_dir/input/input-$rows.sha256
test -s "$input" && test -s "$input_receipt" && test -s "$input_sha_file"
input_sha=$(tr -d '[:space:]' < "$input_sha_file")
[[ "$input_sha" =~ ^[0-9a-f]{64}$ ]]
test "$(sha256sum "$input" | awk '{print $1}')" = "$input_sha"

scratch=$TMPDIR/fevc-projection-akm-$JOB_ID-${SGE_TASK_ID:-gate}-$rows-$cores-$replicate
case "$scratch" in "$TMPDIR"/*) ;; *) exit 198 ;; esac
test ! -e "$scratch"
mkdir -p "$scratch"
chmod 700 "$scratch"
monitor=$source_dir/fevc/benchmarks/comparative_scaling/monitor_process_tree.py

assigned_cpu_list=$(python3 -c 'import os; print(",".join(map(str,sorted(os.sched_getaffinity(0)))))')
active_cpu_list=$(python3 -c 'import os,sys; n=int(sys.argv[1]); print(",".join(map(str,sorted(os.sched_getaffinity(0))[:n])))' "$cores")
stata_processors=$((cores < 4 ? cores : 4))
test -n "$assigned_cpu_list" && test -n "$active_cpu_list"

write_status() {
  local role=$1 app_rc=$2 monitor_rc=$3 outcome=$4 valid=$5 whole_wall=$6 censor_reason=$7
  local role_dir=$output/$role
  {
    printf 'key\tvalue\n'
    printf 'schema\tVCKSS-PROJECTION-AKM-ROLE-STATUS-V2\n'
    printf 'role\t%s\n' "$role"
    printf 'outcome\t%s\n' "$outcome"
    printf 'application_exit_status\t%s\n' "$app_rc"
    printf 'monitor_exit_status\t%s\n' "$monitor_rc"
    printf 'application_receipt_valid\t%s\n' "$valid"
    printf 'right_censored\t%s\n' "$([[ "$outcome" = RIGHT_CENSORED ]] && printf 1 || printf 0)"
    printf 'censor_reason\t%s\n' "$censor_reason"
    printf 'role_cap_seconds\t%s\n' "$role_cap"
    printf 'whole_wall_seconds\t%s\n' "$whole_wall"
    printf 'active_cores\t%s\n' "$cores"
    printf 'stata_processors\t%s\n' "$stata_processors"
    printf 'cpu_affinity\t%s\n' "$active_cpu_list"
    printf 'source_commit\t%s\n' "$source_commit"
    printf 'input_sha256\t%s\n' "$input_sha"
    printf 'task_sha256\t%s\n' "$task_sha"
  } > "$role_dir/status.tsv"
}

run_stata() {
  local role=$1 role_dir=$output/$1 role_scratch=$scratch/$1
  mkdir -p "$role_dir" "$role_scratch"
  local phase_start=$role_dir/phase.start phase_end=$role_dir/phase.end
  module purge
  module load python3/3.12.4
  module load stata-mp/19
  export STATATMP=$role_scratch OMP_NUM_THREADS=$stata_processors MKL_NUM_THREADS=$stata_processors
  export OPENBLAS_NUM_THREADS=$stata_processors RAYON_NUM_THREADS=$cores
  local started finished root_pid monitor_rc app_rc whole_wall valid outcome censor_reason
  started=$(date -u +%s.%N)
  set +e
  /usr/bin/time -v -o "$role_dir/resources.txt" \
    /usr/bin/timeout --signal=TERM --kill-after=60 "$role_cap" \
    taskset -c "$active_cpu_list" stata-mp -q do "$harness/stata_run.do" \
      "$package" "$input" "$input_sha" "$role_dir/result.csv" \
      "$phase_start" "$phase_end" "$role" "$source_commit" "$rows" \
      "$probes" "$seed" "$cores" "$stata_processors" "$memory_gib" "$role_cap" "$maxiter" \
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
  whole_wall=$(python3 -c 'import sys; print(float(sys.argv[2])-float(sys.argv[1]))' "$started" "$finished")
  printf '%s\n' "$whole_wall" > "$role_dir/whole_wall_seconds.txt"
  valid=0
  if test "$app_rc" = 0 && test "$monitor_rc" = 0 && test -s "$role_dir/result.csv" && \
     grep -Fq "FEVC PROJECTION AKM STATA PASS $role rows=$rows cores=$cores" "$role_dir/application.txt"; then
    valid=1
  fi
  if test "$valid" = 1; then outcome=PASS; censor_reason=NONE
  elif test "$app_rc" = 124; then outcome=RIGHT_CENSORED; censor_reason=ROLE_TIME_CAP
  else outcome=FAIL; censor_reason=NONE
  fi
  write_status "$role" "$app_rc" "$monitor_rc" "$outcome" "$valid" "$whole_wall" "$censor_reason"
}

run_matlab() {
  local role=matlab role_dir=$output/matlab role_scratch=$scratch/matlab
  mkdir -p "$role_dir" "$role_scratch"
  module purge
  module load python3/3.12.4
  module load matlab/2024b
  export VPA_OUTPUT_DIR=$role_dir VPA_SCRATCH_DIR=$role_scratch
  export VPA_INPUT_CSV=$input VPA_INPUT_SHA256=$input_sha
  export VPA_MATLAB_ROOT=$matlab_root VPA_MEX_DIR=$mex_dir
  export VPA_PHASE_START=$role_dir/phase.start VPA_PHASE_END=$role_dir/phase.end
  export VPA_PROCESS_IDENTITY=$role_dir/process_identity.json
  export VPA_SOURCE_COMMIT=$source_commit VPA_ROWS=$rows VPA_PROBES=$probes
  export VPA_SEED=$seed VPA_ACTIVE_CORES=$cores
  export MATLAB_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1
  local started finished root_pid monitor_rc app_rc whole_wall valid outcome censor_reason
  started=$(date -u +%s.%N)
  set +e
  /usr/bin/time -v -o "$role_dir/resources.txt" \
    /usr/bin/timeout --signal=TERM --kill-after=60 "$role_cap" \
    taskset -c "$active_cpu_list" matlab -batch "addpath('$harness'); matlab_run" \
      > "$role_dir/application.txt" 2>&1 &
  root_pid=$!
  python3 "$monitor" --root-pid "$root_pid" --interval-seconds .25 \
    --phase-start "$role_dir/phase.start" --phase-end "$role_dir/phase.end" \
    --expected-pool-workers "$cores" --process-identity "$VPA_PROCESS_IDENTITY" \
    --output "$role_dir/process_tree.json" \
    > "$role_dir/process_tree.application.txt" 2>&1
  monitor_rc=$?
  wait "$root_pid"
  app_rc=$?
  set -e
  finished=$(date -u +%s.%N)
  whole_wall=$(python3 -c 'import sys; print(float(sys.argv[2])-float(sys.argv[1]))' "$started" "$finished")
  printf '%s\n' "$whole_wall" > "$role_dir/whole_wall_seconds.txt"
  valid=0
  if test "$app_rc" = 0 && test "$monitor_rc" = 0 && test -s "$role_dir/result.json" && \
     test -s "$role_dir/application.pass" && \
     grep -Fq "FEVC PROJECTION AKM MATLAB PASS rows=$rows cores=$cores" "$role_dir/application.txt"; then
    valid=1
  fi
  if test "$valid" = 1; then
    outcome=PASS
    censor_reason=NONE
  elif test "$app_rc" = 124; then
    outcome=RIGHT_CENSORED
    censor_reason=ROLE_TIME_CAP
  elif test -s "$role_dir/failure.json" && python3 -c \
    'import json,sys; value=json.load(open(sys.argv[1])); raise SystemExit(0 if value.get("identifier")=="fevc:projectionAkm:Fit" and value.get("message")=="Grounded fit did not converge." else 1)' \
    "$role_dir/failure.json"; then
    outcome=RIGHT_CENSORED
    censor_reason=MATLAB_FIT_NONCONVERGENCE
  else
    outcome=FAIL
    censor_reason=NONE
  fi
  write_status "$role" "$app_rc" "$monitor_rc" "$outcome" "$valid" "$whole_wall" "$censor_reason"
}

module purge
module load stata-mp/19
mkdir -p "$scratch/baseline"
/usr/bin/time -v -o "$output/stata_baseline.resources.txt" \
  taskset -c "$active_cpu_list" stata-mp -q do "$harness/stata_baseline.do" "$cores" "$stata_processors" \
  > "$output/stata_baseline.application.txt" 2>&1
grep -Fq "FEVC PROJECTION AKM STATA BASELINE PASS cores=$cores processors=$stata_processors" "$output/stata_baseline.application.txt"
baseline_rss=$(awk -F ': ' '/Maximum resident set size \(kbytes\)/ {print $2}' "$output/stata_baseline.resources.txt")
[[ "$baseline_rss" =~ ^[1-9][0-9]*$ ]]
printf '%s\n' "$baseline_rss" > "$output/stata_baseline_rss_kib.txt"

if test "$include_exact" = 1; then
  run_stata exact
fi
IFS=_ read -r first_role second_role <<< "$order"
for role in "$first_role" "$second_role"; do
  case "$role" in
    rust) run_stata rust ;;
    matlab) run_matlab ;;
    *) exit 198 ;;
  esac
done

cpu_model=$(lscpu | awk -F: '/Model name/{sub(/^[ \t]+/,"",$2); print $2; exit}')
{
  printf 'key\tvalue\n'
  printf 'schema\tVCKSS-PROJECTION-AKM-PAIR-V2\n'
  printf 'status\tCAPTURED\n'
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'task_sha256\t%s\n' "$task_sha"
  printf 'stage\t%s\n' "$stage"
  printf 'task_id\t%s\n' "$task_id"
  printf 'rows\t%s\n' "$rows"
  printf 'workers\t%s\n' "$workers"
  printf 'firms\t%s\n' "$firms"
  printf 'cores\t%s\n' "$cores"
  printf 'stata_processors\t%s\n' "$stata_processors"
  printf 'replicate\t%s\n' "$replicate"
  printf 'probes\t%s\n' "$probes"
  printf 'seed\t%s\n' "$seed"
  printf 'order\t%s\n' "$order"
  printf 'role_cap_seconds\t%s\n' "$role_cap"
  printf 'maxiter\t%s\n' "$maxiter"
  printf 'memory_gib\t%s\n' "$memory_gib"
  printf 'hostname\t%s\n' "$HOSTNAME"
  printf 'cpu_model\t%s\n' "$cpu_model"
  printf 'scheduler_slots\t%s\n' "$NSLOTS"
  printf 'assigned_cpu_affinity\t%s\n' "$assigned_cpu_list"
  printf 'active_cpu_affinity\t%s\n' "$active_cpu_list"
} > "$output/pair_receipt.tsv"
printf 'FEVC PROJECTION AKM PAIR CAPTURED rows=%s cores=%s replicate=%s order=%s\n' \
  "$rows" "$cores" "$replicate" "$order" > "$output/pair.captured"
