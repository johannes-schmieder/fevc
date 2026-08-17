#!/usr/bin/env bash
set -euo pipefail

if (( $# != 7 )); then
  printf '%s\n' \
    "usage: submit_rng_k1.sh RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 SOURCE_MANIFEST EXPERIMENT_ID" >&2
  exit 198
fi

run_dir=$1
source_dir=$2
source_commit=$3
bundle_archive=$4
bundle_sha=$5
source_manifest=$6
experiment_id=$7

# The sortable run ID and experiment ID restrictions also make the qsub -v
# payload unambiguous: none of its values can contain commas or newlines.
[[ "$run_dir" =~ ^/projectnb/welfgr/kss-bc/runs/[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]] || {
  printf '%s\n' "invalid KSS-SCALE run directory" >&2
  exit 198
}
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$experiment_id" =~ ^[A-Za-z0-9._-]+$ ]]

bundle_dir=/projectnb/welfgr/kss-bc/bundles/$bundle_sha
test "$bundle_archive" = "$bundle_dir/$bundle_sha.tar.gz"
test "$source_manifest" = "$bundle_dir/$bundle_sha.files.sha256"
test "$source_dir" = "$bundle_dir/source"
test -d "$run_dir"
test ! -L "$run_dir"
test -d "$bundle_dir"
test ! -L "$bundle_dir"
test -d "$source_dir"
test ! -L "$source_dir"
test -f "$bundle_archive"
test ! -L "$bundle_archive"
test -f "$source_manifest"
test ! -L "$source_manifest"
test -f "$source_dir/BUNDLE_FILES.sha256"
test -f "$source_dir/BUNDLE_FORMAT.txt"
test -f "$source_dir/SOURCE_COMMIT.txt"
test -f "$source_dir/kss_bc/benchmarks/scc/rng_k1_driver.do"
test -f "$source_dir/kss_bc/benchmarks/scc/run_rng_k1.sge"
test -f "$source_dir/kss_bc/benchmarks/scc/validate_rng_k1.py"
test "$(tr -d '[:space:]' < "$run_dir/bundle.kind")" = KSS-SCALE-1
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = \
  "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test "$(cat "$run_dir/bundle.path")" = "$bundle_dir"
test "$(tr -d '[:space:]' < "$source_dir/BUNDLE_FORMAT.txt")" = \
  KSS-SCALE-SOURCE-BUNDLE-V1
test "$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")" = \
  "$source_commit"
test "$(sha256sum "$bundle_archive" | awk '{print $1}')" = "$bundle_sha"
cmp "$source_manifest" "$source_dir/BUNDLE_FILES.sha256"
(
  cd "$source_dir"
  sha256sum -c "$source_manifest"
)

# Fourteen SGE slots reserve CPU, memory, and shared-node capacity. This is
# still one scalar SGE job running one Stata/MP process with four processors.
KSS_REQUESTED_SLOTS=14
KSS_STATA_PROCESSORS=4
KSS_HARD_WALL_SECONDS=3600
mem_per_core_gib=4
total_reserved_gib=$(( KSS_REQUESTED_SLOTS * mem_per_core_gib ))
test "$total_reserved_gib" = 56

output_dir="$run_dir/experiments/$experiment_id"
submission_dir="$run_dir/submissions"
log_dir="$run_dir/logs"
qacct_dir="$run_dir/qacct"
test ! -e "$output_dir"
test ! -e "$submission_dir/$experiment_id.job_id"
test ! -e "$submission_dir/$experiment_id.tsv"
mkdir -p "$output_dir" "$submission_dir" "$log_dir" "$qacct_dir"

{
  printf 'key\tvalue\n'
  printf 'receipt_version\tKSS-RNG-K1-RESERVATION-V1\n'
  printf 'experiment_id\t%s\n' "$experiment_id"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'requested_slots\t%s\n' "$KSS_REQUESTED_SLOTS"
  printf 'requested_stata_processors\t%s\n' "$KSS_STATA_PROCESSORS"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core_gib"
  printf 'total_reserved_gib\t%s\n' "$total_reserved_gib"
  printf 'hard_wall_seconds\t%s\n' "$KSS_HARD_WALL_SECONDS"
  printf 'application_timeout_seconds\t3480\n'
  printf 'timeout_basis\tlocal_stata18_target_per_probe_p3_63.364s\n'
  printf 'timeout_projected_k1_seconds\t2100\n'
  printf 'scheduler_shape\tone_scalar_job_no_array\n'
  printf 'stata_shape\tone_process_four_processors\n'
  printf 'qacct_expected_path\t%s\n' \
    "$qacct_dir/$experiment_id.txt"
} > "$output_dir/reservation.tsv"

environment="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit,KSS_BUNDLE_ARCHIVE=$bundle_archive,KSS_BUNDLE_SHA256=$bundle_sha,KSS_SOURCE_MANIFEST=$source_manifest,KSS_EXPERIMENT_ID=$experiment_id,KSS_OUTPUT_DIR=$output_dir,KSS_REQUESTED_SLOTS=$KSS_REQUESTED_SLOTS,KSS_STATA_PROCESSORS=$KSS_STATA_PROCESSORS,KSS_HARD_WALL_SECONDS=$KSS_HARD_WALL_SECONDS"

# Intentionally omit -t. RNG-K1 is one source-bound scalar job.
raw_job_id=$(qsub -terse -P welfgr -pe omp 14 \
  -l mem_per_core=4G -l h_rt=01:00:00 -j y \
  -o "$log_dir/$experiment_id.stdout.txt" -v "$environment" \
  "$source_dir/kss_bc/benchmarks/scc/run_rng_k1.sge")
[[ "$raw_job_id" =~ ^[0-9]+$ ]] || {
  printf '%s\n' "qsub returned a non-scalar job ID" >&2
  exit 65
}
job_id=$raw_job_id

printf '%s\n' "$job_id" > "$output_dir/job_id.txt"
printf '%s\n' "$job_id" > "$submission_dir/$experiment_id.job_id"
{
  printf 'key\tvalue\n'
  printf 'receipt_version\tKSS-RNG-K1-SUBMISSION-V1\n'
  printf 'experiment_id\t%s\n' "$experiment_id"
  printf 'job_id\t%s\n' "$job_id"
  printf 'submitted_utc\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'requested_slots\t%s\n' "$KSS_REQUESTED_SLOTS"
  printf 'requested_stata_processors\t%s\n' "$KSS_STATA_PROCESSORS"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core_gib"
  printf 'total_reserved_gib\t%s\n' "$total_reserved_gib"
  printf 'hard_wall_seconds\t%s\n' "$KSS_HARD_WALL_SECONDS"
  printf 'scheduler_shape\tone_scalar_job_no_array\n'
  printf 'stata_shape\tone_process_four_processors\n'
  printf 'output_dir\t%s\n' "$output_dir"
  printf 'job_id_file\t%s\n' "$output_dir/job_id.txt"
  printf 'qacct_expected_path\t%s\n' \
    "$qacct_dir/$experiment_id.txt"
} > "$submission_dir/$experiment_id.tsv"

printf '%s\n' "$job_id"
