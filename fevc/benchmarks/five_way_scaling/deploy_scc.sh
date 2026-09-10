#!/bin/bash
set -euo pipefail
run_id=${1:?usage: deploy_scc.sh RUN_ID COMPARATOR_ARCHIVE_DIR}
archive_dir=${2:?}
[[ "$run_id" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]]
harness=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$harness/../../.." && pwd)
python=$repo/.venv/bin/python
test -x "$python"
archive_names=(matlab.tar.gz julia.tar.gz r.tar.gz python.tar.gz sanic_0.0.2.tar.gz)
archive_hashes=(
  d8b48ae7f994d8f9b007b9d6b2c750f82499d4e3e5279e05fbe8a2945548c417
  9ad42cdaa88c47eee66729c99b1ef86f57fd97ce83a28755bc081270af76e571
  f70360b6d880dba0f87ddc2604fe289cc9a731b1d1f98dd5e2291fc8333d5d33
  b3a221e0f145b6f1b5f66b6073d98dd9297ea0ab80008e670fab3a2e18d13675
  998041c3303f63c3651070c49d9d991118223a8ba4d7e8199ea6c3ce3e38175e
)
for index in "${!archive_names[@]}"; do
  name=${archive_names[$index]}
  test -f "$archive_dir/$name"
  test "$(shasum -a 256 "$archive_dir/$name" | awk '{print $1}')" = "${archive_hashes[$index]}"
done
staging=$(mktemp -d /private/tmp/fevc-five-way-deploy.XXXXXX)
trap 'rm -rf "$staging"' EXIT
for profile in smoke pilot exact confirmation; do
  "$python" "$harness/build_manifest.py" --profile "$profile" \
    --output "$staging/$profile.tsv" --identity "$staging/$profile.identity.json"
done
(cd "$harness" && find . -type f ! -path '*/__pycache__/*' -print0 | sort -z | xargs -0 shasum -a 256) > "$staging/code.sha256"
remote=/projectnb/welfgr/vckss/five_way_scaling/runs/$run_id
ssh scc "test ! -e '$remote' && mkdir -p '$remote/code' '$remote/input/comparators' '$remote/logs' '$remote/output' '$remote/receipts' '$remote/submissions' '$remote/qacct'"
rsync -av --exclude '__pycache__/' --exclude '*.pyc' "$harness/" "scc:$remote/code/"
rsync -av "$staging/"*.tsv "$staging/"*.json "$staging/code.sha256" "scc:$remote/input/"
for name in "${archive_names[@]}"; do
  rsync -av "$archive_dir/$name" "scc:$remote/input/comparators/$name"
done
ssh scc "cd '$remote/code' && sha256sum -c '$remote/input/code.sha256' && test \"\$(find '$remote/input/comparators' -type f | wc -l)\" = 5"
printf '%s\n' "$remote"
