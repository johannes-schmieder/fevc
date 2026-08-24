#!/usr/bin/env bash
set -euo pipefail

if (( $# != 21 )); then
  printf '%s\n' \
    "usage: submit_kss_scale.sh RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 SOURCE_MANIFEST DATASET DATASET_SHA256 EXPERIMENT_ID FIXTURE SCALE_FACTOR PROBES SEED BATCH HARD_WALL_SECONDS FREQUENCY_VAR TARGET_VAR DELETION_VAR REQUESTED_SLOTS MEM_PER_CORE_GIB STATA_PROCESSORS" >&2
  exit 198
fi

run_dir=$1
source_dir=$2
source_commit=$3
bundle_archive=$4
bundle_sha=$5
source_manifest=$6
dataset=$7
input_sha=$8
experiment_id=$9
fixture=${10}
scale_factor=${11}
probes=${12}
seed=${13}
batch=${14}
hard_wall_seconds=${15}
frequency_var=${16}
target_var=${17}
deletion_var=${18}
requested_slots=${19}
mem_per_core_gib=${20}
stata_processors=${21}

case "$run_dir" in
  /projectnb/welfgr/vckss/runs/*) ;;
  *) printf '%s\n' "invalid run directory" >&2; exit 198 ;;
esac
case "$source_dir" in
  /projectnb/welfgr/vckss/*) ;;
  *) printf '%s\n' "invalid source directory" >&2; exit 198 ;;
esac
case "$bundle_archive" in
  /projectnb/welfgr/vckss/bundles/*.tar.gz) ;;
  *) printf '%s\n' "invalid bundle archive" >&2; exit 198 ;;
esac
case "$source_manifest" in
  /projectnb/welfgr/vckss/*) ;;
  *) printf '%s\n' "invalid source manifest" >&2; exit 198 ;;
esac
case "$dataset" in
  /projectnb/welfgr/*) ;;
  *) printf '%s\n' "invalid input dataset path" >&2; exit 198 ;;
esac

[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$input_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$experiment_id" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$fixture" =~ ^(local|cz24|cz25|cz18|replicated_blocks|well_connected|ring)$ ]]
if [[ "$fixture" == well_connected ]]; then fixture=replicated_blocks; fi
[[ "$scale_factor" =~ ^[0-9]+$ ]] && (( scale_factor >= 1 ))
if [[ ! "$fixture" =~ ^(replicated_blocks|ring)$ ]]; then
  test "$scale_factor" = 1
fi
if [[ "$fixture" =~ ^(replicated_blocks|ring)$ ]]; then
  (( scale_factor >= 2 ))
fi
[[ "$probes" =~ ^[0-9]+$ ]] && (( probes >= 2 ))
[[ "$seed" =~ ^[0-9]+$ ]] && (( seed <= 2147483646 ))
[[ "$batch" == auto || "$batch" =~ ^[1-9][0-9]*$ ]]
[[ "$hard_wall_seconds" =~ ^[0-9]+$ ]]
(( hard_wall_seconds >= 60 ))
[[ "$requested_slots" =~ ^[0-9]+$ ]] && (( requested_slots >= 1 ))
[[ "$mem_per_core_gib" =~ ^[0-9]+$ ]] && (( mem_per_core_gib >= 1 ))
[[ "$stata_processors" =~ ^[0-9]+$ ]] && (( stata_processors >= 1 ))
(( stata_processors <= requested_slots ))
for variable in "$frequency_var" "$target_var" "$deletion_var"; do
  [[ "$variable" == - || "$variable" =~ ^[A-Za-z_][A-Za-z0-9_]{0,31}$ ]]
done

test -d "$run_dir"
test -d "$source_dir"
bundle_dir=$(dirname "$bundle_archive")
test "$bundle_archive" = "$bundle_dir/$bundle_sha.tar.gz"
test "$source_manifest" = "$bundle_dir/$bundle_sha.files.sha256"
test "$source_dir" = "$bundle_dir/source"
test -f "$source_dir/vckss/benchmarks/scc/kss_scale_driver.do"
test -f "$source_dir/vckss/benchmarks/scc/run_kss_scale.sge"
test -f "$source_manifest"
test -f "$bundle_archive"
test -f "$dataset"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test "$(sha256sum "$bundle_archive" | awk '{print $1}')" = "$bundle_sha"
(
  cd "$source_dir"
  sha256sum -c "$source_manifest"
)
test "$(sha256sum "$dataset" | awk '{print $1}')" = "$input_sha"

# Scheduler capacity and application processors come from this run's explicit
# specification.  There is no repository-wide slot, memory, or processor cap.
KSS_REQUESTED_SLOTS=$requested_slots
KSS_STATA_PROCESSORS=$stata_processors
total_reserved_gib=$(( KSS_REQUESTED_SLOTS * mem_per_core_gib ))

wrapper_reserve_seconds=120
if (( wrapper_reserve_seconds >= hard_wall_seconds )); then
  wrapper_reserve_seconds=$(( hard_wall_seconds / 10 ))
fi
estimator_hard_wall_seconds=$(( hard_wall_seconds - wrapper_reserve_seconds ))

hours=$(( hard_wall_seconds / 3600 ))
minutes=$(( (hard_wall_seconds % 3600) / 60 ))
seconds=$(( hard_wall_seconds % 60 ))
printf -v hard_wall_hms '%02d:%02d:%02d' "$hours" "$minutes" "$seconds"

output_dir="$run_dir/experiments/$experiment_id"
test ! -e "$output_dir"
mkdir -p "$output_dir" "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"

reservation="$output_dir/reservation.tsv"
{
  printf 'key\tvalue\n'
  printf 'experiment_id\t%s\n' "$experiment_id"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'option_contract\tKSS-STREAMLINE-OPTIONS-V1\n'
  printf 'frequency_var\t%s\n' "$frequency_var"
  printf 'target_var\t%s\n' "$target_var"
  printf 'deletion_var\t%s\n' "$deletion_var"
  printf 'deletion_mode\tmatch\n'
  printf 'requested_slots\t%s\n' "$KSS_REQUESTED_SLOTS"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core_gib"
  printf 'total_reserved_gib\t%s\n' "$total_reserved_gib"
  printf 'stata_processors\t%s\n' "$KSS_STATA_PROCESSORS"
  printf 'hard_wall_seconds\t%s\n' "$hard_wall_seconds"
  printf 'estimator_hard_wall_seconds\t%s\n' \
    "$estimator_hard_wall_seconds"
  printf 'fixture\t%s\n' "$fixture"
  printf 'scale_factor\t%s\n' "$scale_factor"
} > "$reservation"

environment="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit,KSS_BUNDLE_ARCHIVE=$bundle_archive,KSS_BUNDLE_SHA256=$bundle_sha,KSS_SOURCE_MANIFEST=$source_manifest,KSS_INPUT_DATASET=$dataset,KSS_INPUT_SHA256=$input_sha,KSS_EXPERIMENT_ID=$experiment_id,KSS_FIXTURE=$fixture,KSS_SCALE_FACTOR=$scale_factor,KSS_PROBES=$probes,KSS_SEED=$seed,KSS_BATCH=$batch,KSS_FREQUENCY_VAR=$frequency_var,KSS_TARGET_VAR=$target_var,KSS_DELETION_VAR=$deletion_var,KSS_REQUESTED_SLOTS=$KSS_REQUESTED_SLOTS,KSS_STATA_PROCESSORS=$KSS_STATA_PROCESSORS,KSS_MEM_PER_CORE_GIB=$mem_per_core_gib,KSS_MEMORY_GIB=$total_reserved_gib,KSS_HARD_WALL_SECONDS=$hard_wall_seconds,KSS_OUTPUT_DIR=$output_dir"

scheduler_request=\
"$run_dir/submissions/$experiment_id.scheduler_request.txt"
test ! -e "$scheduler_request"
qsub_args=(
  -P welfgr
  -pe omp "$KSS_REQUESTED_SLOTS"
  -l "mem_per_core=${mem_per_core_gib}G"
  -l "h_rt=$hard_wall_hms"
  -j y
  -o "$run_dir/logs/$experiment_id.stdout.txt"
  -v "$environment"
  "$source_dir/vckss/benchmarks/scc/run_kss_scale.sge"
)

# The non-submitting verification and the real scalar submission deliberately
# share every scheduler argument.  The raw verification output is durable
# evidence for request fields that BU qacct does not report.
qsub -verify "${qsub_args[@]}" > "$scheduler_request" 2>&1
test -s "$scheduler_request"
scheduler_request_sha=$(sha256sum "$scheduler_request" | awk '{print $1}')
raw_job_id=$(qsub -terse "${qsub_args[@]}")
[[ "$raw_job_id" =~ ^[0-9]+$ ]] || {
  printf '%s\n' "qsub returned a non-scalar job ID" >&2
  exit 65
}
job_id=$raw_job_id
printf '%s\n' "$job_id" > "$run_dir/submissions/$experiment_id.job_id"
{
  printf 'key\tvalue\n'
  printf 'experiment_id\t%s\n' "$experiment_id"
  printf 'job_id\t%s\n' "$job_id"
  printf 'submitted_utc\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'scheduler_request_sha256\t%s\n' "$scheduler_request_sha"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'option_contract\tKSS-STREAMLINE-OPTIONS-V1\n'
  printf 'frequency_var\t%s\n' "$frequency_var"
  printf 'target_var\t%s\n' "$target_var"
  printf 'deletion_var\t%s\n' "$deletion_var"
  printf 'deletion_mode\tmatch\n'
  printf 'requested_slots\t%s\n' "$KSS_REQUESTED_SLOTS"
  printf 'stata_processors\t%s\n' "$KSS_STATA_PROCESSORS"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core_gib"
  printf 'hard_wall_seconds\t%s\n' "$hard_wall_seconds"
} > "$run_dir/submissions/$experiment_id.tsv"
printf '%s\n' "$job_id"
