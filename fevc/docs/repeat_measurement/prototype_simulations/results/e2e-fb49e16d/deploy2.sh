#!/bin/bash
# Deploy the source bundle of the current commit to the private SCC directory, submit the build (with its smoke
# through the plain default command) and, held on it, the end-to-end runs on the three private panels. A run job
# starts only after the build job has ended and refuses to run without a passing build receipt.
set -euo pipefail
here=<session-tmp>/e2e
spi=<repo>/rust/stata_backend/stata-spi
commit=$(cat "$here/source_commit.txt"); short=${commit:0:8}
base=<scc-private>/20261111-se-longpanel/e2e-$short
ssh -o BatchMode=yes scc "umask 077; test ! -e $base && mkdir -p $base/code $base/stata-spi $base/logs" < /dev/null
scp -q "$here/source.tar.gz" "$here/source_commit.txt" scc:$base/
scp -q "$here/job_build.sh" "$here/job_run.sh" "$here/smoke.do" "$here/export_panel.py" "$here/veneto_e2e.do" scc:$base/code/
scp -q "$spi/stplugin.c" "$spi/stplugin.h" scc:$base/stata-spi/
ssh -o BatchMode=yes scc "cd $base && chmod -R go-rwx . && sha256sum source.tar.gz code/* && b=\$(qsub -terse -v BASE=$base -o $base/logs/build.\\\$JOB_ID.log code/job_build.sh) && echo build \$b && for d in design_half design_y3 design; do qsub -terse -hold_jid \$b -v BASE=$base,DDIR=\$d -o $base/logs/run_\$d.\\\$JOB_ID.log code/job_run.sh; done" < /dev/null
echo "deployed to $base"
