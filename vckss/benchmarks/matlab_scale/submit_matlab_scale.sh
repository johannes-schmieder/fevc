#!/bin/bash -l
set -euo pipefail

usage() {
  printf '%s\n' \
    "prepare: submit_matlab_scale.sh prepare RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 SOURCE_MANIFEST LABEL RETAINED_DTA RETAINED_DTA_SHA256 SCALE TOPOLOGY TIMEOUT_SECONDS TIMEOUT_BASIS" \
    "cold/warm: submit_matlab_scale.sh MODE RUN_DIR SOURCE_DIR SOURCE_COMMIT BUNDLE_ARCHIVE BUNDLE_SHA256 SOURCE_MANIFEST LABEL INPUT_CSV INPUT_SHA256 CASE_JSON CASE_SHA256 CONTRACT_SHA256 MATLAB_ROOT PREPARATION_RECEIPT PREPARATION_ACCEPTANCE TIMEOUT_SECONDS TIMEOUT_BASIS" >&2
  exit 198
}

(( $# >= 1 )) || usage
stage=$1
shift
[[ "$stage" =~ ^(prepare|cold|warm)$ ]] || usage

if [[ "$stage" == prepare ]]; then
  (( $# == 13 )) || usage
else
  (( $# == 17 )) || usage
fi

run_dir=$1
source_dir=$2
source_commit=$3
bundle_archive=$4
bundle_sha=$5
source_manifest=$6
label=$7
input_path=$8
input_sha=$9

case "$run_dir" in /projectnb/welfgr/vckss/runs/*) ;; *) exit 198 ;; esac
case "$source_dir" in
  "/projectnb/welfgr/vckss/bundles/$bundle_sha/source") ;;
  *) exit 198 ;;
esac
case "$bundle_archive" in
  "/projectnb/welfgr/vckss/bundles/$bundle_sha/$bundle_sha.tar.gz") ;;
  *) exit 198 ;;
esac
case "$source_manifest" in
  "/projectnb/welfgr/vckss/bundles/$bundle_sha/$bundle_sha.files.sha256") ;;
  *) exit 198 ;;
esac
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$label" =~ ^[A-Za-z0-9._-]+$ ]]
[[ "$input_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$input_path" =~ ^/projectnb/welfgr/[A-Za-z0-9._/-]+$ ]]
test -d "$run_dir"
test -d "$source_dir"
test -f "$bundle_archive"
test -f "$source_manifest"
test -f "$input_path"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
test "$(tr -d '[:space:]' < "$source_dir/SOURCE_COMMIT.txt")" = "$source_commit"
test "$(sha256sum "$bundle_archive" | awk '{print $1}')" = "$bundle_sha"
(cd "$source_dir" && sha256sum -c "$source_manifest")
test "$(sha256sum "$input_path" | awk '{print $1}')" = "$input_sha"

case_sha=NOT_APPLICABLE
contract_sha=NOT_APPLICABLE
matlab_root=NOT_APPLICABLE
scale=NOT_APPLICABLE
topology=NOT_APPLICABLE
sample_mode=fixed
preparation_receipt_sha=NOT_APPLICABLE
preparation_acceptance_sha=NOT_APPLICABLE
case_gate_sha=NOT_APPLICABLE
if [[ "$stage" == prepare ]]; then
  scale=${10}
  topology=${11}
  timeout_seconds=${12}
  timeout_basis=${13}
  case "$input_path" in /projectnb/welfgr/*) ;; *) exit 198 ;; esac
  [[ "$scale" =~ ^(1|2|4)$ ]]
  [[ "$topology" =~ ^(well|well_connected|ring)$ ]]
  if [[ "$scale" == 1 ]]; then
    [[ "$topology" == well ]]
  elif [[ "$scale" == 2 ]]; then
    [[ "$topology" == well_connected || "$topology" == ring ]]
  else
    [[ "$scale" == 4 && "$topology" == well_connected ]]
  fi
  requested_slots=14
  mem_per_core_gib=4
  stata_processors=4
  script="$source_dir/vckss/benchmarks/matlab_scale/run_prepare_fixed_sample.sge"
  job_dir="$run_dir/matlab_scale/$label/prepare"
  environment="KSS_MS_RUN_DIR=$run_dir,KSS_MS_PREP_JOB_DIR=$job_dir,KSS_MS_SOURCE_DIR=$source_dir,KSS_MS_SOURCE_COMMIT=$source_commit,KSS_MS_BUNDLE_SHA256=$bundle_sha,KSS_MS_LABEL=$label,KSS_MS_SCALE=$scale,KSS_MS_TOPOLOGY=$topology,KSS_MS_SOURCE_INPUT_DTA=$input_path,KSS_MS_SOURCE_INPUT_SHA256=$input_sha,KSS_MS_TIMEOUT_SECONDS=$timeout_seconds,KSS_MS_TIMEOUT_BASIS=$timeout_basis,KSS_MS_REQUESTED_SLOTS=$requested_slots,KSS_MS_MEM_PER_CORE_GIB=$mem_per_core_gib,KSS_MS_STATA_PROCESSORS=$stata_processors"
else
  case_json=${10}
  case_sha=${11}
  contract_sha=${12}
  matlab_root=${13}
  preparation_receipt=${14}
  preparation_acceptance=${15}
  timeout_seconds=${16}
  timeout_basis=${17}
  [[ "$case_sha" =~ ^[0-9a-f]{64}$ ]]
  [[ "$contract_sha" =~ ^[0-9a-f]{64}$ ]]
  test "$input_path" = "$run_dir/matlab_scale/$label/prepare/input.csv"
  test "$case_json" = "$run_dir/matlab_scale/$label/case.json"
  test -f "$run_dir/matlab_scale/$label/prepare/wrapper.pass"
  test "$preparation_receipt" = \
    "$run_dir/matlab_scale/$label/prepare/wrapper.json"
  test "$preparation_acceptance" = \
    "$run_dir/matlab_scale/$label/prepare/acceptance.json"
  test -f "$preparation_receipt"
  test -f "$preparation_acceptance"
  preparation_receipt_sha=$(sha256sum "$preparation_receipt" | awk '{print $1}')
  preparation_acceptance_sha=$(sha256sum "$preparation_acceptance" | awk '{print $1}')
  test "$(tr -d '[:space:]' < \
    "$run_dir/matlab_scale/$label/prepare/input.csv.sha256")" = "$input_sha"
  test -f "$case_json"
  test "$(sha256sum "$case_json" | awk '{print $1}')" = "$case_sha"
  contract_path=\
"$source_dir/vckss/benchmarks/matlab_scale/source_contract.json"
  test "$(sha256sum "$contract_path" | awk '{print $1}')" = "$contract_sha"
  case "$matlab_root" in /projectnb/welfgr/separations/*/LeaveOutTwoWay) ;;
    *) exit 198 ;;
  esac
  [[ "$matlab_root" =~ ^/projectnb/welfgr/[A-Za-z0-9._/-]+$ ]]
  module load python3/3.12.4
  python_bin=$(command -v python3)
  record="matlab_scale_${label}_${stage}"
  mkdir -p "$run_dir/submissions"
  case_gate="$run_dir/submissions/$record.case_gate.json"
  test ! -e "$case_gate"
  "$python_bin" \
    "$source_dir/vckss/benchmarks/matlab_scale/verify_submission.py" \
    --case "$case_json" --case-sha256 "$case_sha" \
    --contract "$contract_path" --contract-sha256 "$contract_sha" \
    --input "$input_path" --input-sha256 "$input_sha" \
    --preparation-receipt "$preparation_receipt" \
    --preparation-acceptance "$preparation_acceptance" \
    --executing-source-commit "$source_commit" \
    --executing-bundle-sha256 "$bundle_sha" --label "$label" \
    --output "$case_gate"
  grep -Fq '"status": "PASS"' "$case_gate"
  case_gate_sha=$(sha256sum "$case_gate" | awk '{print $1}')
  requested_slots=4
  mem_per_core_gib=14
  stata_processors=NOT_APPLICABLE
  script="$source_dir/vckss/benchmarks/matlab_scale/run_matlab_scale.sge"
  job_dir="$run_dir/matlab_scale/$label/$stage"
  environment="KSS_MS_RUN_DIR=$run_dir,KSS_MS_JOB_DIR=$job_dir,KSS_MS_SOURCE_DIR=$source_dir,KSS_MS_SOURCE_COMMIT=$source_commit,KSS_MS_BUNDLE_SHA256=$bundle_sha,KSS_MS_LABEL=$label,KSS_MS_MODE=$stage,KSS_MS_CASE_JSON=$case_json,KSS_MS_CASE_SHA256=$case_sha,KSS_MS_INPUT_CSV=$input_path,KSS_MS_INPUT_SHA256=$input_sha,KSS_MS_MATLAB_ROOT=$matlab_root,KSS_MS_CONTRACT_SHA256=$contract_sha,KSS_MS_PREPARATION_RECEIPT=$preparation_receipt,KSS_MS_PREPARATION_RECEIPT_SHA256=$preparation_receipt_sha,KSS_MS_PREPARATION_ACCEPTANCE=$preparation_acceptance,KSS_MS_PREPARATION_ACCEPTANCE_SHA256=$preparation_acceptance_sha,KSS_MS_TIMEOUT_SECONDS=$timeout_seconds,KSS_MS_TIMEOUT_BASIS=$timeout_basis,KSS_MS_REQUESTED_SLOTS=$requested_slots,KSS_MS_MEM_PER_CORE_GIB=$mem_per_core_gib"
fi

[[ "$timeout_seconds" =~ ^[1-9][0-9]*$ ]]
(( timeout_seconds >= 300 && timeout_seconds <= 42600 ))
[[ "$timeout_basis" =~ ^[A-Za-z0-9._-]+$ ]]
scheduler_hard_wall_seconds=$(( timeout_seconds + 600 ))
(( scheduler_hard_wall_seconds <= 43200 ))
environment+=",KSS_MS_SCHEDULER_HARD_WALL_SECONDS=$scheduler_hard_wall_seconds"
test -f "$script"

hours=$(( scheduler_hard_wall_seconds / 3600 ))
minutes=$(( (scheduler_hard_wall_seconds % 3600) / 60 ))
seconds=$(( scheduler_hard_wall_seconds % 60 ))
printf -v scheduler_hard_wall_hms '%02d:%02d:%02d' \
  "$hours" "$minutes" "$seconds"

record="matlab_scale_${label}_${stage}"
request="$run_dir/submissions/$record.request.tsv"
submission="$run_dir/submissions/$record.tsv"
scheduler_request="$run_dir/submissions/$record.scheduler_request.txt"
job_id_file="$run_dir/submissions/$record.job_id"
stdout_path="$run_dir/logs/$record.stdout.txt"
test ! -e "$job_dir"
test ! -e "$request"
test ! -e "$submission"
test ! -e "$scheduler_request"
test ! -e "$job_id_file"
mkdir -p "$job_dir" "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"

{
  printf 'key\tvalue\n'
  printf 'receipt_version\tKSS-MATLAB-SCALE-SUBMISSION-REQUEST-V1\n'
  printf 'stage\t%s\n' "$stage"
  printf 'label\t%s\n' "$label"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'input_path\t%s\n' "$input_path"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'case_sha256\t%s\n' "$case_sha"
  printf 'contract_sha256\t%s\n' "$contract_sha"
  printf 'sample_mode\t%s\n' "$sample_mode"
  printf 'preparation_receipt_sha256\t%s\n' "$preparation_receipt_sha"
  printf 'preparation_acceptance_sha256\t%s\n' \
    "$preparation_acceptance_sha"
  printf 'case_gate_sha256\t%s\n' "$case_gate_sha"
  printf 'scale\t%s\n' "$scale"
  printf 'topology\t%s\n' "$topology"
  printf 'matlab_root\t%s\n' "$matlab_root"
  printf 'requested_slots\t%s\n' "$requested_slots"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core_gib"
  printf 'total_reserved_gib\t%s\n' \
    "$(( requested_slots * mem_per_core_gib ))"
  printf 'stata_processors\t%s\n' "$stata_processors"
  printf 'application_timeout_seconds\t%s\n' "$timeout_seconds"
  printf 'timeout_basis\t%s\n' "$timeout_basis"
  printf 'scheduler_hard_wall_seconds\t%s\n' \
    "$scheduler_hard_wall_seconds"
  printf 'scheduler_hard_wall_hms\t%s\n' "$scheduler_hard_wall_hms"
  printf 'execution_shape\tone_scalar_job_no_array\n'
  printf 'project\twelfgr\n'
  printf 'parallel_environment\tomp\n'
  printf 'job_name\tkms_%s\n' "$stage"
  printf 'job_dir\t%s\n' "$job_dir"
  printf 'source_dir\t%s\n' "$source_dir"
  printf 'script_path\t%s\n' "$script"
  printf 'stdout_path\t%s\n' "$stdout_path"
  printf 'scheduler_request_path\t%s\n' "$scheduler_request"
  printf 'qacct_path\t%s\n' "$run_dir/qacct/$record.txt"
  printf 'automatic_cascade\tdisabled\n'
} > "$request"
request_sha=$(sha256sum "$request" | awk '{print $1}')

qsub_args=( -P welfgr -pe omp "$requested_slots" \
  -l "mem_per_core=${mem_per_core_gib}G" \
  -l "h_rt=$scheduler_hard_wall_hms" -j y -m a \
  -N "kms_${stage}" -o "$stdout_path" -v "$environment" "$script" )
scheduler_request_tmp="$scheduler_request.tmp"
qsub -verify "${qsub_args[@]}" > "$scheduler_request_tmp" 2>&1
test -s "$scheduler_request_tmp"
mv "$scheduler_request_tmp" "$scheduler_request"
scheduler_request_sha=$(sha256sum "$scheduler_request" | awk '{print $1}')

raw_job_id=$(qsub -terse "${qsub_args[@]}")
job_id=$raw_job_id
[[ "$job_id" =~ ^[0-9]+$ ]]
printf '%s\n' "$job_id" > "$job_id_file"
{
  printf 'key\tvalue\n'
  printf 'receipt_version\tKSS-MATLAB-SCALE-SUBMISSION-V1\n'
  printf 'stage\t%s\n' "$stage"
  printf 'label\t%s\n' "$label"
  printf 'job_id\t%s\n' "$job_id"
  printf 'submitted_utc\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'request_sha256\t%s\n' "$request_sha"
  printf 'scheduler_request_sha256\t%s\n' "$scheduler_request_sha"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'case_sha256\t%s\n' "$case_sha"
  printf 'sample_mode\t%s\n' "$sample_mode"
  printf 'preparation_receipt_sha256\t%s\n' "$preparation_receipt_sha"
  printf 'preparation_acceptance_sha256\t%s\n' \
    "$preparation_acceptance_sha"
  printf 'case_gate_sha256\t%s\n' "$case_gate_sha"
  printf 'requested_slots\t%s\n' "$requested_slots"
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core_gib"
  printf 'total_reserved_gib\t%s\n' \
    "$(( requested_slots * mem_per_core_gib ))"
  printf 'stata_processors\t%s\n' "$stata_processors"
  printf 'application_timeout_seconds\t%s\n' "$timeout_seconds"
  printf 'timeout_basis\t%s\n' "$timeout_basis"
  printf 'scheduler_hard_wall_seconds\t%s\n' \
    "$scheduler_hard_wall_seconds"
  printf 'execution_shape\tone_scalar_job_no_array\n'
  printf 'project\twelfgr\n'
  printf 'parallel_environment\tomp\n'
  printf 'job_name\tkms_%s\n' "$stage"
  printf 'job_dir\t%s\n' "$job_dir"
  printf 'source_dir\t%s\n' "$source_dir"
  printf 'script_path\t%s\n' "$script"
  printf 'stdout_path\t%s\n' "$stdout_path"
  printf 'scheduler_request_path\t%s\n' "$scheduler_request"
  printf 'qacct_path\t%s\n' "$run_dir/qacct/$record.txt"
  printf 'automatic_cascade\tdisabled\n'
} > "$submission"
printf '%s\n' "$job_id"
