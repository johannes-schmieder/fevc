from __future__ import annotations

import json
import subprocess
from pathlib import Path

source = subprocess.check_output(
    [
        "git",
        "log",
        "-1",
        "--format=%H",
        "--",
        "varcomp_kss/tests/stata/test_rust_planned_compressed_post.do",
    ],
    text=True,
).strip()
if not source:
    raise SystemExit("could not identify the latest planned bridge test commit")
receipt_path = Path(f".ci/stata/results/{source}.json")
if not receipt_path.is_file():
    raise SystemExit(f"exact-SHA licensed receipt has not been published for {source}")
receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
if (
    receipt.get("tested_sha") != source
    or receipt.get("status") != "success"
    or receipt.get("process_rc") != 0
    or receipt.get("stata_rc") != 0
    or receipt.get("tests_failed") != 0
):
    raise SystemExit(f"planned auto-exact bridge receipt is not green: {receipt}")
helper = Path("varcomp_kss/_vckss_rust_reconcile_exact_v7.ado")
if not helper.is_file():
    raise SystemExit("planned exact V7 reconciler is absent")
test = Path("varcomp_kss/tests/stata/test_rust_planned_compressed_post.do").read_text(
    encoding="utf-8"
)
marker = "// Planned algorithm(auto) selecting exact: direct V4/V7 Stata bridge certificate."
if test.count(marker) != 1 or "capture drop `xkeep'" not in test:
    raise SystemExit("planned auto-exact bridge certificate is incomplete")

out = Path(".ci/stata/qualifications/planned-auto-exact-v7.json")
if out.exists():
    raise SystemExit(f"refusing to overwrite {out}")
out.parent.mkdir(parents=True, exist_ok=True)
payload = {
    "schema_version": 1,
    "qualification": "planned-algorithm-auto-exact-v7-stata-bridge",
    "source_sha": source,
    "licensed_receipt": str(receipt_path),
    "run_id": receipt.get("run_id"),
    "runner_name": receipt.get("runner_name"),
    "stata_version": receipt.get("stata_version"),
    "stata_edition": receipt.get("stata_edition"),
    "stata_processors": receipt.get("stata_processors"),
    "profile": receipt.get("profile"),
    "suite": receipt.get("suite"),
    "status": receipt.get("status"),
    "tests_failed": receipt.get("tests_failed"),
    "process_rc": receipt.get("process_rc"),
    "stata_rc": receipt.get("stata_rc"),
    "contracts": [
        "VCKSS-REQUEST-CAPABILITY-V3",
        "VCKSS-EXECUTION-PLAN-V1",
        "algorithm(auto)-requested",
        "algorithm(exact)-selected",
        "engine-not-applicable",
        "resolved-and-frozen-before-rng",
        "zero-pre-resolution-rng-addressing",
        "exact-result-accounting",
        "caller-rng-and-data-restoration",
    ],
}
out.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
