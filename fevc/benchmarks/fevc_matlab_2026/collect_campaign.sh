#!/bin/bash
# Collect terminal accounting and validate one completed smoke or campaign.
set -euo pipefail
if test "$#" != 1; then
  printf 'usage: collect_campaign.sh RUN_DIR\n' >&2
  exit 198
fi
run_dir=${1%/}
[[ "$run_dir" == /projectnb/welfgr/vckss/runs/* ]]
script_dir=$(cd "$(dirname "$0")" && pwd -P)
collector_source_commit=${FEVC_COLLECTION_SOURCE_COMMIT:-}
if test -z "$collector_source_commit"; then
  collector_source_commit=$(git -C "$script_dir" rev-parse HEAD)
fi
[[ "$collector_source_commit" =~ ^[0-9a-f]{40}$ ]]
export FEVC_COLLECTION_SOURCE_COMMIT=$collector_source_commit
submission=
for candidate in "$run_dir/submissions/campaign.tsv" "$run_dir/submissions/smoke.tsv"; do
  if test -f "$candidate"; then submission=$candidate; break; fi
done
test -n "$submission"
test ! -e "$run_dir/collection/campaign.json"

module purge
module load python3/3.12.4
python_bin=$(command -v python3)
harness=$script_dir
value() { awk -F '\t' -v key="$1" '$1 == key {print $2}' "$submission"; }
smoke_job=$(value smoke_job_id)
pilot_job=$(value pilot_job_id)
production_job=$(value production_job_id)
production_accounting_job=${production_job%%.*}

smoke_attempt=$run_dir/attempts/smoke
smoke_qacct=$smoke_attempt/qacct/smoke.txt
test ! -e "$smoke_qacct"
qacct -j "$smoke_job" > "$smoke_qacct"
smoke_experiment=smoke_strong_d2_n7680_c4_r1
"$python_bin" "$harness/validate_task.py" \
  --job-dir "$smoke_attempt/tasks/$smoke_experiment" \
  --qacct "$smoke_qacct" --output "$smoke_attempt/validations/1.json"

if test "$pilot_job" != NONE; then
  "$python_bin" "$harness/collect_generation.py" --run-dir "$run_dir" \
    --attempt-id pilot --job-id "$pilot_job" --bundle-range 24 \
    --cell-filter 5,235 --output "$run_dir/receipts/pilot.generation.json"
  "$python_bin" "$harness/validate_pilot.py" --run-dir "$run_dir" \
    --attempt-id pilot --output "$run_dir/receipts/pilot.pass.json"
fi

if test "$production_job" != NONE; then
  retry_submission=$run_dir/submissions/retry.tsv
  if test -f "$retry_submission"; then
    retry_value() {
      awk -F '\t' -v key="$1" '$1 == key {print $2}' "$retry_submission"
    }
    retry_job=$(retry_value retry_job_id)
    retry_bundles=$(retry_value retry_bundle_range)
    "$python_bin" "$harness/collect_generation.py" --run-dir "$run_dir" \
      --attempt-id production --job-id "$production_accounting_job" --bundle-range 1-24 \
      --skip-bundles "$retry_bundles" --cell-filter NONE \
      --output "$run_dir/receipts/production.original.generation.json"
    "$python_bin" "$harness/collect_generation.py" --run-dir "$run_dir" \
      --attempt-id retry --job-id "$retry_job" --bundle-range "$retry_bundles" \
      --cell-filter NONE --output "$run_dir/receipts/production.retry.generation.json"
    "$python_bin" "$harness/merge_generations.py" --run-dir "$run_dir" \
      --original "$run_dir/receipts/production.original.generation.json" \
      --retry "$run_dir/receipts/production.retry.generation.json" \
      --validation-dir "$run_dir/collection/merged_validations" \
      --output "$run_dir/receipts/production.generation.json"
    "$python_bin" "$harness/aggregate.py" --run-dir "$run_dir" \
      --attempt-id production --validation-dir "$run_dir/collection/merged_validations" \
      --output-dir "$run_dir/collection/main"
  else
    "$python_bin" "$harness/collect_generation.py" --run-dir "$run_dir" \
      --attempt-id production --job-id "$production_accounting_job" --bundle-range 1-24 \
      --cell-filter NONE --output "$run_dir/receipts/production.generation.json"
    "$python_bin" "$harness/aggregate.py" --run-dir "$run_dir" \
      --attempt-id production --output-dir "$run_dir/collection/main"
  fi
fi

"$python_bin" - "$run_dir" "$smoke_job" "$pilot_job" "$production_job" \
  "$collector_source_commit" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
identity = json.loads((root / "run_identity.json").read_text())
value = {
    "schema": "FEVC-MATLAB-2026-CAMPAIGN-COLLECTION-V1",
    "status": "PASS",
    "run_id": identity["run_id"],
    "source_mode": identity["source_mode"],
    "source_commit": identity["source_commit"],
    "collector_source_commit": sys.argv[5],
    "bundle_sha256": identity["bundle_sha256"],
    "smoke_job_id": sys.argv[2],
    "pilot_job_id": sys.argv[3],
    "production_job_id": sys.argv[4],
    "production_accounting_job_id": sys.argv[4].split(".", 1)[0],
    "production_collected": sys.argv[4] != "NONE",
}
(root / "collection" / "campaign.json").write_text(
    json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
printf 'FEVC MATLAB 2026 CAMPAIGN COLLECTION PASS: %s\n' "${run_dir##*/}"
