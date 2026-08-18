#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  printf '%s\n' "usage: collect_prod_qacct.sh RUN_DIR EXPERIMENT_ID" >&2
  exit 198
fi
run_dir=$1
experiment_id=$2
case "$run_dir" in /projectnb/welfgr/varcomp-kss/runs/*) ;; *) exit 198 ;; esac
[[ "$experiment_id" =~ ^[A-Za-z0-9._-]+$ ]]
receipt="$run_dir/submissions/$experiment_id.job_id"
test -s "$receipt"
job_id=$(tr -d '[:space:]' < "$receipt")
[[ "$job_id" =~ ^[0-9]+([.][0-9:-]+)?$ ]]
mkdir -p "$run_dir/qacct"
report="$run_dir/qacct/$experiment_id.txt"
temporary="$report.tmp.$$"
trap 'rm -f "$temporary"' EXIT

# Accounting often lags qstat. Preserve the complete qacct record atomically;
# never accept or store a truncated tail as evidence.
for attempt in $(seq 1 60); do
  if qacct -j "$job_id" > "$temporary" 2>&1 && \
     awk '$1=="failed" {failed++} $1=="exit_status" {exits++}
          $1=="maxvmem" {memory++} END {exit !(failed==1 && exits==1 && memory==1)}' \
       "$temporary"; then
    mv "$temporary" "$report"
    trap - EXIT
    failed=$(awk '$1=="failed" {print $2}' "$report")
    exit_status=$(awk '$1=="exit_status" {print $2}' "$report")
    printf '%s\n' \
      "collected complete qacct for $experiment_id job $job_id failed=$failed exit_status=$exit_status"
    [[ "$failed" == 0 && "$exit_status" == 0 ]]
    exit
  fi
  sleep 10
done
printf '%s\n' "qacct remained unavailable or structurally incomplete for job $job_id" >&2
exit 75
