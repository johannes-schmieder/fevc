#!/usr/bin/env bash
set -euo pipefail

if (( $# != 3 )); then
  printf '%s\n' "usage: collect_prod_summary.sh RUN_ID LOCAL_DEST PHASE" >&2
  exit 198
fi
run_id=$1
local_dest=$2
phase=$3
[[ "$run_id" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]] || exit 198
[[ "$phase" =~ ^(preflight|calibration|production|stress)$ ]] || exit 198
remote_run="/projectnb/welfgr/kss-bc/runs/$run_id"
test ! -e "$local_dest" || test -d "$local_dest"
mkdir -p "$local_dest"
ssh scc "test -s '$remote_run/validation/$phase.pass' && grep -Fx 'status=PASS' '$remote_run/validation/$phase.pass' && grep -Fx 'phase=$phase' '$remote_run/validation/$phase.pass'"
remote_manifest_sha=$(ssh scc "sha256sum '$remote_run/validation/$phase.evidence.sha256' | awk '{print \$1}'")
[[ "$remote_manifest_sha" =~ ^[0-9a-f]{64}$ ]]
ssh scc "grep -Fx 'evidence_manifest_sha256=$remote_manifest_sha' '$remote_run/validation/$phase.pass' && cd '$remote_run' && sha256sum -c 'validation/$phase.evidence.sha256' >/dev/null"

# Explicit privacy-safe inventory. Row-level inputs, raw paths, prepared DTAs,
# retained keys/rows, MATLAB detail, full RHS CSVs, and application logs are
# intentionally not transferable through this entrypoint.
# Do not preserve remote ownership, groups, permission bits, or timestamps.
# SCC run directories are intentionally immutable and can carry metadata that
# a managed local workspace is not permitted to reproduce.  The collected
# evidence is integrity-checked by content hashes above and below; transport
# metadata is not qualification evidence.  -O also suppresses directory-time
# restoration by older macOS rsync implementations.
rsync -rOv --prune-empty-dirs \
  --include='/run.metadata.json' \
  --include='/source_commit.txt' \
  --include='/bundle.sha256' \
  --include='/input/' \
  --include='/input/data_manifest.sha256' \
  --include='/validation/' \
  --include='/validation/*.pass' \
  --include='/validation/*.evidence.sha256' \
  --include='/validation/stress_projection.txt' \
  --include='/submissions/' \
  --include='/submissions/ledger.tsv' \
  --include='/submissions/*.job_id' \
  --include='/qacct/' \
  --include='/qacct/*.txt' \
  --include='/experiments/' \
  --include='/experiments/*/' \
  --include='/experiments/*/prod_*.csv' \
  --include='/experiments/*/prepare.csv' \
  --include='/experiments/*/prepared.sha256' \
  --include='/experiments/*/sample_comparison.csv' \
  --include='/experiments/*/calibration_selection.csv' \
  --include='/experiments/*/cmg_hierarchy.csv' \
  --include='/experiments/*/retained_sample.sha256' \
  --include='/experiments/*/stress_projection.txt' \
  --include='/experiments/*/node_characteristics.txt' \
  --include='/experiments/*/resources.txt' \
  --include='/experiments/*/wrapper.pass' \
  --exclude='*' \
  "scc:$remote_run/" "$local_dest/"

(
  cd "$local_dest"
  find . -type f ! -name collection.files.sha256 -print0 | sort -z | \
    xargs -0 sha256sum > collection.files.sha256
)
if find "$local_dest" -type f \( -name '*.dta' -o -name 'retained_matches.csv' \
    -o -name 'rhs_repetition_*.csv' -o -name 'data_manifest.tsv' \
    -o -name 'application.txt' \) | grep -q .; then
  printf '%s\n' "privacy-unsafe artifact reached the collection destination" >&2
  exit 459
fi
printf '%s\n' "KSS_PROD PRIVACY-SAFE SUMMARY COLLECTION PASS: $run_id $phase"
