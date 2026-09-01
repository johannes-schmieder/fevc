#!/bin/bash
set -euo pipefail
if test "$#" != 2; then
  printf 'usage: submit_scc.sh RUN_DIR smoke|campaign\n' >&2
  exit 198
fi
run_dir=${1%/}
mode=$2
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
[[ "$mode" =~ ^(smoke|campaign)$ ]]
test -d "$run_dir/source" && test -f "$run_dir/run_identity.json"

module purge
module load python3/3.12.4
python_bin=$(command -v python3)
test "$("$python_bin" --version 2>&1)" = "Python 3.12.4"
read_identity() {
  "$python_bin" -c "import json,sys; print(json.load(open(sys.argv[1]))[$1])" \
    "$run_dir/run_identity.json"
}
schema=$(read_identity '"schema"')
source_mode=$(read_identity '"source_mode"')
source_commit=$(read_identity '"source_commit"')
bundle_sha=$(read_identity '"bundle_sha256"')
memory=$(read_identity '"mem_per_core_gib"')
required_stata_processors=$(read_identity '"required_stata_processors"')
required_rust_threads=$(read_identity '"required_rust_threads"')
test "$schema" = FEVC-MATLAB-2026-CAMPAIGN-V2
[[ "$source_mode" =~ ^(DEVELOPMENT_SNAPSHOT|CLEAN_COMMIT)$ ]]
if test "$mode" = campaign; then test "$source_mode" = CLEAN_COMMIT; fi
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$memory" =~ ^[1-9][0-9]*$ ]]
test "$required_stata_processors" = 4
test "$required_rust_threads" = 28

source_dir=$run_dir/source
source_manifest=$run_dir/input/source.files.sha256
task_manifest=$run_dir/input/tasks.tsv
bundle_manifest=$run_dir/input/bundles.tsv
matlab_root=${VCS_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
test -d "$matlab_root"
harness=$source_dir/fevc/benchmarks/fevc_matlab_2026
environment="VCS_RUN_DIR=$run_dir,VCS_SOURCE_DIR=$source_dir,VCS_SOURCE_COMMIT=$source_commit,VCS_BUNDLE_SHA256=$bundle_sha,VCS_SOURCE_MANIFEST=$source_manifest,VCS_TASK_MANIFEST=$task_manifest,VCS_BUNDLE_MANIFEST=$bundle_manifest,VCS_MATLAB_ROOT=$matlab_root,VCS_REQUIRED_STATA_PROCESSORS=$required_stata_processors,VCS_REQUIRED_RUST_THREADS=$required_rust_threads"

receipt=$run_dir/submissions/$mode.tsv
test ! -e "$receipt"
declare -a job_ids=()
released=FALSE
cleanup_held_jobs() {
  if test "$released" != TRUE; then
    for job_id in "${job_ids[@]}"; do qdel "$job_id" >/dev/null 2>&1 || true; done
  fi
}
trap cleanup_held_jobs EXIT

smoke_job=$(qsub -terse -h -t 1 -pe omp 4 -l h_rt=00:20:00 \
  -l mem_per_core=8G -v "$environment" -o "$run_dir/logs" \
  "$harness/run_smoke.sge")
job_ids+=("$smoke_job")

pilot_job=NONE
production_job=NONE
if test "$mode" = campaign; then
  for attempt_id in pilot production; do
    attempt=$run_dir/attempts/$attempt_id
    test ! -e "$attempt"
    mkdir -p "$attempt"/{tasks,validations,qacct,bundles}
  done
  pilot_environment="$environment,VCS_ATTEMPT_ID=pilot,VCS_STAGE=pilot,VCS_BUNDLE_CELL_FILTER=235"
  pilot_job=$(qsub -terse -h -hold_jid "$smoke_job" -t 24 \
    -v "$pilot_environment" -o "$run_dir/logs" "$harness/run_task.sge")
  job_ids+=("$pilot_job")
  production_environment="$environment,VCS_ATTEMPT_ID=production,VCS_STAGE=production,VCS_BUNDLE_CELL_FILTER=NONE"
  production_job=$(qsub -terse -h -hold_jid "$pilot_job" -t 1-24 \
    -v "$production_environment" -o "$run_dir/logs" "$harness/run_task.sge")
  job_ids+=("$production_job")
fi

qstat_path=$run_dir/submissions/$mode.smoke.effective-qstat.txt
effective=$run_dir/submissions/$mode.smoke.effective-sge.json
qstat -j "$smoke_job" > "$qstat_path"
"$python_bin" "$harness/verify_sge_submission.py" \
  --qstat "$qstat_path" --output "$effective" --expected-slots 4
if test "$mode" = campaign; then
  for stage_job in pilot:$pilot_job production:$production_job; do
    IFS=: read -r stage job_id <<< "$stage_job"
    qstat_path=$run_dir/submissions/$mode.$stage.effective-qstat.txt
    effective=$run_dir/submissions/$mode.$stage.effective-sge.json
    qstat -j "$job_id" > "$qstat_path"
    "$python_bin" "$harness/verify_sge_submission.py" \
      --qstat "$qstat_path" --output "$effective" --expected-slots 28 \
      --require-binding --binding-script "$harness/run_task.sge"
  done
fi

{
  printf 'key\tvalue\n'
  printf 'schema\tFEVC-MATLAB-2026-CAMPAIGN-SUBMISSION-V1\n'
  printf 'status\tPASS\n'
  printf 'mode\t%s\n' "$mode"
  printf 'run_id\t%s\n' "${run_dir##*/}"
  printf 'source_mode\t%s\n' "$source_mode"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'smoke_job_id\t%s\n' "$smoke_job"
  printf 'pilot_job_id\t%s\n' "$pilot_job"
  printf 'production_job_id\t%s\n' "$production_job"
  if test "$mode" = campaign; then
    printf 'dependency_chain\tsmoke,pilot,production\n'
  else
    printf 'dependency_chain\tsmoke\n'
  fi
  printf 'monitoring\tON_DEMAND_CAMPAIGN_STATUS\n'
} > "$receipt"

for job_id in "${job_ids[@]}"; do qrls "$job_id"; done
released=TRUE
trap - EXIT
printf 'FEVC MATLAB 2026 SUBMITTED: mode=%s smoke=%s pilot=%s production=%s\n' \
  "$mode" "$smoke_job" "$pilot_job" "$production_job"
