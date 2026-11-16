#!/bin/bash
# Wait until an SCC job has left the queue, then print the tail of its log. Usage: wait_job.sh <job id> <log path on SCC> [lines]
job=$1; log=$2; lines=${3:-40}
while true; do
    state=$(ssh -o BatchMode=yes -o ConnectTimeout=30 scc "qstat -u \$USER | awk -v j=$job '\$1 == j {print \$5}'" < /dev/null 2>/dev/null)
    rc=$?
    if [ $rc -eq 0 ] && [ -z "$state" ]; then break; fi
    sleep 60
done
ssh -o BatchMode=yes -o ConnectTimeout=30 scc "tail -n $lines $log" < /dev/null
