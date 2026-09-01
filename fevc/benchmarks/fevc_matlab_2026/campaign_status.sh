#!/bin/bash
# Read-only summary for one smoke or full SCC campaign.
set -euo pipefail
if test "$#" != 1; then
  printf 'usage: campaign_status.sh RUN_DIR\n' >&2
  exit 198
fi
run_dir=${1%/}
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
test -f "$run_dir/run_identity.json"
submission=
for candidate in "$run_dir/submissions/campaign.tsv" "$run_dir/submissions/smoke.tsv"; do
  if test -f "$candidate"; then submission=$candidate; break; fi
done
test -n "$submission"

value() { awk -F '\t' -v key="$1" '$1 == key {print $2}' "$submission"; }
stage_status() {
  local stage=$1 job_id=$2 pass_file=$3
  if test "$job_id" = NONE; then
    printf '%s\tNOT_SUBMITTED\tNONE\n' "$stage"
    return
  fi
  if qstat -j "$job_id" >/dev/null 2>&1; then
    printf '%s\tACTIVE\t%s\n' "$stage" "$job_id"
    return
  fi
  accounting=$(qacct -j "$job_id" 2>/dev/null || true)
  if test -z "$accounting"; then
    printf '%s\tACCOUNTING_PENDING\t%s\n' "$stage" "$job_id"
    return
  fi
  if awk '$1 == "failed" || $1 == "exit_status" {if ($2 != 0) bad=1; seen++}
          END {exit !(seen > 0 && bad == 0)}' <<< "$accounting"; then
    if test -f "$pass_file"; then
      printf '%s\tPASS\t%s\n' "$stage" "$job_id"
    elif test "$stage" = production && \
         test "$(find "$run_dir/attempts/production/bundles" -mindepth 2 \
           -maxdepth 2 -name application.pass 2>/dev/null | wc -l | tr -d ' ')" = 24; then
      printf '%s\tPASS_PENDING_COLLECTION\t%s\n' "$stage" "$job_id"
    elif test "$stage" = retry && \
         test "$(find "$run_dir/attempts/retry/bundles" -mindepth 2 \
           -maxdepth 2 -name application.pass 2>/dev/null | wc -l | tr -d ' ')" \
           = "$retry_expected_bundles"; then
      printf '%s\tPASS_PENDING_COLLECTION\t%s\n' "$stage" "$job_id"
    else
      printf '%s\tAPPLICATION_RECEIPT_MISSING\t%s\n' "$stage" "$job_id"
    fi
  else
    printf '%s\tFAIL\t%s\n' "$stage" "$job_id"
  fi
}

printf 'stage\tstatus\tjob_id\n'
stage_status smoke "$(value smoke_job_id)" "$run_dir/receipts/smoke/wrapper.pass"
stage_status pilot "$(value pilot_job_id)" "$run_dir/receipts/pilot/wrapper.pass"
stage_status production "$(value production_job_id)" \
  "$run_dir/receipts/production/wrapper.pass"
if test -f "$run_dir/submissions/retry.tsv"; then
  retry_value() {
    awk -F '\t' -v key="$1" '$1 == key {print $2}' \
      "$run_dir/submissions/retry.tsv"
  }
  retry_range=$(retry_value retry_bundle_range)
  if [[ "$retry_range" =~ ^([0-9]+)-([0-9]+)$ ]]; then
    retry_expected_bundles=$((BASH_REMATCH[2] - BASH_REMATCH[1] + 1))
  else
    [[ "$retry_range" =~ ^[0-9]+$ ]]
    retry_expected_bundles=1
  fi
  stage_status retry "$(retry_value retry_job_id)" \
    "$run_dir/receipts/retry/wrapper.pass"
fi

production_job=$(value production_job_id)
if [[ "$production_job" =~ ^[0-9]+$ ]] && \
   ! qstat -j "$production_job" >/dev/null 2>&1; then
  production_accounting=$(qacct -j "$production_job" 2>/dev/null || true)
  if test -n "$production_accounting"; then
    scheduler_failed=$(awk '
      $1 == "taskid" {task=$2; failed=""}
      $1 == "failed" {failed=$2}
      $1 == "exit_status" && failed != "" && failed != 0 {print task}
    ' <<< "$production_accounting" | paste -sd, -)
    application_failed=$(awk '
      $1 == "taskid" {task=$2; failed=""}
      $1 == "failed" {failed=$2}
      $1 == "exit_status" && failed == 0 && $2 != 0 {print task}
    ' <<< "$production_accounting" | paste -sd, -)
    printf '\ndetail\tvalue\n'
    printf 'production_scheduler_failed_bundles\t%s\n' \
      "${scheduler_failed:-NONE}"
    printf 'production_application_failed_bundles\t%s\n' \
      "${application_failed:-NONE}"
  fi
fi
