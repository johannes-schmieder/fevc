#!/usr/bin/env bash
set -euo pipefail

if (( $# != 12 )); then
  printf '%s\n' \
    "usage: submit_numopt.sh RUN_DIR SOURCE_DIR COMMIT ROUTE SCENARIO WORKERS FIRMS PROBES SEED MEMORY_GIB PROJECTED_SECONDS PROJECTION_BASIS" >&2
  exit 198
fi

run_dir=$1
source_dir=$2
source_commit=$3
route=$4
scenario=$5
workers=$6
firms=$7
probes=$8
seed=$9
memory_gib=${10}
projected_seconds=${11}
projection_basis=${12}

case "$run_dir" in /projectnb/welfgr/vckss/runs/*) ;; *) exit 198 ;; esac
test "$source_dir" = "$run_dir/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$route" =~ ^(b1|cmg)$ ]]
[[ "$scenario" =~ ^(easy|moderate|weak)$ ]]
[[ "$workers" =~ ^[0-9]+$ && "$firms" =~ ^[0-9]+$ && \
   "$probes" =~ ^[0-9]+$ && "$seed" =~ ^[0-9]+$ ]]
[[ "$memory_gib" =~ ^([1-9]|[1-4][0-9]|5[0-6])$ ]]
[[ "$projected_seconds" =~ ^[0-9]+([.][0-9]+)?$ ]]
[[ "$projection_basis" =~ ^[A-Za-z0-9._-]+$ ]]
awk -v value="$projected_seconds" 'BEGIN { exit !(value > 0 && value <= 5400) }'
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test -f "$source_dir/vckss/benchmarks/estimator_cmg_benchmark.do"

mkdir -p "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"
label="numopt_${scenario}_${route}"
environment="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit,KSS_RUN_ID=$(basename "$run_dir"),KSS_ROUTE=$route,KSS_SCENARIO=$scenario,KSS_WORKERS=$workers,KSS_FIRMS=$firms,KSS_PROBES=$probes,KSS_SEED=$seed,KSS_MEMORY_GIB=$memory_gib,KSS_PROJECTED_SECONDS=$projected_seconds,KSS_PROJECTION_BASIS=$projection_basis"
job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt=01:45:00 \
  -l mem_per_core=16G -j y -o "$run_dir/logs/$label.stdout.txt" \
  -v "$environment" "$source_dir/vckss/benchmarks/scc/run_numopt.sge")
[[ "$job_id" =~ ^[0-9]+([.][0-9:-]+)?$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/$label.job_id"
printf '%s\n' "$job_id"
