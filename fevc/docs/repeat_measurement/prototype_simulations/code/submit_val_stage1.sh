#!/bin/bash -l
# Stage 1 of the validation campaign val-v1: theory for the six-year scenarios, one smoke task through the real
# launcher, and the two sub-panels (build -> prepare -> theory) chained by scheduler dependencies.
set -euo pipefail
ROOT=<scc-private>/20261111-se-longpanel; cd "$ROOT"
J=code/job_val7.sh
A=$(qsub -terse -t 1-10 -v RUN_ID=val-v1-six,MODE=theory,DDIR=design -o logs/ $J | cut -d. -f1)
S=$(qsub -terse -t 1 -hold_jid $A -v RUN_ID=val-v1-six,MODE=task,DDIR=design -o logs/ $J | cut -d. -f1)
echo "six theory $A smoke $S" | tee runs/val-v1-six/jobs.txt
B1=$(qsub -terse -v RUN_ID=val-v1-y3,MODE=sub,DDIR=design_y3 -v "SUBARGS=years 1999 2001" -o logs/ $J)
B2=$(qsub -terse -hold_jid $B1 -v RUN_ID=val-v1-y3,MODE=prepare,DDIR=design_y3 -o logs/ $J)
B3=$(qsub -terse -t 1-6 -hold_jid $B2 -v RUN_ID=val-v1-y3,MODE=theory,DDIR=design_y3 -o logs/ $J | cut -d. -f1)
echo "y3 sub $B1 prepare $B2 theory $B3" | tee runs/val-v1-y3/jobs.txt
C1=$(qsub -terse -v RUN_ID=val-v1-half,MODE=sub,DDIR=design_half -v "SUBARGS=half 20261112" -o logs/ $J)
C2=$(qsub -terse -hold_jid $C1 -v RUN_ID=val-v1-half,MODE=prepare,DDIR=design_half -o logs/ $J)
C3=$(qsub -terse -t 1-6 -hold_jid $C2 -v RUN_ID=val-v1-half,MODE=theory,DDIR=design_half -o logs/ $J | cut -d. -f1)
echo "half sub $C1 prepare $C2 theory $C3" | tee runs/val-v1-half/jobs.txt
