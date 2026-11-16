#!/bin/bash -l
#$ -P welfgr
#$ -N lp_r8
#$ -pe omp 4
#$ -l h_rt=02:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# One clean run of the note's standard error on the six-year panel (aggregates only; private inputs read in place).
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}
case "$RUN_ID" in *[!A-Za-z0-9._-]*) echo "invalid run id" >&2; exit 2 ;; esac
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} host ${HOSTNAME:-} $(date --iso-8601=seconds)"
sha256sum lp6.py lp_real8.py
python3 lp_real8.py "$ROOT/design" "$ROOT/runs/$RUN_ID" 1600
echo "done $(date --iso-8601=seconds)"
