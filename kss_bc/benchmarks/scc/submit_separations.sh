#!/usr/bin/env bash
set -euo pipefail

if (( $# < 5 )); then
  printf '%s\n' \
    "usage: submit_separations.sh RUN_DIR SOURCE_DIR COMMIT JOB LABEL [job arguments]" >&2
  exit 198
fi
run_dir=$1
source_dir=$2
source_commit=$3
job=$4
label=$5
shift 5

case "$run_dir" in /projectnb/welfgr/kss-bc/runs/*) ;; *) exit 198 ;; esac
test "$source_dir" = "$run_dir/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$label" =~ ^[A-Za-z0-9._-]+$ ]]
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
mkdir -p "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"
common="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit,KSS_LABEL=$label"

case "$job" in
  prepare)
    (( $# == 5 )) || exit 198
    wage_input=$1
    wage_sha=$2
    sample_mode=$3
    max_workers=$4
    separations_commit=$5
    case "$wage_input" in /projectnb/welfgr/separations/*) ;; *) exit 198 ;; esac
    [[ "$wage_sha" =~ ^[0-9a-f]{64}$ ]]
    [[ "$sample_mode" =~ ^(small|full)$ ]]
    [[ "$max_workers" =~ ^[0-9]+$ ]]
    [[ "$separations_commit" =~ ^[0-9a-f]{7,40}$ ]]
    environment="$common,KSS_WAGE_INPUT=$wage_input,KSS_WAGE_INPUT_SHA256=$wage_sha,KSS_SAMPLE_MODE=$sample_mode,KSS_MAX_WORKERS=$max_workers,KSS_SEPARATIONS_COMMIT=$separations_commit"
    script="$source_dir/kss_bc/benchmarks/scc/run_separations_prepare.sge"
    runtime=00:30:00
    memory=8G
    ;;
  b1|cmg)
    (( $# == 6 )) || exit 198
    probes=$1
    seed=$2
    memory_gib=$3
    projected_seconds=$4
    projection_basis=$5
    wage_sha=$6
    [[ "$probes" =~ ^[0-9]+$ && "$seed" =~ ^[0-9]+$ ]]
    [[ "$memory_gib" =~ ^([1-9]|[1-4][0-9]|5[0-6])$ ]]
    [[ "$projected_seconds" =~ ^[0-9]+([.][0-9]+)?$ ]]
    [[ "$projection_basis" =~ ^[A-Za-z0-9._-]+$ ]]
    [[ "$wage_sha" =~ ^[0-9a-f]{64}$ ]]
    awk -v value="$projected_seconds" 'BEGIN { exit !(value > 0 && value <= 5400) }'
    prepare_dir="$run_dir/separations/$label/prepare"
    prepared_sha=$(tr -d '[:space:]' < "$prepare_dir/prepared.dta.sha256")
    [[ "$prepared_sha" =~ ^[0-9a-f]{64}$ ]]
    environment="$common,KSS_ROUTE=$job,KSS_PREPARED_SHA256=$prepared_sha,KSS_WAGE_INPUT_SHA256=$wage_sha,KSS_PROBES=$probes,KSS_SEED=$seed,KSS_MEMORY_GIB=$memory_gib,KSS_PROJECTED_SECONDS=$projected_seconds,KSS_PROJECTION_BASIS=$projection_basis"
    script="$source_dir/kss_bc/benchmarks/scc/run_separations_estimator.sge"
    runtime=01:45:00
    memory=16G
    ;;
  matlab)
    (( $# == 8 )) || exit 198
    probes=$1
    seed=$2
    projected_seconds=$3
    projection_basis=$4
    matlab_root=$5
    core_sha=$6
    wage_sha=$7
    cmg_sha=$8
    [[ "$probes" =~ ^[0-9]+$ && "$seed" =~ ^[0-9]+$ ]]
    [[ "$projected_seconds" =~ ^[0-9]+([.][0-9]+)?$ ]]
    [[ "$projection_basis" =~ ^[A-Za-z0-9._-]+$ ]]
    [[ "$core_sha" =~ ^[0-9a-f]{64}$ && "$wage_sha" =~ ^[0-9a-f]{64}$ ]]
    [[ "$cmg_sha" =~ ^[0-9a-f]{64}$ ]]
    awk -v value="$projected_seconds" 'BEGIN { exit !(value > 0 && value <= 5400) }'
    prepare_dir="$run_dir/separations/$label/prepare"
    prepared_sha=$(tr -d '[:space:]' < "$prepare_dir/prepared.csv.sha256")
    [[ "$prepared_sha" =~ ^[0-9a-f]{64}$ ]]
    environment="$common,KSS_PREPARED_CSV_SHA256=$prepared_sha,KSS_PROBES=$probes,KSS_SEED=$seed,KSS_PROJECTED_SECONDS=$projected_seconds,KSS_PROJECTION_BASIS=$projection_basis,KSS_MATLAB_ROOT=$matlab_root,KSS_MATLAB_CORE_SHA256=$core_sha,KSS_WAGE_INPUT_SHA256=$wage_sha,KSS_MATLAB_CMG_SHA256=$cmg_sha"
    script="$source_dir/kss_bc/benchmarks/scc/run_separations_matlab.sge"
    runtime=01:45:00
    memory=16G
    ;;
  compare)
    (( $# == 0 )) || exit 198
    environment="$common"
    script="$source_dir/kss_bc/benchmarks/scc/run_separations_compare.sge"
    runtime=00:20:00
    memory=4G
    ;;
  *) exit 198 ;;
esac

record="separations_${label}_${job}"
job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt="$runtime" \
  -l mem_per_core="$memory" -j y -o "$run_dir/logs/$record.stdout.txt" \
  -v "$environment" "$script")
[[ "$job_id" =~ ^[0-9]+([.][0-9:-]+)?$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/$record.job_id"
printf '%s\n' "$job_id"
