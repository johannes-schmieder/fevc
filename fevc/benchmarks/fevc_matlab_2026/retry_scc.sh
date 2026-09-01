#!/bin/bash
# Submit the one permitted retry for scheduler/execution-host failures.
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: retry_scc.sh RUN_DIR BUNDLE_TASK_IDS\n' >&2
  exit 198
fi
run_dir=${1%/}
bundle_ids=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$bundle_ids" =~ ^[0-9,-]+$ ]]
test -f "$run_dir/submissions/campaign.tsv"
test ! -e "$run_dir/submissions/retry.tsv"
test ! -e "$run_dir/submissions/retry.plan.json"
test ! -e "$run_dir/attempts/retry"

module purge
module load python3/3.12.4
python_bin=$(command -v python3)
harness=$run_dir/source/fevc/benchmarks/fevc_matlab_2026
identity=$run_dir/run_identity.json
read_identity() {
  "$python_bin" -c "import json,sys; print(json.load(open(sys.argv[1]))[$1])" \
    "$identity"
}
source_mode=$(read_identity '"source_mode"')
source_commit=$(read_identity '"source_commit"')
bundle_sha=$(read_identity '"bundle_sha256"')
required_stata_processors=$(read_identity '"required_stata_processors"')
required_rust_threads=$(read_identity '"required_rust_threads"')
test "$source_mode" = CLEAN_COMMIT

value() {
  awk -F '\t' -v key="$1" '$1 == key {print $2}' \
    "$run_dir/submissions/campaign.tsv"
}
production_job=$(value production_job_id)
[[ "$production_job" =~ ^[0-9]+$ ]]
if qstat -j "$production_job" >/dev/null 2>&1; then
  printf 'production job is still active: %s\n' "$production_job" >&2
  exit 198
fi
production_qacct=$run_dir/submissions/production.retry-gate.qacct.txt
test ! -e "$production_qacct"
qacct -j "$production_job" > "$production_qacct"
bundle_range=$("$python_bin" "$harness/prepare_retry.py" \
  --run-dir "$run_dir" --bundle-ids "$bundle_ids" --qacct "$production_qacct" \
  --output "$run_dir/submissions/retry.plan.json")
[[ "$bundle_range" =~ ^[0-9,-]+$ ]]

attempt=$run_dir/attempts/retry
mkdir -p "$attempt"/{tasks,validations,qacct,bundles}
source_dir=$run_dir/source
environment="VCS_RUN_DIR=$run_dir,VCS_SOURCE_DIR=$source_dir,VCS_SOURCE_COMMIT=$source_commit,VCS_BUNDLE_SHA256=$bundle_sha,VCS_SOURCE_MANIFEST=$run_dir/input/source.files.sha256,VCS_TASK_MANIFEST=$run_dir/input/tasks.tsv,VCS_BUNDLE_MANIFEST=$run_dir/input/bundles.tsv,VCS_MATLAB_ROOT=${VCS_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay},VCS_REQUIRED_STATA_PROCESSORS=$required_stata_processors,VCS_REQUIRED_RUST_THREADS=$required_rust_threads,VCS_ATTEMPT_ID=retry,VCS_STAGE=production,VCS_BUNDLE_CELL_FILTER=NONE"

retry_job=NONE
released=FALSE
cleanup_held_job() {
  if test "$released" != TRUE && test "$retry_job" != NONE; then
    qdel "$retry_job" >/dev/null 2>&1 || true
  fi
}
trap cleanup_held_job EXIT
retry_job=$(qsub -terse -h -t "$bundle_range" -v "$environment" \
  -o "$run_dir/logs" "$harness/run_task.sge")

qstat_path=$run_dir/submissions/retry.effective-qstat.txt
effective=$run_dir/submissions/retry.effective-sge.json
qstat -j "$retry_job" > "$qstat_path"
"$python_bin" "$harness/verify_sge_submission.py" \
  --qstat "$qstat_path" --output "$effective" --expected-slots 28 \
  --require-binding --binding-script "$harness/run_task.sge"
{
  printf 'key\tvalue\n'
  printf 'schema\tFEVC-MATLAB-2026-RETRY-SUBMISSION-V1\n'
  printf 'status\tPASS\n'
  printf 'run_id\t%s\n' "${run_dir##*/}"
  printf 'source_mode\t%s\n' "$source_mode"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'original_production_job_id\t%s\n' "$production_job"
  printf 'retry_job_id\t%s\n' "$retry_job"
  printf 'retry_bundle_range\t%s\n' "$bundle_range"
  printf 'requested_bundle_task_ids\t%s\n' "$bundle_ids"
  printf 'retry_limit\tONE\n'
  printf 'monitoring\tON_DEMAND_CAMPAIGN_STATUS\n'
} > "$run_dir/submissions/retry.tsv"
qrls "$retry_job"
released=TRUE
trap - EXIT
printf 'FEVC MATLAB 2026 RETRY SUBMITTED: job=%s bundles=%s\n' \
  "$retry_job" "$bundle_range"
