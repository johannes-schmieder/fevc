#!/bin/bash -l
#$ -P welfgr
#$ -N lp_small
#$ -pe omp 1
#$ -l h_rt=00:20:00
#$ -l mem_per_core=8G
#$ -j y
#$ -m n
# Small aggregate computations on the six-year panel arrays (no linear solves). Private arrays read in place.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
module load python3/3.12.4
cd "$ROOT/code"; echo "job ${JOB_ID:-} host ${HOSTNAME:-} $(date --iso-8601=seconds)"; sha256sum "${SCRIPT:?}"
python3 "$SCRIPT" "$ROOT/design" "$ROOT/runs/${RUN_ID:?}"
echo "done $(date --iso-8601=seconds)"
