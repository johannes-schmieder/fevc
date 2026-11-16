#!/bin/bash
# Wait until all listed SCC jobs have left the queue, then print the tails of the run logs and the result inventory.
base=<scc-private>/20261111-se-longpanel/e2e-250a760c
while true; do
    left=$(ssh -o BatchMode=yes -o ConnectTimeout=30 scc "qstat -u \$USER | grep -c fevc_e2e" < /dev/null 2>/dev/null)
    rc=$?
    if [ $rc -le 1 ] && [ "${left:-1}" = "0" ]; then break; fi
    sleep 90
done
ssh -o BatchMode=yes -o ConnectTimeout=30 scc "cd $base && ls results && for f in logs/run_design*.log; do echo == \$f; tail -4 \$f; done" < /dev/null
