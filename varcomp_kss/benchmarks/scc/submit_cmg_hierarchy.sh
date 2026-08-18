#!/usr/bin/env bash
set -euo pipefail

if (( $# != 14 )); then
  printf '%s\n' \
    'usage: submit_cmg_hierarchy.sh RUN SOURCE COMMIT BUNDLE EXP KIND ARG1 ARG2 ARG3 ARG4 WALL SLOTS MEM_PER_CORE STATA_PROCESSORS' >&2
  exit 198
fi
run_dir=$1; source_dir=$2; source_commit=$3; bundle_sha=$4
experiment=$5; kind=$6; arg1=$7; arg2=$8; arg3=$9; arg4=${10}
wall=${11}; slots=${12}; mem_per_core=${13}; processors=${14}

[[ "$run_dir" == /projectnb/welfgr/varcomp-kss/runs/* ]]
test "$source_dir" = "/projectnb/welfgr/varcomp-kss/bundles/$bundle_sha/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$experiment" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$wall" =~ ^[0-9]+$ ]] && (( wall >= 60 && wall <= 7200 ))
[[ "$slots" =~ ^[0-9]+$ ]] && (( slots >= 1 ))
[[ "$mem_per_core" =~ ^[0-9]+$ ]] && (( mem_per_core >= 1 ))
[[ "$processors" =~ ^[0-9]+$ ]] && (( processors >= 1 && processors <= slots ))
if [[ "$kind" == degree ]]; then
  [[ "$arg1" =~ ^[0-9]+$ ]] && (( arg1 >= 32 ))
  [[ "$arg2" =~ ^[2-7]$ && "$arg3" =~ ^[2-7]$ ]] && (( arg2 <= arg3 ))
  [[ "$arg4" =~ ^(STEINER_MATA|API5_REFERENCE)$ ]]
elif [[ "$kind" == family ]]; then
  [[ "$arg1" =~ ^[0-9]+$ ]] && (( arg1 >= 64 ))
  [[ "$arg2" =~ ^[0-9]+$ ]] && (( arg2 >= 1 ))
  [[ "$arg3" =~ ^(path|ring|star|irregular|expander|tied|oneheavy|logspread|barbell|lollipop|cluster_chain|disconnected|singleton)$ ]]
  test "$arg4" = NA
else
  exit 198
fi
test -d "$run_dir"; test -d "$source_dir"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
script="$source_dir/varcomp_kss/benchmarks/scc/run_cmg_hierarchy.sge"
test -f "$script"

output="$run_dir/hierarchy/$experiment"
test ! -e "$output"
mkdir -p "$output" "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"
{
  printf 'key\tvalue\n'
  printf 'task_version\tCMG-MATA-1-HIERARCHY-TASK-V1\n'
  printf 'experiment_id\t%s\n' "$experiment"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'kind\t%s\n' "$kind"
  printf 'arg1\t%s\n' "$arg1"
  printf 'arg2\t%s\n' "$arg2"
  printf 'arg3\t%s\n' "$arg3"
  printf 'arg4\t%s\n' "$arg4"
  printf 'hard_wall_seconds\t%s\n' "$wall"
  printf 'requested_slots\t%s\n' "$slots"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core"
  printf 'stata_processors\t%s\n' "$processors"
} > "$output/task.tsv"
sha256sum "$output/task.tsv" | awk '{print $1}' > "$output/task.sha256"

hours=$(( wall / 3600 )); minutes=$(( (wall % 3600) / 60 )); seconds=$(( wall % 60 ))
printf -v wall_hms '%02d:%02d:%02d' "$hours" "$minutes" "$seconds"
environment="CMG_H_RUN_DIR=$run_dir,CMG_H_SOURCE_DIR=$source_dir,CMG_H_SOURCE_COMMIT=$source_commit,CMG_H_BUNDLE_SHA256=$bundle_sha,CMG_H_EXPERIMENT=$experiment,CMG_H_KIND=$kind,CMG_H_ARG1=$arg1,CMG_H_ARG2=$arg2,CMG_H_ARG3=$arg3,CMG_H_ARG4=$arg4,CMG_H_REQUESTED_SLOTS=$slots,CMG_H_STATA_PROCESSORS=$processors,CMG_H_OUTPUT_DIR=$output"
request="$run_dir/submissions/cmg_hierarchy_$experiment.scheduler_request.txt"
qsub_args=(-P welfgr -pe omp "$slots" -l "mem_per_core=${mem_per_core}G" \
  -l "h_rt=$wall_hms" -j y -o "$run_dir/logs/cmg_hierarchy_$experiment.stdout.txt" \
  -v "$environment" "$script")
qsub -verify "${qsub_args[@]}" > "$request" 2>&1
test -s "$request"
job_id=$(qsub -terse "${qsub_args[@]}")
[[ "$job_id" =~ ^[0-9]+$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/cmg_hierarchy_$experiment.job_id"
printf '%s\n' "$job_id"
