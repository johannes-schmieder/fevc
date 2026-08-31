#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  printf '%s\n' "usage: deploy_prod_bundle.sh SOURCE_ROOT RUN_ID" >&2
  exit 198
fi
source_root=$1
run_id=$2
[[ "$run_id" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]] || {
  printf '%s\n' "invalid sortable run ID" >&2
  exit 198
}
test -x "$source_root/.venv/bin/python"
source_commit=$(git -C "$source_root" rev-parse HEAD)
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
if [[ -n "$(git -C "$source_root" status --porcelain --untracked-files=all)" ]]; then
  printf '%s\n' "production deployment requires a clean committed checkout" >&2
  exit 198
fi
allowlist="$source_root/fevc/benchmarks/prod_bundle_allowlist.txt"
"$source_root/.venv/bin/python" \
  "$source_root/fevc/benchmarks/validate_prod_scc.py" \
  --plan "$source_root/fevc/benchmarks/prod_experiments.tsv" --static
temporary=$(mktemp -d "${TMPDIR:-/tmp}/kss-prod-bundle.XXXXXX")
trap 'rm -rf "$temporary"' EXIT
bundle_sha=$(
  "$source_root/.venv/bin/python" \
    "$source_root/fevc/benchmarks/build_prod_bundle.py" \
    --root "$source_root" --allowlist "$allowlist" \
    --source-commit "$source_commit" --output-dir "$temporary"
)
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]

remote_root=/projectnb/welfgr/fevc
remote_bundle="$remote_root/bundles/$bundle_sha"
run_dir="$remote_root/runs/$run_id"
ssh scc "mkdir -p '$remote_bundle' '$run_dir'"
# Incremental, explicit-file transfer. There is deliberately no --delete and
# no repository-root source argument.
rsync -av --partial \
  "$temporary/$bundle_sha.tar.gz" \
  "$temporary/$bundle_sha.files.sha256" \
  "scc:$remote_bundle/"
ssh scc bash -s -- "$remote_bundle" "$bundle_sha" "$run_dir" "$run_id" \
  "$source_commit" <<'REMOTE'
set -euo pipefail
bundle_dir=$1
bundle_sha=$2
run_dir=$3
run_id=$4
source_commit=$5
archive="$bundle_dir/$bundle_sha.tar.gz"
manifest="$bundle_dir/$bundle_sha.files.sha256"
test "$(sha256sum "$archive" | awk '{print $1}')" = "$bundle_sha"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
if [[ ! -d "$bundle_dir/source" ]]; then
  staging="$bundle_dir/source.tmp.$$"
  mkdir "$staging"
  tar -xzf "$archive" -C "$staging"
  (cd "$staging" && sha256sum -c "$manifest")
  mv "$staging" "$bundle_dir/source"
  chmod -R a-w "$bundle_dir/source" "$archive" "$manifest"
else
  (cd "$bundle_dir/source" && sha256sum -c "$manifest")
fi
test "$(tr -d '[:space:]' < "$bundle_dir/source/SOURCE_COMMIT.txt")" = \
  "$source_commit"
if [[ -e "$run_dir/bundle.sha256" ]]; then
  test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
  test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = "$source_commit"
else
  printf '%s\n' "$bundle_sha" > "$run_dir/bundle.sha256"
  printf '%s\n' "$bundle_dir" > "$run_dir/bundle.path"
  printf '%s\n' "$source_commit" > "$run_dir/source_commit.txt"
  printf '{"run_id":"%s","source_commit":"%s","bundle_sha256":"%s","bundle_dir":"%s","created_utc":"%s"}\n' \
    "$run_id" "$source_commit" "$bundle_sha" "$bundle_dir" \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    > "$run_dir/run.metadata.json"
fi
mkdir -p "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct" "$run_dir/experiments"
REMOTE
printf '%s\n' "$run_dir $bundle_sha $source_commit"
