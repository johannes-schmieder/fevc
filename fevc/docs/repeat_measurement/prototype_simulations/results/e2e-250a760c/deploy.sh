#!/bin/bash
# Deploy the source bundle, the job scripts and the plugin interface files to the private SCC directory and submit
# the build job. Refuses an existing deployment.
set -euo pipefail
here=<session-tmp>/e2e
spi=<repo>/rust/stata_backend/stata-spi
commit=$(cat "$here/source_commit.txt"); short=${commit:0:8}
base=<scc-private>/20261111-se-longpanel/e2e-$short
ssh -o BatchMode=yes scc "umask 077; test ! -e $base && mkdir -p $base/code $base/stata-spi $base/logs" < /dev/null
scp -q "$here/source.tar.gz" "$here/source_commit.txt" scc:$base/
scp -q "$here/job_build.sh" "$here/job_run.sh" "$here/smoke.do" "$here/export_panel.py" "$here/veneto_e2e.do" scc:$base/code/
scp -q "$spi/stplugin.c" "$spi/stplugin.h" scc:$base/stata-spi/
ssh -o BatchMode=yes scc "cd $base && chmod -R go-rwx . && sha256sum source.tar.gz code/* && qsub -terse -v BASE=$base -o $base/logs/build.\\\$JOB_ID.log code/job_build.sh" < /dev/null
echo "deployed to $base"
