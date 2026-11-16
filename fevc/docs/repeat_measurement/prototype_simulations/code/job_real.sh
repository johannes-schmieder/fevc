#!/bin/bash -l
#$ -P welfgr
#$ -N lp_real
#$ -pe omp 4
#$ -l h_rt=06:00:00
#$ -l mem_per_core=16G
#$ -j y
#$ -m n
# One-shot analysis of the six-year panel (STEP=real) or the control-term measurement (STEP=controls).
# Private inputs are read in place; only aggregates are written to runs/<RUN_ID>/.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
PANEL=<scc-private>/20260917-panel-1996-2001/inputs/pooled_adjusted_1996_2001.csv
LCS=<scc-private>/20261030-lcs-1996-2001/inputs/long_panel_lcs_nosingletons.csv
RUN_ID=${RUN_ID:?RUN_ID is required}; STEP=${STEP:?STEP is required}
module load python3/3.12.4
export OMP_NUM_THREADS=${NSLOTS:-4} OPENBLAS_NUM_THREADS=${NSLOTS:-4} MKL_NUM_THREADS=${NSLOTS:-4}
cd "$ROOT/code"; mkdir -p "$ROOT/runs/$RUN_ID"
echo "job ${JOB_ID:-} host ${HOSTNAME:-} slots ${NSLOTS:-} step $STEP $(date --iso-8601=seconds)"; sha256sum lp.py lp_real.py lp_real4.py lp_controls.py lp_diag.py lp_controls_year.py lp_controls_lcs.py lp_controls_lcs2.py lp_check.py lp_real5.py lp5.py lp_real6.py lp6.py lp_real7.py lp_controls_lcs3.py lp_check6.py
case "$STEP" in
  real) python3 lp_real.py "$ROOT/design" "$ROOT/runs/$RUN_ID" 200 ;;
  controls) echo "e4a1928d98c1dbe7185634e14470b9361b58f8d5b33cceacc8d168ccdad4634e  $PANEL" | sha256sum -c --quiet
            python3 lp_controls.py "$PANEL" "$LCS" "$ROOT/design" "$ROOT/runs/$RUN_ID" 200 ;;
  year) python3 lp_controls_year.py "$ROOT/design" "$ROOT/runs/$RUN_ID" ;;
  lcs) python3 lp_controls_lcs.py "$LCS" "$ROOT/runs/$RUN_ID" 200 ;;
  real4) python3 lp_real4.py "$ROOT/design" "$ROOT/runs/$RUN_ID" "${PROBES:?PROBES is required}" ;;
  lcs2) python3 lp_controls_lcs2.py "$LCS" "$ROOT/runs/$RUN_ID" 200 ;;
  check) python3 lp_check.py lp_real5.py lp5.py lp_real6.py lp6.py lp_real7.py lp_controls_lcs3.py lp_check6.py "$ROOT/design" "$ROOT/runs/$RUN_ID" "$ROOT/runs/mc-v4/manifest.json" 3 N1 N2 N6 N8 N9 ;;
  real5) python3 lp_real5.py "$ROOT/design" "$ROOT/runs/$RUN_ID" "${PROBES:?PROBES is required}" ;;
  real6) python3 lp_real6.py "$ROOT/design" "$ROOT/runs/$RUN_ID" "${PROBES:?PROBES is required}" ;;
  real7) python3 lp_real7.py "$ROOT/design" "$ROOT/runs/$RUN_ID" "${PROBES:?PROBES is required}" "${MODEL:?MODEL is required}" ;;
  lcs3) python3 lp_controls_lcs3.py "$LCS" "$ROOT/runs/$RUN_ID" 200 tw twl tlw dq ;;
  check6) python3 lp_check6.py "$ROOT/design" "$ROOT/runs/$RUN_ID" "$ROOT/runs/mc-v6/manifest.json" 3 N1 N6 N10 ;;
  diag) python3 lp_diag.py "$ROOT/design" "$ROOT/runs/$RUN_ID" ;;
  *) echo "unknown STEP" >&2; exit 2 ;;
esac
echo "done $(date --iso-8601=seconds)"
