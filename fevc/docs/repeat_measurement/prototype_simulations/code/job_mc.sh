#!/bin/bash -l
#$ -P welfgr
#$ -N lp_mc
#$ -pe omp 4
#$ -l h_rt=06:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# Simulation on the six-year design. MODE=prepare needs DESIGN; MODE=theory and MODE=task are array jobs whose
# task id (1-based) indexes the manifest's cells (theory) or (cell, task) pairs (task). Each task writes only
# its own directory under runs/<RUN_ID>/; private inputs are read in place and never copied.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}; MODE=${MODE:?MODE is required}
case "$RUN_ID" in *[!A-Za-z0-9._-]*) echo "invalid RUN_ID" >&2; exit 2 ;; esac
MANIFEST="$ROOT/runs/$RUN_ID/manifest.json"
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} task ${SGE_TASK_ID:-} host ${HOSTNAME:-} mode $MODE $(date --iso-8601=seconds)"
sha256sum lp.py lp_mc.py "$MANIFEST" 2>/dev/null || true
case "$MODE" in
  prepare) python3 lp_mc.py prepare "$ROOT/design" "${DESIGN:?DESIGN is required}" "${PROBES:-200}" ;;
  theory)  python3 lp_mc.py theory "$ROOT/design" "$ROOT/runs/$RUN_ID" "$MANIFEST" $((SGE_TASK_ID - 1)) ;;
  task)    python3 lp_mc.py task "$ROOT/design" "$ROOT/runs/$RUN_ID" "$MANIFEST" $((SGE_TASK_ID - 1)) ;;
  *) echo "unknown MODE" >&2; exit 2 ;;
esac
echo "done $(date --iso-8601=seconds)"
