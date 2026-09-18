#!/usr/bin/env bash
set -euo pipefail

for name in OP_PACKAGE OP_INPUT OP_OUTPUT_DIR OP_PROFILE OP_VARIANT OP_THREADS \
  OP_SEED OP_WARMUP OP_REPETITION OP_INPUT_SHA256 OP_SOURCE_SHA256 \
  OP_HARNESS_DIR OP_PYTHON TMPDIR; do
  test -n "${!name:-}" || { printf 'missing %s\n' "$name" >&2; exit 198; }
done
[[ "${OP_OUTPUT_DIR}" == /projectnb/welfgr/vckss/runs/*/campaign/* ]]
[[ "${OP_INPUT_SHA256}" =~ ^[0-9a-f]{64}$ ]]
[[ "${OP_SOURCE_SHA256}" =~ ^[0-9a-f]{64}$ ]]
[[ "${OP_THREADS}" =~ ^(1|4|7)$ ]]
[[ "${OP_VARIANT}" =~ ^(baseline|candidate)$ ]]
test -d "${OP_PACKAGE}" && test ! -L "${OP_PACKAGE}"
test -f "${OP_INPUT}" && test ! -L "${OP_INPUT}"
test "$(sha256sum "${OP_INPUT}" | awk '{print $1}')" = "${OP_INPUT_SHA256}"
test "$(tr -d '[:space:]' < "${OP_PACKAGE}/SOURCE.sha256")" = "${OP_SOURCE_SHA256}"
test ! -e "${OP_OUTPUT_DIR}"
mkdir -p "${OP_OUTPUT_DIR}"
scratch=${TMPDIR}/op-call-${JOB_ID:-local}-${SGE_TASK_ID:-0}-${OP_VARIANT}-${OP_REPETITION}
test ! -e "${scratch}"
mkdir -p "${scratch}/stata"
chmod 700 "${scratch}" "${scratch}/stata"
failure_stage=launch
complete=0
finish() {
  status=$?
  if (( complete == 0 )); then
    printf 'FEVC_OPTIMIZATION_DEVELOPMENT_CALL_FAIL stage=%s status=%s\n' \
      "${failure_stage}" "${status}" > "${OP_OUTPUT_DIR}/wrapper.fail"
  fi
  exit "${status}"
}
trap finish EXIT

export STATATMP=${scratch}/stata
export OMP_NUM_THREADS=${OP_THREADS}
export RAYON_NUM_THREADS=${OP_THREADS}
result=${OP_OUTPUT_DIR}/result.tsv
export OP_OUTPUT=${result}
failure_stage=application
set +e
/usr/bin/time -v -o "${OP_OUTPUT_DIR}/resources.txt" \
  /usr/bin/timeout --signal=TERM --kill-after=30 2400 \
  stata-mp -q do "${OP_HARNESS_DIR}/development_stata.do" \
  > "${OP_OUTPUT_DIR}/application.txt" 2>&1 &
root_pid=$!
"${OP_PYTHON}" "${OP_HARNESS_DIR}/monitor_rss.py" --pid "${root_pid}" \
  --output "${OP_OUTPUT_DIR}/process_tree_rss.json" \
  > "${OP_OUTPUT_DIR}/monitor.txt" 2>&1
monitor_status=$?
wait "${root_pid}"
application_status=$?
set -e
test "${monitor_status}" = 0
test "${application_status}" = 0
test -s "${result}"
grep -F "FEVC OPTIMIZATION DEVELOPMENT CALL PASS:" \
  "${OP_OUTPUT_DIR}/application.txt" >/dev/null
maximum_rss=$("${OP_PYTHON}" -c 'import json,sys; print(json.load(open(sys.argv[1]))["maximum_physical_rss_kib"])' \
  "${OP_OUTPUT_DIR}/process_tree_rss.json")
[[ "${maximum_rss}" =~ ^[0-9]+$ ]]
(( maximum_rss <= 20 * 1024 * 1024 ))
failure_stage=receipt
{
  printf 'schema\tFEVC-OPTIMIZATION-DEVELOPMENT-WRAPPER-V1\n'
  printf 'status\tPASS\n'
  printf 'job_id\t%s\n' "${JOB_ID:-local}"
  printf 'task_id\t%s\n' "${SGE_TASK_ID:-0}"
  printf 'host\t%s\n' "${HOSTNAME:-unknown}"
  printf 'slots\t%s\n' "${NSLOTS:-unknown}"
  printf 'profile\t%s\n' "${OP_PROFILE}"
  printf 'variant\t%s\n' "${OP_VARIANT}"
  printf 'threads\t%s\n' "${OP_THREADS}"
  printf 'seed\t%s\n' "${OP_SEED}"
  printf 'warmup\t%s\n' "${OP_WARMUP}"
  printf 'repetition\t%s\n' "${OP_REPETITION}"
  printf 'physical_rss_kib\t%s\n' "${maximum_rss}"
} > "${OP_OUTPUT_DIR}/wrapper.tsv"
printf 'FEVC OPTIMIZATION DEVELOPMENT WRAPPER PASS: %s %s\n' \
  "${OP_PROFILE}" "${OP_VARIANT}" > "${OP_OUTPUT_DIR}/wrapper.pass"
complete=1
trap - EXIT
