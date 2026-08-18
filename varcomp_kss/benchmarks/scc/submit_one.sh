#!/usr/bin/env bash
set -euo pipefail

if (( $# < 4 )); then
  printf '%s\n' \
    "usage: submit_one.sh RUN_DIR SOURCE_DIR COMMIT JOB [WORKERS FIRMS PROBES SEED]" >&2
  exit 198
fi

run_dir=$1
source_dir=$2
source_commit=$3
job=$4

case "$run_dir" in
  /projectnb/welfgr/varcomp-kss/runs/*) ;;
  *) printf '%s\n' "RUN_DIR must be under /projectnb/welfgr/varcomp-kss/runs/" >&2; exit 198 ;;
esac
case "$source_dir" in
  "$run_dir"/source) ;;
  *) printf '%s\n' "SOURCE_DIR must equal RUN_DIR/source" >&2; exit 198 ;;
esac
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]] || {
  printf '%s\n' "COMMIT must be a full Git object ID" >&2
  exit 198
}
test -f "$source_dir/varcomp_kss/varcomp_kss.ado"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
mkdir -p "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"

common_env="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit"
case "$job" in
  portability)
    script="$source_dir/varcomp_kss/benchmarks/scc/run_portability.sge"
    job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt=02:00:00 \
      -l mem_per_core=4G -j y -o "$run_dir/logs/portability.stdout.txt" \
      -v "$common_env" "$script")
    ;;
  oracle)
    script="$source_dir/varcomp_kss/benchmarks/scc/run_oracle.sge"
    job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt=02:00:00 \
      -l mem_per_core=4G -j y -o "$run_dir/logs/oracle.stdout.txt" \
      -v "$common_env" "$script")
    ;;
  smoke|medium|large)
    if (( $# != 8 )); then
      printf '%s\n' "scale jobs require WORKERS FIRMS PROBES SEED" >&2
      exit 198
    fi
    workers=$5
    firms=$6
    probes=$7
    seed=$8
    [[ "$workers" =~ ^[0-9]+$ && "$firms" =~ ^[0-9]+$ && \
       "$probes" =~ ^[0-9]+$ && "$seed" =~ ^[0-9]+$ ]] || {
      printf '%s\n' "scale arguments must be nonnegative decimal integers" >&2
      exit 198
    }
    scale_env="$common_env,KSS_RUN_ID=$(basename "$run_dir"),KSS_SCENARIO=$job,KSS_WORKERS=$workers,KSS_FIRMS=$firms,KSS_PROBES=$probes,KSS_SEED=$seed"
    script="$source_dir/varcomp_kss/benchmarks/scc/run_scale.sge"
    case "$job" in
      smoke) job_runtime=02:00:00; job_memory=2G ;;
      medium) job_runtime=08:00:00; job_memory=4G ;;
      large) job_runtime=18:00:00; job_memory=8G ;;
    esac
    job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt="$job_runtime" \
      -l mem_per_core="$job_memory" -j y -o "$run_dir/logs/$job.stdout.txt" \
      -v "$scale_env" "$script")
    ;;
  *) printf '%s\n' "unknown job: $job" >&2; exit 198 ;;
esac

printf '%s\n' "$job_id" > "$run_dir/submissions/$job.job_id"
printf '%s\n' "$job_id"
