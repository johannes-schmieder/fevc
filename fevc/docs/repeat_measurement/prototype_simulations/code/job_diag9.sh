#!/bin/bash -l
#$ -P welfgr
#$ -N lp_d9
#$ -pe omp 4
#$ -l h_rt=03:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# Diagnostics for the final model 'dq' on the six-year panel (aggregates only; private inputs read in place).
# STEP=real9: one clean run of the standard error. STEP=cap: the model's error at fixed characteristics for
# several winsorising limits (array task id = manifest cell, 1-based; BD_MODEL=dq).
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}; STEP=${STEP:?STEP is required}
case "$RUN_ID" in *[!A-Za-z0-9._-]*) echo "invalid run id" >&2; exit 2 ;; esac
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} task ${SGE_TASK_ID:-} host ${HOSTNAME:-} step $STEP $(date --iso-8601=seconds)"
sha256sum lp6.py lp7.py lp_mc6.py lp_bdelta.py lp_bdelta_cap2.py lp_real9.py
case "$STEP" in
  real9) python3 lp_real9.py "$ROOT/design" "$ROOT/runs/$RUN_ID" 1600 ;;
  cap)   BD_MODEL=dq python3 lp_bdelta_cap2.py "$ROOT/design" "$ROOT/runs/mc-v6" "$ROOT/runs/$RUN_ID" "$ROOT/runs/mc-v6/manifest.json" $((SGE_TASK_ID - 1)) 32 ;;
  *) echo "unknown STEP" >&2; exit 2 ;;
esac
echo "done $(date --iso-8601=seconds)"
