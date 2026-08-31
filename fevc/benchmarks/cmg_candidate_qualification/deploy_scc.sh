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
remote=/projectnb/welfgr/fevc/runs/$run_id
candidate=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["candidate_commit"])' "$stage/run_identity.json")
comparison=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["comparison_commit"])' "$stage/run_identity.json")
[[ "$candidate" =~ ^[0-9a-f]{40}$ && "$comparison" =~ ^[0-9a-f]{40}$ ]]
ssh scc "test ! -e '$remote' && mkdir -p '$remote'"
rsync -a "$stage/" "scc:$remote/"
ssh scc "set -euo pipefail; cd '$remote/input'; sha256sum -c candidate.tar.gz.sha256; sha256sum -c comparison.tar.gz.sha256; sha256sum -c stata-spi.sha256; cd '$remote'; mkdir sources; tar -xzf input/candidate.tar.gz; mv source sources/candidate; tar -xzf input/comparison.tar.gz; mv source sources/comparison; printf '%s\n' '$candidate' > sources/candidate/SOURCE_COMMIT.txt; printf '%s\n' '$comparison' > sources/comparison/SOURCE_COMMIT.txt; cd sources/candidate; sha256sum -c '$remote/input/candidate.files.sha256' > '$remote/receipts/candidate_source.check'; cd '$remote/sources/comparison'; sha256sum -c '$remote/input/comparison.files.sha256' > '$remote/receipts/comparison_source.check'; printf 'VCKSS_CMG_CANDIDATE_QUALIFICATION_DEPLOY_PASS %s %s\n' '$candidate' '$comparison' > '$remote/receipts/deployment.pass'"
printf 'FEVC CMG CANDIDATE QUALIFICATION DEPLOY PASS: %s\n' "$remote"
