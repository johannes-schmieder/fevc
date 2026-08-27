#!/bin/bash
set -euo pipefail
if test "$#" != 1; then
  printf 'usage: deploy_scc.sh LOCAL_STAGING_DIR\n' >&2
  exit 198
fi
stage=${1%/}
test -d "$stage" && test -f "$stage/run_identity.json"
run_id=${stage##*/}
[[ "$run_id" =~ ^[A-Za-z0-9._-]+$ ]]
python_bin=${VCKSS_PYTHON:-python3}
source_commit=$($python_bin -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["source_commit"])' \
  "$stage/run_identity.json")
bundle_sha=$($python_bin -c \
  'import json,sys; print(json.load(open(sys.argv[1]))["bundle_sha256"])' \
  "$stage/run_identity.json")
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$bundle_sha" =~ ^[0-9a-f]{64}$ ]]
remote=/projectnb/welfgr/vckss/runs/$run_id
ssh scc "test ! -e '$remote' && mkdir -p '$remote'"
rsync -a "$stage/" "scc:$remote/"
ssh scc "set -euo pipefail; cd '$remote/input'; sha256sum -c source.tar.gz.sha256; sha256sum -c stata-spi.sha256; cd '$remote'; test ! -e source; tar -xzf input/source.tar.gz; printf '%s\\n' '$source_commit' > source/SOURCE_COMMIT.txt; cd source; sha256sum -c '$remote/input/source.files.sha256' > '$remote/receipts/deployment_source_manifest.check'; printf 'VCKSS_COMPARATIVE_SCALING_DEPLOY_PASS %s %s\\n' '$source_commit' '$bundle_sha' > '$remote/receipts/deployment.pass'"
printf 'VCKSS COMPARATIVE SCALING SCC DEPLOY PASS: %s\n' "$remote"
