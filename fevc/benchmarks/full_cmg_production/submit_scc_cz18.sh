#!/usr/bin/env bash
# Deploy the exact-source production fixed-CZ18 P200 SCC matrix.

set -euo pipefail

run_id=${1:?usage: submit_scc_cz18.sh RUN_ID}
[[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
repo_root=$(git rev-parse --show-toplevel)
test "$(git -C "$repo_root" symbolic-ref --short HEAD)" = main
test -z "$(git -C "$repo_root" status --porcelain --untracked-files=all)"
source_commit=$(git -C "$repo_root" rev-parse HEAD)
input_dta=${VCKSS_CZ18_INPUT_DTA:-/projectnb/welfgr/kss-bc/runs/20260816T190433Z-5e2687c-prod/experiments/cz18_full200/retained_sample.dta}
input_sha=1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575
matlab_root=${VCKSS_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
remote=/projectnb/welfgr/fevc/runs/$run_id
temporary=$(mktemp -d /private/tmp/fevc-full-cmg-production-scc.XXXXXX)
trap 'rm -rf "$temporary"' EXIT

git -C "$repo_root" archive --format=tar.gz -o "$temporary/source.tar.gz" \
  "$source_commit"
shasum -a 256 "$temporary/source.tar.gz" > "$temporary/source.tar.gz.sha256"

ssh scc "test ! -e '$remote' && test -f '$input_dta' && test -d '$matlab_root' && mkdir -p '$remote/input/stata-spi' '$remote/source' '$remote/artifacts' '$remote/receipts' '$remote/logs' '$remote/qacct' '$remote/submissions'"
scp "$temporary/source.tar.gz" "$temporary/source.tar.gz.sha256" \
  "scc:$remote/input/"
scp "$repo_root/rust/stata_backend/stata-spi/stplugin.c" \
  "$repo_root/rust/stata_backend/stata-spi/stplugin.h" \
  "scc:$remote/input/stata-spi/"

task_sha=$(ssh scc bash -s -- "$remote" "$source_commit" "$input_dta" \
  "$input_sha" "$matlab_root" <<'REMOTE_PREPARE'
set -euo pipefail
remote=$1
source_commit=$2
input_dta=$3
input_sha=$4
matlab_root=$5
expected=$(awk '{print $1}' "$remote/input/source.tar.gz.sha256")
test "$(sha256sum "$remote/input/source.tar.gz" | awk '{print $1}')" = "$expected"
tar -xzf "$remote/input/source.tar.gz" -C "$remote/source"
printf '%s\n' "$source_commit" > "$remote/source/SOURCE_COMMIT.txt"
test -z "$(find "$remote/source" -type l -print -quit)"
test "$(sha256sum "$input_dta" | awk '{print $1}')" = "$input_sha"
{
  printf 'schema=VCKSS_FULL_CMG_PRODUCTION_CZ18_TASK_V1\n'
  printf 'source_commit=%s\n' "$source_commit"
  printf 'cmg_commit=dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10\n'
  printf 'source_archive_sha256=%s\n' "$expected"
  printf 'input_sha256=%s\n' "$input_sha"
  printf 'rows=8201888\nworkers=117529\nfirms=10603\ncells=311730\n'
  printf 'probes=200\nseed=8675309\napplication_threads=4\nrequested_slots=14\n'
  printf 'rounds=6\ncold_rounds=1\nwarm_rounds=5\n'
  printf 'order_schema=POSITION_BALANCED_V1\n'
  printf 'rust_toolchain=1.85.1\nmatlab_module=matlab/2024b\n'
  printf 'matlab_root=%s\n' "$matlab_root"
} > "$remote/receipts/task.txt"
sha256sum "$remote/receipts/task.txt" | awk '{print $1}'
chmod -R a-w "$remote/source"
REMOTE_PREPARE
)
[[ "$task_sha" =~ ^[0-9a-f]{64}$ ]]
printf '%s\n' "$task_sha" | ssh scc "cat > '$remote/receipts/task.sha256'"

job_id=$(ssh scc qsub -terse -P welfgr -pe omp 14 \
  -l h_rt=06:00:00 -l mem_per_core=4G -l no_gpu=TRUE \
  -N fevc-pcz -j y -o "$remote/logs/job.txt" -m a \
  -v "VCKSS_PROD_RUN_DIR=$remote,VCKSS_PROD_SOURCE_COMMIT=$source_commit,VCKSS_PROD_INPUT_DTA=$input_dta,VCKSS_PROD_INPUT_SHA256=$input_sha,VCKSS_PROD_TASK_SHA256=$task_sha,VCKSS_PROD_MATLAB_ROOT=$matlab_root" \
  "$remote/source/fevc/benchmarks/full_cmg_production/run_scc_cz18.sge")
printf '%s\n' "$job_id" | ssh scc "cat > '$remote/submissions/job_id'"
printf 'VCKSS_FULL_CMG_PRODUCTION_CZ18_SUBMITTED job_id=%s run_dir=%s source_commit=%s task_sha256=%s\n' \
  "$job_id" "$remote" "$source_commit" "$task_sha"
