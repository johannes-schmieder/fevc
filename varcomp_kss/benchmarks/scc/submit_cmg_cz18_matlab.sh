#!/usr/bin/env bash
set -euo pipefail

if (( $# != 14 )); then
  printf '%s\n' \
    'usage: submit_cmg_cz18_matlab.sh RUN SOURCE COMMIT BUNDLE EXP INPUT INPUT_SHA PROBES SEED MATLAB_ROOT WALL MEM_PER_CORE EXPECTED_ROWS EXPECTED_CELLS' >&2
  exit 198
fi
run_dir=$1; source_dir=$2; source_commit=$3; bundle_sha=$4
experiment=$5; input_dta=$6; input_sha=$7; probes=$8; seed=$9
matlab_root=${10}; wall=${11}; mem_per_core=${12}
expected_rows=${13}; expected_cells=${14}
requested_slots=4

[[ "$run_dir" == /projectnb/welfgr/varcomp-kss/runs/* ]]
test "$source_dir" = "/projectnb/welfgr/varcomp-kss/bundles/$bundle_sha/source"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$experiment" =~ ^[A-Za-z0-9._-]+$ ]]
case "$input_dta" in /projectnb/welfgr/*) ;; *) exit 198 ;; esac
[[ "$input_sha" =~ ^[0-9a-f]{64}$ ]]
test "$probes" = 20
test "$seed" = 8675309
case "$matlab_root" in /projectnb/welfgr/separations/*/LeaveOutTwoWay) ;;
  *) exit 198 ;;
esac
[[ "$wall" =~ ^[0-9]+$ ]] && (( wall >= 600 && wall <= 42600 ))
[[ "$mem_per_core" =~ ^(14|32)$ ]]
test "$expected_rows" = 8201888
test "$expected_cells" = 311730
test -d "$run_dir"; test -d "$source_dir"; test -f "$input_dta"
test -d "$matlab_root"
test "$(sha256sum "$input_dta" | awk '{print $1}')" = "$input_sha"
test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
contract="$source_dir/varcomp_kss/benchmarks/matlab_scale/source_contract.json"
contract_sha=$(sha256sum "$contract" | awk '{print $1}')
script="$source_dir/varcomp_kss/benchmarks/scc/run_cmg_cz18_matlab.sge"
test -f "$contract"; test -f "$script"

output="$run_dir/matlab_cz18/$experiment"
test ! -e "$output"
mkdir -p "$output" "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct"
{
  printf 'key\tvalue\n'
  printf 'task_version\tCMG-MATA-1-CZ18-MATLAB-TASK-V1\n'
  printf 'experiment_id\t%s\n' "$experiment"
  printf 'source_commit\t%s\n' "$source_commit"
  printf 'bundle_sha256\t%s\n' "$bundle_sha"
  printf 'input_path\t%s\n' "$input_dta"
  printf 'input_sha256\t%s\n' "$input_sha"
  printf 'stored_rows\t%s\n' "$expected_rows"
  printf 'workers\t117529\n'
  printf 'firms\t10603\n'
  printf 'coefficient_cells\t%s\n' "$expected_cells"
  printf 'probes\t%s\n' "$probes"
  printf 'seed\t%s\n' "$seed"
  printf 'matlab_root\t%s\n' "$matlab_root"
  printf 'source_contract_sha256\t%s\n' "$contract_sha"
  printf 'requested_slots\t%s\n' "$requested_slots"
  printf 'matlab_pool_workers\t4\n'
  printf 'stata_prepare_processors\t4\n'
  printf 'mem_per_core_gib\t%s\n' "$mem_per_core"
  printf 'hard_wall_seconds\t%s\n' "$wall"
  printf 'implementation_boundary\tMATLAB comparator may compile official C MEX; Stata implementation remains Mata-only\n'
} > "$output/task.tsv"
task_sha=$(sha256sum "$output/task.tsv" | awk '{print $1}')
printf '%s\n' "$task_sha" > "$output/task.sha256"

scheduler_wall=$(( wall + 600 ))
hours=$(( scheduler_wall / 3600 )); minutes=$(( (scheduler_wall % 3600) / 60 )); seconds=$(( scheduler_wall % 60 ))
printf -v wall_hms '%02d:%02d:%02d' "$hours" "$minutes" "$seconds"
environment="CMG_CZ_M_RUN_DIR=$run_dir,CMG_CZ_M_SOURCE_DIR=$source_dir,CMG_CZ_M_SOURCE_COMMIT=$source_commit,CMG_CZ_M_BUNDLE_SHA256=$bundle_sha,CMG_CZ_M_EXPERIMENT=$experiment,CMG_CZ_M_TASK_SHA256=$task_sha,CMG_CZ_M_INPUT_DTA=$input_dta,CMG_CZ_M_INPUT_SHA256=$input_sha,CMG_CZ_M_PROBES=$probes,CMG_CZ_M_SEED=$seed,CMG_CZ_M_MATLAB_ROOT=$matlab_root,CMG_CZ_M_CONTRACT_SHA256=$contract_sha,CMG_CZ_M_HARD_WALL_SECONDS=$wall,CMG_CZ_M_REQUESTED_SLOTS=$requested_slots,CMG_CZ_M_MEM_PER_CORE_GIB=$mem_per_core,CMG_CZ_M_OUTPUT_DIR=$output"
record="cmg_cz18_matlab_$experiment"
request="$run_dir/submissions/$record.scheduler_request.txt"
qsub_args=(-P welfgr -pe omp "$requested_slots" \
  -l "mem_per_core=${mem_per_core}G" -l "h_rt=$wall_hms" -j y -m a \
  -N cmg_cz_m -o "$run_dir/logs/$record.stdout.txt" \
  -v "$environment" "$script")
qsub -verify "${qsub_args[@]}" > "$request" 2>&1
test -s "$request"
job_id=$(qsub -terse "${qsub_args[@]}")
[[ "$job_id" =~ ^[0-9]+$ ]]
printf '%s\n' "$job_id" > "$run_dir/submissions/$record.job_id"
printf '%s\n' "$job_id"
