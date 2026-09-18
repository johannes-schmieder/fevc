#!/usr/bin/env bash
set -euo pipefail
if (( $# != 1 )); then
  printf 'usage: submit_development.sh RUN_DIR\n' >&2
  exit 198
fi
run_dir=${1%/}
[[ "${run_dir}" == /projectnb/welfgr/vckss/runs/* ]]
test -d "${run_dir}/source"
test -f "${run_dir}/receipts/wrapper.txt"
test -f "${run_dir}/artifacts/candidate/fevc_rust_linux_x64.plugin"
test -f "${run_dir}/input/baseline-source.tar.gz"
test -f "${run_dir}/input/baseline-snapshot.json"
test -f "${run_dir}/input/development/development-manifest.json"
test ! -e "${run_dir}/submissions/development.tsv"
test ! -e "${run_dir}/campaign"
candidate_source=$(tr -d '[:space:]' < "${run_dir}/bundle.sha256")
[[ "${candidate_source}" =~ ^[0-9a-f]{64}$ ]]
baseline=4bdba22c0e6da99b3fd597aabab8df4d01307d01704951f3d5d2c089885cd2ad
harness=${run_dir}/source/rust/experiments/optimization_parity_20260913
environment="OP_RUN_DIR=${run_dir},OP_BASELINE_SNAPSHOT_SHA256=${baseline},OP_CANDIDATE_SOURCE_SHA256=${candidate_source}"
prep=$(qsub -terse -P welfgr -pe omp 4 -l mem_per_core=3G -l h_rt=00:25:00 \
  -j y -o "${run_dir}/logs/development-prep.txt" -v "${environment}" \
  "${harness}/prepare_development_packages.sge")
prep=${prep%%.*}
smoke=$(qsub -terse -P welfgr -hold_jid "${prep}" -pe omp 4 \
  -l mem_per_core=3G -l h_rt=00:25:00 -j y \
  -o "${run_dir}/logs/development-smoke.txt" -v "${environment}" \
  "${harness}/run_development_smoke.sge")
smoke=${smoke%%.*}
development=$(qsub -terse -P welfgr -hold_jid "${smoke}" -t 1-16 \
  -pe omp 14 -l mem_per_core=3G -l h_rt=00:45:00 -j y \
  -o "${run_dir}/logs" -v "${environment}" \
  "${harness}/run_development_cell.sge")
development=${development%%.*}
for job in "${prep}" "${smoke}" "${development}"; do
  [[ "${job}" =~ ^[0-9]+$ ]]
  qstat -j "${job}" > "${run_dir}/submissions/development-${job}.qstat.txt"
done
{
  printf 'schema\tFEVC-OPTIMIZATION-DEVELOPMENT-SUBMISSION-V1\n'
  printf 'status\tSUBMITTED\n'
  printf 'candidate_source_sha256\t%s\n' "${candidate_source}"
  printf 'baseline_source_sha256\t%s\n' "${baseline}"
  printf 'prep_job_id\t%s\n' "${prep}"
  printf 'smoke_job_id\t%s\n' "${smoke}"
  printf 'development_job_id\t%s\n' "${development}"
  printf 'development_tasks\t16\n'
  printf 'timed_calls\t96\n'
  printf 'warmups\t32\n'
} > "${run_dir}/submissions/development.tsv"
printf 'FEVC OPTIMIZATION DEVELOPMENT SUBMITTED: prep=%s smoke=%s development=%s\n' \
  "${prep}" "${smoke}" "${development}"

