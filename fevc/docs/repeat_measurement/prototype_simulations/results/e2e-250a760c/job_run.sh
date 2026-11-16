#!/bin/bash -l
#$ -P welfgr
#$ -N fevc_e2e_run
#$ -pe omp 16
#$ -l h_rt=08:00:00
#$ -l mem_per_core=8G
#$ -j y
#$ -m n
# End-to-end package run on one private panel (DDIR: design, design_y3 or design_half) under both deletion modes.
# The panel is written to node-local scratch for Stata and removed with it; only aggregates are kept.
set -euo pipefail
umask 077
ROOT=<scc-private>/20261111-se-longpanel
BASE=${BASE:?BASE is required}; DDIR=${DDIR:?DDIR is required}
case "$BASE" in "$ROOT"/e2e-*) ;; *) echo "invalid base" >&2; exit 2 ;; esac
case "$DDIR" in design|design_y3|design_half) ;; *) echo "invalid design" >&2; exit 2 ;; esac
test -f "$BASE/build_receipt.txt"; grep -qx "smoke=PASS" "$BASE/build_receipt.txt"
test "$(sha256sum "$BASE/pkg/fevc/fevc_rust_linux_x64.plugin" | awk '{print $1}')" = "$(awk -F= '$1 == "plugin_sha256" {print $2}' "$BASE/build_receipt.txt")"
out=$BASE/results; mkdir -p "$out" "$BASE/logs"
for deletion in match observation; do test ! -e "$out/${DDIR}__$deletion.txt" || { echo "result exists; refusing to overwrite" >&2; exit 2; }; done
echo "job ${JOB_ID:-} host ${HOSTNAME:-} design $DDIR slots ${NSLOTS:-} $(date --iso-8601=seconds)"
module purge
module load python3/3.12.4
python3 "$BASE/code/export_panel.py" "$ROOT/$DDIR" "$TMPDIR/panel.csv"
module load stata-mp/19
stata=${SCC_STATA_MP_BIN:?the Stata module did not expose SCC_STATA_MP_BIN}/stata-mp
export STATATMP=$TMPDIR/statatmp; mkdir -p "$STATATMP"
failed=0
for deletion in match observation; do
    work=$TMPDIR/run-$deletion; mkdir -p "$work"; cd "$work"
    FEVC_DEFAULT_INFERENCE=none "$stata" -b do "$BASE/code/veneto_e2e.do" "$BASE/pkg/fevc" "$TMPDIR/panel.csv" "$DDIR" "$deletion" "${NSLOTS:-16}" "$out" || true
    cp "$work"/*.log "$BASE/logs/${DDIR}__${deletion}__${JOB_ID:-0}.log" 2>/dev/null || true
    if test -f "$out/${DDIR}__$deletion.txt"; then echo "done $DDIR $deletion"; else echo "FAILED $DDIR $deletion"; tail -25 "$work"/*.log; failed=1; fi
done
rm -f "$TMPDIR/panel.csv"
echo "finished $(date --iso-8601=seconds) failed=$failed"
exit $failed
