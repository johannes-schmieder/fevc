#!/bin/bash
# Runs every task of a q = 1 manifest on this machine, P at a time, then validates and aggregates.
# Usage: run_q1_local.sh <manifest> <out-root> <parallel>
set -u
MAN=$1; OUT=$2; PAR=$3; PY=../../../../.venv/bin/python
cd "$(dirname "$0")"; mkdir -p "$OUT/logs"
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 MKL_NUM_THREADS=1
NC=$($PY -c "import json,sys; m=json.load(open('$MAN')); print(len(m['cells']), m['tasks'])")
set -- $NC; CELLS=$1; TASKS=$2
for c in $(seq 0 $((CELLS-1))); do for t in $(seq 0 $((TASKS-1))); do echo "$c $t"; done; done |
  xargs -P "$PAR" -L 1 sh -c "$PY lp_q1sim.py cell '$OUT' '$MAN' \$0 \$1 > '$OUT'/logs/task-\$0-\$1.log 2>&1 || echo \"task \$0 \$1 exit \$?\" >> '$OUT'/logs/nonzero.txt"
$PY lp_q1agg.py "$OUT" "$MAN" > "$OUT/logs/aggregate.log" 2>&1; echo "aggregate exit $?" | tee -a "$OUT/logs/aggregate.log"
