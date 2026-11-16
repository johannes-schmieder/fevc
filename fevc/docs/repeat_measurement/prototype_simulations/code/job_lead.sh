#!/bin/bash -l
#$ -P welfgr
#$ -N lp_lead
#$ -pe omp 4
#$ -l h_rt=04:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# Leading direction by Lanczos on a real design: solves needed and early-stopped q = 1 diagnostics (aggregates only;
# private inputs read in place). RUN_ID: output directory under runs/; DDIR: design, design_y3 or design_half.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}; DDIR=${DDIR:?DDIR is required}; CAP=${CAP:-60}
case "$RUN_ID" in *[!A-Za-z0-9._-]*) echo "invalid run id" >&2; exit 2 ;; esac
case "$DDIR" in design|design_y3|design_half) ;; *) echo "invalid design" >&2; exit 2 ;; esac
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"
echo "job ${JOB_ID:-} host ${HOSTNAME:-} design $DDIR $(date --iso-8601=seconds)"
sha256sum lp6.py lp7.py lp_q1.py lp_lead.py lp_real_lead.py akm_design.py sparse_auto.py dense_mc.py
python3 lp_real_lead.py "$ROOT/$DDIR" "$ROOT/runs/$RUN_ID" 1600 "$CAP"
echo "done $(date --iso-8601=seconds)"
