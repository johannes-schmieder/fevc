#!/bin/bash -l
# Per-cell runner.  run_task.sge invokes this ten times inside one
# topology-by-repetition allocation, so this file has no SGE directives.
# Each cell launches fresh Rust/Stata and MATLAB processes in balanced order.
set -euo pipefail
for name in VCS_RUN_DIR VCS_SOURCE_DIR VCS_SOURCE_COMMIT VCS_BUNDLE_SHA256 \
  VCS_SOURCE_MANIFEST VCS_TASK_MANIFEST VCS_MATLAB_ROOT TMPDIR JOB_ID \
  VCS_ATTEMPT_ID VCS_CELL_TASK_ID VCS_BUNDLE_TASK_ID VCS_MAX_ITERATIONS \
  HOSTNAME NSLOTS; do
  test -n "${!name:-}" || { printf 'missing %s\n' "$name" >&2; exit 198; }
done
[[ "$VCS_RUN_DIR" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$VCS_SOURCE_COMMIT" =~ ^[0-9a-f]{40}$ ]]
[[ "$VCS_BUNDLE_SHA256" =~ ^[0-9a-f]{64}$ ]]
[[ "$VCS_ATTEMPT_ID" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$VCS_CELL_TASK_ID" =~ ^[0-9]+$ ]] && (( VCS_CELL_TASK_ID >= 1 && VCS_CELL_TASK_ID <= 240 ))
[[ "$VCS_BUNDLE_TASK_ID" =~ ^[0-9]+$ ]] && (( VCS_BUNDLE_TASK_ID >= 1 && VCS_BUNDLE_TASK_ID <= 24 ))
test "$VCS_MAX_ITERATIONS" = 10000
scheduler_task_id=$VCS_BUNDLE_TASK_ID
task_start_epoch=$(date -u +%s.%N)
task_start_utc=$(date -u +%Y-%m-%dT%H:%M:%S.%NZ)
test -d "$VCS_SOURCE_DIR" && test ! -L "$VCS_SOURCE_DIR"
test -f "$VCS_SOURCE_MANIFEST" && test ! -L "$VCS_SOURCE_MANIFEST"
test -f "$VCS_TASK_MANIFEST" && test ! -L "$VCS_TASK_MANIFEST"
test "$(tr -d '[:space:]' < "$VCS_SOURCE_DIR/SOURCE_COMMIT.txt")" = \
  "$VCS_SOURCE_COMMIT"
case "$VCS_MATLAB_ROOT" in
  /projectnb/welfgr/separations/*/LeaveOutTwoWay) ;;
  *) exit 198 ;;
esac

artifacts=$VCS_RUN_DIR/artifacts
preparation=$VCS_RUN_DIR/receipts/preparation
test -f "$preparation/wrapper.pass" && test -f "$preparation/binary_manifest.sha256"
(cd "$artifacts" && sha256sum -c "$preparation/binary_manifest.sha256") \
  > /dev/null
package=$artifacts/package
mex_dir=$artifacts/mex
test -f "$package/fevc/fevc_rust_linux_x64.plugin"
test -d "$mex_dir"

module purge
module load python3/3.12.4
python_bin=$(command -v python3)
python_version=$("$python_bin" --version 2>&1)
test "$python_version" = "Python 3.12.4"
harness=$VCS_SOURCE_DIR/fevc/benchmarks/fevc_matlab_2026
task_map_sha=NONE
task_id=$VCS_CELL_TASK_ID
scratch=$TMPDIR/fevc-comparative-$JOB_ID-$scheduler_task_id-$task_id
case "$scratch" in "$TMPDIR"/*) ;; *) exit 198 ;; esac
test ! -e "$scratch"
mkdir -p "$scratch/input" "$scratch/stata" "$scratch/matlab"
chmod 700 "$scratch" "$scratch"/*

provisional=$scratch/task.tsv
"$python_bin" "$harness/read_task.py" --manifest "$VCS_TASK_MANIFEST" \
  --task-id "$task_id" --output "$provisional"
expected_task_id=$task_id
IFS=$'\t' read -r -a task < <(tail -n 1 "$provisional")
test "${#task[@]}" = 27
task_schema=${task[0]}; task_id=${task[1]}; experiment=${task[2]}
task_commit=${task[3]}; task_bundle=${task[4]}; structure=${task[5]}
connectivity=${task[6]}; degree=${task[7]}; rows=${task[8]}
workers=${task[9]}; firms=${task[10]}; active_cores=${task[11]}
stata_processors=${task[12]}; rust_threads=${task[13]}; matlab_workers=${task[14]}
replicate=${task[15]}; seed=${task[16]}; order=${task[17]}; probes=${task[18]}
requested_slots=${task[19]}; mem_per_core=${task[20]}; command_memory=${task[21]}
hard_wall=${task[22]}; estimator_timeout=${task[23]}
sample_contract=${task[24]}; target_contract=${task[25]}
comparison_contract=${task[26]}
[[ "$task_schema" =~ ^FEVC-MATLAB-2026-(MAIN|SMOKE)-CELL-V1$ ]]
test "$task_id" = "$expected_task_id"
test "$task_commit" = "$VCS_SOURCE_COMMIT"
test "$task_bundle" = "$VCS_BUNDLE_SHA256"
test "$requested_slots" = "$NSLOTS"
test "$stata_processors" -le 4
test "$rust_threads" = "$active_cores"
test "$matlab_workers" = "$active_cores"
[[ "$mem_per_core" =~ ^[1-9][0-9]*$ ]]
[[ "$command_memory" =~ ^[1-9][0-9]*$ ]]
(( command_memory <= requested_slots * mem_per_core ))
[[ "$hard_wall" =~ ^[1-9][0-9]*$ ]]
[[ "$estimator_timeout" =~ ^[1-9][0-9]*$ ]]
(( estimator_timeout <= hard_wall ))
test "$sample_contract" = same_literal_match_rows_v3
test "$target_contract" = uniform_stored_rows_v1
test "$comparison_contract" = paired_same_host_rust_matlab_time_absolute_memory_v1

attempt_root=$VCS_RUN_DIR/attempts/$VCS_ATTEMPT_ID
output=$attempt_root/tasks/$experiment
test -d "$attempt_root/tasks" && test -d "$attempt_root/validations" && \
  test -d "$attempt_root/qacct"
test ! -e "$output"
mkdir -p "$output"
cp "$provisional" "$output/task.tsv"
task_sha=$(sha256sum "$output/task.tsv" | awk '{print $1}')
printf '%s\n' "$task_sha" > "$output/task.sha256"

wrapper_complete=0
failure_stage=initialization
failure() {
  rc=$?
  if (( wrapper_complete == 0 )); then
    printf 'FEVC_MATLAB_2026_CELL_WRAPPER_FAIL %s stage=%s rc=%s\n' \
      "$experiment" "$failure_stage" "$rc" > "$output/wrapper.fail"
  fi
  exit "$rc"
}
trap failure EXIT

failure_stage=cpu_block_claim
full_affinity=$(taskset -pc $$ | sed 's/.*: //')
host_cpu_count=$("$python_bin" -c 'import os; print(len(os.sched_getaffinity(0)))')
[[ "$host_cpu_count" =~ ^[0-9]+$ ]] && (( host_cpu_count >= requested_slots ))
claim_capacity=$((host_cpu_count / requested_slots))
(( claim_capacity >= 1 ))
claim_root=/scratch/vckss-core-claims-$UID
if ! mkdir -m 700 "$claim_root" 2>/dev/null; then
  test -d "$claim_root" && test ! -L "$claim_root"
  test "$(stat -c %u "$claim_root")" = "$UID"
fi
claim_fd=
cpu_block_index=
for ((block_index=0; block_index<claim_capacity; block_index++)); do
  candidate_offset=$((block_index * requested_slots))
  candidate_cpu_list=$("$python_bin" "$harness/cpu_subset.py" \
    --count "$requested_slots" --offset "$candidate_offset")
  claim_name=${candidate_cpu_list//,/_}
  exec {candidate_fd}> "$claim_root/cpus-$claim_name.lock"
  if flock -n "$candidate_fd"; then
    claim_fd=$candidate_fd
    cpu_block_index=$block_index
    cpu_offset=$candidate_offset
    assigned_cpu_list=$candidate_cpu_list
    break
  fi
  exec {candidate_fd}>&-
done
test -n "$claim_fd" && test -n "$cpu_block_index"
target_cpu_list=$("$python_bin" "$harness/cpu_subset.py" \
  --count "$active_cores" --offset "$cpu_offset")
test "$(tr ',' '\n' <<< "$assigned_cpu_list" | wc -l)" = "$requested_slots"
test "$(tr ',' '\n' <<< "$target_cpu_list" | wc -l)" = "$active_cores"

failure_stage=input_generation
module purge
module load stata-mp/19
export STATATMP=$scratch/stata/input
mkdir -p "$STATATMP"
taskset -c "$assigned_cpu_list" stata-mp -q do "$harness/generate_input.do" \
  "$scratch/input/input.csv" "$output/input_receipt.csv" \
  "$structure" "$connectivity" "$rows" "$degree" \
  > "$output/input_generation.application.txt" 2>&1
grep -F "FEVC_MATLAB_2026_INPUT_PASS $structure rows=$rows" \
  "$output/input_generation.application.txt"
input_sha=$(sha256sum "$scratch/input/input.csv" | awk '{print $1}')
[[ "$input_sha" =~ ^[0-9a-f]{64}$ ]]
printf '%s\n' "$input_sha" > "$output/input.sha256"

module purge
module load python3/3.12.4
python_bin=$(command -v python3)
monitor=$harness/monitor_process_tree.py

write_status() {
  local role=$1 app_rc=$2 monitor_rc=$3 timed_out=$4 valid=$5
  local effective_cores=$6 role_cpu_list=$7
  local role_dir=$output/$role
  {
    printf 'key\tvalue\n'
    printf 'schema\tFEVC-MATLAB-2026-ROLE-STATUS-V1\n'
    printf 'role\t%s\n' "$role"
    printf 'application_exit_status\t%s\n' "$app_rc"
    printf 'monitor_exit_status\t%s\n' "$monitor_rc"
    printf 'timed_out\t%s\n' "$timed_out"
    printf 'application_receipt_valid\t%s\n' "$valid"
    printf 'active_cores\t%s\n' "$active_cores"
    printf 'effective_role_cores\t%s\n' "$effective_cores"
    printf 'stata_processors\t%s\n' "$stata_processors"
    printf 'cpu_affinity\t%s\n' "$role_cpu_list"
  } > "$role_dir/status.tsv"
}

run_stata() {
  local role=$1 role_dir=$output/$role role_scratch=$scratch/stata/$role
  local role_cores role_cpu_list python_current=$python_bin
  test "$role" = rust
  role_cores=$rust_threads
  role_cpu_list=$target_cpu_list
  mkdir -p "$role_dir" "$role_scratch"
  local empty_ready=$role_dir/empty.ready data_ready=$role_dir/data.ready
  local empty_ack=$role_dir/empty.sampled data_ack=$role_dir/data.sampled
  local phase_start=$role_dir/phase.start phase_end=$role_dir/phase.end
  module purge
  module load stata-mp/19
  # Keep Python's GCC runtime out of Stata's loader environment.  The absolute
  # Python executable has an rpath for libpython and remains usable by the
  # process-tree monitor after the module purge.
  test "$($python_current --version 2>&1)" = "Python 3.12.4"
  export STATATMP=$role_scratch OMP_NUM_THREADS=$stata_processors
  export MKL_NUM_THREADS=$stata_processors RAYON_NUM_THREADS=$role_cores
  unset VCKSS_PRIVATE_CMG_FULL_V1 VCKSS_PRIVATE_CMG_THREADS \
    VCKSS_PRIVATE_CMG_DIAGNOSTICS VCKSS_PRIVATE_CMG_FAST_PREP_V1 \
    VCKSS_PRIVATE_CMG_RAW_MATCH_V1 || true
  unset VCKSS_BENCHMARK_THREAD_CONTRACT VCKSS_BENCHMARK_RUST_THREADS \
    VCKSS_BENCHMARK_ACTIVE_CORES VCKSS_BENCHMARK_ASSIGNED_SLOTS || true
  if test "$role" = rust; then
    export VCKSS_BENCHMARK_THREAD_CONTRACT=FEVC-BENCHMARK-THREADS-V1
    export VCKSS_BENCHMARK_RUST_THREADS=$rust_threads
    export VCKSS_BENCHMARK_ACTIVE_CORES=$active_cores
    export VCKSS_BENCHMARK_ASSIGNED_SLOTS=$NSLOTS
  fi
  # Stata 19 segfaults before opening a do-file when this campaign's long
  # 22-field argument vector is placed on its command line.  Pass the same
  # validated fields through the process environment and keep argv short.
  export VCS_STATA_PACKAGE_ROOT=$package
  export VCS_STATA_INPUT_CSV=$scratch/input/input.csv
  export VCS_STATA_OUTPUT_CSV=$role_dir/result.csv
  export VCS_STATA_EMPTY_READY=$empty_ready VCS_STATA_DATA_READY=$data_ready
  export VCS_STATA_EMPTY_ACK=$empty_ack VCS_STATA_DATA_ACK=$data_ack
  export VCS_STATA_PHASE_START=$phase_start VCS_STATA_PHASE_END=$phase_end
  export VCS_STATA_ROLE=$role VCS_STATA_SOURCE_COMMIT=$VCS_SOURCE_COMMIT
  export VCS_STATA_TASK_SHA=$task_sha VCS_STATA_INPUT_SHA=$input_sha
  export VCS_STATA_STRUCTURE=$structure VCS_STATA_CONNECTIVITY=$connectivity
  export VCS_STATA_ROWS=$rows VCS_STATA_DEGREE=$degree VCS_STATA_PROBES=$probes
  export VCS_STATA_SEED=$seed VCS_STATA_CORES=$active_cores
  export VCS_STATA_MAXITER=$VCS_MAX_ITERATIONS
  export VCS_STATA_REQUESTED_SLOTS=$requested_slots
  export VCS_STATA_PROCESSORS=$stata_processors VCS_STATA_RUST_THREADS=$rust_threads
  export VCS_STATA_MEMORY=$command_memory VCS_STATA_TIMEOUT=$estimator_timeout
  set +e
  /usr/bin/time -v -o "$role_dir/resources.txt" \
    /usr/bin/timeout --signal=TERM --kill-after=60 "$estimator_timeout" \
    taskset -c "$role_cpu_list" stata-mp -q do "$harness/stata_run.do" \
      > "$role_dir/application.txt" 2>&1 &
  local root_pid=$!
  "$python_current" "$monitor" --root-pid "$root_pid" \
    --interval-seconds 0.10 --empty-ready "$empty_ready" --data-ready "$data_ready" \
    --empty-ack "$empty_ack" --data-ack "$data_ack" \
    --phase-start "$phase_start" --phase-end "$phase_end" \
    --output "$role_dir/process_tree.json" \
    > "$role_dir/process_tree.monitor.txt" 2>&1
  local monitor_rc=$?
  wait "$root_pid"
  local app_rc=$?
  unset VCS_STATA_PACKAGE_ROOT VCS_STATA_INPUT_CSV VCS_STATA_OUTPUT_CSV \
    VCS_STATA_EMPTY_READY VCS_STATA_DATA_READY VCS_STATA_PHASE_START \
    VCS_STATA_EMPTY_ACK VCS_STATA_DATA_ACK VCS_STATA_REQUESTED_SLOTS \
    VCS_STATA_PHASE_END VCS_STATA_ROLE VCS_STATA_SOURCE_COMMIT \
    VCS_STATA_TASK_SHA VCS_STATA_INPUT_SHA VCS_STATA_STRUCTURE \
    VCS_STATA_CONNECTIVITY VCS_STATA_ROWS VCS_STATA_DEGREE \
    VCS_STATA_PROBES VCS_STATA_SEED VCS_STATA_CORES VCS_STATA_PROCESSORS \
    VCS_STATA_RUST_THREADS VCS_STATA_MEMORY VCS_STATA_TIMEOUT \
    VCS_STATA_MAXITER
  set -e
  local timed_out=0 valid=0
  test "$app_rc" = 124 && timed_out=1
  if test "$app_rc" = 0 && test "$monitor_rc" = 0 && \
     test -s "$role_dir/result.csv" && \
     grep -Fq "FEVC_MATLAB_2026_RUST_PASS $VCS_SOURCE_COMMIT" \
       "$role_dir/application.txt"; then
    valid=1
  fi
  write_status "$role" "$app_rc" "$monitor_rc" "$timed_out" "$valid" \
    "$role_cores" "$role_cpu_list"
  if (( app_rc != 0 || monitor_rc != 0 || timed_out != 0 || valid != 1 )); then
    return 1
  fi
}

run_matlab() {
  local role=matlab role_dir=$output/matlab role_scratch=$scratch/matlab/run
  local python_current=$python_bin
  mkdir -p "$role_dir" "$role_scratch"
  local empty_ready=$role_dir/empty.ready data_ready=$role_dir/data.ready
  local empty_ack=$role_dir/empty.sampled data_ack=$role_dir/data.sampled
  local phase_start=$role_dir/phase.start phase_end=$role_dir/phase.end
  module purge
  module load matlab/2026a
  test "$($python_current --version 2>&1)" = "Python 3.12.4"
  export VCS_OUTPUT_DIR=$role_dir VCS_SCRATCH_DIR=$role_scratch
  export VCS_INPUT_CSV=$scratch/input/input.csv VCS_INPUT_SHA256=$input_sha
  export VCS_MATLAB_ROOT VCS_MEX_DIR=$mex_dir
  export VCS_SOURCE_IDENTITY=$preparation/matlab_source_identity.json
  export VCS_PROCESS_IDENTITY=$role_dir/process_identity.json
  export VCS_EMPTY_READY=$empty_ready VCS_DATA_READY=$data_ready
  export VCS_EMPTY_ACK=$empty_ack VCS_DATA_ACK=$data_ack
  export VCS_PHASE_START=$phase_start VCS_PHASE_END=$phase_end
  export VCS_EXPERIMENT_ID=$experiment VCS_SOURCE_COMMIT
  export VCS_BUNDLE_SHA256 VCS_TASK_SHA256=$task_sha
  export VCS_STRUCTURE=$structure VCS_CONNECTIVITY=$connectivity
  export VCS_ROWS=$rows VCS_WORKERS=$workers VCS_FIRMS=$firms
  export VCS_DEGREE=$degree VCS_PROBES=$probes VCS_SEED=$seed
  export VCS_ACTIVE_CORES=$matlab_workers
  export MATLAB_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
  export OPENBLAS_NUM_THREADS=1
  set +e
  /usr/bin/time -v -o "$role_dir/resources.txt" \
    /usr/bin/timeout --signal=TERM --kill-after=60 "$estimator_timeout" \
    taskset -c "$target_cpu_list" matlab -batch \
      "addpath('$harness'); matlab_run" \
      > "$role_dir/application.txt" 2>&1 &
  local root_pid=$!
  "$python_current" "$monitor" --root-pid "$root_pid" \
    --interval-seconds 0.10 --empty-ready "$empty_ready" --data-ready "$data_ready" \
    --empty-ack "$empty_ack" --data-ack "$data_ack" \
    --phase-start "$phase_start" --phase-end "$phase_end" \
    --expected-pool-workers "$matlab_workers" \
    --process-identity "$VCS_PROCESS_IDENTITY" \
    --output "$role_dir/process_tree.json" \
    > "$role_dir/process_tree.monitor.txt" 2>&1
  local monitor_rc=$?
  wait "$root_pid"
  local app_rc=$?
  set -e
  local timed_out=0 valid=0
  test "$app_rc" = 124 && timed_out=1
  if test "$app_rc" = 0 && test "$monitor_rc" = 0 && \
     test -s "$role_dir/matlab.json" && test -s "$role_dir/application.pass" && \
     grep -Fq "FEVC MATLAB 2026 MATLAB PASS: $experiment" \
       "$role_dir/application.txt"; then
    valid=1
  fi
  write_status "$role" "$app_rc" "$monitor_rc" "$timed_out" "$valid" \
    "$matlab_workers" "$target_cpu_list"
  if (( app_rc != 0 || monitor_rc != 0 || timed_out != 0 || valid != 1 )); then
    return 1
  fi
}

failure_stage=applications
IFS=',' read -r -a roles <<< "$order"
test "${#roles[@]}" = 2
{
  printf 'position\trole\n'
  for index in "${!roles[@]}"; do
    printf '%s\t%s\n' "$((index+1))" "${roles[$index]}"
  done
} > "$output/order.tsv"
for role in "${roles[@]}"; do
  case "$role" in
    rust) run_stata "$role" ;;
    matlab) run_matlab ;;
    *) exit 198 ;;
  esac
done

failure_stage=node_receipt
task_end_epoch=$(date -u +%s.%N)
task_end_utc=$(date -u +%Y-%m-%dT%H:%M:%S.%NZ)
source_manifest_sha=$(sha256sum "$VCS_SOURCE_MANIFEST" | awk '{print $1}')
binary_manifest_sha=$(sha256sum "$preparation/binary_manifest.sha256" | awk '{print $1}')
cpu_model=$(lscpu | awk -F: '/Model name/{sub(/^[ \t]+/,"",$2); print $2; exit}')
{
  printf 'key\tvalue\n'
  printf 'schema\tFEVC-MATLAB-2026-NODE-V1\n'
  printf 'status\tPASS\n'
  printf 'attempt_id\t%s\n' "$VCS_ATTEMPT_ID"
  printf 'job_id\t%s\n' "$JOB_ID"
  printf 'task_id\t%s\n' "$task_id"
  printf 'scheduler_task_id\t%s\n' "$scheduler_task_id"
  printf 'task_map_sha256\t%s\n' "$task_map_sha"
  printf 'experiment_id\t%s\n' "$experiment"
  printf 'hostname\t%s\n' "$HOSTNAME"
  printf 'cpu_model\t%s\n' "$cpu_model"
  printf 'source_commit\t%s\n' "$VCS_SOURCE_COMMIT"
  printf 'bundle_sha256\t%s\n' "$VCS_BUNDLE_SHA256"
  printf 'source_manifest_sha256\t%s\n' "$source_manifest_sha"
  printf 'binary_manifest_sha256\t%s\n' "$binary_manifest_sha"
  printf 'task_sha256\t%s\n' "$task_sha"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'requested_slots\t%s\n' "$requested_slots"
  printf 'actual_slots\t%s\n' "$NSLOTS"
  printf 'active_cores\t%s\n' "$active_cores"
  printf 'stata_processors\t%s\n' "$stata_processors"
  printf 'rust_threads\t%s\n' "$rust_threads"
  printf 'matlab_workers\t%s\n' "$matlab_workers"
  printf 'max_iterations\t%s\n' "$VCS_MAX_ITERATIONS"
  printf 'active_cpu_affinity\t%s\n' "$target_cpu_list"
  printf 'assigned_cpu_affinity\t%s\n' "$assigned_cpu_list"
  printf 'scheduler_cpu_affinity\t%s\n' "$full_affinity"
  printf 'cpu_block_index\t%s\n' "$cpu_block_index"
  printf 'cpu_block_capacity\t%s\n' "$claim_capacity"
  printf 'binding_enforcement\tHARNESS_TASKSET_FLOCK_V1\n'
  printf 'python_module\tpython3/3.12.4\n'
  printf 'python_executable\t%s\n' "$python_bin"
  printf 'python_version\t%s\n' "$python_version"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core"
  printf 'command_memory_gib\t%s\n' "$command_memory"
  printf 'execution_order\t%s\n' "$order"
  printf 'task_start_utc\t%s\n' "$task_start_utc"
  printf 'task_end_utc\t%s\n' "$task_end_utc"
  printf 'task_start_epoch\t%s\n' "$task_start_epoch"
  printf 'task_end_epoch\t%s\n' "$task_end_epoch"
} > "$output/node_receipt.tsv"
printf 'FEVC_MATLAB_2026_CELL_CAPTURED %s %s\n' \
  "$experiment" "$task_sha" > "$output/wrapper.pass"
wrapper_complete=1
trap - EXIT
printf 'FEVC MATLAB 2026 CELL CAPTURED: %s\n' "$experiment"
