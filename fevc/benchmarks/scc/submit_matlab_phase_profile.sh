#!/usr/bin/env bash
set -euo pipefail

if (( $# != 17 )); then
  printf '%s\n' \
    "usage: submit_matlab_phase_profile.sh RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_SHA256 LABEL INPUT_CSV INPUT_SHA256 MATLAB_ROOT MATLAB_UPSTREAM_COMMIT PROBES SEED PROJECTED_SECONDS PROJECTION_BASIS CORE_SHA256 CMG_SHA256 CMG_MEX_SHA256 CMG_SOLVER_SHA256" >&2
  exit 198
fi

run_dir=$1
source_dir=$2
source_commit=$3
bundle_sha=$4
label=$5
input_csv=$6
input_sha=$7
matlab_root=$8
matlab_upstream_commit=$9
probes=${10}
seed=${11}
projected_seconds=${12}
projection_basis=${13}
core_sha=${14}
cmg_sha=${15}
cmg_mex_sha=${16}
cmg_solver_sha=${17}

registered_core_sha=7ab72bcf1f9e1a0091a6a423b1ef5cbd23688f7c64d753cf9adcc6243989a120
registered_upstream_commit=8b957ffeb10b8465a3584fceb0265cccc48379e1
registered_runtime_tree_sha=7d7581e77bcea131d0041cf7bab2d7a462fd5d535ca22110d080da51ded4f192

runtime_tree_hash() {
  (
    cd "$matlab_root"
    test "$(find codes CMG -type f | wc -l | tr -d '[:space:]')" = 184
    LC_ALL=C find codes CMG -type f -print0 | \
      LC_ALL=C sort -z | xargs -0 sha256sum
  ) | sha256sum | awk '{print $1}'
}

case "$run_dir" in /projectnb/welfgr/fevc/runs/*) ;; *) exit 198 ;; esac
case "$source_dir" in "/projectnb/welfgr/fevc/bundles/$bundle_sha/source") ;; *) exit 198 ;; esac
case "$input_csv" in /projectnb/welfgr/fevc/runs/*) ;; *) exit 198 ;; esac
case "$matlab_root" in /projectnb/welfgr/separations/*/LeaveOutTwoWay) ;; *) exit 198 ;; esac
for value in "$run_dir" "$source_dir" "$label" "$input_csv" "$matlab_root" "$projection_basis"; do
  [[ "$value" != *,* ]] || exit 198
done
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$label" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$input_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$matlab_upstream_commit" == "$registered_upstream_commit" ]]
[[ "$probes" =~ ^[1-9][0-9]*$ && "$seed" =~ ^[0-9]+$ ]]
(( seed <= 4294967295 ))
[[ "$projected_seconds" =~ ^[0-9]+([.][0-9]+)?$ ]]
[[ "$projection_basis" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$core_sha" == "$registered_core_sha" ]]
[[ "$cmg_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$cmg_mex_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$cmg_solver_sha" =~ ^[0-9a-f]{64}$ ]]
awk -v value="$projected_seconds" 'BEGIN { exit !(value > 0 && value <= 3600) }'

test -f "$input_csv"
test "$(sha256sum "$input_csv" | awk '{print $1}')" = "$input_sha"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test "$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")" = "$source_commit"
test "$(runtime_tree_hash)" = "$registered_runtime_tree_sha"
bundle_dir=${source_dir%/source}
bundle_archive="$bundle_dir/$bundle_sha.tar.gz"
bundle_manifest="$bundle_dir/$bundle_sha.files.sha256"
test "$(sha256sum "$bundle_archive" | awk '{print $1}')" = "$bundle_sha"
(cd "$source_dir" && sha256sum -c "$bundle_manifest")
profiler="$source_dir/fevc/benchmarks/separations_matlab_phase_profile.m"
wrapper="$source_dir/fevc/benchmarks/scc/run_matlab_phase_profile.sge"
test -f "$profiler"
test -f "$wrapper"
profiler_sha=$(sha256sum "$profiler" | awk '{print $1}')
[[ "$profiler_sha" =~ ^[0-9a-f]{64}$ ]]

# The projection covers the complete three-call profiler process.  Add 50%
# plus three minutes for module/startup variance, with a one-hour hard ceiling.
timeout_seconds=$(awk -v value="$projected_seconds" \
  'BEGIN { raw=1.5*value+180; projected=int(raw); if (projected<raw) projected++; if (projected<300) projected=300; if (projected>3600) exit 1; print projected }')
hours=$((timeout_seconds / 3600))
minutes=$(((timeout_seconds % 3600) / 60))
seconds=$((timeout_seconds % 60))
printf -v runtime '%02d:%02d:%02d' "$hours" "$minutes" "$seconds"

job_dir="$run_dir/matlab_phase_profile/$label"
test ! -e "$job_dir"
mkdir -p "$job_dir" "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"
submission="$job_dir/submission.tsv"
{
  printf 'label\t%s\n' "$label"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'matlab_upstream_commit\t%s\n' "$matlab_upstream_commit"
  printf 'matlab_runtime_tree_sha256\t%s\n' "$registered_runtime_tree_sha"
  printf 'matlab_core_sha256\t%s\n' "$core_sha"
  printf 'matlab_cmg_sha256\t%s\n' "$cmg_sha"
  printf 'matlab_cmg_mex_sha256\t%s\n' "$cmg_mex_sha"
  printf 'matlab_cmg_solver_sha256\t%s\n' "$cmg_solver_sha"
  printf 'profiler_sha256\t%s\n' "$profiler_sha"
  printf 'seed\t%s\n' "$seed"
  printf 'probes\t%s\n' "$probes"
  printf 'processors\t4\n'
  printf 'projected_seconds\t%s\n' "$projected_seconds"
  printf 'projection_basis\t%s\n' "$projection_basis"
  printf 'timeout_seconds\t%s\n' "$timeout_seconds"
} > "$submission"
chmod 600 "$submission"

environment="KSS_RUN_DIR=$run_dir,KSS_SOURCE_DIR=$source_dir,KSS_SOURCE_COMMIT=$source_commit,KSS_BUNDLE_SHA256=$bundle_sha,KSS_LABEL=$label,KSS_INPUT_CSV=$input_csv,KSS_INPUT_SHA256=$input_sha,KSS_MATLAB_ROOT=$matlab_root,KSS_MATLAB_UPSTREAM_COMMIT=$matlab_upstream_commit,KSS_PROBES=$probes,KSS_SEED=$seed,KSS_PROJECTED_SECONDS=$projected_seconds,KSS_PROJECTION_BASIS=$projection_basis,KSS_MATLAB_CORE_SHA256=$core_sha,KSS_MATLAB_CMG_SHA256=$cmg_sha,KSS_MATLAB_CMG_MEX_SHA256=$cmg_mex_sha,KSS_MATLAB_CMG_SOLVER_SHA256=$cmg_solver_sha,KSS_PROFILER_SHA256=$profiler_sha,KSS_TIMEOUT_SECONDS=$timeout_seconds"
record="matlab_phase_profile_${label}"
job_id=$(qsub -terse -P welfgr -pe omp 4 -l h_rt="$runtime" \
  -l mem_per_core=14G -j y -o "$run_dir/logs/$record.stdout.txt" \
  -v "$environment" "$wrapper")
[[ "$job_id" =~ ^[0-9]+$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/$record.job_id"
printf '%s\n' "$job_id"
