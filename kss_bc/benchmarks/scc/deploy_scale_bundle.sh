#!/usr/bin/env bash
set -euo pipefail

usage() {
  printf '%s\n' \
    "usage: deploy_scale_bundle.sh [--check-only] SOURCE_ROOT [RUN_ID]" >&2
  exit 198
}

check_only=0
if (( $# > 0 )) && [[ "$1" == --check-only ]]; then
  check_only=1
  shift
fi
if (( check_only )); then
  (( $# == 1 )) || usage
else
  (( $# == 2 )) || usage
fi

source_argument=$1
test -d "$source_argument"
test ! -L "$source_argument"
source_root=$(cd "$source_argument" && pwd -P)
test -x "$source_root/.venv/bin/python"
test "$(git -C "$source_root" rev-parse --show-toplevel)" = "$source_root"
test "$(git -C "$source_root" symbolic-ref --short HEAD)" = main
source_commit=$(git -C "$source_root" rev-parse HEAD)
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]

allowlist="$source_root/kss_bc/benchmarks/scale_bundle_allowlist.txt"
builder="$source_root/kss_bc/benchmarks/build_scale_bundle.py"
deployer="$source_root/kss_bc/benchmarks/scc/deploy_scale_bundle.sh"
scale_shell_sources=(
  "$deployer"
  "$source_root/kss_bc/benchmarks/scc/run_kss_scale.sge"
  "$source_root/kss_bc/benchmarks/scc/submit_kss_scale.sh"
)
test -f "$allowlist"
test -f "$builder"
for shell_source in "${scale_shell_sources[@]}"; do
  test -f "$shell_source"
  bash -n "$shell_source"
done

if (( check_only )); then
  if [[ -z "$(git -C "$source_root" status --porcelain --untracked-files=all)" ]]; then
    worktree_clean=1
  else
    worktree_clean=0
  fi
  builder_arguments=(
    --root "$source_root"
    --allowlist "$allowlist"
    --source-commit "$source_commit"
    --check-only
  )
  if (( worktree_clean )); then
    builder_arguments+=(--require-git-tracked)
  fi
  bundle_sha=$(
    "$source_root/.venv/bin/python" "$builder" "${builder_arguments[@]}"
  )
  [[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
  printf '%s\n' \
    "KSS_STREAMLINE_BUNDLE_CHECK_ONLY bundle_sha256=$bundle_sha source_commit=$source_commit worktree_clean=$worktree_clean deployment_ready=$worktree_clean"
  exit 0
fi

run_id=$2
[[ "$run_id" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]] || {
  printf '%s\n' "invalid sortable run ID" >&2
  exit 198
}
if [[ -n "$(git -C "$source_root" status --porcelain --untracked-files=all)" ]]; then
  printf '%s\n' \
    "KSS-STREAMLINE deployment requires a clean committed checkout" >&2
  exit 198
fi

temporary=$(mktemp -d "${TMPDIR:-/tmp}/kss-streamline-bundle.XXXXXX")
trap 'rm -rf "$temporary"' EXIT
bundle_sha=$(
  "$source_root/.venv/bin/python" "$builder" \
    --root "$source_root" --allowlist "$allowlist" \
    --source-commit "$source_commit" --require-git-tracked \
    --output-dir "$temporary"
)
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
test -f "$temporary/$bundle_sha.tar.gz"
test -f "$temporary/$bundle_sha.files.sha256"

remote_root=/projectnb/welfgr/kss-bc
remote_bundle="$remote_root/bundles/$bundle_sha"
remote_upload="$remote_root/uploads/streamline-$run_id-$bundle_sha"
run_dir="$remote_root/runs/$run_id"
ssh scc bash -s -- "$remote_root" "$remote_upload" "$bundle_sha" \
  <<'REMOTE_PREPARE'
set -euo pipefail
remote_root=$1
upload_dir=$2
bundle_sha=$3
test "$remote_root" = /projectnb/welfgr/kss-bc
[[ "$upload_dir" == "$remote_root"/uploads/streamline-* ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
if [[ ! -e "$remote_root" ]]; then
  mkdir "$remote_root"
fi
test -d "$remote_root"
test ! -L "$remote_root"
for child in bundles uploads runs; do
  if [[ ! -e "$remote_root/$child" ]]; then
    mkdir "$remote_root/$child"
  fi
  test -d "$remote_root/$child"
  test ! -L "$remote_root/$child"
done
if [[ ! -e "$upload_dir" ]]; then
  mkdir "$upload_dir"
fi
test -d "$upload_dir"
test ! -L "$upload_dir"
while IFS= read -r existing; do
  case "$existing" in
    "$upload_dir/$bundle_sha.tar.gz"|\
    "$upload_dir/$bundle_sha.files.sha256")
      test -f "$existing"
      test ! -L "$existing"
      ;;
    *)
      printf '%s\n' "unexpected streamlined upload artifact: $existing" >&2
      exit 65
      ;;
  esac
done < <(find "$upload_dir" -mindepth 1 -maxdepth 1 -print)
REMOTE_PREPARE
# Transfer only the two generated bundle artifacts. There is deliberately no
# repository-root source, deleting synchronization, input data, or evidence.
rsync -av --partial \
  "$temporary/$bundle_sha.tar.gz" \
  "$temporary/$bundle_sha.files.sha256" \
  "scc:$remote_upload/"

ssh scc bash -s -- "$remote_upload" "$remote_bundle" "$bundle_sha" \
  "$run_dir" "$run_id" "$source_commit" <<'REMOTE'
set -euo pipefail
upload_dir=$1
bundle_dir=$2
bundle_sha=$3
run_dir=$4
run_id=$5
source_commit=$6
archive_name="$bundle_sha.tar.gz"
manifest_name="$bundle_sha.files.sha256"
upload_archive="$upload_dir/$archive_name"
upload_manifest="$upload_dir/$manifest_name"

[[ "$upload_dir" == /projectnb/welfgr/kss-bc/uploads/streamline-* ]]
[[ "$bundle_dir" == /projectnb/welfgr/kss-bc/bundles/$bundle_sha ]]
[[ "$run_dir" == /projectnb/welfgr/kss-bc/runs/$run_id ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$run_id" =~ ^[0-9]{8}T[0-9]{6}Z-[A-Za-z0-9._-]+$ ]]
test -f "$upload_archive"
test -f "$upload_manifest"
test ! -L "$upload_dir"
test ! -L "$upload_archive"
test ! -L "$upload_manifest"
test "$(sha256sum "$upload_archive" | awk '{print $1}')" = "$bundle_sha"
test ! -L "$bundle_dir"

verify_bundle() {
  local candidate=$1
  local archive="$candidate/$archive_name"
  local manifest="$candidate/$manifest_name"
  local source="$candidate/source"
  test ! -L "$candidate"
  test -d "$source"
  test ! -L "$source"
  test -f "$archive"
  test -f "$manifest"
  test ! -L "$archive"
  test ! -L "$manifest"
  test "$(sha256sum "$archive" | awk '{print $1}')" = "$bundle_sha"
  cmp "$manifest" "$source/BUNDLE_FILES.sha256"
  (
    cd "$source"
    sha256sum -c "$manifest"
  )
  test -z "$(find "$source" -type l -print -quit)"
  diff -u \
    <({ awk '{print $2}' "$manifest"; printf '%s\n' BUNDLE_FILES.sha256; } | \
      LC_ALL=C sort) \
    <(cd "$source" && find . -type f -print | sed 's#^./##' | LC_ALL=C sort)
  test "$(tr -d '[:space:]' < "$source/BUNDLE_FORMAT.txt")" = \
    KSS-STREAMLINE-SOURCE-BUNDLE-V1
  test "$(tr -d '[:space:]' < "$source/SOURCE_COMMIT.txt")" = \
    "$source_commit"
  test -x "$source/kss_bc/benchmarks/scc/deploy_scale_bundle.sh"
}

verify_immutable_bundle() {
  local candidate=$1
  verify_bundle "$candidate"
  test -z "$(find "$candidate" -perm /222 -print -quit)"
}

if [[ -d "$bundle_dir" ]]; then
  verify_immutable_bundle "$bundle_dir"
else
  staging="$bundle_dir.tmp.$$"
  test ! -e "$staging"
  test ! -L "$staging"
  cleanup_staging() {
    if [[ -e "$staging" || -L "$staging" ]]; then
      chmod -R u+w "$staging" 2>/dev/null || true
      rm -rf "$staging"
    fi
  }
  trap cleanup_staging EXIT
  mkdir -p "$staging/source"
  cp "$upload_archive" "$staging/$archive_name"
  cp "$upload_manifest" "$staging/$manifest_name"
  tar --no-same-owner --no-same-permissions -xzf \
    "$staging/$archive_name" -C "$staging/source"
  verify_bundle "$staging"
  chmod -R a-w "$staging"
  if mv -T "$staging" "$bundle_dir" 2>/dev/null; then
    trap - EXIT
  else
    verify_immutable_bundle "$bundle_dir"
  fi
fi
verify_immutable_bundle "$bundle_dir"

mkdir -p "$run_dir"
test ! -L "$run_dir"
binding_count=0
for binding in bundle.sha256 bundle.path source_commit.txt bundle.kind; do
  if [[ -e "$run_dir/$binding" ]]; then
    binding_count=$((binding_count + 1))
  fi
done
if (( binding_count > 0 )); then
  test "$binding_count" = 4
  for binding in bundle.sha256 bundle.path source_commit.txt bundle.kind; do
    test -f "$run_dir/$binding"
    test ! -L "$run_dir/$binding"
  done
  test "$(tr -d '[:space:]' < "$run_dir/bundle.sha256")" = "$bundle_sha"
  test "$(cat "$run_dir/bundle.path")" = "$bundle_dir"
  test "$(tr -d '[:space:]' < "$run_dir/source_commit.txt")" = \
    "$source_commit"
  test "$(tr -d '[:space:]' < "$run_dir/bundle.kind")" = KSS-STREAMLINE-1
  test -f "$run_dir/run.metadata.json"
  test ! -L "$run_dir/run.metadata.json"
  grep -Fq "\"run_id\":\"$run_id\"" "$run_dir/run.metadata.json"
  grep -Fq '"milestone":"KSS-STREAMLINE-1"' "$run_dir/run.metadata.json"
  grep -Fq "\"source_commit\":\"$source_commit\"" \
    "$run_dir/run.metadata.json"
  grep -Fq "\"bundle_sha256\":\"$bundle_sha\"" \
    "$run_dir/run.metadata.json"
  grep -Fq "\"bundle_dir\":\"$bundle_dir\"" \
    "$run_dir/run.metadata.json"
  grep -Fq '"execution_boundary":"one SGE job, one Stata process per estimate"' \
    "$run_dir/run.metadata.json"
else
  test -z "$(find "$run_dir" -mindepth 1 -maxdepth 1 -print -quit)"
  printf '%s\n' "$bundle_sha" > "$run_dir/bundle.sha256"
  printf '%s\n' "$bundle_dir" > "$run_dir/bundle.path"
  printf '%s\n' "$source_commit" > "$run_dir/source_commit.txt"
  printf '%s\n' KSS-STREAMLINE-1 > "$run_dir/bundle.kind"
  printf '{"run_id":"%s","milestone":"KSS-STREAMLINE-1","source_commit":"%s","bundle_sha256":"%s","bundle_dir":"%s","execution_boundary":"one SGE job, one Stata process per estimate","created_utc":"%s"}\n' \
    "$run_id" "$source_commit" "$bundle_sha" "$bundle_dir" \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    > "$run_dir/run.metadata.json"
fi
mkdir -p "$run_dir/logs" "$run_dir/submissions" "$run_dir/qacct" \
  "$run_dir/experiments"
rm -rf "$upload_dir"
REMOTE

printf '%s\n' \
  "KSS_STREAMLINE_BUNDLE_DEPLOYED run_dir=$run_dir bundle_sha256=$bundle_sha source_commit=$source_commit source_dir=$remote_bundle/source"
