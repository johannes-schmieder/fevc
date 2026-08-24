from __future__ import annotations

import csv
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "benchmarks/fe_buf1/analyze_scc.py"


def _module():
    spec = importlib.util.spec_from_file_location("fe_buf1_scc", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _write_single_run(
    module, root: Path, order: str, role: str, command_seconds: str
) -> None:
    directory = root / f"F15625-P256-{order}"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "pair.pass").write_text("PASS\n", encoding="utf-8")
    row = {
        "source_label": role,
        "source_commit": module.BASELINE if role == "baseline" else module.CANDIDATE,
        "run": "1",
        "result_mreldif": "0",
        **{field: "1" for field in module.EXACT},
        **{field: "1" for field in module.TIMINGS},
        "fe_buffered_columns": "0" if role == "baseline" else "9",
        "fe_legacy_columns": "10" if role == "baseline" else "1",
        "fe_workspace_bytes": "0" if role == "baseline" else "164000000",
        "fe_avoided_bytes": "0" if role == "baseline" else "1078181250000",
    }
    row["n_rows"] = "937500"
    row["command_s"] = command_seconds
    with (directory / f"{role}.csv").open(
        "w", newline="", encoding="utf-8"
    ) as handle:
        writer = csv.DictWriter(handle, fieldnames=row)
        writer.writeheader()
        writer.writerow(row)


def test_single_repetition_endpoint_uses_its_only_accounting_row(
    tmp_path: Path, monkeypatch
) -> None:
    module = _module()
    module.SIZES = (15625,)
    standard = tmp_path / "standard"
    large = tmp_path / "large"
    output = tmp_path / "summary.json"
    for order in module.ORDERS:
        _write_single_run(module, large, order, "baseline", "100")
        _write_single_run(module, large, order, "candidate", "95")
    monkeypatch.setattr(
        sys,
        "argv",
        [
            str(SCRIPT),
            "--root",
            str(standard),
            "--large-root",
            str(large),
            "--output",
            str(output),
        ],
    )
    assert module.main() == 0
    summary = json.loads(output.read_text(encoding="utf-8"))
    assert summary["status"] == "PASS"
    assert len(summary["pairs"]) == 2
    assert summary["pairs"][0]["buffered_columns"] == 9
    assert summary["pairs"][0]["modeled_workspace_bytes"] == 164000000
    command_change = summary["median_change_percent_by_size"]["15625"]["command_s"]
    assert abs(command_change + 5) < 1e-12
