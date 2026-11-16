#!/bin/bash -l
# Stage 2 of val-v1: the remaining task arrays (after the smoke task was checked), with scheduler dependencies
# on the theory arrays; then one finishing job per run that validates the inventory, aggregates and applies the
# acceptance thresholds (lp_agg.py, lp_val7.py).
set -euo pipefail
ROOT=<scc-private>/20261111-se-longpanel; cd "$ROOT"
J=code/job_val7.sh
A=$(qsub -terse -t 2-80 -hold_jid 8601188 -v RUN_ID=val-v1-six,MODE=task,DDIR=design -o logs/ $J | cut -d. -f1)
B=$(qsub -terse -t 2-48 -v RUN_ID=val-v1-y3,MODE=task,DDIR=design_y3 -o logs/ $J | cut -d. -f1)
C=$(qsub -terse -t 1-48 -hold_jid 8601195 -v RUN_ID=val-v1-half,MODE=task,DDIR=design_half -o logs/ $J | cut -d. -f1)
echo "six tasks $A (smoke 8601189)" | tee -a runs/val-v1-six/jobs.txt
echo "y3 tasks $B" | tee -a runs/val-v1-y3/jobs.txt
echo "half tasks $C" | tee -a runs/val-v1-half/jobs.txt
FA=$(qsub -terse -hold_jid $A,8601189 -v RUN_ID=val-v1-six -o logs/ code/job_valfinish.sh)
FB=$(qsub -terse -hold_jid $B -v RUN_ID=val-v1-y3 -o logs/ code/job_valfinish.sh)
FC=$(qsub -terse -hold_jid $C -v RUN_ID=val-v1-half -o logs/ code/job_valfinish.sh)
echo "finish jobs $FA $FB $FC"
