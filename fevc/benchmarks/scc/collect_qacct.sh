#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  printf '%s\n' "usage: collect_qacct.sh RUN_DIR JOB" >&2
  exit 198
fi
run_dir=$1
job=$2
case "$run_dir" in
  /projectnb/welfgr/fevc/runs/*) ;;
  *) printf '%s\n' "invalid run directory" >&2; exit 198 ;;
esac
[[ "$job" =~ ^(portability|oracle|smoke|medium|large|numopt_(easy|moderate|weak)_(b1|cmg)|separations_[A-Za-z0-9._-]+)$ ]] || {
  printf '%s\n' "invalid job label" >&2
  exit 198
}
job_id=$(tr -d '[:space:]' < "$run_dir/submissions/$job.job_id")
[[ "$job_id" =~ ^[0-9]+([.][0-9:-]+)?$ ]] || {
  printf '%s\n' "invalid recorded SGE job ID" >&2
  exit 198
}
qacct -j "$job_id" > "$run_dir/qacct/$job.txt"
test -s "$run_dir/qacct/$job.txt"
printf '%s\n' "collected qacct for $job job $job_id"
