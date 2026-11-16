#!/bin/bash -l
#$ -P welfgr
#$ -N lp_bd
#$ -pe omp 4
#$ -l h_rt=03:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# Oracle error b(Delta)/V of the movers' models per scenario (no outcome draws). Array job: task id (1-based)
# indexes the manifest's cells. Reads theory.json from runs/<THEORY_RUN>/, writes only
# runs/<RUN_ID>/<scenario>__<design>/bdelta.json. Private inputs are read in place and never copied.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}; THEORY_RUN=${THEORY_RUN:?THEORY_RUN is required}
case "$RUN_ID$THEORY_RUN" in *[!A-Za-z0-9._-]*) echo "invalid run id" >&2; exit 2 ;; esac
MANIFEST="$ROOT/runs/$RUN_ID/manifest.json"
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} task ${SGE_TASK_ID:-} host ${HOSTNAME:-} $(date --iso-8601=seconds)"
sha256sum lp6.py lp_mc6.py lp_bdelta.py "$MANIFEST"
python3 lp_bdelta.py "$ROOT/design" "$ROOT/runs/$THEORY_RUN" "$ROOT/runs/$RUN_ID" "$MANIFEST" $((SGE_TASK_ID - 1)) "${DRAWS:-32}"
echo "done $(date --iso-8601=seconds)"
