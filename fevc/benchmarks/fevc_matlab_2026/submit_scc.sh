#!/bin/bash
set -euo pipefail
if (( $# < 2 || $# > 3 )); then
  printf 'usage: submit_scc.sh RUN_DIR prepare|pilot-small|pilot-worst|production [ATTEMPT_ID]\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
attempt_id=${3:-$mode}
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$mode" =~ ^(prepare|pilot-small|pilot-worst|production)$ ]]
[[ "$attempt_id" =~ ^[A-Za-z0-9._-]+$ ]]
test -d "$run_dir/source" && test -f "$run_dir/run_identity.json"

module purge
module load python3/3.12.4
python_bin=$(command -v python3)
python_version=$("$python_bin" --version 2>&1)
test "$python_version" = "Python 3.12.4"
read_identity() {
  "$python_bin" -c "import json,sys; print(json.load(open(sys.argv[1]))[$1])" \
    "$run_dir/run_identity.json"
}
source_commit=$(read_identity '"source_commit"')
bundle_sha=$(read_identity '"bundle_sha256"')
memory=$(read_identity '"mem_per_core_gib"')
run_kind=$(read_identity '"run_kind"')
required_stata_processors=$(read_identity '"required_stata_processors"')
required_rust_threads=$(read_identity '"required_rust_threads"')
artifact_source_run_id=$("$python_bin" -c \
  'import json,sys; v=json.load(open(sys.argv[1])).get("artifact_source_run_id"); print("NONE" if v is None else v)' \
  "$run_dir/run_identity.json")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$memory" =~ ^[1-9][0-9]*$ ]]
test "$required_stata_processors" = 4
test "$required_rust_threads" = 28
if test "$mode" != prepare; then test "$mode" = "$run_kind"; fi

source_dir=$run_dir/source
source_manifest=$run_dir/input/source.files.sha256
task_manifest=$run_dir/input/tasks.tsv
bundle_manifest=$run_dir/input/bundles.tsv
matlab_root=${VCS_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
test -d "$matlab_root"
harness=$source_dir/fevc/benchmarks/fevc_matlab_2026
environment="VCS_RUN_DIR=$run_dir,VCS_SOURCE_DIR=$source_dir,VCS_SOURCE_COMMIT=$source_commit,VCS_BUNDLE_SHA256=$bundle_sha,VCS_SOURCE_MANIFEST=$source_manifest,VCS_TASK_MANIFEST=$task_manifest,VCS_BUNDLE_MANIFEST=$bundle_manifest,VCS_MATLAB_ROOT=$matlab_root,VCS_ATTEMPT_ID=$attempt_id,VCS_RUN_KIND=$run_kind,VCS_ARTIFACT_SOURCE_RUN_ID=$artifact_source_run_id,VCS_REQUIRED_STATA_PROCESSORS=$required_stata_processors,VCS_REQUIRED_RUST_THREADS=$required_rust_threads"

submission=$run_dir/submissions/$attempt_id.tsv
qstat_receipt=$run_dir/submissions/$attempt_id.effective-qstat.txt
effective_receipt=$run_dir/submissions/$attempt_id.effective-sge.json
test ! -e "$submission" && test ! -e "$qstat_receipt" && test ! -e "$effective_receipt"
job_id=
released=FALSE
cleanup_held_job() {
  if test "$released" != TRUE && test -n "$job_id"; then
    qdel "$job_id" >/dev/null 2>&1 || true
  fi
}
trap cleanup_held_job EXIT

case "$mode" in
  prepare)
    test ! -e "$run_dir/artifacts"
    job_id=$(qsub -terse -h -v "$environment" -o "$run_dir/logs" \
      "$harness/prepare_artifacts.sge")
    range=NONE
    slots=4
    cell_filter=NONE
    binding=()
    ;;
  pilot-small)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    test -f "$run_dir/receipts/preparation/qacct.pass.json"
    test ! -e "$run_dir/attempts/$attempt_id"
    mkdir -p "$run_dir/attempts/$attempt_id"/{tasks,validations,qacct,bundles}
    cell_filter=1
    environment="$environment,VCS_BUNDLE_CELL_FILTER=$cell_filter"
    job_id=$(qsub -terse -h -t 1 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=1
    slots=28
    binding=(--require-binding --binding-script "$harness/run_task.sge")
    ;;
  pilot-worst)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    test -f "$run_dir/receipts/preparation/qacct.pass.json"
    test ! -e "$run_dir/attempts/$attempt_id"
    mkdir -p "$run_dir/attempts/$attempt_id"/{tasks,validations,qacct,bundles}
    cell_filter=235
    environment="$environment,VCS_BUNDLE_CELL_FILTER=$cell_filter"
    job_id=$(qsub -terse -h -t 24 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=24
    slots=28
    binding=(--require-binding --binding-script "$harness/run_task.sge")
    ;;
  production)
    test -f "$run_dir/receipts/preparation/wrapper.pass"
    test -f "$run_dir/receipts/preparation/qacct.pass.json"
    "$python_bin" "$harness/verify_pilots.py" \
      --production "$run_dir/run_identity.json" \
      --root /projectnb/welfgr/vckss/runs \
      > "$run_dir/receipts/pilot_gate.pass"
    test ! -e "$run_dir/attempts/$attempt_id"
    mkdir -p "$run_dir/attempts/$attempt_id"/{tasks,validations,qacct,bundles}
    cell_filter=NONE
    environment="$environment,VCS_BUNDLE_CELL_FILTER=$cell_filter"
    job_id=$(qsub -terse -h -t 1-24 -l "mem_per_core=${memory}G" \
      -v "$environment" -o "$run_dir/logs" "$harness/run_task.sge")
    range=1-24
    slots=28
    binding=(--require-binding --binding-script "$harness/run_task.sge")
    ;;
esac

qstat -j "$job_id" > "$qstat_receipt"
"$python_bin" "$harness/verify_sge_submission.py" \
  --qstat "$qstat_receipt" --output "$effective_receipt" \
  --expected-slots "$slots" "${binding[@]}"
qrls "$job_id"
released=TRUE
trap - EXIT
{
  printf 'key\tvalue\n'
  printf 'schema\tFEVC-MATLAB-2026-SUBMISSION-V1\n'
  printf 'mode\t%s\n' "$mode"
  printf 'attempt_id\t%s\n' "$attempt_id"
  printf 'job_id\t%s\n' "$job_id"
  printf 'task_range\t%s\n' "$range"
  printf 'cell_filter\t%s\n' "$cell_filter"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'run_kind\t%s\n' "$run_kind"
  printf 'artifact_source_run_id\t%s\n' "$artifact_source_run_id"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'mem_per_core_gib\t%s\n' "$memory"
  printf 'required_rust_threads\t%s\n' "$required_rust_threads"
  printf 'python_module\tpython3/3.12.4\n'
  printf 'python_executable\t%s\n' "$python_bin"
  printf 'python_version\t%s\n' "$python_version"
  printf 'queue\tANY_ELIGIBLE\n'
  printf 'host\tANY_ELIGIBLE\n'
  printf 'cpu_type\tE5-2680v4\n'
  printf 'exclusive\tFALSE\n'
  printf 'effective_sge_receipt\t%s\n' "$effective_receipt"
  printf 'submission_control\tHOLD_VALIDATE_RELEASE\n'
  printf 'comparison_design\tPAIRED_WITHIN_BUNDLE_HOST\n'
} > "$submission"
printf 'FEVC MATLAB 2026 SUBMITTED: mode=%s job=%s range=%s filter=%s\n' \
  "$mode" "$job_id" "$range" "$cell_filter"
