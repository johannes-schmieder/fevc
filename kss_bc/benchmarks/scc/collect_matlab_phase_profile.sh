#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  printf '%s\n' "usage: collect_matlab_phase_profile.sh RUN_DIR LABEL" >&2
  exit 198
fi
run_dir=$1
label=$2
case "$run_dir" in /projectnb/welfgr/kss-bc/runs/*) ;; *) exit 198 ;; esac
[[ "$label" =~ ^[A-Za-z0-9._-]+$ ]]
record="matlab_phase_profile_${label}"
receipt="$run_dir/submissions/$record.job_id"
test -s "$receipt"
job_id=$(tr -d '[:space:]' < "$receipt")
[[ "$job_id" =~ ^[0-9]+$ ]]
mkdir -p "$run_dir/qacct"
report="$run_dir/qacct/$record.txt"
temporary="$report.tmp.$$"
trap 'rm -f "$temporary"' EXIT

# Accounting can lag qstat.  Publish only one complete scalar-job record.
for attempt in $(seq 1 60); do
  if qacct -j "$job_id" > "$temporary" 2>&1 && \
     awk '$1=="jobnumber" {jobs++} $1=="failed" {failed++}
          $1=="exit_status" {exits++} $1=="ru_wallclock" {wall++}
          $1=="maxvmem" {memory++} $1=="slots" {slots++}
          END {exit !(jobs==1 && failed==1 && exits==1 && wall==1 && memory==1 && slots==1)}' \
       "$temporary"; then
    mv "$temporary" "$report"
    trap - EXIT
    failed=$(awk '$1=="failed" {print $2}' "$report")
    exit_status=$(awk '$1=="exit_status" {print $2}' "$report")
    printf '%s\n' \
      "collected complete qacct for $record job $job_id failed=$failed exit_status=$exit_status"
    [[ "$failed" == 0 && "$exit_status" == 0 ]]
    exit
  fi
  sleep 10
done
printf '%s\n' "qacct remained unavailable or incomplete for job $job_id" >&2
exit 75
