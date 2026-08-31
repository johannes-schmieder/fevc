#!/usr/bin/env bash
set -euo pipefail

if (( $# != 5 )); then
  printf '%s\n' \
    "usage: submit_cmg_primary_matrix.sh RUN_DIR SOURCE_DIR COMMIT BUNDLE_SHA MATLAB_ROOT" >&2
  exit 198
fi
run_dir=$1
source_dir=$2
source_commit=$3
bundle_sha=$4
matlab_root=$5

[[ "$run_dir" == /projectnb/welfgr/fevc/runs/* ]]
test "$source_dir" = "/projectnb/welfgr/fevc/bundles/$bundle_sha/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
case "$matlab_root" in /projectnb/welfgr/separations/*/LeaveOutTwoWay) ;;
  *) exit 198 ;;
esac
test -d "$run_dir"
test -d "$source_dir"
test -d "$matlab_root"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"

builder="$source_dir/fevc/benchmarks/scc/build_cmg_primary_manifest.py"
stata_submit="$source_dir/fevc/benchmarks/scc/submit_numopt2_scale.sh"
matlab_submit="$source_dir/fevc/benchmarks/scc/submit_numopt2_matlab.sh"
test -f "$builder"
test -x "$stata_submit"
test -x "$matlab_submit"
mkdir -p "$run_dir/input" "$run_dir/submissions"
manifest="$run_dir/input/cmg_primary_tasks.tsv"
manifest_sha="$run_dir/input/cmg_primary_tasks.sha256"
if test ! -e "$manifest"; then
  test ! -e "$manifest_sha"
  module purge
  module load miniconda/25.3.1
  "$SCC_MINICONDA_BIN/python" "$builder" --output "$manifest" \
    --source-commit "$source_commit" --bundle "$bundle_sha"
  sha256sum "$manifest" | awk '{print $1}' > "$manifest_sha"
else
  test -f "$manifest_sha"
  test "$(sha256sum "$manifest" | awk '{print $1}')" = \
    "$(tr -d '[:space:]' < "$manifest_sha")"
fi

tail -n +2 "$manifest" | while IFS=$'\t' read -r \
  task_version task_commit task_bundle experiment repetition firms workers \
  degree rows_per_cell connectivity probes seed batch stata_wall stata_slots \
  stata_mem stata_processors matlab_wall matlab_mem label; do
  test "$task_version" = CMG-MATA-1-PRIMARY-TASK-V1
  test "$task_commit" = "$source_commit"
  test "$task_bundle" = "$bundle_sha"
  if test ! -e "$run_dir/submissions/$experiment.job_id"; then
    "$stata_submit" "$run_dir" "$source_dir" "$source_commit" \
      "$bundle_sha" "$experiment" "$workers" "$firms" "$degree" \
      "$rows_per_cell" "$connectivity" "$probes" "$seed" "$batch" \
      "$stata_wall" "$stata_slots" "$stata_mem" "$stata_processors" "$label"
  fi
  if test ! -e "$run_dir/submissions/matlab_numopt2_$experiment.job_id"; then
    "$matlab_submit" "$run_dir" "$source_dir" "$source_commit" \
      "$bundle_sha" "$experiment" "$workers" "$firms" "$degree" \
      "$rows_per_cell" "$connectivity" "$probes" "$seed" "$matlab_root" \
      "$matlab_wall" "$matlab_mem" "$source_commit" "$bundle_sha"
  fi
done

ledger_tmp="$run_dir/submissions/cmg_primary_jobs.tsv.tmp"
ledger="$run_dir/submissions/cmg_primary_jobs.tsv"
test ! -e "$ledger"
test ! -e "$ledger_tmp"
printf 'experiment_id\tstata_job_id\tmatlab_job_id\n' > "$ledger_tmp"
tail -n +2 "$manifest" | while IFS=$'\t' read -r \
  task_version task_commit task_bundle experiment rest; do
  stata_job=$(tr -d '[:space:]' < \
    "$run_dir/submissions/$experiment.job_id")
  matlab_job=$(tr -d '[:space:]' < \
    "$run_dir/submissions/matlab_numopt2_$experiment.job_id")
  [[ "$stata_job" =~ ^[0-9]+$ ]]
  [[ "$matlab_job" =~ ^[0-9]+$ ]]
  printf '%s\t%s\t%s\n' "$experiment" "$stata_job" "$matlab_job" \
    >> "$ledger_tmp"
done
test "$(wc -l < "$ledger_tmp")" = 121
mv "$ledger_tmp" "$ledger"
printf '%s\n' \
  "CMG_PRIMARY_SUBMISSION_PASS matched_tasks=120 jobs=240 manifest=$(tr -d '[:space:]' < "$manifest_sha")"
