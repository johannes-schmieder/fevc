#!/bin/bash -l
#$ -P welfgr
#$ -N lp_build
#$ -pe omp 4
#$ -l h_rt=01:30:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# Build the six-year panel arrays and test the direct solver. The private CSV is read in place; the
# arrays stay in the owner-only design directory; only aggregate descriptives are printed.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
PANEL=<scc-private>/20260917-panel-1996-2001/inputs/pooled_adjusted_1996_2001.csv
echo "e4a1928d98c1dbe7185634e14470b9361b58f8d5b33cceacc8d168ccdad4634e  $PANEL" | sha256sum -c --quiet
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} host ${HOSTNAME:-} slots ${NSLOTS:-} $(date --iso-8601=seconds)"
python3 lp_build.py "$PANEL" "$ROOT/design"
python3 lp_feas.py "$ROOT/design"
echo "done $(date --iso-8601=seconds)"
