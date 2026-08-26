#!/usr/bin/env bash
# Deploy one exact-source fixed-CZ18 P20 smoke or P200 decision comparison.

set -euo pipefail

run_id=${1:?usage: submit_scc_cz18_smoke.sh RUN_ID [20|200]}
probes=${2:-20}
[[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
test "$probes" = 20 || test "$probes" = 200
if test "$probes" = 20; then
  candidate_probe_inner_tolerance=1e-8
else
  candidate_probe_inner_tolerance=1e-9
fi
repo_root=$(git rev-parse --show-toplevel)
test "$(git -C "$repo_root" symbolic-ref --short HEAD)" = main
test -z "$(git -C "$repo_root" status --porcelain --untracked-files=all)"
source_commit=$(git -C "$repo_root" rev-parse HEAD)
baseline_commit=4124b34f3ca216dcc3aae27e4b31bbac9e011f11
cmg_commit=dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10
cmg_root=${VCKSS_CMG_ROOT:?set VCKSS_CMG_ROOT to the standalone CMG checkout}
git -C "$cmg_root" cat-file -e "$cmg_commit^{commit}"
input_dta=${FCMG_CZ18_INPUT_DTA:-/projectnb/welfgr/kss-bc/runs/20260816T190433Z-5e2687c-prod/experiments/cz18_full200/retained_sample.dta}
input_sha=1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575
matlab_root=${FCMG_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
remote=/projectnb/welfgr/vckss/runs/$run_id
temporary=$(mktemp -d /private/tmp/vckss-full-cmg-cz18-scc.XXXXXX)
trap 'rm -rf "$temporary"' EXIT

git -C "$repo_root" archive --format=tar.gz -o "$temporary/candidate.tar.gz" \
  "$source_commit"
git -C "$repo_root" archive --format=tar.gz -o "$temporary/baseline.tar.gz" \
  "$baseline_commit"
git -C "$cmg_root" archive --format=tar.gz -o "$temporary/cmg.tar.gz" \
  "$cmg_commit"
for archive in candidate baseline cmg; do
  shasum -a 256 "$temporary/$archive.tar.gz" \
    > "$temporary/$archive.tar.gz.sha256"
done

ssh scc "test ! -e '$remote' && test -f '$input_dta' && test -d '$matlab_root' && mkdir -p '$remote/input/stata-spi' '$remote/source/candidate' '$remote/source/baseline' '$remote/source/cmg' '$remote/artifacts' '$remote/receipts' '$remote/logs' '$remote/qacct' '$remote/submissions'"
scp "$temporary/candidate.tar.gz" "$temporary/candidate.tar.gz.sha256" \
  "$temporary/baseline.tar.gz" "$temporary/baseline.tar.gz.sha256" \
  "$temporary/cmg.tar.gz" "$temporary/cmg.tar.gz.sha256" \
  "scc:$remote/input/"
scp "$repo_root/rust/stata_backend/stata-spi/stplugin.c" \
  "$repo_root/rust/stata_backend/stata-spi/stplugin.h" \
  "scc:$remote/input/stata-spi/"

task_sha=$(ssh scc bash -s -- "$remote" "$source_commit" \
  "$baseline_commit" "$cmg_commit" "$input_dta" "$input_sha" \
  "$matlab_root" "$probes" "$candidate_probe_inner_tolerance" <<'REMOTE_PREPARE'
set -euo pipefail
remote=$1
source_commit=$2
baseline_commit=$3
cmg_commit=$4
input_dta=$5
input_sha=$6
matlab_root=$7
probes=$8
candidate_probe_inner_tolerance=$9
for archive in candidate baseline cmg; do
  expected=$(awk '{print $1}' "$remote/input/$archive.tar.gz.sha256")
  test "$(sha256sum "$remote/input/$archive.tar.gz" | awk '{print $1}')" = "$expected"
  tar -xzf "$remote/input/$archive.tar.gz" -C "$remote/source/$archive"
done
candidate_archive_sha=$(awk '{print $1}' \
  "$remote/input/candidate.tar.gz.sha256")
printf '%s\n' "$source_commit" > "$remote/source/candidate/SOURCE_COMMIT.txt"
printf '%s\n' "$baseline_commit" > "$remote/source/baseline/SOURCE_COMMIT.txt"
printf '%s\n' "$cmg_commit" > "$remote/source/cmg/SOURCE_COMMIT.txt"
test -z "$(find "$remote/source" -type l -print -quit)"
test "$(sha256sum "$input_dta" | awk '{print $1}')" = "$input_sha"
{
  printf 'schema=VCKSS_FULL_CMG_CZ18_SMOKE_TASK_V1\n'
  printf 'source_commit=%s\n' "$source_commit"
  printf 'baseline_commit=%s\n' "$baseline_commit"
  printf 'cmg_commit=%s\n' "$cmg_commit"
  printf 'candidate_archive_sha256=%s\n' "$candidate_archive_sha"
  printf 'input_sha256=%s\n' "$input_sha"
  printf 'rows=8201888\nworkers=117529\nfirms=10603\ncells=311730\n'
  printf 'probes=%s\nseed=8675309\napplication_threads=4\nrequested_slots=14\n' "$probes"
  printf 'candidate_probe_inner_tolerance=%s\n' \
    "$candidate_probe_inner_tolerance"
  printf 'matlab_root=%s\n' "$matlab_root"
} > "$remote/receipts/task.txt"
sha256sum "$remote/receipts/task.txt" | awk '{print $1}'
chmod -R a-w "$remote/source"
REMOTE_PREPARE
)
[[ "$task_sha" =~ ^[0-9a-f]{64}$ ]]
printf '%s\n' "$task_sha" | ssh scc "cat > '$remote/receipts/task.sha256'"

job_id=$(ssh scc qsub -terse -P welfgr -pe omp 14 \
  -l h_rt=04:00:00 -l mem_per_core=4G -l no_gpu=TRUE \
  -N "vckss-cz$probes" -j y -o "$remote/logs/job.txt" -m a \
  -v "FCMG_CZ_RUN_DIR=$remote,FCMG_CZ_SOURCE_COMMIT=$source_commit,FCMG_CZ_BASELINE_COMMIT=$baseline_commit,FCMG_CZ_CMG_COMMIT=$cmg_commit,FCMG_CZ_INPUT_DTA=$input_dta,FCMG_CZ_INPUT_SHA256=$input_sha,FCMG_CZ_TASK_SHA256=$task_sha,FCMG_CZ_MATLAB_ROOT=$matlab_root,FCMG_CZ_PROBES=$probes,FCMG_CZ_PROBE_INNER_TOLERANCE=$candidate_probe_inner_tolerance" \
  "$remote/source/candidate/vckss/benchmarks/full_cmg_spike/run_scc_cz18_smoke.sge")
printf '%s\n' "$job_id" | ssh scc "cat > '$remote/submissions/job_id'"
printf 'VCKSS_FULL_CMG_CZ18_SCC_SUBMITTED job_id=%s run_dir=%s source_commit=%s task_sha256=%s probes=%s\n' \
  "$job_id" "$remote" "$source_commit" "$task_sha" "$probes"
