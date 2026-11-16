#!/bin/bash -l
#$ -P welfgr
#$ -N lp_val
#$ -pe omp 4
#$ -l h_rt=08:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# Validation of the frozen estimator (lp7.LPF). MODE=sub builds a sub-panel (SUBARGS, e.g. "years 1999 2001" or
# "half 20261112") from design/ into DDIR; MODE=prepare caches diagonals and truth for DDIR; MODE=theory and
# MODE=task are array jobs over the manifest's cells and (cell, task) pairs. Each task writes only its own
# directory under runs/<RUN_ID>/; private inputs are read in place and never copied out of the project area.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}; MODE=${MODE:?MODE is required}; DDIR=${DDIR:?DDIR is required}
case "$RUN_ID$DDIR" in *[!A-Za-z0-9._-]*) echo "invalid RUN_ID or DDIR" >&2; exit 2 ;; esac
MANIFEST="$ROOT/runs/$RUN_ID/manifest.json"
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} task ${SGE_TASK_ID:-} host ${HOSTNAME:-} mode $MODE ddir $DDIR $(date --iso-8601=seconds)"
sha256sum lp6.py lp7.py lp_mc6.py lp_mc7.py lp_sub.py "$MANIFEST"
case "$MODE" in
  sub)     python3 lp_sub.py "$ROOT/design" "$ROOT/$DDIR" ${SUBARGS:?SUBARGS is required} ;;
  prepare) python3 lp_mc7.py prepare "$ROOT/$DDIR" default 1600 ;;
  theory)  python3 lp_mc7.py theory "$ROOT/$DDIR" "$ROOT/runs/$RUN_ID" "$MANIFEST" $((SGE_TASK_ID - 1)) ;;
  task)    python3 lp_mc7.py task "$ROOT/$DDIR" "$ROOT/runs/$RUN_ID" "$MANIFEST" $((SGE_TASK_ID - 1)) ;;
  *) echo "unknown MODE" >&2; exit 2 ;;
esac
echo "done $(date --iso-8601=seconds)"
