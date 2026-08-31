#!/bin/bash
set -euo pipefail
if test "$#" != 1; then
  printf 'usage: deploy_scc.sh RUN_ID\n' >&2
  exit 198
fi
run_id=$1
[[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
repo=$(git rev-parse --show-toplevel)
test -z "$(git -C "$repo" status --porcelain)"
source_commit=$(git -C "$repo" rev-parse HEAD)
stage=$(mktemp -d /private/tmp/fevc-projection-scaling.XXXXXX)
mkdir -p "$stage/input/stata-spi"
git -C "$repo" archive --format=tar.gz --prefix=source/ \
  --output="$stage/input/source.tar.gz" "$source_commit"
cp "$repo/rust/stata_backend/stata-spi/stplugin.c" \
  "$repo/rust/stata_backend/stata-spi/stplugin.h" \
  "$stage/input/stata-spi/"
(cd "$stage/input" && shasum -a 256 source.tar.gz > source.tar.gz.sha256)
(cd "$stage/input" && find stata-spi -type f -print0 | sort -z | \
  xargs -0 shasum -a 256 > stata-spi.sha256)
remote=/projectnb/welfgr/fevc/runs/$run_id
ssh scc "test ! -e '$remote' && mkdir -p '$remote/input' '$remote/logs' '$remote/receipts' '$remote/submissions'"
scp "$stage/input/source.tar.gz" "$stage/input/source.tar.gz.sha256" \
  "$stage/input/stata-spi.sha256" "scc:$remote/input/"
scp -r "$stage/input/stata-spi" "scc:$remote/input/"
ssh scc "set -euo pipefail; cd '$remote/input'; sha256sum -c source.tar.gz.sha256; sha256sum -c stata-spi.sha256; cd '$remote'; tar -xzf input/source.tar.gz; printf '%s\n' '$source_commit' > source/SOURCE_COMMIT.txt; printf 'FEVC PROJECTION SCALING DEPLOY PASS %s\n' '$source_commit' > receipts/deployment.pass"
printf 'FEVC PROJECTION SCALING SCC DEPLOY PASS %s %s\n' \
  "$remote" "$source_commit"
