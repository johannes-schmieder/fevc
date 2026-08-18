#!/usr/bin/env bash
set -euo pipefail

if (( $# != 18 )); then
  printf '%s\n' \
    "usage: submit_numopt2_scale.sh RUN_DIR SOURCE_DIR COMMIT BUNDLE_SHA EXPERIMENT WORKERS FIRMS CELLS_PER_WORKER ROWS_PER_CELL CONNECTIVITY PROBES SEED BATCH WALL_SECONDS SLOTS MEM_PER_CORE_GIB STATA_PROCESSORS LABEL" >&2
  exit 198
fi
run_dir=$1; source_dir=$2; source_commit=$3; bundle_sha=$4
experiment_id=$5; workers=$6; firms=$7; cells_per_worker=$8
rows_per_cell=$9; connectivity=${10}; probes=${11}; seed=${12}
batch=${13}; hard_wall_seconds=${14}; slots=${15}; mem_per_core=${16}
processors=${17}; label=${18}

[[ "$run_dir" == /projectnb/welfgr/kss-bc/runs/* ]]
test "$source_dir" = "/projectnb/welfgr/kss-bc/bundles/$bundle_sha/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$experiment_id" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$label" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$workers" =~ ^[0-9]+$ && "$firms" =~ ^[0-9]+$ ]]
(( workers == 40 * firms ))
[[ "$cells_per_worker" =~ ^[2-7]$ ]]
[[ "$rows_per_cell" =~ ^(1|8)$ ]]
[[ "$connectivity" =~ ^(strong|weak)$ ]]
[[ "$probes" =~ ^[0-9]+$ ]] && (( probes >= 2 ))
[[ "$seed" =~ ^[0-9]+$ ]] && (( seed <= 2147483646 ))
[[ "$batch" == auto || "$batch" =~ ^[1-9][0-9]*$ ]]
[[ "$hard_wall_seconds" =~ ^[0-9]+$ ]] && (( hard_wall_seconds >= 600 ))
[[ "$slots" =~ ^[0-9]+$ ]] && (( slots >= 1 ))
[[ "$mem_per_core" =~ ^[0-9]+$ ]] && (( mem_per_core >= 1 ))
[[ "$processors" =~ ^[0-9]+$ ]] && (( processors >= 1 && processors <= slots ))
test -d "$run_dir"; test -d "$source_dir"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test -f "$source_dir/kss_bc/benchmarks/scc/run_numopt2_scale.sge"
test -f "$source_dir/kss_bc/benchmarks/scc/numopt2_generate.do"

output_dir="$run_dir/experiments/$experiment_id"
test ! -e "$output_dir"
mkdir -p "$output_dir" "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"
{
  printf 'key\tvalue\n'
  printf 'task_version\tKSS-NUMOPT-2-TASK-V1\n'
  printf 'label\t%s\n' "$label"
  printf 'experiment_id\t%s\n' "$experiment_id"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'workers\t%s\n' "$workers"
  printf 'firms\t%s\n' "$firms"
  printf 'cells_per_worker\t%s\n' "$cells_per_worker"
  printf 'rows_per_cell\t%s\n' "$rows_per_cell"
  printf 'connectivity\t%s\n' "$connectivity"
  printf 'probes\t%s\n' "$probes"
  printf 'seed\t%s\n' "$seed"
  printf 'batch\t%s\n' "$batch"
  printf 'hard_wall_seconds\t%s\n' "$hard_wall_seconds"
  printf 'slots\t%s\n' "$slots"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core"
  printf 'stata_processors\t%s\n' "$processors"
} > "$output_dir/task.tsv"
task_sha=$(sha256sum "$output_dir/task.tsv" | awk '{print $1}')
printf '%s\n' "$task_sha" > "$output_dir/task.sha256"
memory_gib=$(( slots * mem_per_core ))
hours=$(( hard_wall_seconds / 3600 ))
minutes=$(( (hard_wall_seconds % 3600) / 60 ))
seconds=$(( hard_wall_seconds % 60 ))
printf -v wall_hms '%02d:%02d:%02d' "$hours" "$minutes" "$seconds"

environment="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit,KSS_BUNDLE_SHA256=$bundle_sha,KSS_EXPERIMENT_ID=$experiment_id,KSS_TASK_SHA256=$task_sha,KSS_WORKERS=$workers,KSS_FIRMS=$firms,KSS_CELLS_PER_WORKER=$cells_per_worker,KSS_ROWS_PER_CELL=$rows_per_cell,KSS_CONNECTIVITY=$connectivity,KSS_PROBES=$probes,KSS_SEED=$seed,KSS_BATCH=$batch,KSS_REQUESTED_SLOTS=$slots,KSS_STATA_PROCESSORS=$processors,KSS_MEM_PER_CORE_GIB=$mem_per_core,KSS_MEMORY_GIB=$memory_gib,KSS_HARD_WALL_SECONDS=$hard_wall_seconds,KSS_OUTPUT_DIR=$output_dir"
request="$run_dir/submissions/$experiment_id.scheduler_request.txt"
qsub_args=(-P welfgr -pe omp "$slots" -l "mem_per_core=${mem_per_core}G" \
  -l "h_rt=$wall_hms" -j y -o "$run_dir/logs/$experiment_id.stdout.txt" \
  -v "$environment" "$source_dir/kss_bc/benchmarks/scc/run_numopt2_scale.sge")
qsub -verify "${qsub_args[@]}" > "$request" 2>&1
test -s "$request"
job_id=$(qsub -terse "${qsub_args[@]}")
[[ "$job_id" =~ ^[0-9]+$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/$experiment_id.job_id"
printf '%s\n' "$job_id"
