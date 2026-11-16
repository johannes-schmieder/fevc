#!/bin/bash -l
#$ -P welfgr
#$ -N lp_vfin
#$ -l h_rt=00:30:00
#$ -j y
#$ -m n
# Inventory validation, aggregation and acceptance check of one validation run (aggregates only).
set -uo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
RUN_ID=${RUN_ID:?RUN_ID is required}
case "$RUN_ID" in *[!A-Za-z0-9._-]*) echo "invalid run id" >&2; exit 2 ;; esac
module load python3/3.12.4
cd "$ROOT/code"
(cd "$ROOT/code" && sha256sum -c "$ROOT/runs/$RUN_ID/source.sha256") > "$ROOT/runs/$RUN_ID/source.check" 2>&1; echo "source check rc=$?"
python3 lp_agg.py "$ROOT/runs/$RUN_ID" "$ROOT/runs/$RUN_ID/manifest.json" > "$ROOT/runs/$RUN_ID/agg.log" 2>&1; echo "agg rc=$?"; tail -1 "$ROOT/runs/$RUN_ID/agg.log"
python3 lp_val7.py "$ROOT/runs/$RUN_ID" "$ROOT/runs/$RUN_ID/manifest.json" > "$ROOT/runs/$RUN_ID/validation.log" 2>&1; echo "validation rc=$?"; cat "$ROOT/runs/$RUN_ID/validation.log"
echo "done $(date --iso-8601=seconds)"
