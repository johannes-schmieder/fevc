#!/usr/bin/env bash
set -euo pipefail

if (( $# != 17 )); then
  printf '%s\n' \
    "usage: submit_numopt2_matlab.sh RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_SHA EXPERIMENT WORKERS FIRMS CELLS_PER_WORKER ROWS_PER_CELL CONNECTIVITY PROBES SEED MATLAB_ROOT WALL_SECONDS MEM_PER_CORE_GIB KSS_SOURCE_COMMIT KSS_BUNDLE_SHA" >&2
  exit 198
fi
run_dir=$1; source_dir=$2; source_commit=$3; bundle_sha=$4
experiment_id=$5; workers=$6; firms=$7; cells_per_worker=$8
rows_per_cell=$9; connectivity=${10}; probes=${11}; seed=${12}
matlab_root=${13}; hard_wall_seconds=${14}; mem_per_core=${15}
kss_source_commit=${16}; kss_bundle_sha=${17}
requested_slots=4

[[ "$run_dir" == /projectnb/welfgr/kss-bc/runs/* ]]
test "$source_dir" = "/projectnb/welfgr/kss-bc/bundles/$bundle_sha/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$kss_source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$kss_bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$experiment_id" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$workers" =~ ^[0-9]+$ && "$firms" =~ ^[0-9]+$ ]]
(( workers == 40 * firms ))
[[ "$cells_per_worker" =~ ^[2-7]$ ]]
[[ "$rows_per_cell" =~ ^(1|8)$ ]]
[[ "$connectivity" =~ ^(strong|weak)$ ]]
[[ "$probes" =~ ^[0-9]+$ ]] && (( probes >= 2 ))
[[ "$seed" =~ ^[0-9]+$ ]] && (( seed <= 2147483646 ))
case "$matlab_root" in /projectnb/welfgr/separations/*/LeaveOutTwoWay) ;;
  *) exit 198 ;;
esac
[[ "$hard_wall_seconds" =~ ^[0-9]+$ ]]
(( hard_wall_seconds >= 600 && hard_wall_seconds <= 42600 ))
[[ "$mem_per_core" =~ ^(14|32)$ ]]
test -d "$run_dir"; test -d "$source_dir"; test -d "$matlab_root"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test "$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")" = \
  "$source_commit"
contract="$source_dir/kss_bc/benchmarks/matlab_scale/source_contract.json"
contract_sha=$(sha256sum "$contract" | awk '{print $1}')
[[ "$contract_sha" =~ ^[0-9a-f]{64}$ ]]
script="$source_dir/kss_bc/benchmarks/scc/run_numopt2_matlab.sge"
test -f "$script"

output_dir="$run_dir/matlab_numopt2/$experiment_id"
test ! -e "$output_dir"
mkdir -p "$output_dir" "$run_dir/logs" "$run_dir/submissions" \
  "$run_dir/qacct"
{
  printf 'key\tvalue\n'
  printf 'task_version\tKSS-NUMOPT-2-MATLAB-TASK-V1\n'
  printf 'experiment_id\t%s\n' "$experiment_id"
  printf 'comparison_source_commit\t%s\n' "$source_commit"
  printf 'comparison_bundle_sha256\t%s\n' "$bundle_sha"
  printf 'kss_source_commit\t%s\n' "$kss_source_commit"
  printf 'kss_bundle_sha256\t%s\n' "$kss_bundle_sha"
  printf 'workers\t%s\n' "$workers"
  printf 'firms\t%s\n' "$firms"
  printf 'cells_per_worker\t%s\n' "$cells_per_worker"
  printf 'rows_per_cell\t%s\n' "$rows_per_cell"
  printf 'connectivity\t%s\n' "$connectivity"
  printf 'probes\t%s\n' "$probes"
  printf 'seed\t%s\n' "$seed"
  printf 'matlab_root\t%s\n' "$matlab_root"
  printf 'source_contract_sha256\t%s\n' "$contract_sha"
  printf 'requested_slots\t%s\n' "$requested_slots"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core"
  printf 'hard_wall_seconds\t%s\n' "$hard_wall_seconds"
  printf 'same_stored_row_fixture\t1\n'
  printf 'frequency_semantics_comparable\t%s\n' \
    "$(( rows_per_cell == 8 ? 1 : 0 ))"
  printf 'target_weight_semantics_comparable\t0\n'
  printf 'corrected_estimate_equality_gate\tNONE_DESCRIPTIVE_ONLY\n'
} > "$output_dir/task.tsv"
task_sha=$(sha256sum "$output_dir/task.tsv" | awk '{print $1}')
printf '%s\n' "$task_sha" > "$output_dir/task.sha256"

scheduler_wall=$(( hard_wall_seconds + 600 ))
hours=$(( scheduler_wall / 3600 ))
minutes=$(( (scheduler_wall % 3600) / 60 ))
seconds=$(( scheduler_wall % 60 ))
printf -v wall_hms '%02d:%02d:%02d' "$hours" "$minutes" "$seconds"
environment="KSS_NM_RUN_DIR=$run_dir,KSS_NM_SOURCE_DIR=$source_dir,KSS_NM_SOURCE_COMMIT=$source_commit,KSS_NM_BUNDLE_SHA256=$bundle_sha,KSS_NM_EXPERIMENT_ID=$experiment_id,KSS_NM_TASK_SHA256=$task_sha,KSS_NM_KSS_SOURCE_COMMIT=$kss_source_commit,KSS_NM_KSS_BUNDLE_SHA256=$kss_bundle_sha,KSS_NM_WORKERS=$workers,KSS_NM_FIRMS=$firms,KSS_NM_CELLS_PER_WORKER=$cells_per_worker,KSS_NM_ROWS_PER_CELL=$rows_per_cell,KSS_NM_CONNECTIVITY=$connectivity,KSS_NM_PROBES=$probes,KSS_NM_SEED=$seed,KSS_NM_MATLAB_ROOT=$matlab_root,KSS_NM_CONTRACT_SHA256=$contract_sha,KSS_NM_REQUESTED_SLOTS=$requested_slots,KSS_NM_MEM_PER_CORE_GIB=$mem_per_core,KSS_NM_HARD_WALL_SECONDS=$hard_wall_seconds,KSS_NM_OUTPUT_DIR=$output_dir"
record="matlab_numopt2_$experiment_id"
request="$run_dir/submissions/$record.scheduler_request.txt"
qsub_args=(-P welfgr -pe omp "$requested_slots" \
  -l "mem_per_core=${mem_per_core}G" -l "h_rt=$wall_hms" -j y -m a \
  -N kss_nm -o "$run_dir/logs/$record.stdout.txt" -v "$environment" "$script")
qsub -verify "${qsub_args[@]}" > "$request" 2>&1
test -s "$request"
job_id=$(qsub -terse "${qsub_args[@]}")
[[ "$job_id" =~ ^[0-9]+$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/$record.job_id"
printf '%s\n' "$job_id"
