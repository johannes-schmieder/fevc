#!/bin/bash
# Single local entrypoint for FEVC--MATLAB SCC smoke and production campaigns.
set -euo pipefail
if (( $# < 1 )); then
  printf 'usage: campaign.sh smoke|submit|status|collect|retry [arguments]\n' >&2
  exit 198
fi
action=$1
shift
script_dir=$(cd "$(dirname "$0")" && pwd -P)
repo_root=$(cd "$script_dir/../../.." && pwd -P)
python_bin=${VCKSS_PYTHON:-$repo_root/.venv/bin/python}
remote_root=/projectnb/welfgr/vckss/runs

case "$action" in
  smoke|submit)
    test "$#" = 0
    "$python_bin" -m pytest -q \
      "$script_dir/tests/test_campaign.py"
    short_sha=$(git -C "$repo_root" rev-parse --short=8 HEAD)
    run_id=$(date -u +%Y%m%dT%H%M%SZ)-$short_sha-$action
    stage_parent=$(mktemp -d /private/tmp/fevc-m26-stage.XXXXXX)
    stage=$stage_parent/$run_id
    build_args=(--repo "$repo_root" --output "$stage" \
      --stata-spi "$repo_root/rust/stata_backend/stata-spi")
    if test "$action" = smoke; then build_args+=(--development); fi
    "$python_bin" "$script_dir/build_run.py" "${build_args[@]}"
    "$script_dir/deploy_scc.sh" "$stage"
    remote=$remote_root/$run_id
    submit_mode=$(test "$action" = submit && printf campaign || printf smoke)
    ssh scc "bash '$remote/source/fevc/benchmarks/fevc_matlab_2026/submit_scc.sh' '$remote' '$submit_mode'"
    printf 'FEVC MATLAB 2026 CAMPAIGN STARTED: run=%s staging=%s\n' \
      "$run_id" "$stage"
    ;;
  status)
    test "$#" = 1
    run_id=$1
    [[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
    remote=$remote_root/$run_id
    ssh scc "bash '$remote/source/fevc/benchmarks/fevc_matlab_2026/campaign_status.sh' '$remote'"
    ;;
  collect)
    (( $# == 1 || $# == 2 ))
    run_id=$1
    [[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
    destination=${2:-/private/tmp/fevc-m26-collection-$run_id}
    test ! -e "$destination"
    remote=$remote_root/$run_id
    ssh scc "bash '$remote/source/fevc/benchmarks/fevc_matlab_2026/collect_campaign.sh' '$remote'"
    mkdir -p "$destination"
    rsync -a "scc:$remote/collection/" "$destination/"
    printf 'FEVC MATLAB 2026 COLLECTION READY: %s\n' "$destination"
    ;;
  retry)
    test "$#" = 2
    run_id=$1
    bundle_ids=$2
    [[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
    [[ "$bundle_ids" =~ ^[0-9,-]+$ ]]
    remote=$remote_root/$run_id
    ssh scc "bash '$remote/source/fevc/benchmarks/fevc_matlab_2026/retry_scc.sh' '$remote' '$bundle_ids'"
    ;;
  *)
    printf 'unknown campaign action: %s\n' "$action" >&2
    exit 198
    ;;
esac
