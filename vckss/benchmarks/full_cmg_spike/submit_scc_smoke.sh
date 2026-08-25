#!/usr/bin/env bash
# Deploy and submit one exact-source four-slot SCC architectural smoke.

set -euo pipefail

run_id=${1:?usage: submit_scc_smoke.sh RUN_ID}
[[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
repo_root=$(git rev-parse --show-toplevel)
test "$(git -C "$repo_root" symbolic-ref --short HEAD)" = main
test -z "$(git -C "$repo_root" status --porcelain --untracked-files=all)"
source_commit=$(git -C "$repo_root" rev-parse HEAD)
baseline_commit=4124b34f3ca216dcc3aae27e4b31bbac9e011f11
cmg_commit=dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10
cmg_root=${VCKSS_CMG_ROOT:?set VCKSS_CMG_ROOT to the standalone CMG checkout}
git -C "$cmg_root" cat-file -e "$cmg_commit^{commit}"
remote=/projectnb/welfgr/vckss/runs/$run_id
matlab_root=${FCMG_MATLAB_ROOT:-/projectnb/welfgr/separations/Code_IEB/do/LeaveOutTwoWay}
temporary=$(mktemp -d /private/tmp/vckss-full-cmg-scc.XXXXXX)
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

ssh scc "test ! -e '$remote' && mkdir -p '$remote/input/stata-spi' '$remote/source/candidate' '$remote/source/baseline' '$remote/source/cmg' '$remote/artifacts' '$remote/receipts' '$remote/logs' '$remote/qacct' '$remote/submissions'"
scp "$temporary/candidate.tar.gz" "$temporary/candidate.tar.gz.sha256" \
  "$temporary/baseline.tar.gz" "$temporary/baseline.tar.gz.sha256" \
  "$temporary/cmg.tar.gz" "$temporary/cmg.tar.gz.sha256" \
  "scc:$remote/input/"
scp "$repo_root/rust/stata_backend/stata-spi/stplugin.c" \
  "$repo_root/rust/stata_backend/stata-spi/stplugin.h" \
  "scc:$remote/input/stata-spi/"

ssh scc bash -s -- "$remote" "$source_commit" "$baseline_commit" \
  "$cmg_commit" <<'REMOTE_PREPARE'
set -euo pipefail
remote=$1
source_commit=$2
baseline_commit=$3
cmg_commit=$4
for archive in candidate baseline cmg; do
  expected=$(awk '{print $1}' "$remote/input/$archive.tar.gz.sha256")
  test "$(sha256sum "$remote/input/$archive.tar.gz" | awk '{print $1}')" = "$expected"
  tar -xzf "$remote/input/$archive.tar.gz" -C "$remote/source/$archive"
done
printf '%s\n' "$source_commit" > "$remote/source/candidate/SOURCE_COMMIT.txt"
printf '%s\n' "$baseline_commit" > "$remote/source/baseline/SOURCE_COMMIT.txt"
printf '%s\n' "$cmg_commit" > "$remote/source/cmg/SOURCE_COMMIT.txt"
test -z "$(find "$remote/source" -type l -print -quit)"
chmod -R a-w "$remote/source"
REMOTE_PREPARE

job_id=$(ssh scc qsub -terse -P welfgr -pe omp 4 \
  -l h_rt=04:00:00 -l mem_per_core=16G -l no_gpu=TRUE \
  -N vckss-fcmg4 -j y -o "$remote/logs" -m a \
  -v "FCMG_RUN_DIR=$remote,FCMG_SOURCE_COMMIT=$source_commit,FCMG_BASELINE_COMMIT=$baseline_commit,FCMG_CMG_COMMIT=$cmg_commit,FCMG_MATLAB_ROOT=$matlab_root" \
  "$remote/source/candidate/vckss/benchmarks/full_cmg_spike/run_scc_smoke.sge")
printf '%s\n' "$job_id" | ssh scc "cat > '$remote/submissions/job_id'"
printf 'VCKSS_FULL_CMG_SCC_SUBMITTED job_id=%s run_dir=%s source_commit=%s\n' \
  "$job_id" "$remote" "$source_commit"
